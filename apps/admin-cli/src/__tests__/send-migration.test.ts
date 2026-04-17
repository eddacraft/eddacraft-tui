import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Command, Option } from 'commander';
import {
  runSendMigrationCommand,
  type AdminWriter,
  type DryRunResponse,
  type SendResponse,
  MIGRATION_SOURCES,
} from '../commands/send-migration.js';
import { parseBoundedInt } from '../parsers.js';

const ENV_KEY = 'ANVIL_ADMIN_KEY';

function makeClient(...results: unknown[]): AdminWriter & { post: ReturnType<typeof vi.fn> } {
  const queue = [...results];
  const post = vi.fn(async () => {
    if (queue.length === 0) throw new Error('unexpected extra call to post()');
    return queue.shift();
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
};

const emptyPreviewResponse: DryRunResponse = {
  dryRun: true,
  source: 'import',
  count: 0,
  recipients: [],
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
    expect(client.post).toHaveBeenCalledWith('/admin/send-migration', {
      source: 'import',
      dryRun: true,
      limit: 20,
    });
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
    expect(client.post).toHaveBeenCalledWith('/admin/send-migration', {
      source: 'website',
      dryRun: true,
      limit: 5,
    });
  });

  it('rejects --limit out of range (exitCode 64)', async () => {
    await expect(
      runSendMigrationCommand({ limit: 0 }, { createClient: () => makeClient(previewResponse) })
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
    await expect(
      runSendMigrationCommand({ limit: 101 }, { createClient: () => makeClient(previewResponse) })
    ).rejects.toMatchObject({ name: 'AdminError', exitCode: 64 });
  });

  it('with --yes and --no-dry-run, skips the preview and sends directly', async () => {
    const client = makeClient(sendResponse);
    const writes: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, yes: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    expect(client.post).toHaveBeenCalledTimes(1);
    expect(client.post).toHaveBeenCalledWith('/admin/send-migration', {
      source: 'import',
      dryRun: false,
      limit: 20,
    });
    const out = writes.join('');
    expect(out).toContain('Sent 2/2 (failed: 0)');
    expect(out).toContain('alice@example.com');
    expect(out).toContain('bob@example.com');
  });

  it('throws AdminError exitCode 1 when any recipient failed to send', async () => {
    await expect(
      runSendMigrationCommand(
        { dryRun: false, yes: true },
        { createClient: () => makeClient(partialFailureResponse), stdout: () => {} }
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
    expect(client.post).toHaveBeenNthCalledWith(1, '/admin/send-migration', {
      source: 'import',
      dryRun: true,
      limit: 20,
    });
    expect(client.post).toHaveBeenNthCalledWith(2, '/admin/send-migration', {
      source: 'import',
      dryRun: false,
      limit: 20,
    });
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

  it('emits send JSON with --json (single API call, no preview)', async () => {
    const client = makeClient(sendResponse);
    const writes: string[] = [];
    await runSendMigrationCommand(
      { dryRun: false, yes: true, json: true },
      { createClient: () => client, stdout: (s) => writes.push(s) }
    );
    expect(client.post).toHaveBeenCalledTimes(1);
    expect(writes.join('')).toBe(JSON.stringify(sendResponse, null, 2) + '\n');
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
