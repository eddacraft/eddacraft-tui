import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { registerStatusTool } from './status.tool.js';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

const mockGetAvailableChecks = vi
  .fn()
  .mockReturnValue(['eslint', 'coverage', 'secret', 'dependency', 'architecture', 'antipattern']);

const mockLoadConfigWithDetails = vi.fn();
const mockBaselineExists = vi.fn();

vi.mock('@eddacraft/anvil-runtime', () => {
  return {
    GateRunner: class MockGateRunner {
      getAvailableChecks = mockGetAvailableChecks;
    },
    GateConfigManager: class MockGateConfigManager {
      loadConfigWithDetails = mockLoadConfigWithDetails;
    },
  };
});

vi.mock('@eddacraft/anvil-core', () => ({
  baselineExists: (...args: unknown[]) => mockBaselineExists(...args),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const cleanupFns: Array<() => Promise<void>> = [];

async function createConnectedPair() {
  const server = new McpServer({ name: 'test-status', version: '0.0.1' });
  registerStatusTool(server);

  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  await server.connect(serverTransport);

  const client = new Client({ name: 'test-client', version: '1.0.0' });
  await client.connect(clientTransport);

  cleanupFns.push(async () => {
    await client.close();
    await server.close();
  });

  return { server, client };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('anvil_status tool', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(async () => {
    for (const fn of cleanupFns) {
      await fn();
    }
    cleanupFns.length = 0;
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // Registration
  // -------------------------------------------------------------------------
  describe('registration', () => {
    it('registers anvil_status with the server', async () => {
      const { client } = await createConnectedPair();
      const { tools } = await client.listTools();

      const statusTool = tools.find((t) => t.name === 'anvil_status');
      expect(statusTool).toBeDefined();
      expect(statusTool!.description).toContain('health summary');
    });

    it('has the expected input schema properties', async () => {
      const { client } = await createConnectedPair();
      const { tools } = await client.listTools();
      const statusTool = tools.find((t) => t.name === 'anvil_status');

      expect(statusTool!.inputSchema).toBeDefined();
      expect(statusTool!.inputSchema.type).toBe('object');
      const props = statusTool!.inputSchema.properties as Record<string, unknown>;
      expect(props).toHaveProperty('workspaceRoot');
    });
  });

  // -------------------------------------------------------------------------
  // Successful execution
  // -------------------------------------------------------------------------
  describe('successful execution', () => {
    it('returns full status with config and baseline', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: {
          version: 1,
          checks: [
            { name: 'eslint', enabled: true, config: {} },
            { name: 'coverage', enabled: true, config: {} },
            { name: 'policy', enabled: false, config: {} },
          ],
          thresholds: { overall_score: 80 },
        },
        path: '/tmp/project/.anvilrc',
        isDefault: false,
        errors: [],
      });
      mockBaselineExists.mockReturnValue(true);

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot: '/tmp/project' },
      });

      expect(result.isError).toBeFalsy();
      const content = result.content as Array<{ type: string; text: string }>;
      const parsed = JSON.parse(content[0].text);

      expect(parsed.status).toBe('ok');
      expect(parsed.workspaceRoot).toBe('/tmp/project');
      expect(parsed.version).toBe('0.1.0');
      expect(parsed.hasBaseline).toBe(true);
      expect(parsed.availableChecks).toEqual(
        expect.arrayContaining(['eslint', 'coverage', 'secret'])
      );
      expect(parsed.config.loaded).toBe(true);
      expect(parsed.config.source).toBe('/tmp/project/.anvilrc');
      // Only enabled checks should be listed
      expect(parsed.config.checks).toContain('eslint');
      expect(parsed.config.checks).toContain('coverage');
      expect(parsed.config.checks).not.toContain('policy');
    });

    it('handles missing config (default)', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: {
          version: 1,
          checks: [{ name: 'eslint', enabled: true, config: {} }],
          thresholds: { overall_score: 80 },
        },
        path: null,
        isDefault: true,
        errors: [],
      });
      mockBaselineExists.mockReturnValue(false);

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot: '/tmp/new-project' },
      });

      expect(result.isError).toBeFalsy();
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);

      expect(parsed.status).toBe('ok');
      expect(parsed.hasBaseline).toBe(false);
      expect(parsed.config.loaded).toBe(false);
      expect(parsed.config.source).toBeNull();
    });

    it('handles config load throwing (falls back gracefully)', async () => {
      mockLoadConfigWithDetails.mockImplementation(() => {
        throw new Error('Permission denied');
      });
      mockBaselineExists.mockReturnValue(false);

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot: '/tmp/restricted' },
      });

      expect(result.isError).toBeFalsy();
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.status).toBe('ok');
      expect(parsed.config.loaded).toBe(false);
      expect(parsed.config.checks).toEqual([]);
    });

    it('passes workspaceRoot through to the response', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: { version: 1, checks: [], thresholds: { overall_score: 80 } },
        path: null,
        isDefault: true,
        errors: [],
      });
      mockBaselineExists.mockReturnValue(false);

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot: '/home/user/myproject' },
      });

      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.workspaceRoot).toBe('/home/user/myproject');
    });

    it('calls baselineExists with the correct workspace root', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: { version: 1, checks: [], thresholds: { overall_score: 80 } },
        path: null,
        isDefault: true,
        errors: [],
      });
      mockBaselineExists.mockReturnValue(false);

      const { client } = await createConnectedPair();
      await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot: '/tmp/check-baseline' },
      });

      expect(mockBaselineExists).toHaveBeenCalledWith('/tmp/check-baseline');
    });
  });

  // -------------------------------------------------------------------------
  // Error handling
  // -------------------------------------------------------------------------
  describe('error handling', () => {
    it('returns isError when baselineExists throws', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: { version: 1, checks: [], thresholds: { overall_score: 80 } },
        path: null,
        isDefault: true,
        errors: [],
      });
      mockBaselineExists.mockImplementation(() => {
        throw new Error('Filesystem error');
      });

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot: '/tmp/broken' },
      });

      expect(result.isError).toBe(true);
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.error).toBe('Filesystem error');
    });

    it('returns text content type in error responses', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: { version: 1, checks: [], thresholds: { overall_score: 80 } },
        path: null,
        isDefault: true,
        errors: [],
      });
      mockBaselineExists.mockImplementation(() => {
        throw new Error('Unexpected');
      });

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot: '/tmp/err' },
      });

      const content = result.content as Array<{ type: string; text: string }>;
      expect(content).toHaveLength(1);
      expect(content[0].type).toBe('text');
    });
  });
});
