import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Track execFileSync calls for argument safety verification
const execFileSyncCalls: Array<{ cmd: string; args: string[] }> = [];
let execFileSyncResult = '';
let execFileSyncError: Error | null = null;

vi.mock('node:child_process', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:child_process')>();

  const execFileSyncMock = vi.fn((...fnArgs: unknown[]) => {
    const cmd = fnArgs[0] as string;
    const args = fnArgs[1] as string[];
    execFileSyncCalls.push({ cmd, args });
    if (execFileSyncError) throw execFileSyncError;
    return execFileSyncResult;
  });

  // Provide a working execFile with promisify support for transitive deps
  const execFileMock = Object.assign(vi.fn(), {
    [Symbol.for('nodejs.util.promisify.custom')]: vi.fn(() =>
      Promise.resolve({ stdout: '', stderr: '' })
    ),
  });

  return {
    ...actual,
    default: { ...actual, execFileSync: execFileSyncMock, execFile: execFileMock },
    execFileSync: execFileSyncMock,
    execFile: execFileMock,
  };
});

import {
  parseCommitTrailers,
  extractAgentInfo,
  formatCommitWithAgent,
  getCommitAgentInfo,
  getRecentCommitsAgentInfo,
  getAgentContributions,
  getAiCommitPercentage,
  GIT_TRAILERS,
} from './git-agent.js';

