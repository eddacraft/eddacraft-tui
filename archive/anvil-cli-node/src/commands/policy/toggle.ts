import { Command } from 'commander';
import chalk from 'chalk';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { success, error, info, print } from '../../utils/output.js';
import { PolicyConfigManager, type EnforcementLevel } from '../../services/policy-config.js';
import { formatEnforcement } from './formatting.js';

export function createPolicyDisableCommand(): Command {
  return new Command('disable')
    .description('Disable a policy (adds local override)')
    .argument('<name>')
    .action(async (name: string) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);

        const resolved = configMgr.resolvePolicies();
        const policy = resolved.find((p) => p.name === name);

        if (!policy) {
          error(`Policy '${name}' not found`);
          throw new CliError(`Policy '${name}' not found for disable`);
        }

        if (!policy.active) {
          info(`Policy '${name}' is already inactive`);
          return;
        }

        configMgr.disablePolicy(name);
        success(`Disabled policy '${name}'`);
        print(chalk.dim(`  To re-enable: anvil policy enable ${name}`));
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to disable policy: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to disable policy');
      }
    });
}

export function createPolicyEnableCommand(): Command {
  return new Command('enable')
    .description('Re-enable a disabled policy')
    .argument('<name>')
    .option('-e, --enforcement <level>', 'Enforcement level (block, warn, info, off)', 'block')
    .action(async (name: string, options: { enforcement: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);

        const validEnforcementLevels: readonly string[] = ['block', 'warn', 'info', 'off'];
        if (!validEnforcementLevels.includes(options.enforcement)) {
          error(
            `Invalid enforcement level '${options.enforcement}'. Must be one of: ${validEnforcementLevels.join(', ')}`
          );
          throw new CliError(`Invalid enforcement level: ${options.enforcement}`);
        }
        const enforcement = options.enforcement as EnforcementLevel;

        configMgr.enablePolicy(name, enforcement);
        success(`Enabled policy '${name}' with enforcement: ${formatEnforcement(enforcement)}`);
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to enable policy: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to enable policy');
      }
    });
}
