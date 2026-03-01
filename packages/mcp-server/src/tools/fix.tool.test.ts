import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { registerFixTool } from './fix.tool.js';

describe('anvil_fix tool', () => {
  let server: McpServer;
  let client: Client;
  let tmpDir: string;

  beforeEach(async () => {
    // Create a temp workspace
    tmpDir = mkdtempSync(join(tmpdir(), 'anvil-fix-test-'));
    mkdirSync(join(tmpDir, 'src'), { recursive: true });

    // Create MCP server with just the fix tool
    server = new McpServer({ name: 'test-fix', version: '0.0.1' });
    registerFixTool(server, () => tmpDir);

    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);

    client = new Client({ name: 'test-client', version: '1.0.0' });
    await client.connect(clientTransport);
  });

  afterEach(async () => {
    await client.close();
    await server.close();
    rmSync(tmpDir, { recursive: true, force: true });
  });

  function parseResult(
    result: Awaited<ReturnType<typeof client.callTool>>
  ): Record<string, unknown> {
    const textItem = (result.content as Array<{ type: string; text: string }>)[0];
    return JSON.parse(textItem.text) as Record<string, unknown>;
  }

  // ---------------------------------------------------------------------------
  // AP-003: any -> unknown
  // ---------------------------------------------------------------------------
  describe('AP-003 (any -> unknown)', () => {
    it('replaces `: any` with `: unknown` on the target line', async () => {
      const filePath = 'src/example.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'const x: any = 1;\nconst y = 2;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);
      expect(parsed['description']).toContain('unknown');
      expect(parsed['before']).toBe('const x: any = 1;');
      expect(parsed['after']).toBe('const x: unknown = 1;');

      // Verify the file was actually modified
      const content = readFileSync(absPath, 'utf-8');
      expect(content).toBe('const x: unknown = 1;\nconst y = 2;\n');
    });

    it('replaces `:any` (no space) with `: unknown`', async () => {
      const filePath = 'src/nospace.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'function foo(x:any): void {}\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);

      const content = readFileSync(absPath, 'utf-8');
      expect(content).toContain(':unknown');
      expect(content).not.toContain(':any');
    });

    it('does not change `: any` inside string literals', async () => {
      const filePath = 'src/strings.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, `const msg = "type: any is bad";\n`, 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);
    });

    it('does not change comment-only lines', async () => {
      const filePath = 'src/comments.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, '// x: any should stay\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);
    });

    it('fixes `: any` in code after a block comment', async () => {
      const filePath = 'src/mixed-comment.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, '/* note */ const x: any = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);

      const content = readFileSync(absPath, 'utf-8');
      expect(content).toBe('/* note */ const x: unknown = 1;\n');
    });

    it('does not mutate `: any` in trailing inline comments', async () => {
      const filePath = 'src/trailing.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'const x = 1; // type: any\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);

      const content = readFileSync(absPath, 'utf-8');
      expect(content).toBe('const x = 1; // type: any\n');
    });

    it('does not skip generator method signatures', async () => {
      const filePath = 'src/generator.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, '*run(arg: any) {}\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);

      const content = readFileSync(absPath, 'utf-8');
      expect(content).toContain(': unknown');
    });

    it('preserves original whitespace after colon', async () => {
      const filePath = 'src/spacing.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'const x:   any = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);
      expect(parsed['after']).toBe('const x:   unknown = 1;');
    });

    it('replaces multiple `: any` occurrences on the same line', async () => {
      const filePath = 'src/multi.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'function foo(a: any, b: any): any {}\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);

      const content = readFileSync(absPath, 'utf-8');
      expect(content).toBe('function foo(a: unknown, b: unknown): unknown {}\n');
    });
  });

  // ---------------------------------------------------------------------------
  // AP-004: @ts-ignore -> @ts-expect-error
  // ---------------------------------------------------------------------------
  describe('AP-004 (@ts-ignore -> @ts-expect-error)', () => {
    it('replaces @ts-ignore with @ts-expect-error', async () => {
      const filePath = 'src/tsignore.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, '// @ts-ignore\nconst x = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-004', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);
      expect(parsed['before']).toBe('// @ts-ignore');
      expect(parsed['after']).toBe('// @ts-expect-error');

      const content = readFileSync(absPath, 'utf-8');
      expect(content).toBe('// @ts-expect-error\nconst x = 1;\n');
    });
  });

  // ---------------------------------------------------------------------------
  // AP-001: eslint-disable -> eslint-disable-next-line
  // ---------------------------------------------------------------------------
  describe('AP-001 (eslint-disable -> eslint-disable-next-line)', () => {
    it('replaces broad eslint-disable with eslint-disable-next-line', async () => {
      const filePath = 'src/eslint.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, '/* eslint-disable */\nconst x = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-001', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(true);
      expect(parsed['before']).toBe('/* eslint-disable */');
      expect(parsed['after']).toBe('// eslint-disable-next-line');

      const content = readFileSync(absPath, 'utf-8');
      expect(content).toBe('// eslint-disable-next-line\nconst x = 1;\n');
    });
  });

  // ---------------------------------------------------------------------------
  // Error cases
  // ---------------------------------------------------------------------------
  describe('error handling', () => {
    it('returns fixed: false for an unfixable pattern ID', async () => {
      const filePath = 'src/example.ts';
      writeFileSync(join(tmpDir, filePath), 'const x = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-999', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);
      expect(parsed['reason']).toContain('No auto-fix available for AP-999');
      expect(parsed['reason']).toContain('AP-003');
    });

    it('returns fixed: false when the line is out of range', async () => {
      const filePath = 'src/short.ts';
      writeFileSync(join(tmpDir, filePath), 'const x: any = 1;\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 100 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);
      expect(parsed['reason']).toContain('out of range');
    });

    it('returns fixed: false when pattern is not found on the specified line', async () => {
      const filePath = 'src/nomatch.ts';
      writeFileSync(join(tmpDir, filePath), 'const x: string = "hello";\n', 'utf-8');

      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);
      expect(parsed['reason']).toContain('not found on line 1');
    });

    it('rejects path traversal with ../', async () => {
      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: {
          filePath: '../../../etc/passwd',
          warningId: 'AP-003',
          line: 1,
        },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);
      expect(parsed['reason']).toContain('outside workspace root');
    });

    it('returns isError when file does not exist', async () => {
      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: {
          filePath: 'nonexistent/file.ts',
          warningId: 'AP-003',
          line: 1,
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
    it('applying the same fix twice does not change the file further', async () => {
      const filePath = 'src/idempotent.ts';
      const absPath = join(tmpDir, filePath);
      writeFileSync(absPath, 'const x: any = 1;\n', 'utf-8');

      // First fix
      await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const afterFirst = readFileSync(absPath, 'utf-8');
      expect(afterFirst).toBe('const x: unknown = 1;\n');

      // Second fix -- pattern no longer on line, should return fixed: false
      const result = await client.callTool({
        name: 'anvil_fix',
        arguments: { filePath, warningId: 'AP-003', line: 1 },
      });

      const parsed = parseResult(result);
      expect(parsed['fixed']).toBe(false);

      const afterSecond = readFileSync(absPath, 'utf-8');
      expect(afterSecond).toBe(afterFirst);
    });
  });

  // ---------------------------------------------------------------------------
  // Tool listing
  // ---------------------------------------------------------------------------
  describe('tool registration', () => {
    it('anvil_fix appears in the tool list', async () => {
      const tools = await client.listTools();
      const fixTool = tools.tools.find((t) => t.name === 'anvil_fix');
      expect(fixTool).toBeDefined();
      expect(fixTool!.description).toContain('auto-fix');
    });
  });
});
