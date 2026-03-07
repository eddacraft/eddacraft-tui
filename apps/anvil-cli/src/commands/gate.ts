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
} from '@eddacraft/anvil-runtime';
import { createDebugger } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../utils/cli-error.js';
import { loadPlan, getWorkspaceRoot } from '../utils/file-io.js';
import { resolvePlanPathOrId } from '../utils/plan-resolution.js';
import { PlanLoader } from '../services/plan-loader.js';
import { EvidenceWriter } from '../services/evidence-writer.js';
import type { GateOptions, GateProfile } from '../types/command-options.js';
import { success, error, formatGateResults, info, formatGateResultsJSON } from '../utils/output.js';
import { coerceNonNegativeInt } from '../utils/option-coerce.js';
import ora from 'ora';
import { initKindling, type KindlingContext } from '../services/kindling-bootstrap.js';
import {
  emitSessionStart,
  emitSessionEnd,
  emitGateEvaluated,
  emitError as emitKindlingError,
} from '@eddacraft/anvil-kindling-integration';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUIAndWait } from '../tui/utils/renderer.js';
import { GateExplorer } from '../tui/commands/gate/index.js';
import type {
  GateResult as TUIGateResult,
  CheckResult as TUICheckResult,
} from '../tui/commands/gate/types.js';

