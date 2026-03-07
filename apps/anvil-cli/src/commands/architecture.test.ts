import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const architectureYamlExistsMock = vi.fn();
const parseArchitectureDefinitionMock = vi.fn();

const spinner = {
  text: '',
  start: vi.fn(),
  succeed: vi.fn(),
  fail: vi.fn(),
};

spinner.start.mockReturnValue(spinner);

vi.mock('inquirer', () => ({
  default: {
    prompt: vi.fn(),
    Separator: class {},
  },
}));

vi.mock('ora', () => ({
  default: vi.fn(() => spinner),
}));

vi.mock('beautiful-mermaid', () => ({
  renderMermaidAscii: vi.fn(() => 'graph'),
  renderMermaid: vi.fn(async () => '<svg />'),
}));

vi.mock('@eddacraft/anvil-core', () => ({
  createDebugger: vi.fn(() => vi.fn()),
  validatePathWithinRoot: vi.fn((path: string) => path),
  getAvailableTemplates: vi.fn(() => ['layered', 'custom']),
  getTemplateDefaults: vi.fn(() => ({
    presentation: { patterns: ['apps/**'], depends_on: [], description: 'Presentation layer' },
  })),
  architectureYamlExists: architectureYamlExistsMock,
  getArchitectureYamlPath: vi.fn(() => '/tmp/workspace/.anvil/architecture.yaml'),
  parseArchitectureDefinition: parseArchitectureDefinitionMock,
  compileArchitecture: vi.fn(),
  needsCompilation: vi.fn(async () => ({ dc: false, rego: false })),
  getDCConfigPath: vi.fn(() => '.anvil/dependency-cruiser.cjs'),
  getRegoPath: vi.fn(() => '.anvil/policy.rego'),
  ARCHITECTURE_YAML_FILENAME: 'architecture.yaml',
  ARCHITECTURE_DEFINITION_VERSION: '1.0.0',
  getDefaultOptions: vi.fn(() => ({
    detect_circular: true,
    detect_orphans: true,
    default_severity: 'error',
  })),
}));

describe('architecture command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    architectureYamlExistsMock.mockReturnValue(true);
    parseArchitectureDefinitionMock.mockResolvedValue({
      schema_version: '1.0.0',
      template: 'layered',
      layers: {
        presentation: { patterns: ['apps/**'], depends_on: [] },
        business: { patterns: ['packages/**'], depends_on: ['presentation'] },
      },
      rules: [],
      options: {
        detect_circular: true,
        detect_orphans: true,
        default_severity: 'error',
      },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name, alias, and description', async () => {
    const { createArchitectureCommand } = await import('./architecture.js');
    const command = createArchitectureCommand();

    expect(command.name()).toBe('architecture');
    expect(command.alias()).toBe('arch');
    expect(command.description()).toContain('dependency rules');
  });

  it('should register init, generate, validate, show, and visualise subcommands', async () => {
    const { createArchitectureCommand } = await import('./architecture.js');
    const command = createArchitectureCommand();
    const subcommands = command.commands.map((subcommand) => subcommand.name());

    expect(subcommands).toContain('init');
    expect(subcommands).toContain('generate');
    expect(subcommands).toContain('validate');
    expect(subcommands).toContain('show');
    expect(subcommands).toContain('visualise');
  });

  it('should validate architecture definition on happy path', async () => {
    const { createArchitectureCommand } = await import('./architecture.js');
    const command = createArchitectureCommand();

    await command.parseAsync(['node', 'test', 'validate']);

    expect(architectureYamlExistsMock).toHaveBeenCalledTimes(1);
    expect(parseArchitectureDefinitionMock).toHaveBeenCalledTimes(1);
    expect(spinner.succeed).toHaveBeenCalledWith(expect.stringContaining('valid'));
  });
});
