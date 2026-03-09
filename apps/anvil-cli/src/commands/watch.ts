/**
 * Watch Command - Monitor files for changes and run validation/gates
 */

import { Command } from 'commander';
import chalk from 'chalk';
import inquirer from 'inquirer';
import { createDebugger } from '@eddacraft/anvil-core';
import {
  GateRunner,
  GateConfigManager,
  createWatchOrchestrator,
  getDefaultWatchConfig,
  DEFAULT_WATCH_PATTERNS,
  DEFAULT_EXCLUDE_PATTERNS,
  type WatchConfig,
  type WatchActionResult,
  type WatchStatusEvent,
  type GateRunResultWithCache,
} from '@eddacraft/anvil-runtime';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { PlanLoader } from '../services/plan-loader.js';
import { createWatchOutput } from '../services/watch-output.js';
import { error, print } from '../utils/output.js';
import { CliError, CliExit } from '../utils/cli-error.js';
import { coerceNonNegativeInt } from '../utils/option-coerce.js';
import { initKindling, type KindlingContext } from '../services/kindling-bootstrap.js';
import {
  emitSessionStart,
  emitSessionEnd,
  emitGateEvaluated,
  emitError as emitKindlingError,
} from '@eddacraft/anvil-kindling-integration';

const log = createDebugger('cli');

const SOURCE_WATCH_PATTERNS = ['src/**/*.ts', 'src/**/*.tsx', 'lib/**/*.ts', '**/*.ts', '**/*.tsx'];
const SOURCE_EXCLUDE_PATTERNS = [
  'node_modules/**',
  'dist/**',
  'build/**',
  '.git/**',
  'coverage/**',
  '**/*.test.ts',
  '**/*.spec.ts',
  '**/__tests__/**',
  '**/*.d.ts',
];

type WatchMode = 'plans' | 'source' | 'all';

interface WatchOptions {
  action?: 'validate' | 'gate' | 'check';
  patterns?: string;
  exclude?: string;
  debounce?: string;
  includeUntracked?: boolean;
  // Commander.js --no-git-filter sets options.gitFilter = false
  gitFilter?: boolean;
  profile?: 'dev' | 'ci' | 'production';
  verbose?: boolean;
  source?: boolean;
  plans?: boolean;
  all?: boolean;
  // Multi-agent coordination options
  multiAgent?: boolean;
  agentId?: string;
  exclusive?: boolean;
}

/**
 * Prompt user to select watch mode interactively
 */
async function promptWatchMode(): Promise<WatchMode> {
  const { mode } = await inquirer.prompt<{ mode: WatchMode }>([
    {
      type: 'select',
      name: 'mode',
      message: 'What would you like to watch?',
      choices: [
        {
          name: 'Planning documents (*.md, prd.*, plan.*, spec.*, etc.)',
          value: 'plans',
        },
        {
          name: 'Source files (*.ts, *.tsx) — anti-patterns & architecture',
          value: 'source',
        },
        {
          name: 'Everything (plans + source files)',
          value: 'all',
        },
      ],
    },
  ]);
  return mode;
}

