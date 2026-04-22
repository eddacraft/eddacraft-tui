import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { generateKeyPair, exportPKCS8 } from 'jose';
import { Hono } from 'hono';
import { authOtp } from '../routes/auth-otp.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', () => ({
  findUserByEmail: vi.fn(),
  countActiveOtpCodes: vi.fn().mockResolvedValue(0),
  insertOtpCode: vi.fn().mockResolvedValue(undefined),
  findActiveOtpCodes: vi.fn().mockResolvedValue([]),
  incrementOtpAttemptsBatch: vi.fn().mockResolvedValue(undefined),
  consumeOtpCode: vi.fn().mockResolvedValue(true),
  insertRefreshToken: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../lib/email.js', () => ({
  sendOtpCode: vi.fn().mockResolvedValue({ sent: true }),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    // Deterministic hash so tests can set up matching hashes without
    // knowing the code bytes ahead of time.
    hashToken: vi.fn((input: string) => `hash:${input}`),
  };
});

import {
  consumeOtpCode,
  countActiveOtpCodes,
  findActiveOtpCodes,
  findUserByEmail,
  incrementOtpAttemptsBatch,
  insertOtpCode,
  insertRefreshToken,
  type OtpCode,
} from '../db/queries.js';
import { sendOtpCode } from '../lib/email.js';

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

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(countActiveOtpCodes).mockResolvedValue(0);
  vi.mocked(findActiveOtpCodes).mockResolvedValue([]);
  vi.mocked(consumeOtpCode).mockResolvedValue(true);
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
    expect(vi.mocked(findActiveOtpCodes)).not.toHaveBeenCalled();
  });

  it('returns the same error shape for a suspended user', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser({ status: 'suspended' }));

    const res = await post('/auth/otp/verify', { email: 'x@example.com', code: SUBMITTED_CODE });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(findActiveOtpCodes)).not.toHaveBeenCalled();
  });

  it('returns the same error shape when no active codes exist', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(findActiveOtpCodes).mockResolvedValue([]);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(incrementOtpAttemptsBatch)).not.toHaveBeenCalled();
  });

  it('increments attempts on all active codes when the submitted code does not match', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(findActiveOtpCodes).mockResolvedValue([
      makeCode({ id: 'otp-a', code_hash: 'hash:999999' }),
      makeCode({ id: 'otp-b', code_hash: 'hash:888888' }),
    ]);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(incrementOtpAttemptsBatch)).toHaveBeenCalledWith(expect.anything(), [
      'otp-a',
      'otp-b',
    ]);
    expect(vi.mocked(consumeOtpCode)).not.toHaveBeenCalled();
  });

  it('locks out a matching code once MAX_ATTEMPTS is reached', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(findActiveOtpCodes).mockResolvedValue([
      makeCode({ id: 'otp-locked', code_hash: SUBMITTED_HASH, attempts: 3 }),
    ]);

    const res = await post('/auth/otp/verify', {
      email: 'active@example.com',
      code: SUBMITTED_CODE,
    });

    expect(res.status).toBe(400);
    expect(await res.json()).toEqual(INVALID_CODE_ERROR);
    expect(vi.mocked(incrementOtpAttemptsBatch)).toHaveBeenCalledWith(expect.anything(), [
      'otp-locked',
    ]);
    expect(vi.mocked(consumeOtpCode)).not.toHaveBeenCalled();
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns the same error shape when consume races against another request', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(findActiveOtpCodes).mockResolvedValue([
      makeCode({ id: 'otp-race', code_hash: SUBMITTED_HASH }),
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
    vi.mocked(findActiveOtpCodes).mockResolvedValue([
      makeCode({ id: 'otp-match', code_hash: SUBMITTED_HASH }),
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
