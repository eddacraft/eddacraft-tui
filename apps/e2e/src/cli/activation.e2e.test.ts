/**
 * Activation Golden Path — E2E Tests
 *
 * Covers the ACTMO MCP-optional activation matrix through the real Rust CLI.
 * Every case uses isolated HOME / runtime directories so it cannot read or
 * modify a developer's real editor config or save-time daemon state.
 */

import { afterEach, describe, expect, it } from 'vitest';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { cliBinaryAvailable, runCli } from '../helpers/cli-runner.js';
import { createE2EWorkspace, type E2EWorkspace } from '../helpers/workspace.js';

const describeCli = cliBinaryAvailable() ? describe : describe.skip;

interface IsolatedHome {
  root: string;
  env: Record<string, string>;
}

const workspaces: E2EWorkspace[] = [];
const homes: string[] = [];

function workspace(): E2EWorkspace {
  const ws = createE2EWorkspace({
    withAnvilrc: false,
    withGit: true,
    files: {
      'src/index.ts': 'export const value = 1;\n',
    },
  });
  workspaces.push(ws);
  return ws;
}

function isolatedHome(): IsolatedHome {
  const root = mkdtempSync(join(tmpdir(), 'anvil-e2e-home-'));
  homes.push(root);
  return {
    root,
    env: {
      HOME: root,
      USERPROFILE: root,
      XDG_CONFIG_HOME: join(root, 'xdg'),
      XDG_RUNTIME_DIR: join(root, 'runtime'),
      ANVIL_DEV: '1',
      ANVIL_SKIP_WELCOME: '1',
      ANVIL_NO_PROMPT: '1',
    },
  };
}

function readJson(path: string): unknown {
  return JSON.parse(readFileSync(path, 'utf-8'));
}

afterEach(() => {
  while (workspaces.length > 0) {
    workspaces.pop()?.cleanup();
  }
  while (homes.length > 0) {
    const home = homes.pop();
    if (home && existsSync(home)) {
      rmSync(home, { recursive: true, force: true });
    }
  }
});

describeCli('Activation golden path', () => {
  it('installs MCP config and Claude allow rules by default', async () => {
    const ws = workspace();
    const home = isolatedHome();

    const result = await runCli(['--no-tui', 'start', '--no-daemon'], {
      cwd: ws.root,
      env: home.env,
      timeout: 30_000,
    });

    expect(result.exitCode, result.stderr).toBe(0);
    expect(result.stdout).toContain('state: ready_restart_required');
    expect(result.stdout).not.toContain('state: protecting');
    expect(existsSync(join(ws.root, '.anvilrc'))).toBe(true);
    expect(existsSync(join(home.root, '.cursor/mcp.json'))).toBe(true);
    expect(existsSync(join(home.root, '.claude.json'))).toBe(true);

    const settings = readJson(join(home.root, '.claude/settings.json')) as {
      permissions?: { allow?: unknown[] };
    };
    expect(settings.permissions?.allow).toContain('mcp__anvil__*');
  });

  it('skips MCP writes under --no-mcp while still activating the spine', async () => {
    const ws = workspace();
    const home = isolatedHome();

    const result = await runCli(['--no-tui', 'start', '--no-daemon', '--no-mcp'], {
      cwd: ws.root,
      env: home.env,
      timeout: 30_000,
    });

    expect(result.exitCode, result.stderr).toBe(0);
    expect(result.stdout).toContain('install: skipped');
    expect(result.stdout).not.toContain('state: protecting');
    expect(existsSync(join(ws.root, '.anvilrc'))).toBe(true);
    expect(existsSync(join(ws.root, '.git/hooks/pre-commit'))).toBe(true);
    expect(existsSync(join(ws.root, '.git/hooks/pre-push'))).toBe(true);
    expect(existsSync(join(home.root, '.cursor/mcp.json'))).toBe(false);
    expect(existsSync(join(home.root, '.claude.json'))).toBe(false);
    expect(existsSync(join(home.root, '.claude/settings.json'))).toBe(false);
  });

  it('terminates with daemon repair guidance when MCP is wired but daemon evidence is absent', async () => {
    const ws = workspace();
    const home = isolatedHome();

    const first = await runCli(['--no-tui', 'start', '--no-daemon'], {
      cwd: ws.root,
      env: home.env,
      timeout: 30_000,
    });
    expect(first.exitCode, first.stderr).toBe(0);

    const verify = await runCli(['--no-tui', 'start', '--verify'], {
      cwd: ws.root,
      env: home.env,
      timeout: 30_000,
    });

    expect(verify.exitCode, verify.stderr).toBe(0);
    expect(verify.stdout).toContain('state: ready_restart_required');
    expect(verify.stdout.toLowerCase()).toMatch(/daemon|intercept/);
    expect(verify.stdout).not.toContain('state: protecting');
  });

  // ACTTUI-001 (ADR-103): the opt-in activation TUI must stay on the plain
  // path when the session is not genuinely interactive. The e2e harness pipes
  // stdout (not a TTY), so even with the rollout flag set the run must emit the
  // deterministic plain verdict and never switch to the alternate screen.
  it('stays on the plain path under a non-TTY session even when the TUI opt-in is set', async () => {
    const ws = workspace();
    const home = isolatedHome();

    const result = await runCli(['start', '--verify'], {
      cwd: ws.root,
      env: { ...home.env, ANVIL_ACTIVATION_TUI: '1' },
      timeout: 30_000,
    });

    expect(result.exitCode, result.stderr).toBe(0);
    expect(result.stdout).toContain('ACTIVATION');
    expect(result.stdout).toMatch(/state: /);
    // No alternate-screen enter / raw-mode cursor escapes leaked onto stdout.
    expect(result.stdout).not.toContain('\u001b[?1049h');
    expect(result.stdout).not.toContain('\x1b[?1049h');
  });

  // ACTTUI-001: `--no-tui` is an explicit opt-out that wins over the rollout
  // opt-in flag, mirroring the global plain-output contract.
  it('honours --no-tui as an opt-out even with the TUI opt-in flag set', async () => {
    const ws = workspace();
    const home = isolatedHome();

    const result = await runCli(['--no-tui', 'start', '--no-daemon'], {
      cwd: ws.root,
      env: { ...home.env, ANVIL_ACTIVATION_TUI: '1' },
      timeout: 30_000,
    });

    expect(result.exitCode, result.stderr).toBe(0);
    expect(result.stdout).toContain('state: ready_restart_required');
    expect(result.stdout).not.toContain('\u001b[?1049h');
  });
});
