import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { readFileSync, writeFileSync, realpathSync } from 'node:fs';
import { resolve, relative } from 'node:path';

export function registerSuppressTool(server: McpServer, getWorkspaceRoot: () => string): void {
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
        expiryDays: z.number().optional().describe('Days until suppression expires (default: 30)'),
      },
      annotations: {
        readOnlyHint: false,
        destructiveHint: true,
        idempotentHint: false,
      },
    },
    async ({ filePath, warningId, line, reason, expiryDays }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        // Validate path stays within workspace (logical + symlink check)
        const absPath = resolve(workspaceRoot, filePath);
        const rel = relative(workspaceRoot, absPath);
        if (rel.startsWith('..') || resolve(workspaceRoot, rel) !== absPath) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify({
                  suppressed: false,
                  reason: `Path "${filePath}" resolves outside workspace root`,
                }),
              },
            ],
          };
        }

        // Resolve symlinks to prevent escaping via symlink targets
        const realRoot = realpathSync(workspaceRoot);
        const realAbs = realpathSync(absPath);
        if (!realAbs.startsWith(realRoot + '/') && realAbs !== realRoot) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify({
                  suppressed: false,
                  reason: `Path "${filePath}" resolves outside workspace root (symlink)`,
                }),
              },
            ],
          };
        }

        const days = expiryDays ?? 30;
        const expiry = new Date();
        expiry.setDate(expiry.getDate() + days);
        const expiryStr = expiry.toISOString().split('T')[0];
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
        const comment = `${indent}// @anvil-ignore-until ${expiryStr} ${warningId}: ${reason}`;

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
