import { describe, it, expect, vi, afterEach } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { registerFixViolationPrompt } from './fix-violation.prompt.js';
import { registerSuppressViolationPrompt } from './suppress-violation.prompt.js';
import { registerArchitectureReviewPrompt } from './architecture-review.prompt.js';
import { registerPreGenerationPrompt } from './pre-generation.prompt.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const cleanupFns: Array<() => Promise<void>> = [];

/**
 * Creates an McpServer with all prompt templates registered, connected to an
 * in-memory client. Returns both sides for assertions.
 */
async function createPromptPair() {
  const server = new McpServer({ name: 'test-prompt-server', version: '0.0.1' });

  registerFixViolationPrompt(server);
  registerSuppressViolationPrompt(server);
  registerArchitectureReviewPrompt(server);
  registerPreGenerationPrompt(server);

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

describe('MCP prompt templates', () => {
  afterEach(async () => {
    for (const fn of cleanupFns) {
      await fn();
    }
    cleanupFns.length = 0;
    vi.restoreAllMocks();
  });

  // -------------------------------------------------------------------------
  // 1. Prompt listing
  // -------------------------------------------------------------------------
  describe('listPrompts', () => {
    it('lists all four registered prompts', async () => {
      const { client } = await createPromptPair();
      const result = await client.listPrompts();

      expect(result.prompts).toBeDefined();
      expect(result.prompts).toHaveLength(4);

      const names = result.prompts.map((p) => p.name).sort();
      expect(names).toEqual([
        'architecture-review',
        'fix-violation',
        'pre-generation',
        'suppress-violation',
      ]);
    });

    it('each prompt has a description', async () => {
      const { client } = await createPromptPair();
      const result = await client.listPrompts();

      for (const prompt of result.prompts) {
        expect(prompt.description).toBeDefined();
        expect(prompt.description!.length).toBeGreaterThan(0);
      }
    });
  });

  // -------------------------------------------------------------------------
  // 2. fix-violation prompt
  // -------------------------------------------------------------------------
  describe('fix-violation', () => {
    it('returns a message with the warning ID and file path', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'fix-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts' },
      });

      expect(result.messages).toHaveLength(1);
      expect(result.messages[0].role).toBe('user');

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.type).toBe('text');
      expect(text.text).toContain('AP-003');
      expect(text.text).toContain('src/foo.ts');
    });

    it('includes line number when provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'fix-violation',
        arguments: {
          warningId: 'AP-001',
          filePath: 'src/bar.ts',
          line: '42',
        },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('at line 42');
    });

    it('includes the warning message when provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'fix-violation',
        arguments: {
          warningId: 'AP-003',
          filePath: 'src/foo.ts',
          message: 'Unexpected any. Specify a different type.',
        },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('Unexpected any. Specify a different type.');
    });

    it('omits line reference when line is not provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'fix-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).not.toContain('at line');
    });

    it('includes known fix patterns', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'fix-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('AP-001');
      expect(text.text).toContain('AP-003');
      expect(text.text).toContain('AP-004');
      expect(text.text).toContain('ARCH-');
    });

    it('includes a reminder to run anvil_check', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'fix-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('anvil_check');
    });
  });

  // -------------------------------------------------------------------------
  // 3. suppress-violation prompt
  // -------------------------------------------------------------------------
  describe('suppress-violation', () => {
    it('returns a message with warning ID, file path, and line', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'suppress-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts', line: '10' },
      });

      expect(result.messages).toHaveLength(1);
      expect(result.messages[0].role).toBe('user');

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.type).toBe('text');
      expect(text.text).toContain('AP-003');
      expect(text.text).toContain('src/foo.ts');
      expect(text.text).toContain('line 10');
    });

    it('includes the reason when provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'suppress-violation',
        arguments: {
          warningId: 'AP-003',
          filePath: 'src/foo.ts',
          line: '10',
          reason: 'Third-party callback signature requires any',
        },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('Third-party callback signature requires any');
    });

    it('includes suppression format documentation', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'suppress-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts', line: '10' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('@anvil-ignore');
      expect(text.text).toContain('expires');
    });

    it('includes guidance on when suppression is appropriate', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'suppress-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts', line: '10' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('When suppression is appropriate');
      expect(text.text).toContain('When you should fix instead');
    });

    it('references anvil_suppress tool', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'suppress-violation',
        arguments: { warningId: 'AP-003', filePath: 'src/foo.ts', line: '10' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('anvil_suppress');
    });
  });

  // -------------------------------------------------------------------------
  // 4. architecture-review prompt
  // -------------------------------------------------------------------------
  describe('architecture-review', () => {
    it('returns a message with the file path', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'architecture-review',
        arguments: { filePath: 'src/services/auth.ts' },
      });

      expect(result.messages).toHaveLength(1);
      expect(result.messages[0].role).toBe('user');

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.type).toBe('text');
      expect(text.text).toContain('src/services/auth.ts');
    });

    it('uses workspace root in tool call examples', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'architecture-review',
        arguments: {
          filePath: 'src/services/auth.ts',
          workspaceRoot: '/home/user/project',
        },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('/home/user/project');
    });

    it('defaults workspace root to "." when not provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'architecture-review',
        arguments: { filePath: 'src/services/auth.ts' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('workspaceRoot: "."');
    });

    it('includes instructions to use anvil_query_boundary', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'architecture-review',
        arguments: { filePath: 'src/services/auth.ts' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('anvil_query_boundary');
    });

    it('includes instructions to run anvil_check with architecture checks', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'architecture-review',
        arguments: { filePath: 'src/services/auth.ts' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('anvil_check');
      expect(text.text).toContain('"architecture"');
    });

    it('includes layer assignment and import boundary checks', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'architecture-review',
        arguments: { filePath: 'src/services/auth.ts' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('Layer assignment');
      expect(text.text).toContain('Import boundary');
    });
  });

  // -------------------------------------------------------------------------
  // 5. pre-generation prompt
  // -------------------------------------------------------------------------
  describe('pre-generation', () => {
    it('returns a message with architecture constraints', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: { workspaceRoot: '/home/user/project' },
      });

      expect(result.messages).toHaveLength(1);
      expect(result.messages[0].role).toBe('user');

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.type).toBe('text');
      expect(text.text).toContain('ARCHITECTURE CONSTRAINTS');
    });

    it('includes layer definitions', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: { workspaceRoot: '/home/user/project' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('domain/core');
      expect(text.text).toContain('application');
      expect(text.text).toContain('infrastructure');
      expect(text.text).toContain('ui/presentation');
    });

    it('includes antipattern warnings', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: { workspaceRoot: '/home/user/project' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('AP-001');
      expect(text.text).toContain('AP-003');
      expect(text.text).toContain('AP-004');
      expect(text.text).toContain('eslint-disable');
      expect(text.text).toContain(': any');
      expect(text.text).toContain('@ts-ignore');
    });

    it('includes target file constraints when targetFile is provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: {
          workspaceRoot: '/home/user/project',
          targetFile: 'src/domain/user.ts',
        },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('src/domain/user.ts');
      expect(text.text).toContain('Target file constraints');
      expect(text.text).toContain('anvil_query_boundary');
    });

    it('omits target file section when targetFile is not provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: { workspaceRoot: '/home/user/project' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).not.toContain('Target file constraints');
    });

    it('includes instruction to run anvil_check after generation', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: { workspaceRoot: '/home/user/project' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('anvil_check');
    });

    it('uses targetFile in anvil_check example when provided', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: {
          workspaceRoot: '/home/user/project',
          targetFile: 'src/domain/user.ts',
        },
      });

      const text = result.messages[0].content as { type: string; text: string };
      // The anvil_check example should reference the target file
      expect(text.text).toContain('files: ["src/domain/user.ts"]');
    });

    it('includes anvil_query_boundary instruction for cross-layer imports', async () => {
      const { client } = await createPromptPair();
      const result = await client.getPrompt({
        name: 'pre-generation',
        arguments: { workspaceRoot: '/home/user/project' },
      });

      const text = result.messages[0].content as { type: string; text: string };
      expect(text.text).toContain('anvil_query_boundary');
      expect(text.text).toContain('BEFORE writing');
    });
  });
});
