/**
 * Gate Command - Run quality gates on APS plans and external formats
 */

import { Command } from 'commander';
import chalk from 'chalk';
import {
  GateRunner,
  GateConfigManager,
  createCacheProvider,
  type GateRunOptions,
  type ProgressEvent,
  type GateRunResultWithCache,
} from '@anvil/core';
import { loadPlan, findPlanById, getWorkspaceRoot } from '../utils/file-io.js';
import { PlanLoader } from '../services/plan-loader.js';
import { EvidenceWriter } from '../services/evidence-writer.js';
import type { GateOptions, GateProfile } from '../types/command-options.js';
import { success, error, formatGateResults, info, formatGateResultsJSON } from '../utils/output.js';
import ora from 'ora';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUIAndWait } from '../tui/utils/renderer.js';
import { GateExplorer } from '../tui/commands/gate/index.js';
import type {
  GateResult as TUIGateResult,
  CheckResult as TUICheckResult,
} from '../tui/commands/gate/types.js';

/**
 * Predefined gate profiles for different environments
 */
const GATE_PROFILES: Record<GateProfile, { skipChecks?: string[]; description: string }> = {
  dev: {
    skipChecks: ['coverage', 'dependency'],
    description: 'Development mode - skips coverage and dependency checks for faster iteration',
  },
  ci: {
    skipChecks: [],
    description: 'CI mode - runs all checks',
  },
  production: {
    skipChecks: [],
    description: 'Production mode - runs all checks with strict thresholds',
  },
};

function convertToTUIGateResult(
  results: GateRunResultWithCache,
  planId: string,
  planPath?: string
): TUIGateResult {
  const checks: TUICheckResult[] = results.checks.map((check) => {
    const details: string[] = [];

    if (check.error) {
      details.push(check.error);
    }

    const warnings = check.details?.warnings?.warnings ?? [];
    for (const w of warnings) {
      const loc = w.location ? ` (${w.location.file}:${w.location.line})` : '';
      details.push(`[${w.id}] ${w.message}${loc}`);
    }

    const hasNonBlockingWarnings = warnings.some((w) => w.severity !== 'error' && !w.suppressed);
    const hasBlockingIssues = !check.passed && !check.skipped;

    let status: TUICheckResult['status'];
    if (check.skipped) {
      status = 'skipped';
    } else if (hasBlockingIssues) {
      status = 'failed';
    } else if (hasNonBlockingWarnings) {
      status = 'warning';
    } else {
      status = 'passed';
    }

    return {
      id: check.check,
      name: check.check,
      status,
      score: check.score ?? 0,
      message: check.message ?? '',
      details: details.length > 0 ? details : undefined,
      duration: undefined,
      category: undefined,
    };
  });

  return {
    planId,
    planPath,
    overall: results.overall,
    score: results.score,
    checks,
    duration: results.timing?.totalMs ?? 0,
    timestamp: new Date(),
  };
}

