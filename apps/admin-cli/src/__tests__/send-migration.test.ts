import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Command, Option } from 'commander';
import {
  runSendMigrationCommand,
  type DryRunResponse,
  type SendResponse,
  MIGRATION_SOURCES,
} from '../commands/send-migration.js';
import { parseBoundedInt } from '../parsers.js';
import { AdminError, type AdminWriter } from '../client.js';

const ENV_KEY = 'ANVIL_ADMIN_KEY';

type Result = unknown | (() => unknown);

function makeClient(...results: Result[]): AdminWriter & { post: ReturnType<typeof vi.fn> } {
  const queue = [...results];
  const post = vi.fn(async () => {
    if (queue.length === 0) throw new Error('unexpected extra call to post()');
    const next = queue.shift();
    if (typeof next === 'function') return (next as () => unknown)();
    return next;
  }) as unknown as AdminWriter['post'] & ReturnType<typeof vi.fn>;
  return { post } as AdminWriter & { post: ReturnType<typeof vi.fn> };
}

const previewResponse: DryRunResponse = {
  dryRun: true,
  source: 'import',
  count: 2,
  recipients: [
    { email: 'alice@example.com', name: 'Alice' },
    { email: 'bob@example.com', name: null },
  ],
  previewToken: 'snap-token-abc',
  expiresAt: '2026-04-17T09:10:00Z',
};

const emptyPreviewResponse: DryRunResponse = {
  dryRun: true,
  source: 'import',
  count: 0,
  recipients: [],
  previewToken: 'snap-token-empty',
  expiresAt: '2026-04-17T09:10:00Z',
};

const sendResponse: SendResponse = {
  source: 'import',
  total: 2,
  sent: 2,
  failed: 0,
  results: [
    { email: 'alice@example.com', sent: true },
    { email: 'bob@example.com', sent: true },
  ],
};

const partialFailureResponse: SendResponse = {
  source: 'import',
  total: 2,
  sent: 1,
  failed: 1,
  results: [
    { email: 'alice@example.com', sent: true },
    { email: 'bob@example.com', sent: false, error: 'bounced' },
  ],
};