export function createWatchCommand(): Command {
  const command = new Command('watch');

  command
    .description('Watch files for changes and run validation or gates in real-time')
    .argument('[file]', 'Specific file to watch (optional)')
    .option('-a, --action <action>', 'Action to run: validate, gate, or check')
    .option('--plans', 'Watch planning documents (*.md, prd.*, plan.*, etc.)')
    .option('--source', 'Watch source files and run checks (anti-patterns, architecture)')
    .option('--all', 'Watch both planning documents and source files')
    .option('--patterns <patterns>', 'Glob patterns to watch (comma-separated)')
    .option('--exclude <patterns>', 'Patterns to exclude (comma-separated)')
    .option('--debounce <ms>', 'Debounce interval in milliseconds (defaults to config, 300ms if unset)')
    .option('--include-untracked', 'Include untracked git files in watch')
    .option('--no-git-filter', 'Disable git filtering (watch all file changes)')
    .option('-p, --profile <profile>', 'Gate profile to use (dev, ci, production)')
    .option('-v, --verbose', 'Verbose output')
    .option('--multi-agent', 'Enable multi-agent coordination')
    .option('--no-multi-agent', 'Disable multi-agent coordination')
    .option('--agent-id <id>', 'Custom agent identifier')
    .option('--no-exclusive', 'Allow multiple watch instances (disable exclusive lock)')
    .action(async (file: string | undefined, options: WatchOptions) => {
      log(
        `watch command entered: file=${file ?? '(none)'} source=${options.source} plans=${options.plans} all=${options.all}`
      );
      const startTime = Date.now();
      let kindling: KindlingContext | null = null;
      let sessionId: string | undefined;
      let capsuleId: string | undefined;
      let gatesEvaluated = 0;
      let gatesPassed = 0;
      let gatesFailed = 0;
      let errorsEncountered = 0;

      try {
        const workspaceRoot = getWorkspaceRoot();

        // Initialize Kindling provenance recording
        kindling = initKindling(workspaceRoot);
        if (kindling) {
          sessionId = emitSessionStart(kindling.service, {
            working_directory: workspaceRoot,
            anvil_version: '0.1.0',
            command: 'watch',
            args: file ? [file] : [],
            environment: process.env.CI ? 'ci' : 'development',
          });
          const capsule = kindling.adapter.startSession(sessionId, 'anvil watch');
          capsuleId = capsule.id;
          kindling.bridge.setCapsuleId(capsuleId);
        }

        const configManager = new GateConfigManager(workspaceRoot);

        const savedConfig = configManager.getWatchConfig();
        const defaultConfig = getDefaultWatchConfig();

        // Determine watch mode
        log('watch: determining watch mode');
        let watchMode: WatchMode;
        const hasExplicitMode =
          options.source || options.plans || options.all || options.patterns || file;

        if (options.all) {
          watchMode = 'all';
        } else if (options.source) {
          watchMode = 'source';
        } else if (options.plans || options.patterns || file) {
          watchMode = 'plans';
        } else if (!hasExplicitMode && process.stdin.isTTY) {
          // Interactive mode - prompt user to choose
          watchMode = await promptWatchMode();
        } else {
          // Non-interactive fallback - default to plans
          watchMode = 'plans';
        }

        // Determine patterns based on mode
        let patterns: string[];
        let excludePatterns: string[];
        let action: 'validate' | 'gate' | 'check';

        if (options.patterns) {
          patterns = options.patterns.split(',').map((p) => p.trim());
          excludePatterns = options.exclude
            ? options.exclude.split(',').map((p) => p.trim())
            : DEFAULT_EXCLUDE_PATTERNS;
          action = (options.action as 'validate' | 'gate' | 'check') ?? 'validate';
        } else {
          switch (watchMode) {
            case 'source':
              patterns = SOURCE_WATCH_PATTERNS;
              excludePatterns = SOURCE_EXCLUDE_PATTERNS;
              action = options.action ? (options.action as 'validate' | 'gate' | 'check') : 'check';
              break;
            case 'all':
              patterns = [...DEFAULT_WATCH_PATTERNS, ...SOURCE_WATCH_PATTERNS];
              excludePatterns = SOURCE_EXCLUDE_PATTERNS; // Use stricter excludes
              action = options.action ? (options.action as 'validate' | 'gate' | 'check') : 'check';
              break;
            case 'plans':
            default:
              patterns = savedConfig?.patterns ?? DEFAULT_WATCH_PATTERNS;
              excludePatterns = options.exclude
                ? options.exclude.split(',').map((p) => p.trim())
                : (savedConfig?.exclude ?? DEFAULT_EXCLUDE_PATTERNS);
              action =
                (options.action as 'validate' | 'gate' | 'check') ??
                savedConfig?.action ??
                'validate';
              break;
          }
        }

        // Build effective config
        const debounceMs = options.debounce
          ? coerceNonNegativeInt(options.debounce, '--debounce')
          : (savedConfig?.debounceMs ?? defaultConfig.debounceMs);

        const watchConfig = {
          enabled: true,
          patterns,
          exclude: excludePatterns,
          action,
          debounceMs,
          git: {
            unstagedOnly: options.gitFilter !== false,
            includeUntracked:
              options.includeUntracked ??
              savedConfig?.git?.includeUntracked ??
              defaultConfig.git.includeUntracked,
          },
          gateProfile: options.profile ?? savedConfig?.gateProfile,
        } satisfies WatchConfig;

        // If a specific file is provided, watch only that file
        if (file) {
          watchConfig.patterns = [file];
        }

        // Create output handler
        const output = createWatchOutput({ verbose: options.verbose });

        // Show header
        output.showHeader({
          patterns: watchConfig.patterns,
          action: watchConfig.action,
          gitFilter: watchConfig.git.unstagedOnly,
          profile: watchConfig.gateProfile,
        });

        // Create orchestrator with multi-agent support
        const orchestrator = createWatchOrchestrator({
          workspaceRoot,
          config: watchConfig,
          onEvent: (event: WatchStatusEvent) => output.handleEvent(event),
          verbose: options.verbose,
          multiAgent: {
            enabled: options.multiAgent !== false,
            exclusiveWatch: options.exclusive !== false,
            coordinatedActions: options.multiAgent !== false,
            agentId: options.agentId,
            waitForLock: true,
          },
        });

        // Set up action handlers
        const planLoader = new PlanLoader();
        const gateRunner = new GateRunner();
        const gateConfig = configManager.loadConfig();

        // Validate handler
        orchestrator.setValidateHandler(async (files: string[]): Promise<WatchActionResult> => {
          const errors: string[] = [];
          let success = true;

          for (const filePath of files) {
            try {
              await planLoader.loadPlan(filePath, {
                validateHash: false,
                strict: false,
              });
            } catch (err) {
              success = false;
              errors.push(err instanceof Error ? err.message : String(err));
            }
          }

          return {
            success,
            action: 'validate',
            files,
            executionTimeMs: 0,
            error: errors.length > 0 ? errors.join('; ') : undefined,
          };
        });

        // Gate handler
        orchestrator.setGateHandler(async (files: string[]): Promise<WatchActionResult> => {
          const filePath = files[0];

          try {
            const loadResult = await planLoader.loadPlan(filePath, {
              validateHash: false,
              strict: false,
            });

            const gateOptions: { skipChecks?: string[] } = {};

            if (watchConfig.gateProfile === 'dev') {
              gateOptions.skipChecks = ['coverage', 'dependency'];
            }

            const results: GateRunResultWithCache = await gateRunner.runGate(
              loadResult.plan,
              gateConfig,
              workspaceRoot,
              gateOptions
            );

            // Record in Kindling
            gatesEvaluated++;
            if (results.overall) {
              gatesPassed++;
            } else {
              gatesFailed++;
            }
            if (kindling && sessionId) {
              emitGateEvaluated(kindling.service, {
                session_id: sessionId,
                gate_id: loadResult.plan.id ?? 'watch-gate',
                inputs: { file_count: files.length, changed_files: files.slice(0, 50) },
                outcome: results.overall ? 'pass' : 'fail',
                rules_evaluated: results.checks.map((c) => c.check),
                enforcement: 'blocking',
                duration_ms: results.timing?.totalMs ?? 0,
                violation_count: results.checks.filter((c) => !c.passed && !c.skipped).length,
                warning_count: results.checks.filter(
                  (c) => c.passed && c.score !== undefined && c.score < 1
                ).length,
              });
            }

            return {
              success: results.overall,
              action: 'gate',
              files,
              executionTimeMs: results.timing?.totalMs ?? 0,
              details: {
                score: results.score,
                checks: results.checks,
                summary: results.summary,
              },
            };
          } catch (err) {
            errorsEncountered++;
            if (kindling && sessionId) {
              emitKindlingError(kindling.service, {
                session_id: sessionId,
                error_type: 'command_failure',
                context: { component: 'watch-gate' },
                error_message: err instanceof Error ? err.message : String(err),
                recoverable: true,
              });
            }
            return {
              success: false,
              action: 'gate',
              files,
              executionTimeMs: 0,
              error: err instanceof Error ? err.message : String(err),
            };
          }
        });

        // Check handler (for source mode)
        orchestrator.setCheckHandler(async (files: string[]): Promise<WatchActionResult> => {
          try {
            const result = await gateRunner.analyzeFiles(files, workspaceRoot, {
              checks: ['architecture', 'antipattern'],
              suppressions: true,
            });

            // Record in Kindling
            gatesEvaluated++;
            if (result.hasBlockingWarnings) {
              gatesFailed++;
            } else {
              gatesPassed++;
            }
            if (kindling && sessionId) {
              emitGateEvaluated(kindling.service, {
                session_id: sessionId,
                gate_id: 'watch-check',
                inputs: { file_count: files.length, changed_files: files.slice(0, 50) },
                outcome: result.hasBlockingWarnings ? 'fail' : 'pass',
                rules_evaluated: result.checksRun,
                enforcement: 'warning',
                duration_ms: result.executionTimeMs,
                violation_count: result.warnings.summary.errors,
                warning_count: result.warnings.summary.warnings,
              });
            }

            return {
              success: !result.hasBlockingWarnings,
              action: 'check',
              files,
              executionTimeMs: result.executionTimeMs,
              details: {
                warnings: result.warnings.warnings,
                summary: result.warnings.summary,
                checksRun: result.checksRun,
              },
            };
          } catch (err) {
            errorsEncountered++;
            if (kindling && sessionId) {
              emitKindlingError(kindling.service, {
                session_id: sessionId,
                error_type: 'command_failure',
                context: { component: 'watch-check' },
                error_message: err instanceof Error ? err.message : String(err),
                recoverable: true,
              });
            }
            return {
              success: false,
              action: 'check',
              files,
              executionTimeMs: 0,
              error: err instanceof Error ? err.message : String(err),
            };
          }
        });

        // Handle graceful shutdown
        const shutdown = async () => {
          print(chalk.gray('\n  Stopping watch mode...'));

          // Record session end in Kindling
          if (kindling && sessionId) {
            emitSessionEnd(kindling.service, sessionId, {
              outcome: gatesFailed > 0 ? 'partial' : 'success',
              exit_code: 0,
              duration_ms: Date.now() - startTime,
              summary: {
                gates_evaluated: gatesEvaluated,
                gates_passed: gatesPassed,
                gates_failed: gatesFailed,
                actions_executed: 0,
                errors_encountered: errorsEncountered,
              },
            });
            if (capsuleId) kindling.adapter.endSession(capsuleId);
            kindling.close();
          }

          try {
            await orchestrator.stop();
          } finally {
            // Signal handlers run outside the main async flow, so CliExit
            // would become an unhandled rejection. process.exit(0) is correct here.
            // try/finally ensures exit even if orchestrator.stop() rejects.
            process.exit(0);
          }
        };

        process.on('SIGINT', shutdown);
        process.on('SIGTERM', shutdown);

        // Start watching
        log('watch: starting orchestrator', { mode: watchMode, action, patterns });
        await orchestrator.start();
        output.showWatching();

        // Keep process alive
        await new Promise<void>(() => {
          // Intentionally never call resolve - keeps process running until Ctrl+C
        });
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        // Record error and session end in Kindling
        if (kindling && sessionId) {
          emitKindlingError(kindling.service, {
            session_id: sessionId,
            error_type: 'command_failure',
            context: { component: 'watch' },
            error_message: err instanceof Error ? err.message : String(err),
            exit_code: 1,
            recoverable: false,
          });
          emitSessionEnd(kindling.service, sessionId, {
            outcome: 'failure',
            exit_code: 1,
            duration_ms: Date.now() - startTime,
            summary: {
              gates_evaluated: gatesEvaluated,
              gates_passed: gatesPassed,
              gates_failed: gatesFailed,
              actions_executed: 0,
              errors_encountered: errorsEncountered + 1,
            },
          });
          if (capsuleId) kindling.adapter.endSession(capsuleId);
          kindling.close();
        }

        error(`Watch failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      }
    });

  return command;
}
