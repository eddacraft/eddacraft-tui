import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { runRevokeCommand, CONFIRM_WORD, type RevokeResponse } from '../commands/revoke.js';
import type { AdminWriter } from '../client.js';

function makeClient(result: unknown): AdminWriter & { post: ReturnType<typeof vi.fn> } {
  const post = vi.fn(async () => result) as unknown as AdminWriter['post'] &
    ReturnType<typeof vi.fn>;
  return { post } as AdminWriter & { post: ReturnType<typeof vi.fn> };
}

const ENV_KEY = 'ANVIL_ADMIN_KEY';

const emailResponse: RevokeResponse = { revoked: 3 };
const tokenResponse: RevokeResponse = { revoked: 1 };

describe('runRevokeCommand', () => {
  const originalKey = process.env[ENV_KEY];

  beforeEach(() => {
    process.env[ENV_KEY] = 'test-key';
  });

  afterEach(() => {
    if (originalKey === undefined) delete process.env[ENV_KEY];
    else process.env[ENV_KEY] = originalKey;
  });

  it('POSTs { email } and renders revoked count when --yes is set', async () => {
    const writes: string[] = [];
    const client = makeClient(emailResponse);
    await runRevokeCommand(
      'alice@example.com',
      { yes: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    expect(client.post).toHaveBeenCalledWith(
      '/admin/revoke',
      { email: 'alice@example.com' },
      expect.anything()
    );
    const out = writes.join('');
    expect(out).toContain('Revoked 3 token(s)');
    expect(out).toContain('alice@example.com');
  });

  it('POSTs { token } when --token is set', async () => {
    const writes: string[] = [];
    const client = makeClient(tokenResponse);
    await runRevokeCommand(
      undefined,
      { token: 'raw-token-abc', yes: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    expect(client.post).toHaveBeenCalledWith(
      '/admin/revoke',
      { token: 'raw-token-abc' },
      expect.anything()
    );
    expect(writes.join('')).toContain('Revoked 1 token(s) for token');
  });

  it('requires literal "revoke" as confirmation; aborts on anything else', async () => {
    const outs: string[] = [];
    const errs: string[] = [];
    const client = makeClient(emailResponse);
    const prompt = vi.fn(async () => 'y');
    await runRevokeCommand(
      'alice@example.com',
      {},
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: (s) => errs.push(s),
        prompt,
        isTTY: true,
      }
    );
    expect(prompt).toHaveBeenCalledOnce();
    expect(errs.join('')).toContain(`Type "${CONFIRM_WORD}"`);
    expect(errs.join('')).toContain('all tokens for alice@example.com');
    expect(client.post).not.toHaveBeenCalled();
    expect(outs.join('')).toContain('Aborted.');
  });

  it('proceeds when operator types the literal confirmation word', async () => {
    const client = makeClient(emailResponse);
    const prompt = vi.fn(async () => CONFIRM_WORD);
    await runRevokeCommand(
      'alice@example.com',
      {},
      { createClient: () => client, stdout: () => {}, stderr: () => {}, prompt, isTTY: true }
    );
    expect(client.post).toHaveBeenCalled();
  });

  it('does NOT accept "y" or "yes" as confirmation', async () => {
    const client = makeClient(emailResponse);
    const prompt = vi.fn(async () => 'yes');
    await runRevokeCommand(
      'alice@example.com',
      {},
      { createClient: () => client, stdout: () => {}, stderr: () => {}, prompt, isTTY: true }
    );
    expect(client.post).not.toHaveBeenCalled();
  });

  // #948: with --json, a declined confirmation must not pollute stdout.
  it('with --json, a declined confirmation routes "Aborted" to stderr', async () => {
    const client = makeClient(emailResponse);
    const outs: string[] = [];
    const errs: string[] = [];
    const prompt = vi.fn(async () => 'n');
    await runRevokeCommand(
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

  it('trims whitespace from the confirmation answer', async () => {
    const client = makeClient(emailResponse);
    const prompt = vi.fn(async () => `  ${CONFIRM_WORD}\n`);
    await runRevokeCommand(
      'alice@example.com',
      {},
      { createClient: () => client, stdout: () => {}, stderr: () => {}, prompt, isTTY: true }
    );
    expect(client.post).toHaveBeenCalled();
  });

  it('builds a token-specific confirmation banner', async () => {
    const errs: string[] = [];
    const prompt = vi.fn(async () => CONFIRM_WORD);
    await runRevokeCommand(
      undefined,
      { token: 'raw-token-abc' },
      {
        createClient: () => makeClient(tokenResponse),
        stdout: () => {},
        stderr: (s) => errs.push(s),
        prompt,
        isTTY: true,
      }
    );
    expect(errs.join('')).toContain('the supplied token');
  });

  it('throws AdminError (exitCode 4) when non-TTY and --yes is not set', async () => {
    const client = makeClient(emailResponse);
    await expect(
      runRevokeCommand(
        'alice@example.com',
        {},
        { createClient: () => client, stdout: () => {}, isTTY: false }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 4 });
  });

  it('rejects missing email and token (exitCode 64)', async () => {
    await expect(
      runRevokeCommand(undefined, { yes: true }, { createClient: () => makeClient(emailResponse) })
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('rejects combining email with --token (exitCode 64)', async () => {
    await expect(
      runRevokeCommand(
        'alice@example.com',
        { token: 'raw-token-abc', yes: true },
        { createClient: () => makeClient(emailResponse) }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('emits pretty-printed JSON when --json is set', async () => {
    const writes: string[] = [];
    await runRevokeCommand(
      'alice@example.com',
      { yes: true, json: true },
      {
        createClient: () => makeClient(emailResponse),
        stdout: (s) => writes.push(s),
      }
    );
    expect(writes.join('')).toBe(JSON.stringify(emailResponse, null, 2) + '\n');
  });

  it('renders refresh-session and account-suspension counters when present (SEC-007 / #1672)', async () => {
    const writes: string[] = [];
    const client = makeClient({
      revoked: 2,
      refreshSessionsRevoked: 3,
      accountSuspended: true,
    } satisfies RevokeResponse);
    await runRevokeCommand(
      'alice@example.com',
      { yes: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('Revoked 2 token(s)');
    expect(out).toContain('refresh sessions revoked: 3');
    expect(out).toContain('account suspended');
  });

  it('omits the account-suspended line when the server reports false (grant-level revoke)', async () => {
    const writes: string[] = [];
    const client = makeClient({
      revoked: 1,
      refreshSessionsRevoked: 0,
    } satisfies RevokeResponse);
    await runRevokeCommand(
      undefined,
      { token: 'raw-token-abc', yes: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('Revoked 1 token(s)');
    expect(out).toContain('refresh sessions revoked: 0');
    expect(out).not.toContain('account suspended');
  });

  it('propagates AdminError on 400 from client', async () => {
    const post = vi.fn(async () => {
      throw Object.assign(new Error('Either email or token must be provided'), {
        name: 'AdminError',
        exitCode: 1,
        status: 400,
      });
    });
    const client = { post } as unknown as AdminWriter;
    await expect(
      runRevokeCommand(
        'nobody@example.com',
        { yes: true },
        { createClient: () => client, stdout: () => {} }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 1, status: 400 });
  });

  it('throws MissingConfigError (exitCode 5) when no key is available', async () => {
    delete process.env[ENV_KEY];
    await expect(
      runRevokeCommand(
        'alice@example.com',
        { yes: true },
        { createClient: () => makeClient(emailResponse) }
      )
    ).rejects.toMatchObject({ exitCode: 5, name: 'MissingConfigError' });
  });
});
