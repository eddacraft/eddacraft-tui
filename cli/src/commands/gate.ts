/**
 * Gate Command - Run quality gates on APS plans and external formats
 */

import { Command } from 'commander';
import chalk from 'chalk';
import { GateRunner, GateConfigManager } from '@anvil/core';
import { loadPlan, findPlanById, getWorkspaceRoot } from '../utils/file-io.js';
import { PlanLoader } from '../services/plan-loader.js';
import { EvidenceWriter } from '../services/evidence-writer.js';
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
    .option('--inject', 'Inject evidence back into source document (SpecKit, BMAD)')
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

        // Prepare gate options
        const gateOptions: {
          skipChecks?: string[];
          onlyChecks?: string[];
          failFast?: boolean;
        } = {};

        if (options.skipChecks) {
          gateOptions.skipChecks = options.skipChecks.split(',').map((s) => s.trim());
        }

        if (options.onlyChecks) {
          gateOptions.onlyChecks = options.onlyChecks.split(',').map((s) => s.trim());
        }

        if (options.failFast) {
          gateOptions.failFast = true;
        }

        // Run gate
        spinner.start('Running quality gates...');
        const results = await gateRunner.runGate(plan, config, workspaceRoot, gateOptions);
        spinner.succeed('Quality gates completed');

        // Display results
        formatGateResults(results);

        // Evidence injection
        if (options.inject) {
          if (!sourceFormat) {
            console.log(
              chalk.yellow(
                '\n⚠️  Evidence injection only supported for external formats (SpecKit, BMAD)'
              )
            );
            console.log(chalk.gray('Skipping injection for native APS format'));
          } else {
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
