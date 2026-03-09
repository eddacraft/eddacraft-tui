import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const { mockExecFile, mockExecFileSync, mockPromisify, mockExecFileAsync } = vi.hoisted(() => {
  const mockExecFileAsync = vi.fn();
  return {
    mockExecFile: vi.fn(),
    mockExecFileSync: vi.fn(),
    mockPromisify: vi.fn(() => mockExecFileAsync),
    mockExecFileAsync,
  };
});

vi.mock('node:child_process', () => ({
  default: { execFile: mockExecFile, execFileSync: mockExecFileSync },
  execFile: mockExecFile,
  execFileSync: mockExecFileSync,
}));

vi.mock('node:util', () => ({
  default: { promisify: mockPromisify },
  promisify: mockPromisify,
}));

describe('gitExec', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('trims trailing whitespace from stdout and fully trims stderr', async () => {
    mockExecFileAsync.mockResolvedValue({
      stdout: ' M file.ts\n',
      stderr: '  warn\n',
    });

    const { gitExec } = await import('./git-operations.js');
    const result = await gitExec(['status', '--porcelain'], { cwd: '/repo' });
    // Leading whitespace preserved (significant for porcelain format)
    expect(result.stdout).toBe(' M file.ts');
    expect(result.stderr).toBe('warn');
  });

  it('passes cwd and timeout to execFileAsync', async () => {
    mockExecFileAsync.mockResolvedValue({ stdout: '', stderr: '' });

    const { gitExec } = await import('./git-operations.js');
    await gitExec(['rev-parse', 'HEAD'], { cwd: '/my/repo', timeout: 5_000 });

    expect(mockExecFileAsync).toHaveBeenCalledWith(
      'git',
      ['rev-parse', 'HEAD'],
      expect.objectContaining({ cwd: '/my/repo', timeout: 5_000, encoding: 'utf8' })
    );
  });

  it('uses default 30s timeout', async () => {
    mockExecFileAsync.mockResolvedValue({ stdout: '', stderr: '' });

    const { gitExec } = await import('./git-operations.js');
    await gitExec(['log']);

    expect(mockExecFileAsync).toHaveBeenCalledWith(
      'git',
      ['log'],
      expect.objectContaining({ timeout: 30_000 })
    );
  });

  it('throws GitOperationError on failure', async () => {
    const error = Object.assign(new Error('git failed'), {
      code: 128,
      stderr: 'fatal: not a git repository',
    });

    mockExecFileAsync.mockRejectedValue(error);

    const { gitExec, GitOperationError } = await import('./git-operations.js');
    await expect(gitExec(['status'])).rejects.toThrow(GitOperationError);

    try {
      await gitExec(['status']);
    } catch (err) {
      expect(err).toBeInstanceOf(GitOperationError);
      const gitErr = err as InstanceType<typeof GitOperationError>;
      expect(gitErr.command).toBe('status');
      expect(gitErr.args).toEqual(['status']);
      expect(gitErr.exitCode).toBe(128);
      expect(gitErr.stderr).toBe('fatal: not a git repository');
    }
  });
});

