import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, writeFileSync } from 'node:fs';
import path, { join } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync } from 'node:child_process';
import { GitStatusChecker, getChangedFiles } from './git-status.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

/**
 * CRB-014: Tests for git command composition in watch/git-status.ts
 *
 * Verifies that argument handling, status parsing, and error paths behave
 * correctly — including paths with special characters and edge-case status
 * codes. Uses a real temporary git repo to avoid CJS mock issues with
 * vitest v4 ESM interop.
 */

let tmpDir: string;
let checker: GitStatusChecker;

function git(...args: string[]) {
  return execFileSync('git', args, { cwd: tmpDir, encoding: 'utf-8' }).trim();
}

function writeFile(name: string, content = '') {
  writeFileSync(join(tmpDir, name), content);
}

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), 'git-status-test-'));
  git('init');
  git('config', 'user.email', 'test@test.com');
  git('config', 'user.name', 'Test');
  // Create initial commit so HEAD exists
  writeFile('.gitkeep', '');
  git('add', '.');
  git('commit', '-m', 'init');
  checker = new GitStatusChecker(tmpDir);
});

afterEach(async () => {
  await safeCleanup(tmpDir);
});

describe('GitStatusChecker', () => {
  describe('isGitRepository', () => {
    it('returns true for a git repo', async () => {
      expect(await checker.isGitRepository()).toBe(true);
    });

    it('returns false for a non-repo directory', async () => {
      const nonRepo = mkdtempSync(join(tmpdir(), 'not-a-repo-'));
      const nonRepoChecker = new GitStatusChecker(nonRepo);
      expect(await nonRepoChecker.isGitRepository()).toBe(false);
      await safeCleanup(nonRepo);
    });
  });

  describe('getFileStatus — argument composition with special paths', () => {
    it('handles paths with spaces', async () => {
      writeFile('my file.ts', 'content');
      const status = await checker.getFileStatus(join(tmpDir, 'my file.ts'));

      expect(status.isUntracked).toBe(true);
      expect(status.statusCode).toBe('??');
    });

    it('handles paths with dollar signs', async () => {
      writeFile('$pecial.ts', 'content');
      const status = await checker.getFileStatus(join(tmpDir, '$pecial.ts'));

      expect(status.isUntracked).toBe(true);
    });

    it('handles paths with brackets', async () => {
      writeFile('[test].ts', 'content');
      const status = await checker.getFileStatus(join(tmpDir, '[test].ts'));

      expect(status.isUntracked).toBe(true);
    });

    it('handles paths with single quotes', async () => {
      writeFile("file'quoted.ts", 'content');
      const status = await checker.getFileStatus(join(tmpDir, "file'quoted.ts"));

      expect(status.isUntracked).toBe(true);
    });

    it('returns clean status for tracked, unmodified file', async () => {
      writeFile('clean.ts', 'content');
      git('add', 'clean.ts');
      git('commit', '-m', 'add clean');

      const status = await checker.getFileStatus(join(tmpDir, 'clean.ts'));

      expect(status.isStaged).toBe(false);
      expect(status.isUnstaged).toBe(false);
      expect(status.isUntracked).toBe(false);
    });
  });

  describe('status code parsing', () => {
    it('detects unstaged modification ( M)', async () => {
      writeFile('a.ts', 'original');
      git('add', 'a.ts');
      git('commit', '-m', 'add a');
      writeFile('a.ts', 'modified');

      const status = await checker.getFileStatus(join(tmpDir, 'a.ts'));

      expect(status.isUnstaged).toBe(true);
      expect(status.isStaged).toBe(false);
    });

    it('detects staged modification (M )', async () => {
      writeFile('b.ts', 'original');
      git('add', 'b.ts');
      git('commit', '-m', 'add b');
      writeFile('b.ts', 'modified');
      git('add', 'b.ts');

      const status = await checker.getFileStatus(join(tmpDir, 'b.ts'));

      expect(status.isStaged).toBe(true);
      expect(status.isUnstaged).toBe(false);
    });

    it('detects staged + unstaged (MM)', async () => {
      writeFile('c.ts', 'original');
      git('add', 'c.ts');
      git('commit', '-m', 'add c');
      writeFile('c.ts', 'staged-change');
      git('add', 'c.ts');
      writeFile('c.ts', 'unstaged-change');

      const status = await checker.getFileStatus(join(tmpDir, 'c.ts'));

      expect(status.isStaged).toBe(true);
      expect(status.isUnstaged).toBe(true);
    });

    it('detects untracked file (??)', async () => {
      writeFile('new.ts', 'content');

      const status = await checker.getFileStatus(join(tmpDir, 'new.ts'));

      expect(status.isUntracked).toBe(true);
      expect(status.isTracked).toBe(false);
    });

    it('detects staged new file (A )', async () => {
      writeFile('added.ts', 'content');
      git('add', 'added.ts');

      const status = await checker.getFileStatus(join(tmpDir, 'added.ts'));

      expect(status.isStaged).toBe(true);
      expect(status.isUntracked).toBe(false);
    });

    it('detects staged deletion (D )', async () => {
      writeFile('to-delete.ts', 'content');
      git('add', 'to-delete.ts');
      git('commit', '-m', 'add file');
      git('rm', 'to-delete.ts');

      const status = await checker.getFileStatus(join(tmpDir, 'to-delete.ts'));

      expect(status.isStaged).toBe(true);
    });
  });

  describe('getUnstagedFiles', () => {
    it('returns only files with unstaged changes', async () => {
      writeFile('unstaged.ts', 'v1');
      git('add', 'unstaged.ts');
      git('commit', '-m', 'add');
      writeFile('unstaged.ts', 'v2');

      writeFile('staged.ts', 'v1');
      git('add', 'staged.ts');

      const files = await checker.getUnstagedFiles();
      const names = files.map((f) => path.relative(tmpDir, f));

      expect(names).toContain('unstaged.ts');
      expect(names).not.toContain('staged.ts');
    });

    it('returns empty array for clean repo', async () => {
      const files = await checker.getUnstagedFiles();
      expect(files).toEqual([]);
    });
  });

  describe('getUntrackedFiles', () => {
    it('returns only untracked files', async () => {
      writeFile('tracked.ts', 'v1');
      git('add', 'tracked.ts');
      git('commit', '-m', 'add');

      writeFile('untracked.ts', 'new');

      const files = await checker.getUntrackedFiles();
      const names = files.map((f) => path.relative(tmpDir, f));

      expect(names).toContain('untracked.ts');
      expect(names).not.toContain('tracked.ts');
    });
  });

  describe('filterUnstaged', () => {
    it('filters to only unstaged files', async () => {
      writeFile('mod.ts', 'v1');
      git('add', 'mod.ts');
      git('commit', '-m', 'add');
      writeFile('mod.ts', 'v2');

      writeFile('clean.ts', 'ok');
      git('add', 'clean.ts');
      git('commit', '-m', 'add clean');

      const result = await checker.filterUnstaged([
        join(tmpDir, 'mod.ts'),
        join(tmpDir, 'clean.ts'),
      ]);

      expect(result).toHaveLength(1);
      expect(result[0]).toContain('mod.ts');
    });
  });
});

