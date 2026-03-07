import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import {
  ProposalStore,
  ProposalStatusSchema,
  ProposalTypeSchema,
  type ProposalStatus,
  type ProposalType,
} from '@eddacraft/anvil-edda-stack';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { coercePositiveInt } from '../../utils/option-coerce.js';

interface EmberListOptions {
  json?: boolean;
  type?: string;
  status: string;
  limit: number;
}

export function createEmberListCommand(): Command {
  const command = new Command('list');

  command
    .alias('ls')
    .description('List Ember proposals with filtering')
    .option('--json', 'Output as JSON')
    .option('--type <type>', 'Filter by proposal type (comma-separated for multiple)')
    .option('--status <status>', 'Filter by proposal status', 'active')
    .option('--limit <n>', 'Maximum proposals to display', parseLimit, 20)
    .action(async (options: EmberListOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const dbPath = join(workspaceRoot, '.anvil', 'ember.db');

      const parsedTypes = parseTypes(options.type);
      const parsedStatus = parseStatus(options.status);

      if (!existsSync(dbPath)) {
        const message = `No Ember database found at ${dbPath}`;
        if (options.json) {
          console.log(
            JSON.stringify(
              {
                error: message,
                database_found: false,
                database_path: dbPath,
                total: 0,
                proposals: [],
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

      const spinner = options.json ? null : ora('Loading Ember proposals...').start();
      let store: ProposalStore | null = null;

      try {
        store = new ProposalStore(dbPath);
        const result = await store.queryProposals({
          types: parsedTypes.length > 0 ? parsedTypes : undefined,
          statuses: [parsedStatus],
          include_expired: parsedStatus === 'expired',
          limit: options.limit,
          offset: 0,
          sort_by: 'created_at',
          sort_order: 'desc',
        });

        if (options.json) {
          console.log(
            JSON.stringify(
              {
                database_found: true,
                database_path: dbPath,
                total: result.total,
                limit: result.limit,
                has_more: result.has_more,
                filters: {
                  status: parsedStatus,
                  type: parsedTypes.length > 0 ? parsedTypes : null,
                },
                proposals: result.proposals,
              },
              null,
              2
            )
          );
          return;
        }

        spinner?.stop();
        console.error(chalk.bold('\nEmber Proposals'));
        console.error(
          chalk.gray(
            `${result.total} found  |  status: ${parsedStatus}  |  type: ${parsedTypes.length > 0 ? parsedTypes.join(', ') : 'all'}`
          )
        );
        console.error(chalk.gray('─'.repeat(124)));
        console.error(
          chalk.cyan(
            `  ${'ID'.padEnd(14)} ${'Type'.padEnd(11)} ${'Status'.padEnd(10)} ${'Confidence'.padEnd(12)} ${'Summary'.padEnd(34)} ${'Created'.padEnd(16)} ${'Expires'.padEnd(16)}`
          )
        );
        console.error(chalk.gray('  ' + '─'.repeat(122)));

        for (const proposal of result.proposals) {
          const id = truncate(proposal.id, 12).padEnd(14);
          const type = proposal.type.padEnd(11);
          const status = colourStatus(proposal.status).padEnd(10);
          const confidence = colourConfidence(proposal.confidence).padEnd(12);
          const summary = truncate(proposal.summary, 32).padEnd(34);
          const created = formatRelativeTime(proposal.created_at).padEnd(16);
          const expires = formatRelativeTime(proposal.expires_at).padEnd(16);

          console.error(`  ${id} ${type} ${status} ${confidence} ${summary} ${created} ${expires}`);
        }

        if (result.proposals.length === 0) {
          console.error(chalk.gray('  No proposals match the current filters.'));
        }

        console.error('');
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) {
          throw err;
        }

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
          spinner?.fail(chalk.red('Failed to list Ember proposals'));
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

function parseLimit(value: string): number {
  return coercePositiveInt(value, '--limit');
}

function parseTypes(value?: string): ProposalType[] {
  if (!value) return [];
  return value.split(',').map((v) => {
    const trimmed = v.trim();
    const parsed = ProposalTypeSchema.safeParse(trimmed);
    if (!parsed.success) {
      throw new CliError(`Invalid proposal type: ${trimmed}`);
    }
    return parsed.data;
  });
}

function parseStatus(value: string): ProposalStatus {
  const parsed = ProposalStatusSchema.safeParse(value);
  if (!parsed.success) {
    throw new CliError(`Invalid proposal status: ${value}`);
  }
  return parsed.data;
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

function colourStatus(status: ProposalStatus): string {
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

function truncate(value: string, width: number): string {
  if (value.length <= width) return value;
  return `${value.slice(0, width - 2)}..`;
}

function formatRelativeTime(value: string): string {
  const date = new Date(value);
  const diffMs = Date.now() - date.getTime();
  const absMs = Math.abs(diffMs);
  const minute = 60 * 1000;
  const hour = 60 * minute;
  const day = 24 * hour;

  if (absMs < minute) {
    return diffMs >= 0 ? 'just now' : 'soon';
  }

  if (absMs < hour) {
    const minutes = Math.floor(absMs / minute);
    return diffMs >= 0 ? `${minutes}m ago` : `in ${minutes}m`;
  }

  if (absMs < day) {
    const hours = Math.floor(absMs / hour);
    return diffMs >= 0 ? `${hours}h ago` : `in ${hours}h`;
  }

  const days = Math.floor(absMs / day);
  return diffMs >= 0 ? `${days}d ago` : `in ${days}d`;
}
