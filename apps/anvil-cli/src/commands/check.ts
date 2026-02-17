import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { glob } from 'glob';
import { createDebugger } from '@eddacraft/anvil-core';
import {
  GateRunner,
  createCacheProvider,
  getChangedFiles,
  type AnalyzeResult,
} from '@eddacraft/anvil-runtime';
import type { Warning } from '@eddacraft/anvil-core/antipattern';
import {
  DEFAULT_ANALYSABLE_EXTENSIONS,
  DEFAULT_NUDGE_CONFIG,
  meetsNudgeThreshold,
  type NudgeConfig,
  type NudgeSeverityThreshold,
} from '@eddacraft/anvil-core';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { saveRecentWarnings } from '../services/recent-warnings-store.js';
import { success, error, info } from '../utils/output.js';
import { initKindling, type KindlingContext } from '../services/kindling-bootstrap.js';
import {
  emitSessionStart,
  emitSessionEnd,
  emitGateEvaluated,
  emitError as emitKindlingError,
} from '@eddacraft/anvil-kindling-integration';

interface CheckOptions {
  verbose?: boolean;
  json?: boolean;
  cache?: boolean;
  changed?: boolean;
  staged?: boolean;
  since?: string;
  all?: boolean;
  extensions?: string;
  interactive?: boolean;
  nudge?: boolean;
  nudgeThreshold?: string;
}

interface JSONCheckOutput {
  version: '1.0.0';
  timestamp: string;
  files: string[];
  hasBlockingWarnings: boolean;
  executionTimeMs: number;
  checksRun: string[];
  provenance_id?: string;
  warnings: Array<{
    id: string;
    category: string;
    severity: string;
    title: string;
    message: string;
    file: string;
    line: number;
    suggestion: string;
    nudge?: string;
  }>;
  summary: {
    total: number;
    errors: number;
    warnings: number;
    info: number;
    suppressed: number;
  };
}

const log = createDebugger('cli');

/** Pattern IDs that have deterministic fixes available */
const FIXABLE_PATTERNS = new Set(['AP-001', 'AP-004']);

type InteractiveAction = 'skip' | 'fix' | 'suppress' | 'quit';

/**
 * Display a warning interactively and prompt for action.
 * Extracted for testability — accepts a prompt function.
 */
const VALID_THRESHOLDS: ReadonlySet<string> = new Set<NudgeSeverityThreshold>([
  'error',
  'warning',
  'info',
]);

function isNudgeSeverityThreshold(value: string): value is NudgeSeverityThreshold {
  return VALID_THRESHOLDS.has(value);
}

export async function promptForWarning(
  w: Warning,
  promptFn: (
    choices: Array<{ name: string; value: InteractiveAction }>
  ) => Promise<InteractiveAction>,
  showNudge = true
): Promise<InteractiveAction> {
  const icon = w.severity === 'error' ? '✗' : w.severity === 'warning' ? '⚠' : 'ℹ';

  console.log('');
  console.log(chalk.bold(`  ${icon} [${w.id}] ${w.title}`));
  console.log(chalk.gray(`    ${w.location.file}:${w.location.line}`));
  console.log(`    ${w.message}`);

  if (showNudge && w.nudge) {
    console.log(chalk.green(`\n    → ${w.nudge}`));
  }

  console.log('');

  const choices: Array<{ name: string; value: InteractiveAction }> = [
    { name: '[s]kip — move to next warning', value: 'skip' },
  ];

  if (FIXABLE_PATTERNS.has(w.id)) {
    choices.push({ name: '[f]ix — apply deterministic fix', value: 'fix' });
  }

  choices.push({ name: 's[u]ppress — add @anvil-ignore comment', value: 'suppress' });
  choices.push({ name: '[q]uit — stop reviewing', value: 'quit' });

  const answer = await promptFn(choices);
  return answer;
}

/**
 * Run interactive review loop over warnings.
 * Returns the list of actions taken.
 */
