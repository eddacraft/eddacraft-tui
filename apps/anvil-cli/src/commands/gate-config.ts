import { Command } from 'commander';
import { GateConfigManager } from '@eddacraft/anvil-runtime';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { success, error, blank, print } from '../utils/output.js';
import inquirer from 'inquirer';
import { CliError } from '../utils/cli-error.js';

export function createGateConfigCommand(): Command {
  const command = new Command('gate:config');

  command
    .description('Configure gate settings')
    .option('-l, --list', 'List current configuration')
    .option('-e, --enable <check>', 'Enable a specific check')
    .option('-d, --disable <check>', 'Disable a specific check')
    .option('-i, --interactive', 'Interactive configuration')
    .action(
      async (options: {
        list?: boolean;
        enable?: string;
        disable?: string;
        interactive?: boolean;
      }) => {
        try {
          const workspaceRoot = getWorkspaceRoot();
          const configManager = new GateConfigManager(workspaceRoot);

          if (options.list) {
            const config = configManager.loadConfig();
            print('\nCurrent Gate Configuration:');
            print('========================');
            print(`Overall Score Threshold: ${config.thresholds.overall_score}%`);
            print('\nChecks:');
            config.checks.forEach((check) => {
              const status = check.enabled ? '✓' : '✗';
              print(`  ${status} ${check.name}: ${check.description}`);
              if (check.config && Object.keys(check.config).length > 0) {
                print(`    Config: ${JSON.stringify(check.config, null, 2)}`);
              }
            });
            return;
          }

          if (options.enable) {
            configManager.enableCheck(options.enable);
            success(`Enabled check: ${options.enable}`);
            return;
          }

          if (options.disable) {
            configManager.disableCheck(options.disable);
            success(`Disabled check: ${options.disable}`);
            return;
          }

          if (options.interactive) {
            await runInteractiveConfig(configManager);
            return;
          }

          // Default: show help
          command.help();
        } catch (err) {
          error(`Configuration failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
          throw new CliError('Configuration failed');
        }
      }
    );

  return command;
}

/** Answer type for overall threshold prompt */
interface ThresholdAnswers {
  overallThreshold: number;
}

/** Answer type for individual check configuration */
interface CheckConfigAnswers {
  enabled: boolean;
  minScore?: number;
}

async function runInteractiveConfig(configManager: GateConfigManager): Promise<void> {
  const config = configManager.loadConfig();

  print('\nInteractive Gate Configuration');
  print('=============================');
  blank();

  // Configure overall threshold
  const thresholdResult = (await inquirer.prompt([
    {
      type: 'number' as const,
      name: 'overallThreshold' as const,
      message: 'Overall score threshold (0-100):',
      default: config.thresholds.overall_score,
      validate: (input: number) => (input >= 0 && input <= 100) || 'Must be between 0 and 100',
    },
  ])) as ThresholdAnswers;

  config.thresholds.overall_score = thresholdResult.overallThreshold;

  // Configure individual checks
  for (const check of config.checks) {
    const answers = (await inquirer.prompt([
      {
        type: 'confirm' as const,
        name: 'enabled' as const,
        message: `Enable ${check.name} check?`,
        default: check.enabled,
      },
      {
        type: 'number' as const,
        name: 'minScore' as const,
        message: `Minimum score for ${check.name} (0-100):`,
        default: check.config?.min_score || 80,
        validate: (input: number) => (input >= 0 && input <= 100) || 'Must be between 0 and 100',
        when: (currentAnswers: Record<string, unknown>) => Boolean(currentAnswers['enabled']),
      },
    ])) as CheckConfigAnswers;

    check.enabled = answers.enabled;
    if (answers.enabled && answers.minScore !== undefined) {
      check.config = { ...check.config, min_score: answers.minScore };
    }
  }

  configManager.saveConfig(config);
  success('Configuration saved successfully!');
}
