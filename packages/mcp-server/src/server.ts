import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import {
  registerCheckTool,
  registerFixTool,
  registerGateTool,
  registerQueryBoundaryTool,
  registerStatusTool,
  registerSuppressTool,
} from './tools/index.js';

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
 */
export function createAnvilMcpServer(options: AnvilMcpServerOptions = {}): McpServer {
  const { name = 'anvil-mcp-server', version = '0.1.0' } = options;

  const server = new McpServer({
    name,
    version,
  });

  // Register tools
  registerCheckTool(server);
  registerGateTool(server);
  registerStatusTool(server);
  registerFixTool(server);
  registerSuppressTool(server);
  registerQueryBoundaryTool(server);

  return server;
}

export { McpServer };
