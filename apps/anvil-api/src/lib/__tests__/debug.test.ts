import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createDebugger, createInfoLogger } from '../debug.js';

/**
 * Capture everything `console.debug` receives for one debug call and flatten it
 * to a single searchable string. Asserting on the captured console output (not
 * on an internal redaction helper) is deliberate: CIB-214's guarantee is about
 * what reaches the operator's console, so the test has to cross the same
 * boundary the production call sites cross.
 */
function captureDebug(message: string, data?: unknown): string {
  const spy = vi.spyOn(console, 'debug').mockImplementation(() => {});
  try {
    createDebugger('auth-device')(message, data);
    return spy.mock.calls.map((call) => call.map((arg) => inspect(arg)).join(' ')).join('\n');
  } finally {
    spy.mockRestore();
  }
}

function inspect(value: unknown): string {
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}

describe('debug structured payload redaction (CIB-214)', () => {
  beforeEach(() => {
    vi.stubEnv('ANVIL_DEBUG', '1');
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it('redacts a device user code passed as a structured field', () => {
    const output = captureDebug('device code created', { userCode: 'ANVIL-8F3A21BC' });

    expect(output).not.toContain('ANVIL-8F3A21BC');
    expect(output).toContain('[REDACTED]');
  });

  it('survives a circular payload instead of overflowing the stack', () => {
    const cyclic: Record<string, unknown> = { userId: 'u-1' };
    cyclic.self = cyclic;

    expect(() => captureDebug('cyclic context', cyclic)).not.toThrow();
    expect(captureDebug('cyclic context', cyclic)).toContain('[CIRCULAR]');
  });

  it('walks arrays without reshaping them into index-keyed objects', () => {
    const output = captureDebug('batch skip', {
      rows: [{ userCode: 'ANVIL-AAAA1111' }, { userCode: 'ANVIL-BBBB2222' }],
    });

    expect(output).not.toContain('ANVIL-AAAA1111');
    expect(output).not.toContain('ANVIL-BBBB2222');
    expect(output).toContain('"rows":[{"userCode":"[REDACTED]"},{"userCode":"[REDACTED]"}]');
  });

  it('keeps nested errors and dates meaningful instead of collapsing them to {}', () => {
    const output = captureDebug('link failed', {
      cause: new Error('upstream rejected ghp_aaaabbbbccccddddeeee'),
      expiresAt: new Date('2026-08-06T00:00:00.000Z'),
    });

    expect(output).not.toContain('ghp_aaaabbbbccccddddeeee');
    expect(output).toContain('upstream rejected [REDACTED]');
    expect(output).toContain('2026-08-06T00:00:00.000Z');
  });

  it('caps recursion depth so a hostile nested payload cannot overflow the stack', () => {
    // JSON.parse happily produces thousands of levels, so an upstream body that
    // ever reached a debug call could otherwise exhaust the stack.
    let deep: Record<string, unknown> = { userCode: 'ANVIL-DEEPLEAK' };
    for (let i = 0; i < 5000; i += 1) {
      deep = { nested: deep };
    }

    let output = '';
    expect(() => {
      output = captureDebug('deep upstream body', deep);
    }).not.toThrow();
    expect(output).toContain('[TRUNCATED]');
    expect(output).not.toContain('ANVIL-DEEPLEAK');
  });

  // Guard against a future widening of the deny-list. Each field below is a
  // near-miss for a credential pattern but is ordinary operational context, and
  // redacting it would make the debug output useless without making it safer.
  it('leaves ordinary operational context readable', () => {
    const output = captureDebug('upstream settled', {
      windowMs: 60000,
      max: 30,
      count: 7,
      attempt: 2,
      ms: 412,
      httpStatus: 502,
      errorClass: 'fetch_error',
      outcome: 'non_ok',
      status: 'pending',
      reason: 'no_match',
      userId: 'usr_7f3a',
      githubId: 4242,
      familyId: 'fam_91b2',
      deliveryCode: 'rate_limited',
      githubDeviceSessions: 12,
      authMethod: 'shared',
    });

    expect(output).not.toContain('[REDACTED]');
    for (const expected of [
      '"windowMs":60000',
      '"httpStatus":502',
      '"errorClass":"fetch_error"',
      '"userId":"usr_7f3a"',
      '"githubId":4242',
      '"deliveryCode":"rate_limited"',
      '"githubDeviceSessions":12',
      '"authMethod":"shared"',
    ]) {
      expect(output).toContain(expected);
    }
  });

  // These are the exact object literals the production call sites pass. Hand
  // picking a "representative" subset is how the first version of this guard
  // missed that `refreshTokens` (a purge row count) and `hasToken` (a presence
  // flag) were being destroyed by the credential rule.
  it('leaves the real cron cleanup payload fully readable', () => {
    const output = captureDebug('cleanup complete', {
      deviceCodes: 4,
      githubDeviceSessions: 12,
      otpCodes: 3,
      refreshTokens: 87,
      broadcastSnapshots: 2,
      telemetryBeacons: 9,
    });

    expect(output).not.toContain('[REDACTED]');
    expect(output).toContain('"refreshTokens":87');
  });

  it('leaves the real admin revoke and invite payloads fully readable', () => {
    const revoke = captureDebug('POST /admin/revoke', { hasEmail: true, hasToken: true });
    expect(revoke).not.toContain('[REDACTED]');
    expect(revoke).toContain('"hasToken":true');

    const invite = captureDebug('POST /admin/invite', {
      hasEmail: true,
      scopes: ['beta'],
      days: 30,
      tokenOnly: false,
      edict: false,
    });
    expect(invite).not.toContain('[REDACTED]');
    expect(invite).toContain('"tokenOnly":false');
  });

  it('does not let a throwing accessor turn a debug call into a request failure', () => {
    const hostile = {
      ok: 1,
      get boom(): string {
        throw new Error('getter exploded');
      },
    };

    expect(() => captureDebug('driver error context', hostile)).not.toThrow();
  });

  it('redacts emails and device codes even under an unlisted key name', () => {
    const output = captureDebug('unexpected context', {
      recipient: 'victim@example.com',
      note: 'user code ANVIL-8F3A21BC issued',
      detail: { username: 'admin@internal.example' },
    });

    expect(output).not.toContain('victim@example.com');
    expect(output).not.toContain('ANVIL-8F3A21BC');
    expect(output).not.toContain('admin@internal.example');
  });

  it('redacts identifying map and object keys, not just their values', () => {
    // rate-limit buckets by client IP and waitlist-throttle by email, so the
    // key is the PII in exactly the structures most likely to be dumped.
    const fromObject = captureDebug('bucket state', { 'victim@example.com': 3 });
    const fromMap = captureDebug('bucket state', new Map([['victim@example.com', 3]]));

    expect(fromObject).not.toContain('victim@example.com');
    expect(fromMap).not.toContain('victim@example.com');
  });

  it('summarises binary payloads instead of spilling them byte by byte', () => {
    const output = captureDebug('raw body', { chunk: Buffer.from('ANVIL-8F3A21BC') });

    expect(output).not.toContain('"0":65');
    expect(output).not.toContain('ANVIL-8F3A21BC');
    expect(output).toContain('Binary');
  });
});

describe('info log redaction (CIB-214)', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  function captureInfo(event: string, fields?: Record<string, unknown>): string {
    const spy = vi.spyOn(console, 'info').mockImplementation(() => {});
    try {
      createInfoLogger('auth-github-device')(event, fields as never);
      return spy.mock.calls.map((call) => call.map((arg) => inspect(arg)).join(' ')).join('\n');
    } finally {
      spy.mockRestore();
    }
  }

  it('redacts credential-shaped fields on the ungated production path', () => {
    // infoLog is not gated behind ANVIL_DEBUG, so a caller slip here reaches
    // production logs directly — the same key boundary has to apply.
    const output = captureInfo('token_exchange.upstream', {
      outcome: 'ok',
      ms: 118,
      accessToken: 'gho_liveproductiontoken',
      email: 'operator@example.com',
    });

    expect(output).not.toContain('gho_liveproductiontoken');
    expect(output).not.toContain('operator@example.com');
    expect(output).toContain('"outcome":"ok"');
    expect(output).toContain('"ms":118');
  });
});
