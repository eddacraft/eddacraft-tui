import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

/**
 * Registers the `anvil://config` resource on the given MCP server.
 *
 * Returns the current Anvil gate configuration including checks,
 * thresholds, and the source file path.
 */
export function registerConfigResource(server: McpServer, getWorkspaceRoot: () => string): void {
  server.registerResource(
    'config',
    'anvil://config',
    {
      title: 'Gate Configuration',
      description:
        'Current Anvil gate configuration with enabled checks, thresholds, and config source.',
      mimeType: 'application/json',
    },
    async (uri) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { GateConfigManager } = await import('@eddacraft/anvil-runtime');

        const configManager = new GateConfigManager(workspaceRoot);
        const result = configManager.loadConfigWithDetails();

        return {
          contents: [
            {
              uri: uri.href,
              mimeType: 'application/json',
              text: JSON.stringify(
                {
                  config: result.config,
                  source: result.path,
                  isDefault: result.isDefault,
                  errors: result.errors,
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
