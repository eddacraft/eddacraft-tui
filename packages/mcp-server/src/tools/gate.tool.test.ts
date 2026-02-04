import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { registerGateTool } from './gate.tool.js';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

const mockRunGate = vi.fn();
const mockAnalyzeFiles = vi.fn();
const mockGetAvailableChecks = vi.fn().mockReturnValue(['eslint', 'coverage', 'secret']);
const mockLoadConfig = vi.fn();

vi.mock('@eddacraft/anvil-runtime', () => {
  return {
    GateRunner: class MockGateRunner {
      runGate = mockRunGate;
      analyzeFiles = mockAnalyzeFiles;
      getAvailableChecks = mockGetAvailableChecks;
    },
    GateConfigManager: class MockGateConfigManager {
      loadConfig = mockLoadConfig;
      loadConfigWithDetails = mockLoadConfig;
    },
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const cleanupFns: Array<() => Promise<void>> = [];

async function createConnectedPair() {
  const server = new McpServer({ name: 'test-gate', version: '0.0.1' });
  registerGateTool(server);

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

describe('anvil_gate tool', () => {
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
    it('registers anvil_gate with the server', async () => {
      const { client } = await createConnectedPair();
      const { tools } = await client.listTools();

      const gateTool = tools.find((t) => t.name === 'anvil_gate');
      expect(gateTool).toBeDefined();
      expect(gateTool!.description).toContain('quality gate');
    });

    it('has the expected input schema properties', async () => {
      const { client } = await createConnectedPair();
      const { tools } = await client.listTools();
      const gateTool = tools.find((t) => t.name === 'anvil_gate');

      expect(gateTool!.inputSchema).toBeDefined();
      expect(gateTool!.inputSchema.type).toBe('object');
      const props = gateTool!.inputSchema.properties as Record<string, unknown>;
      expect(props).toHaveProperty('workspaceRoot');
      expect(props).toHaveProperty('targetFiles');
      expect(props).toHaveProperty('skipChecks');
      expect(props).toHaveProperty('failFast');
    });
  });

  // -------------------------------------------------------------------------
  // Full gate run
  // -------------------------------------------------------------------------
  describe('full gate run', () => {
    it('returns gate results with overall pass', async () => {
      mockLoadConfig.mockReturnValue({
        version: 1,
        checks: [{ name: 'eslint', enabled: true, config: {} }],
        thresholds: { overall_score: 80 },
      });
      mockRunGate.mockResolvedValue({
        overall: true,
        score: 95,
        checks: [{ check: 'eslint', passed: true, score: 95, message: 'Lint clean' }],
        summary: { total: 1, passed: 1, failed: 0, skipped: 0 },
        timing: { totalMs: 120, checks: { eslint: 120 } },
        cacheStats: { hits: 0, misses: 1, timeSavedMs: 0 },
      });

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_gate',
        arguments: { workspaceRoot: '/tmp/project' },
      });

      expect(result.isError).toBeFalsy();
      const content = result.content as Array<{ type: string; text: string }>;
      const parsed = JSON.parse(content[0].text);

      expect(parsed.mode).toBe('full');
      expect(parsed.overall).toBe(true);
      expect(parsed.score).toBe(95);
      expect(parsed.summary.total).toBe(1);
      expect(parsed.summary.passed).toBe(1);
      expect(parsed.checks).toHaveLength(1);
      expect(parsed.checks[0].check).toBe('eslint');
      expect(parsed.checks[0].passed).toBe(true);
    });

    it('passes skipChecks and failFast options through', async () => {
      mockLoadConfig.mockReturnValue({
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      });
      mockRunGate.mockResolvedValue({
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
        timing: { totalMs: 5, checks: {} },
      });

      const { client } = await createConnectedPair();
      await client.callTool({
        name: 'anvil_gate',
        arguments: {
          workspaceRoot: '/tmp/project',
          skipChecks: ['coverage'],
          failFast: true,
        },
      });

      expect(mockRunGate).toHaveBeenCalledWith({}, expect.any(Object), '/tmp/project', {
        skipChecks: ['coverage'],
        failFast: true,
      });
    });

    it('returns gate results with overall failure', async () => {
      mockLoadConfig.mockReturnValue({
        version: 1,
        checks: [{ name: 'secret', enabled: true, config: {} }],
        thresholds: { overall_score: 80 },
      });
      mockRunGate.mockResolvedValue({
        overall: false,
        score: 40,
        checks: [{ check: 'secret', passed: false, score: 40, message: 'Secrets found' }],
        summary: { total: 1, passed: 0, failed: 1, skipped: 0 },
        timing: { totalMs: 50, checks: { secret: 50 } },
      });

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_gate',
        arguments: { workspaceRoot: '/tmp/project' },
      });

      expect(result.isError).toBeFalsy();
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.overall).toBe(false);
      expect(parsed.score).toBe(40);
      expect(parsed.summary.failed).toBe(1);
    });
  });

  // -------------------------------------------------------------------------
  // Planless mode (targetFiles)
  // -------------------------------------------------------------------------
  describe('planless mode', () => {
    it('calls analyzeFiles when targetFiles are provided', async () => {
      mockAnalyzeFiles.mockResolvedValue({
        warnings: { warnings: [], count: 0 },
        executionTimeMs: 30,
        checksRun: ['antipattern'],
        hasBlockingWarnings: false,
      });

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_gate',
        arguments: {
          workspaceRoot: '/tmp/project',
          targetFiles: ['src/index.ts', 'src/util.ts'],
        },
      });

      expect(result.isError).toBeFalsy();
      expect(mockAnalyzeFiles).toHaveBeenCalledWith(
        ['src/index.ts', 'src/util.ts'],
        '/tmp/project'
      );
      expect(mockRunGate).not.toHaveBeenCalled();

      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.mode).toBe('planless');
      expect(parsed.checksRun).toEqual(['antipattern']);
      expect(parsed.hasBlockingWarnings).toBe(false);
    });

    it('does not use planless mode for empty targetFiles', async () => {
      mockLoadConfig.mockReturnValue({
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      });
      mockRunGate.mockResolvedValue({
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
        timing: { totalMs: 1, checks: {} },
      });

      const { client } = await createConnectedPair();
      await client.callTool({
        name: 'anvil_gate',
        arguments: {
          workspaceRoot: '/tmp/project',
          targetFiles: [],
        },
      });

      expect(mockRunGate).toHaveBeenCalled();
      expect(mockAnalyzeFiles).not.toHaveBeenCalled();
    });
  });

  // -------------------------------------------------------------------------
  // Error handling
  // -------------------------------------------------------------------------
  describe('error handling', () => {
    it('returns isError when runGate throws', async () => {
      mockLoadConfig.mockReturnValue({
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      });
      mockRunGate.mockRejectedValue(new Error('ESLint binary not found'));

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_gate',
        arguments: { workspaceRoot: '/tmp/project' },
      });

      expect(result.isError).toBe(true);
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.error).toBe('ESLint binary not found');
    });

    it('returns isError when loadConfig throws', async () => {
      mockLoadConfig.mockImplementation(() => {
        throw new Error('Config parse error');
      });

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_gate',
        arguments: { workspaceRoot: '/tmp/project' },
      });

      expect(result.isError).toBe(true);
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.error).toBe('Config parse error');
    });

    it('handles non-Error thrown values', async () => {
      mockLoadConfig.mockReturnValue({
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      });
      mockRunGate.mockRejectedValue('string error');

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_gate',
        arguments: { workspaceRoot: '/tmp/project' },
      });

      expect(result.isError).toBe(true);
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.error).toBe('string error');
    });

    it('returns isError when analyzeFiles throws in planless mode', async () => {
      mockAnalyzeFiles.mockRejectedValue(new Error('File not found'));

      const { client } = await createConnectedPair();
      const result = await client.callTool({
        name: 'anvil_gate',
        arguments: {
          workspaceRoot: '/tmp/project',
          targetFiles: ['nonexistent.ts'],
        },
      });

      expect(result.isError).toBe(true);
      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.error).toBe('File not found');
    });
  });
});
