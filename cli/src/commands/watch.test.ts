import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createWatchCommand } from './watch.js';

describe('watch command', () => {
  beforeEach(() => {
    vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
  });

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

  it('should have --action option with default value', () => {
    const command = createWatchCommand();
    const actionOpt = command.options.find((o) => o.long === '--action');

    expect(actionOpt).toBeDefined();
    expect(actionOpt?.short).toBe('-a');
    expect(actionOpt?.defaultValue).toBe('validate');
    expect(actionOpt?.description()).toContain('validate');
    expect(actionOpt?.description()).toContain('gate');
    expect(actionOpt?.description()).toContain('check');
  });

  it('should have --source option for source file watching', () => {
    const command = createWatchCommand();
    const sourceOpt = command.options.find((o) => o.long === '--source');

    expect(sourceOpt).toBeDefined();
    expect(sourceOpt?.description()).toContain('source');
  });

  it('should have --patterns option for custom glob patterns', () => {
    const command = createWatchCommand();
    const patternsOpt = command.options.find((o) => o.long === '--patterns');

    expect(patternsOpt).toBeDefined();
    expect(patternsOpt?.description()).toContain('Glob');
  });

  it('should have --exclude option for exclusion patterns', () => {
    const command = createWatchCommand();
    const excludeOpt = command.options.find((o) => o.long === '--exclude');

    expect(excludeOpt).toBeDefined();
    expect(excludeOpt?.description()).toContain('exclude');
  });

  it('should have --debounce option with default value', () => {
    const command = createWatchCommand();
    const debounceOpt = command.options.find((o) => o.long === '--debounce');

    expect(debounceOpt).toBeDefined();
    expect(debounceOpt?.defaultValue).toBe('300');
    expect(debounceOpt?.description()).toContain('milliseconds');
  });

  it('should have --include-untracked option for git files', () => {
    const command = createWatchCommand();
    const untrackedOpt = command.options.find((o) => o.long === '--include-untracked');

    expect(untrackedOpt).toBeDefined();
    expect(untrackedOpt?.description()).toContain('untracked');
  });

  it('should have --no-git-filter option', () => {
    const command = createWatchCommand();
    const noGitOpt = command.options.find((o) => o.long === '--no-git-filter');

    expect(noGitOpt).toBeDefined();
    expect(noGitOpt?.description()).toContain('git');
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

  it('should have --tui option for interactive mode', () => {
    const command = createWatchCommand();
    const tuiOpt = command.options.find((o) => o.long === '--tui');

    expect(tuiOpt).toBeDefined();
  });

  it('should have --no-tui option to force plain text', () => {
    const command = createWatchCommand();
    const noTuiOpt = command.options.find((o) => o.long === '--no-tui');

    expect(noTuiOpt).toBeDefined();
  });
});
