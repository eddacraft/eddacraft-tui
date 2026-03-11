import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import { ProposalStore, createProposalId } from '@eddacraft/anvil-edda-stack';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { blank, json, print } from '../../utils/output.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { colourConfidence, colourStatus } from './utils.js';

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
          json({ error: message, database_found: false });
        } else {
          print(chalk.red(message));
        }
        throw new CliError(message);
      }

      let store: ProposalStore | null = null;
      try {
        store = new ProposalStore(dbPath);
        const proposal = await store.getProposal(proposalId);

        if (!proposal) {
          const message = `Proposal not found: ${id}`;
          if (options.json) {
            json({ error: message });
          } else {
            print(chalk.red(message));
          }
          throw new CliError(message, 1);
        }

        if (options.json) {
          json(proposal);
          return;
        }

        print(chalk.bold('\nEmber Proposal'));
        print(chalk.gray('─'.repeat(88)));
        print(`  ${chalk.cyan('ID:')} ${proposal.id}`);
        print(`  ${chalk.cyan('Type:')} ${proposal.type}`);
        print(`  ${chalk.cyan('Status:')} ${colourStatus(proposal.status)}`);
        print(`  ${chalk.cyan('Confidence:')} ${colourConfidence(proposal.confidence)}`);
        print(`  ${chalk.cyan('Summary:')} ${proposal.summary}`);
        print(`  ${chalk.cyan('Rationale:')} ${proposal.rationale || 'No rationale provided'}`);
        print(`  ${chalk.cyan('Created at:')} ${new Date(proposal.created_at).toISOString()}`);
        print(`  ${chalk.cyan('Expires at:')} ${new Date(proposal.expires_at).toISOString()}`);
        print(`  ${chalk.cyan('TTL days:')} ${proposal.ttl_days}`);

        print(chalk.bold('\nProvenance'));
        print(chalk.gray('─'.repeat(88)));
        print(
          `  ${chalk.cyan('Observation IDs:')} ${proposal.provenance.observation_ids.join(', ')}`
        );
        print(`  ${chalk.cyan('Session IDs:')} ${proposal.provenance.session_ids.join(', ')}`);
        print(
          `  ${chalk.cyan('Earliest observation:')} ${new Date(proposal.provenance.earliest_observation).toISOString()}`
        );
        print(
          `  ${chalk.cyan('Latest observation:')} ${new Date(proposal.provenance.latest_observation).toISOString()}`
        );

        print(chalk.bold('\nResolution'));
        print(chalk.gray('─'.repeat(88)));
        if (proposal.resolution) {
          print(
            `  ${chalk.cyan('Resolved at:')} ${new Date(proposal.resolution.resolved_at).toISOString()}`
          );
          print(
            `  ${chalk.cyan('Resolved by:')} ${proposal.resolution.resolved_by ?? 'Not provided'}`
          );
          print(
            `  ${chalk.cyan('Resolution reason:')} ${proposal.resolution.resolution_reason ?? 'Not provided'}`
          );
          print(
            `  ${chalk.cyan('Promoted to memory ID:')} ${proposal.resolution.memory_id ?? 'Not promoted to memory'}`
          );
        } else {
          print(`  ${chalk.gray('No resolution recorded')}`);
        }

        print(chalk.bold('\nMetadata'));
        print(chalk.gray('─'.repeat(88)));
        print(
          `  ${proposal.metadata ? JSON.stringify(proposal.metadata, null, 2) : 'No metadata'}`
        );
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        store?.close();
      }
    });

  return command;
}
