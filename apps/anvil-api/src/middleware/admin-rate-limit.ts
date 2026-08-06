import type { Context, MiddlewareHandler, Next } from 'hono';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

// Registry of every store ever created so test code can reset all
// rate-limit state between cases. The mounting site (route module)
// constructs the middleware exactly once at import time, so without
// this the bucket persists across the entire vitest run and a single
// noisy test would 429 every test that comes after it.
const allStores = new Set<Map<string, RateLimitEntry>>();

/** Test-only: drop all per-actor counters across every middleware. */
export function _resetAdminRateLimitForTests(): void {
  for (const s of allStores) s.clear();
}

export interface AdminRateLimitOptions {
  /** Window length in milliseconds. */
  windowMs: number;
  /** Maximum requests permitted per actor within the window. */
  max: number;
  /**
   * Namespace label included in the bucket key. Use a distinct scope
   * per route group (e.g. `'all'` for the coarse per-actor limit and
   * `'send-migration'` for the dedicated send-migration cap) so the
   * counters do not share state.
   */
  scope: string;
}

/**
 * Per-admin-actor rate limiter for admin routes.
 *
 * Keys on `adminActor` (set by `adminAuth`), so per-operator keys each
 * get their own bucket and a compromised key cannot burst through the
 * whole admin surface before audit-log review. Shared-key callers
 * collapse into a single `shared-key@anvil` bucket — that is the
 * intended posture during the dual-auth rollout window: the shared
 * path is a fallback, not a sustained workload.
 *
 * Best-effort on serverless (per-instance counter, no Redis). The
 * goal is to make a compromised key visibly noisy, not to enforce a
 * cluster-wide quota.
 */
export function adminRateLimit(opts: AdminRateLimitOptions): MiddlewareHandler {
  const { windowMs, max, scope } = opts;
  debug('admin rate limiter initialized', { windowMs, max, scope });
  const store = new Map<string, RateLimitEntry>();
  allStores.add(store);

  // Periodic cleanup keeps `store` from growing unboundedly when
  // distinct actors come and go.
  setInterval(() => {
    const now = Date.now();
    for (const [key, entry] of store) {
      if (now >= entry.resetAt) {
        store.delete(key);
      }
    }
  }, windowMs).unref();

  return async (c: Context, next: Next) => {
    // Resolve the actor identity. The middleware is intended to sit
    // *after* `adminAuth`, but the request may also reach this point
    // before auth attached (programmatic mounts in tests, mis-ordered
    // middleware, etc.). Fall back to a sentinel so the limiter still
    // produces a deterministic bucket rather than throwing.
    const actor = c.get('adminActor') ?? 'unauthenticated';
    const key = `${scope}:${actor}`;
    const now = Date.now();

    let entry = store.get(key);
    if (!entry || now >= entry.resetAt) {
      entry = { count: 0, resetAt: now + windowMs };
      store.set(key, entry);
    }
    entry.count++;

    const retryAfterSeconds = Math.max(0, Math.ceil((entry.resetAt - now) / 1000));
    c.res.headers.set('X-RateLimit-Limit', String(max));
    c.res.headers.set('X-RateLimit-Remaining', String(Math.max(0, max - entry.count)));
    c.res.headers.set('X-RateLimit-Reset', String(Math.ceil(entry.resetAt / 1000)));
    c.res.headers.set('X-RateLimit-Scope', scope);

    if (entry.count > max) {
      debug('admin rate limit exceeded', { scope, count: entry.count, max });
      c.res.headers.set('Retry-After', String(retryAfterSeconds));
      return c.json(
        {
          error: 'Admin rate limit exceeded',
          code: 'admin_rate_limited',
          scope,
          retry_after_seconds: retryAfterSeconds,
        },
        429
      );
    }

    return await next();
  };
}
