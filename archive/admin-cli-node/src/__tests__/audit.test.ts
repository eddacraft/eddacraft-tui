import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { runAuditCommand, type AdminReader, type AuditResponse } from '../commands/audit.js';

function makeClient(result: unknown): AdminReader & { get: ReturnType<typeof vi.fn> } {
  const get = vi.fn(async () => result) as unknown as AdminReader['get'] & ReturnType<typeof vi.fn>;
  return { get } as AdminReader & { get: ReturnType<typeof vi.fn> };
}

const ENV_KEY = 'ANVIL_ADMIN_KEY';

const sampleResponse: AuditResponse = {
  total: 2,
  items: [
    {
      id: '42',
      action: 'user.approved',
      actor: 'josh@arkahna.io',
      metadata: { email: 'alice@example.com' },
      created_at: '2026-04-17T12:34:56Z',
    },
    {
      id: '41',
      action: 'user.invited',
      actor: 'admin',
      metadata: {},
      created_at: '2026-04-16T10:00:00Z',
    },
  ],
};

describe('runAuditCommand', () => {
  const originalKey = process.env[ENV_KEY];

  beforeEach(() => {
    process.env[ENV_KEY] = 'test-key';
  });

  afterEach(() => {
    if (originalKey === undefined) delete process.env[ENV_KEY];
    else process.env[ENV_KEY] = originalKey;
  });

  it('forwards filters to GET /admin/audit, mapping filterActor to actor', async () => {
    const client = makeClient({ total: 0, items: [] } satisfies AuditResponse);
    await runAuditCommand(
      {
        action: 'user.approved',
        filterActor: 'josh@arkahna.io',
        limit: 25,
        offset: 10,
        json: true,
      },
      { createClient: () => client, stdout: () => {} }
    );
    expect(client.get).toHaveBeenCalledWith(
      '/admin/audit',
      {
        action: 'user.approved',
        actor: 'josh@arkahna.io',
        limit: 25,
        offset: 10,
      },
      expect.anything()
    );
  });

  it('omits unset filters from the query object', async () => {
    const client = makeClient({ total: 0, items: [] } satisfies AuditResponse);
    await runAuditCommand({}, { createClient: () => client, stdout: () => {} });
    expect(client.get).toHaveBeenCalledWith('/admin/audit', {}, expect.anything());
  });

  it('emits pretty-printed JSON when --json is set', async () => {
    const writes: string[] = [];
    await runAuditCommand(
      { json: true },
      { createClient: () => makeClient(sampleResponse), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toBe(JSON.stringify(sampleResponse, null, 2) + '\n');
  });

  it('renders a table with WHEN/ACTION/ACTOR/METADATA and a total footer', async () => {
    const writes: string[] = [];
    await runAuditCommand(
      {},
      { createClient: () => makeClient(sampleResponse), stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('WHEN');
    expect(out).toContain('ACTION');
    expect(out).toContain('ACTOR');
    expect(out).toContain('METADATA');
    expect(out).toContain('user.approved');
    expect(out).toContain('josh@arkahna.io');
    expect(out).toContain('2026-04-17 12:34:56');
    expect(out).not.toContain('T12:34');
    expect(out).toContain('Showing 2 of 2');
  });

  it('serialises non-empty metadata as JSON and leaves empty metadata blank', async () => {
    const writes: string[] = [];
    await runAuditCommand(
      {},
      { createClient: () => makeClient(sampleResponse), stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('{"email":"alice@example.com"}');
    expect(out).not.toContain('{}');
  });

  it('prints "No audit entries." on empty result without --json', async () => {
    const writes: string[] = [];
    await runAuditCommand(
      {},
      {
        createClient: () => makeClient({ total: 0, items: [] } satisfies AuditResponse),
        stdout: (s) => writes.push(s),
      }
    );
    expect(writes.join('')).toBe('No audit entries.\n');
  });

  it('still emits JSON on empty result when --json is set', async () => {
    const writes: string[] = [];
    await runAuditCommand(
      { json: true },
      {
        createClient: () => makeClient({ total: 0, items: [] } satisfies AuditResponse),
        stdout: (s) => writes.push(s),
      }
    );
    const out = writes.join('');
    expect(out).toContain('"total": 0');
    expect(out).toContain('"items": []');
    expect(out).not.toContain('No audit entries');
  });

  it('throws MissingConfigError (exitCode 5) when no key is available', async () => {
    delete process.env[ENV_KEY];
    await expect(
      runAuditCommand({}, { createClient: () => makeClient(sampleResponse) })
    ).rejects.toMatchObject({ exitCode: 5, name: 'MissingConfigError' });
  });
});
