import type { Context, Next, MiddlewareHandler } from 'hono';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

/**
 * Simple in-memory sliding-window rate limiter.
 * Best-effort on serverless (per-instance), but provides basic DoS protection.
 */
export function rateLimiter(opts?: { windowMs?: number; max?: number }): MiddlewareHandler {
  const windowMs = opts?.windowMs ?? 60_000;
  const max = opts?.max ?? 60;
  debug('rate limiter initialized', { windowMs, max });
  const store = new Map<string, RateLimitEntry>();

  // Periodic cleanup to prevent memory growth
  setInterval(() => {
    const now = Date.now();
    for (const [key, entry] of store) {
      if (now >= entry.resetAt) {
        store.delete(key);
      }
    }
  }, windowMs).unref();

  return async (c: Context, next: Next) => {
    const ip = c.req.header('x-forwarded-for')?.split(',')[0]?.trim() ?? 'unknown';
    const now = Date.now();

    let entry = store.get(ip);
    if (!entry || now >= entry.resetAt) {
      entry = { count: 0, resetAt: now + windowMs };
      store.set(ip, entry);
    }

    entry.count++;

    c.res.headers.set('X-RateLimit-Limit', String(max));
    c.res.headers.set('X-RateLimit-Remaining', String(Math.max(0, max - entry.count)));
    c.res.headers.set('X-RateLimit-Reset', String(Math.ceil(entry.resetAt / 1000)));

    if (entry.count > max) {
      debug('rate limit exceeded', { ip, count: entry.count, max });
      return c.json({ error: 'Too many requests, please try again later' }, 429);
    }

    return await next();
  };
}

/**
 * Process-wide fixed-window limiter: one budget shared by ALL callers, for
 * endpoints that proxy a credentialed upstream call (e.g. the GitHub
 * device-flow broker, ADR-066 ops precondition). Layer it behind `rateLimiter`
 * so a single IP exhausts its own budget before it can drain the shared one.
 * Best-effort per instance, like `rateLimiter`.
 *
 * NOTE: each call mints an independent budget — mount the SAME returned
 * middleware on every route that should share one, or they fork silently.
 */
export function globalRateLimiter(opts?: { windowMs?: number; max?: number }): MiddlewareHandler {
  const windowMs = opts?.windowMs ?? 60_000;
  const max = opts?.max ?? 60;
  debug('global rate limiter initialized', { windowMs, max });
  let count = 0;
  let resetAt = 0;

  return async (c: Context, next: Next) => {
    const now = Date.now();
    if (now >= resetAt) {
      count = 0;
      resetAt = now + windowMs;
    }
    count++;

    if (count > max) {
      debug('global rate limit exceeded', { count, max });
      c.res.headers.set('Retry-After', String(Math.max(1, Math.ceil((resetAt - now) / 1000))));
      return c.json({ error: 'Too many requests, please try again later' }, 429);
    }

    return await next();
  };
}
