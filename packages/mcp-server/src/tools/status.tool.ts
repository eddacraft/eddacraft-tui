import { z } from 'zod';
import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil_status` tool on the given MCP server.
 *
 * The tool returns a quick health summary for a project including
 * available checks, config details, and baseline status.
 */
export function registerStatusTool(server: McpServer): void {
  server.registerTool(
    'anvil_status',
    {
      title: 'Anvil Status',
      description:
        'Quick project health summary. Returns available checks, configuration info, and baseline status.',
      inputSchema: {
        workspaceRoot: z.string().describe('Absolute path to the project root directory'),
      },
      annotations: {
        readOnlyHint: true,
        destructiveHint: false,
        idempotentHint: true,
      },
    },
    async ({ workspaceRoot }) => {
      try {
        const { GateRunner, GateConfigManager } = await import('@eddacraft/anvil-runtime');
        const { baselineExists } = await import('@eddacraft/anvil-core');

        const runner = new GateRunner();
        const availableChecks = runner.getAvailableChecks();

        let configInfo: {
          loaded: boolean;
          source: string | null;
          checks: string[];
        } = { loaded: false, source: null, checks: [] };

        try {
          const configManager = new GateConfigManager(workspaceRoot);
          const result = configManager.loadConfigWithDetails();
          configInfo = {
            loaded: !result.isDefault,
            source: result.path,
            checks: result.config.checks.filter((c) => c.enabled).map((c) => c.name),
          };
        } catch (configError) {
          configInfo = {
            loaded: false,
            source: null,
            checks: [],
            error: configError instanceof Error ? configError.message : String(configError),
          } as typeof configInfo & { error: string };
        }

        const hasBaseline = baselineExists(workspaceRoot);

        return {
          content: [
            {
              type: 'text' as const,
              text: JSON.stringify(
                {
                  status: 'ok',
                  workspaceRoot,
                  availableChecks,
                  config: configInfo,
                  hasBaseline,
                  version: '0.1.0',
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