function parseSkipGatesEnv(): string[] {
  const envValue = process.env.ANVIL_SKIP_GATES;
  if (!envValue) return [];
  return envValue
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

export function createGateCommand(): Command {
  const command = new Command('gate');

  command
    .description('Run quality gates on the codebase, optionally scoped to a plan')
    .argument('[plan]', 'Optional plan ID or file path (if omitted, scans entire codebase)')
    .option('-c, --config <path>', 'Custom config file path')
    .option('-v, --verbose', 'Verbose output')
    .option('--format <format>', 'Explicitly specify input format (bypasses auto-detection)')
    .option('--native', 'Skip format detection and treat as native APS')
    .option('--inject', 'Inject evidence back into source document (SpecKit, BMAD)')
    .option('--skip-checks <checks>', 'Comma-separated list of checks to skip')
    .option('--only-checks <checks>', 'Only run specified checks (comma-separated)')
    .option('--fail-fast', 'Stop on first check failure')
    .option('-p, --profile <profile>', 'Use predefined profile (dev, ci, production)')
    .option('--list-profiles', 'List available gate profiles')
    .option('--no-cache', 'Disable caching (always run checks fresh)')
    .option('--parallel <limit>', 'Limit parallel check execution (0 = sequential)')
    .option('-o, --output <format>', 'Output format: human (default) or json', 'human')
    .option('--progress', 'Show real-time progress for each check')
    .option('--tui', 'Show interactive explorer after gate execution')
    .option('--no-tui', 'Force plain text mode')
    .action(async (planArg: string | undefined, options: GateOptions) => {
      // Handle --list-profiles
      if (options.listProfiles) {
        console.log(chalk.bold('\nAvailable Gate Profiles:\n'));
        for (const [name, profile] of Object.entries(GATE_PROFILES)) {
          console.log(chalk.cyan(`  ${name}`));
          console.log(chalk.gray(`    ${profile.description}`));
          if (profile.skipChecks && profile.skipChecks.length > 0) {
            console.log(chalk.gray(`    Skips: ${profile.skipChecks.join(', ')}`));
          }
          console.log('');
        }
        console.log(chalk.gray('Usage: anvil gate [plan] --profile=dev'));
        process.exit(0);
      }
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);
        const gateRunner = new GateRunner();

        const spinner = ora().start();
        let plan;
        let sourceFormat;
        let planPath: string | undefined;
        let isFullScan = false;

        // Handle case when no plan is provided (full codebase scan)
        if (!planArg) {
          spinner.text = 'Running full codebase scan...';
          isFullScan = true;

          // Create a minimal plan for full codebase scanning
          const now = new Date().toISOString();
          plan = {
            schema_version: '0.1.0' as const,
            id: 'aps-00000000',
            hash: '0'.repeat(64),
            intent: 'Full codebase quality gate run without a specific plan',
            proposed_changes: [],
            provenance: {
              timestamp: now,
              source: 'cli' as const,
              version: '0.0.1',
            },
            validations: {
              required_checks: [],
              skip_checks: [],
            },
          };

          spinner.succeed('Full codebase scan mode');
        } else {
          // Resolve plan path
          if (planArg.startsWith('aps-') && planArg.length === 12) {
            // Plan ID
            const foundPath = findPlanById(planArg, workspaceRoot);
            if (!foundPath) {
              error(`Plan not found: ${planArg}`);
              process.exit(1);
            }
            planPath = foundPath;
          } else {
            // File path
            planPath = planArg;
          }

          // Load plan with format detection
          spinner.text = 'Loading plan...';

          if (options.native) {
            // Use legacy loadPlan for native APS
            plan = await loadPlan(planPath);
            spinner.succeed('Plan loaded (native APS)');
          } else {
            // Use PlanLoader for auto-detection and external formats
            const planLoader = new PlanLoader();
            spinner.text = 'Detecting format...';

            const loadResult = await planLoader.loadPlan(planPath, {
              format: options.format as string | undefined,
              validateHash: false, // Gate doesn't require hash validation
              strict: false,
            });

            plan = loadResult.plan;
            sourceFormat = loadResult.sourceFormat;

            // Show detected format if applicable
            if (sourceFormat) {
              spinner.succeed(
                `Plan loaded (format: ${sourceFormat.format}, ${sourceFormat.confidence}% confidence)`
              );
            } else {
              spinner.succeed('Plan loaded');
            }
          }
        }

        // Load config
        spinner.start('Loading gate configuration...');
        const config = configManager.loadConfig();
        spinner.succeed('Configuration loaded');

        // Create cache provider
        const cacheDisabled = options.noCache === true;
        const cache = createCacheProvider({
          type: cacheDisabled ? 'null' : 'file',
          workspaceRoot,
          disabled: cacheDisabled,
        });

        // Parse parallel limit
        const parallelLimit =
          options.parallel !== undefined ? parseInt(options.parallel, 10) : undefined;

        // Prepare gate options
        const gateOptions: GateRunOptions = {
          cache,
          parallelLimit,
          noCache: cacheDisabled,
          fullScan: isFullScan,
        };

        // Collect skip checks from multiple sources (profile, CLI, environment)
        const skipChecksSet = new Set<string>();

        // 1. Apply profile settings first
        if (options.profile) {
          const profileName = options.profile as GateProfile;
          const profile = GATE_PROFILES[profileName];

          if (!profile) {
            error(
              `Unknown profile: ${profileName}. Use --list-profiles to see available profiles.`
            );
            process.exit(1);
          }

          if (profile.skipChecks) {
            profile.skipChecks.forEach((check) => skipChecksSet.add(check));
          }

          if (options.verbose) {
            info(`Using profile: ${profileName}`);
          }
        }

        // 2. Apply ANVIL_SKIP_GATES environment variable
        const envSkipGates = parseSkipGatesEnv();
        if (envSkipGates.length > 0) {
          envSkipGates.forEach((check) => skipChecksSet.add(check));
          if (options.verbose) {
            info(`ANVIL_SKIP_GATES: ${envSkipGates.join(', ')}`);
          }
        }

        // 3. Apply CLI --skip-checks (highest priority, adds to set)
        if (options.skipChecks) {
          options.skipChecks
            .split(',')
            .map((s) => s.trim())
            .forEach((check) => skipChecksSet.add(check));
        }

        // Convert set to array
        if (skipChecksSet.size > 0) {
          gateOptions.skipChecks = Array.from(skipChecksSet);
        }

        if (options.onlyChecks) {
          gateOptions.onlyChecks = options.onlyChecks.split(',').map((s) => s.trim());
        }

        if (options.failFast) {
          gateOptions.failFast = true;
        }

        // Run gate with progress reporting
        const showProgress = options.progress && options.output !== 'json';

        if (showProgress) {
          // Progress mode: show real-time updates
          console.log(chalk.bold('\nRunning quality gates:\n'));

          // Track active checks for parallel display
          const activeChecks = new Set<string>();
          const completedChecks: Array<{
            name: string;
            passed: boolean;
            cached: boolean;
            timeMs: number;
          }> = [];

          gateOptions.onProgress = (event: ProgressEvent) => {
            if (event.type === 'check:start') {
              activeChecks.add(event.checkName);
              // Show running indicator
              process.stdout.write(
                chalk.cyan(`  ▶ ${event.checkName}`) + chalk.gray(' running...\n')
              );
            } else {
              activeChecks.delete(event.checkName);
              const passed = event.result?.passed ?? false;
              const cached = event.cached ?? false;
              const timeMs = event.executionTimeMs ?? 0;

              completedChecks.push({
                name: event.checkName,
                passed,
                cached,
                timeMs,
              });

              // Show completion status
              const statusIcon = passed ? chalk.green('✓') : chalk.red('✗');
              const cacheLabel = cached ? chalk.gray(' (cached)') : '';
              const timeLabel = chalk.gray(` ${timeMs}ms`);
              const progressLabel = chalk.gray(` [${event.current}/${event.total}]`);

              process.stdout.write(
                `  ${statusIcon} ${event.checkName}${cacheLabel}${timeLabel}${progressLabel}\n`
              );
            }
          };
        } else {
          // Standard mode: use spinner
          spinner.start('Running quality gates...');
        }

        const results = await gateRunner.runGate(plan, config, workspaceRoot, gateOptions);

        if (showProgress) {
          console.log(''); // Add newline after progress
        } else {
          spinner.succeed('Quality gates completed');
        }

        // Display results based on output format
        if (options.output === 'json') {
          formatGateResultsJSON(results);
        } else {
          formatGateResults(results);

          // Show cache and timing stats in verbose mode
          if (options.verbose && results.cacheStats) {
            console.log(chalk.gray('\nCache Statistics:'));
            console.log(chalk.gray(`  Hits: ${results.cacheStats.hits}`));
            console.log(chalk.gray(`  Misses: ${results.cacheStats.misses}`));
            if (results.cacheStats.timeSavedMs > 0) {
              console.log(chalk.gray(`  Time saved: ${results.cacheStats.timeSavedMs}ms`));
            }
          }

          if (options.verbose && results.timing) {
            console.log(chalk.gray('\nExecution Timing:'));
            console.log(chalk.gray(`  Total: ${results.timing.totalMs}ms`));
            for (const [checkName, timeMs] of Object.entries(results.timing.checks)) {
              console.log(chalk.gray(`  ${checkName}: ${timeMs}ms`));
            }
          }
        }

        // Evidence injection
        if (options.inject) {
          if (isFullScan) {
            console.log(
              chalk.yellow('\n⚠️  Evidence injection not available in full codebase scan mode')
            );
            console.log(chalk.gray('Provide a plan file to enable evidence injection'));
          } else if (!sourceFormat) {
            console.log(
              chalk.yellow(
                '\n⚠️  Evidence injection only supported for external formats (SpecKit, BMAD)'
              )
            );
            console.log(chalk.gray('Skipping injection for native APS format'));
          } else if (planPath) {
            spinner.start('Injecting evidence into source document...');

            const evidenceWriter = new EvidenceWriter();
            const writeResult = await evidenceWriter.writeEvidence({
              format: sourceFormat.format,
              filePath: planPath,
              gateResults: results,
              plan,
              mode: 'replace', // Replace existing evidence section
            });

            if (writeResult.success) {
              spinner.succeed(chalk.green('✓ Evidence injected successfully'));
              console.log(chalk.gray('  Updated:'), chalk.cyan(writeResult.filePath));
            } else {
              spinner.fail(chalk.red('✗ Failed to inject evidence'));
              console.log(chalk.red('  Error:'), writeResult.error);
            }
          }
        }

        const useTUI = isTUIAvailable({ tui: options.tui, noTui: options.noTui });
        if (useTUI && options.tui && options.output !== 'json') {
          const tuiResult = convertToTUIGateResult(results, plan.id, planPath);
          await renderTUIAndWait(GateExplorer, { result: tuiResult });
        }

        if (results.overall) {
          success('All quality gates passed!');
          process.exit(0);
        } else {
          error('Quality gates failed');
          process.exit(1);
        }
      } catch (err) {
        error(`Gate execution failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
        process.exit(1);
      }
    });

  return command;
}
