import type { MiddlewareHandler } from 'hono';
import { timingSafeEqual } from 'node:crypto';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

/**
 * Middleware that validates the Authorization: Bearer <ADMIN_KEY> header.
 * Uses timing-safe comparison to prevent timing attacks.
 */
export const adminAuth: MiddlewareHandler = async (c, next) => {
  const adminKey = process.env['ADMIN_KEY'];
  if (!adminKey) {
    debug('admin auth: ADMIN_KEY not configured');
    return c.json({ error: 'Server misconfigured' }, 500);
  }

  const header = c.req.header('Authorization');
  if (!header) {
    debug('admin auth: missing Authorization header');
    return c.json({ error: 'Authorization header required' }, 401);
  }

  const match = header.match(/^Bearer\s+(.+)$/);
  if (!match) {
    debug('admin auth: invalid authorization format');
    return c.json({ error: 'Invalid authorization format' }, 401);
  }

  const provided = match[1];

  // Timing-safe comparison
  const a = Buffer.from(adminKey, 'utf-8');
  const b = Buffer.from(provided, 'utf-8');

  if (a.length !== b.length || !timingSafeEqual(a, b)) {
    debug('admin auth: key mismatch');
    return c.json({ error: 'Forbidden' }, 403);
  }

  debug('admin auth: authorized');
  return await next();
};
