import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export function registerSuppressTool(server: McpServer): void {
  server.registerTool(
    'anvil_suppress',
    {
      title: 'Anvil Suppress',
      description:
        'Insert a time-boxed suppression comment for a specific warning. Requires a reason.',
      inputSchema: {
        filePath: z.string().describe('File path relative to workspace root'),
        warningId: z.string().describe('Warning ID to suppress (e.g., AP-003)'),
        line: z.number().describe('Line number to suppress (1-based)'),
        reason: z.string().min(1).describe('Reason for suppression (mandatory)'),
        workspaceRoot: z.string().describe('Workspace root directory'),
        expiryDays: z.number().optional().describe('Days until suppression expires (default: 30)'),
      },
      annotations: {
        readOnlyHint: false,
        destructiveHint: false,
        idempotentHint: true,
      },
    },
    async ({ filePath, warningId, line, reason, workspaceRoot, expiryDays }) => {
      try {
        const days = expiryDays ?? 30;
        const expiry = new Date();
        expiry.setDate(expiry.getDate() + days);
        const expiryStr = expiry.toISOString().split('T')[0];

        const absPath = join(workspaceRoot, filePath);
        const content = readFileSync(absPath, 'utf-8');
        const lines = content.split('\n');
        const lineIndex = line - 1;

        if (lineIndex < 0 || lineIndex >= lines.length) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify({
                  suppressed: false,
                  reason: `Line ${line} out of range (file has ${lines.length} lines)`,
                }),
              },
            ],
          };
        }

        // Detect indentation of the target line
        const indent = lines[lineIndex].match(/^(\s*)/)?.[1] ?? '';
        const comment = `${indent}// @anvil-ignore ${warningId}: ${reason} [expires: ${expiryStr}]`;

        // Insert suppression comment above the target line
        lines.splice(lineIndex, 0, comment);
        writeFileSync(absPath, lines.join('\n'), 'utf-8');

        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify(
                {
                  suppressed: true,
                  filePath,
                  line,
                  comment,
                  expiryDate: expiryStr,
                  warningId,
                },
                null,
                2
              ),
            },
          ],
        };
      } catch (error) {
        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify({
                error: error instanceof Error ? error.message : String(error),
              }),
            },
          ],
          isError: true,
        };
      }
    }
  );
}
