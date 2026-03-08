import { Command } from 'commander';
import chalk from 'chalk';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { error, print } from '../../utils/output.js';
import { PolicyConfigManager } from '../../services/policy-config.js';
import { printWhyBlock } from './formatting.js';

export function createPolicyWhyCommand(): Command {
  return new Command('why')
    .description('Explain the business reason behind a policy violation')
    .argument('<violation>')
    .action(async (violation: string) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);
        const resolved = configMgr.resolvePolicies();

        const match = resolved.find(
          (p) => p.name === violation || p.name.includes(violation) || violation.includes(p.name)
        );

        if (!match) {
          const partial = resolved.filter(
            (p) =>
              violation.toLowerCase().includes(p.name.toLowerCase().replace(/_/g, '-')) ||
              violation.toLowerCase().includes(p.name.toLowerCase().replace(/-/g, '_'))
          );

          if (partial.length === 0) {
            error(`Could not match '${violation}' to any known policy`);
            print(chalk.dim('\nAvailable policies:'));
            for (const p of resolved) {
              print(chalk.dim(`  • ${p.name}`));
            }
            throw new CliError(`Could not match '${violation}' to any known policy`);
          }

          for (const p of partial) {
            printWhyBlock(p);
          }
          return;
        }

        printWhyBlock(match);
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(
          `Failed to explain violation: ${err instanceof Error ? err.message : 'Unknown error'}`
        );
        throw new CliError('Failed to explain policy violation');
      }
    });
}
