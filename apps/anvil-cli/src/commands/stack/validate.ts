/**
 * Stack Validate Subcommand (STACK-014)
 *
 * Validates stack configuration and provenance integrity.
 *
 * Usage:
 *   anvil stack validate           Validate stack configuration
 *   anvil stack validate --json    Output as JSON
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { GateConfigManager } from '@eddacraft/anvil-runtime';
import { StackConfigSchema, isLayerEnabled, getEnabledLayers, type StackConfig } from './config.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { blank, data, error, print, success } from '../../utils/output.js';
import { CliError, CliExit } from '../../utils/cli-error.js';

/**
 * Validate command options
 */
export interface ValidateOptions {
  json?: boolean;
}

/**
 * Validation issue severity
 */
type IssueSeverity = 'error' | 'warning' | 'info';

/**
 * A single validation issue
 */
interface ValidationIssue {
  severity: IssueSeverity;
  code: string;
  message: string;
  path?: string;
  suggestion?: string;
}

/**
 * Validation result
 */
interface ValidationResult {
  valid: boolean;
  issues: ValidationIssue[];
  summary: {
    errors: number;
    warnings: number;
    infos: number;
  };
  configPath?: string;
}

/**
 * Validate stack configuration
 */
function validateStackConfig(workspaceRoot: string): ValidationResult {
  const issues: ValidationIssue[] = [];
  const configManager = new GateConfigManager(workspaceRoot);
  const { config, path, errors: configErrors } = configManager.loadConfigWithDetails();

  // Check for config load errors
  if (configErrors && configErrors.length > 0) {
    for (const err of configErrors) {
      issues.push({
        severity: 'error',
        code: 'CONFIG_PARSE_ERROR',
        message: err,
        path: path ?? undefined,
      });
    }
  }

  // Check if stack section exists
  if (!config.stack) {
    issues.push({
      severity: 'info',
      code: 'STACK_NOT_CONFIGURED',
      message: 'Stack section not found in configuration',
      suggestion: 'Add a "stack" section to .anvilrc to configure layers',
    });

    return {
      valid: true, // Not invalid, just unconfigured
      issues,
      summary: countIssues(issues),
      configPath: path ?? undefined,
    };
  }

  // Validate with Zod schema for strict checking
  const schemaResult = StackConfigSchema.safeParse(config.stack);
  if (!schemaResult.success) {
    for (const zodIssue of schemaResult.error.issues) {
      issues.push({
        severity: 'error',
        code: 'SCHEMA_VALIDATION_ERROR',
        message: zodIssue.message,
        path: `stack.${zodIssue.path.join('.')}`,
      });
    }
  }

  const stackConfig = schemaResult.success ? schemaResult.data : undefined;

  if (stackConfig) {
    // Check layer dependencies
    validateLayerDependencies(stackConfig, issues);

    // Check validation settings
    validateValidationSettings(stackConfig, issues);

    // Provenance integrity check (placeholder for future implementation)
    if (stackConfig.validation?.check_provenance_integrity) {
      // This would check actual provenance links in storage
      // For now, just report that the check is enabled
      issues.push({
        severity: 'info',
        code: 'PROVENANCE_CHECK_ENABLED',
        message: 'Provenance integrity checking is enabled',
      });
    }

    // Schema compatibility check (placeholder for future implementation)
    if (stackConfig.validation?.check_schema_compatibility) {
      issues.push({
        severity: 'info',
        code: 'SCHEMA_CHECK_ENABLED',
        message: 'Schema compatibility checking is enabled',
      });
    }
  }

  // Calculate if valid (no errors)
  const summary = countIssues(issues);
  const valid = summary.errors === 0;

  return {
    valid,
    issues,
    summary,
    configPath: path ?? undefined,
  };
}

/**
 * Validate layer dependencies
 */
function validateLayerDependencies(config: StackConfig, issues: ValidationIssue[]): void {
  const enabledLayers = getEnabledLayers(config);

  // Edda requires Ember (for promotion workflow)
  if (isLayerEnabled(config, 'edda') && !isLayerEnabled(config, 'ember')) {
    issues.push({
      severity: 'warning',
      code: 'MISSING_DEPENDENCY',
      message: 'Edda layer is enabled but Ember layer is disabled',
      suggestion: 'Enable Ember layer for the promotion workflow to function',
      path: 'stack.edda',
    });
  }

  // Ember requires Kindling (for observation aggregation)
  if (isLayerEnabled(config, 'ember') && !isLayerEnabled(config, 'kindling')) {
    issues.push({
      severity: 'warning',
      code: 'MISSING_DEPENDENCY',
      message: 'Ember layer is enabled but Kindling layer is disabled',
      suggestion: 'Enable Kindling layer for observation aggregation to function',
      path: 'stack.ember',
    });
  }

  // No layers enabled is fine but worth noting
  if (enabledLayers.length === 0) {
    issues.push({
      severity: 'info',
      code: 'NO_LAYERS_ENABLED',
      message: 'No stack layers are currently enabled',
      suggestion: 'Enable layers in .anvilrc to start using the Edda Stack',
    });
  }
}

