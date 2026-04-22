import {
  DryRunResponseSchema,
  MIGRATION_SOURCES,
  SendResponseSchema,
  type DryRunResponse,
  type MigrationRecipient,
  type MigrationSource,
  type SendMigrationResponse,
  type SendResponse,
  type SendResultEntry,
} from '@eddacraft/admin-contracts';
import { AdminClient, AdminError, type AdminWriter } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, formatSuccess, renderTable, type Row } from '../format.js';
import { defaultPrompt, isInteractiveTTY } from '../prompt.js';

export type {
  DryRunResponse,
  MigrationRecipient,
  MigrationSource,
  SendMigrationResponse,
  SendResponse,
  SendResultEntry,
};
export { MIGRATION_SOURCES };

export interface SendMigrationOptions extends ConfigFlags {
  source?: MigrationSource;
  limit?: number;
  dryRun?: boolean;
  yes?: boolean;
  json?: boolean;
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
    const preview = await client.post(
      '/admin/send-migration',
      { source, dryRun: true, limit },
      DryRunResponseSchema
    );
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
  const preview = await client.post(
    '/admin/send-migration',
    { source, dryRun: true, limit },
    DryRunResponseSchema
  );

  if (preview.count === 0) {
    // #948: when --json is set, stdout is reserved for JSON only — route the
    // zero-recipient notice to stderr so stdout stays empty and parseable.
    const writeStatus = options.json ? stderr : stdout;
    writeStatus('No recipients match the filter. Nothing to send.\n');
    return;
  }

  if (!options.yes) {
    if (!isTTY) {
      throw new AdminError(
        'refusing to send migration without --yes in a non-interactive session',
        4
      );
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
      // #948: route abort notice to stderr when --json so stdout stays empty
      // (stdout is reserved for JSON output under --json; an abort produces
      // no JSON body).
      (options.json ? stderr : stdout)('Aborted.\n');
      return;
    }
  }

  let result: SendResponse;
  try {
    result = await client.post(
      '/admin/send-migration',
      { source, dryRun: false, limit, previewToken: preview.previewToken },
      SendResponseSchema
    );
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
