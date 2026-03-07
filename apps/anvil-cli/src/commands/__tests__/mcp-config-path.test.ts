// @vitest-environment node
import { describe, it, expect, vi, afterEach } from 'vitest';
import { mkdtempSync, mkdirSync, symlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createMcpConfigCommand } from '../mcp-config.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

afterEach(async () => {
  vi.restoreAllMocks();
});

describe('mcp-config --write outside-workspace check (M-6)', () => {
  it('rejects unknown target', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await expect(command.parseAsync(['-t', 'unknown-editor'], { from: 'user' })).rejects.toThrow(
      'Unknown target'
    );
  });

  it('rejects invalid transport', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await expect(
      command.parseAsync(['-t', 'cursor', '--transport', 'grpc'], { from: 'user' })
    ).rejects.toThrow('Unknown transport');
  });

  it('rejects invalid port', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await expect(
      command.parseAsync(['-t', 'cursor', '--port', '99999'], { from: 'user' })
    ).rejects.toThrow('--port must be an integer between 1 and 65535');
  });

  it('prints config JSON to stdout without --write', async () => {
    const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await command.parseAsync(['-t', 'cursor'], { from: 'user' });

    const output = logSpy.mock.calls.map((c) => c[0]).join('\n');
    const parsed = JSON.parse(output);
    expect(parsed.mcpServers).toBeDefined();
    expect(parsed.mcpServers.anvil).toBeDefined();
  });

  it('detects outside-workspace for windsurf ~ path (non-TTY)', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await expect(
      command.parseAsync(['-t', 'windsurf', '--write'], { from: 'user' })
    ).rejects.toThrow('outside workspace');
  });

  it('detects symlink at final path component pointing outside workspace', async () => {
    // Create a temp "workspace" with .cursor/mcp.json symlinked to an outside file
    const workspace = mkdtempSync(join(tmpdir(), 'anvil-mcp-test-ws-'));
    const outsideDir = mkdtempSync(join(tmpdir(), 'anvil-mcp-test-outside-'));
    const outsideFile = join(outsideDir, 'mcp.json');
    writeFileSync(outsideFile, '{}');

    mkdirSync(join(workspace, '.cursor'), { recursive: true });
    symlinkSync(outsideFile, join(workspace, '.cursor', 'mcp.json'));

    const originalCwd = process.cwd();
    try {
      process.chdir(workspace);

      vi.spyOn(console, 'error').mockImplementation(() => {});

      const command = createMcpConfigCommand();
      await expect(
        command.parseAsync(['-t', 'cursor', '--write'], { from: 'user' })
      ).rejects.toThrow('outside workspace');
    } finally {
      process.chdir(originalCwd);
      await safeCleanup(workspace);
      await safeCleanup(outsideDir);
    }
  });

  it('detects symlink at parent directory component pointing outside workspace', async () => {
    // Create a temp "workspace" where .cursor/ itself is a symlink to an outside directory
    const workspace = mkdtempSync(join(tmpdir(), 'anvil-mcp-test-ws-'));
    const outsideDir = mkdtempSync(join(tmpdir(), 'anvil-mcp-test-outside-'));
    writeFileSync(join(outsideDir, 'mcp.json'), '{}');

    // Symlink .cursor/ -> outsideDir/ (parent directory is the symlink)
    symlinkSync(outsideDir, join(workspace, '.cursor'));

    const originalCwd = process.cwd();
    try {
      process.chdir(workspace);

      vi.spyOn(console, 'error').mockImplementation(() => {});

      const command = createMcpConfigCommand();
      await expect(
        command.parseAsync(['-t', 'cursor', '--write'], { from: 'user' })
      ).rejects.toThrow('outside workspace');
    } finally {
      process.chdir(originalCwd);
      await safeCleanup(workspace);
      await safeCleanup(outsideDir);
    }
  });

  it('bypasses outside-workspace check when --yes is passed', async () => {
    const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    // Redirect HOME to a temp directory so the test never touches the real filesystem.
    // We write the config to the temp dir so --write succeeds without hitting the real HOME.
    // Using process.env.HOME instead of vi.spyOn(os, 'homedir') because ESM module
    // namespace is not configurable on macOS/Node 20.
    const tempHome = mkdtempSync(join(tmpdir(), 'anvil-mcp-config-'));
    const targetDir = join(tempHome, '.codeium', 'windsurf');
    mkdirSync(targetDir, { recursive: true });

    // Save and override all homedir-related env vars (HOME on Unix, USERPROFILE on Windows)
    const savedEnv = {
      HOME: process.env.HOME,
      USERPROFILE: process.env.USERPROFILE,
    };
    process.env.HOME = tempHome;
    process.env.USERPROFILE = tempHome;

    try {
      const command = createMcpConfigCommand();
      // windsurf writes to ~ which is outside workspace — --yes skips the prompt
      await command.parseAsync(['-t', 'windsurf', '--write', '--yes'], { from: 'user' });

      const output = logSpy.mock.calls.map((c) => c[0]).join('\n');
      // Should succeed and attempt to write rather than throwing
      expect(output).toContain('Wrote');
    } finally {
      // Restore or delete env vars — assigning undefined coerces to "undefined"
      for (const [key, val] of Object.entries(savedEnv)) {
        if (val === undefined) {
          delete process.env[key];
        } else {
          process.env[key] = val;
        }
      }
      await safeCleanup(tempHome);
    }
  });

  it('generates correct stdio config for each target', async () => {
    const targets = ['claude-code', 'cursor', 'windsurf', 'vscode'] as const;

    for (const target of targets) {
      const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const command = createMcpConfigCommand();
      await command.parseAsync(['-t', target], { from: 'user' });

      const output = logSpy.mock.calls.map((c) => c[0]).join('\n');
      const parsed = JSON.parse(output);

      const serverKey = target === 'vscode' ? 'servers' : 'mcpServers';
      expect(parsed[serverKey]).toBeDefined();
      expect(parsed[serverKey].anvil).toBeDefined();

      logSpy.mockRestore();
    }
  });
});
