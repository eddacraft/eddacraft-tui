import { describe, it, expect } from 'vitest';
import {
  generateMcpConfig,
  SUPPORTED_TARGETS,
  generateClaudeCodeConfig,
  generateCursorConfig,
  generateWindsurfConfig,
  generateVscodeConfig,
} from './index.js';
import type { McpConfigTarget } from './index.js';

describe('SUPPORTED_TARGETS', () => {
  it('includes all 4 targets', () => {
    expect(SUPPORTED_TARGETS).toHaveLength(4);
    expect(SUPPORTED_TARGETS).toContain('claude-code');
    expect(SUPPORTED_TARGETS).toContain('cursor');
    expect(SUPPORTED_TARGETS).toContain('windsurf');
    expect(SUPPORTED_TARGETS).toContain('vscode');
  });
});

describe('generateMcpConfig', () => {
  it.each<McpConfigTarget>(['claude-code', 'cursor', 'windsurf', 'vscode'])(
    'generates valid config for %s with default options',
    (target) => {
      const config = generateMcpConfig(target);
      expect(config.target).toBe(target);
      expect(config.configPath).toBeTruthy();
      expect(config.content).toBeDefined();
      expect(typeof config.content).toBe('object');
    }
  );

  it.each<McpConfigTarget>(['claude-code', 'cursor', 'windsurf', 'vscode'])(
    'all configs include "anvil" server name for %s',
    (target) => {
      const config = generateMcpConfig(target);
      const json = JSON.stringify(config.content);
      expect(json).toContain('anvil');
    }
  );
});

describe('generateClaudeCodeConfig', () => {
  it('generates stdio config with correct structure', () => {
    const config = generateClaudeCodeConfig();
    expect(config.target).toBe('claude-code');
    expect(config.configPath).toBe('.claude/mcp.json');

    const servers = config.content['mcpServers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']).toBeDefined();
    expect(servers['anvil']['command']).toBe('npx');
    expect(servers['anvil']['args']).toEqual(['@eddacraft/anvil-mcp-server']);
    expect(servers['anvil']['env']).toEqual({});
  });

  it('generates HTTP config with url format', () => {
    const config = generateClaudeCodeConfig({ transport: 'http' });
    expect(config.configPath).toBe('.claude/mcp.json');

    const servers = config.content['mcpServers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']).toBeDefined();
    expect(servers['anvil']['url']).toBe('http://localhost:3000/mcp');
    expect(servers['anvil']).not.toHaveProperty('command');
  });

  it('uses custom port for HTTP config', () => {
    const config = generateClaudeCodeConfig({ transport: 'http', port: 8080 });

    const servers = config.content['mcpServers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']['url']).toBe('http://localhost:8080/mcp');
  });
});

describe('generateCursorConfig', () => {
  it('generates stdio config with correct structure', () => {
    const config = generateCursorConfig();
    expect(config.target).toBe('cursor');
    expect(config.configPath).toBe('.cursor/mcp.json');

    const servers = config.content['mcpServers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']).toBeDefined();
    expect(servers['anvil']['command']).toBe('npx');
    expect(servers['anvil']['args']).toEqual(['@eddacraft/anvil-mcp-server']);
  });

  it('generates HTTP config with url format', () => {
    const config = generateCursorConfig({ transport: 'http', port: 4000 });

    const servers = config.content['mcpServers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']['url']).toBe('http://localhost:4000/mcp');
  });
});

describe('generateWindsurfConfig', () => {
  it('generates stdio config with correct structure', () => {
    const config = generateWindsurfConfig();
    expect(config.target).toBe('windsurf');
    expect(config.configPath).toBe('~/.codeium/windsurf/mcp_config.json');

    const servers = config.content['mcpServers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']).toBeDefined();
    expect(servers['anvil']['command']).toBe('npx');
    expect(servers['anvil']['args']).toEqual(['@eddacraft/anvil-mcp-server']);
  });

  it('generates HTTP config with url format', () => {
    const config = generateWindsurfConfig({ transport: 'http' });

    const servers = config.content['mcpServers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']['url']).toBe('http://localhost:3000/mcp');
  });
});

describe('generateVscodeConfig', () => {
  it('generates stdio config with type "stdio"', () => {
    const config = generateVscodeConfig();
    expect(config.target).toBe('vscode');
    expect(config.configPath).toBe('.vscode/mcp.json');

    const servers = config.content['servers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']).toBeDefined();
    expect(servers['anvil']['type']).toBe('stdio');
    expect(servers['anvil']['command']).toBe('npx');
    expect(servers['anvil']['args']).toEqual(['@eddacraft/anvil-mcp-server']);
  });

  it('generates HTTP config with type "http"', () => {
    const config = generateVscodeConfig({ transport: 'http' });

    const servers = config.content['servers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']).toBeDefined();
    expect(servers['anvil']['type']).toBe('http');
    expect(servers['anvil']['url']).toBe('http://localhost:3000/mcp');
    expect(servers['anvil']).not.toHaveProperty('command');
  });

  it('uses custom port for HTTP config', () => {
    const config = generateVscodeConfig({ transport: 'http', port: 9000 });

    const servers = config.content['servers'] as Record<string, Record<string, unknown>>;
    expect(servers['anvil']['url']).toBe('http://localhost:9000/mcp');
  });
});

describe('configPath correctness', () => {
  it('claude-code config path is .claude/mcp.json', () => {
    expect(generateMcpConfig('claude-code').configPath).toBe('.claude/mcp.json');
  });

  it('cursor config path is .cursor/mcp.json', () => {
    expect(generateMcpConfig('cursor').configPath).toBe('.cursor/mcp.json');
  });

  it('windsurf config path is ~/.codeium/windsurf/mcp_config.json', () => {
    expect(generateMcpConfig('windsurf').configPath).toBe('~/.codeium/windsurf/mcp_config.json');
  });

  it('vscode config path is .vscode/mcp.json', () => {
    expect(generateMcpConfig('vscode').configPath).toBe('.vscode/mcp.json');
  });
});
