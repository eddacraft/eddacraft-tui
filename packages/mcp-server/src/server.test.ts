import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { createAnvilMcpServer } from './server.js';

// Mock runtime dependencies used by tool callbacks via dynamic import().
vi.mock('@eddacraft/anvil-runtime', () => {
  return {
    GateRunner: class MockGateRunner {
      getAvailableChecks = vi.fn().mockReturnValue(['eslint', 'coverage', 'secret']);
      runGate = vi.fn().mockResolvedValue({
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
        timing: { totalMs: 1, checks: {} },
      });
      analyzeFiles = vi.fn().mockResolvedValue({
        warnings: { warnings: [], count: 0 },
        executionTimeMs: 1,
        checksRun: [],
        hasBlockingWarnings: false,
      });
    },
    GateConfigManager: class MockGateConfigManager {
      loadConfig = vi.fn().mockReturnValue({
        version: 1,
        checks: [{ name: 'eslint', enabled: true, config: {} }],
        thresholds: { overall_score: 80 },
      });
      loadConfigWithDetails = vi.fn().mockReturnValue({
        config: {
          version: 1,
          checks: [{ name: 'eslint', enabled: true, config: {} }],
          thresholds: { overall_score: 80 },
        },
        path: null,
        isDefault: true,
        errors: [],
      });
    },
  };
});

vi.mock('@eddacraft/anvil-core', () => ({
  baselineExists: vi.fn().mockReturnValue(false),
}));

