import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const explainByIdMock = vi.fn();
const explainByRuleMock = vi.fn();
const parseWarningIdMock = vi.fn();
const isExplainableMock = vi.fn();
const getExplainableRulesMock = vi.fn();
const loadRecentWarningsMock = vi.fn();
const getWorkspaceRootMock = vi.fn();

vi.mock('@eddacraft/anvil-core', () => ({
  explainById: explainByIdMock,
  explainByRule: explainByRuleMock,
  parseWarningId: parseWarningIdMock,
  isExplainable: isExplainableMock,
  getExplainableRules: getExplainableRulesMock,
  createDebugger: vi.fn(() => vi.fn()),
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: getWorkspaceRootMock,
}));

vi.mock('../services/recent-warnings-store.js', () => ({
  loadRecentWarnings: loadRecentWarningsMock,
}));

describe('explain command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getWorkspaceRootMock.mockReturnValue('/tmp/workspace');
    loadRecentWarningsMock.mockResolvedValue([]);
    getExplainableRulesMock.mockReturnValue(['AP-003', 'ARCH-001']);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name, description, and warning-id argument', async () => {
    const { createExplainCommand } = await import('./explain.js');
    const command = createExplainCommand();

    expect(command.name()).toBe('explain');
    expect(command.description()).toContain('detailed explanation');
    expect(command.registeredArguments[0]?.name()).toBe('warning-id');
  });

  it('should register list, rules, and json options', async () => {
    const { createExplainCommand } = await import('./explain.js');
    const command = createExplainCommand();

    expect(command.options.find((option) => option.long === '--list')).toBeDefined();
    expect(command.options.find((option) => option.long === '--rules')).toBeDefined();
    expect(command.options.find((option) => option.long === '--json')).toBeDefined();
  });

  it('should output JSON explanation for an explainable rule', async () => {
    parseWarningIdMock.mockReturnValue(null);
    isExplainableMock.mockReturnValue(true);
    explainByRuleMock.mockReturnValue({
      ruleId: 'AP-003',
      title: 'Explicit any type',
      whyItMatters: { title: 'Why', content: 'Type safety matters' },
      howToAddress: { title: 'Fix', content: 'Use strict types' },
      whenToSuppress: { title: 'Suppress', content: 'Only for generated files' },
    });
    const stdoutWriteSpy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);

    const { createExplainCommand } = await import('./explain.js');
    const command = createExplainCommand();

    await command.parseAsync(['node', 'test', 'AP-003', '--json']);

    expect(loadRecentWarningsMock).toHaveBeenCalledWith('/tmp/workspace');
    expect(explainByRuleMock).toHaveBeenCalledWith('AP-003');
    const output = stdoutWriteSpy.mock.calls.map((call) => String(call[0])).join('');
    expect(output).toContain('"ruleId": "AP-003"');
  });
});
