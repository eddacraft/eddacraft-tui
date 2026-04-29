#!/usr/bin/env node

/**
 * Anvil MCP Server -- stdio entry point
 *
 * Start with: npx @eddacraft/anvil-mcp-server
 * Or via: anvil-mcp-server
 */

import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { createAnvilMcpServer } from './server.js';

async function main(): Promise<void> {
  const server = createAnvilMcpServer();

  const transport = new StdioServerTransport();
  await server.connect(transport);

  // Handle graceful shutdown
  const shutdown = async () => {
    await server.close();
    process.exit(0);
  };

  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
}

main().catch((error: unknown) => {
  console.error('Anvil MCP server failed to start:', error);
  process.exit(1);
});
