import { describe, it, expect, vi, afterEach } from 'vitest';

const mockArchitectureYamlExists = vi.hoisted(() => vi.fn(() => false));
const mockGetArchitectureYamlPath = vi.hoisted(() => vi.fn(() => '/mock/.anvil/architecture.yaml'));
const mockGetAvailableTemplates = vi.hoisted(() =>
  vi.fn(() => [
    'starter',
    'layered',
    'hexagonal',
    'clean',
    'ddd',
    'monorepo',
    'serverless',
    'nx-workspace',
    'custom',
  ])
);
const mockParseArchitectureDefinition = vi.hoisted(() => vi.fn());
const mockNeedsCompilation = vi.hoisted(() => vi.fn());
const mockCompileArchitecture = vi.hoisted(() => vi.fn());
const mockGetTemplateDefaults = vi.hoisted(() => vi.fn(() => ({})));

vi.mock('@eddacraft/anvil-core', () => ({
  createDebugger: () => () => {},
  validatePathWithinRoot: (p: string) => p,
  architectureYamlExists: mockArchitectureYamlExists,
  getArchitectureYamlPath: mockGetArchitectureYamlPath,
  getAvailableTemplates: mockGetAvailableTemplates,
  getTemplateDefaults: mockGetTemplateDefaults,
  parseArchitectureDefinition: mockParseArchitectureDefinition,
  needsCompilation: mockNeedsCompilation,
  compileArchitecture: mockCompileArchitecture,
  getDCConfigPath: () => '/mock/.dependency-cruiser.cjs',
  getRegoPath: () => '/mock/.anvil/architecture.rego',
  ARCHITECTURE_YAML_FILENAME: 'architecture.yaml',
  ARCHITECTURE_DEFINITION_VERSION: '1.0.0',
  getDefaultOptions: () => ({
    detect_circular: true,
    detect_orphans: true,
    default_severity: 'warn',
  }),
}));

vi.mock('ora', () => {
  const spinnerInstance = {
    start: vi.fn().mockReturnThis(),
    stop: vi.fn(),
    succeed: vi.fn(),
    fail: vi.fn(),
    text: '',
  };
  return { default: vi.fn(() => spinnerInstance) };
});

vi.mock('inquirer', () => ({
  default: {
    prompt: vi.fn(),
    Separator: class {},
  },
}));

vi.mock('chalk', () => ({
  default: {
    bold: Object.assign((s: string) => s, { cyan: (s: string) => s, green: (s: string) => s }),
    cyan: (s: string) => s,
    dim: (s: string) => s,
    gray: (s: string) => s,
    green: (s: string) => s,
    red: (s: string) => s,
    white: (s: string) => s,
    yellow: (s: string) => s,
  },
}));

vi.mock('YAML', () => ({
  default: { stringify: vi.fn(() => 'yaml content') },
}));

vi.mock('beautiful-mermaid', () => ({
  renderMermaidAscii: vi.fn(() => 'ASCII diagram'),
  renderMermaid: vi.fn(() => '<svg></svg>'),
}));

vi.mock('node:fs', () => {
  const mock = { existsSync: vi.fn(() => false) };
  return { ...mock, default: mock };
});

vi.mock('node:fs/promises', () => {
  const mock = { mkdir: vi.fn(), writeFile: vi.fn() };
  return { ...mock, default: mock };
});

import { createArchitectureCommand } from './architecture.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockArchitectureYamlExists.mockReset().mockReturnValue(false);
  mockParseArchitectureDefinition.mockReset();
});

describe('architecture command', () => {
  it('should create command with correct name, alias, and subcommands', () => {
    const command = createArchitectureCommand();

    expect(command.name()).toBe('architecture');
    expect(command.aliases()).toContain('arch');
    expect(command.description()).toContain('architecture');

    const subcommandNames = command.commands.map((c) => c.name());
    expect(subcommandNames).toContain('init');
    expect(subcommandNames).toContain('generate');
    expect(subcommandNames).toContain('validate');
    expect(subcommandNames).toContain('show');
    expect(subcommandNames).toContain('visualise');
  });

  describe('init subcommand', () => {
    it('should have expected options', () => {
      const command = createArchitectureCommand();
      const initCmd = command.commands.find((c) => c.name() === 'init')!;

      const templateOpt = initCmd.options.find((o) => o.long === '--template');
      expect(templateOpt).toBeDefined();

      const forceOpt = initCmd.options.find((o) => o.long === '--force');
      expect(forceOpt).toBeDefined();

      const nonInteractiveOpt = initCmd.options.find((o) => o.long === '--non-interactive');
      expect(nonInteractiveOpt).toBeDefined();
    });

    it('should reject when architecture.yaml exists without --force', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});
      mockArchitectureYamlExists.mockReturnValue(true);

      const command = createArchitectureCommand();
      await expect(
        command.parseAsync(['init', '--non-interactive'], { from: 'user' })
      ).rejects.toThrow('already exists');
    });
  });

  describe('generate subcommand', () => {
    it('should have expected options', () => {
      const command = createArchitectureCommand();
      const genCmd = command.commands.find((c) => c.name() === 'generate')!;

      expect(genCmd.aliases()).toContain('gen');

      const forceOpt = genCmd.options.find((o) => o.long === '--force');
      expect(forceOpt).toBeDefined();

      const skipDcOpt = genCmd.options.find((o) => o.long === '--skip-dc');
      expect(skipDcOpt).toBeDefined();

      const skipRegoOpt = genCmd.options.find((o) => o.long === '--skip-rego');
      expect(skipRegoOpt).toBeDefined();
    });

    it('should reject when no architecture.yaml exists', async () => {
      vi.spyOn(console, 'log').mockImplementation(() => {});
      mockArchitectureYamlExists.mockReturnValue(false);

      const command = createArchitectureCommand();
      await expect(command.parseAsync(['generate'], { from: 'user' })).rejects.toThrow('not found');
    });
  });

  describe('show subcommand', () => {
    it('should have --json and --yaml options', () => {
      const command = createArchitectureCommand();
      const showCmd = command.commands.find((c) => c.name() === 'show')!;

      const jsonOpt = showCmd.options.find((o) => o.long === '--json');
      expect(jsonOpt).toBeDefined();

      const yamlOpt = showCmd.options.find((o) => o.long === '--yaml');
      expect(yamlOpt).toBeDefined();
    });
  });

  describe('visualise subcommand', () => {
    it('should have format and output options and aliases', () => {
      const command = createArchitectureCommand();
      const vizCmd = command.commands.find((c) => c.name() === 'visualise')!;

      expect(vizCmd.aliases()).toContain('visualize');
      expect(vizCmd.aliases()).toContain('viz');

      const formatOpt = vizCmd.options.find((o) => o.long === '--format');
      expect(formatOpt).toBeDefined();

      const outputOpt = vizCmd.options.find((o) => o.long === '--output');
      expect(outputOpt).toBeDefined();
    });
  });
});
