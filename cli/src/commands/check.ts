import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import {
  GateRunner,
  createCacheProvider,
  getChangedFiles,
  type AnalyzeResult,
  type Warning,
} from '@anvil/core';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { success, error, info } from '../utils/output.js';

const ANALYSABLE_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'];

interface CheckOptions {
  verbose?: boolean;
  json?: boolean;
  cache?: boolean;
  changed?: boolean;
  staged?: boolean;
  since?: string;
}

interface JSONCheckOutput {
  version: '1.0.0';
  timestamp: string;
  files: string[];
  hasBlockingWarnings: boolean;
  executionTimeMs: number;
  checksRun: string[];
  warnings: Array<{
    id: string;
    category: string;
    severity: string;
    title: string;
    message: string;
    file: string;
    line: number;
    suggestion: string;
  }>;
  summary: {
    total: number;
    errors: number;
    warnings: number;
    info: number;
    suppressed: number;
  };
}

function formatWarning(w: Warning, verbose: boolean): void {
  const severityColors: Record<string, (s: string) => string> = {
    error: chalk.red,
    warning: chalk.yellow,
    info: chalk.blue,
  };
  const colorFn = severityColors[w.severity] ?? chalk.white;
  const icon = w.severity === 'error' ? '✗' : w.severity === 'warning' ? '⚠' : 'ℹ';

  console.log(colorFn(`  ${icon} [${w.id}] ${w.title}`));
  console.log(chalk.gray(`    ${w.location.file}:${w.location.line}`));
  console.log(`    ${w.message}`);

  if (verbose) {
    console.log(chalk.gray(`    Why: ${w.explanation}`));
    console.log(chalk.cyan(`    Fix: ${w.suggestion}`));
  }
  console.log('');
}

function formatResultsJSON(files: string[], result: AnalyzeResult): void {
  const output: JSONCheckOutput = {
    version: '1.0.0',
    timestamp: new Date().toISOString(),
    files,
    hasBlockingWarnings: result.hasBlockingWarnings,
    executionTimeMs: result.executionTimeMs,
    checksRun: result.checksRun,
    warnings: result.warnings.warnings.map((w) => ({
      id: w.id,
      category: w.category,
      severity: w.severity,
      title: w.title,
      message: w.message,
      file: w.location.file,
      line: w.location.line,
      suggestion: w.suggestion,
    })),
    summary: result.warnings.summary,
  };

  console.log(JSON.stringify(output, null, 2));
}

function formatResultsHuman(result: AnalyzeResult, verbose: boolean): void {
  const { warnings, summary } = result.warnings;

  if (warnings.length === 0) {
    success('No warnings found');
    return;
  }

  console.log(chalk.bold('\nWarnings:\n'));

  const errors = warnings.filter((w) => w.severity === 'error' && !w.suppressed);
  const warns = warnings.filter((w) => w.severity === 'warning' && !w.suppressed);
  const infos = warnings.filter((w) => w.severity === 'info' && !w.suppressed);

  if (errors.length > 0) {
    console.log(chalk.red.bold('Errors:'));
    errors.forEach((w) => formatWarning(w, verbose));
  }

  if (warns.length > 0) {
    console.log(chalk.yellow.bold('Warnings:'));
    warns.forEach((w) => formatWarning(w, verbose));
  }

  if (infos.length > 0 && verbose) {
    console.log(chalk.blue.bold('Info:'));
    infos.forEach((w) => formatWarning(w, verbose));
  }

  console.log(chalk.bold('Summary:'));
  console.log(`  Total: ${summary.total}`);
  if (summary.errors > 0) console.log(`  Errors: ${chalk.red(summary.errors)}`);
  if (summary.warnings > 0) console.log(`  Warnings: ${chalk.yellow(summary.warnings)}`);
  if (summary.info > 0) console.log(`  Info: ${chalk.blue(summary.info)}`);
  if (summary.suppressed > 0) console.log(`  Suppressed: ${chalk.gray(summary.suppressed)}`);
  console.log(`  Time: ${result.executionTimeMs}ms`);
}

export function createCheckCommand(): Command {
  const command = new Command('check');

  command
    .description('Analyse files for architecture violations and anti-patterns (planless mode)')
    .argument('[files...]', 'Files to analyse (optional if using --changed)')
    .option('-v, --verbose', 'Show detailed output including explanations and suggestions')
    .option('--json', 'Output results as JSON')
    .option('--no-cache', 'Disable caching')
    .option('--changed', 'Analyse git-changed files only')
    .option('--staged', 'With --changed, analyse only staged files')
    .option('--since <ref>', 'With --changed, compare against git ref (e.g., main, HEAD~3)')
    .action(async (files: string[], options: CheckOptions) => {
      const spinner = options.json ? null : ora('Analysing files...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        let filesToAnalyse = files;

        if (options.changed) {
          if (spinner) spinner.text = 'Detecting changed files...';

          const changedFiles = await getChangedFiles(workspaceRoot, {
            staged: options.staged ? true : !options.since,
            unstaged: options.staged ? false : !options.since,
            untracked: false,
            since: options.since,
            extensions: ANALYSABLE_EXTENSIONS,
          });

          if (changedFiles.length === 0) {
            spinner?.stop();
            if (options.json) {
              console.log(
                JSON.stringify(
                  {
                    version: '1.0.0',
                    timestamp: new Date().toISOString(),
                    files: [],
                    hasBlockingWarnings: false,
                    executionTimeMs: 0,
                    checksRun: [],
                    warnings: [],
                    summary: { total: 0, errors: 0, warnings: 0, info: 0, suppressed: 0 },
                    message: 'No changed files to analyse',
                  },
                  null,
                  2
                )
              );
            } else {
              info('No changed files to analyse');
            }
            process.exit(0);
          }

          filesToAnalyse = changedFiles;
          if (spinner) spinner.text = `Analysing ${changedFiles.length} changed file(s)...`;
        }

        if (filesToAnalyse.length === 0) {
          spinner?.stop();
          error('No files specified. Use --changed or provide file paths.');
          process.exit(1);
        }

        const gateRunner = new GateRunner();

        const cacheDisabled = options.cache === false;
        const cache = createCacheProvider({
          type: cacheDisabled ? 'null' : 'file',
          workspaceRoot,
          disabled: cacheDisabled,
        });

        const result = await gateRunner.analyzeFiles(filesToAnalyse, workspaceRoot, {
          cache,
          noCache: cacheDisabled,
          checks: ['architecture'],
        });

        spinner?.stop();

        if (options.json) {
          formatResultsJSON(filesToAnalyse, result);
        } else {
          if (options.changed) {
            console.log(chalk.gray(`\nChecked ${filesToAnalyse.length} changed file(s)\n`));
          }
          formatResultsHuman(result, options.verbose ?? false);

          if (options.verbose) {
            info(`Checks run: ${result.checksRun.join(', ')}`);
          }
        }

        if (result.hasBlockingWarnings) {
          if (!options.json) {
            error('Blocking warnings found (severity: error)');
          }
          process.exit(1);
        } else if (result.warnings.warnings.length > 0) {
          if (!options.json) {
            info('Warnings found but none are blocking');
          }
          process.exit(0);
        } else {
          process.exit(0);
        }
      } catch (err) {
        spinner?.fail('Analysis failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        process.exit(1);
      }
    });

  return command;
}
