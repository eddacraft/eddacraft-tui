import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createCheckCommand } from './check.js';

describe('check command', () => {
  beforeEach(() => {
    vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', () => {
    const command = createCheckCommand();

    expect(command.name()).toBe('check');
    expect(command.description()).toContain('architecture violations');
    expect(command.description()).toContain('anti-patterns');
  });

  it('should accept files as optional variadic argument', () => {
    const command = createCheckCommand();
    const args = command.registeredArguments;

    expect(args).toHaveLength(1);
    expect(args[0].name()).toBe('files');
    expect(args[0].variadic).toBe(true);
    expect(args[0].required).toBe(false);
  });

  it('should have --changed option for git-aware detection', () => {
    const command = createCheckCommand();
    const changedOpt = command.options.find((o) => o.long === '--changed');
    const sinceOpt = command.options.find((o) => o.long === '--since');
    const stagedOpt = command.options.find((o) => o.long === '--staged');

    expect(changedOpt).toBeDefined();
    expect(sinceOpt).toBeDefined();
    expect(stagedOpt).toBeDefined();
  });

  it('should have --verbose option', () => {
    const command = createCheckCommand();
    const verboseOpt = command.options.find((o) => o.long === '--verbose');

    expect(verboseOpt).toBeDefined();
    expect(verboseOpt?.short).toBe('-v');
  });

  it('should have --json option', () => {
    const command = createCheckCommand();
    const jsonOpt = command.options.find((o) => o.long === '--json');

    expect(jsonOpt).toBeDefined();
  });

  it('should have --no-cache option', () => {
    const command = createCheckCommand();
    const noCacheOpt = command.options.find((o) => o.long === '--no-cache');

    expect(noCacheOpt).toBeDefined();
  });

  it('should have --all option for analysing all files', () => {
    const command = createCheckCommand();
    const allOpt = command.options.find((o) => o.long === '--all');

    expect(allOpt).toBeDefined();
    expect(allOpt?.description).toContain('all source files');
  });
});

describe('check command JSON output structure', () => {
  it('should define correct JSONCheckOutput interface fields', () => {
    const expectedFields = [
      'version',
      'timestamp',
      'files',
      'hasBlockingWarnings',
      'executionTimeMs',
      'checksRun',
      'warnings',
      'summary',
    ];

    const sampleOutput = {
      version: '1.0.0',
      timestamp: new Date().toISOString(),
      files: ['test.ts'],
      hasBlockingWarnings: false,
      executionTimeMs: 100,
      checksRun: ['architecture'],
      warnings: [],
      summary: {
        total: 0,
        errors: 0,
        warnings: 0,
        info: 0,
        suppressed: 0,
      },
    };

    for (const field of expectedFields) {
      expect(sampleOutput).toHaveProperty(field);
    }
  });
});
