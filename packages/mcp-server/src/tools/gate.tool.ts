import { z } from 'zod';
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import type { PlanData } from '@eddacraft/anvil-runtime';
import { validateWorkspaceRoot } from '../utils/validate-workspace.js';

/**
 * Registers the `anvil_gate` tool on the given MCP server.
 *
 * The tool runs the full Anvil quality gate on a project, or analyses specific
 * files in planless mode when `targetFiles` is supplied.
 */
export function registerGateTool(server: McpServer): void {
  server.registerTool(
    'anvil_gate',
    {
      title: 'Anvil Gate',
      description:
        'Run the full Anvil quality gate on a project. ' +
        'Supply targetFiles for planless file analysis, or omit for a full config-driven gate run.',
      inputSchema: {
        workspaceRoot: z.string().describe('Absolute path to the project root directory'),
        targetFiles: z
          .array(z.string())
          .optional()
          .describe('Specific files to analyse (planless mode). Omit for full gate run.'),
        skipChecks: z
          .array(z.string())
          .optional()
          .describe('Names of checks to skip during the gate run'),
        failFast: z.boolean().optional().describe('Stop on the first failing check'),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: false,
      },
    },
    async ({ workspaceRoot, targetFiles, skipChecks, failFast }) => {
      try {
        validateWorkspaceRoot(workspaceRoot);
        const { GateRunner, GateConfigManager } = await import('@eddacraft/anvil-runtime');
        const runner = new GateRunner();

        // Planless mode: analyse specific files
        if (targetFiles && targetFiles.length > 0) {
          const result = await runner.analyzeFiles(targetFiles, workspaceRoot);
          return {
            content: [
              {
                type: 'text' as const,
                text: JSON.stringify(
                  {
                    mode: 'planless',
                    checksRun: result.checksRun,
                    hasBlockingWarnings: result.hasBlockingWarnings,
                    executionTimeMs: result.executionTimeMs,
                    warnings: result.warnings,
                    suppressionStats: result.suppressionStats,
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        // Full gate run with config
        const configManager = new GateConfigManager(workspaceRoot);
        const config = configManager.loadConfig();
        // Minimal valid plan for planless full-codebase gate runs
        const emptyPlan: PlanData = {
          schema_version: '0.1.0',
          id: 'aps-00000000',
          hash: '0'.repeat(64),
          intent: 'Full codebase quality gate run (MCP)',
          proposed_changes: [],
          provenance: {
            timestamp: new Date().toISOString(),
            source: 'automation',
            version: '0.0.1',
          },
          validations: {
            required_checks: [],
            skip_checks: [],
          },
        };
        const result = await runner.runGate(emptyPlan, config, workspaceRoot, {
          skipChecks,
          failFast,
        });

        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify(
                {
                  mode: 'full',
                  overall: result.overall,
                  score: result.score,
                  summary: result.summary,
                  checks: result.checks.map((r) => ({
                    check: r.check,
                    passed: r.passed,
                    score: r.score,
                    message: r.message,
                    skipped: r.skipped,
                  })),
                  timing: result.timing,
                  cacheStats: result.cacheStats,
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
