import { describe, expect, it, vi } from 'vitest';
import {
  BACKFILL_COUNT_SQL,
  BACKFILL_UPDATE_SQL,
  runActivityBackfill,
} from '../db/activity-backfill.js';
import type { QueryRunner } from '../db/activity-backfill.js';

// BACT-012 (ADR-121, OQ-B): one-shot proxy that seeds `last_activity_at` from
// `max(refresh_tokens.created_at)` for accounts that have never recorded any
// activity, so pre-BACT-008 token-era users are not all "never active" on
// day one. Must NEVER touch first_login_at / last_login_at / last_login_method
// (OQ-B) — those are set only by BACT-002's interactive-login mint paths.
//
// No live Postgres in this suite (same posture as migrate.test.ts /
// plan-activity-migration.test.ts): a fake QueryRunner interprets the two
// literal SQL statements the implementation issues against an in-memory
// fixture, so assertions cover both the exact SQL text (proves the UPDATE
// never names a login column) and the resulting row state.

interface BetaUserRow {
  id: string;
  last_activity_at: string | null;
  last_activity_kind: string | null;
  first_login_at: string | null;
  last_login_at: string | null;
  last_login_method: string | null;
}

interface RefreshTokenRow {
  user_id: string;
  created_at: string;
}

interface FixtureState {
  betaUsers: BetaUserRow[];
  refreshTokens: RefreshTokenRow[];
}

interface FakeRunner extends QueryRunner {
  calls: Array<{ text: string; params?: unknown[] }>;
  state: FixtureState;
}

function makeRunner(state: FixtureState): FakeRunner {
  const calls: Array<{ text: string; params?: unknown[] }> = [];

  const query = vi.fn(async (text: string, params?: unknown[]) => {
    calls.push({ text, params });
    const trimmed = text.trim();

    if (trimmed.startsWith('SELECT count(*)')) {
      const eligible = state.betaUsers.filter(
        (u) => u.last_activity_at === null && state.refreshTokens.some((rt) => rt.user_id === u.id)
      );
      return { rows: [{ affected: eligible.length }] };
    }

    if (trimmed.startsWith('UPDATE beta_users')) {
      const maxByUser = new Map<string, string>();
      for (const rt of state.refreshTokens) {
        const current = maxByUser.get(rt.user_id);
        if (!current || rt.created_at > current) {
          maxByUser.set(rt.user_id, rt.created_at);
        }
      }

      const updatedIds: string[] = [];
      for (const user of state.betaUsers) {
        const maxCreatedAt = maxByUser.get(user.id);
        if (user.last_activity_at === null && maxCreatedAt) {
          user.last_activity_at = maxCreatedAt;
          user.last_activity_kind = 'refresh';
          updatedIds.push(user.id);
        }
      }

      return { rows: updatedIds.map((id) => ({ id })) };
    }

    throw new Error(`unexpected query in fake runner: ${trimmed}`);
  });

  return { query, calls, state };
}

function makeFixture(): FixtureState {
  return {
    betaUsers: [
      // A: never active, has refresh tokens — eligible for backfill.
      {
        id: 'user-a',
        last_activity_at: null,
        last_activity_kind: null,
        first_login_at: null,
        last_login_at: null,
        last_login_method: null,
      },
      // B: already has a login-derived last_activity_at — must NOT be touched
      // (only-null guard).
      {
        id: 'user-b',
        last_activity_at: '2026-08-01T00:00:00.000Z',
        last_activity_kind: 'login',
        first_login_at: '2026-08-01T00:00:00.000Z',
        last_login_at: '2026-08-01T00:00:00.000Z',
        last_login_method: 'github',
      },
      // C: never active, no refresh tokens at all — must stay null.
      {
        id: 'user-c',
        last_activity_at: null,
        last_activity_kind: null,
        first_login_at: null,
        last_login_at: null,
        last_login_method: null,
      },
    ],
    refreshTokens: [
      { user_id: 'user-a', created_at: '2026-07-10T00:00:00.000Z' },
      { user_id: 'user-a', created_at: '2026-07-20T00:00:00.000Z' }, // latest for A
      { user_id: 'user-b', created_at: '2026-07-15T00:00:00.000Z' },
    ],
  };
}

describe('SQL text (OQ-B proof)', () => {
  it('the UPDATE statement never names a login column', () => {
    expect(BACKFILL_UPDATE_SQL).not.toMatch(/first_login_at/);
    expect(BACKFILL_UPDATE_SQL).not.toMatch(/last_login_at/);
    expect(BACKFILL_UPDATE_SQL).not.toMatch(/last_login_method/);
  });

  it('the UPDATE statement only writes last_activity_at and last_activity_kind, guarded by IS NULL', () => {
    expect(BACKFILL_UPDATE_SQL).toMatch(/UPDATE beta_users/);
    expect(BACKFILL_UPDATE_SQL).toMatch(/SET\s+last_activity_at\s*=/);
    expect(BACKFILL_UPDATE_SQL).toMatch(/last_activity_kind\s*=\s*'refresh'/);
    expect(BACKFILL_UPDATE_SQL).toMatch(/last_activity_at IS NULL/);
  });

  it('the COUNT statement is a read-only SELECT', () => {
    expect(BACKFILL_COUNT_SQL).toMatch(/^SELECT count\(\*\)/);
    expect(BACKFILL_COUNT_SQL).not.toMatch(/UPDATE|INSERT|DELETE/i);
  });
});

