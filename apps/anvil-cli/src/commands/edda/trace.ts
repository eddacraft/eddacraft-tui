import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import {
  EvolutionService,
  MemoryStore,
  ProvenanceService,
  createMemoryId,
} from '@eddacraft/anvil-edda-stack';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { colourStatus } from './utils.js';

interface EddaTraceOptions {
  json?: boolean;
}

export function createEddaTraceCommand(): Command {
  const command = new Command('trace');

  command
    .description('Trace evolution chain and provenance for a memory')
    .argument('<id>', 'Memory ID to trace')
    .option('--json', 'Output as JSON')
    .action(async (id: string, options: EddaTraceOptions) => {
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
                evolution_chain: [],
                provenance: null,
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
        type: 'git',
        path: storagePath,
        format: 'yaml',
      });
      const evolutionService = new EvolutionService({ store });
      const provenanceService = new ProvenanceService({ store });
      const memoryId = createMemoryId(id);
      const spinner = options.json ? null : ora('Resolving evolution and provenance...').start();

      try {
        const evolutionChain = await evolutionService.getEvolutionChain(memoryId);
        const provenance = await provenanceService.getMemoryProvenance(memoryId);

        if (options.json) {
          console.log(
            JSON.stringify(
              {
                evolution_chain: evolutionChain,
                provenance,
              },
              null,
              2
            )
          );
          return;
        }

        spinner?.stop();

        console.error(chalk.bold('\nMemory Evolution Chain'));
        console.error(chalk.gray('─'.repeat(60)));

        if (evolutionChain.length === 0) {
          console.error(chalk.yellow(`  No evolution chain found for memory: ${id}`));
        } else {
          const orderedChain = [...evolutionChain].reverse();
          for (const [index, memory] of orderedChain.entries()) {
            console.error(
              `  [${index + 1}] ${memory.id} (${colourStatus(memory.status)}) - ${formatStatement(memory.statement)}`
            );

            if (index < orderedChain.length - 1) {
              console.error(`  ${chalk.gray('↓ superseded by')}`);
            }
          }
        }

        console.error(chalk.bold('\nProvenance'));
        console.error(chalk.gray('─'.repeat(60)));

        if (!provenance) {
          console.error(chalk.yellow(`  No provenance found for memory: ${id}`));
          console.error('');
          return;
        }

        const emberSource = provenance.memory.provenance.ember_source;
        console.error(
          `  ${chalk.cyan('Ember source:')} ${emberSource ? emberSource.proposal_id : 'None'}`
        );
        console.error(
          `  ${chalk.cyan('Ember type:')} ${emberSource ? emberSource.proposal_type : 'None'}`
        );
        console.error(
          `  ${chalk.cyan('Ember confidence:')} ${emberSource ? emberSource.confidence.toFixed(2) : 'None'}`
        );
        console.error(
          `  ${chalk.cyan('Kindling sources:')} ${provenance.memory.provenance.kindling_sources.length}`
        );
        console.error(
          `  ${chalk.cyan('Source sessions:')} ${provenance.memory.provenance.source_sessions.length}`
        );
        console.error(
          `  ${chalk.cyan('Resolution status:')} ${provenance.resolution.complete ? chalk.green('complete') : chalk.yellow('incomplete')}`
        );
        console.error(
          `  ${chalk.cyan('Missing links:')} ${formatList(provenance.resolution.missing_links)}`
        );
        console.error(`  ${chalk.cyan('Warnings:')} ${formatList(provenance.resolution.warnings)}`);
        console.error('');
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner?.fail(chalk.red('Failed to trace memory provenance'));
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

function formatStatement(statement: string): string {
  const trimmed = statement.trim();
  if (trimmed.length <= 72) {
    return `"${trimmed}"`;
  }
  return `"${trimmed.slice(0, 69)}..."`;
}

function formatList(values: string[]): string {
  if (values.length === 0) {
    return chalk.gray('None');
  }

  return values.join(', ');
}
