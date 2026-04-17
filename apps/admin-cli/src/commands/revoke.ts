import { AdminClient, AdminError } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, formatSuccess } from '../format.js';
import { defaultPrompt, isInteractiveTTY } from '../prompt.js';

export interface RevokeOptions extends ConfigFlags {
  token?: string;
  yes?: boolean;
  json?: boolean;
}

export interface RevokeResponse {
  revoked: number;
}

export interface AdminWriter {
  post<T>(path: string, body?: unknown): Promise<T>;
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
      `About to revoke ${target}.\n` + `This cannot be undone. Type "${CONFIRM_WORD}" to confirm.\n`
    );
    const answer = (await prompt(`> `)).trim();
    if (answer !== CONFIRM_WORD) {
      stdout('Aborted.\n');
      return;
    }
  }

  const body = hasEmail ? { email } : { token: options.token };
  const result = await client.post<RevokeResponse>('/admin/revoke', body);

  if (options.json) {
    stdout(formatJson(result) + '\n');
    return;
  }

  const subject = hasEmail ? email : 'token';
  stdout(formatSuccess(`Revoked ${result.revoked} token(s) for ${subject}`) + '\n');
}
