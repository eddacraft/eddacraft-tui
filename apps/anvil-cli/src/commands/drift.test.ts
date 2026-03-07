import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const getWorkspaceRootMock = vi.fn();
const listMock = vi.fn();
const loadMock = vi.fn();
const getLatestMock = vi.fn();
const saveMock = vi.fn();

const spinner = {
  text: '',
  start: vi.fn(),
  succeed: vi.fn(),
  fail: vi.fn(),
};

spinner.start.mockReturnValue(spinner);

vi.mock('ora', () => ({
  default: vi.fn(() => spinner),
}));

vi.mock('glob', () => ({
  glob: vi.fn().mockResolvedValue([]),
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: getWorkspaceRootMock,
}));

vi.mock('@eddacraft/anvil-core', () => ({
  createDebugger: vi.fn(() => vi.fn()),
  SnapshotStore: class {
    list = listMock;
    load = loadMock;
    getLatest = getLatestMock;
    save = saveMock;
  },
  SnapshotCaptureService: class {
    capture = vi.fn();
  },
  compareSnapshots: vi.fn(),
  generateReport: vi.fn(),
  formatReportAsText: vi.fn(() => 'report'),
  formatReportAsJson: vi.fn(() => '{"report":true}'),
}));

describe('drift command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getWorkspaceRootMock.mockReturnValue('/tmp/workspace');
    listMock.mockResolvedValue([
      {
        filename: 'snapshot-a.json',
        name: 'snapshot-a',
        created_at: '2026-01-01T00:00:00.000Z',
        metrics: {
          boundary_violations: 1,
          antipattern_count: 2,
          suppression_count: 0,
        },
      },
    ]);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { createDriftCommand } = await import('./drift.js');
    const command = createDriftCommand();

    expect(command.name()).toBe('drift');
    expect(command.description()).toContain('architecture drift');
  });

  it('should register snapshot, compare, report, and list subcommands', async () => {
    const { createDriftCommand } = await import('./drift.js');
    const command = createDriftCommand();
    const subcommands = command.commands.map((subcommand) => subcommand.name());

    expect(subcommands).toContain('snapshot');
    expect(subcommands).toContain('compare');
    expect(subcommands).toContain('report');
    expect(subcommands).toContain('list');
  });

  it('should list snapshots as JSON on happy path', async () => {
    const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    const { createDriftCommand } = await import('./drift.js');
    const command = createDriftCommand();

    await command.parseAsync(['node', 'test', 'list', '--json']);

    expect(getWorkspaceRootMock).toHaveBeenCalledTimes(1);
    expect(listMock).toHaveBeenCalledTimes(1);
    expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('snapshot-a.json'));
  });
});
