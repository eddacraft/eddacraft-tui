import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import { ProposalStore, createProposalId } from '@eddacraft/anvil-edda-stack';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { CliError, CliExit } from '../../utils/cli-error.js';

interface EmberPromoteOptions {
  json?: boolean;
  reason: string;
  by: string;
}

export function createEmberPromoteCommand(): Command {
  const command = new Command('promote');

  command
    .description('Mark an Ember proposal as promoted')
    .argument('<id>', 'Proposal ID')
    .requiredOption('--reason <reason>', 'Reason for promotion')
    .requiredOption('--by <name>', 'Who promoted the proposal')
    .option('--json', 'Output as JSON')
    .action(async (id: string, options: EmberPromoteOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const dbPath = join(workspaceRoot, '.anvil', 'ember.db');

      if (!existsSync(dbPath)) {
        const message = `No Ember database found at ${dbPath}`;
        if (options.json) {
          console.log(
            JSON.stringify(
              {
                error: message,
                database_found: false,
              },
              null,
              2
            )
          );
          throw new CliError(message);
        } else {
          console.error(chalk.yellow(message));
          throw new CliError(message);
        }
      }

      const proposalId = createProposalId(id);
      let store: ProposalStore | null = null;

      try {
        store = new ProposalStore(dbPath);
        const existing = await store.getProposal(proposalId);

        if (!existing) {
          const message = `Proposal not found: ${id}`;
          if (options.json) {
            console.log(JSON.stringify({ error: message }, null, 2));
          } else {
            console.error(chalk.red(message));
          }
          throw new CliError(message);
        }

        if (existing.status !== 'active') {
          const message = `Proposal ${id} is not active (current status: ${existing.status})`;
          if (options.json) {
            console.log(JSON.stringify({ error: message }, null, 2));
          } else {
            console.error(chalk.red(message));
          }
          throw new CliError(message);
        }

        const proposal = await store.resolveProposal(proposalId, {
          status: 'promoted',
          resolved_by: options.by,
          resolution_reason: options.reason,
        });

        if (!proposal) {
          const message = `Failed to promote proposal: ${id}`;
          if (options.json) {
            console.log(JSON.stringify({ error: message }, null, 2));
          } else {
            console.error(chalk.red(message));
          }
          throw new CliError(message);
        }

        if (options.json) {
          console.log(JSON.stringify(proposal, null, 2));
          return;
        }

        console.error(chalk.green('Proposal promoted successfully'));
        console.error(`  ${chalk.cyan('ID:')} ${proposal.id}`);
        console.error(`  ${chalk.cyan('Type:')} ${proposal.type}`);
        console.error(`  ${chalk.cyan('Summary:')} ${proposal.summary}`);
        console.error(`  ${chalk.cyan('Status:')} ${chalk.green(proposal.status)}`);
        console.error(`  ${chalk.cyan('Resolved by:')} ${options.by}`);
        console.error(`  ${chalk.cyan('Reason:')} ${options.reason}`);
        console.error('');
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        if (options.json) {
          console.log(
            JSON.stringify(
              {
                error: err instanceof Error ? err.message : 'Unknown error',
              },
              null,
              2
            )
          );
        } else {
          console.error(
            chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`)
          );
        }
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        store?.close();
      }
    });

  return command;
}
