import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { runShowCommand, type AdminReader, type ShowResponse } from '../commands/show.js';

function makeClient(result: unknown): AdminReader & { get: ReturnType<typeof vi.fn> } {
  const get = vi.fn(async () => result) as unknown as AdminReader['get'] & ReturnType<typeof vi.fn>;
  return { get } as AdminReader & { get: ReturnType<typeof vi.fn> };
}

const ENV_KEY = 'ANVIL_ADMIN_KEY';

const sampleResponse: ShowResponse = {
  user: {
    id: '11111111-1111-1111-1111-111111111111',
    email: 'alice@example.com',
    name: 'Alice',
    status: 'active',
    notes: 'beta cohort 1',
    created_at: '2026-01-02T03:04:05Z',
    updated_at: '2026-01-05T06:07:08Z',
  },
  tokens: [
    {
      id: 'tok-1',
      scopes: ['beta', 'preview'],
      expires_at: '2026-06-01T00:00:00Z',
      revoked_at: null,
      created_at: '2026-01-02T03:04:05Z',
    },
  ],
  recentAudit: [
    {
      id: 'aud-1',
      action: 'waitlist.approve',
      actor: 'ops@example.com',
      metadata: { email: 'alice@example.com' },
      created_at: '2026-01-05T12:00:00Z',
    },
  ],
};

describe('runShowCommand', () => {
  const originalKey = process.env[ENV_KEY];

  beforeEach(() => {
    process.env[ENV_KEY] = 'test-key';
  });

  afterEach(() => {
    if (originalKey === undefined) delete process.env[ENV_KEY];
    else process.env[ENV_KEY] = originalKey;
  });

  it('GETs /admin/user/:email with the email URL-encoded', async () => {
    const client = makeClient(sampleResponse);
    await runShowCommand(
      'a+b@example.com',
      { json: true },
      { createClient: () => client, stdout: () => {} }
    );
    expect(client.get).toHaveBeenCalledWith(
      '/admin/user/a%2Bb%40example.com',
      undefined,
      expect.anything()
    );
  });

  it('emits pretty-printed JSON when --json is set', async () => {
    const writes: string[] = [];
    await runShowCommand(
      'alice@example.com',
      { json: true },
      { createClient: () => makeClient(sampleResponse), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toBe(JSON.stringify(sampleResponse, null, 2) + '\n');
  });

  it('renders user, tokens, and recent audit sections in text mode', async () => {
    const writes: string[] = [];
    await runShowCommand(
      'alice@example.com',
      {},
      { createClient: () => makeClient(sampleResponse), stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('USER');
    expect(out).toContain('email:      alice@example.com');
    expect(out).toContain('name:       Alice');
    expect(out).toContain('status:     active');
    expect(out).toContain('notes:      beta cohort 1');
    expect(out).toContain('TOKENS');
    expect(out).toContain('SCOPES');
    expect(out).toContain('beta,preview');
    expect(out).toContain('RECENT AUDIT');
    expect(out).toContain('waitlist.approve');
    expect(out).toContain('ops@example.com');
    expect(out).toContain('2026-01-05 12:00:00');
  });

  it('renders "(none)" when the user has no tokens', async () => {
    const writes: string[] = [];
    await runShowCommand(
      'alice@example.com',
      {},
      {
        createClient: () => makeClient({ ...sampleResponse, tokens: [] } satisfies ShowResponse),
        stdout: (s) => writes.push(s),
      }
    );
    expect(writes.join('')).toContain('TOKENS\n------\n(none)');
  });

  it('renders "(none)" when recentAudit is empty', async () => {
    const writes: string[] = [];
    await runShowCommand(
      'alice@example.com',
      {},
      {
        createClient: () =>
          makeClient({ ...sampleResponse, recentAudit: [] } satisfies ShowResponse),
        stdout: (s) => writes.push(s),
      }
    );
    expect(writes.join('')).toContain('RECENT AUDIT\n------------\n(none)');
  });

  it('renders em-dash for revoked_at when null', async () => {
    const writes: string[] = [];
    await runShowCommand(
      'alice@example.com',
      {},
      { createClient: () => makeClient(sampleResponse), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toContain('—');
  });

  it('prints auditError warning on stderr when server signals it', async () => {
    const errs: string[] = [];
    await runShowCommand(
      'alice@example.com',
      {},
      {
        createClient: () =>
          makeClient({ ...sampleResponse, auditError: true } satisfies ShowResponse),
        stdout: () => {},
        stderr: (s) => errs.push(s),
      }
    );
    expect(errs.join('')).toContain('audit lookup failed');
  });

  it('prints auditError warning on stderr in --json mode too', async () => {
    const outs: string[] = [];
    const errs: string[] = [];
    await runShowCommand(
      'alice@example.com',
      { json: true },
      {
        createClient: () =>
          makeClient({ ...sampleResponse, auditError: true } satisfies ShowResponse),
        stdout: (s) => outs.push(s),
        stderr: (s) => errs.push(s),
      }
    );
    expect(errs.join('')).toContain('audit lookup failed');
    expect(outs.join('')).toContain('"auditError": true');
  });

  it('propagates AdminError on 404 from client', async () => {
    const get = vi.fn(async () => {
      throw Object.assign(new Error('User not found'), {
        name: 'AdminError',
        exitCode: 1,
        status: 404,
      });
    });
    const client = { get } as unknown as AdminReader;
    await expect(
      runShowCommand('nobody@example.com', {}, { createClient: () => client, stdout: () => {} })
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 1, status: 404 });
  });

  it('throws MissingConfigError (exitCode 5) when no key is available', async () => {
    delete process.env[ENV_KEY];
    await expect(
      runShowCommand('alice@example.com', {}, { createClient: () => makeClient(sampleResponse) })
    ).rejects.toMatchObject({ exitCode: 5, name: 'MissingConfigError' });
  });
});
