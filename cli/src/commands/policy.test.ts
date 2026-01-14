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
    });
  });

  describe('bundle subcommand', () => {
    it('should have bundle subcommand', () => {
      const command = createPolicyCommand();
      const bundleCmd = command.commands.find((c) => c.name() === 'bundle');

      expect(bundleCmd).toBeDefined();
      expect(bundleCmd?.description()).toContain('bundle');
    });

    describe('bundle list', () => {
      it('should have list subcommand under bundle', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const listCmd = bundleCmd?.commands.find((c) => c.name() === 'list');

        expect(listCmd).toBeDefined();
        expect(listCmd?.description()).toContain('List');
        expect(listCmd?.description()).toContain('bundle');
      });
    });

    describe('bundle add', () => {
      it('should have add subcommand under bundle', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');

        expect(addCmd).toBeDefined();
        expect(addCmd?.description()).toContain('Add');
      });

      it('should require url argument', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const args = addCmd?.registeredArguments;

        expect(args).toHaveLength(1);
        expect(args?.[0].name()).toBe('url');
        expect(args?.[0].required).toBe(true);
      });

      it('should have --name option', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const nameOpt = addCmd?.options.find((o) => o.long === '--name');

        expect(nameOpt).toBeDefined();
        expect(nameOpt?.short).toBe('-n');
      });

      it('should have --refresh option', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const refreshOpt = addCmd?.options.find((o) => o.long === '--refresh');

        expect(refreshOpt).toBeDefined();
        expect(refreshOpt?.short).toBe('-r');
        expect(refreshOpt?.defaultValue).toBe('300000');
      });

      it('should have --key option for signature verification', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const keyOpt = addCmd?.options.find((o) => o.long === '--key');

        expect(keyOpt).toBeDefined();
        expect(keyOpt?.short).toBe('-k');
      });

      it('should have --auth-user option for basic auth', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const authUserOpt = addCmd?.options.find((o) => o.long === '--auth-user');

        expect(authUserOpt).toBeDefined();
      });

      it('should have --auth-pass-env option for basic auth password', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const authPassEnvOpt = addCmd?.options.find((o) => o.long === '--auth-pass-env');

        expect(authPassEnvOpt).toBeDefined();
      });

      it('should have --auth-token-env option for bearer auth', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const authTokenEnvOpt = addCmd?.options.find((o) => o.long === '--auth-token-env');

        expect(authTokenEnvOpt).toBeDefined();
      });

      it('should have --no-sync option', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const addCmd = bundleCmd?.commands.find((c) => c.name() === 'add');
        const noSyncOpt = addCmd?.options.find((o) => o.long === '--no-sync');

        expect(noSyncOpt).toBeDefined();
      });
    });

    describe('bundle remove', () => {
      it('should have remove subcommand under bundle', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const removeCmd = bundleCmd?.commands.find((c) => c.name() === 'remove');

        expect(removeCmd).toBeDefined();
        expect(removeCmd?.description()).toContain('Remove');
      });

      it('should require name argument', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const removeCmd = bundleCmd?.commands.find((c) => c.name() === 'remove');
        const args = removeCmd?.registeredArguments;

        expect(args).toHaveLength(1);
        expect(args?.[0].name()).toBe('name');
        expect(args?.[0].required).toBe(true);
      });

      it('should have --keep-cache option', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const removeCmd = bundleCmd?.commands.find((c) => c.name() === 'remove');
        const keepCacheOpt = removeCmd?.options.find((o) => o.long === '--keep-cache');

        expect(keepCacheOpt).toBeDefined();
      });
    });

    describe('bundle sync', () => {
      it('should have sync subcommand under bundle', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const syncCmd = bundleCmd?.commands.find((c) => c.name() === 'sync');

        expect(syncCmd).toBeDefined();
        expect(syncCmd?.description()).toContain('Download');
        expect(syncCmd?.description()).toContain('update');
      });

      it('should have --force option', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const syncCmd = bundleCmd?.commands.find((c) => c.name() === 'sync');
        const forceOpt = syncCmd?.options.find((o) => o.long === '--force');

        expect(forceOpt).toBeDefined();
        expect(forceOpt?.short).toBe('-f');
      });

      it('should have --name option to sync specific bundle', () => {
        const command = createPolicyCommand();
        const bundleCmd = command.commands.find((c) => c.name() === 'bundle');
        const syncCmd = bundleCmd?.commands.find((c) => c.name() === 'sync');
        const nameOpt = syncCmd?.options.find((o) => o.long === '--name');

        expect(nameOpt).toBeDefined();
        expect(nameOpt?.short).toBe('-n');
      });
    });
  });
});
