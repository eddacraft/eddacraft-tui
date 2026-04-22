import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { runApproveCommand, type ApproveResponse } from '../commands/approve.js';
import type { AdminWriter } from '../client.js';

function makeClient(result: unknown): AdminWriter & { post: ReturnType<typeof vi.fn> } {
  const post = vi.fn(async () => result) as unknown as AdminWriter['post'] &
    ReturnType<typeof vi.fn>;
  return { post } as AdminWriter & { post: ReturnType<typeof vi.fn> };
}

const ENV_KEY = 'ANVIL_ADMIN_KEY';

const singleResponse: ApproveResponse = {
  approved: [{ email: 'alice@example.com', expiresAt: '2026-05-01T12:00:00Z' }],
};

const batchResponse: ApproveResponse = {
  approved: [
    { email: 'alice@example.com', expiresAt: '2026-05-01T12:00:00Z' },
    { email: 'bob@example.com', expiresAt: '2026-05-01T12:00:00Z' },
  ],
  skipped: [{ email: 'charlie@example.com', reason: 'collision', message: 'user_code collision' }],
};

describe('runApproveCommand', () => {
  const originalKey = process.env[ENV_KEY];

  beforeEach(() => {
    process.env[ENV_KEY] = 'test-key';
  });

  afterEach(() => {
    if (originalKey === undefined) delete process.env[ENV_KEY];
    else process.env[ENV_KEY] = originalKey;
  });

  it('POSTs { email } and renders approved table when --yes is set', async () => {
    const writes: string[] = [];
    const client = makeClient(singleResponse);
    await runApproveCommand(
      'alice@example.com',
      { yes: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    expect(client.post).toHaveBeenCalledWith(
      '/admin/approve',
      { email: 'alice@example.com' },
      expect.anything()
    );
    const out = writes.join('');
    expect(out).toContain('Approved 1');
    expect(out).toContain('alice@example.com');
    expect(out).toContain('EMAIL');
    expect(out).toContain('INVITE EXPIRES');
    expect(out).toContain('2026-05-01 12:00:00');
  });

  it('POSTs { batch: N } and prints skipped entries to stderr', async () => {
    const outs: string[] = [];
    const errs: string[] = [];
    const client = makeClient(batchResponse);
    await runApproveCommand(
      undefined,
      { batch: 5, yes: true },
      { createClient: () => client, stdout: (s) => outs.push(s), stderr: (s) => errs.push(s) }
    );
    expect(client.post).toHaveBeenCalledWith('/admin/approve', { batch: 5 }, expect.anything());
    expect(outs.join('')).toContain('Approved 2');
    const err = errs.join('');
    expect(err).toContain('Skipped 1');
    expect(err).toContain('charlie@example.com');
    expect(err).toContain('collision');
  });

  it('prompts the operator and aborts on "n"', async () => {
    const outs: string[] = [];
    const client = makeClient(singleResponse);
    const prompt = vi.fn(async () => 'n');
    await runApproveCommand(
      'alice@example.com',
      {},
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        prompt,
        isTTY: true,
      }
    );
    expect(prompt).toHaveBeenCalledOnce();
    expect(prompt.mock.calls[0]?.[0]).toContain('Approve alice@example.com?');
    expect(client.post).not.toHaveBeenCalled();
    expect(outs.join('')).toContain('Aborted.');
  });

  // #948: with --json, a declined confirmation must not pollute stdout.
  it('with --json, a declined confirmation routes "Aborted" to stderr', async () => {
    const client = makeClient(singleResponse);
    const outs: string[] = [];
    const errs: string[] = [];
    const prompt = vi.fn(async () => 'n');
    await runApproveCommand(
      'alice@example.com',
      { json: true },
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: (s) => errs.push(s),
        prompt,
        isTTY: true,
      }
    );
    expect(client.post).not.toHaveBeenCalled();
    expect(outs.join('')).toBe(''); // stdout stays pure JSON-safe (empty)
    expect(errs.join('')).toContain('Aborted.');
  });

  it('prompts and proceeds on "y"', async () => {
    const client = makeClient(singleResponse);
    const prompt = vi.fn(async () => 'Y');
    await runApproveCommand(
      'alice@example.com',
      {},
      { createClient: () => client, stdout: () => {}, prompt, isTTY: true }
    );
    expect(client.post).toHaveBeenCalled();
  });

  it('builds a batch-specific prompt summary', async () => {
    const client = makeClient({ approved: [], skipped: [] } satisfies ApproveResponse);
    const prompt = vi.fn(async () => 'y');
    await runApproveCommand(
      undefined,
      { batch: 7 },
      { createClient: () => client, stdout: () => {}, prompt, isTTY: true }
    );
    expect(prompt.mock.calls[0]?.[0]).toContain('oldest 7 unapproved');
  });

  it('throws AdminError (exitCode 4) when non-TTY and --yes is not set', async () => {
    const client = makeClient(singleResponse);
    await expect(
      runApproveCommand(
        'alice@example.com',
        {},
        { createClient: () => client, stdout: () => {}, isTTY: false }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 4 });
  });

  it('rejects missing email and batch (exitCode 64)', async () => {
    await expect(
      runApproveCommand(
        undefined,
        { yes: true },
        { createClient: () => makeClient(singleResponse) }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('rejects combining email with --batch (exitCode 64)', async () => {
    await expect(
      runApproveCommand(
        'alice@example.com',
        { batch: 5, yes: true },
        { createClient: () => makeClient(singleResponse) }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('rejects --batch out of range (exitCode 64)', async () => {
    await expect(
      runApproveCommand(
        undefined,
        { batch: 0, yes: true },
        { createClient: () => makeClient(singleResponse) }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
    await expect(
      runApproveCommand(
        undefined,
        { batch: 101, yes: true },
        { createClient: () => makeClient(singleResponse) }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('emits pretty-printed JSON when --json is set', async () => {
    const writes: string[] = [];
    await runApproveCommand(
      'alice@example.com',
      { yes: true, json: true },
      {
        createClient: () => makeClient(singleResponse),
        stdout: (s) => writes.push(s),
      }
    );
    expect(writes.join('')).toBe(JSON.stringify(singleResponse, null, 2) + '\n');
  });

  it('propagates AdminError on 404 from client', async () => {
    const post = vi.fn(async () => {
      throw Object.assign(new Error('Email not found on waitlist'), {
        name: 'AdminError',
        exitCode: 1,
        status: 404,
      });
    });
    const client = { post } as unknown as AdminWriter;
    await expect(
      runApproveCommand(
        'nobody@example.com',
        { yes: true },
        { createClient: () => client, stdout: () => {} }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 1, status: 404 });
  });

  it('throws MissingConfigError (exitCode 5) when no key is available', async () => {
    delete process.env[ENV_KEY];
    await expect(
      runApproveCommand(
        'alice@example.com',
        { yes: true },
        { createClient: () => makeClient(singleResponse) }
      )
    ).rejects.toMatchObject({ exitCode: 5, name: 'MissingConfigError' });
  });
});
