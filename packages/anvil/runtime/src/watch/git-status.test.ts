import { describe, it, expect, beforeEach, vi } from 'vitest';
import { GitStatusChecker, getChangedFiles } from './git-status.js';

const { mockGitExec } = vi.hoisted(() => ({
  mockGitExec: vi.fn(),
}));

vi.mock('@eddacraft/anvil-core', () => ({
  createDebugger: () => vi.fn(),
  gitExec: mockGitExec,
}));

function gitResult(stdout: string, stderr = ''): { stdout: string; stderr: string } {
  return { stdout, stderr };
}

describe('GitStatusChecker', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('getFileStatus', () => {
    it('preserves leading whitespace for unstaged-only porcelain output', async () => {
      mockGitExec.mockResolvedValueOnce(gitResult(' M src/file.ts\n'));

      const checker = new GitStatusChecker('/repo');
      const status = await checker.getFileStatus('/repo/src/file.ts');

      expect(mockGitExec).toHaveBeenCalledWith(['status', '--porcelain', '--', 'src/file.ts'], {
        cwd: '/repo',
      });
      expect(status).toEqual({
        path: 'src/file.ts',
        isTracked: true,
        isStaged: false,
        isUnstaged: true,
        isUntracked: false,
        statusCode: ' M',
      });
    });

    it('uses the destination path for renamed quoted files', async () => {
      mockGitExec.mockResolvedValueOnce(
        gitResult('R  "old -> name.ts" -> "new -> name.ts"\n')
      );

      const checker = new GitStatusChecker('/repo');
      const status = await checker.getFileStatus('/repo/new -> name.ts');

      expect(status.path).toBe('new -> name.ts');
      expect(status.isTracked).toBe(true);
      expect(status.isStaged).toBe(true);
      expect(status.isUnstaged).toBe(false);
      expect(status.statusCode).toBe('R ');
    });
  });

  describe('filterUnstaged', () => {
    it('includes untracked files when requested and excludes staged-only files', async () => {
      mockGitExec.mockResolvedValueOnce(
        gitResult(' M src/live.ts\n?? "new file.ts"\nM  staged-only.ts\n')
      );

      const checker = new GitStatusChecker('/repo');
      const files = await checker.filterUnstaged(
        ['/repo/src/live.ts', '/repo/new file.ts', '/repo/staged-only.ts'],
        true
      );

      expect(files).toEqual(['/repo/src/live.ts', '/repo/new file.ts']);
    });
  });
});

describe('getChangedFiles', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('uses renamed destination paths and applies extension filters', async () => {
    mockGitExec.mockResolvedValueOnce(
      gitResult('R  "old name.ts" -> "new name.ts"\n?? docs/readme.md\n M src/live.ts\n')
    );

    const files = await getChangedFiles('/repo', {
      staged: true,
      unstaged: true,
      untracked: true,
      extensions: ['.ts'],
    });

    expect(files).toEqual(['/repo/new name.ts', '/repo/src/live.ts']);
  });
});
