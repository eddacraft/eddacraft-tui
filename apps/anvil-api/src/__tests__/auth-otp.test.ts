import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { generateKeyPair, exportPKCS8 } from 'jose';
import { Hono } from 'hono';
import { authOtp } from '../routes/auth-otp.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', () => ({
  findUserByEmail: vi.fn(),
  countActiveOtpCodes: vi.fn(),
  insertOtpCode: vi.fn(),
  registerActiveOtpAttempts: vi.fn(),
  consumeOtpCode: vi.fn(),
  insertRefreshToken: vi.fn(),
  findActiveScopesForUser: vi.fn(),
}));

vi.mock('../lib/email.js', () => ({
  sendOtpCode: vi.fn(),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    hashToken: vi.fn(),
  };
});

import {
  consumeOtpCode,
  countActiveOtpCodes,
  registerActiveOtpAttempts,
  findUserByEmail,
  insertOtpCode,
  insertRefreshToken,
  findActiveScopesForUser,
  type OtpCode,
} from '../db/queries.js';
import { sendOtpCode } from '../lib/email.js';
import { hashToken } from '../lib/token.js';

const app = new Hono();
app.route('/auth/otp', authOtp);

let originalSigningKey: string | undefined;

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  const { privateKey } = await generateKeyPair('ES256', { extractable: true });
  process.env['LICENSE_SIGNING_KEY'] = await exportPKCS8(privateKey);
});

afterAll(() => {
  if (originalSigningKey === undefined) delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
});

