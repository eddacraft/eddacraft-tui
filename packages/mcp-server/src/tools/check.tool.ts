import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';

/**
 * Registers the `anvil_check` tool on the given MCP server.
 *
 * The tool validates files against Anvil architecture and antipattern rules,
 * returning warnings with locations, explanations, and suggestions.
 */
export function registerCheckTool(server: McpServer): void {
  server.registerTool(
    'anvil_check',
    {
      title: 'Anvil Check',
      description:
        'Validate files against Anvil architecture and antipattern rules. Returns warnings with locations, explanations, and suggestions.',
      inputSchema: {
        files: z.array(z.string()).describe('File paths to check (relative to workspace root)'),
        workspaceRoot: z.string().describe('Workspace root directory'),
        checks: z
          .array(z.enum(['architecture', 'antipattern']))
          .optional()
          .describe('Which checks to run (default: all)'),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
      },
    },
    async ({ files, workspaceRoot, checks }) => {
      try {
        // Dynamic import to avoid requiring runtime at module load
        const { GateRunner } = await import('@eddacraft/anvil-runtime');
        const runner = new GateRunner();

        const result = await runner.analyzeFiles(files, workspaceRoot, {
          checks: checks as ('architecture' | 'antipattern')[] | undefined,
        });

        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify(
                {
                  warnings: result.warnings.warnings.map((w) => ({
                    id: w.id,
                    severity: w.severity,
                    title: w.title,
                    message: w.message,
                    suggestion: w.suggestion,
                    location: w.location,
                    category: w.category,
                  })),
                  summary: result.warnings.summary,
                  executionTimeMs: result.executionTimeMs,
                  checksRun: result.checksRun,
                  hasBlockingWarnings: result.hasBlockingWarnings,
                },
                null,
                2
              ),
            },
          ],
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify({ error: message }),
            },
          ],
          isError: true,
        };
      }
    }
  );
}