/**
 * Validate validation settings
 */
function validateValidationSettings(config: StackConfig, issues: ValidationIssue[]): void {
  // Check if validation settings exist when layers are enabled
  const enabledLayers = getEnabledLayers(config);

  if (enabledLayers.length > 0 && !config.validation) {
    issues.push({
      severity: 'info',
      code: 'VALIDATION_DEFAULTS',
      message: 'Validation settings not specified, using defaults',
      suggestion: 'Add stack.validation section to customize integrity checks',
    });
  }
}

/**
 * Count issues by severity
 */
function countIssues(issues: ValidationIssue[]): {
  errors: number;
  warnings: number;
  infos: number;
} {
  return {
    errors: issues.filter((i) => i.severity === 'error').length,
    warnings: issues.filter((i) => i.severity === 'warning').length,
    infos: issues.filter((i) => i.severity === 'info').length,
  };
}

/**
 * Format issue for display
 */
function formatIssue(issue: ValidationIssue): void {
  const icon =
    issue.severity === 'error'
      ? chalk.red('✗')
      : issue.severity === 'warning'
        ? chalk.yellow('⚠')
        : chalk.blue('ℹ');

  const severityColor =
    issue.severity === 'error'
      ? chalk.red
      : issue.severity === 'warning'
        ? chalk.yellow
        : chalk.blue;

  print(`  ${icon} ${severityColor(`[${issue.code}]`)} ${issue.message}`);

  if (issue.path) {
    print(chalk.dim(`      Path: ${issue.path}`));
  }

  if (issue.suggestion) {
    print(chalk.dim(`      Suggestion: ${issue.suggestion}`));
  }
}

/**
 * Display validation result
 */
function displayResult(result: ValidationResult): void {
  blank();
  print(chalk.bold.underline('Stack Validation'));
  blank();

  if (result.issues.length === 0) {
    success('Stack configuration is valid');
    return;
  }

  // Group issues by severity
  const errors = result.issues.filter((i) => i.severity === 'error');
  const warnings = result.issues.filter((i) => i.severity === 'warning');
  const infos = result.issues.filter((i) => i.severity === 'info');

  // Display errors first
  if (errors.length > 0) {
    print(chalk.red.bold(`Errors (${errors.length}):`));
    blank();
    for (const issue of errors) {
      formatIssue(issue);
      blank();
    }
  }

  // Then warnings
  if (warnings.length > 0) {
    print(chalk.yellow.bold(`Warnings (${warnings.length}):`));
    blank();
    for (const issue of warnings) {
      formatIssue(issue);
      blank();
    }
  }

  // Then info
  if (infos.length > 0) {
    print(chalk.blue.bold(`Info (${infos.length}):`));
    blank();
    for (const issue of infos) {
      formatIssue(issue);
      blank();
    }
  }

  // Summary
  print(chalk.bold('Summary:'));
  print(
    `  ${chalk.red(result.summary.errors + ' error(s)')}, ` +
      `${chalk.yellow(result.summary.warnings + ' warning(s)')}, ` +
      `${chalk.blue(result.summary.infos + ' info(s)')}`
  );
  blank();

  if (result.valid) {
    success('Stack configuration is valid');
  } else {
    error('Stack configuration has errors');
  }

  // Config path
  if (result.configPath) {
    print(chalk.dim(`\nConfiguration: ${result.configPath}`));
  }
}

/**
 * Create the validate subcommand
 */
export function createValidateSubcommand(): Command {
  return new Command('validate')
    .description('Validate stack configuration and provenance integrity')
    .option('--json', 'Output as JSON')
    .action(async (options: ValidateOptions) => {
      if (options.json) {
        // JSON mode
        try {
          const workspaceRoot = getWorkspaceRoot();
          const result = validateStackConfig(workspaceRoot);
          data(JSON.stringify(result, null, 2));
          if (result.valid) throw new CliExit();
          throw new CliError('Validation failed');
        } catch (err) {
          if (err instanceof CliError || err instanceof CliExit) throw err;
          data(
            JSON.stringify(
              {
                valid: false,
                error: err instanceof Error ? err.message : 'Unknown error',
                issues: [],
                summary: { errors: 1, warnings: 0, infos: 0 },
              },
              null,
              2
            )
          );
          throw new CliError(err instanceof Error ? err.message : 'Validation failed');
        }
      } else {
        // Human-readable mode
        const spinner = ora('Validating stack configuration...').start();

        try {
          const workspaceRoot = getWorkspaceRoot();
          const result = validateStackConfig(workspaceRoot);

          spinner.stop();
          displayResult(result);

          if (!result.valid) {
            throw new CliError('Validation failed');
          }
        } catch (err) {
          if (err instanceof CliError || err instanceof CliExit) throw err;
          spinner.fail(chalk.red('Validation failed'));
          print(chalk.red('Error:'), err instanceof Error ? err.message : String(err));
          throw new CliError(err instanceof Error ? err.message : 'Validation failed');
        }
      }
    });
}
