import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const writeFileSyncMock = vi.fn();
const existsSyncMock = vi.fn();
const validatePathWithinRootMock = vi.fn();
const isTUIAvailableMock = vi.fn();
const getWorkspaceRootMock = vi.fn();

const loadTemplatesMock = vi.fn();
const getAllTemplatesMock = vi.fn();
const getTemplateMock = vi.fn();
const renderTemplateMock = vi.fn();

vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  return {
    ...actual,
    default: { ...actual, writeFileSync: writeFileSyncMock, existsSync: existsSyncMock },
    writeFileSync: writeFileSyncMock,
    existsSync: existsSyncMock,
  };
});

vi.mock('@eddacraft/anvil-core', () => ({
  validatePathWithinRoot: validatePathWithinRootMock,
}));

vi.mock('../services/template-loader.js', () => ({
  createTemplateLoader: vi.fn(() => ({
    loadTemplates: loadTemplatesMock,
    getAllTemplates: getAllTemplatesMock,
    getTemplate: getTemplateMock,
    renderTemplate: renderTemplateMock,
  })),
}));

vi.mock('../tui/utils/tty-detection.js', () => ({
  isTUIAvailable: isTUIAvailableMock,
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: getWorkspaceRootMock,
}));

describe('new command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    loadTemplatesMock.mockResolvedValue(undefined);
    getWorkspaceRootMock.mockReturnValue('/tmp/workspace');
    validatePathWithinRootMock.mockReturnValue('/tmp/workspace/output.md');
    existsSyncMock.mockReturnValue(false);
    isTUIAvailableMock.mockReturnValue(false);
    getAllTemplatesMock.mockReturnValue([
      {
        metadata: {
          id: 'feature-spec',
          name: 'Feature Spec',
          description: 'Feature planning template',
          category: 'planning',
          variables: [{ name: 'feature', required: true, description: 'Feature name' }],
        },
      },
    ]);
    getTemplateMock.mockReturnValue({
      metadata: {
        id: 'feature-spec',
        name: 'Feature Spec',
        description: 'Feature planning template',
        category: 'planning',
        variables: [{ name: 'feature', required: true, description: 'Feature name' }],
      },
    });
    renderTemplateMock.mockReturnValue({ content: '# Feature Spec' });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { createNewCommand } = await import('./new.js');
    const command = createNewCommand();

    expect(command.name()).toBe('new');
    expect(command.description()).toContain('new plan from a template');
  });

  it('should register list, output, force, and var options', async () => {
    const { createNewCommand } = await import('./new.js');
    const command = createNewCommand();

    expect(command.options.find((option) => option.long === '--list')).toBeDefined();
    expect(command.options.find((option) => option.long === '--output')).toBeDefined();
    expect(command.options.find((option) => option.long === '--force')).toBeDefined();
    expect(command.options.find((option) => option.long === '--var')).toBeDefined();
  });

  it('should render and write template output on happy path', async () => {
    const { createNewCommand } = await import('./new.js');
    const command = createNewCommand();

    await command.parseAsync([
      'node',
      'test',
      'feature-spec',
      '--var',
      'feature=Auth',
      '--output',
      'plans/auth.md',
    ]);

    expect(loadTemplatesMock).toHaveBeenCalledTimes(1);
    expect(getTemplateMock).toHaveBeenCalledWith('feature-spec');
    expect(writeFileSyncMock).toHaveBeenCalledWith(
      '/tmp/workspace/output.md',
      '# Feature Spec',
      'utf-8'
    );
  });
});
