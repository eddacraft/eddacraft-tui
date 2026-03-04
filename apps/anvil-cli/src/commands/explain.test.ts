import { describe, it, expect, vi, afterEach } from 'vitest';

const mockGetExplainableRules = vi.hoisted(() => vi.fn());
const mockExplainById = vi.hoisted(() => vi.fn());
const mockExplainByRule = vi.hoisted(() => vi.fn());
const mockParseWarningId = vi.hoisted(() => vi.fn());
const mockIsExplainable = vi.hoisted(() => vi.fn());
const mockLoadRecentWarnings = vi.hoisted(() => vi.fn());

vi.mock('@eddacraft/anvil-core', () => ({
  explainById: mockExplainById,
  explainByRule: mockExplainByRule,
  parseWarningId: mockParseWarningId,
  isExplainable: mockIsExplainable,
  getExplainableRules: mockGetExplainableRules,
  createDebugger: () => () => {},
}));

vi.mock('../services/recent-warnings-store.js', () => ({
  loadRecentWarnings: mockLoadRecentWarnings,
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

vi.mock('chalk', () => ({
  default: {
    bold: Object.assign((s: string) => s, { underline: (s: string) => s }),
    gray: (s: string) => s,
    red: (s: string) => s,
    yellow: Object.assign((s: string) => s, { bold: (s: string) => s }),
    cyan: Object.assign((s: string) => s, { bold: (s: string) => s }),
  },
}));

import { createExplainCommand } from './explain.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockGetExplainableRules.mockReset();
  mockExplainById.mockReset();
  mockExplainByRule.mockReset();
  mockParseWarningId.mockReset();
  mockIsExplainable.mockReset();
  mockLoadRecentWarnings.mockReset();
});

describe('explain command', () => {
  it('should create command with correct name and options', () => {
    const command = createExplainCommand();

    expect(command.name()).toBe('explain');
    expect(command.description()).toContain('explanation');

    const listOpt = command.options.find((o) => o.long === '--list');
    expect(listOpt).toBeDefined();

    const rulesOpt = command.options.find((o) => o.long === '--rules');
    expect(rulesOpt).toBeDefined();

    const jsonOpt = command.options.find((o) => o.long === '--json');
    expect(jsonOpt).toBeDefined();
  });

  it('should list explainable rules with --rules flag', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    mockGetExplainableRules.mockReturnValue(['AP-001', 'AP-002', 'ARCH-001']);

    const command = createExplainCommand();
    await command.parseAsync(['--rules'], { from: 'user' });

    const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(output).toContain('AP-001');
    expect(output).toContain('ARCH-001');
  });

  it('should list recent warnings with --list flag', async () => {
    const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    mockLoadRecentWarnings.mockResolvedValue([
      {
        warningId: 'AP-003-src/utils.ts:42',
        title: 'God function detected',
        location: { file: 'src/utils.ts', line: 42 },
      },
    ]);
    mockParseWarningId.mockReturnValue({ rule: 'AP-003', file: 'src/utils.ts', line: 42 });

    const command = createExplainCommand();
    await command.parseAsync(['--list'], { from: 'user' });

    const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(output).toContain('AP-003');
  });

  it('should list recent warnings when called with no arguments', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    mockLoadRecentWarnings.mockResolvedValue([]);

    const command = createExplainCommand();
    await command.parseAsync([], { from: 'user' });

    expect(mockLoadRecentWarnings).toHaveBeenCalled();
  });

  it('should explain a warning by ID', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    mockLoadRecentWarnings.mockResolvedValue([]);
    mockParseWarningId.mockReturnValue({ rule: 'AP-003', file: 'src/utils.ts', line: 42 });
    mockExplainById.mockReturnValue({
      ruleId: 'AP-003',
      title: 'God function',
      whyItMatters: { title: 'WHY', content: 'Too complex' },
      howToAddress: { title: 'HOW', content: 'Split it' },
      whenToSuppress: { title: 'WHEN', content: 'Never' },
    });

    const command = createExplainCommand();
    await command.parseAsync(['AP-003-src/utils.ts:42'], { from: 'user' });

    expect(mockExplainById).toHaveBeenCalled();
  });

  it('should explain by rule name when ID parse fails', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    mockLoadRecentWarnings.mockResolvedValue([]);
    mockParseWarningId.mockReturnValue(null);
    mockIsExplainable.mockReturnValue(true);
    mockExplainByRule.mockReturnValue({
      ruleId: 'AP-003',
      title: 'God function',
      whyItMatters: { title: 'WHY', content: 'Complex' },
      howToAddress: { title: 'HOW', content: 'Refactor' },
      whenToSuppress: { title: 'WHEN', content: 'Rarely' },
    });

    const command = createExplainCommand();
    await command.parseAsync(['AP-003'], { from: 'user' });

    expect(mockExplainByRule).toHaveBeenCalledWith('AP-003');
  });

  it('should throw on unknown warning ID', async () => {
    vi.spyOn(console, 'log').mockImplementation(() => {});
    vi.spyOn(console, 'error').mockImplementation(() => {});
    mockLoadRecentWarnings.mockResolvedValue([]);
    mockParseWarningId.mockReturnValue(null);
    mockIsExplainable.mockReturnValue(false);

    const command = createExplainCommand();
    await expect(command.parseAsync(['UNKNOWN-999'], { from: 'user' })).rejects.toThrow(
      'Unknown warning ID or rule'
    );
  });
});
