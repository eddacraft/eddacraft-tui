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
  previewToken: string;
  expiresAt: string;
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

  // Dry-run mode: fetch a preview + snapshot token and return. The token
  // lets the operator follow up with a real-send pinned to this exact
  // recipient set, within the 10-minute TTL.
  if (dryRun) {
    const preview = await client.post<DryRunResponse>('/admin/send-migration', {
      source,
      dryRun: true,
      limit,
    });
    renderResult(preview, { stdout, json: !!options.json });
    return;
  }

  // Refuse non-TTY real-sends without --yes before creating a server-side
  // snapshot — otherwise the early abort would leave orphaned snapshot
  // rows behind and burn a network round-trip for no gain.
  if (!options.yes && !isTTY) {
    throw new AdminError(
      'refusing to send migration without --yes in a non-interactive session',
      4
    );
  }

  // Real-send always starts with a fresh dry-run so the snapshot the
  // server will consume matches what we're about to show the operator.
  const preview = await client.post<DryRunResponse>('/admin/send-migration', {
    source,
    dryRun: true,
    limit,
  });

  if (preview.count === 0) {
    stdout('No recipients match the filter. Nothing to send.\n');
    return;
  }

  if (!options.yes) {
    stderr(
      `About to send migration email to ${preview.count} recipient(s) (source: ${source}).\n` +
        renderRecipientsTable(preview.recipients) +
        '\n'
    );
    const prompt = deps.prompt ?? defaultPrompt;
    const answer = (await prompt(`Continue? [y/N] `)).trim().toLowerCase();
    if (answer !== 'y' && answer !== 'yes') {
      stdout('Aborted.\n');
      return;
    }
  }

  let result: SendResponse;
  try {
    result = await client.post<SendResponse>('/admin/send-migration', {
      source,
      dryRun: false,
      limit,
      previewToken: preview.previewToken,
    });
  } catch (err) {
    throw rewriteSnapshotError(err);
  }
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
    out.stdout(
      `Preview token: ${result.previewToken} (expires ${result.expiresAt})\n` +
        'Re-run without --dry-run within 10 minutes to send this exact set.\n'
    );
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

/**
 * Translate server-side snapshot error codes into tailored recovery
 * messages. Leaves non-AdminError / non-coded failures untouched.
 */
function rewriteSnapshotError(err: unknown): unknown {
  if (!(err instanceof AdminError) || !err.body) return err;
  let parsed: { code?: unknown; added?: unknown; removed?: unknown };
  try {
    parsed = JSON.parse(err.body) as typeof parsed;
  } catch {
    return err;
  }
  if (typeof parsed.code !== 'string') return err;

  switch (parsed.code) {
    case 'cohort_drift': {
      const added = Array.isArray(parsed.added) ? (parsed.added as string[]) : [];
      const removed = Array.isArray(parsed.removed) ? (parsed.removed as string[]) : [];
      const lines = ['recipient set changed since preview; re-run with --dry-run and retry'];
      if (added.length) lines.push(`  added:   ${added.join(', ')}`);
      if (removed.length) lines.push(`  removed: ${removed.join(', ')}`);
      return new AdminError(lines.join('\n'), 1, err.status, err.body);
    }
    case 'preview_token_expired':
      return new AdminError(
        'preview token expired (10-minute TTL). Re-run with --dry-run and retry within 10 minutes.',
        1,
        err.status,
        err.body
      );
    case 'preview_token_consumed':
      return new AdminError(
        'preview token already used. A prior send may have completed; re-run with --dry-run to verify recipients before retrying.',
        1,
        err.status,
        err.body
      );
    case 'preview_token_missing':
      // The server merges the wrong-actor case into `preview_token_missing`
      // to avoid confirming token existence to non-owners. Surface both
      // recovery paths so the operator can fix actor mismatches too.
      return new AdminError(
        'preview token not found. If another operator created the preview, set --actor (or ANVIL_ADMIN_ACTOR) to the matching identity. Otherwise re-run with --dry-run to generate a fresh snapshot.',
        1,
        err.status,
        err.body
      );
    case 'preview_token_required':
      return new AdminError(
        'server rejected send without a preview token. This usually means the CLI skipped the dry-run step — re-run with --dry-run first.',
        1,
        err.status,
        err.body
      );
    default:
      return err;
  }
}
