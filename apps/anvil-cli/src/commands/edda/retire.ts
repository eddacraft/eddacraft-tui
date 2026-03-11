import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import { EvolutionService, MemoryStore, createMemoryId } from '@eddacraft/anvil-edda-stack';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { blank, json, print } from '../../utils/output.js';
import { colourStatus } from './utils.js';
import { createSpinner } from '../../utils/spinner.js';

interface EddaRetireOptions {
  reason: string;
  by: string;
  json?: boolean;
}

export function createEddaRetireCommand(): Command {
  const command = new Command('retire');

  command
    .description('Retire an Edda memory')
    .argument('<id>', 'Memory ID to retire')
    .requiredOption('--reason <reason>', 'Why this is being retired')
    .requiredOption('--by <name>', 'Who is retiring')
    .option('--json', 'Output as JSON')
    .action(async (id: string, options: EddaRetireOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const actor = options.by.trim();
      if (actor.length === 0) {
        const message = '--by must not be empty';
        if (options.json) {
          json({ error: message });
        } else {
          print(chalk.red(message));
        }
        throw new CliError(message);
      }
      if (actor.length > 100) {
        const message = '--by must be 100 characters or fewer';
        if (options.json) {
          json({ error: message });
        } else {
          print(chalk.red(message));
        }
        throw new CliError(message);
      }

      const storagePath = resolve(workspaceRoot, '.anvil', 'edda');

      if (!existsSync(storagePath)) {
        const message = `No Edda storage found at ${storagePath}`;
        if (options.json) {
          json({ error: message, storage_found: false });
        } else {
          print(chalk.red(message));
        }
        throw new CliError(message);
      }

      const store = new MemoryStore({
        type: 'git',
        path: storagePath,
        format: 'yaml',
      });
      const evolutionService = new EvolutionService({ store });
      const spinner = options.json ? null : createSpinner('Retiring Edda memory...');

      try {
        const retiredMemory = await evolutionService.retireMemory(createMemoryId(id), {
          reason: options.reason,
          retired_by: actor,
        });

        if (!retiredMemory) {
          const message = `Memory not found: ${id}`;
          if (options.json) {
            json({ error: message });
          } else {
            spinner?.fail(chalk.red(message));
          }
          throw new CliError(message);
        }

        if (options.json) {
          json(retiredMemory);
          return;
        }

        spinner?.stop();
        print(chalk.green('Memory retired successfully'));
        print(`  ${chalk.cyan('ID:')} ${retiredMemory.id}`);
        print(`  ${chalk.cyan('Status:')} ${colourStatus(retiredMemory.status)}`);
        print(`  ${chalk.cyan('Reason:')} ${options.reason}`);
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner?.fail(chalk.red('Failed to retire memory'));
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      }
    });

  return command;
}
