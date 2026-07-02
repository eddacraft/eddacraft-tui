import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  createAdminKey,
  createOutputLine,
  revokeAdminKey,
  sslConfigFor,
} from '../../scripts/admin-key-manage.mjs';

const repoRoot = fileURLToPath(new URL('../../../', import.meta.url));
const scriptPath = join(repoRoot, 'infra/scripts/admin-key-manage.mjs');

const SSL_ON = { rejectUnauthorized: true };

describe('sslConfigFor', () => {
  it('keeps SSL on for remote hosts', () => {
    expect(sslConfigFor('postgres://user:pass@db.example.com:5432/app')).toEqual(SSL_ON);
  });

  it('disables SSL for loopback hostnames without an explicit sslmode', () => {
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app')).toBeUndefined();
    expect(sslConfigFor('postgres://user:pass@127.0.0.1:5432/app')).toBeUndefined();
    expect(sslConfigFor('postgres://user@[::1]:5432/app')).toBeUndefined();
  });

  it('matches the hostname, not a substring of the connection string', () => {
    // These all contain "localhost" somewhere other than the hostname and
    // previously disabled SSL via the naive `includes('localhost')` check.
    expect(sslConfigFor('postgres://user:pass@localhost.example.com:5432/app')).toEqual(SSL_ON);
    expect(sslConfigFor('postgres://user:pass@db.example.com:5432/localhost_shadow')).toEqual(
      SSL_ON
    );
    expect(
      sslConfigFor('postgres://user:pass@db.example.com:5432/app?application_name=localhost-test')
    ).toEqual(SSL_ON);
  });

  it('honours an explicit sslmode=disable on any host', () => {
    expect(
      sslConfigFor('postgres://user:pass@db.example.com:5432/app?sslmode=disable')
    ).toBeUndefined();
  });

  it('honours an explicit sslmode requesting SSL, even on loopback', () => {
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=require')).toEqual(SSL_ON);
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=verify-full')).toEqual(
      SSL_ON
    );
  });

  it('compares sslmode case-insensitively', () => {
    expect(
      sslConfigFor('postgres://user:pass@db.example.com:5432/app?sslmode=DISABLE')
    ).toBeUndefined();
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=Require')).toEqual(SSL_ON);
  });

  it('keeps SSL on for unknown sslmode values', () => {
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=bogus')).toEqual(SSL_ON);
  });

  it('keeps SSL on when the connection string is not URL-parseable', () => {
    expect(sslConfigFor('host=localhost dbname=app')).toEqual(SSL_ON);
  });
});

describe('createOutputLine', () => {
  it('emits the documented snake_case hashed_key field', () => {
    const parsed = JSON.parse(createOutputLine(42, 'abc123')) as Record<string, unknown>;
    expect(parsed).toEqual({ id: 42, hashed_key: 'abc123' });
  });

  it('terminates the line with a newline', () => {
    expect(createOutputLine(1, 'x')).toMatch(/\n$/);
  });
});

// ── CIB-119: active-key invariant under concurrent creates ────────────────
//
// The fake below emulates just enough Postgres for the create/revoke paths:
// an `admin_keys` table, an audit table, and — crucially — transaction-scoped
// advisory locks (`pg_advisory_xact_lock`), released on COMMIT/ROLLBACK.
// SELECTs yield to the event loop before returning, so without the advisory
// lock two concurrent creates would both observe "no active row" and both
// insert, breaking the invariant. No live database or credentials involved.

interface FakeRow {
  id: number;
  hashed_key: string;
  actor_email: string;
  note: string;
  revoked_at: string | null;
  created_at: number;
}

class Mutex {
  private tail: Promise<void> = Promise.resolve();

  acquire(): Promise<() => void> {
    const previous = this.tail;
    let release!: () => void;
    this.tail = new Promise<void>((resolve) => (release = resolve));
    return previous.then(() => release);
  }
}

class FakeDb {
  rows: FakeRow[] = [];
  audit: Array<{ admin_key_id: number; action: string }> = [];
  nextId = 1;
  private mutexes = new Map<string, Mutex>();

  mutexFor(key: string): Mutex {
    let mutex = this.mutexes.get(key);
    if (!mutex) {
      mutex = new Mutex();
      this.mutexes.set(key, mutex);
    }
    return mutex;
  }

  activeRowsFor(actorEmail: string): FakeRow[] {
    return this.rows.filter((r) => r.actor_email === actorEmail && r.revoked_at === null);
  }
}

interface FakeClient {
  query(sql: string, params?: unknown[]): Promise<{ rows: Array<Record<string, unknown>> }>;
  queries: string[];
}

