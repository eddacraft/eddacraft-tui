import { describe, it, expect, vi, afterEach } from 'vitest';
import { createWatchCommand } from './watch.js';

describe('watch command', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', () => {
    const command = createWatchCommand();

    expect(command.name()).toBe('watch');
    expect(command.description()).toContain('Watch');
    expect(command.description()).toContain('changes');
  });

  it('should accept optional file argument', () => {
    const command = createWatchCommand();
    const args = command.registeredArguments;

    expect(args).toHaveLength(1);
    expect(args[0].name()).toBe('file');
    expect(args[0].required).toBe(false);
  });

  it('should have --action option', () => {
    const command = createWatchCommand();
    const actionOpt = command.options.find((o) => o.long === '--action');

    expect(actionOpt).toBeDefined();
    expect(actionOpt?.short).toBe('-a');
    // Default 'validate' is applied in action handler, not option definition
  });

  it('should have --source option for source file watching', () => {
    const command = createWatchCommand();
    const sourceOpt = command.options.find((o) => o.long === '--source');

    expect(sourceOpt).toBeDefined();
  });

  it('should have --patterns option for custom glob patterns', () => {
    const command = createWatchCommand();
    const patternsOpt = command.options.find((o) => o.long === '--patterns');

    expect(patternsOpt).toBeDefined();
  });

  it('should have --exclude option for exclusion patterns', () => {
    const command = createWatchCommand();
    const excludeOpt = command.options.find((o) => o.long === '--exclude');

    expect(excludeOpt).toBeDefined();
  });

  it('should have --debounce option with default value', () => {
    const command = createWatchCommand();
    const debounceOpt = command.options.find((o) => o.long === '--debounce');

    expect(debounceOpt).toBeDefined();
    expect(debounceOpt?.defaultValue).toBe('300');
  });

  it('should have --include-untracked option for git files', () => {
    const command = createWatchCommand();
    const untrackedOpt = command.options.find((o) => o.long === '--include-untracked');

    expect(untrackedOpt).toBeDefined();
  });

  it('should have --no-git-filter option', () => {
    const command = createWatchCommand();
    const noGitOpt = command.options.find((o) => o.long === '--no-git-filter');

    expect(noGitOpt).toBeDefined();
  });

  it('should have --profile option', () => {
    const command = createWatchCommand();
    const profileOpt = command.options.find((o) => o.long === '--profile');

    expect(profileOpt).toBeDefined();
    expect(profileOpt?.short).toBe('-p');
  });

  it('should have --verbose option', () => {
    const command = createWatchCommand();
    const verboseOpt = command.options.find((o) => o.long === '--verbose');

    expect(verboseOpt).toBeDefined();
    expect(verboseOpt?.short).toBe('-v');
  });
});

describe('watch signal-handler shutdown', () => {
  it('shutdown handler uses process.exit(0), not throw (regression)', async () => {
    // Signal handlers run outside the main async flow. If they throw
    // CliExit instead of calling process.exit(0), it becomes an
    // unhandled rejection. This test verifies by checking the compiled
    // createWatchCommand module for the correct pattern. Uses dynamic
    // import of the source to avoid brittle file-path coupling.
    const watchModule = await import('./watch.js');
    const source = watchModule.createWatchCommand.toString();

    // The stringified function must contain process.exit(0) in the
    // shutdown path and must not throw CliExit from signal handlers.
    expect(source).toContain('process.exit(0)');
    expect(source).toContain('SIGINT');
    expect(source).toContain('SIGTERM');
    expect(source).not.toMatch(/throw\s+new\s+CliExit/);
  });
});
