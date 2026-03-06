import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import { ProposalStore, createProposalId } from '@eddacraft/anvil-edda-stack';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { CliError, CliExit } from '../../utils/cli-error.js';

interface EmberShowOptions {
  json?: boolean;
}

export function createEmberShowCommand(): Command {
  const command = new Command('show');

  command
    .description('Show full details for an Ember proposal')
    .argument('<id>', 'Proposal ID')
    .option('--json', 'Output as JSON')
    .action(async (id: string, options: EmberShowOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const dbPath = join(workspaceRoot, '.anvil', 'ember.db');
      const proposalId = createProposalId(id);

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

      let store: ProposalStore | null = null;
      try {
        store = new ProposalStore(dbPath);
        const proposal = await store.getProposal(proposalId);

        if (!proposal) {
          const message = `Proposal not found: ${id}`;
          if (options.json) {
            console.log(JSON.stringify({ error: message }, null, 2));
          } else {
            console.error(chalk.red(message));
          }
          throw new CliError(message, 1);
        }

        if (options.json) {
          console.log(JSON.stringify(proposal, null, 2));
          return;
        }

        console.error(chalk.bold('\nEmber Proposal'));
        console.error(chalk.gray('─'.repeat(88)));
        console.error(`  ${chalk.cyan('ID:')} ${proposal.id}`);
        console.error(`  ${chalk.cyan('Type:')} ${proposal.type}`);
        console.error(`  ${chalk.cyan('Status:')} ${colourStatus(proposal.status)}`);
        console.error(`  ${chalk.cyan('Confidence:')} ${colourConfidence(proposal.confidence)}`);
        console.error(`  ${chalk.cyan('Summary:')} ${proposal.summary}`);
        console.error(
          `  ${chalk.cyan('Rationale:')} ${proposal.rationale || 'No rationale provided'}`
        );
        console.error(
          `  ${chalk.cyan('Created at:')} ${new Date(proposal.created_at).toISOString()}`
        );
        console.error(
          `  ${chalk.cyan('Expires at:')} ${new Date(proposal.expires_at).toISOString()}`
        );
        console.error(`  ${chalk.cyan('TTL days:')} ${proposal.ttl_days}`);

        console.error(chalk.bold('\nProvenance'));
        console.error(chalk.gray('─'.repeat(88)));
        console.error(
          `  ${chalk.cyan('Observation IDs:')} ${proposal.provenance.observation_ids.join(', ')}`
        );
        console.error(
          `  ${chalk.cyan('Session IDs:')} ${proposal.provenance.session_ids.join(', ')}`
        );
        console.error(
          `  ${chalk.cyan('Earliest observation:')} ${new Date(proposal.provenance.earliest_observation).toISOString()}`
        );
        console.error(
          `  ${chalk.cyan('Latest observation:')} ${new Date(proposal.provenance.latest_observation).toISOString()}`
        );

        console.error(chalk.bold('\nResolution'));
        console.error(chalk.gray('─'.repeat(88)));
        if (proposal.resolution) {
          console.error(
            `  ${chalk.cyan('Resolved at:')} ${new Date(proposal.resolution.resolved_at).toISOString()}`
          );
          console.error(
            `  ${chalk.cyan('Resolved by:')} ${proposal.resolution.resolved_by ?? 'Not provided'}`
          );
          console.error(
            `  ${chalk.cyan('Resolution reason:')} ${proposal.resolution.resolution_reason ?? 'Not provided'}`
          );
          console.error(
            `  ${chalk.cyan('Promoted to memory ID:')} ${proposal.resolution.memory_id ?? 'Not promoted to memory'}`
          );
        } else {
          console.error(`  ${chalk.gray('No resolution recorded')}`);
        }

        console.error(chalk.bold('\nMetadata'));
        console.error(chalk.gray('─'.repeat(88)));
        console.error(
          `  ${proposal.metadata ? JSON.stringify(proposal.metadata, null, 2) : 'No metadata'}`
        );
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

function colourConfidence(confidence: number): string {
  const text = confidence.toFixed(2);
  if (confidence > 0.7) {
    return chalk.green(text);
  }
  if (confidence >= 0.4) {
    return chalk.yellow(text);
  }
  return chalk.red(text);
}

function colourStatus(status: string): string {
  switch (status) {
    case 'active':
      return chalk.cyan(status);
    case 'promoted':
      return chalk.green(status);
    case 'dismissed':
      return chalk.yellow(status);
    case 'expired':
      return chalk.red(status);
    default:
      return status;
  }
}