export async function runInteractiveReview(
  warnings: Warning[],
  promptFn?: (
    choices: Array<{ name: string; value: InteractiveAction }>
  ) => Promise<InteractiveAction>,
  showNudge = true
): Promise<Array<{ warning: Warning; action: InteractiveAction }>> {
  const results: Array<{ warning: Warning; action: InteractiveAction }> = [];

  // Default prompt uses inquirer
  const defaultPromptFn = async (
    choices: Array<{ name: string; value: InteractiveAction }>
  ): Promise<InteractiveAction> => {
    const inquirer = await import('inquirer');
    const { action } = await inquirer.default.prompt<{ action: string }>([
      {
        type: 'select',
        name: 'action',
        message: 'What would you like to do?',
        choices,
      },
    ]);
    return action as InteractiveAction;
  };

  const askFn = promptFn ?? defaultPromptFn;

  // Only review non-suppressed warnings
  const reviewable = warnings.filter((w) => !w.suppressed);

  if (reviewable.length === 0) {
    return results;
  }

  console.log(chalk.bold(`\n🔍 Interactive review: ${reviewable.length} warning(s) to review\n`));

  for (let i = 0; i < reviewable.length; i++) {
    const w = reviewable[i];
    console.log(chalk.gray(`  [${i + 1}/${reviewable.length}]`));

    const action = await promptForWarning(w, askFn, showNudge);
    results.push({ warning: w, action });

    if (action === 'quit') {
      console.log(chalk.gray('\n  Review stopped.'));
      break;
    }

    if (action === 'fix') {
      console.log(chalk.cyan(`    ℹ Fix for ${w.id} noted — apply fixes after review.`));
    }

    if (action === 'suppress') {
      console.log(
        chalk.yellow(
          `    ℹ Add \`// @anvil-ignore ${w.id}: <reason>\` above line ${w.location.line} in ${w.location.file}`
        )
      );
    }
  }

  // Summary
  const skipped = results.filter((r) => r.action === 'skip').length;
  const fixed = results.filter((r) => r.action === 'fix').length;
  const suppressed = results.filter((r) => r.action === 'suppress').length;

  console.log(chalk.bold('\n  Review summary:'));
  if (skipped > 0) console.log(`    Skipped: ${skipped}`);
  if (fixed > 0) console.log(`    To fix: ${fixed}`);
  if (suppressed > 0) console.log(`    To suppress: ${suppressed}`);
  console.log('');

  return results;
}

function formatWarning(w: Warning, verbose: boolean, nudgeConfig: NudgeConfig): void {
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
    if (
      w.nudge &&
      nudgeConfig.enabled &&
      meetsNudgeThreshold(w.severity, nudgeConfig.severityThreshold)
    ) {
      console.log(chalk.green(`    → ${w.nudge}`));
    }
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
    ...(result.provenance_id ? { provenance_id: result.provenance_id } : {}),
    warnings: result.warnings.warnings.map((w) => ({
      id: w.id,
      category: w.category,
      severity: w.severity,
      title: w.title,
      message: w.message,
      file: w.location.file,
      line: w.location.line,
      suggestion: w.suggestion,
      ...(w.nudge ? { nudge: w.nudge } : {}),
    })),
    summary: result.warnings.summary,
  };

  console.log(JSON.stringify(output, null, 2));
}

