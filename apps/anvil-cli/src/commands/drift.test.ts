import { describe, it, expect, vi, afterEach } from 'vitest';

const mockSave = vi.hoisted(() => vi.fn());
const mockLoad = vi.hoisted(() => vi.fn());
const mockList = vi.hoisted(() => vi.fn());
const mockGetLatest = vi.hoisted(() => vi.fn());
const mockCapture = vi.hoisted(() => vi.fn());

const mockOra = vi.hoisted(() => {
  const spinnerInstance = {
    start: vi.fn().mockReturnThis(),
    stop: vi.fn(),
    succeed: vi.fn(),
    fail: vi.fn(),
    text: '',
  };
  const oraFn = vi.fn(() => spinnerInstance);
  return { oraFn, spinnerInstance };
});

vi.mock('ora', () => ({ default: mockOra.oraFn }));

vi.mock('@eddacraft/anvil-core', () => ({
  SnapshotStore: class {
    save = mockSave;
    load = mockLoad;
    list = mockList;
    getLatest = mockGetLatest;
  },
  SnapshotCaptureService: class {
    capture = mockCapture;
  },
  compareSnapshots: vi.fn(() => ({
    duration_days: 7,
    metrics: {},
    net_change: 0,
    overall_trend: 'stable',
    violations: { added: [], removed: [] },
    antipatterns: { added: [], removed: [] },
  })),
  generateReport: vi.fn(() => ({})),
  formatReportAsText: vi.fn(() => 'Report text'),
  formatReportAsJson: vi.fn(() => '{}'),
  createDebugger: () => () => {},
}));

vi.mock('glob', () => ({
  glob: vi.fn(() => Promise.resolve(['src/index.ts', 'src/main.ts'])),
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

vi.mock('../utils/output.js', () => ({
  error: vi.fn(),
  info: vi.fn(),
}));

vi.mock('chalk', () => ({
  default: {
    bold: (s: string) => s,
    cyan: (s: string) => s,
    gray: (s: string) => s,
    dim: (s: string) => s,
    green: (s: string) => s,
    red: (s: string) => s,
    yellow: (s: string) => s,
  },
}));

import { createDriftCommand } from './drift.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockOra.oraFn.mockClear();
  mockOra.spinnerInstance.succeed.mockClear();
  mockOra.spinnerInstance.fail.mockClear();
  mockSave.mockReset();
  mockLoad.mockReset();
  mockList.mockReset();
  mockCapture.mockReset();
});

describe('drift command', () => {
  it('should create command with correct name and subcommands', () => {
    const command = createDriftCommand();

    expect(command.name()).toBe('drift');
    expect(command.description()).toContain('drift');

    const subcommandNames = command.commands.map((c) => c.name());
    expect(subcommandNames).toContain('snapshot');
    expect(subcommandNames).toContain('compare');
    expect(subcommandNames).toContain('report');
    expect(subcommandNames).toContain('list');
  });

  describe('snapshot subcommand', () => {
    it('should have --name and --json options', () => {
      const command = createDriftCommand();
      const snapshotCmd = command.commands.find((c) => c.name() === 'snapshot')!;

      const nameOpt = snapshotCmd.options.find((o) => o.long === '--name');
      expect(nameOpt).toBeDefined();

      const jsonOpt = snapshotCmd.options.find((o) => o.long === '--json');
      expect(jsonOpt).toBeDefined();
    });

    it('should capture and save a snapshot', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});

      const mockSnapshot = {
        name: 'test',
        created_at: new Date().toISOString(),
        metrics: {
          boundary_violations: 2,
          antipattern_count: 1,
          suppression_count: 0,
          files_analysed: 10,
        },
      };
      mockCapture.mockResolvedValue(mockSnapshot);
      mockSave.mockResolvedValue('/mock/.anvil/drift/snapshot-test.json');

      const command = createDriftCommand();
      await command.parseAsync(['snapshot', '--name', 'test'], { from: 'user' });

      expect(mockCapture).toHaveBeenCalled();
      expect(mockSave).toHaveBeenCalledWith(mockSnapshot, 'test');
      expect(mockOra.spinnerInstance.succeed).toHaveBeenCalled();
    });
  });

  describe('list subcommand', () => {
    it('should list available snapshots', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});

      mockList.mockResolvedValue([
        {
          name: 'release-1',
          filename: 'snapshot-release-1.json',
          created_at: '2026-01-01T00:00:00Z',
          metrics: { boundary_violations: 0, antipattern_count: 0, suppression_count: 0 },
        },
      ]);

      const command = createDriftCommand();
      await command.parseAsync(['list'], { from: 'user' });

      expect(mockList).toHaveBeenCalled();
      expect(mockOra.spinnerInstance.succeed).toHaveBeenCalled();
    });

    it('should show info when no snapshots found', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});
      mockList.mockResolvedValue([]);

      const command = createDriftCommand();
      await command.parseAsync(['list'], { from: 'user' });

      expect(mockOra.spinnerInstance.succeed).toHaveBeenCalledWith(
        expect.stringContaining('0 snapshot')
      );
    });
  });

  describe('compare subcommand', () => {
    it('should have --json option', () => {
      const command = createDriftCommand();
      const compareCmd = command.commands.find((c) => c.name() === 'compare')!;

      const jsonOpt = compareCmd.options.find((o) => o.long === '--json');
      expect(jsonOpt).toBeDefined();
    });

    it('should fail when snapshot not found', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});
      mockLoad.mockResolvedValue(null);

      const command = createDriftCommand();
      await expect(
        command.parseAsync(['compare', 'snap1', 'snap2'], { from: 'user' })
      ).rejects.toThrow();
    });
  });

  describe('report subcommand', () => {
    it('should have --since, --json, and --no-details options', () => {
      const command = createDriftCommand();
      const reportCmd = command.commands.find((c) => c.name() === 'report')!;

      const sinceOpt = reportCmd.options.find((o) => o.long === '--since');
      expect(sinceOpt).toBeDefined();

      const jsonOpt = reportCmd.options.find((o) => o.long === '--json');
      expect(jsonOpt).toBeDefined();

      const noDetailsOpt = reportCmd.options.find((o) => o.long === '--no-details');
      expect(noDetailsOpt).toBeDefined();
    });
  });
});
