import { AdminClient } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, renderTable, type Row } from '../format.js';

export interface ListOptions extends ConfigFlags {
  status?: string;
  source?: string;
  limit?: string;
  offset?: string;
  json?: boolean;
}

export interface WaitlistItem {
  email: string;
  name: string | null;
  source: string;
  created_at: string;
  approved_at: string | null;
}

export interface WaitlistResponse {
  total: number;
  items: WaitlistItem[];
}

export interface AdminReader {
  get<T>(path: string, query?: Record<string, string | number | boolean | undefined>): Promise<T>;
}

export interface ListDeps {
  createClient?: (cfg: AdminConfig) => AdminReader;
  stdout?: (chunk: string) => void;
}

export async function runListCommand(
  options: ListOptions = {},
  deps: ListDeps = {}
): Promise<void> {
  const config = resolveConfig({
    key: options.key,
    url: options.url,
    actor: options.actor,
  });
  const client: AdminReader = deps.createClient?.(config) ?? new AdminClient(config);
  const stdout = deps.stdout ?? ((chunk) => process.stdout.write(chunk));

  const query: Record<string, string | number | undefined> = {};
  if (options.status !== undefined) query.status = options.status;
  if (options.source !== undefined) query.source = options.source;
  if (options.limit !== undefined) query.limit = Number(options.limit);
  if (options.offset !== undefined) query.offset = Number(options.offset);

  const result = await client.get<WaitlistResponse>('/admin/waitlist', query);

  if (options.json) {
    stdout(formatJson(result) + '\n');
    return;
  }

  if (result.items.length === 0) {
    stdout('No waitlist entries.\n');
    return;
  }

  const rows: Row[] = result.items.map((item) => ({ ...item }));
  const table = renderTable(rows, [
    { key: 'email', header: 'EMAIL' },
    { key: 'name', header: 'NAME' },
    { key: 'source', header: 'SOURCE' },
    { key: 'created_at', header: 'CREATED', format: (v) => String(v).slice(0, 10) },
    {
      key: 'approved_at',
      header: 'APPROVED',
      format: (v) => (v == null ? '—' : String(v).slice(0, 10)),
    },
  ]);
  stdout(table + '\n');
  stdout(`\nShowing ${result.items.length} of ${result.total}\n`);
}
