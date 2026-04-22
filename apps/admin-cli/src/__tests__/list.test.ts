import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { runListCommand, type AdminReader, type WaitlistResponse } from '../commands/list.js';

function makeClient(result: unknown): AdminReader & { get: ReturnType<typeof vi.fn> } {
  const get = vi.fn(async () => result) as unknown as AdminReader['get'] & ReturnType<typeof vi.fn>;
  return { get } as AdminReader & { get: ReturnType<typeof vi.fn> };
}

const ENV_KEY = 'ANVIL_ADMIN_KEY';

describe('runListCommand', () => {
  const originalKey = process.env[ENV_KEY];

  beforeEach(() => {
    process.env[ENV_KEY] = 'test-key';
  });

  afterEach(() => {
    if (originalKey === undefined) delete process.env[ENV_KEY];
    else process.env[ENV_KEY] = originalKey;
  });

  it('forwards filters to GET /admin/waitlist with numeric limit/offset', async () => {
    const client = makeClient({ total: 0, items: [] } satisfies WaitlistResponse);
    await runListCommand(
      { status: 'approved', source: 'manual', limit: 25, offset: 10, json: true },
      { createClient: () => client, stdout: () => {} }
    );
    expect(client.get).toHaveBeenCalledWith(
      '/admin/waitlist',
      {
        status: 'approved',
        source: 'manual',
        limit: 25,
        offset: 10,
      },
      expect.anything()
    );
  });

  it('omits unset filters from the query object', async () => {
    const client = makeClient({ total: 0, items: [] } satisfies WaitlistResponse);
    await runListCommand({}, { createClient: () => client, stdout: () => {} });
    expect(client.get).toHaveBeenCalledWith('/admin/waitlist', {}, expect.anything());
  });

  it('emits pretty-printed JSON when --json is set', async () => {
    const result: WaitlistResponse = {
      total: 1,
      items: [
        {
          email: 'a@b.c',
          name: null,
          source: 'manual',
          created_at: '2026-01-01T00:00:00Z',
          approved_at: null,
        },
      ],
    };
    const writes: string[] = [];
    await runListCommand(
      { json: true },
      { createClient: () => makeClient(result), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toBe(JSON.stringify(result, null, 2) + '\n');
  });

  it('renders a table with EMAIL/NAME/SOURCE/CREATED/APPROVED and a total footer', async () => {
    const result: WaitlistResponse = {
      total: 2,
      items: [
        {
          email: 'a@b.c',
          name: 'Alice',
          source: 'manual',
          created_at: '2026-01-01T12:00:00Z',
          approved_at: null,
        },
      ],
    };
    const writes: string[] = [];
    await runListCommand(
      {},
      { createClient: () => makeClient(result), stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('EMAIL');
    expect(out).toContain('NAME');
    expect(out).toContain('SOURCE');
    expect(out).toContain('CREATED');
    expect(out).toContain('APPROVED');
    expect(out).toContain('a@b.c');
    expect(out).toContain('Alice');
    expect(out).toContain('2026-01-01');
    expect(out).not.toContain('T12:00');
    expect(out).toContain('Showing 1 of 2');
  });

  it('renders approved_at as an em-dash when null', async () => {
    const result: WaitlistResponse = {
      total: 1,
      items: [
        {
          email: 'a@b.c',
          name: null,
          source: 'manual',
          created_at: '2026-01-01T00:00:00Z',
          approved_at: null,
        },
      ],
    };
    const writes: string[] = [];
    await runListCommand(
      {},
      { createClient: () => makeClient(result), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toContain('—');
  });

  it('prints "No waitlist entries." on empty result without --json', async () => {
    const writes: string[] = [];
    await runListCommand(
      {},
      {
        createClient: () => makeClient({ total: 0, items: [] } satisfies WaitlistResponse),
        stdout: (s) => writes.push(s),
      }
    );
    expect(writes.join('')).toBe('No waitlist entries.\n');
  });

  it('still emits JSON on empty result when --json is set', async () => {
    const writes: string[] = [];
    await runListCommand(
      { json: true },
      {
        createClient: () => makeClient({ total: 0, items: [] } satisfies WaitlistResponse),
        stdout: (s) => writes.push(s),
      }
    );
    const out = writes.join('');
    expect(out).toContain('"total": 0');
    expect(out).toContain('"items": []');
    expect(out).not.toContain('No waitlist entries');
  });

  it('throws MissingConfigError (exitCode 5) when no key is available', async () => {
    delete process.env[ENV_KEY];
    await expect(
      runListCommand({}, { createClient: () => makeClient({ total: 0, items: [] }) })
    ).rejects.toMatchObject({ exitCode: 5, name: 'MissingConfigError' });
  });

  it('accepts --key flag to override missing env', async () => {
    delete process.env[ENV_KEY];
    const client = makeClient({ total: 0, items: [] } satisfies WaitlistResponse);
    await runListCommand(
      { key: 'flag-key', actor: 'flag-actor' },
      { createClient: () => client, stdout: () => {} }
    );
    expect(client.get).toHaveBeenCalled();
  });
});