describe('runSendMigrationCommand', () => {
  const originalKey = process.env[ENV_KEY];

  beforeEach(() => {
    process.env[ENV_KEY] = 'test-key';
  });

  afterEach(() => {
    if (originalKey === undefined) delete process.env[ENV_KEY];
    else process.env[ENV_KEY] = originalKey;
  });

  it('defaults to a dry-run call with source=import, limit=20 when no flags are given', async () => {
    const client = makeClient(previewResponse);
    await runSendMigrationCommand({}, { createClient: () => client, stdout: () => {} });
    expect(client.post).toHaveBeenCalledTimes(1);
    expect(client.post).toHaveBeenCalledWith(
      '/admin/send-migration',
      {
        source: 'import',
        dryRun: true,
        limit: 20,
      },
      expect.anything()
    );
  });

  it('renders the dry-run recipient table with count and source', async () => {
    const writes: string[] = [];
    await runSendMigrationCommand(
      {},
      { createClient: () => makeClient(previewResponse), stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('Dry run: 2 recipient(s) from source "import"');
    expect(out).toContain('alice@example.com');
    expect(out).toContain('bob@example.com');
    expect(out).toContain('Alice');
  });

  it('prints "No recipients match" when the dry-run returns zero', async () => {
    const writes: string[] = [];
    await runSendMigrationCommand(
      {},
      { createClient: () => makeClient(emptyPreviewResponse), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toContain('No recipients match');
  });

  it('emits pretty-printed JSON for dry-run with --json', async () => {
    const writes: string[] = [];
    await runSendMigrationCommand(
      { json: true },
      { createClient: () => makeClient(previewResponse), stdout: (s) => writes.push(s) }
    );
    expect(writes.join('')).toBe(JSON.stringify(previewResponse, null, 2) + '\n');
  });

  it('forwards --source and --limit to the server', async () => {
    const client = makeClient(previewResponse);
    await runSendMigrationCommand(
      { source: 'website', limit: 5 },
      { createClient: () => client, stdout: () => {} }
    );
    expect(client.post).toHaveBeenCalledWith(
      '/admin/send-migration',
      {
        source: 'website',
        dryRun: true,
        limit: 5,
      },
      expect.anything()
    );
  });

  it('rejects --limit out of range (exitCode 64)', async () => {
    await expect(
      runSendMigrationCommand({ limit: 0 }, { createClient: () => makeClient(previewResponse) })
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
    await expect(
      runSendMigrationCommand({ limit: 101 }, { createClient: () => makeClient(previewResponse) })
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('with --yes and --no-dry-run, fetches a preview token then sends with it', async () => {
    const client = makeClient(previewResponse, sendResponse);
    const writes: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, yes: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    expect(client.post).toHaveBeenCalledTimes(2);
    expect(client.post).toHaveBeenNthCalledWith(
      1,
      '/admin/send-migration',
      {
        source: 'import',
        dryRun: true,
        limit: 20,
      },
      expect.anything()
    );
    expect(client.post).toHaveBeenNthCalledWith(
      2,
      '/admin/send-migration',
      {
        source: 'import',
        dryRun: false,
        limit: 20,
        previewToken: 'snap-token-abc',
      },
      expect.anything()
    );
    const out = writes.join('');
    expect(out).toContain('Sent 2/2 (failed: 0)');
    expect(out).toContain('alice@example.com');
    expect(out).toContain('bob@example.com');
  });

  it('bails (nothing to send) when --yes preview returns zero recipients', async () => {
    const client = makeClient(emptyPreviewResponse);
    const outs: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, yes: true },
      { createClient: () => client, stdout: (s) => outs.push(s) }
    );
    expect(client.post).toHaveBeenCalledTimes(1);
    expect(outs.join('')).toContain('Nothing to send');
  });

  it('throws AdminError exitCode 1 when any recipient failed to send', async () => {
    await expect(
      runSendMigrationCommand(
        { dryRun: false, yes: true },
        {
          createClient: () => makeClient(previewResponse, partialFailureResponse),
          stdout: () => {},
        }
      )
    ).rejects.toMatchObject({
      name: 'AdminError',
      exitCode: 1,
      message: expect.stringContaining('1 of 2 recipient(s) failed'),
    });
  });

  it('interactive send: previews, confirms, then sends on "y"', async () => {
    const client = makeClient(previewResponse, sendResponse);
    const prompt = vi.fn(async () => 'y');
    const outs: string[] = [];
    const errs: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false },
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: (s) => errs.push(s),
        prompt,
        isTTY: true,
      }
    );
    expect(client.post).toHaveBeenNthCalledWith(
      1,
      '/admin/send-migration',
      {
        source: 'import',
        dryRun: true,
        limit: 20,
      },
      expect.anything()
    );
    expect(client.post).toHaveBeenNthCalledWith(
      2,
      '/admin/send-migration',
      {
        source: 'import',
        dryRun: false,
        limit: 20,
        previewToken: 'snap-token-abc',
      },
      expect.anything()
    );
    expect(errs.join('')).toContain('About to send migration email to 2 recipient(s)');
    expect(outs.join('')).toContain('Sent 2/2');
  });

  it('interactive send: aborts on non-"y" answer without calling the real send', async () => {
    const client = makeClient(previewResponse);
    const prompt = vi.fn(async () => 'n');
    const outs: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false },
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: () => {},
        prompt,
        isTTY: true,
      }
    );
    expect(client.post).toHaveBeenCalledTimes(1);
    expect(outs.join('')).toContain('Aborted.');
  });

  it('interactive send: exits early when preview count is 0', async () => {
    const client = makeClient(emptyPreviewResponse);
    const outs: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false },
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: () => {},
        prompt: async () => {
          throw new Error('prompt should not run when count is 0');
        },
        isTTY: true,
      }
    );
    expect(client.post).toHaveBeenCalledTimes(1);
    expect(outs.join('')).toContain('Nothing to send');
  });

  it('non-TTY without --yes refuses to send (exit 4)', async () => {
    await expect(
      runSendMigrationCommand(
        { dryRun: false },
        {
          createClient: () => makeClient(previewResponse),
          stdout: () => {},
          stderr: () => {},
          isTTY: false,
        }
      )
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 4 });
  });

  it('emits send JSON with --json (preview fetched silently first)', async () => {
    const client = makeClient(previewResponse, sendResponse);
    const writes: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, yes: true, json: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    expect(client.post).toHaveBeenCalledTimes(2);
    expect(writes.join('')).toBe(JSON.stringify(sendResponse, null, 2) + '\n');
  });

  // #948: --json --no-dry-run without --yes previously wrote the ASCII
  // recipient table to stderr and the send JSON to stdout. `2>&1 | jq`
  // then choked on the table preamble. The contract is now: stdout is
  // pure JSON; stderr carries a one-line preview hint and any abort/empty
  // notice.
  it('interactive --json send: stderr one-liner, stdout is pure send JSON', async () => {
    const client = makeClient(previewResponse, sendResponse);
    const outs: string[] = [];
    const errs: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, json: true },
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: (s) => errs.push(s),
        prompt: async () => 'y',
        isTTY: true,
      }
    );
    expect(outs.join('')).toBe(JSON.stringify(sendResponse, null, 2) + '\n');
    const err = errs.join('');
    expect(err).toContain('preview: 2 recipient(s)');
    expect(err).toContain('pass --yes to skip this prompt');
    expect(err).not.toContain('alice@example.com'); // no ASCII table in --json
  });

  it('interactive --json send: abort routes to stderr, stdout stays empty', async () => {
    const client = makeClient(previewResponse);
    const outs: string[] = [];
    const errs: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, json: true },
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: (s) => errs.push(s),
        prompt: async () => 'n',
        isTTY: true,
      }
    );
    expect(outs.join('')).toBe(''); // no stdout pollution
    expect(errs.join('')).toContain('Aborted.');
  });

  it('--json --no-dry-run with zero recipients: "Nothing to send" goes to stderr', async () => {
    const client = makeClient(emptyPreviewResponse);
    const outs: string[] = [];
    const errs: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, yes: true, json: true },
      {
        createClient: () => client,
        stdout: (s) => outs.push(s),
        stderr: (s) => errs.push(s),
      }
    );
    expect(outs.join('')).toBe(''); // no stdout pollution
    expect(errs.join('')).toContain('Nothing to send');
  });

  it('dry-run output surfaces the preview token and expiry hint', async () => {
    const writes: string[] = [];
    await runSendMigrationCommand(
      {},
      { createClient: () => makeClient(previewResponse), stdout: (s) => writes.push(s) }
    );
    const out = writes.join('');
    expect(out).toContain('Preview token: snap-token-abc');
    expect(out).toContain('expires 2026-04-17T09:10:00Z');
    expect(out).toMatch(/Re-run without --dry-run within 10 minutes/i);
  });

  it('rewrites 409 cohort_drift into a message listing added/removed', async () => {
    const driftBody = JSON.stringify({
      code: 'cohort_drift',
      error: 'recipient set changed since preview; re-run with --dry-run',
      added: ['carol@example.com'],
      removed: ['bob@example.com'],
    });
    const client = makeClient(previewResponse, () => {
      throw new AdminError(
        'recipient set changed since preview; re-run with --dry-run',
        1,
        409,
        driftBody
      );
    });
    await expect(
      runSendMigrationCommand(
        { dryRun: false, yes: true },
        { createClient: () => client, stdout: () => {} }
      )
    ).rejects.toMatchObject({
      name: 'AdminError',
      exitCode: 1,
      status: 409,
      message: expect.stringMatching(/recipient set changed/),
    });
  });

  it('cohort_drift message includes the diff details', async () => {
    const driftBody = JSON.stringify({
      code: 'cohort_drift',
      error: 'x',
      added: ['carol@example.com', 'dan@example.com'],
      removed: ['bob@example.com'],
    });
    const client = makeClient(previewResponse, () => {
      throw new AdminError('x', 1, 409, driftBody);
    });
    let caught: unknown;
    try {
      await runSendMigrationCommand(
        { dryRun: false, yes: true },
        { createClient: () => client, stdout: () => {} }
      );
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(AdminError);
    expect((caught as AdminError).message).toContain('added:   carol@example.com, dan@example.com');
    expect((caught as AdminError).message).toContain('removed: bob@example.com');
  });

  it('rewrites 410 preview_token_expired with TTL recovery guidance', async () => {
    const body = JSON.stringify({ code: 'preview_token_expired', error: 'expired' });
    const client = makeClient(previewResponse, () => {
      throw new AdminError('expired', 1, 410, body);
    });
    await expect(
      runSendMigrationCommand(
        { dryRun: false, yes: true },
        { createClient: () => client, stdout: () => {} }
      )
    ).rejects.toMatchObject({
      name: 'AdminError',
      status: 410,
      message: expect.stringMatching(/10-minute TTL/),
    });
  });

  it('rewrites 410 preview_token_consumed with verify-before-retry guidance', async () => {
    const body = JSON.stringify({ code: 'preview_token_consumed', error: 'consumed' });
    const client = makeClient(previewResponse, () => {
      throw new AdminError('consumed', 1, 410, body);
    });
    await expect(
      runSendMigrationCommand(
        { dryRun: false, yes: true },
        { createClient: () => client, stdout: () => {} }
      )
    ).rejects.toMatchObject({
      name: 'AdminError',
      status: 410,
      message: expect.stringMatching(/already used/),
    });
  });

  it('rewrites 410 preview_token_missing with merged actor/regenerate guidance', async () => {
    // Server intentionally merges the wrong-actor case into
    // `preview_token_missing` to avoid confirming token existence to
    // non-owners; the CLI message must surface both recovery paths.
    const body = JSON.stringify({ code: 'preview_token_missing', error: 'missing' });
    const client = makeClient(previewResponse, () => {
      throw new AdminError('missing', 1, 410, body);
    });
    await expect(
      runSendMigrationCommand(
        { dryRun: false, yes: true },
        { createClient: () => client, stdout: () => {} }
      )
    ).rejects.toMatchObject({
      name: 'AdminError',
      status: 410,
      message: expect.stringMatching(/--actor|ANVIL_ADMIN_ACTOR/),
    });
  });

  it('preserves non-coded errors unchanged', async () => {
    const client = makeClient(previewResponse, () => {
      throw new AdminError('server error 500: boom', 2, 500, 'boom');
    });
    await expect(
      runSendMigrationCommand(
        { dryRun: false, yes: true },
        { createClient: () => client, stdout: () => {} }
      )
    ).rejects.toMatchObject({
      name: 'AdminError',
      exitCode: 2,
      message: 'server error 500: boom',
    });
  });

  it('throws MissingConfigError (exitCode 5) when no key is available', async () => {
    delete process.env[ENV_KEY];
    await expect(
      runSendMigrationCommand({}, { createClient: () => makeClient(previewResponse) })
    ).rejects.toMatchObject({ exitCode: 5, name: 'MissingConfigError' });
  });
});

