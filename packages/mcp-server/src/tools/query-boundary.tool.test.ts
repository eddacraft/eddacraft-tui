import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { registerQueryBoundaryTool } from './query-boundary.tool.js';

// ---------------------------------------------------------------------------
// Mock @eddacraft/anvil-core
// ---------------------------------------------------------------------------
const mockBaselineExists = vi.fn<(root: string) => boolean>();
const mockLoadBaseline = vi.fn<(root: string) => unknown>();
const mockDetectLayer =
  vi.fn<(filePath: string) => { file: string; layer: string | null; confidence: string }>();
const mockIsAllowedDependency = vi.fn<(from: string, to: string, layers?: unknown) => boolean>();

vi.mock('@eddacraft/anvil-core', () => ({
  baselineExists: (...args: Parameters<typeof mockBaselineExists>) => mockBaselineExists(...args),
  loadBaseline: (...args: Parameters<typeof mockLoadBaseline>) => mockLoadBaseline(...args),
  createLayerDetector: () => ({
    detectLayer: (...args: Parameters<typeof mockDetectLayer>) => mockDetectLayer(...args),
    isAllowedDependency: (...args: Parameters<typeof mockIsAllowedDependency>) =>
      mockIsAllowedDependency(...args),
  }),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Standard layers fixture used across tests. */
const MOCK_LAYERS = {
  presentation: {
    patterns: ['**/controllers/**'],
    depends_on: ['application', 'shared'],
    description: 'HTTP handlers',
  },
  application: {
    patterns: ['**/services/**'],
    depends_on: ['domain', 'shared'],
    description: 'Business logic',
  },
  domain: {
    patterns: ['**/domain/**'],
    depends_on: ['shared'],
    description: 'Domain entities',
  },
  shared: {
    patterns: ['**/utils/**'],
    depends_on: [],
    description: 'Shared utilities',
  },
};

const MOCK_BASELINE = {
  schema_version: '0.1.0',
  created_at: '2025-01-01T00:00:00.000Z',
  updated_at: '2025-01-01T00:00:00.000Z',
  entry_points: [],
  layers: MOCK_LAYERS,
  boundaries: [],
  baseline_snapshot: {
    module_count: 10,
    timestamp: '2025-01-01T00:00:00.000Z',
    violations: [],
  },
};

function parseResult(
  result: Awaited<ReturnType<typeof Client.prototype.callTool>>
): Record<string, unknown> {
  const textItem = (result.content as Array<{ type: string; text: string }>)[0];
  return JSON.parse(textItem.text) as Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
describe('anvil_query_boundary tool', () => {
  let server: McpServer;
  let client: Client;

  beforeEach(async () => {
    vi.clearAllMocks();

    server = new McpServer({ name: 'test-qb', version: '0.0.1' });
    registerQueryBoundaryTool(server);

    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);

    client = new Client({ name: 'test-client', version: '1.0.0' });
    await client.connect(clientTransport);
  });

  afterEach(async () => {
    await client.close();
    await server.close();
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // Tool registration
  // -------------------------------------------------------------------------
  describe('tool registration', () => {
    it('appears in the tool list with correct metadata', async () => {
      const tools = await client.listTools();
      const tool = tools.tools.find((t) => t.name === 'anvil_query_boundary');

      expect(tool).toBeDefined();
      expect(tool!.description).toContain('boundary');
      expect(tool!.inputSchema).toBeDefined();
      expect(tool!.inputSchema.type).toBe('object');
    });
  });

  // -------------------------------------------------------------------------
  // 1. No baseline
  // -------------------------------------------------------------------------
  describe('no baseline', () => {
    it('returns allowed with no-baseline reason', async () => {
      mockBaselineExists.mockReturnValue(false);

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/controllers/user.ts',
          targetFile: 'src/domain/user.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(true);
      expect(parsed['reason']).toBe('no-baseline');
      expect(parsed['message']).toContain('anvil init');
    });
  });

  // -------------------------------------------------------------------------
  // 2. Same layer
  // -------------------------------------------------------------------------
  describe('same layer', () => {
    it('returns allowed with same-layer reason', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(MOCK_BASELINE);
      mockDetectLayer.mockImplementation((filePath: string) => ({
        file: filePath,
        layer: 'domain',
        confidence: 'high',
      }));

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/domain/user.ts',
          targetFile: 'src/domain/order.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(true);
      expect(parsed['reason']).toBe('same-layer');
      expect(parsed['sourceLayer']).toBe('domain');
      expect(parsed['targetLayer']).toBe('domain');
      expect(parsed['message']).toContain('domain');
    });
  });

  // -------------------------------------------------------------------------
  // 3. Allowed cross-layer
  // -------------------------------------------------------------------------
  describe('allowed cross-layer', () => {
    it('returns allowed with boundary-ok reason', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(MOCK_BASELINE);
      mockDetectLayer.mockImplementation((filePath: string) => {
        if (filePath.includes('controllers')) {
          return { file: filePath, layer: 'presentation', confidence: 'high' };
        }
        return { file: filePath, layer: 'application', confidence: 'high' };
      });
      mockIsAllowedDependency.mockReturnValue(true);

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/controllers/user.ts',
          targetFile: 'src/services/user-service.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(true);
      expect(parsed['reason']).toBe('boundary-ok');
      expect(parsed['sourceLayer']).toBe('presentation');
      expect(parsed['targetLayer']).toBe('application');
    });
  });

  // -------------------------------------------------------------------------
  // 4. Boundary violation
  // -------------------------------------------------------------------------
  describe('boundary violation', () => {
    it('returns not allowed with violation details', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(MOCK_BASELINE);
      mockDetectLayer.mockImplementation((filePath: string) => {
        if (filePath.includes('domain')) {
          return { file: filePath, layer: 'domain', confidence: 'high' };
        }
        return { file: filePath, layer: 'presentation', confidence: 'high' };
      });
      mockIsAllowedDependency.mockReturnValue(false);

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/domain/user.ts',
          targetFile: 'src/controllers/user-controller.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(false);
      expect(parsed['reason']).toBe('boundary-violation');
      expect(parsed['sourceLayer']).toBe('domain');
      expect(parsed['targetLayer']).toBe('presentation');
      expect(parsed['message']).toContain('violates');

      const violation = parsed['violation'] as Record<string, string>;
      expect(violation['from']).toBe('domain');
      expect(violation['to']).toBe('presentation');
    });
  });

  // -------------------------------------------------------------------------
  // 5. Unassigned layer
  // -------------------------------------------------------------------------
  describe('unassigned layer', () => {
    it('returns allowed with unassigned-layer reason when source has no layer', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(MOCK_BASELINE);
      mockDetectLayer.mockImplementation((filePath: string) => {
        if (filePath.includes('random')) {
          return { file: filePath, layer: null, confidence: 'low' };
        }
        return { file: filePath, layer: 'domain', confidence: 'high' };
      });

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/random/helper.ts',
          targetFile: 'src/domain/user.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(true);
      expect(parsed['reason']).toBe('unassigned-layer');
      expect(parsed['sourceLayer']).toBeNull();
      expect(parsed['targetLayer']).toBe('domain');
      expect(parsed['message']).toContain('source');
    });

    it('returns allowed with unassigned-layer reason when target has no layer', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(MOCK_BASELINE);
      mockDetectLayer.mockImplementation((filePath: string) => {
        if (filePath.includes('random')) {
          return { file: filePath, layer: null, confidence: 'low' };
        }
        return { file: filePath, layer: 'domain', confidence: 'high' };
      });

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/domain/user.ts',
          targetFile: 'src/random/helper.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(true);
      expect(parsed['reason']).toBe('unassigned-layer');
      expect(parsed['sourceLayer']).toBe('domain');
      expect(parsed['targetLayer']).toBeNull();
      expect(parsed['message']).toContain('target');
    });

    it('returns allowed with unassigned-layer reason when both have no layer', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(MOCK_BASELINE);
      mockDetectLayer.mockReturnValue({ file: 'test', layer: null, confidence: 'low' });

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/foo.ts',
          targetFile: 'src/bar.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(true);
      expect(parsed['reason']).toBe('unassigned-layer');
      expect(parsed['message']).toContain('source');
      expect(parsed['message']).toContain('target');
    });
  });

  // -------------------------------------------------------------------------
  // 6. Error handling
  // -------------------------------------------------------------------------
  describe('error handling', () => {
    it('returns isError when an exception is thrown', async () => {
      mockBaselineExists.mockImplementation(() => {
        throw new Error('Disk read failed');
      });

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/a.ts',
          targetFile: 'src/b.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBe(true);
      const parsed = parseResult(result);
      expect(parsed['error']).toBe('Disk read failed');
    });

    it('returns isError with stringified non-Error exceptions', async () => {
      mockBaselineExists.mockImplementation(() => {
        throw 'something went wrong';
      });

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/a.ts',
          targetFile: 'src/b.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBe(true);
      const parsed = parseResult(result);
      expect(parsed['error']).toBe('something went wrong');
    });
  });

  // -------------------------------------------------------------------------
  // 7. Baseline load failure
  // -------------------------------------------------------------------------
  describe('baseline load failure', () => {
    it('returns allowed with baseline-load-failed reason when loadBaseline returns null', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(null);

      const result = await client.callTool({
        name: 'anvil_query_boundary',
        arguments: {
          sourceFile: 'src/a.ts',
          targetFile: 'src/b.ts',
          workspaceRoot: '/workspace',
        },
      });

      expect(result.isError).toBeFalsy();
      const parsed = parseResult(result);
      expect(parsed['allowed']).toBe(true);
      expect(parsed['reason']).toBe('baseline-load-failed');
    });
  });
});
