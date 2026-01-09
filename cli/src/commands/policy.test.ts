import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createPolicyCommand } from './policy.js';

describe('policy command', () => {
  beforeEach(() => {
    vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', () => {
    const command = createPolicyCommand();

    expect(command.name()).toBe('policy');
    expect(command.description()).toContain('OPA');
    expect(command.description()).toContain('Rego');
  });

  describe('list subcommand', () => {
    it('should have list subcommand', () => {
      const command = createPolicyCommand();
      const listCmd = command.commands.find((c) => c.name() === 'list');

      expect(listCmd).toBeDefined();
      expect(listCmd?.description()).toContain('List');
    });

    it('should have --dir option with default value', () => {
      const command = createPolicyCommand();
      const listCmd = command.commands.find((c) => c.name() === 'list');
      const dirOpt = listCmd?.options.find((o) => o.long === '--dir');

      expect(dirOpt).toBeDefined();
      expect(dirOpt?.short).toBe('-d');
      expect(dirOpt?.defaultValue).toBe('.anvil/policies');
    });
  });

  describe('validate subcommand', () => {
    it('should have validate subcommand', () => {
      const command = createPolicyCommand();
      const validateCmd = command.commands.find((c) => c.name() === 'validate');

      expect(validateCmd).toBeDefined();
      expect(validateCmd?.description()).toContain('Validate');
      expect(validateCmd?.description()).toContain('Rego');
    });

    it('should require file argument', () => {
      const command = createPolicyCommand();
      const validateCmd = command.commands.find((c) => c.name() === 'validate');
      const args = validateCmd?.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args?.[0].name()).toBe('file');
      expect(args?.[0].required).toBe(true);
    });
  });

  describe('test subcommand', () => {
    it('should have test subcommand', () => {
      const command = createPolicyCommand();
      const testCmd = command.commands.find((c) => c.name() === 'test');

      expect(testCmd).toBeDefined();
      expect(testCmd?.description()).toContain('test');
    });

    it('should accept optional policy argument', () => {
      const command = createPolicyCommand();
      const testCmd = command.commands.find((c) => c.name() === 'test');
      const args = testCmd?.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args?.[0].name()).toBe('policy');
      expect(args?.[0].required).toBe(false);
    });

    it('should have --dir option', () => {
      const command = createPolicyCommand();
      const testCmd = command.commands.find((c) => c.name() === 'test');
      const dirOpt = testCmd?.options.find((o) => o.long === '--dir');

      expect(dirOpt).toBeDefined();
      expect(dirOpt?.short).toBe('-d');
      expect(dirOpt?.defaultValue).toBe('.anvil/policies');
    });

    it('should have --verbose option', () => {
      const command = createPolicyCommand();
      const testCmd = command.commands.find((c) => c.name() === 'test');
      const verboseOpt = testCmd?.options.find((o) => o.long === '--verbose');

      expect(verboseOpt).toBeDefined();
      expect(verboseOpt?.short).toBe('-v');
    });
  });

  describe('init subcommand', () => {
    it('should have init subcommand', () => {
      const command = createPolicyCommand();
      const initCmd = command.commands.find((c) => c.name() === 'init');

      expect(initCmd).toBeDefined();
      expect(initCmd?.description()).toContain('Initialise');
      expect(initCmd?.description()).toContain('example');
    });

    it('should have --dir option', () => {
      const command = createPolicyCommand();
      const initCmd = command.commands.find((c) => c.name() === 'init');
      const dirOpt = initCmd?.options.find((o) => o.long === '--dir');

      expect(dirOpt).toBeDefined();
      expect(dirOpt?.short).toBe('-d');
      expect(dirOpt?.defaultValue).toBe('.anvil/policies');
    });

    it('should have --force option', () => {
      const command = createPolicyCommand();
      const initCmd = command.commands.find((c) => c.name() === 'init');
      const forceOpt = initCmd?.options.find((o) => o.long === '--force');

      expect(forceOpt).toBeDefined();
      expect(forceOpt?.description()).toContain('Overwrite');
    });
  });
});
