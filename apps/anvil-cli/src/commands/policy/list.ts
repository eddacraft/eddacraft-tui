import { Command } from 'commander';
import chalk from 'chalk';
import { createDebugger } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { success, error, info, warning, print, blank, data } from '../../utils/output.js';
import { PolicyLoader } from '@eddacraft/anvil-runtime';
import { PolicyConfigManager } from '../../services/policy-config.js';
import { DEFAULT_POLICY_DIR } from './constants.js';
import { formatEnforcement, formatSource, truncate } from './formatting.js';

const log = createDebugger('cli');

export function createPolicyListCommand(): Command {
  return new Command('list')
    .description('List active policies with source, enforcement level, and ownership')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .option('-a, --all', 'Include disabled and pending policies')
    .option('--json', 'Output as JSON')
    .action(async (options: { dir: string; all?: boolean; json?: boolean }) => {
      log(`policy list: dir=${options.dir} all=${options.all}`);
      try {
        const workspaceRoot = getWorkspaceRoot();
        const policyDir = options.dir;

        const configMgr = new PolicyConfigManager(workspaceRoot);
        const resolved = configMgr.resolvePolicies();

        const loader = new PolicyLoader();
        const regoResult = await loader.loadPolicies(workspaceRoot, { policyDir });
        const regoByName = new Map(regoResult.policies.map((p) => [p.name, p]));

        const allPolicies = [...resolved];
        for (const rego of regoResult.policies) {
          if (!allPolicies.some((p) => p.name === rego.name)) {
            allPolicies.push({
              name: rego.name,
              source: 'starter',
              enforcement: 'block',
              active: true,
              hasRegoFile: true,
              regoPath: rego.path,
            });
          }
        }

        const displayPolicies = options.all ? allPolicies : allPolicies.filter((p) => p.active);

        if (displayPolicies.length === 0) {
          info('No policies found');
          print(chalk.dim('\nRun `anvil policy init` to create example policies'));
          return;
        }

        if (options.json) {
          data(JSON.stringify(displayPolicies, null, 2));
          return;
        }

        print(chalk.bold('\nPolicies:\n'));

        print(
          chalk.dim('  ') +
            chalk.bold('Name'.padEnd(22)) +
            chalk.bold('Source'.padEnd(10)) +
            chalk.bold('Enforce'.padEnd(10)) +
            chalk.bold('Owner'.padEnd(18)) +
            chalk.bold('Reason')
        );
        print(chalk.dim('  ' + '─'.repeat(90)));

        for (const p of displayPolicies) {
          const rego = regoByName.get(p.name);
          const tests = rego?.hasTests ? chalk.green(' ✓') : '';

          print(
            '  ' +
              p.name.padEnd(22).replace(p.name, p.active ? chalk.cyan(p.name) : chalk.dim(p.name)) +
              (p.source as string).padEnd(10).replace(p.source, formatSource(p.source)) +
              (p.enforcement as string)
                .padEnd(10)
                .replace(p.enforcement, formatEnforcement(p.enforcement)) +
              chalk.dim((p.owner ?? '-').padEnd(18)) +
              chalk.dim(truncate(p.reason ?? '', 40)) +
              tests
          );

          if (p.effective && !p.active) {
            print(chalk.dim(`                      effective: ${p.effective}`));
          }
        }

        blank();

        const activeCount = allPolicies.filter((p) => p.active).length;
        const totalCount = allPolicies.length;
        if (options.all) {
          success(
            `${activeCount} active, ${totalCount - activeCount} inactive (${totalCount} total)`
          );
        } else {
          success(`${activeCount} active policies`);
          if (totalCount > activeCount) {
            print(chalk.dim(`  ${totalCount - activeCount} more hidden. Use --all to show.`));
          }
        }

        const config = configMgr.load();
        if (config.policies?.org) {
          blank();
          info(
            `Org source: ${chalk.cyan(config.policies.org.source)}${config.policies.org.ref ? ` @ ${config.policies.org.ref}` : ''}`
          );
        }

        if (regoResult.errors.length > 0) {
          blank();
          warning(`${regoResult.errors.length} policies failed to load:`);
          for (const err of regoResult.errors) {
            print(chalk.red(`  • ${err.path}: ${err.error}`));
          }
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to list policies: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to list policies');
      }
    });
}