describe('runActivityBackfill — dry-run (default)', () => {
  it('reports the affected count without issuing any write', async () => {
    const runner = makeRunner(makeFixture());

    const result = await runActivityBackfill(runner);

    expect(result).toEqual({ dryRun: true, affected: 1 }); // only user-a is eligible
    expect(runner.calls).toHaveLength(1);
    expect(runner.calls[0].text.trim()).toMatch(/^SELECT count\(\*\)/);
  });

  it('leaves every row byte-identical, including login columns', async () => {
    const fixture = makeFixture();
    const before = JSON.parse(JSON.stringify(fixture.betaUsers));
    const runner = makeRunner(fixture);

    await runActivityBackfill(runner, { apply: false });

    expect(fixture.betaUsers).toEqual(before);
  });

  it('is explicit when apply is not passed at all (default is dry-run)', async () => {
    const runner = makeRunner(makeFixture());
    const result = await runActivityBackfill(runner, {});
    expect(result.dryRun).toBe(true);
  });
});

describe('runActivityBackfill — apply', () => {
  it('writes last_activity_at/last_activity_kind only for null-activity rows with refresh tokens', async () => {
    const fixture = makeFixture();
    const runner = makeRunner(fixture);

    const result = await runActivityBackfill(runner, { apply: true });

    expect(result).toEqual({ dryRun: false, affected: 1 });

    const a = fixture.betaUsers.find((u) => u.id === 'user-a')!;
    expect(a.last_activity_at).toBe('2026-07-20T00:00:00.000Z'); // max(created_at)
    expect(a.last_activity_kind).toBe('refresh');
  });

  it('never overwrites an account that already has last_activity_at (only-null guard)', async () => {
    const fixture = makeFixture();
    const runner = makeRunner(fixture);

    await runActivityBackfill(runner, { apply: true });

    const b = fixture.betaUsers.find((u) => u.id === 'user-b')!;
    expect(b.last_activity_at).toBe('2026-08-01T00:00:00.000Z');
    expect(b.last_activity_kind).toBe('login');
  });

  it('leaves accounts with no refresh tokens null', async () => {
    const fixture = makeFixture();
    const runner = makeRunner(fixture);

    await runActivityBackfill(runner, { apply: true });

    const c = fixture.betaUsers.find((u) => u.id === 'user-c')!;
    expect(c.last_activity_at).toBeNull();
    expect(c.last_activity_kind).toBeNull();
  });

  it('never touches first_login_at / last_login_at / last_login_method (OQ-B)', async () => {
    const fixture = makeFixture();
    const before = fixture.betaUsers.map((u) => ({
      id: u.id,
      first_login_at: u.first_login_at,
      last_login_at: u.last_login_at,
      last_login_method: u.last_login_method,
    }));
    const runner = makeRunner(fixture);

    await runActivityBackfill(runner, { apply: true });

    const after = fixture.betaUsers.map((u) => ({
      id: u.id,
      first_login_at: u.first_login_at,
      last_login_at: u.last_login_at,
      last_login_method: u.last_login_method,
    }));
    expect(after).toEqual(before);
  });

  it('issues exactly one UPDATE call, never a SELECT count', async () => {
    const runner = makeRunner(makeFixture());

    await runActivityBackfill(runner, { apply: true });

    expect(runner.calls).toHaveLength(1);
    expect(runner.calls[0].text.trim()).toMatch(/^UPDATE beta_users/);
  });

  it('is idempotent — a second apply run affects 0 rows', async () => {
    const fixture = makeFixture();
    const runner = makeRunner(fixture);

    const first = await runActivityBackfill(runner, { apply: true });
    expect(first.affected).toBe(1);

    const second = await runActivityBackfill(runner, { apply: true });
    expect(second).toEqual({ dryRun: false, affected: 0 });

    // State is stable across the second no-op run.
    const a = fixture.betaUsers.find((u) => u.id === 'user-a')!;
    expect(a.last_activity_at).toBe('2026-07-20T00:00:00.000Z');
  });
});

describe('runActivityBackfill — logging', () => {
  it('calls the log callback with a human-readable summary', async () => {
    const runner = makeRunner(makeFixture());
    const log = vi.fn();

    await runActivityBackfill(runner, { apply: false, log });

    expect(log).toHaveBeenCalledTimes(1);
    expect(log.mock.calls[0][0]).toMatch(/dry-run/i);
    expect(log.mock.calls[0][0]).toMatch(/1/);
  });
});
