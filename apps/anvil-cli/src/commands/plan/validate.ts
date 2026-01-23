/**
 * Plan Validate Command
 * Validates an APS planning document
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'path';
import {
  validatePlanningDoc,
  formatValidationIssues,
  type ValidationResult,
} from '@eddacraft/anvil-aps';

export interface ValidateOptions {
  json?: boolean;
}

/**
 * Format validation result as JSON
 */
function formatAsJson(result: ValidationResult, filePath: string): string {
  return JSON.stringify(
    {
      file: filePath,
      valid: result.valid,
      errorCount: result.errors.length,
      warningCount: result.warnings.length,
      issues: result.issues.map((issue) => ({
        severity: issue.severity,
        rule: issue.rule,
        message: issue.message,
        path: issue.path,
        line: issue.lineNumber,
        context: issue.context,
      })),
    },
    null,
    2
  );
}

export function createValidateSubcommand(): Command {
  return new Command('validate')
    .description('Validate an APS planning document')
    .argument('[path]', 'Path to planning document', 'docs/planning/APS.md')
    .option('--json', 'Output as JSON')
    .action(async (path: string, options: ValidateOptions) => {
      const filePath = resolve(path);

      if (options.json) {
        // JSON mode - no spinner, structured output
        try {
          const result = await validatePlanningDoc(filePath);
          console.log(formatAsJson(result, filePath));
          process.exit(result.valid ? 0 : 1);
        } catch (error) {
          console.log(
            JSON.stringify(
              {
                file: filePath,
                valid: false,
                error: error instanceof Error ? error.message : String(error),
              },
              null,
              2
            )
          );
          process.exit(1);
        }
      } else {
        // Human-readable mode
        const spinner = ora(`Validating ${path}...`).start();

        try {
          const result = await validatePlanningDoc(filePath);

          if (result.valid) {
            if (result.warnings.length > 0) {
              spinner.warn(
                chalk.yellow(`Validation passed with ${result.warnings.length} warning(s)`)
              );
            } else {
              spinner.succeed(chalk.green('Validation passed'));
            }
          } else {
            spinner.fail(chalk.red(`Validation failed with ${result.errors.length} error(s)`));
          }

          // Print issues
          if (result.issues.length > 0) {
            console.log('');
            console.log(formatValidationIssues(result));
          }

          process.exit(result.valid ? 0 : 1);
        } catch (error) {
          spinner.fail(chalk.red('Validation failed'));
          console.error(
            chalk.red('Error:'),
            error instanceof Error ? error.message : String(error)
          );
          process.exit(1);
        }
      }
    });
}
