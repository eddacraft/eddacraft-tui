import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';

/**
 * Registers the `fix-violation` prompt template on the given MCP server.
 *
 * Provides step-by-step guidance for resolving a specific Anvil warning,
 * including known fix patterns for common warning IDs.
 */
export function registerFixViolationPrompt(server: McpServer): void {
  server.registerPrompt(
    'fix-violation',
    {
      title: 'Fix Violation',
      description: 'Guided prompt for resolving a specific Anvil warning',
      argsSchema: {
        warningId: z.string().describe('The warning ID (e.g., AP-003)'),
        filePath: z.string().describe('File containing the violation'),
        line: z.coerce.number().optional().describe('Line number of the violation'),
        message: z.string().optional().describe('The warning message'),
      },
    },
    ({ warningId, filePath, line, message }) => {
      // Sanitize inputs to prevent injection into prompt template
      const safeWarningId = String(warningId)
        .replace(/[\r\n`]/g, '')
        .slice(0, 50);
      const safeFilePath = String(filePath)
        .replace(/[\r\n`]/g, '')
        .slice(0, 500);
      const safeMessage = message
        ? String(message)
            .replace(/[\r\n]/g, ' ')
            .slice(0, 1000)
        : '';

      return {
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Fix Anvil violation ${safeWarningId} in ${safeFilePath}${line ? ` at line ${line}` : ''}.

${safeMessage ? `Warning: ${safeMessage}\n\n` : ''}## Known fix patterns:

- **AP-001** (broad-eslint-disable): Replace \`/* eslint-disable */\` with targeted \`// eslint-disable-next-line <rule>\` comments
- **AP-003** (untyped-any): Replace \`: any\` with a proper type or \`: unknown\` if the type is truly unknown
- **AP-004** (ts-ignore): Replace \`@ts-ignore\` with \`@ts-expect-error\` and add a description of the expected error
- **ARCH-***  (architecture violations): Restructure imports to respect layer boundaries

## Steps:
1. Read the file and understand the context around the violation
2. Apply the appropriate fix pattern above
3. Run \`anvil_check\` to verify the fix resolved the warning
4. If the fix is not straightforward, consider using \`anvil_suppress\` with a clear reason`,
            },
          },
        ],
      };
    }
  );
}
