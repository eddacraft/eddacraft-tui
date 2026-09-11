import { describe, expect, it } from 'vitest';
import {
  isCredentialEnvVarName,
  isCredentialShapedValue,
  redactCredentialShapedValue,
  stripGitRemoteUserinfo,
} from './credential-redaction.js';

const FAKE_COPILOT_TOKEN = 'ghu_testfixture000000000000000000000000';
const FAKE_GITHUB_PAT = 'github_pat_testfixture000000000000000000000000000000';
const FAKE_CLASSIC_HEX = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

describe('isCredentialEnvVarName', () => {
  it('treats Copilot token and other credential env names as unsafe session sources', () => {
    expect(isCredentialEnvVarName('GITHUB_COPILOT_TOKEN')).toBe(true);
    expect(isCredentialEnvVarName('ANTHROPIC_API_KEY')).toBe(true);
    expect(isCredentialEnvVarName('OPENAI_API_KEY')).toBe(true);
    expect(isCredentialEnvVarName('GH_TOKEN')).toBe(true);
    expect(isCredentialEnvVarName('MY_SECRET')).toBe(true);
    expect(isCredentialEnvVarName('DB_PASSWORD')).toBe(true);
  });

  it('allows non-secret session env names', () => {
    expect(isCredentialEnvVarName('COPILOT_SESSION')).toBe(false);
    expect(isCredentialEnvVarName('CLAUDE_SESSION_ID')).toBe(false);
    expect(isCredentialEnvVarName('CURSOR_SESSION')).toBe(false);
    expect(isCredentialEnvVarName('AWS_CODEWHISPERER_SESSION')).toBe(false);
  });
});

describe('isCredentialShapedValue', () => {
  it('detects Copilot-like and GitHub token prefixes', () => {
    expect(isCredentialShapedValue(FAKE_COPILOT_TOKEN)).toBe(true);
    expect(isCredentialShapedValue(FAKE_GITHUB_PAT)).toBe(true);
    expect(isCredentialShapedValue('ghp_testfixture000000000000000000000000')).toBe(true);
    expect(isCredentialShapedValue(FAKE_CLASSIC_HEX)).toBe(true);
    expect(isCredentialShapedValue('eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.signature')).toBe(
      true
    );
  });

  it('does not treat legitimate session identifiers as credentials', () => {
    expect(isCredentialShapedValue('session-123')).toBe(false);
    expect(isCredentialShapedValue('copilot-session-42')).toBe(false);
    expect(isCredentialShapedValue('1710000000000-abcd1234')).toBe(false);
    expect(isCredentialShapedValue('550e8400-e29b-41d4-a716-446655440000')).toBe(false);
  });
});

describe('redactCredentialShapedValue', () => {
  it('replaces token-shaped values and leaves ordinary ids intact', () => {
    expect(redactCredentialShapedValue(FAKE_COPILOT_TOKEN)).toBe('[redacted]');
    expect(redactCredentialShapedValue('session-123')).toBe('session-123');
  });
});

describe('stripGitRemoteUserinfo', () => {
  it('strips user:password and token userinfo from HTTPS remotes', () => {
    expect(stripGitRemoteUserinfo('https://user:pass@github.com/org/repo.git')).toBe(
      'https://github.com/org/repo.git'
    );
    expect(stripGitRemoteUserinfo(`https://${FAKE_COPILOT_TOKEN}@github.com/org/repo.git`)).toBe(
      'https://github.com/org/repo.git'
    );
    expect(
      stripGitRemoteUserinfo('https://x-access-token:ghs_testfixture@github.com/org/repo.git')
    ).toBe('https://github.com/org/repo.git');
  });

  it('leaves clean remotes and scp-style SSH remotes unchanged', () => {
    expect(stripGitRemoteUserinfo('https://github.com/org/repo.git')).toBe(
      'https://github.com/org/repo.git'
    );
    expect(stripGitRemoteUserinfo('git@github.com:org/repo.git')).toBe(
      'git@github.com:org/repo.git'
    );
    expect(stripGitRemoteUserinfo('ssh://git@github.com/org/repo.git')).toBe(
      'ssh://git@github.com/org/repo.git'
    );
  });
});
