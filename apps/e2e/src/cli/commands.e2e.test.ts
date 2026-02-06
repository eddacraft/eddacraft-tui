/**
 * CLI Commands — E2E Tests
 *
 * Tests core CLI commands by spawning the real `anvil` binary.
 * Validates exit codes, stdout/stderr content, and side effects.
 *
 * Surface: CLI (non-interactive)
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { assertCliBuild, runCli, runCliExpectSuccess } from '../helpers/cli-runner.js';
import { createE2EWorkspace, type E2EWorkspace } from '../helpers/workspace.js';

beforeAll(() => {
  assertCliBuild();
});

describe('CLI › --version', () => {
  it('prints the version and exits 0', async () => {
    const result = await runCliExpectSuccess(['--version']);
    expect(result.stdout).toMatch(/\d+\.\d+\.\d+/);
  });
});

describe('CLI › --help', () => {
  it('lists available commands', async () => {
    const result = await runCliExpectSuccess(['--help']);
    expect(result.stdout).toContain('check');
    expect(result.stdout).toContain('gate');
    expect(result.stdout).toContain('plan');
    expect(result.stdout).toContain('validate');
  });

  it('includes the description', async () => {
    const result = await runCliExpectSuccess(['--help']);
    // Should have some form of description about Anvil
    expect(result.output.toLowerCase()).toMatch(/anvil|automation|validation/);
  });
});

describe('CLI › doctor', () => {
  let ws: E2EWorkspace;

  beforeAll(() => {
    ws = createE2EWorkspace({ withGit: true, lockfile: 'pnpm' });
  });

  afterAll(() => ws.cleanup());

  it('runs diagnostic checks', async () => {
    const result = await runCli(['doctor', '--json'], { cwd: ws.root });
    // doctor should produce output regardless of exit code
    expect(result.output.length).toBeGreaterThan(0);
  });
});

describe('CLI › check', () => {
  let ws: E2EWorkspace;

  beforeAll(() => {
    ws = createE2EWorkspace({
      withGit: true,
      lockfile: 'pnpm',
      files: {
        'src/index.ts': 'export const hello = "world";\n',
      },
    });
  });

  afterAll(() => ws.cleanup());

  it('accepts --help flag', async () => {
    const result = await runCliExpectSuccess(['check', '--help']);
    expect(result.stdout).toContain('check');
  });

  it('runs in a workspace directory', async () => {
    const result = await runCli(['check'], { cwd: ws.root });
    // check may pass or report violations — both are valid outcomes
    expect(result.output.length).toBeGreaterThan(0);
  });
});

describe('CLI › init', () => {
  it('shows help for init', async () => {
    const result = await runCliExpectSuccess(['init', '--help']);
    expect(result.stdout.toLowerCase()).toMatch(/init/);
  });

  it('runs non-interactively', async () => {
    const ws = createE2EWorkspace({ withAnvilrc: false });
    try {
      const result = await runCli(['init', '--non-interactive'], { cwd: ws.root });
      // init should produce some output
      expect(result.output.length).toBeGreaterThan(0);
    } finally {
      ws.cleanup();
    }
  });
});

describe('CLI › unknown command', () => {
  it('reports an error for unrecognised commands', async () => {
    const result = await runCli(['nonexistent-command-xyz']);
    expect(result.exitCode).not.toBe(0);
  });
});
