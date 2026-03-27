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

  describe('explain subcommand', () => {
    it('should have explain subcommand', () => {
      const command = createPolicyCommand();
      const explainCmd = command.commands.find((c) => c.name() === 'explain');

      expect(explainCmd).toBeDefined();
      expect(explainCmd?.description()).toContain('explanation');
    });

    it('should require name argument', () => {
      const command = createPolicyCommand();
      const explainCmd = command.commands.find((c) => c.name() === 'explain');
      const args = explainCmd?.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args?.[0].name()).toBe('name');
      expect(args?.[0].required).toBe(true);
    });
  });

  describe('why subcommand', () => {
    it('should have why subcommand', () => {
      const command = createPolicyCommand();
      const whyCmd = command.commands.find((c) => c.name() === 'why');

      expect(whyCmd).toBeDefined();
      expect(whyCmd?.description()).toContain('business reason');
    });

    it('should require violation argument', () => {
      const command = createPolicyCommand();
      const whyCmd = command.commands.find((c) => c.name() === 'why');
      const args = whyCmd?.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args?.[0].name()).toBe('violation');
      expect(args?.[0].required).toBe(true);
    });
  });

  describe('diff subcommand', () => {
    it('should have diff subcommand', () => {
      const command = createPolicyCommand();
      const diffCmd = command.commands.find((c) => c.name() === 'diff');

      expect(diffCmd).toBeDefined();
      expect(diffCmd?.description()).toContain('changes');
    });

    it('should have --dir option', () => {
      const command = createPolicyCommand();
      const diffCmd = command.commands.find((c) => c.name() === 'diff');
      const dirOpt = diffCmd?.options.find((o) => o.long === '--dir');

      expect(dirOpt).toBeDefined();
      expect(dirOpt?.defaultValue).toBe('.anvil/policies');
    });
  });

  describe('disable subcommand', () => {
    it('should have disable subcommand', () => {
      const command = createPolicyCommand();
      const disableCmd = command.commands.find((c) => c.name() === 'disable');

      expect(disableCmd).toBeDefined();
      expect(disableCmd?.description()).toContain('Disable');
    });

    it('should require name argument', () => {
      const command = createPolicyCommand();
      const disableCmd = command.commands.find((c) => c.name() === 'disable');
      const args = disableCmd?.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args?.[0].name()).toBe('name');
      expect(args?.[0].required).toBe(true);
    });
  });

  describe('enable subcommand', () => {
    it('should have enable subcommand', () => {
      const command = createPolicyCommand();
      const enableCmd = command.commands.find((c) => c.name() === 'enable');

      expect(enableCmd).toBeDefined();
      expect(enableCmd?.description()).toContain('enable');
    });

    it('should require name argument', () => {
      const command = createPolicyCommand();
      const enableCmd = command.commands.find((c) => c.name() === 'enable');
      const args = enableCmd?.registeredArguments;

      expect(args).toHaveLength(1);
      expect(args?.[0].name()).toBe('name');
      expect(args?.[0].required).toBe(true);
    });

    it('should have --enforcement option with default', () => {
      const command = createPolicyCommand();
      const enableCmd = command.commands.find((c) => c.name() === 'enable');
      const enforcementOpt = enableCmd?.options.find((o) => o.long === '--enforcement');

      expect(enforcementOpt).toBeDefined();
      expect(enforcementOpt?.short).toBe('-e');
      expect(enforcementOpt?.defaultValue).toBe('block');
    });
  });

  describe('doc subcommand', () => {
    it('should have doc subcommand', () => {
      const command = createPolicyCommand();
      const docCmd = command.commands.find((c) => c.name() === 'doc');

      expect(docCmd).toBeDefined();
      expect(docCmd?.description()).toContain('POLICIES.md');
    });

    it('should have --output option with default', () => {
      const command = createPolicyCommand();
      const docCmd = command.commands.find((c) => c.name() === 'doc');
      const outputOpt = docCmd?.options.find((o) => o.long === '--output');

      expect(outputOpt).toBeDefined();
      expect(outputOpt?.short).toBe('-o');
      expect(outputOpt?.defaultValue).toBe('.anvil/POLICIES.md');
    });
  });

  describe('scaffold subcommand', () => {
    it('should have scaffold subcommand', () => {
      const command = createPolicyCommand();
      const scaffoldCmd = command.commands.find((c) => c.name() === 'scaffold');

      expect(scaffoldCmd).toBeDefined();
      expect(scaffoldCmd?.description()).toContain('Scaffold');
    });

    it('should require --org option', () => {
      const command = createPolicyCommand();
      const scaffoldCmd = command.commands.find((c) => c.name() === 'scaffold');
      const orgOpt = scaffoldCmd?.options.find((o) => o.long === '--org');

      expect(orgOpt).toBeDefined();
      expect(orgOpt?.required).toBe(true);
    });

    it('should have --out option with default', () => {
      const command = createPolicyCommand();
      const scaffoldCmd = command.commands.find((c) => c.name() === 'scaffold');
      const outOpt = scaffoldCmd?.options.find((o) => o.long === '--out');

      expect(outOpt).toBeDefined();
      expect(outOpt?.defaultValue).toBe('./anvil-policies');
    });
  });

  describe('list subcommand options', () => {
    it('should have --all option', () => {
      const command = createPolicyCommand();
      const listCmd = command.commands.find((c) => c.name() === 'list');
      const allOpt = listCmd?.options.find((o) => o.long === '--all');

      expect(allOpt).toBeDefined();
      expect(allOpt?.short).toBe('-a');
    });

    it('should have --json option', () => {
      const command = createPolicyCommand();
      const listCmd = command.commands.find((c) => c.name() === 'list');
      const jsonOpt = listCmd?.options.find((o) => o.long === '--json');

      expect(jsonOpt).toBeDefined();
    });
  });

  describe('all subcommands present', () => {
    it('should have all expected subcommands', () => {
      const command = createPolicyCommand();
      const subcommandNames = command.commands.map((c) => c.name());

      expect(subcommandNames).toContain('list');
      expect(subcommandNames).toContain('explain');
      expect(subcommandNames).toContain('why');
      expect(subcommandNames).toContain('diff');
      expect(subcommandNames).toContain('disable');
      expect(subcommandNames).toContain('enable');
      expect(subcommandNames).toContain('doc');
      expect(subcommandNames).toContain('scaffold');
      expect(subcommandNames).toContain('validate');
      expect(subcommandNames).toContain('test');
      expect(subcommandNames).toContain('init');
      expect(subcommandNames).toContain('bundle');
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
