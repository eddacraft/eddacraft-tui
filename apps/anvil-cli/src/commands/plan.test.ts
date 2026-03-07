import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const savePlanMock = vi.fn();
const getWorkspaceRootMock = vi.fn();

const spinner = {
  text: '',
  start: vi.fn(),
  succeed: vi.fn(),
  fail: vi.fn(),
  stop: vi.fn(),
};

spinner.start.mockReturnValue(spinner);

vi.mock('ora', () => ({
  default: vi.fn(() => spinner),
}));

vi.mock('../utils/file-io.js', () => ({
  savePlan: savePlanMock,
  getWorkspaceRoot: getWorkspaceRootMock,
}));

describe('plan command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getWorkspaceRootMock.mockReturnValue('/tmp/workspace');
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
    const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const { createPlanCommand } = await import('./plan.js');
    const command = createPlanCommand();

    await command.parseAsync([
      'node',
      'test',
      'create',
      'Create a complete release plan for this sprint',
      '--json',
    ]);

    expect(spinner.stop).toHaveBeenCalledTimes(1);
    expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('"intent"'));
    expect(savePlanMock).not.toHaveBeenCalled();
  });
});
