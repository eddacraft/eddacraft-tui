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

/** The redacted structured argument itself, before console formatting. */
function capturePayload(message: string, data: unknown): unknown {
  const spy = vi.spyOn(console, 'debug').mockImplementation(() => {});
  try {
    createDebugger('auth-device')(message, data);
    return spy.mock.calls[0]?.[1];
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

  // The type-aware key rule exists so counters and flags stay readable. It must
  // exempt ONLY those: a structured value under a credential name is still a
  // credential, and writing the guard as "not a string" instead of "is a number
  // or boolean" silently reopened every one of these.
  it('redacts structured values filed under a credential key', () => {
    const object = captureDebug('cfg', { credentials: { user: 'svc', pass: 'hunter2' } });
    expect(object).not.toContain('hunter2');

    const array = captureDebug('cfg', { tokens: ['plainshortvalue'] });
    expect(array).not.toContain('plainshortvalue');

    const map = captureDebug('cfg', { apiKeys: new Map([['prod', 'shortkeyvalue']]) });
    expect(map).not.toContain('shortkeyvalue');

    const set = captureDebug('cfg', { secrets: new Set(['hunter2']) });
    expect(set).not.toContain('hunter2');

    const nested = captureDebug('cfg', { password: { current: 'hunter2' } });
    expect(nested).not.toContain('hunter2');
  });

  it('keeps distinct redacted keys distinct instead of collapsing them', () => {
    // Three throttle buckets must not render as one entry whose value is
    // whichever happened to come last — that is specific-but-wrong output.
    // Asserted on the payload object rather than the rendered line: the
    // timestamp prefix contains stray digits and would satisfy a text match.
    const payload = capturePayload('bucket state', {
      'a@example.com': 1,
      'b@example.com': 2,
      'c@example.com': 3,
    }) as Record<string, unknown>;

    expect(JSON.stringify(payload)).not.toContain('@example.com');
    expect(Object.keys(payload)).toHaveLength(3);
    expect(Object.values(payload).sort()).toEqual([1, 2, 3]);
  });

  it('renders prototype-shadowing key names as ordinary own properties', () => {
    // `'constructor' in {}` is true through the prototype chain, and assigning
    // `__proto__` on a literal sets the prototype instead of a property — both
    // would misrender a field the caller actually passed. Built via JSON.parse
    // because that is the realistic source of such a key (a parsed upstream
    // body) and it yields own properties rather than a prototype write.
    const payload = capturePayload(
      'proto probe',
      JSON.parse('{"__proto__":"protovalue","constructor":"ctorvalue","toString":"tsvalue"}')
    ) as Record<string, unknown>;

    expect(Object.getOwnPropertyNames(payload).sort()).toEqual([
      '__proto__',
      'constructor',
      'toString',
    ]);
    expect(Object.getOwnPropertyDescriptor(payload, '__proto__')?.value).toBe('protovalue');
    expect(payload.constructor).toBe('ctorvalue');
    expect(Object.getPrototypeOf(payload)).toBe(Object.prototype);
  });

  it('contains a hostile node without discarding its siblings', () => {
    const hostile = new Proxy(
      {},
      {
        getPrototypeOf() {
          throw new Error('proxy denies');
        },
      }
    );

    const output = captureDebug('mixed context', {
      usefulA: 'keep-me-A',
      hostile,
      usefulC: 'keep-me-C',
    });

    expect(output).toContain('keep-me-A');
    expect(output).toContain('keep-me-C');
  });

  it('filters an error name as well as its message', () => {
    const err = new Error('inner');
    err.name = 'ErrFor victim@example.com';

    expect(captureDebug('link failed', { cause: err })).not.toContain('victim@example.com');
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
