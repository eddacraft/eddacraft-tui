import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { mkdirSync, writeFileSync, readFileSync, rmSync, mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { registerSuppressTool } from './suppress.tool.js';

describe('anvil_suppress tool', () => {
  let server: McpServer;
  let client: Client;
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-suppress-test-'));
    mkdirSync(join(tmpDir, 'src'), { recursive: true });

    server = new McpServer({ name: 'test-suppress', version: '0.0.1' });
    registerSuppressTool(server, () => tmpDir);

    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);

    client = new Client({ name: 'test-client', version: '1.0.0' });
    await client.connect(clientTransport);
  });

  afterEach(async () => {
    await client.close();
    await server.close();
    rmSync(tmpDir, { recursive: true, force: true });
    vi.useRealTimers();
  });

  function parseResult(
    result: Awaited<ReturnType<typeof client.callTool>>
  ): Record<string, unknown> {
    const textItem = (result.content as Array<{ type: string; text: string }>)[0];
    return JSON.parse(textItem.text) as Record<string, unknown>;
  }

  // ---------------------------------------------------------------------------
  // Basic suppression
  // ---------------------------------------------------------------------------
  describe('basic suppression', () => {
    it('inserts a suppression comment above the target line', async () => {
      const filePath = 'src/example.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'const x: any = 1;\nconst y = 2;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-003',
          line: 1,
          reason: 'Legacy code, will refactor later',
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(true);
      expect(parsed['warningId']).toBe('AP-003');
      expect(parsed['line']).toBe(1);

      // Verify file content
      const content = readFileSync(absPath, 'utf-8');
      const lines = content.split('\n');
      expect(lines[0]).toContain('// @anvil-ignore-until');
      expect(lines[0]).toContain('AP-003:');
      expect(lines[0]).toContain('Legacy code, will refactor later');
      // Original line should now be on line 2
      expect(lines[1]).toBe('const x: any = 1;');
    });

    it('comment has the correct format: // @anvil-ignore-until YYYY-MM-DD ID: reason', async () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date('2026-01-15T00:00:00Z'));

      const filePath = 'src/format.ts';
      writeFileSync(join(tmpDir, filePath), 'const x = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-004',
          line: 1,
          reason: 'Tech debt',
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(true);
      expect(parsed['comment']).toBe('// @anvil-ignore-until 2026-02-14 AP-004: Tech debt');
      expect(parsed['expiryDate']).toBe('2026-02-14');
    });
  });

  // ---------------------------------------------------------------------------
  // Indentation preservation
  // ---------------------------------------------------------------------------
  describe('indentation', () => {
    it('preserves the indentation of the target line', async () => {
      const filePath = 'src/indented.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'function foo() {\n    const x: any = 1;\n}\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-003',
          line: 2,
          reason: 'Needs refactor',
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(true);

      const content = readFileSync(absPath, 'utf-8');
      const lines = content.split('\n');
      // The suppression comment should have the same indentation as the target line
      expect(lines[1]).toMatch(/^ {4}\/\/ @anvil-ignore-until \d{4}-\d{2}-\d{2} AP-003:/);
      expect(lines[2]).toBe('    const x: any = 1;');
    });
  });

  // ---------------------------------------------------------------------------
  // Custom expiry
  // ---------------------------------------------------------------------------
  describe('custom expiryDays', () => {
    it('uses custom expiry when specified', async () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date('2026-03-01T00:00:00Z'));

      const filePath = 'src/custom-expiry.ts';
      writeFileSync(join(tmpDir, filePath), 'const x = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-001',
          line: 1,
          reason: 'Short-term workaround',

          expiryDays: 7,
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(true);
      expect(parsed['expiryDate']).toBe('2026-03-08');
    });

    it('defaults to 30 days when expiryDays is not provided', async () => {
      vi.useFakeTimers();
      vi.setSystemTime(new Date('2026-06-01T00:00:00Z'));

      const filePath = 'src/default-expiry.ts';
      writeFileSync(join(tmpDir, filePath), 'const x = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-003',
          line: 1,
          reason: 'Default expiry test',
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(true);
      expect(parsed['expiryDate']).toBe('2026-07-01');
    });
  });

  // ---------------------------------------------------------------------------
  // Error cases
  // ---------------------------------------------------------------------------
  describe('error handling', () => {
    it('returns suppressed: false when line is out of range', async () => {
      const filePath = 'src/short.ts';
      writeFileSync(join(tmpDir, filePath), 'const x = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-003',
          line: 999,
          reason: 'Out of range',
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(false);
      expect(parsed['reason']).toContain('out of range');
    });

    it('rejects path traversal with ../', async () => {
      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath: '../../../etc/passwd',
          warningId: 'AP-003',
          line: 1,
          reason: 'Traversal attempt',
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(false);
      expect(parsed['reason']).toContain('outside workspace root');
    });

    it('returns isError when file does not exist', async () => {
      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath: 'nonexistent/file.ts',
          warningId: 'AP-003',
          line: 1,
          reason: 'Does not exist',
        },
      });

      expect(result.isError).toBe(true);
      const parsed = parseResult(result);
      expect(parsed['error']).toBeDefined();
    });
  });

  // ---------------------------------------------------------------------------
  // Idempotency
  // ---------------------------------------------------------------------------
  describe('idempotency', () => {
    it('inserting a second suppression adds another comment line', async () => {
      const filePath = 'src/double.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'const x: any = 1;\n', 'utf-8');

      // First suppression
      await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-003',
          line: 1,
          reason: 'First suppression',
        },
      });

      // Second suppression (line 2 now is the original line due to insertion)
      const result = await client.callTool({
        name: 'anvil_suppress',
        arguments: {
          filePath,
          warningId: 'AP-003',
          line: 2,
          reason: 'Second suppression',
        },
      });

      const parsed = parseResult(result);
      expect(parsed['suppressed']).toBe(true);

      const content = readFileSync(absPath, 'utf-8');
      const lines = content.split('\n');
      // Should have two suppression comments and then the original line
      expect(lines[0]).toContain('First suppression');
      expect(lines[1]).toContain('Second suppression');
      expect(lines[2]).toBe('const x: any = 1;');
    });
  });

  // ---------------------------------------------------------------------------
  // Tool listing
  // ---------------------------------------------------------------------------
  describe('tool registration', () => {
    it('anvil_suppress appears in the tool list', async () => {
      const tools = await client.listTools();
      const suppressTool = tools.tools.find((t) => t.name === 'anvil_suppress');
      expect(suppressTool).toBeDefined();
      expect(suppressTool!.description).toContain('suppression');
    });
  });
});
