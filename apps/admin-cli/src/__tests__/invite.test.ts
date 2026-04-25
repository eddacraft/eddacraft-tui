import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { runInviteCommand, type InviteResponse } from '../commands/invite.js';
import type { AdminWriter } from '../client.js';

function makeClient(result: unknown): AdminWriter & { post: ReturnType<typeof vi.fn> } {
  const post = vi.fn(async () => result) as unknown as AdminWriter['post'] &
    ReturnType<typeof vi.fn>;
  return { post } as AdminWriter & { post: ReturnType<typeof vi.fn> };
}

const ENV_KEY = 'ANVIL_ADMIN_KEY';

const defaultResponse: InviteResponse = {
  user: { email: 'alice@example.com', id: 'usr-1' },
  scopes: ['beta'],
};

const tokenOnlyResponse: InviteResponse = {
  user: { email: 'ci@example.com', id: 'usr-2' },
  scopes: ['internal'],
  token: 'tok_abc.SECRET',
  expiresAt: '2026-07-15T00:00:00Z',
};

describe('runInviteCommand', () => {
  const originalKey = process.env[ENV_KEY];

  beforeEach(() => {
    process.env[ENV_KEY] = 'test-key';
  });

  afterEach(() => {
    if (originalKey === undefined) delete process.env[ENV_KEY];
    else process.env[ENV_KEY] = originalKey;
  });

  it('POSTs /admin/invite with just the email when no other flags are set', async () => {
    const client = makeClient(defaultResponse);
    await runInviteCommand(
      'alice@example.com',
      {},
      { createClient: () => client, stdout: () => {} }
    );
    expect(client.post).toHaveBeenCalledWith(
      '/admin/invite',
      { email: 'alice@example.com' },
      expect.anything()
    );
  });

  it('forwards name, notes, days, and scopes when provided', async () => {
    const client = makeClient(defaultResponse);
    await runInviteCommand(
      'alice@example.com',
      { name: 'Alice', notes: 'beta cohort 1', days: 30, scope: ['beta', 'preview'] },
      { createClient: () => client, stdout: () => {} }
    );
    expect(client.post).toHaveBeenCalledWith(
      '/admin/invite',
      {
        email: 'alice@example.com',
        name: 'Alice',
        notes: 'beta cohort 1',
        days: 30,
        scopes: ['beta', 'preview'],
      },
      expect.anything()
    );
  });

  it('renders a success summary in text mode', async () => {
    const writes: string[] = [];
    await runInviteCommand(
      'alice@example.com',
      {},
      { createClient: () => makeClient(defaultResponse), stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('Invited alice@example.com');
    expect(out).toContain('scopes: beta');
  });

  it('emits pretty-printed JSON with --json', async () => {
    const writes: string[] = [];
    await runInviteCommand(
      'alice@example.com',
      { json: true },
      { createClient: () => makeClient(defaultResponse), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toBe(JSON.stringify(defaultResponse, null, 2) + '\n');
  });

  it('sends tokenOnly and prints raw token to stdout + banner to stderr', async () => {
    const outs: string[] = [];
    const errs: string[] = [];
    const client = makeClient(tokenOnlyResponse);
    await runInviteCommand(
      'ci@example.com',
      { tokenOnly: true, scope: ['internal'] },
      { createClient: () => client, stdout: (s) => outs.push(s), stderr: (s) => errs.push(s) }
    );
    expect(client.post).toHaveBeenCalledWith(
      '/admin/invite',
      {
        email: 'ci@example.com',
        scopes: ['internal'],
        tokenOnly: true,
      },
      expect.anything()
    );
    expect(outs.join('')).toBe('tok_abc.SECRET\n');
    const err = errs.join('');
    expect(err).toContain('ONE-TIME ACCESS TOKEN');
    expect(err).toContain('ci@example.com');
    expect(err).toContain('internal');
    expect(err).toContain('2026-07-15T00:00:00Z');
  });

  it('throws (exitCode 2) when tokenOnly response lacks a token', async () => {
    const client = makeClient({ ...tokenOnlyResponse, token: undefined });
    await expect(
      runInviteCommand(
        'ci@example.com',
        { tokenOnly: true },
        { createClient: () => client, stdout: () => {}, stderr: () => {} }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 2 });
  });

  it('rejects combining --token-only with --json (exitCode 64)', async () => {
    const client = makeClient(tokenOnlyResponse);
    await expect(
      runInviteCommand(
        'ci@example.com',
        { tokenOnly: true, json: true },
        { createClient: () => client, stdout: () => {}, stderr: () => {} }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
    expect(client.post).not.toHaveBeenCalled();
  });

  it('rejects --days out of range (exitCode 64)', async () => {
    await expect(
      runInviteCommand(
        'alice@example.com',
        { days: 0 },
        { createClient: () => makeClient(defaultResponse) }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
    await expect(
      runInviteCommand(
        'alice@example.com',
        { days: 366 },
        { createClient: () => makeClient(defaultResponse) }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('propagates AdminError on 409 conflict from the server', async () => {
    const post = vi.fn(async () => {
      throw Object.assign(new Error('Email already invited'), {
        name: 'AdminError',
        exitCode: 1,
        status: 409,
      });
    });
    const client = { post } as unknown as AdminWriter;
    await expect(
      runInviteCommand('alice@example.com', {}, { createClient: () => client, stdout: () => {} })
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 1, status: 409 });
  });

  it('throws MissingConfigError (exitCode 5) when no key is available', async () => {
    delete process.env[ENV_KEY];
    await expect(
      runInviteCommand('alice@example.com', {}, { createClient: () => makeClient(defaultResponse) })
    ).rejects.toMatchObject({ exitCode: 5, name: 'MissingConfigError' });
  });
});
