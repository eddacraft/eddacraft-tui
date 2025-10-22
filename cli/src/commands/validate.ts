/**
 * Validate Command - Validates APS plans and external formats
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { verifyHash } from '@anvil/core';
import { loadPlan, findPlanById, getWorkspaceRoot } from '../utils/file-io.js';
import { PlanLoader } from '../services/plan-loader.js';
import type { ValidateOptions } from '../types/command-options.js';
import { existsSync } from 'fs';

export function createValidateCommand(): Command {
  return new Command('validate')
    .description('Validate an Anvil plan (supports APS and external formats like SpecKit)')
    .argument('<plan>', 'Plan file path or plan ID')
    .option('-v, --verbose', 'Show detailed validation results')
    .option('--format <format>', 'Explicitly specify input format (bypasses auto-detection)')
    .option('--native', 'Skip format detection and treat as native APS')
    .option('--validate-hash', 'Validate hash integrity', true)
    .action(async (planPathOrId: string, options: ValidateOptions) => {
      const spinner = ora('Loading plan...').start();

      try {
        // Resolve plan path
        let planPath = planPathOrId;

        // Check if it's a plan ID (starts with 'aps-')
        if (planPathOrId.startsWith('aps-')) {
          const workspaceRoot = getWorkspaceRoot();
          const resolvedPath = findPlanById(planPathOrId, workspaceRoot);

          if (!resolvedPath) {
            throw new Error(`Plan with ID '${planPathOrId}' not found`);
          }

          planPath = resolvedPath;
        } else if (!existsSync(planPath)) {
          throw new Error(`Plan file not found: ${planPath}`);
        }

        // Load plan using PlanLoader (supports APS and external formats)
        let plan;
        let sourceFormat;
        let warnings;
        let validationResult;

        if (options.native) {
          // Use legacy loadPlan for native APS
          spinner.text = 'Loading native APS plan...';
          plan = await loadPlan(planPath);
          validationResult = { valid: true, data: plan };
        } else {
          // Use PlanLoader for auto-detection and external formats
          const planLoader = new PlanLoader();
          spinner.text = 'Detecting format...';

          const loadResult = await planLoader.loadPlan(planPath, {
            format: options.format as string | undefined,
            validateHash: options.validateHash ?? true,
            strict: false,
          });

          plan = loadResult.plan;
          validationResult = loadResult.validation;
          sourceFormat = loadResult.sourceFormat;
          warnings = loadResult.warnings;

          // Show detected format if applicable
          if (sourceFormat) {
            spinner.succeed(
              chalk.green(
                `✓ Detected format: ${chalk.cyan(sourceFormat.format)} (${sourceFormat.confidence}% confidence)`
              )
            );
            spinner.start('Validating plan...');
          } else {
            spinner.text = 'Validating plan...';
          }
        }

        // Check validation result
        if (!validationResult.valid) {
          spinner.fail(chalk.red('✗ Plan validation failed'));
          console.error(chalk.red('\nValidation Errors:'));

          if (options.verbose && validationResult.issues) {
            // Show detailed errors
            validationResult.issues.forEach((issue: { path?: string; message: string }) => {
              console.error(chalk.yellow(`  - ${issue.path || 'root'}:`), issue.message);
            });
          } else if (validationResult.issues) {
            // Show error summary
            console.error(
              chalk.red(`  Found ${validationResult.issues.length} validation error(s)`)
            );
            validationResult.issues.slice(0, 3).forEach((issue: { message: string }) => {
              console.error(chalk.yellow(`  - ${issue.message}`));
            });
            if (validationResult.issues.length > 3) {
              console.error(chalk.gray(`  ... and ${validationResult.issues.length - 3} more`));
            }
          }

          process.exit(1);
        }

        // Show warnings if any
        if (warnings && warnings.length > 0) {
          console.log(chalk.yellow('\n⚠ Warnings:'));
          warnings.forEach((warning) => {
            console.log(chalk.yellow(`  - ${warning.message}`));
          });
        }

        // Verify hash integrity if requested
        if (options.validateHash) {
          spinner.text = 'Verifying plan hash...';
          const hashValid = verifyHash(plan, plan.hash);

          if (!hashValid) {
            spinner.fail(chalk.red('✗ Hash verification failed'));
            console.error(chalk.red('\nThe plan hash does not match its content.'));
            console.error(chalk.yellow('This may indicate the plan has been tampered with.'));
            process.exit(1);
          }
        }

        spinner.succeed(chalk.green('✓ Plan is valid'));

        // Display plan details
        console.log('\n' + chalk.bold('Plan Details:'));

        // Show source format if detected
        if (sourceFormat) {
          console.log(chalk.gray('  Source Format:'), chalk.cyan(sourceFormat.format));
          console.log(chalk.gray('  Adapter:      '), chalk.cyan(sourceFormat.adapter));
        }

        console.log(chalk.gray('  ID:           '), chalk.cyan(plan.id));
        console.log(chalk.gray('  Schema:       '), chalk.cyan(plan.schema_version));
        console.log(chalk.gray('  Hash:         '), chalk.cyan(plan.hash.substring(0, 16) + '...'));
        console.log(chalk.gray('  Intent:       '), chalk.white(plan.intent));
        console.log(
          chalk.gray('  Changes:      '),
          chalk.cyan(plan.proposed_changes.length.toString())
        );
        console.log(
          chalk.gray('  Evidence:     '),
          chalk.cyan((plan.evidence?.length ?? 0).toString())
        );

        if (plan.provenance) {
          console.log(
            chalk.gray('  Created By:   '),
            chalk.cyan(plan.provenance.author || 'unknown')
          );
          console.log(chalk.gray('  Created At:   '), chalk.cyan(plan.provenance.timestamp));
        }

        if (options.verbose && plan.validations) {
          console.log('\n' + chalk.bold('Required Checks:'));
          plan.validations.required_checks.forEach((check: string) => {
            console.log(chalk.gray('  - '), chalk.cyan(check));
          });
        }

        console.log(chalk.green('\n✓ All validation checks passed'));
      } catch (error) {
        spinner.fail(chalk.red('Validation failed'));
        console.error(chalk.red('Error:'), error instanceof Error ? error.message : String(error));
        process.exit(1);
      }
    });
}
