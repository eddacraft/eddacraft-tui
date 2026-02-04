/**
 * @eddacraft/anvil-mcp-server
 *
 * MCP server exposing Anvil validation as tools for AI assistants.
 * Supports stdio and HTTP transports.
 *
 * @module @eddacraft/anvil-mcp-server
 */

export { createAnvilMcpServer } from './server.js';
export type { AnvilMcpServerOptions } from './server.js';

export { generateMcpConfig, SUPPORTED_TARGETS } from './config/index.js';
export type { McpConfig, McpConfigOptions, McpConfigTarget } from './config/index.js';

export { startHttpServer } from './transports/index.js';
export type { HttpTransportOptions, HttpServerHandle } from './transports/index.js';
