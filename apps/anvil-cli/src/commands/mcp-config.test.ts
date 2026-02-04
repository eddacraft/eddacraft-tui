import { describe, it, expect, vi, afterEach } from 'vitest';
import { createMcpConfigCommand } from './mcp-config.js';

describe('mcp-config command', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', () => {
    const command = createMcpConfigCommand();

    expect(command.name()).toBe('mcp-config');
    expect(command.description()).toContain('MCP configuration');
  });

  it('should have required --target option', () => {
    const command = createMcpConfigCommand();
    const targetOpt = command.options.find((o) => o.long === '--target');

    expect(targetOpt).toBeDefined();
    expect(targetOpt?.mandatory).toBe(true);
  });

  it('should have --transport option with default "stdio"', () => {
    const command = createMcpConfigCommand();
    const transportOpt = command.options.find((o) => o.long === '--transport');

    expect(transportOpt).toBeDefined();
    expect(transportOpt?.defaultValue).toBe('stdio');
  });

  it('should have --port option with default "3000"', () => {
    const command = createMcpConfigCommand();
    const portOpt = command.options.find((o) => o.long === '--port');

    expect(portOpt).toBeDefined();
    expect(portOpt?.defaultValue).toBe('3000');
  });

  it('should have --write option', () => {
    const command = createMcpConfigCommand();
    const writeOpt = command.options.find((o) => o.long === '--write');

    expect(writeOpt).toBeDefined();
  });
});
