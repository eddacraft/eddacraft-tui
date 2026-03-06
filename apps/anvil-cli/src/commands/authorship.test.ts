import { describe, it, expect, vi, afterEach } from 'vitest';

const mockReadAuthorshipNote = vi.hoisted(() => vi.fn());
const mockListAuthorshipNotes = vi.hoisted(() => vi.fn());
const mockGetAuthorshipStats = vi.hoisted(() => vi.fn());
const mockGetWorkspaceRoot = vi.hoisted(() => vi.fn(() => '/mock/workspace'));

vi.mock('@eddacraft/anvil-core', () => ({
  readAuthorshipNote: mockReadAuthorshipNote,
  listAuthorshipNotes: mockListAuthorshipNotes,
  getAuthorshipStats: mockGetAuthorshipStats,
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: mockGetWorkspaceRoot,
}));

vi.mock('chalk', () => ({
  default: {
    bold: (s: string) => s,
    cyan: (s: string) => s,
    gray: (s: string) => s,
    green: (s: string) => s,
    red: (s: string) => s,
    hex: () => Object.assign((s: string) => s, { bold: (s: string) => s }),
  },
}));

vi.mock('../tui/utils/theme.js', () => ({
  theme: {
    colours: { smoke: '#888', ember: '#f60', steel: '#aaa', molten: '#f90' },
    icons: { bullet: '*', info: 'i' },
  },
}));

import { createAuthorshipCommand } from './authorship.js';

afterEach(() => {
  vi.restoreAllMocks();
  mockReadAuthorshipNote.mockReset();
  mockListAuthorshipNotes.mockReset();
  mockGetAuthorshipStats.mockReset();
});

describe('authorship command', () => {
  it('should create command with correct name and subcommands', () => {
    const command = createAuthorshipCommand();

    expect(command.name()).toBe('authorship');
    expect(command.description()).toContain('AI authorship');

    const subcommandNames = command.commands.map((c) => c.name());
    expect(subcommandNames).toContain('show');
    expect(subcommandNames).toContain('list');
    expect(subcommandNames).toContain('stats');
  });

  describe('show subcommand', () => {
    it('should output JSON when --json flag is set and log exists', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const mockLog = {
        attestations: {},
        metadata: { schema_version: '3.0.0', base_commit_sha: 'abc123', prompts: {} },
      };
      mockReadAuthorshipNote.mockResolvedValue(mockLog);

      const command = createAuthorshipCommand();
      await command.parseAsync(['show', '--json'], { from: 'user' });

      expect(mockReadAuthorshipNote).toHaveBeenCalledWith('HEAD', '/mock/workspace');

      const jsonOutput = consoleSpy.mock.calls.find((c) => {
        try {
          JSON.parse(c[0]);
          return true;
        } catch {
          return false;
        }
      });
      expect(jsonOutput).toBeDefined();
    });

    it('should show "not found" message when no log exists', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockReadAuthorshipNote.mockResolvedValue(null);

      const command = createAuthorshipCommand();
      await command.parseAsync(['show', 'abc123'], { from: 'user' });

      expect(mockReadAuthorshipNote).toHaveBeenCalledWith('abc123', '/mock/workspace');
      const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
      expect(output).toContain('No AI authorship');
    });
  });

  describe('list subcommand', () => {
    it('should list commits with AI authorship', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      mockListAuthorshipNotes.mockResolvedValue(['abc123', 'def456', 'ghi789']);

      const command = createAuthorshipCommand();
      await command.parseAsync(['list'], { from: 'user' });

      const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
      expect(output).toContain('abc123');
    });

    it('should output JSON when --json flag is set', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      mockListAuthorshipNotes.mockResolvedValue(['abc123']);

      const command = createAuthorshipCommand();
      await command.parseAsync(['list', '--json'], { from: 'user' });

      const jsonCall = consoleSpy.mock.calls.find((c) => c[0].includes('"total"'));
      expect(jsonCall).toBeDefined();
    });

    it('should show help message when no commits found', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      mockListAuthorshipNotes.mockResolvedValue([]);

      const command = createAuthorshipCommand();
      await command.parseAsync(['list'], { from: 'user' });

      const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
      expect(output).toContain('No commits');
    });
  });

  describe('stats subcommand', () => {
    it('should display statistics for default range', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      mockGetAuthorshipStats.mockResolvedValue({
        totalCommits: 10,
        commitsWithAI: 5,
        totalAdditions: 200,
        totalDeletions: 50,
        tools: { 'claude-code': 3, cursor: 2 },
      });

      const command = createAuthorshipCommand();
      await command.parseAsync(['stats'], { from: 'user' });

      expect(mockGetAuthorshipStats).toHaveBeenCalledWith('HEAD~10..HEAD', '/mock/workspace');
      const output = consoleSpy.mock.calls.map((c) => c[0]).join('\n');
      expect(output).toContain('5/10');
    });

    it('should output JSON when --json flag is set', async () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      mockGetAuthorshipStats.mockResolvedValue({
        totalCommits: 1,
        commitsWithAI: 1,
        totalAdditions: 10,
        totalDeletions: 0,
        tools: {},
      });

      const command = createAuthorshipCommand();
      await command.parseAsync(['stats', '--json'], { from: 'user' });

      const jsonCall = consoleSpy.mock.calls.find((c) => c[0].includes('"totalCommits"'));
      expect(jsonCall).toBeDefined();
    });
  });
});
