import { describe, it, expect, vi, afterEach } from 'vitest';

const mockTemplateLoader = vi.hoisted(() => ({
  loadTemplates: vi.fn(),
  getAllTemplates: vi.fn(() => []),
  getTemplate: vi.fn(),
  renderTemplate: vi.fn(),
}));

vi.mock('../services/template-loader.js', () => ({
  createTemplateLoader: () => mockTemplateLoader,
}));

vi.mock('../tui/utils/tty-detection.js', () => ({
  isTUIAvailable: () => false,
}));

vi.mock('../tui/utils/theme.js', () => ({
  theme: {
    icons: { error: 'X', success: 'OK', arrow: '>' },
  },
}));

vi.mock('@eddacraft/anvil-core', () => ({
  validatePathWithinRoot: (p: string) => p,
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

vi.mock('node:fs', () => ({
  writeFileSync: vi.fn(),
  existsSync: vi.fn(() => false),
}));

import { createNewCommand } from './new.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockTemplateLoader.loadTemplates.mockReset();
  mockTemplateLoader.getAllTemplates.mockReset();
  mockTemplateLoader.getTemplate.mockReset();
  mockTemplateLoader.renderTemplate.mockReset();
});

describe('new command', () => {
  it('should create command with correct name and options', () => {
    const command = createNewCommand();

    expect(command.name()).toBe('new');
    expect(command.description()).toContain('template');

    const listOpt = command.options.find((o) => o.long === '--list');
    expect(listOpt).toBeDefined();

    const outputOpt = command.options.find((o) => o.long === '--output');
    expect(outputOpt).toBeDefined();

    const forceOpt = command.options.find((o) => o.long === '--force');
    expect(forceOpt).toBeDefined();

    const categoryOpt = command.options.find((o) => o.long === '--category');
    expect(categoryOpt).toBeDefined();
  });

  it('should throw when no templates found', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    mockTemplateLoader.getAllTemplates.mockReturnValue([]);

    const command = createNewCommand();
    await expect(command.parseAsync(['some-template'], { from: 'user' })).rejects.toThrow(
      'No templates found'
    );
  });

  it('should list templates with --list flag', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    mockTemplateLoader.getAllTemplates.mockReturnValue([
      {
        metadata: {
          id: 'basic-plan',
          name: 'Basic Plan',
          description: 'A basic plan',
          category: 'planning',
          variables: [],
        },
        content: '',
      },
    ]);

    const command = createNewCommand();
    await command.parseAsync(['--list'], { from: 'user' });

    const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(output).toContain('basic-plan');
  });

  it('should throw when template not found', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
    mockTemplateLoader.getAllTemplates.mockReturnValue([
      {
        metadata: {
          id: 'exists',
          name: 'Exists',
          description: '',
          category: 'planning',
          variables: [],
        },
        content: '',
      },
    ]);
    mockTemplateLoader.getTemplate.mockReturnValue(null);

    const command = createNewCommand();
    await expect(command.parseAsync(['nonexistent'], { from: 'user' })).rejects.toThrow(
      'Template not found'
    );
  });

  it('should generate from template with --var', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    const mockTemplate = {
      metadata: {
        id: 'basic',
        name: 'Basic',
        description: '',
        category: 'planning',
        variables: [{ name: 'project', required: true, description: 'Project name' }],
      },
      content: 'Hello {{project}}',
    };
    mockTemplateLoader.getAllTemplates.mockReturnValue([mockTemplate]);
    mockTemplateLoader.getTemplate.mockReturnValue(mockTemplate);
    mockTemplateLoader.renderTemplate.mockReturnValue({ content: 'Hello MyProject' });

    const command = createNewCommand();
    await command.parseAsync(['basic', '--var', 'project=MyProject'], { from: 'user' });

    expect(mockTemplateLoader.renderTemplate).toHaveBeenCalledWith(
      mockTemplate,
      expect.objectContaining({ project: 'MyProject' })
    );
  });
});
