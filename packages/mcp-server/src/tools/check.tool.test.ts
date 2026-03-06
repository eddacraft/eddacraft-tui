import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { registerCheckTool } from './check.tool.js';

// ---------------------------------------------------------------------------
// Mock @eddacraft/anvil-runtime
// ---------------------------------------------------------------------------

const mockAnalyzeFiles = vi.fn();

vi.mock('@eddacraft/anvil-runtime', () => {
  return {
    GateRunner: class MockGateRunner {
      analyzeFiles = mockAnalyzeFiles;
    },
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Creates an McpServer with only the check tool registered, connected to an
 * in-memory client. Returns both sides for assertions.
 */
async function createCheckToolPair() {
  const server = new McpServer({ name: 'test-check-server', version: '0.0.1' });
  registerCheckTool(server);

  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  await server.connect(serverTransport);

  const client = new Client({ name: 'test-client', version: '1.0.0' });
  await client.connect(clientTransport);

  return { server, client };
}

function makeAnalyzeResult(overrides: Record<string, unknown> = {}) {
  return {
    warnings: {
      warnings: [],
      summary: { total: 0, errors: 0, warnings: 0, info: 0, suppressed: 0 },
      patterns_checked: [],
    },
    executionTimeMs: 42,
    checksRun: ['architecture', 'antipattern'],
    hasBlockingWarnings: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('anvil_check tool', () => {
  const cleanupFns: Array<() => Promise<void>> = [];
  let workspaceRoot: string;

  beforeEach(() => {
    mockAnalyzeFiles.mockReset();
    workspaceRoot = mkdtempSync(join(tmpdir(), 'anvil-mcp-check-'));
  });

  afterEach(async () => {
    for (const fn of cleanupFns) {
      await fn();
    }
    cleanupFns.length = 0;
    rmSync(workspaceRoot, { recursive: true, force: true });
    vi.restoreAllMocks();
  });

  async function setup() {
    const pair = await createCheckToolPair();
    cleanupFns.push(async () => {
      await pair.client.close();
      await pair.server.close();
    });
    return pair;
  }

  // -------------------------------------------------------------------------
  // 1. Tool registration
  // -------------------------------------------------------------------------
  describe('registration', () => {
    it('registers a tool named anvil_check', async () => {
      const { client } = await setup();
      const { tools } = await client.listTools();

      const checkTool = tools.find((t) => t.name === 'anvil_check');
      expect(checkTool).toBeDefined();
    });

    it('has the correct annotations and description', async () => {
      const { client } = await setup();
      const { tools } = await client.listTools();

      const checkTool = tools.find((t) => t.name === 'anvil_check');
      expect(checkTool!.annotations?.readOnlyHint).toBe(true);
      expect(checkTool!.annotations?.destructiveHint).toBe(false);
      expect(checkTool!.annotations?.idempotentHint).toBe(true);
      expect(checkTool!.description).toContain('Validate files');
    });

    it('defines the expected input schema properties', async () => {
      const { client } = await setup();
      const { tools } = await client.listTools();

      const checkTool = tools.find((t) => t.name === 'anvil_check');
      expect(checkTool!.inputSchema).toBeDefined();
      expect(checkTool!.inputSchema.type).toBe('object');

      const props = checkTool!.inputSchema.properties as Record<string, unknown>;
      expect(props).toHaveProperty('files');
      expect(props).toHaveProperty('workspaceRoot');
      expect(props).toHaveProperty('checks');
    });
  });

  // -------------------------------------------------------------------------
  // 2. Successful check with no warnings
  // -------------------------------------------------------------------------
  describe('successful check', () => {
    it('returns JSON with empty warnings array and summary', async () => {
      mockAnalyzeFiles.mockResolvedValue(makeAnalyzeResult());
      const { client } = await setup();

      const result = await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/app.ts'],
          workspaceRoot,
        },
      });

      expect(result.isError).toBeFalsy();

      const content = result.content as Array<{ type: string; text: string }>;
      expect(content).toHaveLength(1);
      expect(content[0].type).toBe('text');

      const parsed = JSON.parse(content[0].text);
      expect(parsed.warnings).toEqual([]);
      expect(parsed.summary).toEqual({
        total: 0,
        errors: 0,
        warnings: 0,
        info: 0,
        suppressed: 0,
      });
      expect(parsed.executionTimeMs).toBe(42);
      expect(parsed.checksRun).toEqual(['architecture', 'antipattern']);
      expect(parsed.hasBlockingWarnings).toBe(false);
    });

    it('calls GateRunner.analyzeFiles with correct arguments', async () => {
      mockAnalyzeFiles.mockResolvedValue(makeAnalyzeResult());
      const { client } = await setup();

      await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/a.ts', 'src/b.ts'],
          workspaceRoot,
        },
      });

      expect(mockAnalyzeFiles).toHaveBeenCalledOnce();
      expect(mockAnalyzeFiles).toHaveBeenCalledWith(['src/a.ts', 'src/b.ts'], workspaceRoot, {
        checks: undefined,
      });
    });
  });

  // -------------------------------------------------------------------------
  // 3. Check with warnings
  // -------------------------------------------------------------------------
  describe('check with warnings', () => {
    it('maps warnings to the expected output shape', async () => {
      const warnings = [
        {
          id: 'ARCH-001',
          category: 'architecture',
          severity: 'error',
          confidence: 'high',
          title: 'Circular dependency detected',
          message: 'Module A imports Module B which imports Module A',
          explanation: 'Circular dependencies cause tight coupling.',
          suggestion: 'Extract shared code into a third module.',
          location: { file: 'src/a.ts', line: 10, column: 5 },
        },
        {
          id: 'AP-002',
          category: 'antipattern',
          severity: 'warning',
          confidence: 'medium',
          title: 'God class',
          message: 'Class AppService has too many responsibilities',
          explanation: 'Single Responsibility Principle violation.',
          suggestion: 'Split into focused services.',
          location: { file: 'src/app-service.ts', line: 1 },
        },
      ];

      mockAnalyzeFiles.mockResolvedValue(
        makeAnalyzeResult({
          warnings: {
            warnings,
            summary: { total: 2, errors: 1, warnings: 1, info: 0, suppressed: 0 },
            patterns_checked: ['circular-dep', 'god-class'],
          },
          hasBlockingWarnings: true,
        })
      );

      const { client } = await setup();

      const result = await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/a.ts', 'src/app-service.ts'],
          workspaceRoot,
        },
      });

      expect(result.isError).toBeFalsy();

      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);

      expect(parsed.warnings).toHaveLength(2);
      expect(parsed.hasBlockingWarnings).toBe(true);
      expect(parsed.summary.total).toBe(2);
      expect(parsed.summary.errors).toBe(1);

      // First warning - architecture error
      const w0 = parsed.warnings[0];
      expect(w0.id).toBe('ARCH-001');
      expect(w0.severity).toBe('error');
      expect(w0.category).toBe('architecture');
      expect(w0.title).toBe('Circular dependency detected');
      expect(w0.message).toContain('Module A');
      expect(w0.explanation).toBe('Circular dependencies cause tight coupling.');
      expect(w0.suggestion).toContain('Extract shared code');
      expect(w0.location).toEqual({ file: 'src/a.ts', line: 10, column: 5 });

      // Second warning - antipattern warning
      const w1 = parsed.warnings[1];
      expect(w1.id).toBe('AP-002');
      expect(w1.severity).toBe('warning');
      expect(w1.category).toBe('antipattern');

      // Verify fields NOT included in the mapped output
      expect(w0).not.toHaveProperty('confidence');
    });
  });

  // -------------------------------------------------------------------------
  // 4. Error handling
  // -------------------------------------------------------------------------
  describe('error handling', () => {
    it('returns isError when GateRunner throws an Error', async () => {
      mockAnalyzeFiles.mockRejectedValue(new Error('Cannot read workspace'));
      const { client } = await setup();

      const result = await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/app.ts'],
          workspaceRoot,
        },
      });

      expect(result.isError).toBe(true);

      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.error).toBe('Cannot read workspace');
    });

    it('returns isError when GateRunner throws a non-Error value', async () => {
      mockAnalyzeFiles.mockRejectedValue('string error');
      const { client } = await setup();

      const result = await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/app.ts'],
          workspaceRoot,
        },
      });

      expect(result.isError).toBe(true);

      const parsed = JSON.parse((result.content as Array<{ type: string; text: string }>)[0].text);
      expect(parsed.error).toBe('string error');
    });
  });

  // -------------------------------------------------------------------------
  // 5. Optional checks parameter
  // -------------------------------------------------------------------------
  describe('optional checks parameter', () => {
    it('passes checks array through to analyzeFiles', async () => {
      mockAnalyzeFiles.mockResolvedValue(makeAnalyzeResult({ checksRun: ['architecture'] }));
      const { client } = await setup();

      await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/app.ts'],
          workspaceRoot,
          checks: ['architecture'],
        },
      });

      expect(mockAnalyzeFiles).toHaveBeenCalledWith(['src/app.ts'], workspaceRoot, {
        checks: ['architecture'],
      });
    });

    it('passes undefined checks when parameter is omitted', async () => {
      mockAnalyzeFiles.mockResolvedValue(makeAnalyzeResult());
      const { client } = await setup();

      await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/app.ts'],
          workspaceRoot,
        },
      });

      expect(mockAnalyzeFiles).toHaveBeenCalledWith(['src/app.ts'], workspaceRoot, {
        checks: undefined,
      });
    });

    it('passes both checks when both are specified', async () => {
      mockAnalyzeFiles.mockResolvedValue(makeAnalyzeResult());
      const { client } = await setup();

      await client.callTool({
        name: 'anvil_check',
        arguments: {
          files: ['src/app.ts'],
          workspaceRoot,
          checks: ['architecture', 'antipattern'],
        },
      });

      expect(mockAnalyzeFiles).toHaveBeenCalledWith(['src/app.ts'], workspaceRoot, {
        checks: ['architecture', 'antipattern'],
      });
    });
  });
});
