/**
 * anvil_query_boundary MCP tool
 *
 * Checks whether a module/file can import from another module/file
 * given the architecture rules defined in the project's baseline.
 *
 * Use BEFORE writing code to verify an import would not violate
 * architecture boundaries.
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { validateWorkspaceRoot } from '../utils/validate-workspace.js';

export function registerQueryBoundaryTool(server: McpServer): void {
  server.registerTool(
    'anvil_query_boundary',
    {
      title: 'Anvil Query Boundary',
      description:
        'Check if a file can import from another file given architecture boundary rules. Use before writing import statements to prevent violations.',
      inputSchema: {
        sourceFile: z.string().describe('File that wants to import (relative to workspace root)'),
        targetFile: z.string().describe('File being imported from (relative to workspace root)'),
        workspaceRoot: z.string().describe('Workspace root directory'),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
      },
    },
    async ({ sourceFile, targetFile, workspaceRoot }) => {
      try {
        validateWorkspaceRoot(workspaceRoot);
        const {
          baselineExists: checkBaseline,
          loadBaseline: getBaseline,
          createLayerDetector,
        } = await import('@eddacraft/anvil-core');

        // --- No baseline --------------------------------------------------
        if (!checkBaseline(workspaceRoot)) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify(
                  {
                    allowed: true,
                    reason: 'no-baseline',
                    message:
                      'No architecture baseline found. Run `anvil init` to create one. Without a baseline, all imports are allowed.',
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // --- Baseline load failed -----------------------------------------
        const baseline = getBaseline(workspaceRoot);
        if (!baseline) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify(
                  {
                    allowed: true,
                    reason: 'baseline-load-failed',
                    message: 'Could not load architecture baseline.',
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // --- Detect layers ------------------------------------------------
        const detector = createLayerDetector(baseline.layers);
        const sourceAssignment = detector.detectLayer(sourceFile);
        const targetAssignment = detector.detectLayer(targetFile);

        const sourceLayer = sourceAssignment.layer;
        const targetLayer = targetAssignment.layer;

        // --- Unassigned layer(s) ------------------------------------------
        if (!sourceLayer || !targetLayer) {
          const unassigned: string[] = [];
          if (!sourceLayer) unassigned.push(`source (${sourceFile})`);
          if (!targetLayer) unassigned.push(`target (${targetFile})`);

          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify(
                  {
                    allowed: true,
                    reason: 'unassigned-layer',
                    message: `Cannot determine layer for: ${unassigned.join(', ')}. Import is allowed by default.`,
                    sourceLayer: sourceLayer ?? null,
                    targetLayer: targetLayer ?? null,
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // --- Same layer ---------------------------------------------------
        if (sourceLayer === targetLayer) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify(
                  {
                    allowed: true,
                    reason: 'same-layer',
                    message: `Both files are in the "${sourceLayer}" layer. Same-layer imports are always allowed.`,
                    sourceLayer,
                    targetLayer,
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // --- Check boundary rules -----------------------------------------
        const allowed = detector.isAllowedDependency(sourceLayer, targetLayer, baseline.layers);

        if (!allowed) {
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify(
                  {
                    allowed: false,
                    reason: 'boundary-violation',
                    message: `Import from "${sourceLayer}" to "${targetLayer}" violates architecture boundaries.`,
                    sourceLayer,
                    targetLayer,
                    violation: {
                      from: sourceLayer,
                      to: targetLayer,
                    },
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify(
                {
                  allowed: true,
                  reason: 'boundary-ok',
                  message: `Import from "${sourceLayer}" to "${targetLayer}" is allowed by architecture rules.`,
                  sourceLayer,
                  targetLayer,
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
