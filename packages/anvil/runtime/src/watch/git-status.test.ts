import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const toFwd = (p: string): string => p.replace(/\\/g, '/');

// Track execFile calls for argument safety verification
const execFileCalls: Array<{ cmd: string; args: string[] }> = [];
let execFileStdout = '';
let execFileError: Error | null = null;

vi.mock('node:child_process', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:child_process')>();

  const execFile = Object.assign(
    vi.fn((...fnArgs: unknown[]) => {
      const cmd = fnArgs[0] as string;
      const args = fnArgs[1] as string[];
      const cb =
        typeof fnArgs[fnArgs.length - 1] === 'function'
          ? (fnArgs[fnArgs.length - 1] as (...cbArgs: unknown[]) => void)
          : null;

      execFileCalls.push({ cmd, args });

      if (cb) {
        if (execFileError) {
          cb(execFileError, '', '');
        } else {
          cb(null, execFileStdout, '');
        }
      }
    }),
    {
      // Custom promisify returns { stdout, stderr } like real execFile
      [Symbol.for('nodejs.util.promisify.custom')]: vi.fn((...fnArgs: unknown[]) => {
        const cmd = fnArgs[0] as string;
        const args = fnArgs[1] as string[];

        execFileCalls.push({ cmd, args });

        if (execFileError) {
          return Promise.reject(execFileError);
        }
        return Promise.resolve({ stdout: execFileStdout, stderr: '' });
      }),
    }
  );

  return { ...actual, default: { ...actual, execFile }, execFile };
});

import { GitStatusChecker, getChangedFiles } from './git-status.js';

