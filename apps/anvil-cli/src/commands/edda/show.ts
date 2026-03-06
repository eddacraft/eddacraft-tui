import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import { MemoryStore, createMemoryId } from '@eddacraft/anvil-edda-stack';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { colourConfidence, colourStatus } from './utils.js';

interface EddaShowOptions {
  json?: boolean;
}

export function createEddaShowCommand(): Command {
  const command = new Command('show');

  command
    .description('Show full details for an Edda memory')
    .argument('<id>', 'Memory ID')
    .option('--json', 'Output as JSON')
    .action(async (id: string, options: EddaShowOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const storagePath = resolve(workspaceRoot, '.anvil', 'edda');

      if (!existsSync(storagePath)) {
        const message = `No Edda storage found at ${storagePath}`;
        if (options.json) {
          console.log(
            JSON.stringify(
              {
                error: message,
                storage_found: false,
              },
              null,
              2
            )
          );
        } else {
          console.error(chalk.yellow(message));
        }
        throw new CliError(message);
      }

      const store = new MemoryStore({
        type: 'git' as const,
        path: storagePath,
        format: 'yaml' as const,
      });

      try {
        const memory = await store.getMemory(createMemoryId(id));

        if (!memory) {
          const message = `Memory not found: ${id}`;
          if (options.json) {
            console.log(JSON.stringify({ error: message }, null, 2));
          } else {
            console.error(chalk.red(message));
          }
          throw new CliError(message, 1);
        }

        if (options.json) {
          console.log(JSON.stringify(memory, null, 2));
          return;
        }

        console.error(chalk.bold('\nMemory'));
        console.error(chalk.gray('─'.repeat(90)));
        console.error(`  ${chalk.cyan('ID:')} ${memory.id}`);
        console.error(`  ${chalk.cyan('Type:')} ${memory.type}`);
        console.error(`  ${chalk.cyan('Status:')} ${colourStatus(memory.status)}`);
        console.error(`  ${chalk.cyan('Confidence:')} ${colourConfidence(memory.confidence)}`);
        console.error(`  ${chalk.cyan('Statement:')} ${memory.statement}`);

        console.error(chalk.bold('\nContext'));
        console.error(chalk.gray('─'.repeat(90)));
        console.error(`  ${chalk.cyan('When:')} ${memory.context.when}`);
        console.error(`  ${chalk.cyan('Why:')} ${memory.context.why}`);
        console.error(
          `  ${chalk.cyan('Conditions:')} ${formatList(memory.context.conditions, 'None recorded')}`
        );
        console.error(`  ${chalk.cyan('Scope:')} ${memory.context.scope ?? 'Not specified'}`);
        console.error(`  ${chalk.cyan('Tags:')} ${formatList(memory.context.tags, 'No tags')}`);

        console.error(chalk.bold('\nProvenance'));
        console.error(chalk.gray('─'.repeat(90)));
        console.error(
          `  ${chalk.cyan('Kindling sources:')} ${memory.provenance.kindling_sources.length}`
        );
        console.error(
          `  ${chalk.cyan('Ember proposal:')} ${memory.provenance.ember_source?.proposal_id ?? 'Not linked'}`
        );
        console.error(
          `  ${chalk.cyan('Source sessions:')} ${formatList(memory.provenance.source_sessions, 'None recorded')}`
        );

        console.error(chalk.bold('\nAttribution'));
        console.error(chalk.gray('─'.repeat(90)));
        console.error(`  ${chalk.cyan('Actor:')} ${memory.attribution.actor}`);
        console.error(`  ${chalk.cyan('Timestamp:')} ${memory.attribution.timestamp}`);
        console.error(`  ${chalk.cyan('Method:')} ${memory.attribution.method}`);
        console.error(`  ${chalk.cyan('Reason:')} ${memory.attribution.reason ?? 'Not provided'}`);

        console.error(chalk.bold('\nEvolution'));
        console.error(chalk.gray('─'.repeat(90)));
        console.error(
          `  ${chalk.cyan('Supersedes:')} ${formatList(memory.evolution.supersedes, 'None')}`
        );
        console.error(
          `  ${chalk.cyan('Superseded by:')} ${memory.evolution.superseded_by ?? 'Not superseded'}`
        );
        console.error(
          `  ${chalk.cyan('Retired at:')} ${memory.evolution.retired_at ?? 'Not retired'}`
        );
        console.error(
          `  ${chalk.cyan('Retired reason:')} ${memory.evolution.retired_reason ?? 'Not retired'}`
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
      }
    });

  return command;
}

function formatList(values: string[], fallback: string): string {
  if (values.length === 0) {
    return fallback;
  }
  return values.join(', ');
}
