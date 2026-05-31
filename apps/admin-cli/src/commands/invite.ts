import { InviteResponseSchema, type InviteResponse } from '@eddacraft/admin-contracts';
import { AdminClient, AdminError, type AdminWriter } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, formatSuccess } from '../format.js';

export type { InviteResponse };

// Note: This inline constant should eventually be retired once admin-cli consumes the
// api.scope.* manifest through @eddacraft/admin-contracts. Until then, this
// list MUST mirror API_SCOPE_NAMES in apps/anvil-api/src/lib/feature-flags.ts.
export const ALLOWED_SCOPES = ['beta', 'preview', 'internal'] as const;
export type InviteScope = (typeof ALLOWED_SCOPES)[number];

export interface InviteOptions extends ConfigFlags {
  name?: string;
  notes?: string;
  days?: number;
  scope?: InviteScope[];
  tokenOnly?: boolean;
  json?: boolean;
}

export interface InviteRequestBody {
  email: string;
  name?: string;
  notes?: string;
  days?: number;
  scopes?: InviteScope[];
  tokenOnly?: boolean;
}

export interface InviteDeps {
  createClient?: (cfg: AdminConfig) => AdminWriter;
  stdout?: (chunk: string) => void;
  stderr?: (chunk: string) => void;
}

export async function runInviteCommand(
  email: string,
  options: InviteOptions = {},
  deps: InviteDeps = {}
): Promise<void> {
  if (options.days !== undefined) {
    if (!Number.isInteger(options.days) || options.days < 1 || options.days > 365) {
      throw new AdminError('--days must be an integer between 1 and 365', 64);
    }
  }

  if (options.tokenOnly && options.json) {
    throw new AdminError(
      '--token-only and --json are mutually exclusive: --json would emit the raw token in a JSON blob, defeating the purpose of the one-time banner',
      64
    );
  }

  const config = resolveConfig({
    key: options.key,
    url: options.url,
    actor: options.actor,
  });
  const client: AdminWriter = deps.createClient?.(config) ?? new AdminClient(config);
  const stdout = deps.stdout ?? ((chunk) => process.stdout.write(chunk));
  const stderr = deps.stderr ?? ((chunk) => process.stderr.write(chunk));

  const body: InviteRequestBody = { email };
  if (options.name !== undefined) body.name = options.name;
  if (options.notes !== undefined) body.notes = options.notes;
  if (options.days !== undefined) body.days = options.days;
  if (options.scope !== undefined && options.scope.length > 0) body.scopes = options.scope;
  if (options.tokenOnly) body.tokenOnly = true;

  const result = await client.post('/admin/invite', body, InviteResponseSchema);

  if (options.json) {
    stdout(formatJson(result) + '\n');
    return;
  }

  if (options.tokenOnly) {
    if (!result.token) {
      throw new AdminError('server returned tokenOnly response without a token', 2);
    }
    stderr(
      '================================================================\n' +
        '  ONE-TIME ACCESS TOKEN — store it now; it will not be shown again\n' +
        `  user:    ${result.user.email}\n` +
        `  scopes:  ${result.scopes.join(',')}\n` +
        (result.expiresAt ? `  expires: ${result.expiresAt}\n` : '') +
        '================================================================\n'
    );
    stdout(result.token + '\n');
    return;
  }

  stdout(formatSuccess(`Invited ${result.user.email}`) + '\n');
  stdout(`scopes: ${result.scopes.join(',')}\n`);
}
