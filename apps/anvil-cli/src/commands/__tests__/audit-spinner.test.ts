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

const mockScanResult = {
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
};

// Hoisted so mock factory can reference it; shared across all tests
const mockScan = vi.hoisted(() => vi.fn());

vi.mock('../../services/repo-scanner.js', () => {
  class MockRepoScanner {
    scan = mockScan;
  }
  return { RepoScanner: MockRepoScanner };
});

import { createAuditCommand } from '../audit.js';

// Set default scan behaviour before tests run
mockScan.mockResolvedValue(mockScanResult);

afterEach(() => {
  vi.restoreAllMocks();
  mockOra.oraFn.mockClear();
  mockOra.spinnerInstance.start.mockClear();
  mockOra.spinnerInstance.stop.mockClear();
  mockOra.spinnerInstance.fail.mockClear();
  mockScan.mockReset().mockResolvedValue(mockScanResult);
});

describe('audit spinner lifecycle (H-4)', () => {
  it('does not create spinner when --days-back is invalid', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createAuditCommand();
    await expect(command.parseAsync(['--days-back', '-5'], { from: 'user' })).rejects.toThrow(
      '--days-back must be a positive integer'
    );

    expect(mockOra.oraFn).not.toHaveBeenCalled();
  });

  it('does not create spinner when --max-commits is invalid', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createAuditCommand();
    await expect(command.parseAsync(['--max-commits', '0'], { from: 'user' })).rejects.toThrow(
      '--max-commits must be a positive integer'
    );

    expect(mockOra.oraFn).not.toHaveBeenCalled();
  });

  it('stops spinner cleanly when scan throws mid-operation', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});

    // Override the shared mockScan to reject for this test only
    mockScan.mockRejectedValueOnce(new Error('Scan exploded'));

    const command = createAuditCommand();
    await expect(command.parseAsync(['--days-back', '30'], { from: 'user' })).rejects.toThrow();

    expect(mockOra.oraFn).toHaveBeenCalled();
    expect(mockOra.spinnerInstance.fail).toHaveBeenCalled();
  });

  it('creates spinner only after validation passes', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createAuditCommand();
    // Valid options — spinner should be created and stopped; CliExit is thrown for clean exit
    await expect(
      command.parseAsync(['--days-back', '30', '--max-commits', '50'], { from: 'user' })
    ).rejects.toThrow('Clean exit');

    expect(mockOra.oraFn).toHaveBeenCalled();
    expect(mockOra.spinnerInstance.stop).toHaveBeenCalled();
  });
});
