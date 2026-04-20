import type { ZodType } from 'zod';
import {
  ShowResponseSchema,
  type ShowAuditEntry,
  type ShowResponse,
  type ShowToken,
  type ShowUser,
} from '@eddacraft/admin-contracts';
import { AdminClient } from '../client.js';
import { resolveConfig, type AdminConfig, type ConfigFlags } from '../config.js';
import { formatJson, renderTable, type Row } from '../format.js';

export type { ShowAuditEntry, ShowResponse, ShowToken, ShowUser };

export interface ShowOptions extends ConfigFlags {
  json?: boolean;
}

export interface AdminReader {
  get<T>(
    path: string,
    query?: Record<string, string | number | boolean | undefined>,
    schema?: ZodType<T>
  ): Promise<T>;
}

export interface ShowDeps {
  createClient?: (cfg: AdminConfig) => AdminReader;
  stdout?: (chunk: string) => void;
  stderr?: (chunk: string) => void;
}

export async function runShowCommand(
  email: string,
  options: ShowOptions = {},
  deps: ShowDeps = {}
): Promise<void> {
  const config = resolveConfig({
    key: options.key,
    url: options.url,
    actor: options.actor,
  });
  const client: AdminReader = deps.createClient?.(config) ?? new AdminClient(config);
  const stdout = deps.stdout ?? ((chunk) => process.stdout.write(chunk));
  const stderr = deps.stderr ?? ((chunk) => process.stderr.write(chunk));

  const path = `/admin/user/${encodeURIComponent(email)}`;
  const result = await client.get(path, undefined, ShowResponseSchema);

  if (result.auditError) {
    stderr('warning: audit lookup failed; user and tokens still shown.\n');
  }

  if (options.json) {
    stdout(formatJson(result) + '\n');
    return;
  }

  stdout(renderUserPanel(result.user) + '\n\n');
  stdout(renderTokensSection(result.tokens) + '\n\n');
  stdout(renderAuditSection(result.recentAudit) + '\n');
}

function renderUserPanel(user: ShowUser): string {
  const lines = [
    'USER',
    '----',
    `email:      ${user.email}`,
    `name:       ${user.name ?? '—'}`,
    `status:     ${user.status}`,
    `id:         ${user.id}`,
    `created:    ${user.created_at.slice(0, 10)}`,
    `updated:    ${user.updated_at.slice(0, 10)}`,
  ];
  if (user.notes) lines.push(`notes:      ${user.notes}`);
  return lines.join('\n');
}

function renderTokensSection(tokens: ShowToken[]): string {
  if (tokens.length === 0) return 'TOKENS\n------\n(none)';
  const rows: Row[] = tokens.map((t) => ({ ...t, scopes: t.scopes.join(',') }));
  const table = renderTable(rows, [
    { key: 'id', header: 'ID' },
    { key: 'scopes', header: 'SCOPES' },
    { key: 'created_at', header: 'CREATED', format: (v) => String(v).slice(0, 10) },
    { key: 'expires_at', header: 'EXPIRES', format: (v) => String(v).slice(0, 10) },
    {
      key: 'revoked_at',
      header: 'REVOKED',
      format: (v) => (v == null ? '—' : String(v).slice(0, 10)),
    },
  ]);
  return `TOKENS\n------\n${table}`;
}

function renderAuditSection(entries: ShowAuditEntry[]): string {
  if (entries.length === 0) return 'RECENT AUDIT\n------------\n(none)';
  const rows: Row[] = entries.map((e) => ({ ...e }));
  const table = renderTable(rows, [
    { key: 'created_at', header: 'WHEN', format: (v) => String(v).slice(0, 19).replace('T', ' ') },
    { key: 'action', header: 'ACTION' },
    { key: 'actor', header: 'ACTOR' },
    {
      key: 'metadata',
      header: 'META',
      format: (v) => {
        if (v == null || typeof v !== 'object') return '';
        const obj = v as Record<string, unknown>;
        if (Object.keys(obj).length === 0) return '';
        return JSON.stringify(obj);
      },
    },
  ]);
  return `RECENT AUDIT\n------------\n${table}`;
}