function fakeClient(db: FakeDb): FakeClient {
  const heldLocks: Array<() => void> = [];
  const queries: string[] = [];

  return {
    queries,
    async query(sql: string, params: unknown[] = []) {
      queries.push(sql);

      if (sql === 'BEGIN') {
        return { rows: [] };
      }
      if (sql === 'COMMIT' || sql === 'ROLLBACK') {
        while (heldLocks.length > 0) {
          heldLocks.pop()!();
        }
        return { rows: [] };
      }
      if (sql.includes('pg_advisory_xact_lock')) {
        const release = await db.mutexFor(String(params[0])).acquire();
        heldLocks.push(release);
        return { rows: [] };
      }
      if (sql.includes('SELECT id, hashed_key, revoked_at FROM admin_keys')) {
        // Yield so an unserialised concurrent create can interleave here.
        await new Promise((resolve) => setImmediate(resolve));
        const [actorEmail] = params as [string];
        const active = db
          .activeRowsFor(actorEmail)
          .sort((a, b) => b.created_at - a.created_at)
          .slice(0, 1);
        return { rows: active as unknown as Array<Record<string, unknown>> };
      }
      if (sql.includes('INSERT INTO admin_keys_audit')) {
        const [adminKeyId, ...rest] = params as [number, ...unknown[]];
        void rest;
        db.audit.push({
          admin_key_id: adminKeyId,
          action: sql.includes("'created'") ? 'created' : 'revoked',
        });
        return { rows: [] };
      }
      if (sql.includes('INSERT INTO admin_keys')) {
        const [hashedKey, actorEmail, note] = params as [string, string, string];
        const row: FakeRow = {
          id: db.nextId++,
          hashed_key: hashedKey,
          actor_email: actorEmail,
          note,
          revoked_at: null,
          created_at: Date.now(),
        };
        db.rows.push(row);
        return { rows: [{ id: row.id }] };
      }
      if (sql.includes('UPDATE admin_keys SET revoked_at')) {
        const [actorEmail, hashedKey] = params as [string, string];
        const row = db.rows.find(
          (r) => r.actor_email === actorEmail && r.hashed_key === hashedKey && r.revoked_at === null
        );
        if (!row) {
          return { rows: [] };
        }
        row.revoked_at = new Date().toISOString();
        return { rows: [{ id: row.id }] };
      }
      throw new Error(`fake client: unhandled query: ${sql}`);
    },
  };
}

const baseArgs = {
  actorEmail: 'op@example.com',
  note: 'test key',
  changeActor: 'tester',
  commitSha: 'deadbeef',
};

describe('createAdminKey — active-key invariant (CIB-119)', () => {
  it('takes a per-actor advisory lock before checking for an existing active key', async () => {
    const db = new FakeDb();
    const client = fakeClient(db);

    await createAdminKey(client, { ...baseArgs, hashedKey: 'hash-a' });

    const lockIndex = client.queries.findIndex((q) => q.includes('pg_advisory_xact_lock'));
    const selectIndex = client.queries.findIndex((q) =>
      q.includes('SELECT id, hashed_key, revoked_at FROM admin_keys')
    );
    expect(lockIndex).toBeGreaterThan(-1);
    expect(selectIndex).toBeGreaterThan(-1);
    expect(lockIndex).toBeLessThan(selectIndex);
  });

  it('preserves the invariant under concurrent creates with different bearers', async () => {
    const db = new FakeDb();

    const results = await Promise.allSettled([
      createAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-a' }),
      createAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-b' }),
    ]);

    const fulfilled = results.filter((r) => r.status === 'fulfilled');
    const rejected = results.filter((r) => r.status === 'rejected');

    expect(fulfilled).toHaveLength(1);
    expect(rejected).toHaveLength(1);
    expect((rejected[0] as PromiseRejectedResult).reason.message).toMatch(
      /active admin_keys row already exists/
    );

    // The invariant: at most one active key per actor, ever.
    expect(db.activeRowsFor(baseArgs.actorEmail)).toHaveLength(1);
    expect(db.audit.filter((a) => a.action === 'created')).toHaveLength(1);
  });

  it('is idempotent under concurrent creates with the same bearer', async () => {
    const db = new FakeDb();

    const [a, b] = await Promise.all([
      createAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-same' }),
      createAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-same' }),
    ]);

    expect(a.id).toBe(b.id);
    expect(db.activeRowsFor(baseArgs.actorEmail)).toHaveLength(1);
    expect(db.audit.filter((x) => x.action === 'created')).toHaveLength(1);
  });

  it('refuses sequentially when a different-hash active row already exists', async () => {
    const db = new FakeDb();

    await createAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-a' });
    await expect(
      createAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-b' })
    ).rejects.toThrow(/revoke it first or re-use the same bearer/);

    expect(db.activeRowsFor(baseArgs.actorEmail)).toHaveLength(1);
  });
});

describe('revokeAdminKey (CIB-119)', () => {
  it('revokes the matching active row and writes an audit entry', async () => {
    const db = new FakeDb();
    await createAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-a' });

    const result = await revokeAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-a' });

    expect(result.revoked).toBe(1);
    expect(db.activeRowsFor(baseArgs.actorEmail)).toHaveLength(0);
    expect(db.audit.filter((a) => a.action === 'revoked')).toHaveLength(1);
  });

  it('treats a missing row as a no-op, not an error', async () => {
    const db = new FakeDb();

    const result = await revokeAdminKey(fakeClient(db), { ...baseArgs, hashedKey: 'hash-x' });

    expect(result.revoked).toBe(0);
    expect(db.audit).toHaveLength(0);
  });

  it('serialises against creates via the same per-actor advisory lock', async () => {
    const db = new FakeDb();
    const client = fakeClient(db);

    await revokeAdminKey(client, { ...baseArgs, hashedKey: 'hash-x' });

    expect(client.queries.some((q) => q.includes('pg_advisory_xact_lock'))).toBe(true);
  });
});

describe('direct invocation', () => {
  it('still enforces the usage contract when run as a script', () => {
    const result = spawnSync('node', [scriptPath, 'frobnicate'], { encoding: 'utf8' });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain('usage: admin-key-manage.mjs <create|revoke>');
  });

  it('fails fast on missing env before touching any database', () => {
    const env = { ...process.env };
    delete env['DATABASE_URL'];
    const result = spawnSync('node', [scriptPath, 'create'], { encoding: 'utf8', env });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain('missing required env DATABASE_URL');
  });
});
