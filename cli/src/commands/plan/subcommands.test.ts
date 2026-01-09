import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createLoadSubcommand } from './load.js';
import { createLockSubcommand } from './lock.js';
import { createUnlockSubcommand } from './unlock.js';
import { createStatusSubcommand } from './status.js';
import { createValidateSubcommand } from './validate.js';

describe('plan subcommands', () => {
  beforeEach(() => {
    vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('load subcommand', () => {
    it('should create command with correct name and description', () => {
      const command = createLoadSubcommand();

      expect(command.name()).toBe('load');
      expect(command.description()).toContain('Load');
    });

    it('should require path argument', () => {
      const command = createLoadSubcommand();
      const args = command.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args[0].name()).toBe('path');
      expect(args[0].required).toBe(true);
    });

    it('should have filter options', () => {
      const command = createLoadSubcommand();

      const scopeOpt = command.options.find((o) => o.long === '--scope');
      const moduleOpt = command.options.find((o) => o.long === '--module');
      const taskOpt = command.options.find((o) => o.long === '--task');
      const ownerOpt = command.options.find((o) => o.long === '--owner');
      const tagOpt = command.options.find((o) => o.long === '--tag');

      expect(scopeOpt).toBeDefined();
      expect(moduleOpt).toBeDefined();
      expect(taskOpt).toBeDefined();
      expect(ownerOpt).toBeDefined();
      expect(tagOpt).toBeDefined();
    });

    it('should have output format options', () => {
      const command = createLoadSubcommand();

      const jsonOpt = command.options.find((o) => o.long === '--json');
      const textOpt = command.options.find((o) => o.long === '--text');
      const filesOnlyOpt = command.options.find((o) => o.long === '--files-only');

      expect(jsonOpt).toBeDefined();
      expect(textOpt).toBeDefined();
      expect(filesOnlyOpt).toBeDefined();
    });
  });

  describe('lock subcommand', () => {
    it('should create command with correct name and description', () => {
      const command = createLockSubcommand();

      expect(command.name()).toBe('lock');
      expect(command.description()).toContain('Lock');
      expect(command.description()).toContain('task');
    });

    it('should require task argument', () => {
      const command = createLockSubcommand();
      const args = command.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args[0].name()).toBe('task');
      expect(args[0].required).toBe(true);
    });

    it('should have --plan option with default value', () => {
      const command = createLockSubcommand();
      const planOpt = command.options.find((o) => o.long === '--plan');

      expect(planOpt).toBeDefined();
      expect(planOpt?.defaultValue).toBe('docs/planning/APS.md');
    });

    it('should have --user option for provenance', () => {
      const command = createLockSubcommand();
      const userOpt = command.options.find((o) => o.long === '--user');

      expect(userOpt).toBeDefined();
      expect(userOpt?.description()).toContain('provenance');
    });

    it('should have --skip-validation option', () => {
      const command = createLockSubcommand();
      const skipOpt = command.options.find((o) => o.long === '--skip-validation');

      expect(skipOpt).toBeDefined();
    });

    it('should have --json option', () => {
      const command = createLockSubcommand();
      const jsonOpt = command.options.find((o) => o.long === '--json');

      expect(jsonOpt).toBeDefined();
    });
  });

  describe('unlock subcommand', () => {
    it('should create command with correct name and description', () => {
      const command = createUnlockSubcommand();

      expect(command.name()).toBe('unlock');
      expect(command.description()).toContain('Unlock');
      expect(command.description()).toContain('cancel');
    });

    it('should require task argument', () => {
      const command = createUnlockSubcommand();
      const args = command.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args[0].name()).toBe('task');
      expect(args[0].required).toBe(true);
    });

    it('should have --plan option with default value', () => {
      const command = createUnlockSubcommand();
      const planOpt = command.options.find((o) => o.long === '--plan');

      expect(planOpt).toBeDefined();
      expect(planOpt?.defaultValue).toBe('docs/planning/APS.md');
    });

    it('should have --json option', () => {
      const command = createUnlockSubcommand();
      const jsonOpt = command.options.find((o) => o.long === '--json');

      expect(jsonOpt).toBeDefined();
    });
  });

  describe('status subcommand', () => {
    it('should create command with correct name and description', () => {
      const command = createStatusSubcommand();

      expect(command.name()).toBe('status');
      expect(command.description()).toContain('status');
      expect(command.description()).toContain('task');
    });

    it('should have --plan option with default value', () => {
      const command = createStatusSubcommand();
      const planOpt = command.options.find((o) => o.long === '--plan');

      expect(planOpt).toBeDefined();
      expect(planOpt?.defaultValue).toBe('docs/planning/APS.md');
    });

    it('should have --json option', () => {
      const command = createStatusSubcommand();
      const jsonOpt = command.options.find((o) => o.long === '--json');

      expect(jsonOpt).toBeDefined();
    });

    it('should have --summary option', () => {
      const command = createStatusSubcommand();
      const summaryOpt = command.options.find((o) => o.long === '--summary');

      expect(summaryOpt).toBeDefined();
    });
  });

  describe('validate subcommand', () => {
    it('should create command with correct name and description', () => {
      const command = createValidateSubcommand();

      expect(command.name()).toBe('validate');
      expect(command.description()).toContain('Validate');
    });

    it('should have optional path argument with default', () => {
      const command = createValidateSubcommand();
      const args = command.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args[0].name()).toBe('path');
      expect(args[0].required).toBe(false);
      expect(args[0].defaultValue).toBe('docs/planning/APS.md');
    });

    it('should have --json option', () => {
      const command = createValidateSubcommand();
      const jsonOpt = command.options.find((o) => o.long === '--json');

      expect(jsonOpt).toBeDefined();
    });
  });
});
