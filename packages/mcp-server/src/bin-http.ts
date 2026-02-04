#!/usr/bin/env node

/**
 * Anvil MCP Server -- Streamable HTTP entry point
 *
 * Start with: anvil-mcp-server-http
 *
 * Configuration via environment variables:
 *   ANVIL_MCP_PORT  -- port to listen on (default: 3000)
 *   ANVIL_MCP_HOST  -- host to bind to  (default: localhost)
 */

import { startHttpServer } from './transports/streamable-http.js';

const port = parseInt(process.env.ANVIL_MCP_PORT ?? '3000', 10);
const host = process.env.ANVIL_MCP_HOST ?? 'localhost';

async function main(): Promise<void> {
  const server = await startHttpServer({ port, host });

  // Use stderr so stdout stays clean for potential piped output.
  console.error(`Anvil MCP server (HTTP) listening at http://${host}:${port}/mcp`);

  process.on('SIGINT', async () => {
    await server.close();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await server.close();
    process.exit(0);
  });
}

main().catch((error: unknown) => {
  console.error('Anvil MCP HTTP server failed to start:', error);
  process.exit(1);
});