const log = createDebugger('cli');

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
  process.stderr.write('Warning: ANVIL_SKIP_GATES is set — gate checks are being skipped.\n');
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
    .option('--skip-command-safety', 'Skip command safety validation check')
    .option('--no-provenance', 'Disable provenance recording')
    .action(async (planArg: string | undefined, options: GateOptions) => {
      log(`gate command entered`, {
        plan: planArg ?? '(full scan)',
        profile: options.profile ?? 'none',
      });

      // Handle --list-profiles
      if (options.listProfiles) {
        console.error(chalk.bold('\nAvailable Gate Profiles:\n'));
        for (const [name, profile] of Object.entries(GATE_PROFILES)) {
          console.error(chalk.cyan(`  ${name}`));
          console.error(chalk.gray(`    ${profile.description}`));
          if (profile.skipChecks && profile.skipChecks.length > 0) {
            console.error(chalk.gray(`    Skips: ${profile.skipChecks.join(', ')}`));
          }
          console.error('');
        }
        console.error(chalk.gray('Usage: anvil gate [plan] --profile=dev'));
        throw new CliExit();
      }

      const startTime = Date.now();
      let kindling: KindlingContext | null = null;
      let sessionId: string | undefined;
      let capsuleId: string | undefined;

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
          log('no plan argument provided, using full codebase scan mode');
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
          try {
            const { path: resolvedPath } = resolvePlanPathOrId(planArg, workspaceRoot);
            planPath = resolvedPath;
          } catch (err) {
            if (err instanceof CliError || err instanceof CliExit) throw err;
            error(err instanceof Error ? err.message : String(err));
            throw new CliError('Failed to resolve plan path');
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
        let parallelLimit: number | undefined;
        if (options.parallel !== undefined) {
          parallelLimit = coerceNonNegativeInt(options.parallel, '--parallel');
        }

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
            throw new CliError('Unknown gate profile');
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

        // 4. Apply --skip-command-safety convenience flag
        if (options.skipCommandSafety) {
          skipChecksSet.add('command-safety');
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

        // Configure provenance recording (on by default)
        if (options.provenance !== false) {
          const trigger = process.env.CI
            ? ('ci' as const)
            : process.env.ANVIL_TRIGGER === 'pre-commit'
              ? ('pre-commit' as const)
              : ('manual' as const);

          const scope = isFullScan ? ('directory' as const) : ('plan' as const);

          // Extract file paths from proposed_changes for provenance
          const filesChecked = plan.proposed_changes.map((c) => c.path);

          gateOptions.provenance = {
            enabled: true,
            trigger,
            scope,
            planId: plan.id !== 'aps-00000000' ? plan.id : undefined,
            filesChecked,
          };
        }

        // Initialize Kindling provenance recording (after arg validation, no-op when disabled)
        kindling = initKindling(workspaceRoot);
        if (kindling) {
          sessionId = emitSessionStart(kindling.service, {
            working_directory: workspaceRoot,
            anvil_version: '0.1.0',
            command: 'gate',
            args: planArg ? [planArg] : [],
            environment: process.env.CI ? 'ci' : 'development',
          });
          const capsule = kindling.adapter.startSession(sessionId, 'anvil gate');
          capsuleId = capsule.id;
          kindling.bridge.setCapsuleId(capsuleId);
        }

        // Run gate with progress reporting
        const showProgress = options.progress && options.output !== 'json';

        if (showProgress) {
          // Progress mode: show real-time updates
          console.error(chalk.bold('\nRunning quality gates:\n'));

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
              process.stderr.write(
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

              process.stderr.write(
                `  ${statusIcon} ${event.checkName}${cacheLabel}${timeLabel}${progressLabel}\n`
              );
            }
          };
        } else {
          // Standard mode: use spinner
          spinner.start('Running quality gates...');
        }

        log('running gate with options', {
          skipChecks: gateOptions.skipChecks,
          onlyChecks: gateOptions.onlyChecks,
          failFast: gateOptions.failFast,
          fullScan: gateOptions.fullScan,
        });
        const results = await gateRunner.runGate(plan, config, workspaceRoot, gateOptions);

        // Record gate evaluation in Kindling
        if (kindling && sessionId) {
          emitGateEvaluated(kindling.service, {
            session_id: sessionId,
            gate_id: plan.id,
            inputs: {
              file_count: results.checks.length,
            },
            outcome: results.overall ? 'pass' : 'fail',
            rules_evaluated: results.checks.map((c) => c.check),
            rules_violated: results.checks
              .filter((c) => !c.passed && !c.skipped)
              .map((c) => c.check),
            enforcement: 'blocking',
            duration_ms: results.timing?.totalMs ?? 0,
            violation_count: results.checks.filter((c) => !c.passed && !c.skipped).length,
            warning_count: results.checks.filter(
              (c) => c.passed && c.score !== undefined && c.score < 1
            ).length,
          });
        }

        if (showProgress) {
          console.error(''); // Add newline after progress
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
            console.error(chalk.gray('\nCache Statistics:'));
            console.error(chalk.gray(`  Hits: ${results.cacheStats.hits}`));
            console.error(chalk.gray(`  Misses: ${results.cacheStats.misses}`));
            if (results.cacheStats.timeSavedMs > 0) {
              console.error(chalk.gray(`  Time saved: ${results.cacheStats.timeSavedMs}ms`));
            }
          }

          if (options.verbose && results.timing) {
            console.error(chalk.gray('\nExecution Timing:'));
            console.error(chalk.gray(`  Total: ${results.timing.totalMs}ms`));
            for (const [checkName, timeMs] of Object.entries(results.timing.checks)) {
              console.error(chalk.gray(`  ${checkName}: ${timeMs}ms`));
            }
          }
        }

        // Show provenance ID
        if (results.provenance_id && options.output !== 'json') {
          console.error(chalk.gray(`\nProvenance: ${results.provenance_id}`));
        }

        // Evidence injection
        if (options.inject) {
          if (isFullScan) {
            console.error(
              chalk.yellow('\n⚠️  Evidence injection not available in full codebase scan mode')
            );
            console.error(chalk.gray('Provide a plan file to enable evidence injection'));
          } else if (!sourceFormat) {
            console.error(
              chalk.yellow(
                '\n⚠️  Evidence injection only supported for external formats (SpecKit, BMAD)'
              )
            );
            console.error(chalk.gray('Skipping injection for native APS format'));
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
              console.error(chalk.gray('  Updated:'), chalk.cyan(writeResult.filePath));
            } else {
              spinner.fail(chalk.red('✗ Failed to inject evidence'));
              console.error(chalk.red('  Error:'), writeResult.error);
            }
          }
        }

        const useTUI = isTUIAvailable({ tui: options.tui });
        if (useTUI && options.tui && options.output !== 'json') {
          const tuiResult = convertToTUIGateResult(results, plan.id, planPath);
          await renderTUIAndWait(GateExplorer, { result: tuiResult });
        }

        // Record session end in Kindling
        if (kindling && sessionId) {
          const exitCode = results.overall ? 0 : 1;
          emitSessionEnd(kindling.service, sessionId, {
            outcome: results.overall ? 'success' : 'failure',
            exit_code: exitCode,
            duration_ms: Date.now() - startTime,
            summary: {
              gates_evaluated: results.checks.length,
              gates_passed: results.checks.filter((c) => c.passed).length,
              gates_failed: results.checks.filter((c) => !c.passed && !c.skipped).length,
              actions_executed: 0,
              errors_encountered: 0,
            },
          });
          if (capsuleId) kindling.adapter.endSession(capsuleId);
          kindling.close();
        }

        if (results.overall) {
          log(`gate result: PASSED score=${results.score}`);
          success('All quality gates passed!');
          throw new CliExit();
        } else {
          log('gate result: FAILED', {
            score: results.score,
            passed: results.summary.passed,
            failed: results.summary.failed,
          });
          error('Quality gates failed');
          throw new CliError('Quality gates failed');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        // Record error and session end in Kindling
        if (kindling && sessionId) {
          emitKindlingError(kindling.service, {
            session_id: sessionId,
            error_type: 'command_failure',
            context: { component: 'gate' },
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

        error(`Gate execution failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Gate execution failed');
      }
    });

  return command;
}
