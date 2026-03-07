import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const runReleaseMock = vi.fn();

vi.mock('../services/release-runner.js', () => ({
  runRelease: runReleaseMock,
}));

describe('release command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    runReleaseMock.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { createReleaseCommand } = await import('./release.js');
    const command = createReleaseCommand();

    expect(command.name()).toBe('release');
    expect(command.description()).toContain('Interactive release workflow');
  });

  it('should register profile, target, execute, resume, and json options', async () => {
    const { createReleaseCommand } = await import('./release.js');
    const command = createReleaseCommand();

    expect(command.options.find((option) => option.long === '--profile')?.defaultValue).toBe(
      'beta'
    );
    expect(command.options.find((option) => option.long === '--target')).toBeDefined();
    expect(command.options.find((option) => option.long === '--execute')).toBeDefined();
    expect(command.options.find((option) => option.long === '--resume')).toBeDefined();
    expect(command.options.find((option) => option.long === '--json')).toBeDefined();
  });

  it('should call release runner with mapped options on happy path', async () => {
    const { createReleaseCommand } = await import('./release.js');
    const command = createReleaseCommand();

    await command.parseAsync([
      'node',
      'test',
      '--profile',
      'stable',
      '--target',
      '1.2.3',
      '--execute',
      '--resume',
      '--skip-preflight',
      '--verbose',
    ]);

    expect(runReleaseMock).toHaveBeenCalledWith({
      execute: true,
      verbose: true,
      profile: 'stable',
      skipPreflight: true,
      targetVersion: '1.2.3',
      resume: true,
    });
  });
});