describe('GitStatusChecker — command safety (CRB-014)', () => {
  beforeEach(() => {
    execFileCalls.length = 0;
    execFileStdout = '';
    execFileError = null;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('argument escaping with special characters', () => {
    it('should pass file paths with spaces as a single argument', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/path with spaces/file.ts');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
      const pathArg = call!.args.find((a) => a.includes('path with spaces'));
      expect(pathArg).toBeDefined();
      expect(toFwd(pathArg!)).toBe('path with spaces/file.ts');
      // Verify it's a single element, not split by spaces
      expect(call!.args.filter((a) => a.includes('path with spaces'))).toHaveLength(1);
    });

    it('should pass file paths with semicolons as a single argument', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/file;rm -rf /.ts');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
      // The dangerous string stays as one argument — no shell interpretation
      expect(call!.args).toContain('file;rm -rf /.ts');
    });

    it('should pass file paths with backticks as a single argument', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/file`whoami`.ts');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
      expect(call!.args).toContain('file`whoami`.ts');
    });

    it('should pass file paths with $() as a single argument', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/$(cat /etc/passwd).ts');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
      expect(call!.args).toContain('$(cat /etc/passwd).ts');
    });

    it('should pass file paths with quotes as a single argument', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/file"with\'quotes.ts');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
      expect(call!.args).toContain('file"with\'quotes.ts');
    });

    it('should use -- separator to prevent option injection', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/--exec=malicious');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
      // The -- separator prevents --exec from being interpreted as a git option
      const dashDashIndex = call!.args.indexOf('--');
      const pathIndex = call!.args.indexOf('--exec=malicious');
      expect(dashDashIndex).toBeLessThan(pathIndex);
    });
  });

  describe('path traversal prevention', () => {
    it('should pass ../ traversal paths without shell interpretation', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/../../../etc/passwd');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
      // execFile ensures the path is a single arg, not shell-expanded
      const pathArg = call!.args.find((a) => a.includes('..'));
      expect(pathArg).toBeDefined();
    });

    it('should pass absolute paths outside workspace as a single argument', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/etc/passwd');

      const call = execFileCalls.find((c) => c.args.includes('--porcelain'));
      expect(call).toBeDefined();
    });
  });

  describe('since parameter injection prevention', () => {
    it('should pass since ref with semicolons as a single argument', async () => {
      execFileStdout = '';

      await getChangedFiles('/workspace', { since: 'main; rm -rf /' });

      const call = execFileCalls.find((c) => c.args.includes('--name-only'));
      expect(call).toBeDefined();
      // The malicious string stays as one argument
      expect(call!.args).toContain('main; rm -rf /');
    });

    it('should pass since ref with backticks as a single argument', async () => {
      execFileStdout = '';

      await getChangedFiles('/workspace', { since: '`whoami`' });

      const call = execFileCalls.find((c) => c.args.includes('--name-only'));
      expect(call).toBeDefined();
      expect(call!.args).toContain('`whoami`');
    });

    it('should pass since ref with $() as a single argument', async () => {
      execFileStdout = '';

      await getChangedFiles('/workspace', { since: '$(cat /etc/passwd)' });

      const call = execFileCalls.find((c) => c.args.includes('--name-only'));
      expect(call).toBeDefined();
      expect(call!.args).toContain('$(cat /etc/passwd)');
    });
  });

  describe('uses execFile (not exec) for shell safety', () => {
    it('should call git as a direct executable, not through shell', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileStdout = '';

      await checker.getFileStatus('/workspace/test.ts');

      expect(execFileCalls.length).toBeGreaterThan(0);
      // execFile receives the command as first arg, arguments as array
      expect(execFileCalls[0].cmd).toBe('git');
      expect(Array.isArray(execFileCalls[0].args)).toBe(true);
    });

    it('should call git with argument array for getChangedFiles', async () => {
      execFileStdout = '';

      await getChangedFiles('/workspace');

      expect(execFileCalls.length).toBeGreaterThan(0);
      expect(execFileCalls[0].cmd).toBe('git');
      expect(Array.isArray(execFileCalls[0].args)).toBe(true);
    });
  });

  describe('parseStatusLine correctness', () => {
    // Access private method via cast
    type Internals = { parseStatusLine: (line: string, defaultPath: string) => unknown };

    it('should parse modified-unstaged status', () => {
      const checker = new GitStatusChecker('/workspace');
      const result = (checker as unknown as Internals).parseStatusLine(' M src/file.ts', '');

      expect(result).toMatchObject({
        path: 'src/file.ts',
        isTracked: true,
        isStaged: false,
        isUnstaged: true,
        isUntracked: false,
        statusCode: ' M',
      });
    });

    it('should parse staged status', () => {
      const checker = new GitStatusChecker('/workspace');
      const result = (checker as unknown as Internals).parseStatusLine('M  src/file.ts', '');

      expect(result).toMatchObject({
        isStaged: true,
        isUnstaged: false,
        statusCode: 'M ',
      });
    });

    it('should parse untracked status', () => {
      const checker = new GitStatusChecker('/workspace');
      const result = (checker as unknown as Internals).parseStatusLine('?? new-file.ts', '');

      expect(result).toMatchObject({
        path: 'new-file.ts',
        isTracked: false,
        isUntracked: true,
        statusCode: '??',
      });
    });

    it('should handle paths with special characters in status output', () => {
      const checker = new GitStatusChecker('/workspace');
      const result = (checker as unknown as Internals).parseStatusLine(' M file;injection.ts', '');

      expect(result).toMatchObject({
        path: 'file;injection.ts',
        isUnstaged: true,
      });
    });

    it('should handle empty/short lines gracefully', () => {
      const checker = new GitStatusChecker('/workspace');

      const empty = (checker as unknown as Internals).parseStatusLine('', 'default.ts');
      expect(empty).toMatchObject({ path: 'default.ts', isTracked: true, isUnstaged: false });

      const short = (checker as unknown as Internals).parseStatusLine('M', 'default.ts');
      expect(short).toMatchObject({ path: 'default.ts', isTracked: true });
    });
  });

  describe('error handling', () => {
    it('should treat git errors as untracked (not crash)', async () => {
      const checker = new GitStatusChecker('/workspace');
      execFileError = new Error('not a git repository');

      const status = await checker.getFileStatus('/workspace/test.ts');

      expect(status.isUntracked).toBe(true);
      expect(status.isTracked).toBe(false);
    });

    it('should return empty array on getChangedFiles error', async () => {
      execFileError = new Error('not a git repository');

      const files = await getChangedFiles('/workspace');

      expect(files).toEqual([]);
    });
  });
});
