import {
  ApproveResponseSchema,
  type ApproveResponse,
  type ApprovedEntry,
  type SkippedEntry,
} from '@eddacraft/admin-contracts';
import { AdminClient, AdminError, type AdminWriter } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, formatSuccess, renderTable, type Row } from '../format.js';
import { defaultPrompt, isInteractiveTTY } from '../prompt.js';

export type { ApproveResponse, ApprovedEntry, SkippedEntry };

export interface ApproveOptions extends ConfigFlags {
  batch?: number;
  yes?: boolean;
  json?: boolean;
}

export interface ApproveDeps {
  createClient?: (cfg: AdminConfig) => AdminWriter;
  stdout?: (chunk: string) => void;
  stderr?: (chunk: string) => void;
  prompt?: (message: string) => Promise<string>;
  isTTY?: boolean;
}

export async function runApproveCommand(
  email: string | undefined,
  options: ApproveOptions = {},
  deps: ApproveDeps = {}
): Promise<void> {
  const hasEmail = typeof email === 'string' && email.length > 0;
  const batch = options.batch;

  if (hasEmail && batch !== undefined) {
    throw new AdminError('cannot combine <email> with --batch', 64);
  }
  if (!hasEmail && batch === undefined) {
    throw new AdminError('approve requires <email> or --batch N', 64);
  }
  if (batch !== undefined && (!Number.isInteger(batch) || batch < 1 || batch > 100)) {
    throw new AdminError('--batch must be an integer between 1 and 100', 64);
  }

  const config = resolveConfig({
    key: options.key,
    url: options.url,
    actor: options.actor,
  });
  const client: AdminWriter = deps.createClient?.(config) ?? new AdminClient(config);
  const stdout = deps.stdout ?? ((chunk) => process.stdout.write(chunk));
  const stderr = deps.stderr ?? ((chunk) => process.stderr.write(chunk));
  const isTTY = deps.isTTY ?? isInteractiveTTY();

  const summary = hasEmail
    ? `Approve ${email}?`
    : `Approve the oldest ${batch} unapproved waitlist entries?`;

  if (!options.yes) {
    if (!isTTY) {
      throw new AdminError('refusing to approve without --yes in a non-interactive session', 4);
    }
    const prompt = deps.prompt ?? defaultPrompt;
    const answer = (await prompt(`${summary} [y/N] `)).trim().toLowerCase();
    if (answer !== 'y' && answer !== 'yes') {
      // #948: route abort notice to stderr when --json so stdout stays pure JSON.
      (options.json ? stderr : stdout)('Aborted.\n');
      return;
    }
  }

  const body = hasEmail ? { email } : { batch };
  const result = await client.post('/admin/approve', body, ApproveResponseSchema);

  if (options.json) {
    stdout(formatJson(result) + '\n');
    return;
  }

  if (result.approved.length === 0) {
    stdout('No entries approved.\n');
  } else {
    stdout(formatSuccess(`Approved ${result.approved.length}`) + '\n');
    const rows: Row[] = result.approved.map((e) => ({ ...e }));
    const table = renderTable(rows, [
      { key: 'email', header: 'EMAIL' },
      {
        key: 'expiresAt',
        header: 'INVITE EXPIRES',
        format: (v) => String(v).slice(0, 19).replace('T', ' '),
      },
    ]);
    stdout(table + '\n');
  }

  if (result.skipped && result.skipped.length > 0) {
    const rows: Row[] = result.skipped.map((e) => ({ ...e }));
    const table = renderTable(rows, [
      { key: 'email', header: 'EMAIL' },
      { key: 'reason', header: 'REASON' },
      { key: 'message', header: 'MESSAGE', format: (v) => (v == null ? '' : String(v)) },
    ]);
    stderr(`Skipped ${result.skipped.length}:\n${table}\n`);
  }
}
