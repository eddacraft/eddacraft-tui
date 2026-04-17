import { describe, it, expect, vi, afterEach } from 'vitest';
import { handleError, run } from '../index.js';
import { AdminError } from '../client.js';
import { MissingConfigError } from '../config.js';

afterEach(() => {
  vi.restoreAllMocks();
});

class ExitSignal extends Error {
  constructor(readonly code: number) {
    super(`process.exit(${code})`);
  }
}

function harness() {
  const writes: string[] = [];
  const exits: number[] = [];
  const stderr = {
    write: (chunk: string) => {
      writes.push(chunk);
      return true;
    },
  };
  // Model process.exit's `never` return by throwing — the production call
  // terminates the process and never resumes; the test stub short-circuits
  // the same way so control flow stays honest.
  const exit: (code: number) => never = (code: number) => {
    exits.push(code);
    throw new ExitSignal(code);
  };
  return { writes, exits, stderr, exit };
}

async function catchExit<T>(fn: () => T | Promise<T>): Promise<void> {
  try {
    await fn();
  } catch (err) {
    if (!(err instanceof ExitSignal)) throw err;
  }
}

describe('handleError', () => {
  it('maps AdminError to its exitCode and writes the formatted message', async () => {
    const { writes, exits, stderr, exit } = harness();
    await catchExit(() => handleError(new AdminError('boom', 1), stderr, exit));
    expect(exits).toEqual([1]);
    expect(writes.join('')).toBe('error: boom\n');
  });

  it('maps MissingConfigError to exit 5', async () => {
    const { writes, exits, stderr, exit } = harness();
    await catchExit(() =>
      handleError(new MissingConfigError('admin API URL missing'), stderr, exit)
    );
    expect(exits).toEqual([5]);
    expect(writes.join('')).toBe('error: admin API URL missing\n');
  });

  it('maps unknown Error to exit 2', async () => {
    const { writes, exits, stderr, exit } = harness();
    await catchExit(() => handleError(new Error('unexpected'), stderr, exit));
    expect(exits).toEqual([2]);
    expect(writes.join('')).toBe('error: unexpected\n');
  });

  it('maps non-Error thrown values to exit 2 via String()', async () => {
    const { writes, exits, stderr, exit } = harness();
    await catchExit(() => handleError('string thrown', stderr, exit));
    expect(exits).toEqual([2]);
    expect(writes.join('')).toBe('error: string thrown\n');
  });

  it('maps commander errors to their carried exitCode without writing stderr', async () => {
    const { writes, exits, stderr, exit } = harness();
    const commanderErr = Object.assign(new Error('(outputHelp)'), {
      exitCode: 0,
      code: 'commander.helpDisplayed',
    });
    await catchExit(() => handleError(commanderErr, stderr, exit));
    expect(exits).toEqual([0]);
    expect(writes.join('')).toBe('');
  });

  it('does not treat a non-numeric exitCode as a commander error', async () => {
    const { writes, exits, stderr, exit } = harness();
    const fake = Object.assign(new Error('bad'), {
      exitCode: 'not-a-number',
      code: 'commander.something',
    });
    await catchExit(() => handleError(fake, stderr, exit));
    expect(exits).toEqual([2]);
    expect(writes.join('')).toBe('error: bad\n');
  });
});

describe('run()', () => {
  it('exits 0 for --help', async () => {
    const { exits, exit } = harness();
    vi.spyOn(process.stdout, 'write').mockImplementation(() => true);
    await catchExit(() => run({ argv: ['node', 'anvil-admin', '--help'], exit }));
    expect(exits).toEqual([0]);
  });
});
