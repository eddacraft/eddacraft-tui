import { describe, it, expect, vi, afterEach } from 'vitest';

const mockRunRelease = vi.hoisted(() => vi.fn());

vi.mock('../services/release-runner.js', () => ({
  runRelease: mockRunRelease,
}));

import { createReleaseCommand } from './release.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockRunRelease.mockReset();
});

describe('release command', () => {
  it('should create command with correct name and description', () => {
    const command = createReleaseCommand();

    expect(command.name()).toBe('release');
    expect(command.description()).toContain('release workflow');
  });

  it('should have expected options with defaults', () => {
    const command = createReleaseCommand();

    const profileOpt = command.options.find((o) => o.long === '--profile');
    expect(profileOpt).toBeDefined();
    expect(profileOpt?.defaultValue).toBe('beta');

    const executeOpt = command.options.find((o) => o.long === '--execute');
    expect(executeOpt).toBeDefined();

    const resumeOpt = command.options.find((o) => o.long === '--resume');
    expect(resumeOpt).toBeDefined();

    const skipPreflightOpt = command.options.find((o) => o.long === '--skip-preflight');
    expect(skipPreflightOpt).toBeDefined();

    const verboseOpt = command.options.find((o) => o.long === '--verbose');
    expect(verboseOpt).toBeDefined();
    expect(verboseOpt?.short).toBe('-v');

    const jsonOpt = command.options.find((o) => o.long === '--json');
    expect(jsonOpt).toBeDefined();
  });

  it('should pass config to runRelease with defaults', async () => {
    mockRunRelease.mockResolvedValue(undefined);

    const command = createReleaseCommand();
    await command.parseAsync([], { from: 'user' });

    expect(mockRunRelease).toHaveBeenCalledWith(
      expect.objectContaining({
        execute: false,
        verbose: false,
        profile: 'beta',
        skipPreflight: false,
        resume: false,
      })
    );
  });

  it('should forward --execute and --target to runRelease', async () => {
    mockRunRelease.mockResolvedValue(undefined);

    const command = createReleaseCommand();
    await command.parseAsync(['--execute', '--target', '1.2.3', '--profile', 'stable'], {
      from: 'user',
    });

    expect(mockRunRelease).toHaveBeenCalledWith(
      expect.objectContaining({
        execute: true,
        profile: 'stable',
        targetVersion: '1.2.3',
      })
    );
  });
});
