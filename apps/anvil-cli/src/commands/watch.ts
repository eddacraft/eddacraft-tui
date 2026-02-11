/**
 * Watch Command - Monitor files for changes and run validation/gates
 */

import { Command } from 'commander';
import chalk from 'chalk';
import inquirer from 'inquirer';
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
import { error } from '../utils/output.js';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';

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
  // Commander.js --no-tui sets options.tui = false (not options.noTui = true)
  tui?: boolean;
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
    .option('--debounce <ms>', 'Debounce interval in milliseconds', '300')
    .option('--include-untracked', 'Include untracked git files in watch')
    .option('--no-git-filter', 'Disable git filtering (watch all file changes)')
    .option('-p, --profile <profile>', 'Gate profile to use (dev, ci, production)')
    .option('-v, --verbose', 'Verbose output')
    .option('--tui', 'Force TUI dashboard mode')
    .option('--no-tui', 'Force plain text mode')
    .option('--multi-agent', 'Enable multi-agent coordination (default: true)')
    .option('--no-multi-agent', 'Disable multi-agent coordination')
    .option('--agent-id <id>', 'Custom agent identifier')
    .option('--no-exclusive', 'Allow multiple watch instances (disable exclusive lock)')
    .action(async (file: string | undefined, options: WatchOptions) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);

        const savedConfig = configManager.getWatchConfig();
        const defaultConfig = getDefaultWatchConfig();

        // Determine watch mode
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
        const watchConfig = {
          enabled: true,
          patterns,
          exclude: excludePatterns,
          action,
          debounceMs: (() => {
            if (!options.debounce) return savedConfig?.debounceMs ?? defaultConfig.debounceMs;
            const val = parseInt(options.debounce, 10);
            if (Number.isNaN(val) || val < 0) {
              throw new Error('--debounce must be a non-negative integer');
            }
            return val;
          })(),
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

        const useTUI = isTUIAvailable({ tui: options.tui });
        if (useTUI && options.tui) {
          console.log(
            chalk.yellow(
              '⚠  --tui flag: Watch dashboard TUI not yet integrated. Using standard output.'
            )
          );
          console.log(
            chalk.gray('   Dashboard components available at cli/src/tui/commands/watch/')
          );
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
          console.log(chalk.gray('\n  Stopping watch mode...'));
          await orchestrator.stop();
          process.exit(0);
        };

        process.on('SIGINT', shutdown);
        process.on('SIGTERM', shutdown);

        // Start watching
        await orchestrator.start();
        output.showWatching();

        // Keep process alive
        await new Promise(() => {
          // Never resolves - keeps process running until Ctrl+C
        });
      } catch (err) {
        error(`Watch failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
        process.exit(1);
      }
    });

  return command;
}
