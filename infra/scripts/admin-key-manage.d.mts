// Declarations for the plain-.mjs operator script so TypeScript consumers
// (the vitest suite) type-check without @ts-expect-error suppression.
// Keep in sync with admin-key-manage.mjs exports.

export function sslConfigFor(databaseUrl: string): { rejectUnauthorized: boolean } | undefined;

export function createOutputLine(id: string | number, hashedKey: string): string;

export interface AdminKeyClient {
  query(text: string, params?: unknown[]): Promise<{ rows: Array<Record<string, unknown>> }>;
}

export function createAdminKey(
  client: AdminKeyClient,
  args: {
    hashedKey: string;
    actorEmail: string;
    note?: string;
    changeActor: string;
    commitSha: string;
  }
): Promise<{ id: string | number; hashedKey: string }>;

export function revokeAdminKey(
  client: AdminKeyClient,
  args: {
    hashedKey: string;
    actorEmail: string;
    changeActor: string;
    commitSha: string;
  }
): Promise<{ revoked: number }>;
