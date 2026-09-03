import { randomUUID } from 'node:crypto';
import { Client, neon } from '@neondatabase/serverless';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { registerActiveOtpAttempts } from '../db/queries.js';
import { waitForOtpClaimContention } from './support/neon-otp-contention.js';
import { requireSafeNeonTestDatabase } from './support/neon-test-database.js';

const MAX_ATTEMPTS = 3;
const CONCURRENT_CLAIMS = 8;
const { url } = requireSafeNeonTestDatabase();
const sql = neon(url);
const lockHolder = new Client(url);
const lockObserver = new Client(url);

const runId = randomUUID();
const userId = randomUUID();
const otpId = randomUUID();

describe('registerActiveOtpAttempts against Neon Postgres', () => {
  beforeAll(async () => {
    await Promise.all([lockHolder.connect(), lockObserver.connect()]);

    // An ephemeral branch normally inherits these production tables. The
    // minimum authoritative columns keep the test runnable against an empty
    // dedicated test branch without reproducing any attempt-cap logic.
    await sql.query(`
      CREATE TABLE IF NOT EXISTS beta_users (
        id uuid PRIMARY KEY,
        email text UNIQUE NOT NULL,
        status text NOT NULL DEFAULT 'active'
      )
    `);
    await sql.query(`
      CREATE TABLE IF NOT EXISTS otp_codes (
        id uuid PRIMARY KEY,
        user_id uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
        code_hash text NOT NULL,
        attempts int NOT NULL DEFAULT 0,
        expires_at timestamptz NOT NULL,
        consumed_at timestamptz,
        created_at timestamptz NOT NULL DEFAULT now()
      )
    `);
    await sql`
      INSERT INTO beta_users (id, email, status)
      VALUES (${userId}, ${`clawopen-011-${runId}@example.test`}, 'active')
    `;
    await sql`
      INSERT INTO otp_codes (id, user_id, code_hash, expires_at)
      VALUES (${otpId}, ${userId}, 'synthetic-non-matching-hash', now() + interval '10 minutes')
    `;
  });

  afterAll(async () => {
    await sql`DELETE FROM beta_users WHERE id = ${userId}`;
    await Promise.all([lockHolder.end(), lockObserver.end()]);
  });

  it('allows exactly three claims after more than three callers visibly contend on the row lock', async () => {
    let barrierHeld = false;
    let pendingClaims: Array<ReturnType<typeof registerActiveOtpAttempts>> = [];
    let claims: Awaited<ReturnType<typeof registerActiveOtpAttempts>>[] = [];

    try {
      await lockHolder.query('BEGIN');
      barrierHeld = true;
      await lockHolder.query('SELECT id FROM otp_codes WHERE id = $1 FOR UPDATE', [otpId]);
      const marker = await lockObserver.query<{ claims_started_after: string }>(
        'SELECT clock_timestamp()::text AS claims_started_after'
      );
      const claimsStartedAfter = marker.rows[0]?.claims_started_after;
      expect(claimsStartedAfter).toBeTypeOf('string');

      // The production HTTP-tagged UPDATE calls start while the interactive
      // transaction holds their target row. pg_stat_activity must show at
      // least four lock-waiting UPDATE statements before the barrier releases.
      pendingClaims = Array.from({ length: CONCURRENT_CLAIMS }, () =>
        registerActiveOtpAttempts(sql, userId, MAX_ATTEMPTS)
      );
      const contention = await waitForOtpClaimContention(lockObserver, claimsStartedAfter!, {
        minimumWaitingClaimants: MAX_ATTEMPTS + 1,
        maximumObservations: 500,
      });
      expect(contention.waitingClaimants).toBeGreaterThan(MAX_ATTEMPTS);

      await lockHolder.query('COMMIT');
      barrierHeld = false;
      claims = await Promise.all(pendingClaims);
    } finally {
      if (barrierHeld) await lockHolder.query('ROLLBACK');
      if (pendingClaims.length > 0) await Promise.allSettled(pendingClaims);
    }

    const returnedAttempts = claims
      .flat()
      .map((claim) => claim.attempts)
      .sort((left, right) => left - right);

    expect(returnedAttempts).toEqual([1, 2, 3]);

    const storedRows = await sql`
      SELECT attempts
      FROM otp_codes
      WHERE id = ${otpId}
    `;
    expect(storedRows).toEqual([{ attempts: MAX_ATTEMPTS }]);

    await expect(registerActiveOtpAttempts(sql, userId, MAX_ATTEMPTS)).resolves.toEqual([]);
  });
});
