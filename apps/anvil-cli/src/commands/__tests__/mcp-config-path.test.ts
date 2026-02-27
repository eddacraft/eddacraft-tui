// @vitest-environment node
import { describe, it, expect, vi, afterEach } from 'vitest';
import { mkdtempSync, mkdirSync, symlinkSync, writeFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createMcpConfigCommand } from '../mcp-config.js';

afterEach(() => {
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
    ).rejects.toThrow('Invalid port');
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
      rmSync(workspace, { recursive: true, force: true });
      rmSync(outsideDir, { recursive: true, force: true });
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
      rmSync(workspace, { recursive: true, force: true });
      rmSync(outsideDir, { recursive: true, force: true });
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
