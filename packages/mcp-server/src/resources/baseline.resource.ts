import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil://baseline` resource on the given MCP server.
 *
 * Returns the current architecture baseline JSON from `.anvil/architecture.json`.
 */
export function registerBaselineResource(server: McpServer, getWorkspaceRoot: () => string): void {
  server.registerResource(
    'baseline',
    'anvil://baseline',
    {
      title: 'Architecture Baseline',
      description:
        'Current architecture baseline including layers, boundaries, entry points, and violation snapshot.',
      mimeType: 'application/json',
    },
    async (uri) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { loadBaseline, baselineExists } = await import('@eddacraft/anvil-core');

        if (!baselineExists(workspaceRoot)) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    error: 'no-baseline',
                    message: 'No architecture baseline found. Run `anvil init` to create one.',
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        const baseline = loadBaseline(workspaceRoot);

        if (!baseline) {
          return {
            contents: [
              {
                uri: uri.href,
                mimeType: 'application/json',
                text: JSON.stringify(
                  {
                    error: 'baseline-load-failed',
                    message: 'Architecture baseline exists but could not be loaded.',
                  },
                  null,
                  2
                ),
              },
            ],
          };
        }

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify(baseline, null, 2),
            },
          ],
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify({ error: message }, null, 2),
            },
          ],
        };
      }
    }
  );
}
