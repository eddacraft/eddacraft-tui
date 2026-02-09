import express from 'express';
import { randomUUID } from 'node:crypto';
import type { Server } from 'node:http';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { isInitializeRequest } from '@modelcontextprotocol/sdk/types.js';
import { createAnvilMcpServer } from '../server.js';
import type { AnvilMcpServerOptions } from '../server.js';

export interface HttpTransportOptions extends AnvilMcpServerOptions {
  /** Port to listen on (default: 3000) */
  port?: number;
  /** Host to bind to (default: 'localhost') */
  host?: string;
}

export interface HttpServerHandle {
  /** Gracefully shut down the HTTP server and all active sessions. */
  close: () => Promise<void>;
  /** The underlying Node.js HTTP server (useful for tests). */
  httpServer: Server;
}

/**
 * Starts an Express-based HTTP server with the MCP Streamable HTTP transport.
 *
 * Each MCP session gets its own `StreamableHTTPServerTransport` instance and a
 * fresh `McpServer` wired up via `createAnvilMcpServer`.
 */
export async function startHttpServer(
  options: HttpTransportOptions = {}
): Promise<HttpServerHandle> {
  const { port = 3000, host = 'localhost', ...serverOptions } = options;

  const app = express();
  app.use(express.json());

  // API key authentication middleware (opt-in via ANVIL_MCP_API_KEY env var)
  const apiKey = process.env['ANVIL_MCP_API_KEY'];
  if (apiKey) {
    app.use('/mcp', (req, res, next) => {
      const provided = req.headers['authorization'];
      if (!provided || provided !== `Bearer ${apiKey}`) {
        res.status(401).json({
          jsonrpc: '2.0',
          error: { code: -32001, message: 'Unauthorized: invalid or missing API key' },
          id: null,
        });
        return;
      }
      next();
    });
  }

  // Active sessions keyed by their session ID.
  const transports = new Map<string, StreamableHTTPServerTransport>();

  // --- POST /mcp ---------------------------------------------------------
  app.post('/mcp', async (req, res) => {
    // Validate Content-Type for POST requests
    const contentType = req.headers['content-type'];
    if (!contentType || !contentType.includes('application/json')) {
      res.status(415).json({
        jsonrpc: '2.0',
        error: { code: -32700, message: 'Content-Type must be application/json' },
        id: null,
      });
      return;
    }
    const sessionId = req.headers['mcp-session-id'] as string | undefined;
    let transport: StreamableHTTPServerTransport;

    if (sessionId && transports.has(sessionId)) {
      // Existing session -- reuse the transport.
      transport = transports.get(sessionId)!;
    } else if (!sessionId && isInitializeRequest(req.body)) {
      // New session -- create transport + wire up a fresh MCP server.
      transport = new StreamableHTTPServerTransport({
        sessionIdGenerator: () => randomUUID(),
        onsessioninitialized: (id) => {
          transports.set(id, transport);
        },
        onsessionclosed: (id) => {
          transports.delete(id);
        },
      });

      transport.onclose = () => {
        const id = transport.sessionId;
        if (id) transports.delete(id);
      };

      const server = createAnvilMcpServer(serverOptions);
      await server.connect(transport);
    } else {
      // Invalid: no session and not an initialize request.
      res.status(400).json({
        jsonrpc: '2.0',
        error: {
          code: -32000,
          message: 'Invalid session. Send initialize request without session ID.',
        },
        id: null,
      });
      return;
    }

    await transport.handleRequest(req, res, req.body);
  });

  // --- GET /mcp (SSE stream) ---------------------------------------------
  app.get('/mcp', async (req, res) => {
    const sessionId = req.headers['mcp-session-id'] as string;
    const transport = sessionId ? transports.get(sessionId) : undefined;

    if (transport) {
      await transport.handleRequest(req, res);
    } else {
      res.status(400).json({ error: 'Invalid session' });
    }
  });

  // --- DELETE /mcp (session termination) ----------------------------------
  app.delete('/mcp', async (req, res) => {
    const sessionId = req.headers['mcp-session-id'] as string;
    const transport = sessionId ? transports.get(sessionId) : undefined;

    if (transport) {
      await transport.handleRequest(req, res);
    } else {
      res.status(400).json({ error: 'Invalid session' });
    }
  });

  // --- Health check -------------------------------------------------------
  app.get('/health', (_req, res) => {
    res.json({ status: 'ok', sessions: transports.size });
  });

  // Start the HTTP server.
  const httpServer: Server = await new Promise((resolve) => {
    const srv = app.listen(port, host, () => {
      resolve(srv);
    });
  });

  return {
    httpServer,
    close: async () => {
      // Tear down every active session transport first.
      const closePromises = [...transports.values()].map((t) => t.close());
      try {
        await Promise.all(closePromises);
      } finally {
        await new Promise<void>((resolve, reject) => {
          httpServer.close((err) => (err ? reject(err) : resolve()));
        });
      }
    },
  };
}
