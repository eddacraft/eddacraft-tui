import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil://constraints` resource on the given MCP server.
 *
 * Returns aggregated llms.txt-style constraints collected from the
 * architecture baseline, anti-pattern catalogue, and project conventions.
 */
export function registerConstraintsResource(
  server: McpServer,
  getWorkspaceRoot: () => string
): void {
  server.registerResource(
    'constraints',
    'anvil://constraints',
    {
      title: 'Aggregated Constraints',
      description:
        'All project constraints (boundaries, anti-patterns, conventions) aggregated for AI consumption.',
      mimeType: 'application/json',
    },
    async (uri) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { collectConstraints } = await import('@eddacraft/anvil-runtime');

        const constraints = await collectConstraints(workspaceRoot);

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify(constraints, null, 2),
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