describe('send-migration commander wiring', () => {
  function buildProgram(): Command {
    const prog = new Command();
    prog.exitOverride();
    prog
      .command('send-migration')
      .addOption(
        new Option('--source <source>', 'filter by source')
          .choices([...MIGRATION_SOURCES])
          .default('import')
      )
      .addOption(
        new Option('--limit <n>', 'max recipients (1-100)')
          .default(20)
          .argParser(parseBoundedInt('--limit', 1, 100))
      )
      .option('--no-dry-run', 'actually send (default is to preview only)')
      .option('-y, --yes', 'skip confirmation prompt')
      .option('--json', 'emit raw JSON')
      .action(() => {});
    return prog;
  }

  function parse(argv: string[]): Record<string, unknown> {
    const prog = buildProgram();
    prog.parse(['node', 'anvil-admin', 'send-migration', ...argv]);
    const sub = prog.commands.find((c) => c.name() === 'send-migration')!;
    return sub.opts();
  }

  it('dry-run defaults to true when no flag is passed', () => {
    expect(parse([])).toMatchObject({ dryRun: true, source: 'import', limit: 20 });
  });

  it('--no-dry-run flips dryRun to false', () => {
    expect(parse(['--no-dry-run'])).toMatchObject({ dryRun: false });
  });

  it('rejects --limit out of range at parse time', () => {
    expect(() => parse(['--limit', '0'])).toThrow(/between 1 and 100/);
    expect(() => parse(['--limit', '101'])).toThrow(/between 1 and 100/);
  });

  it('rejects an unknown --source choice', () => {
    expect(() => parse(['--source', 'bogus'])).toThrow(/Allowed choices/);
  });
});