describe('AnvilMcpServer', () => {
  // Track connections for cleanup so transports are closed even if a test fails.
  const cleanupFns: Array<() => Promise<void>> = [];
  let workspaceRoot: string;

  beforeEach(() => {
    workspaceRoot = mkdtempSync(join(tmpdir(), 'anvil-mcp-server-'));
  });

  afterEach(async () => {
    for (const fn of cleanupFns) {
      await fn();
    }
    cleanupFns.length = 0;
    rmSync(workspaceRoot, { recursive: true, force: true });
    vi.restoreAllMocks();
  });

  /**
   * Helper: creates a server and client connected via in-memory transport.
   * Registers cleanup functions so both sides are closed after each test.
   */
  async function createConnectedPair(options?: Parameters<typeof createAnvilMcpServer>[0]) {
    const server = createAnvilMcpServer(options);
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
  // 1. Server creation
  // ---------------------------------------------------------------------------
  describe('createAnvilMcpServer', () => {
    it('returns an McpServer instance with default options', () => {
      const server = createAnvilMcpServer();
      expect(server).toBeDefined();
      expect(server).toBeInstanceOf(McpServer);
    });

    it('accepts custom name and version options', () => {
      const server = createAnvilMcpServer({
        name: 'custom-server',
        version: '2.0.0',
      });
      expect(server).toBeInstanceOf(McpServer);
    });

    it('accepts partial options (name only)', () => {
      const server = createAnvilMcpServer({ name: 'partial-server' });
      expect(server).toBeInstanceOf(McpServer);
    });

    it('accepts partial options (version only)', () => {
      const server = createAnvilMcpServer({ version: '9.9.9' });
      expect(server).toBeInstanceOf(McpServer);
    });
  });

  // ---------------------------------------------------------------------------
  // 2. MCP handshake via in-memory transport
  // ---------------------------------------------------------------------------
  describe('MCP handshake', () => {
    it('completes initialization handshake without error', async () => {
      // createConnectedPair internally calls server.connect + client.connect,
      // which performs the MCP initialize/initialized exchange.
      const { client } = await createConnectedPair();

      // If we reach here, the handshake succeeded. Verify that the client
      // received server version info.
      const serverVersion = client.getServerVersion();
      expect(serverVersion).toBeDefined();
      expect(serverVersion?.name).toBe('anvil-mcp-server');
      expect(serverVersion?.version).toBe('0.1.0');
    });

    it('reflects custom server name and version after handshake', async () => {
      const { client } = await createConnectedPair({
        name: 'my-anvil',
        version: '3.5.0',
      });

      const serverVersion = client.getServerVersion();
      expect(serverVersion).toBeDefined();
      expect(serverVersion?.name).toBe('my-anvil');
      expect(serverVersion?.version).toBe('3.5.0');
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Server capabilities
  // ---------------------------------------------------------------------------
  describe('server capabilities', () => {
    it('advertises tool capabilities after handshake', async () => {
      const { client } = await createConnectedPair();

      const capabilities = client.getServerCapabilities();
      expect(capabilities).toBeDefined();
      // The server registers at least one tool, so it should advertise tools.
      expect(capabilities?.tools).toBeDefined();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Tool listing
  // ---------------------------------------------------------------------------
  describe('listTools', () => {
    it('returns at least the anvil_status tool', async () => {
      const { client } = await createConnectedPair();

      const result = await client.listTools();
      expect(result.tools).toBeDefined();
      expect(result.tools.length).toBeGreaterThanOrEqual(1);

      const statusTool = result.tools.find((t) => t.name === 'anvil_status');
      expect(statusTool).toBeDefined();
    });

    it('anvil_status tool has correct metadata', async () => {
      const { client } = await createConnectedPair();

      const result = await client.listTools();
      const statusTool = result.tools.find((t) => t.name === 'anvil_status');

      expect(statusTool).toBeDefined();
      expect(statusTool!.name).toBe('anvil_status');
      expect(statusTool!.description).toEqual(expect.stringContaining('health summary'));
      expect(statusTool!.inputSchema).toBeDefined();
      expect(statusTool!.inputSchema.type).toBe('object');
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Tool invocation
  // ---------------------------------------------------------------------------
  describe('callTool: anvil_status', () => {
    it('returns a result with content array', async () => {
      const { client } = await createConnectedPair();

      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot },
      });

      expect(result).toBeDefined();
      expect(result.content).toBeDefined();
      expect(Array.isArray(result.content)).toBe(true);
      expect((result.content as unknown[]).length).toBeGreaterThanOrEqual(1);
    });

    it('first content item is a text entry', async () => {
      const { client } = await createConnectedPair();

      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot },
      });
      const firstItem = (result.content as Array<{ type: string }>)[0];

      expect(firstItem).toBeDefined();
      expect(firstItem.type).toBe('text');
    });

    it('returns valid JSON with status, version, and config fields', async () => {
      const { client } = await createConnectedPair();

      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot },
      });
      const textItem = (result.content as Array<{ type: string; text: string }>)[0];

      expect(textItem.type).toBe('text');

      // The text should be parseable JSON
      const parsed = JSON.parse(textItem.text) as Record<string, unknown>;
      expect(parsed).toHaveProperty('status');
      expect(parsed).toHaveProperty('version');
      expect(parsed).toHaveProperty('config');
      expect(parsed).toHaveProperty('availableChecks');
    });

    it('returns status "ok"', async () => {
      const { client } = await createConnectedPair();

      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot },
      });
      const textItem = (result.content as Array<{ type: string; text: string }>)[0];
      const parsed = JSON.parse(textItem.text) as Record<string, unknown>;

      expect(parsed.status).toBe('ok');
    });

    it('returns version string', async () => {
      const { client } = await createConnectedPair();

      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot },
      });
      const textItem = (result.content as Array<{ type: string; text: string }>)[0];
      const parsed = JSON.parse(textItem.text) as Record<string, unknown>;

      expect(typeof parsed.version).toBe('string');
      expect(parsed.version).toBe('0.1.0');
    });

    it('does not report isError', async () => {
      const { client } = await createConnectedPair();

      const result = await client.callTool({
        name: 'anvil_status',
        arguments: { workspaceRoot },
      });

      // isError should be undefined or false for a successful invocation
      expect(result.isError).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Error handling
  // ---------------------------------------------------------------------------
  describe('error handling', () => {
    it('returns isError for a non-existent tool', async () => {
      const { client } = await createConnectedPair();

      // The MCP SDK returns an error result (not a rejection) for unknown tools.
      const result = await client.callTool({
        name: 'nonexistent_tool',
        arguments: {},
      });

      expect(result.isError).toBe(true);
      expect(result.content).toBeDefined();

      const textItem = (result.content as Array<{ type: string; text: string }>)[0];
      expect(textItem.type).toBe('text');
      expect(textItem.text).toContain('nonexistent_tool');
    });
  });
});
