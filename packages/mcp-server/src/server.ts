import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import {
  registerCheckTool,
  registerFixTool,
  registerGateTool,
  registerQueryBoundaryTool,
  registerStatusTool,
  registerSuppressTool,
} from './tools/index.js';
import {
  registerFixViolationPrompt,
  registerSuppressViolationPrompt,
  registerArchitectureReviewPrompt,
  registerPreGenerationPrompt,
} from './prompts/index.js';
import {
  registerBaselineResource,
  registerBoundariesResource,
  registerPatternsResource,
  registerSuppressionsResource,
  registerConfigResource,
  registerConstraintsResource,
  registerDriftResource,
  registerFileWarningsResource,
} from './resources/index.js';

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
  const { name = 'anvil-mcp-server', version = '0.1.0', projectRoot } = options;

  const server = new McpServer({
    name,
    version,
  });

  // Workspace root resolution: use projectRoot option or fall back to cwd
  const getWorkspaceRoot = (): string => {
    if (projectRoot) return projectRoot;
    return process.cwd();
  };

  // Register tools
  registerCheckTool(server);
  registerGateTool(server);
  registerStatusTool(server);
  registerFixTool(server);
  registerSuppressTool(server);
  registerQueryBoundaryTool(server);

  // Register prompts
  registerFixViolationPrompt(server);
  registerSuppressViolationPrompt(server);
  registerArchitectureReviewPrompt(server);
  registerPreGenerationPrompt(server);

  // Register resources
  registerBaselineResource(server, getWorkspaceRoot);
  registerBoundariesResource(server, getWorkspaceRoot);
  registerPatternsResource(server);
  registerSuppressionsResource(server, getWorkspaceRoot);
  registerConfigResource(server, getWorkspaceRoot);
  registerConstraintsResource(server, getWorkspaceRoot);
  registerDriftResource(server, getWorkspaceRoot);
  registerFileWarningsResource(server, getWorkspaceRoot);

  return server;
}

export { McpServer };
