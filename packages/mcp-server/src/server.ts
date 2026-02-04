import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';

export interface AnvilMcpServerOptions {
  /** Project root directory (overridden by client roots) */
  projectRoot?: string;
  /** Server name for MCP handshake */
  name?: string;
  /** Server version for MCP handshake */
  version?: string;
}

/**
 * Creates and configures the Anvil MCP server with all tools, resources, and prompts.
 *
 * Currently registers placeholder handlers. Real tool implementations will be added
 * in MCP-002 through MCP-007.
 */
export function createAnvilMcpServer(options: AnvilMcpServerOptions = {}): McpServer {
  const { name = 'anvil-mcp-server', version = '0.1.0' } = options;

  const server = new McpServer({
    name,
    version,
  });

  // Register placeholder tools
  // Real implementations will be added in:
  // - MCP-002: anvil_check
  // - MCP-003: anvil_gate, anvil_status
  // - MCP-004: anvil_fix, anvil_suppress
  // - MCP-005: anvil_query_boundary

  registerPlaceholderTools(server);

  return server;
}

function registerPlaceholderTools(server: McpServer): void {
  // Register a minimal anvil_status tool so the server has at least one tool
  // for handshake validation. Full implementations come in later tasks.
  server.registerTool(
    'anvil_status',
    {
      title: 'Anvil Status',
      description: 'Returns current Anvil project health summary. Full implementation pending.',
      inputSchema: {},
    },
    async () => {
      return {
        content: [
          {
            type: 'text' as const,
            text: JSON.stringify({
              status: 'ok',
              message: 'Anvil MCP server is running. Tool implementations pending.',
              version: '0.1.0',
            }),
          },
        ],
      };
    }
  );
}

export { McpServer };
