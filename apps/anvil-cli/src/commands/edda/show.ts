import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import { MemoryStore, createMemoryId } from '@eddacraft/anvil-edda-stack';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { blank, data, print } from '../../utils/output.js';
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
          data(
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
          print(chalk.yellow(message));
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
            data(JSON.stringify({ error: message }, null, 2));
          } else {
            print(chalk.red(message));
          }
          throw new CliError(message, 1);
        }

        if (options.json) {
          data(JSON.stringify(memory, null, 2));
          return;
        }

        print(chalk.bold('\nMemory'));
        print(chalk.gray('─'.repeat(90)));
        print(`  ${chalk.cyan('ID:')} ${memory.id}`);
        print(`  ${chalk.cyan('Type:')} ${memory.type}`);
        print(`  ${chalk.cyan('Status:')} ${colourStatus(memory.status)}`);
        print(`  ${chalk.cyan('Confidence:')} ${colourConfidence(memory.confidence)}`);
        print(`  ${chalk.cyan('Statement:')} ${memory.statement}`);

        print(chalk.bold('\nContext'));
        print(chalk.gray('─'.repeat(90)));
        print(`  ${chalk.cyan('When:')} ${memory.context.when}`);
        print(`  ${chalk.cyan('Why:')} ${memory.context.why}`);
        print(
          `  ${chalk.cyan('Conditions:')} ${formatList(memory.context.conditions, 'None recorded')}`
        );
        print(`  ${chalk.cyan('Scope:')} ${memory.context.scope ?? 'Not specified'}`);
        print(`  ${chalk.cyan('Tags:')} ${formatList(memory.context.tags, 'No tags')}`);

        print(chalk.bold('\nProvenance'));
        print(chalk.gray('─'.repeat(90)));
        print(`  ${chalk.cyan('Kindling sources:')} ${memory.provenance.kindling_sources.length}`);
        print(
          `  ${chalk.cyan('Ember proposal:')} ${memory.provenance.ember_source?.proposal_id ?? 'Not linked'}`
        );
        print(
          `  ${chalk.cyan('Source sessions:')} ${formatList(memory.provenance.source_sessions, 'None recorded')}`
        );

        print(chalk.bold('\nAttribution'));
        print(chalk.gray('─'.repeat(90)));
        print(`  ${chalk.cyan('Actor:')} ${memory.attribution.actor}`);
        print(`  ${chalk.cyan('Timestamp:')} ${memory.attribution.timestamp}`);
        print(`  ${chalk.cyan('Method:')} ${memory.attribution.method}`);
        print(`  ${chalk.cyan('Reason:')} ${memory.attribution.reason ?? 'Not provided'}`);

        print(chalk.bold('\nEvolution'));
        print(chalk.gray('─'.repeat(90)));
        print(`  ${chalk.cyan('Supersedes:')} ${formatList(memory.evolution.supersedes, 'None')}`);
        print(
          `  ${chalk.cyan('Superseded by:')} ${memory.evolution.superseded_by ?? 'Not superseded'}`
        );
        print(`  ${chalk.cyan('Retired at:')} ${memory.evolution.retired_at ?? 'Not retired'}`);
        print(
          `  ${chalk.cyan('Retired reason:')} ${memory.evolution.retired_reason ?? 'Not retired'}`
        );

        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;

        if (options.json) {
          data(
            JSON.stringify(
              {
                error: err instanceof Error ? err.message : 'Unknown error',
              },
              null,
              2
            )
          );
        } else {
          print(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
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
