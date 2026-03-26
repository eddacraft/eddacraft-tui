import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { Command } from 'commander';
import chalk from 'chalk';
import {
  MemoryStore,
  MemoryStatusSchema,
  MemoryTypeSchema,
  EddaConfidenceLevelSchema,
  type MemoryStatus,
  type MemoryType,
  type EddaConfidenceLevel,
} from '@eddacraft/anvil-edda-stack';
import { createSpinner } from '../../utils/spinner.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { coercePositiveInt } from '../../utils/option-coerce.js';
import { blank, json, print } from '../../utils/output.js';
import { colourConfidence, colourStatus } from './utils.js';

interface EddaListOptions {
  json?: boolean;
  type?: string;
  status?: string;
  confidence?: string;
  since?: string;
  limit: number;
}

export function createEddaListCommand(): Command {
  const command = new Command('list');

  command
    .alias('ls')
    .description('List Edda memories with filtering')
    .option('--json', 'Output as JSON')
    .option('--type <type>', 'Filter by memory type (comma-separated for multiple)')
    .option('--status <status>', 'Filter by memory status', 'active')
    .option(
      '--confidence <level>',
      'Filter by confidence level(s) (low, medium, high; comma-separated for multiple)'
    )
    .option(
      '--since <duration>',
      'Filter by age; supports m (minutes), h (hours), d (days) (e.g. 30m, 24h, 7d, 30d)'
    )
    .option('--limit <n>', 'Maximum memories to display', parseLimit, 20)
    .action(async (options: EddaListOptions) => {
      const workspaceRoot = getWorkspaceRoot();
      const storagePath = resolve(workspaceRoot, '.anvil', 'edda');
      const parsedTypes = parseTypes(options.type);
      const parsedStatus = parseStatus(options.status ?? 'active');
      const parsedConfidence = parseConfidence(options.confidence);
      const createdAfter = parseSince(options.since);

      if (!existsSync(storagePath)) {
        const message = `No Edda storage found at ${storagePath}`;
        if (options.json) {
          json({
            error: message,
            storage_found: false,
            storage_path: storagePath,
            total: 0,
            limit: options.limit,
            has_more: false,
            filters: {
              status: parsedStatus,
              type: parsedTypes.length > 0 ? parsedTypes : null,
              confidence: parsedConfidence.length > 0 ? parsedConfidence : null,
              since: options.since ?? null,
            },
            memories: [],
          });
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

      if (options.json) {
        try {
          const result = await store.queryMemories({
            types: parsedTypes.length > 0 ? parsedTypes : undefined,
            statuses: [parsedStatus],
            confidence_levels: parsedConfidence.length > 0 ? parsedConfidence : undefined,
            created_after: createdAfter,
            include_superseded: parsedStatus === 'superseded',
            limit: options.limit,
            offset: 0,
            sort_by: 'created_at',
            sort_order: 'desc',
          });

          json({
            storage_found: true,
            storage_path: storagePath,
            total: result.total,
            limit: result.limit,
            has_more: result.has_more,
            filters: {
              status: parsedStatus,
              type: parsedTypes.length > 0 ? parsedTypes : null,
              confidence: parsedConfidence.length > 0 ? parsedConfidence : null,
              since: options.since ?? null,
            },
            memories: result.memories,
          });
          return;
        } catch (err) {
          json({
            error: err instanceof Error ? err.message : 'Unknown error',
          });
          throw new CliError(err instanceof Error ? err.message : 'Unknown error');
        }
      }

      const spinner = createSpinner('Loading Edda memories...');

      try {
        const result = await store.queryMemories({
          types: parsedTypes.length > 0 ? parsedTypes : undefined,
          statuses: [parsedStatus],
          confidence_levels: parsedConfidence.length > 0 ? parsedConfidence : undefined,
          created_after: createdAfter,
          include_superseded: parsedStatus === 'superseded',
          limit: options.limit,
          offset: 0,
          sort_by: 'created_at',
          sort_order: 'desc',
        });

        spinner.stop();
        print(chalk.bold('\nEdda Memories'));
        const filterParts = [
          `status: ${parsedStatus}`,
          `type: ${parsedTypes.length > 0 ? parsedTypes.join(', ') : 'all'}`,
        ];
        if (parsedConfidence.length > 0)
          filterParts.push(`confidence: ${parsedConfidence.join(', ')}`);
        if (options.since) filterParts.push(`since: ${options.since}`);
        print(chalk.gray(`${result.total} found  |  ${filterParts.join('  |  ')}`));
        print(chalk.gray('─'.repeat(118)));
        print(
          chalk.cyan(
            `  ${'ID'.padEnd(14)} ${'Type'.padEnd(11)} ${'Status'.padEnd(12)} ${'Confidence'.padEnd(12)} ${'Statement'.padEnd(48)} ${'Created'.padEnd(16)}`
          )
        );
        print(chalk.gray('  ' + '─'.repeat(116)));

        for (const memory of result.memories) {
          const id = truncate(memory.id, 12).padEnd(14);
          const type = memory.type.padEnd(11);
          const status = colourStatus(memory.status).padEnd(12);
          const confidence = colourConfidence(memory.confidence).padEnd(12);
          const statement = truncate(memory.statement, 46).padEnd(48);
          const created = formatRelativeTime(memory.created_at).padEnd(16);

          print(`  ${id} ${type} ${status} ${confidence} ${statement} ${created}`);
        }

        if (result.memories.length === 0) {
          print(chalk.gray('  No memories match the current filters.'));
        }

        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail(chalk.red('Failed to list Edda memories'));
        print(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      }
    });

  return command;
}

function parseLimit(value: string): number {
  return coercePositiveInt(value, '--limit');
}

function parseTypes(value?: string): MemoryType[] {
  if (!value) return [];
  return value.split(',').map((v) => {
    const trimmed = v.trim();
    const parsed = MemoryTypeSchema.safeParse(trimmed);
    if (!parsed.success) {
      throw new CliError(`Invalid memory type: ${trimmed}`);
    }
    return parsed.data;
  });
}

function parseStatus(value: string): MemoryStatus {
  const parsed = MemoryStatusSchema.safeParse(value);
  if (!parsed.success) {
    throw new CliError(`Invalid memory status: ${value}`);
  }
  return parsed.data;
}

/** @internal Exported for testing. */
export function parseConfidence(value?: string): EddaConfidenceLevel[] {
  if (!value) return [];
  return value.split(',').map((v) => {
    const trimmed = v.trim();
    const parsed = EddaConfidenceLevelSchema.safeParse(trimmed);
    if (!parsed.success) {
      throw new CliError(`Invalid confidence level: ${trimmed}. Valid values: low, medium, high`);
    }
    return parsed.data;
  });
}

/** @internal Exported for testing. */
export function parseSince(value?: string): string | undefined {
  if (!value) return undefined;
  const match = value.match(/^(\d+)([dhm])$/);
  if (!match) {
    throw new CliError(`Invalid --since format: ${value}. Use e.g. 7d, 24h, 30m`);
  }
  const amount = Number(match[1]);
  const unit = match[2];
  const now = Date.now();
  const ms = unit === 'd' ? amount * 86400000 : unit === 'h' ? amount * 3600000 : amount * 60000;
  return new Date(now - ms).toISOString();
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
