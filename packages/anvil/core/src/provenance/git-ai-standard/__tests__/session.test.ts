import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  createAgentId,
  createExplicitAgent,
  detectCurrentAgent,
  generateSessionHash,
} from '../session.js';

const FAKE_COPILOT_TOKEN = 'ghu_testfixture000000000000000000000000';

const SESSION_ENV_VARS = [
  'CLAUDE_SESSION_ID',
  'CLAUDE_CODE_SESSION',
  'CLAUDE_MODEL',
  'CURSOR_SESSION',
  'CURSOR_SESSION_ID',
  'CURSOR_MODEL',
  'GITHUB_COPILOT_TOKEN',
  'COPILOT_SESSION',
  'AWS_CODEWHISPERER_SESSION',
  'TABNINE_SESSION',
] as const;

function isolateAgentEnv(overrides: Record<string, string> = {}): void {
  for (const name of SESSION_ENV_VARS) {
    vi.stubEnv(name, '');
  }
  for (const [name, value] of Object.entries(overrides)) {
    vi.stubEnv(name, value);
  }
}

describe('detectCurrentAgent credential handling', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('does not use GITHUB_COPILOT_TOKEN as a session identifier', () => {
    isolateAgentEnv({ GITHUB_COPILOT_TOKEN: FAKE_COPILOT_TOKEN });

    const agent = detectCurrentAgent();

    expect(agent).toBeNull();
    expect(JSON.stringify(agent)).not.toContain(FAKE_COPILOT_TOKEN);
  });

  it('uses a non-secret COPILOT_SESSION id and ignores a sibling Copilot token', () => {
    isolateAgentEnv({
      GITHUB_COPILOT_TOKEN: FAKE_COPILOT_TOKEN,
      COPILOT_SESSION: 'copilot-session-42',
    });

    const agent = detectCurrentAgent();

    expect(agent).toMatchObject({
      tool: 'copilot',
      id: 'copilot-session-42',
    });
    expect(JSON.stringify(agent)).not.toContain(FAKE_COPILOT_TOKEN);
  });

  it('skips a COPILOT_SESSION value that is itself token-shaped', () => {
    isolateAgentEnv({ COPILOT_SESSION: FAKE_COPILOT_TOKEN });

    const agent = detectCurrentAgent();

    expect(agent).toBeNull();
    expect(JSON.stringify(agent)).not.toContain(FAKE_COPILOT_TOKEN);
  });

  it('still detects legitimate Claude session ids', () => {
    isolateAgentEnv({ CLAUDE_SESSION_ID: 'session-123' });

    const agent = detectCurrentAgent();

    expect(agent).toMatchObject({
      tool: 'claude-code',
      id: 'session-123',
    });
  });
});

describe('createAgentId credential handling', () => {
  it('redacts token-shaped conversation ids', () => {
    const agent = createAgentId({
      tool: 'copilot',
      conversationId: FAKE_COPILOT_TOKEN,
    });

    expect(agent.id).not.toBe(FAKE_COPILOT_TOKEN);
    expect(JSON.stringify(agent)).not.toContain(FAKE_COPILOT_TOKEN);
    expect(agent.id).toBe('[redacted]');
  });

  it('keeps ordinary conversation ids', () => {
    const agent = createAgentId({
      tool: 'claude-code',
      conversationId: 'session-123',
    });

    expect(agent.id).toBe('session-123');
  });
});

describe('createExplicitAgent credential handling', () => {
  it('redacts token-shaped explicit session ids', () => {
    const agent = createExplicitAgent('copilot', FAKE_COPILOT_TOKEN);

    expect(agent.id).toBe('[redacted]');
    expect(JSON.stringify(agent)).not.toContain(FAKE_COPILOT_TOKEN);
  });
});

describe('generateSessionHash', () => {
  it('remains stable for non-secret session ids', () => {
    expect(generateSessionHash('claude-code', 'session-123')).toBe(
      generateSessionHash('claude-code', 'session-123')
    );
  });
});
