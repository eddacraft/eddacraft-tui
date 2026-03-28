/**
 * Stack Status Subcommand (STACK-013)
 *
 * Displays the health and status of all Edda Stack layers.
 *
 * Usage:
 *   anvil stack status           Show all layers status
 *   anvil stack status --json    Output as JSON
 */

import { Command } from 'commander';
import chalk from 'chalk';
import { GateConfigManager } from '@eddacraft/anvil-runtime';
import {
  StackConfigSchema,
  getEnabledLayerCount,
  isLayerEnabled,
  type StackConfig,
} from './config.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { blank, json, info, print, success, warning } from '../../utils/output.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { createSpinner } from '../../utils/spinner.js';

/**
 * Status command options
 */
export interface StatusOptions {
  json?: boolean;
}

/**
 * Layer status for display
 */
interface LayerStatus {
  name: string;
  enabled: boolean;
  description: string;
  status: 'active' | 'disabled' | 'unavailable';
}

/**
 * Stack status result
 */
interface StackStatusResult {
  configured: boolean;
  layers: {
    kindling: LayerStatus;
    ember: LayerStatus;
    edda: LayerStatus;
  };
  validation: {
    check_provenance_integrity: boolean;
    check_schema_compatibility: boolean;
  };
  enabledCount: number;
  configPath?: string;
}

/**
 * Get layer description
 */
function getLayerDescription(layer: 'kindling' | 'ember' | 'edda'): string {
  const descriptions: Record<string, string> = {
    kindling: 'Observation layer - captures activity without judgement',
    ember: 'Candidate layer - proposes meaning without authority',
    edda: 'Memory layer - preserves curated truths with restraint',
  };
  return descriptions[layer];
}

/**
 * Build layer status
 */
function buildLayerStatus(
  config: StackConfig | undefined,
  layer: 'kindling' | 'ember' | 'edda'
): LayerStatus {
  const enabled = isLayerEnabled(config, layer);

  return {
    name: layer.charAt(0).toUpperCase() + layer.slice(1),
    enabled,
    description: getLayerDescription(layer),
    status: enabled ? 'active' : 'disabled',
  };
}

/**
 * Get stack status from configuration
 */
function getStackStatus(workspaceRoot: string): StackStatusResult {
  const configManager = new GateConfigManager(workspaceRoot);
  const { config, path } = configManager.loadConfigWithDetails();

  const stackConfig = config.stack ? StackConfigSchema.parse(config.stack) : undefined;
  const configured = stackConfig !== undefined;

  return {
    configured,
    layers: {
      kindling: buildLayerStatus(stackConfig, 'kindling'),
      ember: buildLayerStatus(stackConfig, 'ember'),
      edda: buildLayerStatus(stackConfig, 'edda'),
    },
    validation: {
      check_provenance_integrity: stackConfig?.validation?.check_provenance_integrity ?? true,
      check_schema_compatibility: stackConfig?.validation?.check_schema_compatibility ?? true,
    },
    enabledCount: getEnabledLayerCount(stackConfig),
    configPath: path ?? undefined,
  };
}

/**
 * Format status indicator
 */
function formatStatusIndicator(status: LayerStatus['status']): string {
  switch (status) {
    case 'active':
      return chalk.green('●');
    case 'disabled':
      return chalk.dim('○');
    case 'unavailable':
      return chalk.yellow('◌');
    default:
      return chalk.dim('?');
  }
}

/**
 * Format status text
 */
function formatStatusText(status: LayerStatus['status']): string {
  switch (status) {
    case 'active':
      return chalk.green('active');
    case 'disabled':
      return chalk.dim('disabled');
    case 'unavailable':
      return chalk.yellow('unavailable');
    default:
      return chalk.dim('unknown');
  }
}

/**
 * Display status in human-readable format
 */
function displayStatus(result: StackStatusResult): void {
  blank();
  print(chalk.bold.underline('Edda Stack Status'));
  blank();

  // Configuration status
  if (!result.configured) {
    warning('Stack not configured in .anvilrc');
    print(chalk.dim('  Add a "stack" section to enable layers'));
    blank();
  }

  // Layer table header
  print(chalk.bold('Layers:'));
  blank();

  // Display each layer
  const layers = ['kindling', 'ember', 'edda'] as const;
  for (const layerName of layers) {
    const layer = result.layers[layerName];
    const indicator = formatStatusIndicator(layer.status);
    const statusText = formatStatusText(layer.status);

    print(`  ${indicator} ${chalk.cyan(layer.name.padEnd(10))} ${statusText}`);
    print(chalk.dim(`      ${layer.description}`));
    blank();
  }

  // Validation settings
  print(chalk.bold('Validation:'));
  blank();

  const provenanceIcon = result.validation.check_provenance_integrity
    ? chalk.green('✓')
    : chalk.dim('○');
  const schemaIcon = result.validation.check_schema_compatibility
    ? chalk.green('✓')
    : chalk.dim('○');

  print(`  ${provenanceIcon} Provenance integrity checking`);
  print(`  ${schemaIcon} Schema compatibility checking`);
  blank();

  // Summary
  if (result.enabledCount === 0) {
    info('No layers enabled. Enable layers in .anvilrc to start using the stack.');
  } else if (result.enabledCount === 3) {
    success(`All ${result.enabledCount} layers active`);
  } else {
    info(`${result.enabledCount} of 3 layers enabled`);
  }

  // Config path
  if (result.configPath) {
    print(chalk.dim(`\nConfiguration: ${result.configPath}`));
  }
}

/**
 * Create the status subcommand
 */
export function createStatusSubcommand(): Command {
  return new Command('status')
    .description('Show Edda Stack health and status')
    .option('--json', 'Output as JSON')
    .action(async (options: StatusOptions) => {
      if (options.json) {
        // JSON mode - no spinner, structured output
        try {
          const workspaceRoot = getWorkspaceRoot();
          const result = getStackStatus(workspaceRoot);
          json(result);
        } catch (err) {
          if (err instanceof CliError || err instanceof CliExit) throw err;
          json({
            error: err instanceof Error ? err.message : 'Unknown error',
          });
          throw new CliError(err instanceof Error ? err.message : 'Unknown error');
        }
      } else {
        // Human-readable mode
        const spinner = createSpinner('Loading stack status...');

        try {
          const workspaceRoot = getWorkspaceRoot();
          const result = getStackStatus(workspaceRoot);

          spinner.stop();
          displayStatus(result);
        } catch (err) {
          if (err instanceof CliError || err instanceof CliExit) throw err;
          spinner.fail(chalk.red('Failed to load stack status'));
          print(chalk.red('Error:'), err instanceof Error ? err.message : String(err));
          throw new CliError(err instanceof Error ? err.message : 'Failed to load stack status');
        }
      }
    });
}
