/**
 * Watch Command - Monitor files for changes and run validation/gates
 */

import { Command } from 'commander';
import chalk from 'chalk';
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
} from '@anvil/core';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { PlanLoader } from '../services/plan-loader.js';
import { createWatchOutput } from '../services/watch-output.js';
import { error } from '../utils/output.js';

/**
 * Watch command options
 */
interface WatchOptions {
  /** Action to run: validate or gate */
  action?: 'validate' | 'gate';
  /** Glob patterns to watch (comma-separated) */
  patterns?: string;
  /** Patterns to exclude (comma-separated) */
  exclude?: string;
  /** Debounce interval in ms */
  debounce?: string;
  /** Include untracked files */
  includeUntracked?: boolean;
  /** Disable git filtering (watch all changes) */
  noGitFilter?: boolean;
  /** Gate profile to use */
  profile?: 'dev' | 'ci' | 'production';
  /** Verbose output */
  verbose?: boolean;
}

export function createWatchCommand(): Command {
  const command = new Command('watch');

  command
    .description('Watch files for changes and run validation or gates in real-time')
    .argument('[file]', 'Specific file to watch (optional)')
    .option('-a, --action <action>', 'Action to run: validate or gate', 'validate')
    .option('--patterns <patterns>', 'Glob patterns to watch (comma-separated)')
    .option('--exclude <patterns>', 'Patterns to exclude (comma-separated)')
    .option('--debounce <ms>', 'Debounce interval in milliseconds', '300')
    .option('--include-untracked', 'Include untracked git files in watch')
    .option('--no-git-filter', 'Disable git filtering (watch all file changes)')
    .option('-p, --profile <profile>', 'Gate profile to use (dev, ci, production)')
    .option('-v, --verbose', 'Verbose output')
    .action(async (file: string | undefined, options: WatchOptions) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);

        // Load watch config from .anvilrc or use defaults
        const savedConfig = configManager.getWatchConfig();
        const defaultConfig = getDefaultWatchConfig();

        // Build effective config (CLI options override file config)
        const watchConfig: WatchConfig = {
          enabled: true,
          patterns: options.patterns
            ? options.patterns.split(',').map((p) => p.trim())
            : (savedConfig?.patterns ?? DEFAULT_WATCH_PATTERNS),
          exclude: options.exclude
            ? options.exclude.split(',').map((p) => p.trim())
            : (savedConfig?.exclude ?? DEFAULT_EXCLUDE_PATTERNS),
          action: (options.action as 'validate' | 'gate') ?? savedConfig?.action ?? 'validate',
          debounceMs: options.debounce
            ? parseInt(options.debounce, 10)
            : (savedConfig?.debounceMs ?? defaultConfig.debounceMs),
          git: {
            unstagedOnly: options.noGitFilter !== true,
            includeUntracked:
              options.includeUntracked ?? savedConfig?.git.includeUntracked ?? false,
          },
          gateProfile: options.profile ?? savedConfig?.gateProfile,
        };

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

        // Create orchestrator
        const orchestrator = createWatchOrchestrator({
          workspaceRoot,
          config: watchConfig,
          onEvent: (event: WatchStatusEvent) => output.handleEvent(event),
          verbose: options.verbose,
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
          // For gate, we typically run on the first plan file
          const filePath = files[0];

          try {
            const loadResult = await planLoader.loadPlan(filePath, {
              validateHash: false,
              strict: false,
            });

            // Build gate options
            const gateOptions: { skipChecks?: string[] } = {};

            // Apply profile
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