function makeOtpCodeRow(overrides: Partial<OtpCode> = {}): OtpCode {
  return {
    id: 'otp-inserted',
    user_id: 'user-1',
    code_hash: 'hash:000000',
    attempts: 0,
    expires_at: new Date(Date.now() + 600_000).toISOString(),
    consumed_at: null,
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

function makeRefreshTokenRow() {
  return {
    id: 'rt-inserted',
    user_id: 'user-1',
    token_hash: 'hash:refresh',
    family_id: 'family-1',
    expires_at: new Date(Date.now() + 90 * 24 * 60 * 60 * 1000).toISOString(),
    revoked_at: null,
    consumed_at: null,
    created_at: new Date().toISOString(),
  };
}

beforeEach(() => {
  vi.resetAllMocks();
  // Re-state every default because resetAllMocks wipes implementations as
  // well as call history. Deterministic hashToken so tests can set up
  // matching hashes without knowing the raw code bytes ahead of time.
  vi.mocked(hashToken).mockImplementation((input: string) => `hash:${input}`);
  vi.mocked(findUserByEmail).mockResolvedValue(null);
  vi.mocked(countActiveOtpCodes).mockResolvedValue(0);
  vi.mocked(registerActiveOtpAttempts).mockResolvedValue([]);
  vi.mocked(insertOtpCode).mockResolvedValue(makeOtpCodeRow());
  vi.mocked(consumeOtpCode).mockResolvedValue(true);
  vi.mocked(insertRefreshToken).mockResolvedValue(makeRefreshTokenRow());
  vi.mocked(findActiveScopesForUser).mockResolvedValue(['beta']);
  vi.mocked(sendOtpCode).mockResolvedValue({ sent: true });
});

afterEach(() => {
  vi.restoreAllMocks();
});

type UserRow = {
  id: string;
  email: string;
  name: string | null;
  status: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
};

function activeUser(overrides: Partial<UserRow> = {}): UserRow {
  return {
    id: 'user-1',
    email: 'active@example.com',
    name: 'Active',
    status: 'active',
    notes: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  };
}

function makeCode(overrides: Partial<OtpCode> = {}): OtpCode {
  return {
    id: 'otp-1',
    user_id: 'user-1',
    code_hash: 'hash:123456',
    attempts: 0,
    expires_at: new Date(Date.now() + 60_000).toISOString(),
    consumed_at: null,
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

function post(path: string, body: unknown) {
  return app.request(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

const SUCCESS_RESPONSE = { sent: true, expiresIn: 600 };
const INVALID_CODE_ERROR = { error: 'Invalid or expired code' };

describe('POST /auth/otp/request', () => {
  it('returns the success shape for an unknown email without inserting a code', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(null);

    const res = await post('/auth/otp/request', { email: 'nobody@example.com' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(SUCCESS_RESPONSE);
    expect(vi.mocked(insertOtpCode)).not.toHaveBeenCalled();
    expect(vi.mocked(sendOtpCode)).not.toHaveBeenCalled();
  });

  it('returns the success shape for a suspended user without inserting a code', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser({ status: 'suspended' }));

    const res = await post('/auth/otp/request', { email: 'susp@example.com' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(SUCCESS_RESPONSE);
    expect(vi.mocked(insertOtpCode)).not.toHaveBeenCalled();
    expect(vi.mocked(sendOtpCode)).not.toHaveBeenCalled();
  });

  it('returns the same success shape for a known active user and triggers email', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());

    const res = await post('/auth/otp/request', { email: 'active@example.com' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(SUCCESS_RESPONSE);
    expect(vi.mocked(insertOtpCode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(sendOtpCode)).toHaveBeenCalledWith(
      'active@example.com',
      expect.stringMatching(/^\d{6}$/)
    );
  });

  it('lowercases the email before looking up the user', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());

    const res = await post('/auth/otp/request', { email: 'Active@Example.COM' });

    expect(res.status).toBe(200);
    expect(vi.mocked(findUserByEmail)).toHaveBeenCalledWith(
      expect.anything(),
      'active@example.com'
    );
  });

  it('silently rate-limits when MAX_ACTIVE_CODES is reached', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(countActiveOtpCodes).mockResolvedValue(3);

    const res = await post('/auth/otp/request', { email: 'active@example.com' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(SUCCESS_RESPONSE);
    expect(vi.mocked(insertOtpCode)).not.toHaveBeenCalled();
    expect(vi.mocked(sendOtpCode)).not.toHaveBeenCalled();
  });

  it('inserts a fresh code when active count is below the limit', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(countActiveOtpCodes).mockResolvedValue(2);

    const res = await post('/auth/otp/request', { email: 'active@example.com' });

    expect(res.status).toBe(200);
    expect(vi.mocked(insertOtpCode)).toHaveBeenCalledTimes(1);
  });

  it('still returns the success shape when the email provider reports a failure', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(sendOtpCode).mockResolvedValue({
      sent: false,
      code: 'provider_error',
      message: 'down',
    });

    const res = await post('/auth/otp/request', { email: 'active@example.com' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(SUCCESS_RESPONSE);
  });

  it.each([
    { name: 'missing email', body: {} },
    { name: 'invalid email format', body: { email: 'not-an-email' } },
    { name: 'over-length email', body: { email: `${'a'.repeat(250)}@example.com` } },
  ])('returns 400 for $name without calling the DB', async ({ body }) => {
    const res = await post('/auth/otp/request', body);
    expect(res.status).toBe(400);
    expect(vi.mocked(findUserByEmail)).not.toHaveBeenCalled();
  });
});

describe('POST /auth/otp/verify', () => {
  const SUBMITTED_CODE = '123456';
  const SUBMITTED_HASH = `hash:${SUBMITTED_CODE}`;

  it('returns the anti-enumeration error shape for unknown email', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(null);

    const res = await post('/auth/otp/verify', {
      email: 'nobody@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(registerActiveOtpAttempts)).not.toHaveBeenCalled();
  });

  it('returns the same error shape for a suspended user', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser({ status: 'suspended' }));

    const res = await post('/auth/otp/verify', { email: 'x@example.com', code: SUBMITTED_CODE });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(registerActiveOtpAttempts)).not.toHaveBeenCalled();
  });

  it('returns the same error shape when no active codes exist', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(registerActiveOtpAttempts).mockResolvedValue([]);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(consumeOtpCode)).not.toHaveBeenCalled();
  });

  it('registers the attempt atomically and rejects when the submitted code does not match', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    // registerActiveOtpAttempts has already incremented + filtered by cap; it
    // returns the eligible (below-cap) codes for comparison. None match here.
    vi.mocked(registerActiveOtpAttempts).mockResolvedValue([
      makeCode({ id: 'otp-a', code_hash: 'hash:999999', attempts: 1 }),
      makeCode({ id: 'otp-b', code_hash: 'hash:888888', attempts: 1 }),
    ]);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    // The route drives the cap through the single atomic query, keyed by user
    // and MAX_ATTEMPTS — not a separate read-then-write increment.
    expect(vi.mocked(registerActiveOtpAttempts)).toHaveBeenCalledWith(
      expect.anything(),
      'user-1',
      3
    );
    expect(vi.mocked(consumeOtpCode)).not.toHaveBeenCalled();
  });

  it('rejects a code at MAX_ATTEMPTS without evaluating it (atomic cap excludes it)', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    // A code already at the cap fails the `attempts < max` predicate, so the
    // atomic UPDATE never returns it — the correct hash is never handed back
    // for comparison, so the guess cannot be evaluated even though it matches.
    vi.mocked(registerActiveOtpAttempts).mockResolvedValue([]);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(consumeOtpCode)).not.toHaveBeenCalled();
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns the same error shape when consume races against another request', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(registerActiveOtpAttempts).mockResolvedValue([
      makeCode({ id: 'otp-race', code_hash: SUBMITTED_HASH, attempts: 1 }),
    ]);
    // Simulate concurrent verification: atomic consume reports 0 rows
    // because the sibling request already consumed the code.
    vi.mocked(consumeOtpCode).mockResolvedValue(false);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it('issues a licence and refresh token on the happy path', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(registerActiveOtpAttempts).mockResolvedValue([
      makeCode({ id: 'otp-match', code_hash: SUBMITTED_HASH, attempts: 1 }),
    ]);
    vi.mocked(consumeOtpCode).mockResolvedValue(true);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(typeof body.license).toBe('string');
    expect(body.license.split('.').length).toBe(3);
    expect(typeof body.refreshToken).toBe('string');
    expect(body.refreshToken.length).toBeGreaterThanOrEqual(32);
    expect(body.expiresAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);

    expect(vi.mocked(consumeOtpCode)).toHaveBeenCalledWith(expect.anything(), 'otp-match');
    expect(vi.mocked(insertRefreshToken)).toHaveBeenCalledWith(
      expect.anything(),
      'user-1',
      expect.stringMatching(/^hash:[0-9a-f]{64}$/),
      expect.stringMatching(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i),
      expect.any(Date)
    );
  });

  it('preserves graded scopes in the issued licence', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(registerActiveOtpAttempts).mockResolvedValue([
      makeCode({ id: 'otp-match', code_hash: SUBMITTED_HASH, attempts: 1 }),
    ]);
    vi.mocked(findActiveScopesForUser).mockResolvedValue(['preview', 'beta']);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(200);
    const body = await res.json();
    const { decodeJwt } = await import('jose');
    const claims = decodeJwt(body.license) as { scopes?: string[] };
    expect(claims.scopes).toEqual(['preview', 'beta']);
    expect(vi.mocked(findActiveScopesForUser)).toHaveBeenCalledWith(expect.anything(), 'user-1');
  });

  // CONTRACT TEST — NOT a real concurrency proof.
  //
  // The query layer is fully mocked in this suite (no Postgres harness exists
  // in anvil-api's tests — see queries.test.ts, which uses a `vi.fn()` sql).
  // The ATOMICITY of the increment (that at most MAX_ATTEMPTS guesses can ever
  // be incremented under N concurrent callers) is a single-statement
  // PostgreSQL row-lock guarantee, argued from SQL semantics in the
  // `registerActiveOtpAttempts` docstring — it is NOT, and cannot be, proven
  // by a single-threaded JS mock (a closure that re-implements the cap check
  // can never fail even if the SQL were wrong).
  //
  // What this test DOES prove is the route-level property we own: the route
  // delegates the cap entirely to the atomic claim and evaluates ONLY the rows
  // the claim returns. Here the mock models the claim's CONTRACT — return
  // below-cap rows, withhold capped rows — and we assert the route makes no
  // independent cap decision: it never evaluates or consumes a code the claim
  // did not hand back, no matter how many requests arrive.
  it('route only evaluates codes the atomic claim returns (delegates the cap; contract)', async () => {
    const MAX_ATTEMPTS = 3;
    const REQUESTS = 20;

    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());

    // The claim's contract: increment-and-return while below the cap, withhold
    // once at the cap. `returnedForComparison` counts rows the route was
    // actually given to compare.
    const claim = { attempts: 0 };
    let returnedForComparison = 0;
    let maxAttemptsSeen = -1;
    vi.mocked(registerActiveOtpAttempts).mockImplementation(
      async (_sql, _userId, maxAttempts: number) => {
        maxAttemptsSeen = maxAttempts;
        if (claim.attempts >= maxAttempts) return []; // capped: withheld
        claim.attempts += 1;
        returnedForComparison += 1;
        // Deliberately a NON-matching hash so no request ever consumes.
        return [makeCode({ id: 'otp-1', code_hash: 'hash:999999', attempts: claim.attempts })];
      }
    );

    const results = await Promise.all(
      Array.from({ length: REQUESTS }, () =>
        post('/auth/otp/verify', { email: 'active@example.com', code: SUBMITTED_CODE })
      )
    );

    // Route passes the cap to the atomic layer (never re-derives it) …
    expect(maxAttemptsSeen).toBe(MAX_ATTEMPTS);
    // … evaluates ONLY the rows the claim returned (nothing beyond the cap) …
    expect(returnedForComparison).toBe(MAX_ATTEMPTS);
    // … rejects every wrong guess and consumes nothing.
    expect(results.every((r) => r.status === 400)).toBe(true);
    expect(vi.mocked(consumeOtpCode)).not.toHaveBeenCalled();
  });

  it.each([
    { name: 'missing code', body: { email: 'active@example.com' } },
    { name: 'non-digit code', body: { email: 'active@example.com', code: 'abcdef' } },
    { name: 'short code', body: { email: 'active@example.com', code: '12345' } },
    { name: 'long code', body: { email: 'active@example.com', code: '1234567' } },
  ])('returns 400 for $name via Zod', async ({ body }) => {
    const res = await post('/auth/otp/verify', body);
    expect(res.status).toBe(400);
    expect(vi.mocked(findUserByEmail)).not.toHaveBeenCalled();
  });
});
