import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const { savePlanMock, getWorkspaceRootMock, execFileSyncMock, execFileMock, spinner } = vi.hoisted(
  () => {
    const s = {
      text: '',
      start: vi.fn(),
      succeed: vi.fn(),
      fail: vi.fn(),
      stop: vi.fn(),
    };
    s.start.mockReturnValue(s);
    return {
      savePlanMock: vi.fn(),
      getWorkspaceRootMock: vi.fn(),
      execFileSyncMock: vi.fn(),
      execFileMock: vi.fn(),
      spinner: s,
    };
  }
);

vi.mock('ora', () => ({
  default: vi.fn(() => spinner),
}));

vi.mock('node:child_process', () => ({
  default: { execFileSync: execFileSyncMock, execFile: execFileMock },
  execFileSync: execFileSyncMock,
  execFile: execFileMock,
}));

vi.mock('../utils/file-io.js', () => ({
  savePlan: savePlanMock,
  getWorkspaceRoot: getWorkspaceRootMock,
}));

describe('plan command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getWorkspaceRootMock.mockReturnValue('/tmp/workspace');
    execFileSyncMock.mockReturnValue('main');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { createPlanCommand } = await import('./plan.js');
    const command = createPlanCommand();

    expect(command.name()).toBe('plan');
    expect(command.description()).toContain('APS planning document management');
  });

  it('should register expected subcommands including legacy create', async () => {
    const { createPlanCommand } = await import('./plan.js');
    const command = createPlanCommand();
    const subcommands = command.commands.map((subcommand) => subcommand.name());

    expect(subcommands).toContain('validate');
    expect(subcommands).toContain('load');
    expect(subcommands).toContain('lock');
    expect(subcommands).toContain('unlock');
    expect(subcommands).toContain('status');
    expect(subcommands).toContain('create');
  });

  it('should output JSON from create subcommand on happy path', async () => {
    const stdoutWriteSpy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);

    const { createPlanCommand } = await import('./plan.js');
    const command = createPlanCommand();

    await command.parseAsync([
      'node',
      'test',
      'create',
      'Create a complete release plan for this sprint',
      '--json',
    ]);

    const output = stdoutWriteSpy.mock.calls.map((c) => String(c[0])).join('');

    expect(spinner.stop).toHaveBeenCalledTimes(1);
    expect(output).toContain('"intent"');
    expect(savePlanMock).not.toHaveBeenCalled();
  });
});
