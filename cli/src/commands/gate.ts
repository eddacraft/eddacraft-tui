/**
 * Gate Command - Run quality gates on APS plans and external formats
 */

import { Command } from 'commander';
import { GateRunner, GateConfigManager } from '@anvil/core';
import { loadPlan, findPlanById, getWorkspaceRoot } from '../utils/file-io.js';
import { PlanLoader } from '../services/plan-loader.js';
import type { GateOptions } from '../types/command-options.js';
import { success, error, formatGateResults } from '../utils/output.js';
import ora from 'ora';

export function createGateCommand(): Command {
  const command = new Command('gate');

  command
    .description('Run quality gates on a plan (supports APS and external formats like SpecKit)')
    .argument('<plan>', 'Plan ID or file path')
    .option('-c, --config <path>', 'Custom config file path')
    .option('-v, --verbose', 'Verbose output')
    .option('--format <format>', 'Explicitly specify input format (bypasses auto-detection)')
    .option('--native', 'Skip format detection and treat as native APS')
    .option('--inject', 'Inject evidence back into source document (future feature)')
    .option('--skip-checks <checks>', 'Comma-separated list of checks to skip')
    .option('--only-checks <checks>', 'Only run specified checks (comma-separated)')
    .option('--fail-fast', 'Stop on first check failure')
    .action(async (planArg: string, options: GateOptions) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);
        const gateRunner = new GateRunner();

        // Resolve plan path
        let planPath: string;
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
        const spinner = ora('Loading plan...').start();

        let plan;
        let sourceFormat;

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

        // Load config
        spinner.start('Loading gate configuration...');
        const config = configManager.loadConfig();
        spinner.succeed('Configuration loaded');

        // Run gate
        spinner.start('Running quality gates...');
        const results = await gateRunner.runGate(plan, config, workspaceRoot);
        spinner.succeed('Quality gates completed');

        // Display results
        formatGateResults(results);

        // TODO: Evidence injection (Task 7 - not implemented yet)
        if (options.inject) {
          console.log('\n⚠️  Evidence injection not yet implemented');
          console.log('Evidence will be stored in .anvil/evidence/ directory');
        }

        // Exit with appropriate code
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
