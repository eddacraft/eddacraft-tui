// @vitest-environment node
import { describe, it, expect, vi, afterEach } from 'vitest';

const mockOra = vi.hoisted(() => {
  const spinnerInstance = {
    start: vi.fn().mockReturnThis(),
    stop: vi.fn(),
    fail: vi.fn(),
    text: '',
  };
  const oraFn = vi.fn(() => spinnerInstance);
  return { oraFn, spinnerInstance };
});

vi.mock('ora', () => ({ default: mockOra.oraFn }));

vi.mock('../../tui/utils/tty-detection.js', () => ({
  isTUIAvailable: () => false,
}));

vi.mock('../../tui/utils/renderer.js', () => ({
  renderTUI: vi.fn(),
}));

vi.mock('../../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

vi.mock('../../services/repo-scanner.js', () => {
  class MockRepoScanner {
    scan = vi.fn().mockResolvedValue({
      timestamp: new Date(),
      project: {
        framework: 'none',
        size: 'small',
        fileCount: 10,
        monorepo: 'none',
        tsStrictness: 'strict',
        workspacePackages: [],
      },
      currentIssues: {
        filesScanned: 10,
        totalWarnings: 0,
        bySeverity: { errors: 0, warnings: 0, info: 0 },
        byCategory: {},
        topIssues: [],
        hasBlockingWarnings: false,
        executionTimeMs: 50,
        checksRun: ['test-check'],
      },
      historical: {
        totalCommits: 0,
        totalViolations: 0,
        avgViolationsPerCommit: 0,
        patternOccurrences: [],
        dateRange: { from: new Date(), to: new Date() },
      },
      totalDurationMs: 100,
    });
  }
  return { RepoScanner: MockRepoScanner };
});

import { createAuditCommand } from '../audit.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockOra.oraFn.mockClear();
  mockOra.spinnerInstance.start.mockClear();
  mockOra.spinnerInstance.stop.mockClear();
  mockOra.spinnerInstance.fail.mockClear();
});

describe('audit spinner lifecycle (H-4)', () => {
  it('does not create spinner when --days-back is invalid', async () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(() => {
      throw new Error('process.exit');
    });
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createAuditCommand();
    await expect(command.parseAsync(['--days-back', '-5'], { from: 'user' })).rejects.toThrow(
      'process.exit'
    );

    expect(mockOra.oraFn).not.toHaveBeenCalled();
    expect(exitSpy).toHaveBeenCalledWith(1);
  });

  it('does not create spinner when --max-commits is invalid', async () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(() => {
      throw new Error('process.exit');
    });
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createAuditCommand();
    await expect(command.parseAsync(['--max-commits', '0'], { from: 'user' })).rejects.toThrow(
      'process.exit'
    );

    expect(mockOra.oraFn).not.toHaveBeenCalled();
    expect(exitSpy).toHaveBeenCalledWith(1);
  });

  it('creates spinner only after validation passes', async () => {
    vi.spyOn(process, 'exit').mockImplementation(() => {
      throw new Error('process.exit');
    });
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createAuditCommand();
    // Valid options — spinner should be created and stopped
    await expect(
      command.parseAsync(['--days-back', '30', '--max-commits', '50'], { from: 'user' })
    ).rejects.toThrow('process.exit');

    expect(mockOra.oraFn).toHaveBeenCalled();
    expect(mockOra.spinnerInstance.stop).toHaveBeenCalled();
  });
});