describe('gitExecSync', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('trims trailing whitespace from stdout on success', async () => {
    mockExecFileSync.mockReturnValue('main\n');

    const { gitExecSync } = await import('./git-operations.js');
    const result = gitExecSync(['rev-parse', '--abbrev-ref', 'HEAD'], { cwd: '/repo' });
    expect(result).toBe('main');
  });

  it('passes cwd, timeout, and stdio to execFileSync', async () => {
    mockExecFileSync.mockReturnValue('');

    const { gitExecSync } = await import('./git-operations.js');
    gitExecSync(['status'], { cwd: '/repo', timeout: 10_000 });

    expect(mockExecFileSync).toHaveBeenCalledWith(
      'git',
      ['status'],
      expect.objectContaining({
        cwd: '/repo',
        timeout: 10_000,
        encoding: 'utf8',
        stdio: ['pipe', 'pipe', 'pipe'],
      })
    );
  });

  it('throws GitOperationError on failure', async () => {
    const error = Object.assign(new Error('git failed'), {
      status: 1,
      stderr: 'error: pathspec not found',
    });

    mockExecFileSync.mockImplementation(() => {
      throw error;
    });

    const { gitExecSync, GitOperationError } = await import('./git-operations.js');
    expect(() => gitExecSync(['checkout', 'nonexistent'])).toThrow(GitOperationError);

    try {
      gitExecSync(['checkout', 'nonexistent']);
    } catch (err) {
      expect(err).toBeInstanceOf(GitOperationError);
      const gitErr = err as InstanceType<typeof GitOperationError>;
      expect(gitErr.command).toBe('checkout');
      expect(gitErr.exitCode).toBe(1);
      expect(gitErr.stderr).toBe('error: pathspec not found');
    }
  });
});

describe('GitOperationError', () => {
  it('includes command and stderr in message', async () => {
    const { GitOperationError } = await import('./git-operations.js');
    const err = new GitOperationError('push', ['push', 'origin'], 1, 'rejected', new Error());
    expect(err.message).toContain('git push failed');
    expect(err.message).toContain('exit 1');
    expect(err.message).toContain('rejected');
    expect(err.name).toBe('GitOperationError');
  });

  it('handles null exit code', async () => {
    const { GitOperationError } = await import('./git-operations.js');
    const err = new GitOperationError('fetch', ['fetch'], null, 'timeout');
    expect(err.message).toContain('exit ?');
    expect(err.exitCode).toBeNull();
  });

  it('truncates long stderr in message', async () => {
    const { GitOperationError } = await import('./git-operations.js');
    const longStderr = 'x'.repeat(1000);
    const err = new GitOperationError('log', ['log'], 1, longStderr);
    expect(err.message.length).toBeLessThan(600);
    expect(err.stderr).toBe(longStderr);
  });

  it('stores string spawn codes separately from numeric exitCode', async () => {
    const { GitOperationError } = await import('./git-operations.js');
    const err = new GitOperationError('push', ['push'], null, '', undefined, 'ENOENT');
    expect(err.exitCode).toBeNull();
    expect(err.spawnCode).toBe('ENOENT');
  });
});

describe('gitExec error handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('sets spawnCode for string error.code (e.g. ENOENT)', async () => {
    const error = Object.assign(new Error('spawn git ENOENT'), {
      code: 'ENOENT',
      stderr: '',
    });
    mockExecFileAsync.mockRejectedValue(error);

    const { gitExec, GitOperationError } = await import('./git-operations.js');
    try {
      await gitExec(['status']);
    } catch (err) {
      expect(err).toBeInstanceOf(GitOperationError);
      const gitErr = err as InstanceType<typeof GitOperationError>;
      expect(gitErr.exitCode).toBeNull();
      expect(gitErr.spawnCode).toBe('ENOENT');
    }
  });
});

describe('gitRemoteUrl', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns undefined for "No such remote" errors', async () => {
    const error = Object.assign(new Error('git failed'), {
      code: 2,
      stderr: "fatal: No such remote 'upstream'",
    });
    mockExecFileAsync.mockRejectedValue(error);

    const { gitRemoteUrl } = await import('./git-operations.js');
    const result = await gitRemoteUrl('/repo', 'upstream');
    expect(result).toBeUndefined();
  });

  it('propagates non-remote errors', async () => {
    const error = Object.assign(new Error('spawn git ENOENT'), {
      code: 'ENOENT',
      stderr: '',
    });
    mockExecFileAsync.mockRejectedValue(error);

    const { gitRemoteUrl, GitOperationError } = await import('./git-operations.js');
    await expect(gitRemoteUrl('/repo')).rejects.toThrow(GitOperationError);
  });
});
