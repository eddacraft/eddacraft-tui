import { AdminClient, AdminError } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, formatSuccess, renderTable, type Row } from '../format.js';
import { defaultPrompt, isInteractiveTTY } from '../prompt.js';

export const MIGRATION_SOURCES = ['import', 'website', 'manual'] as const;
export type MigrationSource = (typeof MIGRATION_SOURCES)[number];

export interface SendMigrationOptions extends ConfigFlags {
  source?: MigrationSource;
  limit?: number;
  dryRun?: boolean;
  yes?: boolean;
  json?: boolean;
}

export interface MigrationRecipient {
  email: string;
  name: string | null;
}

export interface DryRunResponse {
  dryRun: true;
  source: MigrationSource;
  count: number;
  recipients: MigrationRecipient[];
}

export interface SendResultEntry {
  email: string;
  sent: boolean;
  error?: string;
}

export interface SendResponse {
  source: MigrationSource;
  total: number;
  sent: number;
  failed: number;
  results: SendResultEntry[];
}

export type SendMigrationResponse = DryRunResponse | SendResponse;

export interface AdminWriter {
  post<T>(path: string, body?: unknown): Promise<T>;
}

export interface SendMigrationDeps {
  createClient?: (cfg: AdminConfig) => AdminWriter;
  stdout?: (chunk: string) => void;
  stderr?: (chunk: string) => void;
  prompt?: (message: string) => Promise<string>;
  isTTY?: boolean;
}

export async function runSendMigrationCommand(
  options: SendMigrationOptions = {},
  deps: SendMigrationDeps = {}
): Promise<void> {
  if (options.limit !== undefined) {
    if (!Number.isInteger(options.limit) || options.limit < 1 || options.limit > 100) {
      throw new AdminError('--limit must be an integer between 1 and 100', 64);
    }
  }

  const source: MigrationSource = options.source ?? 'import';
  const limit = options.limit ?? 20;
  const dryRun = options.dryRun ?? true;

  const config = resolveConfig({
    key: options.key,
    url: options.url,
    actor: options.actor,
  });
  const client: AdminWriter = deps.createClient?.(config) ?? new AdminClient(config);
  const stdout = deps.stdout ?? ((chunk) => process.stdout.write(chunk));
  const stderr = deps.stderr ?? ((chunk) => process.stderr.write(chunk));
  const isTTY = deps.isTTY ?? isInteractiveTTY();

  if (dryRun) {
    const preview = await client.post<DryRunResponse>('/admin/send-migration', {
      source,
      dryRun: true,
      limit,
    });
    renderResult(preview, { stdout, json: !!options.json });
    return;
  }

  if (!options.yes) {
    if (!isTTY) {
      throw new AdminError(
        'refusing to send migration without --yes in a non-interactive session',
        4
      );
    }

    const preview = await client.post<DryRunResponse>('/admin/send-migration', {
      source,
      dryRun: true,
      limit,
    });

    if (preview.count === 0) {
      stdout('No recipients match the filter. Nothing to send.\n');
      return;
    }

    // #948: when --json is set, suppress the ASCII preview table so stdout
    // remains valid JSON. Print a compact one-liner to stderr instead.
    if (options.json) {
      stderr(`preview: ${preview.count} recipient(s) — pass --yes to skip this prompt\n`);
    } else {
      stderr(
        `About to send migration email to ${preview.count} recipient(s) (source: ${source}).\n` +
          renderRecipientsTable(preview.recipients) +
          '\n'
      );
    }
    const prompt = deps.prompt ?? defaultPrompt;
    // #947: PromptEOFError propagates naturally — top-level handler exits with code 4.
    const answer = (await prompt(`Continue? [y/N] `)).trim().toLowerCase();
    if (answer !== 'y' && answer !== 'yes') {
      stdout('Aborted.\n');
      return;
    }
  }

  const result = await client.post<SendResponse>('/admin/send-migration', {
    source,
    dryRun: false,
    limit,
  });
  renderResult(result, { stdout, json: !!options.json });

  if (result.failed > 0) {
    throw new AdminError(`${result.failed} of ${result.total} recipient(s) failed to send`, 1);
  }
}

function renderRecipientsTable(recipients: MigrationRecipient[]): string {
  if (recipients.length === 0) return '(none)';
  const rows: Row[] = recipients.map((r) => ({ email: r.email, name: r.name ?? '' }));
  return renderTable(rows, [
    { key: 'email', header: 'EMAIL' },
    { key: 'name', header: 'NAME' },
  ]);
}

function renderResult(
  result: SendMigrationResponse,
  out: { stdout: (s: string) => void; json: boolean }
): void {
  if (out.json) {
    out.stdout(formatJson(result) + '\n');
    return;
  }

  if ('dryRun' in result) {
    if (result.count === 0) {
      out.stdout('No recipients match the filter.\n');
      return;
    }
    out.stdout(`Dry run: ${result.count} recipient(s) from source "${result.source}"\n`);
    out.stdout(renderRecipientsTable(result.recipients) + '\n');
    return;
  }

  out.stdout(
    formatSuccess(`Sent ${result.sent}/${result.total} (failed: ${result.failed})`) + '\n'
  );
  const rows: Row[] = result.results.map((r) => ({
    email: r.email,
    sent: r.sent ? 'yes' : 'no',
    error: r.error ?? '',
  }));
  out.stdout(
    renderTable(rows, [
      { key: 'email', header: 'EMAIL' },
      { key: 'sent', header: 'SENT' },
      { key: 'error', header: 'ERROR' },
    ]) + '\n'
  );
}
