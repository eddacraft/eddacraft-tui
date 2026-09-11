import { afterEach, describe, expect, it, vi } from 'vitest';
import { execSync, spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { safeCleanup } from '../../../../../../../tools/test-utils/safe-cleanup.js';
import { createAuthorshipLog } from '../../collector.js';
import { serializeAuthorshipLog } from '../serializer.js';
import { writeAuthorshipNote, readAuthorshipNote, SCHEMA_VERSION } from '../index.js';
import type { AuthorshipLog } from '../types.js';

const FAKE_COPILOT_TOKEN = 'ghu_testfixture000000000000000000000000';
const COMMIT_SHA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

const gitAvailable = (() => {
  const result = spawnSync('git', ['--version'], { stdio: 'pipe' });
  return !result.error && result.status === 0;
})();

function isolateAgentEnv(overrides: Record<string, string> = {}): void {
  const names = [
    'CLAUDE_SESSION_ID',
    'CLAUDE_CODE_SESSION',
    'CURSOR_SESSION',
    'CURSOR_SESSION_ID',
    'GITHUB_COPILOT_TOKEN',
    'COPILOT_SESSION',
    'AWS_CODEWHISPERER_SESSION',
    'TABNINE_SESSION',
  ];
  for (const name of names) {
    vi.stubEnv(name, '');
  }
  for (const [name, value] of Object.entries(overrides)) {
    vi.stubEnv(name, value);
  }
}

function makeLog(agentId: string): AuthorshipLog {
  return {
    attestations: {
      'src/feature.ts': [{ sessionHash: 'a1b2c3d4e5f67890', lineRanges: '1-10' }],
    },
    metadata: {
      schema_version: SCHEMA_VERSION,
      base_commit_sha: COMMIT_SHA,
      prompts: {
        a1b2c3d4e5f67890: {
          agent_id: {
            tool: 'copilot',
            id: agentId,
          },
          messages: [{ type: 'user', text: 'Implement feature' }],
          total_additions: 10,
          total_deletions: 0,
          accepted_lines: 10,
          overridden_lines: 0,
        },
      },
    },
  };
}

describe('authorship serialisation credential redaction', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('does not serialise Copilot-like tokens in agent ids', () => {
    const output = serializeAuthorshipLog(makeLog(FAKE_COPILOT_TOKEN));

    expect(output).not.toContain(FAKE_COPILOT_TOKEN);
    expect(output).toContain('[redacted]');
    expect(output).toContain('"copilot"');
  });

  it('keeps legitimate non-secret agent ids', () => {
    const output = serializeAuthorshipLog(makeLog('copilot-session-42'));

    expect(output).toContain('copilot-session-42');
    expect(output).not.toContain('[redacted]');
  });

  it('does not put GITHUB_COPILOT_TOKEN into an authorship log', () => {
    isolateAgentEnv({ GITHUB_COPILOT_TOKEN: FAKE_COPILOT_TOKEN });

    const log = createAuthorshipLog({
      commitSha: COMMIT_SHA,
      fileLineMap: { 'src/feature.ts': '1-10' },
      messages: [{ type: 'user', text: 'hello' }],
    });

    expect(log).toBeNull();
  });

  it('records a non-secret Copilot session without leaking a sibling token', () => {
    isolateAgentEnv({
      GITHUB_COPILOT_TOKEN: FAKE_COPILOT_TOKEN,
      COPILOT_SESSION: 'copilot-session-42',
    });

    const log = createAuthorshipLog({
      commitSha: COMMIT_SHA,
      fileLineMap: { 'src/feature.ts': '1-10' },
      messages: [{ type: 'user', text: 'hello' }],
    });

    expect(log).not.toBeNull();
    const serialised = serializeAuthorshipLog(log!);
    expect(serialised).toContain('copilot-session-42');
    expect(serialised).not.toContain(FAKE_COPILOT_TOKEN);
  });
});

describe('git notes credential redaction', { timeout: 30_000 }, () => {
  const itIfGit = gitAvailable ? it : it.skip;

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  itIfGit('does not persist Copilot-like tokens in Git notes payloads', async () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'anvil-notes-redact-'));
    try {
      execSync('git init', { cwd: tempDir, stdio: 'pipe' });
      execSync('git config user.email "test@example.com"', { cwd: tempDir, stdio: 'pipe' });
      execSync('git config user.name "Test User"', { cwd: tempDir, stdio: 'pipe' });
      execSync('git config commit.gpgsign false', { cwd: tempDir, stdio: 'pipe' });
      writeFileSync(join(tempDir, 'readme.txt'), 'hello');
      execSync('git add .', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "Initial commit"', { cwd: tempDir, stdio: 'pipe' });
      const commitSha = execSync('git rev-parse HEAD', { cwd: tempDir, encoding: 'utf8' }).trim();

      const log: AuthorshipLog = {
        ...makeLog(FAKE_COPILOT_TOKEN),
        metadata: {
          ...makeLog(FAKE_COPILOT_TOKEN).metadata,
          base_commit_sha: commitSha,
        },
      };

      await writeAuthorshipNote(commitSha, log, tempDir);
      const raw = execSync(`git notes --ref=refs/notes/ai show ${commitSha}`, {
        cwd: tempDir,
        encoding: 'utf8',
      });
      expect(raw).not.toContain(FAKE_COPILOT_TOKEN);
      expect(raw).toContain('[redacted]');

      const parsed = await readAuthorshipNote(commitSha, tempDir);
      expect(JSON.stringify(parsed)).not.toContain(FAKE_COPILOT_TOKEN);
      expect(parsed?.metadata.prompts['a1b2c3d4e5f67890'].agent_id.id).toBe('[redacted]');
    } finally {
      await safeCleanup(tempDir);
    }
  });
});
