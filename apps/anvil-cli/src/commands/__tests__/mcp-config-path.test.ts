// @vitest-environment node
import { describe, it, expect, vi, afterEach } from 'vitest';
import { createMcpConfigCommand } from '../mcp-config.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('mcp-config --write outside-workspace check (M-6)', () => {
  it('rejects unknown target', async () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(() => {
      throw new Error('process.exit');
    });
    const stderrSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await expect(command.parseAsync(['-t', 'unknown-editor'], { from: 'user' })).rejects.toThrow(
      'process.exit'
    );

    const allStderr = stderrSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(allStderr).toContain('Unknown target');
    expect(exitSpy).toHaveBeenCalledWith(1);
  });

  it('rejects invalid transport', async () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(() => {
      throw new Error('process.exit');
    });
    const stderrSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await expect(
      command.parseAsync(['-t', 'cursor', '--transport', 'grpc'], { from: 'user' })
    ).rejects.toThrow('process.exit');

    const allStderr = stderrSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(allStderr).toContain('Unknown transport');
    expect(exitSpy).toHaveBeenCalledWith(1);
  });

  it('rejects invalid port', async () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(() => {
      throw new Error('process.exit');
    });
    const stderrSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const command = createMcpConfigCommand();
    await expect(
      command.parseAsync(['-t', 'cursor', '--port', '99999'], { from: 'user' })
    ).rejects.toThrow('process.exit');

    const allStderr = stderrSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(allStderr).toContain('Invalid port');
    expect(exitSpy).toHaveBeenCalledWith(1);
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

  it('uses realpathSync for symlink-safe outside-workspace detection', async () => {
    // Verify the code imports realpathSync — we test by checking that the
    // module can handle the --write flag with a target that resolves outside
    // the workspace (windsurf writes to ~/.codeium/ which is always outside).
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(() => {
      throw new Error('process.exit');
    });
    const stderrSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // windsurf config path is ~/.codeium/windsurf/mcp_config.json — outside workspace
    const command = createMcpConfigCommand();
    await expect(
      command.parseAsync(['-t', 'windsurf', '--write'], { from: 'user' })
    ).rejects.toThrow('process.exit');

    // In non-TTY test environment, it should error about needing --yes
    const allStderr = stderrSpy.mock.calls.map((c) => c[0]).join('\n');
    expect(allStderr).toContain('outside workspace');
    expect(allStderr).toContain('--yes');
    expect(exitSpy).toHaveBeenCalledWith(1);
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