describe('getChangedFiles', () => {
  describe('with --since ref', () => {
    it('returns files changed since a ref', async () => {
      git('tag', 'v1');
      writeFile('after-tag.ts', 'new');
      git('add', 'after-tag.ts');
      git('commit', '-m', 'add after tag');

      const files = await getChangedFiles(tmpDir, { since: 'v1' });

      expect(files.some((f) => f.endsWith('after-tag.ts'))).toBe(true);
    });

    it('handles refs with unusual characters safely', async () => {
      // Create a tag with special chars (git allows this)
      git('tag', 'v1.0.0-beta+build.1');
      writeFile('x.ts', 'new');
      git('add', 'x.ts');
      git('commit', '-m', 'add x');

      const files = await getChangedFiles(tmpDir, { since: 'v1.0.0-beta+build.1' });
      expect(files.some((f) => f.endsWith('x.ts'))).toBe(true);
    });

    it('returns empty array for invalid ref (graceful error)', async () => {
      const files = await getChangedFiles(tmpDir, { since: 'nonexistent-ref' });
      expect(files).toEqual([]);
    });
  });

  describe('status mode (no --since)', () => {
    it('respects extension filter', async () => {
      writeFile('a.ts', 'ts');
      writeFile('b.js', 'js');
      git('add', '.');
      git('commit', '-m', 'add');
      writeFile('a.ts', 'changed');
      writeFile('b.js', 'changed');

      const files = await getChangedFiles(tmpDir, { extensions: ['.ts'] });

      expect(files.every((f) => f.endsWith('.ts'))).toBe(true);
    });

    it('returns sorted results', async () => {
      writeFile('z.ts', 'z');
      writeFile('a.ts', 'a');

      const files = await getChangedFiles(tmpDir, { untracked: true });
      const names = files.map((f) => f.split('/').pop());

      // Should be sorted
      for (let i = 1; i < names.length; i++) {
        expect(names[i]! >= names[i - 1]!).toBe(true);
      }
    });
  });
});
