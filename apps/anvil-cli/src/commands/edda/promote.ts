import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import {
  EddaConfidenceLevelSchema,
  MemoryStore,
  MemoryTypeSchema,
  PromotionService,
  createProposalId,
  type EddaConfidenceLevel,
  type MemoryType,
  type PromoteProposalInput,
} from '@eddacraft/anvil-edda-stack';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { blank, data, print } from '../../utils/output.js';
import { colourStatus } from './utils.js';

interface EddaPromoteOptions {
  reason: string;
  by: string;
  confidence: string;
  type: string;
  why?: string;
  conditions?: string;
  scope?: string;
  tags?: string;
  statement?: string;
  json?: boolean;
}

export function createEddaPromoteCommand(): Command {
  const command = new Command('promote');

  command
    .description('Promote an Ember candidate to Edda canonical memory')
    .argument('<candidate_id>', 'The Ember proposal ID to promote')
    .requiredOption('--reason <reason>', 'Why this is being promoted')
    .requiredOption('--by <name>', 'Who is promoting')
    .requiredOption('--confidence <level>', 'Confidence level: low, medium, high')
    .requiredOption(
      '--type <type>',
      'Memory type: decision, pattern, constraint, warning, doctrine, lesson'
    )
    .option('--why <why>', 'Context: rationale for preserving this memory')
    .option('--conditions <conditions>', 'Comma-separated applicability conditions')
    .option('--scope <scope>', 'Where this applies')
    .option('--tags <tags>', 'Comma-separated tags')
    .option('--statement <statement>', 'Override statement (defaults to proposal summary)')
    .option('--json', 'Output as JSON')
    .action(async (candidateId: string, options: EddaPromoteOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const actor = options.by.trim();
      if (actor.length === 0) {
        const message = '--by must not be empty';
        if (options.json) {
          data(JSON.stringify({ error: message }, null, 2));
        } else {
          print(chalk.red(message));
        }
        throw new CliError(message);
      }
      if (actor.length > 100) {
        const message = '--by must be 100 characters or fewer';
        if (options.json) {
          data(JSON.stringify({ error: message }, null, 2));
        } else {
          print(chalk.red(message));
        }
        throw new CliError(message);
      }

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

      const parsedConfidence = parseConfidence(options.confidence);
      const parsedType = parseMemoryType(options.type);
      const store = new MemoryStore({
        type: 'git',
        path: storagePath,
        format: 'yaml',
      });
      const promotionService = new PromotionService({ store });
      const input: PromoteProposalInput = {
        proposal_id: createProposalId(candidateId),
        type: parsedType,
        confidence: parsedConfidence,
        promoted_by: actor,
        reason: options.reason,
        statement: options.statement,
        context: {
          when: new Date().toISOString(),
          why: options.why ?? options.reason,
          conditions: splitCsv(options.conditions),
          scope: options.scope,
          tags: splitCsv(options.tags),
        },
      };

      const spinner = options.json
        ? null
        : ora('Promoting Ember candidate to Edda memory...').start();

      try {
        const memory = await promotionService.promoteProposal(input);

        if (options.json) {
          data(JSON.stringify(memory, null, 2));
          return;
        }

        spinner?.stop();
        print(chalk.green('Memory promoted successfully'));
        print(`  ${chalk.cyan('ID:')} ${memory.id}`);
        print(`  ${chalk.cyan('Type:')} ${memory.type}`);
        print(`  ${chalk.cyan('Statement:')} ${memory.statement}`);
        print(`  ${chalk.cyan('Status:')} ${colourStatus(memory.status)}`);
        print(`  ${chalk.cyan('Confidence:')} ${memory.confidence}`);
        print(`  ${chalk.cyan('Promoted by:')} ${actor}`);
        print(`  ${chalk.cyan('Reason:')} ${options.reason}`);
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner?.fail(chalk.red('Failed to promote memory'));
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

function parseConfidence(value: string): EddaConfidenceLevel {
  const parsed = EddaConfidenceLevelSchema.safeParse(value);
  if (!parsed.success) {
    throw new CliError(`Invalid confidence level: ${value}`);
  }
  return parsed.data;
}

function parseMemoryType(value: string): MemoryType {
  const parsed = MemoryTypeSchema.safeParse(value);
  if (!parsed.success) {
    throw new CliError(`Invalid memory type: ${value}`);
  }
  return parsed.data;
}

function splitCsv(value?: string): string[] {
  if (!value) {
    return [];
  }

  return value
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}
