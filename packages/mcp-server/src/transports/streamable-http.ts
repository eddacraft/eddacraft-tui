import express from 'express';
import type { Request, Response, NextFunction } from 'express';
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

// ---------------------------------------------------------------------------
// Simple in-memory rate limiter (per IP, per minute)
// ---------------------------------------------------------------------------
interface RateBucket {
  count: number;
  resetAt: number;
}

const rateBuckets = new Map<string, RateBucket>();

function getRateLimit(): number {
  const env = process.env['ANVIL_MCP_RATE_LIMIT'];
  if (env) {
    const parsed = parseInt(env, 10);
    if (!Number.isNaN(parsed) && parsed > 0) return parsed;
  }
  return 60; // default: 60 requests per minute
}

function isLocalhost(ip: string): boolean {
  return ip === '127.0.0.1' || ip === '::1' || ip === '::ffff:127.0.0.1';
}

function rateLimitMiddleware(req: Request, res: Response, next: NextFunction): void {
  const ip = req.ip ?? req.socket.remoteAddress ?? 'unknown';
  if (isLocalhost(ip)) {
    next();
    return;
  }

  const limit = getRateLimit();
  const now = Date.now();
  const windowMs = 60_000; // 1 minute

  let bucket = rateBuckets.get(ip);
  if (!bucket || now >= bucket.resetAt) {
    bucket = { count: 0, resetAt: now + windowMs };
    rateBuckets.set(ip, bucket);
  }

  bucket.count++;

  if (bucket.count > limit) {
    const retryAfter = Math.ceil((bucket.resetAt - now) / 1000);
    res.set('Retry-After', String(retryAfter));
    res.status(429).json({
      jsonrpc: '2.0',
      error: { code: -32000, message: 'Too Many Requests' },
      id: null,
    });
    return;
  }

  next();
}

// Periodically clean up expired rate-limit buckets (every 5 minutes)
setInterval(() => {
  const now = Date.now();
  for (const [ip, bucket] of rateBuckets) {
    if (now >= bucket.resetAt) {
      rateBuckets.delete(ip);
    }
  }
}, 300_000).unref();

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

  // Security headers
  app.use((_req: Request, res: Response, next: NextFunction) => {
    res.set('X-Content-Type-Options', 'nosniff');
    res.set('X-Frame-Options', 'DENY');
    res.set('Cache-Control', 'no-store');

    // HSTS only for non-localhost deployments
    if (!isLocalhost(host)) {
      res.set('Strict-Transport-Security', 'max-age=31536000; includeSubDomains');
    }

    next();
  });

  // Rate limiting
  app.use(rateLimitMiddleware);

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
    const rawContentType = req.headers['content-type'];
    const baseType =
      typeof rawContentType === 'string'
        ? rawContentType.split(';', 1)[0]?.trim().toLowerCase()
        : undefined;
    if (baseType !== 'application/json') {
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
