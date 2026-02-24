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

describe('watch signal-handler shutdown', () => {
  it('shutdown handler uses process.exit(0), not throw (regression)', async () => {
    // Signal handlers run outside the main async flow. If they throw
    // CliExit instead of calling process.exit(0), it becomes an
    // unhandled rejection. This test reads the source to guard against
    // regression — the shutdown function registered on SIGINT/SIGTERM
    // must call process.exit(0) directly.
    const { readFileSync } = await import('node:fs');
    const { fileURLToPath } = await import('node:url');
    const { dirname, join } = await import('node:path');

    const thisDir = dirname(fileURLToPath(import.meta.url));
    const source = readFileSync(join(thisDir, 'watch.ts'), 'utf-8');

    // Find the shutdown handler and the process.on registrations
    expect(source).toContain("process.on('SIGINT', shutdown)");
    expect(source).toContain("process.on('SIGTERM', shutdown)");

    // Extract the shutdown function body (between "const shutdown = async () => {"
    // and the closing of that function before process.on registrations)
    const shutdownStart = source.indexOf('const shutdown = async () => {');
    const sigintReg = source.indexOf("process.on('SIGINT'", shutdownStart);
    const shutdownBody = source.slice(shutdownStart, sigintReg);

    // Must use process.exit(0), not throw CliExit
    expect(shutdownBody).toContain('process.exit(0)');
    expect(shutdownBody).not.toMatch(/throw\s+new\s+CliExit/);
  });
});