function formatResultsHuman(
  result: AnalyzeResult,
  verbose: boolean,
  nudgeConfig: NudgeConfig
): void {
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
    errors.forEach((w) => formatWarning(w, verbose, nudgeConfig));
  }

  if (warns.length > 0) {
    console.log(chalk.yellow.bold('Warnings:'));
    warns.forEach((w) => formatWarning(w, verbose, nudgeConfig));
  }

  if (infos.length > 0 && verbose) {
    console.log(chalk.blue.bold('Info:'));
    infos.forEach((w) => formatWarning(w, verbose, nudgeConfig));
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
    .option('-i, --interactive', 'Review each warning interactively with nudge coaching')
    .option('--no-nudge', 'Disable coaching nudges')
    .option(
      '--nudge-threshold <level>',
      'Minimum severity for nudges: error, warning, info (default: warning)'
    )
    .action(async (files: string[], options: CheckOptions) => {
      log(
        `check command entered: files=${files.length} all=${options.all} changed=${options.changed} staged=${options.staged}`
      );
      const spinner = options.json ? null : ora('Analysing files...').start();
      const startTime = Date.now();
      let kindling: KindlingContext | null = null;
      let sessionId: string | undefined;
      let capsuleId: string | undefined;

      try {
        const workspaceRoot = getWorkspaceRoot();

        // Validate --nudge-threshold if provided
        if (options.nudgeThreshold && !isNudgeSeverityThreshold(options.nudgeThreshold)) {
          spinner?.stop();
          error(
            `Invalid --nudge-threshold "${options.nudgeThreshold}". ` +
              `Allowed values: error, warning, info`
          );
          process.exit(1);
        }

        // Resolve nudge configuration: CLI flags override defaults
        const nudgeConfig: NudgeConfig = {
          ...DEFAULT_NUDGE_CONFIG,
          ...(options.nudge === false ? { enabled: false } : {}),
          ...(options.nudgeThreshold && isNudgeSeverityThreshold(options.nudgeThreshold)
            ? { severityThreshold: options.nudgeThreshold }
            : {}),
        };

        // --interactive flag or config default
        const useInteractive = options.interactive ?? (nudgeConfig.interactive && !options.json);
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

        // Initialize Kindling provenance recording (after arg validation, no-op when disabled)
        kindling = initKindling(workspaceRoot);
        if (kindling) {
          sessionId = emitSessionStart(kindling.service, {
            working_directory: workspaceRoot,
            anvil_version: '0.1.0',
            command: 'check',
            args: filesToAnalyse,
            environment: process.env.CI ? 'ci' : 'development',
          });
          const capsule = kindling.adapter.startSession(sessionId, 'anvil check');
          capsuleId = capsule.id;
          kindling.bridge.setCapsuleId(capsuleId);
        }

        const gateRunner = new GateRunner();

        const cacheDisabled = options.cache === false;
        const cache = createCacheProvider({
          type: cacheDisabled ? 'null' : 'file',
          workspaceRoot,
          disabled: cacheDisabled,
        });

        // Determine provenance scope
        const provenanceScope = options.staged
          ? ('staged' as const)
          : options.changed
            ? ('files' as const)
            : options.all
              ? ('directory' as const)
              : ('files' as const);

        const provenanceTrigger = process.env.CI
          ? ('ci' as const)
          : process.env.ANVIL_TRIGGER === 'pre-commit'
            ? ('pre-commit' as const)
            : ('manual' as const);

        log(`check: analysing ${filesToAnalyse.length} files with scope=${provenanceScope}`);
        const result = await gateRunner.analyzeFiles(filesToAnalyse, workspaceRoot, {
          cache,
          noCache: cacheDisabled,
          checks: ['architecture'],
          provenance: {
            enabled: true,
            trigger: provenanceTrigger,
            scope: provenanceScope,
          },
        });
        log('check result', {
          warnings: result.warnings.warnings.length,
          blocking: result.hasBlockingWarnings,
          checksRun: result.checksRun,
        });

        // Record gate evaluation in Kindling
        if (kindling && sessionId) {
          emitGateEvaluated(kindling.service, {
            session_id: sessionId,
            gate_id: 'architecture-check',
            inputs: {
              file_count: filesToAnalyse.length,
              changed_files: filesToAnalyse.slice(0, 50),
            },
            outcome: result.hasBlockingWarnings ? 'fail' : 'pass',
            rules_evaluated: result.checksRun,
            rules_violated: result.hasBlockingWarnings
              ? result.warnings.warnings
                  .filter((w) => w.severity === 'error' && !w.suppressed)
                  .map((w) => w.id)
              : undefined,
            enforcement: 'blocking',
            duration_ms: result.executionTimeMs,
            violation_count: result.warnings.summary.errors,
            warning_count: result.warnings.summary.warnings,
          });
        }

        try {
          await saveRecentWarnings(workspaceRoot, result.warnings.warnings);
        } catch (saveErr) {
          // Write to stderr to avoid polluting --json stdout output
          process.stderr.write(
            `ℹ Failed to save recent warnings: ${
              saveErr instanceof Error ? saveErr.message : String(saveErr)
            }\n`
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
          formatResultsHuman(result, options.verbose ?? false, nudgeConfig);

          if (options.verbose) {
            info(`Checks run: ${result.checksRun.join(', ')}`);
          }

          if (result.provenance_id) {
            console.log(chalk.gray(`\nProvenance: ${result.provenance_id}`));
          }
        }

        // Interactive mode: review each warning with actions and optional nudge coaching
        if (useInteractive && !options.json && result.warnings.warnings.length > 0) {
          if (!process.stdout.isTTY || !process.stdin.isTTY) {
            info('--interactive ignored: not a TTY environment');
          } else {
            // Filter warnings by nudge severity threshold for interactive review
            const reviewableWarnings = nudgeConfig.enabled
              ? result.warnings.warnings.filter((w) =>
                  meetsNudgeThreshold(w.severity, nudgeConfig.severityThreshold)
                )
              : result.warnings.warnings.filter((w) => !w.suppressed);
            if (reviewableWarnings.length > 0) {
              await runInteractiveReview(reviewableWarnings, undefined, nudgeConfig.enabled);
            }
          }
        }

        const exitCode = result.hasBlockingWarnings ? 1 : 0;

        // Record session end in Kindling
        if (kindling && sessionId) {
          emitSessionEnd(kindling.service, sessionId, {
            outcome: result.hasBlockingWarnings ? 'failure' : 'success',
            exit_code: exitCode,
            duration_ms: Date.now() - startTime,
            summary: {
              gates_evaluated: 1,
              gates_passed: result.hasBlockingWarnings ? 0 : 1,
              gates_failed: result.hasBlockingWarnings ? 1 : 0,
              actions_executed: 0,
              errors_encountered: 0,
            },
          });
          if (capsuleId) kindling.adapter.endSession(capsuleId);
          kindling.close();
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
        // Record error and session end in Kindling
        if (kindling && sessionId) {
          emitKindlingError(kindling.service, {
            session_id: sessionId,
            error_type: 'command_failure',
            context: { component: 'check' },
            error_message: err instanceof Error ? err.message : String(err),
            exit_code: 1,
            recoverable: false,
          });
          emitSessionEnd(kindling.service, sessionId, {
            outcome: 'failure',
            exit_code: 1,
            duration_ms: Date.now() - startTime,
            summary: {
              gates_evaluated: 0,
              gates_passed: 0,
              gates_failed: 0,
              actions_executed: 0,
              errors_encountered: 1,
            },
          });
          if (capsuleId) kindling.adapter.endSession(capsuleId);
          kindling.close();
        }

        spinner?.fail('Analysis failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        process.exit(1);
      }
    });

  return command;
}
