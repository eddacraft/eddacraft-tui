import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { execFileSync } from 'node:child_process';
import {
  parseCommitTrailers,
  extractAgentInfo,
  formatCommitWithAgent,
  GIT_TRAILERS,
  getCommitAgentInfo,
  getRecentCommitsAgentInfo,
  getAgentContributions,
  getAiCommitPercentage,
} from './git-agent.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

/**
 * CRB-014: Tests for git command composition in concurrency/git-agent.ts
 *
 * Pure function tests (parseCommitTrailers, extractAgentInfo,
 * formatCommitWithAgent) need no mocking. Git-calling functions use a real
 * temporary repo to verify safe command composition.
 */

// ============================================================================
// parseCommitTrailers — pure function tests
// ============================================================================

describe('parseCommitTrailers', () => {
  it('extracts trailers after blank line', () => {
    const msg =
      'feat: add feature\n\nSome body text.\n\nAnvil-Agent-ID: agent-123\nAnvil-Agent-Type: claude';
    const trailers = parseCommitTrailers(msg);

    expect(trailers['Anvil-Agent-ID']).toBe('agent-123');
    expect(trailers['Anvil-Agent-Type']).toBe('claude');
  });

  it('handles trailers with colons in values', () => {
    const msg = 'fix: thing\n\nCo-authored-by: Bot <bot@example.com>';
    const trailers = parseCommitTrailers(msg);

    expect(trailers['Co-authored-by']).toBe('Bot <bot@example.com>');
  });

  it('handles varied whitespace around colon', () => {
    const msg = 'chore: tidy\n\nKey:value\nKey2 : spaced';
    const trailers = parseCommitTrailers(msg);

    expect(trailers['Key']).toBe('value');
    expect(trailers['Key2']).toBe('spaced');
  });

  it('handles hyphenated trailer keys', () => {
    const msg = 'feat: add\n\nCo-Authored-By: User <user@example.com>';
    const trailers = parseCommitTrailers(msg);

    expect(trailers['Co-Authored-By']).toBe('User <user@example.com>');
  });

  it('returns empty record for message with no trailers', () => {
    const msg = 'fix: simple fix\n\nNo trailers here.';
    expect(parseCommitTrailers(msg)).toEqual({});
  });

  it('returns empty record for empty message', () => {
    expect(parseCommitTrailers('')).toEqual({});
  });

  it('stops parsing trailers at blank line (reads bottom-up)', () => {
    const msg = 'feat: add\n\nParagraph with Key: value\n\nReal-Trailer: yes';
    const trailers = parseCommitTrailers(msg);

    expect(trailers['Real-Trailer']).toBe('yes');
    expect(trailers['Key']).toBeUndefined();
  });

  it('handles message with only subject line', () => {
    expect(parseCommitTrailers('fix: quick')).toEqual({});
  });
});

// ============================================================================
// extractAgentInfo — pure function tests
// ============================================================================

describe('extractAgentInfo', () => {
  it('detects AI-generated commit from agent ID', () => {
    const info = extractAgentInfo({ [GIT_TRAILERS.AGENT_ID]: 'agent-123' });
    expect(info.isAiGenerated).toBe(true);
    expect(info.agentId).toBe('agent-123');
  });

  it('detects AI from co-author containing "claude"', () => {
    const info = extractAgentInfo({
      'Co-authored-by': 'Claude <claude@anthropic.com>',
    });
    expect(info.isAiGenerated).toBe(true);
    expect(info.coAuthors).toContain('Claude <claude@anthropic.com>');
  });

  it('detects AI from co-author containing "copilot"', () => {
    const info = extractAgentInfo({
      'Co-authored-by': 'GitHub Copilot <copilot@github.com>',
    });
    expect(info.isAiGenerated).toBe(true);
  });

  it('returns isAiGenerated=false for human commits', () => {
    const info = extractAgentInfo({ 'Signed-off-by': 'Human <human@example.com>' });
    expect(info.isAiGenerated).toBe(false);
  });

  it('extracts all trailer fields', () => {
    const info = extractAgentInfo({
      [GIT_TRAILERS.AGENT_ID]: 'id-1',
      [GIT_TRAILERS.AGENT_TYPE]: 'cursor',
      [GIT_TRAILERS.SESSION_ID]: 'sess-1',
      [GIT_TRAILERS.AGENT_NAME]: 'my-cursor',
    });

    expect(info.agentId).toBe('id-1');
    expect(info.agentType).toBe('cursor');
    expect(info.sessionId).toBe('sess-1');
    expect(info.agentName).toBe('my-cursor');
  });

  it('returns empty co-authors when none present', () => {
    const info = extractAgentInfo({});
    expect(info.coAuthors).toEqual([]);
  });
});

// ============================================================================
// formatCommitWithAgent — pure function tests
// ============================================================================

