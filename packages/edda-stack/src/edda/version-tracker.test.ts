import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { VersionTracker } from './version-tracker.js';

const { mockExecFile } = vi.hoisted(() => ({ mockExecFile: vi.fn() }));
vi.mock('node:child_process', () => ({
  default: { execFile: mockExecFile },
  execFile: mockExecFile,
}));

afterEach(() => {
  vi.clearAllMocks();
  vi.restoreAllMocks();
});

interface ExecResult {
  stdout?: string;
  stderr?: string;
  error?: Error;
}

function queueExecResults(results: ExecResult[]): void {
  mockExecFile.mockReset();

  mockExecFile.mockImplementation(
    (
      _file: string,
      _args: string[],
      _options: Record<string, unknown>,
      callback: (...cbArgs: unknown[]) => void
    ) => {
      const next = results.shift();
      if (!next) {
        throw new Error('Unexpected execFile call');
      }

      if (!callback) {
        throw new Error('Expected execFile callback');
      }

      if (next.error) {
        callback(next.error, next.stdout ?? '', next.stderr ?? '');
      } else {
        callback(null, next.stdout ?? '', next.stderr ?? '');
      }

      return {} as never;
    }
  );
}

describe('VersionTracker (EDDA-008)', () => {
  it('initialises a repository when missing', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    const tracker = new VersionTracker(storagePath);

    queueExecResults([{ stdout: 'Initialised empty Git repository\n' }]);

    try {
      await tracker.init();

      expect(mockExecFile).toHaveBeenCalledTimes(1);
      expect(mockExecFile).toHaveBeenCalledWith(
        'git',
        ['init'],
        expect.objectContaining({ cwd: storagePath, encoding: 'utf8' }),
        expect.any(Function)
      );
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('tracks changes with attribution and returns commit hash', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    const tracker = new VersionTracker(storagePath);

    queueExecResults([
      { stdout: 'Initialised empty Git repository\n' },
      { stdout: '' },
      { stdout: '[main abc1234] message\n' },
      { stdout: 'abc1234def5678\n' },
    ]);

    try {
      const hash = await tracker.trackChange(
        ['index.yaml', 'memories/pattern/test.yaml'],
        'Persist memory update',
        'Memory Agent <agent@eddacraft.dev>'
      );

      expect(hash).toBe('abc1234def5678');
      expect(mockExecFile).toHaveBeenNthCalledWith(
        1,
        'git',
        ['init'],
        expect.any(Object),
        expect.any(Function)
      );
      expect(mockExecFile).toHaveBeenNthCalledWith(
        2,
        'git',
        ['add', 'index.yaml', 'memories/pattern/test.yaml'],
        expect.any(Object),
        expect.any(Function)
      );
      expect(mockExecFile).toHaveBeenNthCalledWith(
        3,
        'git',
        ['commit', '-m', 'Persist memory update', '--author', 'Memory Agent <agent@eddacraft.dev>'],
        expect.any(Object),
        expect.any(Function)
      );
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('normalises bare author names for git commit attribution', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    const tracker = new VersionTracker(storagePath);

    queueExecResults([
      { stdout: 'Initialised empty Git repository\n' },
      { stdout: '' },
      { stdout: '[main abc1234] message\n' },
      { stdout: 'abc1234def5678\n' },
    ]);

    try {
      await tracker.trackChange(['index.yaml'], 'Persist memory update', 'joshua');

      expect(mockExecFile).toHaveBeenNthCalledWith(
        3,
        'git',
        ['commit', '-m', 'Persist memory update', '--author', 'joshua <joshua@anvil.local>'],
        expect.any(Object),
        expect.any(Function)
      );
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('returns parsed history for a specific file', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    mkdirSync(join(storagePath, '.git'));
    const tracker = new VersionTracker(storagePath);

    queueExecResults([
      {
        stdout:
          'hash1\x1fFirst commit\x1fAlice\x1f2026-02-01T10:00:00.000Z\n' +
          'hash2\x1fSecond commit\x1fBob\x1f2026-02-02T10:00:00.000Z\n',
      },
    ]);

    try {
      const history = await tracker.getHistory('index.yaml', 2);

      expect(history).toEqual([
        {
          hash: 'hash1',
          message: 'First commit',
          author: 'Alice',
          timestamp: '2026-02-01T10:00:00.000Z',
        },
        {
          hash: 'hash2',
          message: 'Second commit',
          author: 'Bob',
          timestamp: '2026-02-02T10:00:00.000Z',
        },
      ]);
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('reads file contents from a specific commit version', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    const tracker = new VersionTracker(storagePath);

    queueExecResults([{ stdout: 'statement: Test\n' }]);

    try {
      const contents = await tracker.getVersion('index.yaml', 'abc123');

      expect(contents).toBe('statement: Test\n');
      expect(mockExecFile).toHaveBeenCalledWith(
        'git',
        ['show', 'abc123:index.yaml'],
        expect.objectContaining({ cwd: storagePath, encoding: 'utf8' }),
        expect.any(Function)
      );
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('reports initialisation state from .git directory presence', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    const tracker = new VersionTracker(storagePath);

    try {
      expect(await tracker.isInitialised()).toBe(false);

      mkdirSync(join(storagePath, '.git'));
      expect(await tracker.isInitialised()).toBe(true);
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('rejects parent-directory traversal when tracking changes', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    mkdirSync(join(storagePath, '.git'));
    const tracker = new VersionTracker(storagePath);

    try {
      await expect(
        tracker.trackChange(['../outside.yaml'], 'Persist memory update', 'joshua')
      ).rejects.toThrow('parent-directory traversal');
      expect(mockExecFile).not.toHaveBeenCalled();
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('rejects absolute paths when reading a specific version', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-version-'));
    const tracker = new VersionTracker(storagePath);
    const absolutePath = join(storagePath, 'index.yaml');

    try {
      await expect(tracker.getVersion(absolutePath, 'abc123')).rejects.toThrow(
        'relative to storage root'
      );
      expect(mockExecFile).not.toHaveBeenCalled();
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });
});