describe('git-agent — command safety (CRB-014)', () => {
  beforeEach(() => {
    execFileSyncCalls.length = 0;
    execFileSyncResult = '';
    execFileSyncError = null;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('parseCommitTrailers', () => {
    it('should parse valid trailers', () => {
      // Trailing newline matches real git %B output — parseCommitTrailers
      // walks backwards and needs the empty last element to trigger collection
      const message = `fix: something\n\nSigned-off-by: test@example.com\nCo-authored-by: AI <ai@example.com>\n`;
      const trailers = parseCommitTrailers(message);

      expect(trailers['Signed-off-by']).toBe('test@example.com');
      expect(trailers['Co-authored-by']).toBe('AI <ai@example.com>');
    });

    it('should handle messages with no trailers', () => {
      const message = 'fix: simple commit\n\nJust a body paragraph.\n';
      const trailers = parseCommitTrailers(message);

      expect(Object.keys(trailers)).toHaveLength(0);
    });

    it('should not treat body lines as trailers if not after blank line', () => {
      const message = 'fix: something\nKey: Value';
      const trailers = parseCommitTrailers(message);

      expect(Object.keys(trailers)).toHaveLength(0);
    });

    it('should handle injection attempts in trailer values', () => {
      const message = `fix: commit\n\nAnvil-Agent-ID: $(cat /etc/passwd)\nAnvil-Agent-Type: \`whoami\`\n`;
      const trailers = parseCommitTrailers(message);

      expect(trailers['Anvil-Agent-ID']).toBe('$(cat /etc/passwd)');
      expect(trailers['Anvil-Agent-Type']).toBe('`whoami`');
    });

    it('should handle newline injection attempts', () => {
      const message = 'fix: commit\n\nTrailer: value\nmalicious\n\nAnother: safe';
      const trailers = parseCommitTrailers(message);

      expect(trailers['malicious']).toBeUndefined();
    });

    it('should reject trailers with invalid key format', () => {
      // Trailing newline is required — parseCommitTrailers walks backwards
      // and needs the empty last element to trigger trailer collection
      const message = 'fix: commit\n\n; rm -rf /: true\n../../../etc: value\n';
      const trailers = parseCommitTrailers(message);

      expect(trailers['; rm -rf /']).toBeUndefined();
      expect(trailers['../../../etc']).toBeUndefined();
    });
  });

  describe('extractAgentInfo', () => {
    it('should extract agent info from valid trailers', () => {
      const trailers = {
        [GIT_TRAILERS.AGENT_ID]: 'agent-123',
        [GIT_TRAILERS.AGENT_TYPE]: 'claude',
        [GIT_TRAILERS.SESSION_ID]: 'session-456',
        [GIT_TRAILERS.AGENT_NAME]: 'test-agent',
      };

      const info = extractAgentInfo(trailers);

      expect(info.agentId).toBe('agent-123');
      expect(info.agentType).toBe('claude');
      expect(info.sessionId).toBe('session-456');
      expect(info.isAiGenerated).toBe(true);
    });

    it('should detect AI-generated commits from co-author values', () => {
      const trailers = { 'Co-authored-by': 'Claude <claude@anthropic.com>' };
      const info = extractAgentInfo(trailers);

      expect(info.isAiGenerated).toBe(true);
      expect(info.coAuthors).toContain('Claude <claude@anthropic.com>');
    });

    it('should not flag non-AI commits as AI-generated', () => {
      const trailers = { 'Signed-off-by': 'human@example.com' };
      const info = extractAgentInfo(trailers);

      expect(info.isAiGenerated).toBe(false);
    });

    it('should handle injection attempt values safely', () => {
      const trailers = {
        [GIT_TRAILERS.AGENT_ID]: '$(rm -rf /)',
        [GIT_TRAILERS.AGENT_TYPE]: '`whoami`',
      };

      const info = extractAgentInfo(trailers);

      expect(info.agentId).toBe('$(rm -rf /)');
      expect(info.agentType).toBe('`whoami`');
    });
  });

  describe('getCommitAgentInfo — argument safety', () => {
    it('should pass commit ref with special characters as a single argument', () => {
      execFileSyncResult = 'fix: test commit\n';

      getCommitAgentInfo('HEAD; rm -rf /');

      const call = execFileSyncCalls[0];
      expect(call.cmd).toBe('git');
      expect(call.args).toContain('HEAD; rm -rf /');
      expect(call.args.filter((a) => a.includes('; rm'))).toHaveLength(1);
    });

    it('should pass commit ref with backticks as a single argument', () => {
      execFileSyncResult = 'fix: test\n';

      getCommitAgentInfo('`whoami`');

      expect(execFileSyncCalls[0].args).toContain('`whoami`');
    });

    it('should pass commit ref with $() as a single argument', () => {
      execFileSyncResult = 'fix: test\n';

      getCommitAgentInfo('$(cat /etc/passwd)');

      expect(execFileSyncCalls[0].args).toContain('$(cat /etc/passwd)');
    });

    it('should use execFileSync (not execSync) for shell safety', () => {
      execFileSyncResult = 'fix: test\n';

      getCommitAgentInfo('HEAD');

      expect(execFileSyncCalls.length).toBeGreaterThan(0);
      expect(execFileSyncCalls[0].cmd).toBe('git');
      expect(Array.isArray(execFileSyncCalls[0].args)).toBe(true);
    });

    it('should handle errors gracefully', () => {
      execFileSyncError = new Error('not a git repository');

      const result = getCommitAgentInfo('HEAD');

      expect(result).toBeNull();
    });
  });

  describe('getAgentContributions — sinceRef injection prevention', () => {
    it('should pass sinceRef with injection attempts as a single argument', () => {
      execFileSyncResult = '';

      getAgentContributions('main; rm -rf /', '/workspace');

      const call = execFileSyncCalls[0];
      expect(call.cmd).toBe('git');
      const refArg = call.args.find((a) => a.includes('..HEAD'));
      expect(refArg).toBe('main; rm -rf /..HEAD');
    });

    it('should pass sinceRef with backticks safely', () => {
      execFileSyncResult = '';

      getAgentContributions('`whoami`', '/workspace');

      const refArg = execFileSyncCalls[0].args.find((a) => a.includes('..HEAD'));
      expect(refArg).toBe('`whoami`..HEAD');
    });

    it('should handle errors gracefully', () => {
      execFileSyncError = new Error('not a git repository');

      const contributions = getAgentContributions('main', '/workspace');

      expect(contributions.size).toBe(0);
    });
  });

  describe('getAiCommitPercentage — sinceRef injection prevention', () => {
    it('should pass sinceRef safely to rev-list', () => {
      execFileSyncResult = '0\n';

      getAiCommitPercentage('main; echo pwned', '/workspace');

      const refArg = execFileSyncCalls[0].args.find((a) => a.includes('..HEAD'));
      expect(refArg).toBe('main; echo pwned..HEAD');
    });

    it('should handle errors gracefully', () => {
      execFileSyncError = new Error('not a git repository');

      const percentage = getAiCommitPercentage('main', '/workspace');

      expect(percentage).toBe(0);
    });
  });

  describe('formatCommitWithAgent', () => {
    it('should produce correctly formatted trailers', () => {
      const result = formatCommitWithAgent({
        message: 'fix: test commit',
        agent: {
          id: 'agent-123',
          type: 'claude',
          sessionId: 'session-456',
          name: 'test-agent',
          timestamp: new Date().toISOString(),
        },
      });

      expect(result).toContain('Anvil-Agent-ID: agent-123');
      expect(result).toContain('Anvil-Agent-Type: claude');
      expect(result).toContain('Anvil-Session-ID: session-456');
      expect(result).toContain('Co-authored-by: Claude <claude@anthropic.com>');
    });

    it('should add blank line before trailers if none exist', () => {
      const result = formatCommitWithAgent({
        message: 'fix: test commit',
        agent: {
          id: 'agent-123',
          type: 'claude',
          timestamp: new Date().toISOString(),
        },
      });

      expect(result).toMatch(/fix: test commit\n\nAnvil-Agent-ID/);
    });

    it('should handle message with special characters', () => {
      const result = formatCommitWithAgent({
        message: 'fix: handle $(cmd) and `backtick` in commit',
        agent: {
          id: 'agent-123',
          type: 'claude',
          timestamp: new Date().toISOString(),
        },
      });

      expect(result).toContain('$(cmd)');
      expect(result).toContain('`backtick`');
    });
  });

  describe('getRecentCommitsAgentInfo', () => {
    it('should not crash on empty git log output', () => {
      execFileSyncResult = '';

      const results = getRecentCommitsAgentInfo(10, '/workspace');

      expect(Array.isArray(results)).toBe(true);
    });

    it('should handle errors gracefully', () => {
      execFileSyncError = new Error('not a git repository');

      const results = getRecentCommitsAgentInfo(10, '/workspace');

      expect(results).toEqual([]);
    });
  });
});