describe('formatCommitWithAgent', () => {
  const testAgent = {
    id: 'test-001',
    type: 'claude' as const,
    name: 'test-claude',
    sessionId: 'sess-abc',
  };

  it('appends trailers after blank line', () => {
    const result = formatCommitWithAgent({ message: 'feat: add', agent: testAgent });

    expect(result).toContain('\n\n');
    expect(result).toContain(`${GIT_TRAILERS.AGENT_ID}: test-001`);
    expect(result).toContain(`${GIT_TRAILERS.AGENT_TYPE}: claude`);
  });

  it('includes session ID trailer', () => {
    const result = formatCommitWithAgent({ message: 'fix: bug', agent: testAgent });
    expect(result).toContain(`${GIT_TRAILERS.SESSION_ID}: sess-abc`);
  });

  it('includes co-authored-by for AI agents', () => {
    const result = formatCommitWithAgent({ message: 'fix: bug', agent: testAgent });
    expect(result).toContain('Co-authored-by: Claude <claude@anthropic.com>');
  });

  it('omits co-authored-by when disabled', () => {
    const result = formatCommitWithAgent({
      message: 'fix: bug',
      agent: testAgent,
      includeCoAuthor: false,
    });
    expect(result).not.toContain('Co-authored-by');
  });

  it('includes additional trailers', () => {
    const result = formatCommitWithAgent({
      message: 'chore: tidy',
      agent: testAgent,
      additionalTrailers: { 'Custom-Key': 'custom-value' },
    });
    expect(result).toContain('Custom-Key: custom-value');
  });

  it('appends to existing trailers without extra blank line', () => {
    const msg = 'feat: add\n\nSigned-off-by: Human <human@example.com>';
    const result = formatCommitWithAgent({ message: msg, agent: testAgent });

    expect(result).not.toContain('\n\n\n');
    expect(result).toContain('Signed-off-by');
    expect(result).toContain(GIT_TRAILERS.AGENT_ID);
  });

  it('trims trailing whitespace from message', () => {
    const result = formatCommitWithAgent({ message: 'feat: add   \n\n  ', agent: testAgent });
    // Should not have trailing spaces before trailers
    expect(result).toMatch(/feat: add\n\n/);
  });
});

// ============================================================================
// Git-calling functions — real temp repo tests
// ============================================================================

describe('git command composition (real repo)', () => {
  let tmpDir: string;

  function git(...args: string[]) {
    return execFileSync('git', args, { cwd: tmpDir, encoding: 'utf-8' }).trim();
  }

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'git-agent-test-'));
    git('init');
    git('config', 'user.email', 'test@test.com');
    git('config', 'user.name', 'Test');

    // Create a commit with agent trailers using -F to ensure proper newlines
    writeFileSync(join(tmpDir, 'file.ts'), 'content');
    git('add', '.');
    const commitMsg = `feat: add file\n\n${GIT_TRAILERS.AGENT_ID}: agent-abc\n${GIT_TRAILERS.AGENT_TYPE}: claude`;
    writeFileSync(join(tmpDir, '.commit-msg'), commitMsg);
    git('commit', '-F', join(tmpDir, '.commit-msg'));
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  describe('getCommitAgentInfo', () => {
    it('extracts agent info from HEAD', () => {
      const info = getCommitAgentInfo('HEAD', tmpDir);

      expect(info).not.toBeNull();
      expect(info!.agentId).toBe('agent-abc');
      expect(info!.agentType).toBe('claude');
      expect(info!.isAiGenerated).toBe(true);
    });

    it('returns null for invalid ref', () => {
      const info = getCommitAgentInfo('nonexistent-ref-12345', tmpDir);
      expect(info).toBeNull();
    });

    it('handles ref with special characters gracefully', () => {
      // execFileSync passes this as a single array element — no shell injection
      const info = getCommitAgentInfo('HEAD; echo pwned', tmpDir);
      expect(info).toBeNull(); // Invalid ref, returns null
    });
  });

  describe('getRecentCommitsAgentInfo', () => {
    it('returns agent info for recent commits', () => {
      const results = getRecentCommitsAgentInfo(5, tmpDir);

      expect(results.length).toBeGreaterThan(0);
      expect(results[0].info.agentId).toBe('agent-abc');
    });

    it('handles count of 0', () => {
      // git log -0 returns nothing
      const results = getRecentCommitsAgentInfo(0, tmpDir);
      expect(results).toEqual([]);
    });
  });

  describe('getAgentContributions', () => {
    it('aggregates contributions from commits', () => {
      const contributions = getAgentContributions(undefined, tmpDir);

      expect(contributions.size).toBeGreaterThan(0);
      const entry = contributions.get('agent-abc');
      expect(entry).toBeDefined();
      expect(entry!.commitCount).toBe(1);
      expect(entry!.agentType).toBe('claude');
    });

    it('handles invalid sinceRef gracefully', () => {
      const contributions = getAgentContributions('nonexistent-ref', tmpDir);
      expect(contributions).toEqual(new Map());
    });
  });

  describe('getAiCommitPercentage', () => {
    it('calculates AI commit percentage', () => {
      const pct = getAiCommitPercentage(undefined, tmpDir);
      // All commits have agent trailers, so should be 100%
      expect(pct).toBe(100);
    });

    it('returns 0 for invalid ref', () => {
      const pct = getAiCommitPercentage('nonexistent', tmpDir);
      expect(pct).toBe(0);
    });
  });
});
