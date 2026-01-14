import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createGateCommand } from './gate.js';

describe('gate command', () => {
  beforeEach(() => {
    vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', () => {
    const command = createGateCommand();

    expect(command.name()).toBe('gate');
    expect(command.description()).toContain('quality gates');
  });

  it('should accept optional plan argument', () => {
    const command = createGateCommand();
    const args = command.registeredArguments;

    expect(args).toHaveLength(1);
    expect(args[0].name()).toBe('plan');
    expect(args[0].required).toBe(false);
  });

  it('should have --config option', () => {
    const command = createGateCommand();
    const configOpt = command.options.find((o) => o.long === '--config');

    expect(configOpt).toBeDefined();
    expect(configOpt?.short).toBe('-c');
  });

  it('should have --verbose option', () => {
    const command = createGateCommand();
    const verboseOpt = command.options.find((o) => o.long === '--verbose');

    expect(verboseOpt).toBeDefined();
    expect(verboseOpt?.short).toBe('-v');
  });

  it('should have --format option for explicit format specification', () => {
    const command = createGateCommand();
    const formatOpt = command.options.find((o) => o.long === '--format');

    expect(formatOpt).toBeDefined();
  });

  it('should have --native option to skip format detection', () => {
    const command = createGateCommand();
    const nativeOpt = command.options.find((o) => o.long === '--native');

    expect(nativeOpt).toBeDefined();
  });

  it('should have --inject option for evidence injection', () => {
    const command = createGateCommand();
    const injectOpt = command.options.find((o) => o.long === '--inject');

    expect(injectOpt).toBeDefined();
  });

  it('should have --skip-checks option', () => {
    const command = createGateCommand();
    const skipOpt = command.options.find((o) => o.long === '--skip-checks');

    expect(skipOpt).toBeDefined();
  });

  it('should have --only-checks option', () => {
    const command = createGateCommand();
    const onlyOpt = command.options.find((o) => o.long === '--only-checks');

    expect(onlyOpt).toBeDefined();
  });

  it('should have --fail-fast option', () => {
    const command = createGateCommand();
    const failFastOpt = command.options.find((o) => o.long === '--fail-fast');

    expect(failFastOpt).toBeDefined();
  });

  it('should have --profile option for predefined profiles', () => {
    const command = createGateCommand();
    const profileOpt = command.options.find((o) => o.long === '--profile');

    expect(profileOpt).toBeDefined();
    expect(profileOpt?.short).toBe('-p');
  });

  it('should have --list-profiles option', () => {
    const command = createGateCommand();
    const listOpt = command.options.find((o) => o.long === '--list-profiles');

    expect(listOpt).toBeDefined();
  });

  it('should have --no-cache option', () => {
    const command = createGateCommand();
    const noCacheOpt = command.options.find((o) => o.long === '--no-cache');

    expect(noCacheOpt).toBeDefined();
  });

  it('should have --parallel option for execution control', () => {
    const command = createGateCommand();
    const parallelOpt = command.options.find((o) => o.long === '--parallel');

    expect(parallelOpt).toBeDefined();
  });

  it('should have --output option with default human format', () => {
    const command = createGateCommand();
    const outputOpt = command.options.find((o) => o.long === '--output');

    expect(outputOpt).toBeDefined();
    expect(outputOpt?.short).toBe('-o');
    expect(outputOpt?.defaultValue).toBe('human');
  });

  it('should have --progress option', () => {
    const command = createGateCommand();
    const progressOpt = command.options.find((o) => o.long === '--progress');

    expect(progressOpt).toBeDefined();
  });

  it('should have --tui option for interactive mode', () => {
    const command = createGateCommand();
    const tuiOpt = command.options.find((o) => o.long === '--tui');

    expect(tuiOpt).toBeDefined();
  });

  it('should have --no-tui option to force plain text', () => {
    const command = createGateCommand();
    const noTuiOpt = command.options.find((o) => o.long === '--no-tui');

    expect(noTuiOpt).toBeDefined();
  });

  it('should have --skip-command-safety option', () => {
    const command = createGateCommand();
    const skipSafetyOpt = command.options.find((o) => o.long === '--skip-command-safety');

    expect(skipSafetyOpt).toBeDefined();
  });
});
