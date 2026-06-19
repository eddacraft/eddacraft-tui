import { RevokeResponseSchema, type RevokeResponse } from '@eddacraft/admin-contracts';
import { AdminClient, AdminError, type AdminWriter } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, formatSuccess } from '../format.js';
import { defaultPrompt, isInteractiveTTY } from '../prompt.js';

export type { RevokeResponse };

export interface RevokeOptions extends ConfigFlags {
  token?: string;
  yes?: boolean;
  json?: boolean;
}

export interface RevokeDeps {
  createClient?: (cfg: AdminConfig) => AdminWriter;
  stdout?: (chunk: string) => void;
  stderr?: (chunk: string) => void;
  prompt?: (message: string) => Promise<string>;
  isTTY?: boolean;
}

export const CONFIRM_WORD = 'revoke';

export async function runRevokeCommand(
  email: string | undefined,
  options: RevokeOptions = {},
  deps: RevokeDeps = {}
): Promise<void> {
  const hasEmail = typeof email === 'string' && email.length > 0;
  const hasToken = typeof options.token === 'string' && options.token.length > 0;

  if (hasEmail && hasToken) {
    throw new AdminError('cannot combine <email> with --token', 64);
  }
  if (!hasEmail && !hasToken) {
    throw new AdminError('revoke requires <email> or --token <raw>', 64);
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

  const target = hasEmail ? `all tokens for ${email}` : 'the supplied token';

  if (!options.yes) {
    if (!isTTY) {
      throw new AdminError('refusing to revoke without --yes in a non-interactive session', 4);
    }
    const prompt = deps.prompt ?? defaultPrompt;
    stderr(
      `About to revoke ${target}.\nThis cannot be undone. Type "${CONFIRM_WORD}" to confirm.\n`
    );
    const answer = (await prompt(`> `)).trim();
    if (answer !== CONFIRM_WORD) {
      // #948: route abort notice to stderr when --json so stdout stays pure JSON.
      (options.json ? stderr : stdout)('Aborted.\n');
      return;
    }
  }

  const body = hasEmail ? { email } : { token: options.token };
  const result = await client.post('/admin/revoke', body, RevokeResponseSchema);

  if (options.json) {
    stdout(formatJson(result) + '\n');
    return;
  }

  const subject = hasEmail ? email : 'token';
  stdout(formatSuccess(`Revoked ${result.revoked} token(s) for ${subject}`) + '\n');
  // SEC-007 / GH #1672: surface the refresh-session and account-suspension
  // counters when the server provides them, so operators see that revocation
  // closed every credential surface (not just access tokens).
  if (typeof result.refreshSessionsRevoked === 'number') {
    stdout(`  refresh sessions revoked: ${result.refreshSessionsRevoked}\n`);
  }
  if (result.accountSuspended === true) {
    stdout('  account suspended (re-approve to restore access)\n');
  }
}
