import type { ZodType } from 'zod';
import {
  AuditResponseSchema,
  type AuditItem,
  type AuditResponse,
} from '@eddacraft/admin-contracts';
import { AdminClient } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, renderTable, type Row } from '../format.js';

export type { AuditItem, AuditResponse };

export interface AuditOptions extends ConfigFlags {
  action?: string;
  filterActor?: string;
  limit?: number;
  offset?: number;
  json?: boolean;
}

export interface AdminReader {
  get<T>(
    path: string,
    query?: Record<string, string | number | boolean | undefined>,
    schema?: ZodType<T>
  ): Promise<T>;
}

export interface AuditDeps {
  createClient?: (cfg: AdminConfig) => AdminReader;
  stdout?: (chunk: string) => void;
}

export async function runAuditCommand(
  options: AuditOptions = {},
  deps: AuditDeps = {}
): Promise<void> {
  const config = resolveConfig({
    key: options.key,
    url: options.url,
    actor: options.actor,
  });
  const client: AdminReader = deps.createClient?.(config) ?? new AdminClient(config);
  const stdout = deps.stdout ?? ((chunk) => process.stdout.write(chunk));

  const query: Record<string, string | number | undefined> = {};
  if (options.action !== undefined) query.action = options.action;
  if (options.filterActor !== undefined) query.actor = options.filterActor;
  if (options.limit !== undefined) query.limit = options.limit;
  if (options.offset !== undefined) query.offset = options.offset;

  const result = await client.get('/admin/audit', query, AuditResponseSchema);

  if (options.json) {
    stdout(formatJson(result) + '\n');
    return;
  }

  if (result.items.length === 0) {
    stdout('No audit entries.\n');
    return;
  }

  const rows: Row[] = result.items.map((item) => ({ ...item }));
  const table = renderTable(rows, [
    {
      key: 'created_at',
      header: 'WHEN',
      format: (v) => String(v).slice(0, 19).replace('T', ' '),
    },
    { key: 'action', header: 'ACTION' },
    { key: 'actor', header: 'ACTOR' },
    {
      key: 'metadata',
      header: 'METADATA',
      format: (v) => {
        if (v == null || typeof v !== 'object') return '';
        const obj = v as Record<string, unknown>;
        if (Object.keys(obj).length === 0) return '';
        return JSON.stringify(obj);
      },
    },
  ]);
  stdout(table + '\n');
  stdout(`\nShowing ${result.items.length} of ${result.total}\n`);
}
