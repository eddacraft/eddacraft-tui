import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { glob } from 'glob';
import {
  GateRunner,
  createCacheProvider,
  getChangedFiles,
  type AnalyzeResult,
} from '@eddacraft/anvil-runtime';
import type { Warning } from '@eddacraft/anvil-core/antipattern';
import { DEFAULT_ANALYSABLE_EXTENSIONS } from '@eddacraft/anvil-platform-config';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { saveRecentWarnings } from '../services/recent-warnings-store.js';
import { success, error, info } from '../utils/output.js';

interface CheckOptions {
  verbose?: boolean;
  json?: boolean;
  cache?: boolean;
  changed?: boolean;
  staged?: boolean;
  since?: string;
  all?: boolean;
  extensions?: string;
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

async function getSourceFiles(
  workspaceRoot: string,
  extensions: string[] = DEFAULT_ANALYSABLE_EXTENSIONS
): Promise<string[]> {
  const patterns = extensions.map((ext) => `**/*${ext}`);
  const ignorePatterns = ['**/node_modules/**', '**/dist/**', '**/build/**', '**/.git/**'];

  const files: string[] = [];
  for (const pattern of patterns) {
    const matches = await glob(pattern, {
      cwd: workspaceRoot,
      ignore: ignorePatterns,
      nodir: true,
    });
    files.push(...matches);
  }

  return [...new Set(files)].sort();
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
    .option('--all', 'Analyse all source files in the project')
    .option(
      '--extensions <list>',
      'Comma-separated file extensions to analyse (e.g., .ts,.tsx,.html)'
    )
    .action(async (files: string[], options: CheckOptions) => {
      const spinner = options.json ? null : ora('Analysing files...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const activeExtensions = options.extensions
          ? [
              ...new Set(
                options.extensions
                  .split(',')
                  .map((e) => e.trim().toLowerCase())
                  .filter((e) => e.length > 0)
                  .map((e) => (e.startsWith('.') ? e : `.${e}`))
              ),
            ]
          : DEFAULT_ANALYSABLE_EXTENSIONS;
        let filesToAnalyse = files;

        if (options.all && options.changed) {
          spinner?.stop();
          error('Cannot use --all and --changed together. Choose one.');
          process.exit(1);
        }

        if (options.all) {
          if (spinner) spinner.text = 'Gathering all source files...';

          const allFiles = await getSourceFiles(workspaceRoot, activeExtensions);

          if (allFiles.length === 0) {
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
                    message: 'No source files found',
                  },
                  null,
                  2
                )
              );
            } else {
              info('No source files found');
            }
            process.exit(0);
          }

          filesToAnalyse = allFiles;
          if (spinner) spinner.text = `Analysing ${allFiles.length} file(s)...`;
        } else if (options.changed) {
          if (spinner) spinner.text = 'Detecting changed files...';

          const changedFiles = await getChangedFiles(workspaceRoot, {
            staged: options.staged ? true : !options.since,
            unstaged: options.staged ? false : !options.since,
            untracked: false,
            since: options.since,
            extensions: activeExtensions,
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
          error('No files specified. Use --all, --changed, or provide file paths.');
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

        try {
          await saveRecentWarnings(workspaceRoot, result.warnings.warnings);
        } catch (saveErr) {
          info(
            `Failed to save recent warnings: ${
              saveErr instanceof Error ? saveErr.message : String(saveErr)
            }`
          );
        }

        spinner?.stop();

        if (options.json) {
          formatResultsJSON(filesToAnalyse, result);
        } else {
          if (options.all) {
            console.log(chalk.gray(`\nChecked ${filesToAnalyse.length} file(s)\n`));
          } else if (options.changed) {
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
