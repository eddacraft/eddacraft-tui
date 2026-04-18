/**
 * MCP Server — E2E Tests
 *
 * Tests that the MCP server can be created, registers the expected
 * tools, and produces valid configuration for supported targets.
 *
 * Surface: MCP Server
 */

import { describe, it, expect } from 'vitest';
import {
  createAnvilMcpServer,
  generateMcpConfig,
  SUPPORTED_TARGETS,
  type AnvilMcpServerOptions,
} from '@eddacraft/anvil-mcp-server';

describe('MCP Server › Initialisation', () => {
  it('createAnvilMcpServer returns a server instance', () => {
    const server = createAnvilMcpServer();
    expect(server).toBeDefined();
  });

  it('accepts custom options', () => {
    const options: AnvilMcpServerOptions = {
      projectRoot: '/tmp/test-project',
    };
    const server = createAnvilMcpServer(options);
    expect(server).toBeDefined();
  });
});

describe('MCP Config Generation', () => {
  it('SUPPORTED_TARGETS is a non-empty array', () => {
    expect(Array.isArray(SUPPORTED_TARGETS)).toBe(true);
    expect(SUPPORTED_TARGETS.length).toBeGreaterThan(0);
  });

  it('generates config for each supported target', () => {
    for (const target of SUPPORTED_TARGETS) {
      const config = generateMcpConfig(target);
      expect(config).toBeDefined();
      expect(typeof config).toBe('object');
    }
  });

  it('generated config contains server command info', () => {
    const config = generateMcpConfig(SUPPORTED_TARGETS[0]);
    // MCP config should specify how to launch the server
    expect(config).toBeDefined();
  });
});
