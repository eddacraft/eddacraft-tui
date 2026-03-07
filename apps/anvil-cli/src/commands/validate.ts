/**
 * Validate Command - Validates APS plans and external formats
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { verifyHash, createDebugger } from '@eddacraft/anvil-core';
import { loadPlan } from '../utils/file-io.js';
import { resolvePlanPathOrId } from '../utils/plan-resolution.js';
import { PlanLoader } from '../services/plan-loader.js';
import type { ValidateOptions } from '../types/command-options.js';
import { CliError } from '../utils/cli-error.js';
import { print } from '../utils/output.js';

const log = createDebugger('cli');

export function createValidateCommand(): Command {
  return new Command('validate')
    .description('Validate an Anvil plan (supports APS and external formats like SpecKit)')
    .argument('<plan>', 'Plan file path or plan ID')
    .option('-v, --verbose', 'Show detailed validation results')
    .option('--format <format>', 'Explicitly specify input format (bypasses auto-detection)')
    .option('--native', 'Skip format detection and treat as native APS')
    .option('--validate-hash', 'Validate hash integrity', true)
    .action(async (planPathOrId: string, options: ValidateOptions) => {
      log(
        `validate command entered: plan=${planPathOrId} native=${options.native} validateHash=${options.validateHash}`
      );
      const spinner = ora('Loading plan...').start();

      try {
        // Resolve plan path
        const { path: planPath } = resolvePlanPathOrId(planPathOrId);

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
            format: options.format,
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
          print(chalk.red('\nValidation Errors:'));

          if (options.verbose && validationResult.issues) {
            // Show detailed errors
            validationResult.issues.forEach((issue: { path?: string; message: string }) => {
              print(chalk.yellow(`  - ${issue.path || 'root'}:`), issue.message);
            });
          } else if (validationResult.issues) {
            // Show error summary
            print(chalk.red(`  Found ${validationResult.issues.length} validation error(s)`));
            validationResult.issues.slice(0, 3).forEach((issue: { message: string }) => {
              print(chalk.yellow(`  - ${issue.message}`));
            });
            if (validationResult.issues.length > 3) {
              print(chalk.gray(`  ... and ${validationResult.issues.length - 3} more`));
            }
          }

          throw new CliError('Plan validation failed');
        }

        // Show warnings if any
        if (warnings && warnings.length > 0) {
          print(chalk.yellow('\n⚠ Warnings:'));
          warnings.forEach((warning) => {
            print(chalk.yellow(`  - ${warning.message}`));
          });
        }

        // Verify hash integrity if requested
        // Note: Only validate hash for native APS plans. External formats (SpecKit, BMAD)
        // generate hashes during parsing, so hash validation doesn't apply.
        if (options.validateHash && !sourceFormat) {
          spinner.text = 'Verifying plan hash...';
          // Exclude hash field before verification to avoid circular dependency
          const { hash, ...planWithoutHash } = plan;
          const hashValid = verifyHash(planWithoutHash, hash);

          if (!hashValid) {
            spinner.fail(chalk.red('✗ Hash verification failed'));
            print(chalk.red('\nThe plan hash does not match its content.'));
            print(chalk.yellow('This may indicate the plan has been tampered with.'));
            throw new CliError('Hash verification failed');
          }
        } else if (options.validateHash && sourceFormat) {
          // External formats: hash is generated during parsing, so we skip validation
          spinner.text = 'Skipping hash validation (external format)';
        }

        spinner.succeed(chalk.green('✓ Plan is valid'));

        // Display plan details
        print('\n' + chalk.bold('Plan Details:'));

        // Show source format if detected
        if (sourceFormat) {
          print(chalk.gray('  Source Format:'), chalk.cyan(sourceFormat.format));
          print(chalk.gray('  Adapter:      '), chalk.cyan(sourceFormat.adapter));
        }

        print(chalk.gray('  ID:           '), chalk.cyan(plan.id));
        print(chalk.gray('  Schema:       '), chalk.cyan(plan.schema_version));
        print(chalk.gray('  Hash:         '), chalk.cyan(plan.hash.substring(0, 16) + '...'));
        print(chalk.gray('  Intent:       '), chalk.white(plan.intent));
        print(chalk.gray('  Changes:      '), chalk.cyan(plan.proposed_changes.length.toString()));
        print(chalk.gray('  Evidence:     '), chalk.cyan((plan.evidence?.length ?? 0).toString()));

        if (plan.provenance) {
          print(chalk.gray('  Created By:   '), chalk.cyan(plan.provenance.author || 'unknown'));
          print(chalk.gray('  Created At:   '), chalk.cyan(plan.provenance.timestamp));
        }

        if (options.verbose && plan.validations) {
          print('\n' + chalk.bold('Required Checks:'));
          plan.validations.required_checks.forEach((check: string) => {
            print(chalk.gray('  - '), chalk.cyan(check));
          });
        }

        log(`validate result: PASSED plan=${plan.id}`);
        print(chalk.green('\n✓ All validation checks passed'));
      } catch (error) {
        if (error instanceof CliError) throw error;
        spinner.fail(chalk.red('Validation failed'));
        print(chalk.red('Error:'), error instanceof Error ? error.message : String(error));
        throw new CliError(error instanceof Error ? error.message : String(error));
      }
    });
}
