import type { Context, Next, MiddlewareHandler } from 'hono';
import { isIP } from 'node:net';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

/**
 * Sentinel bucket key used when the request carries no client identity we can
 * trust (local dev, unit tests, a customer proxy in front of Vercel, or a
 * misconfigured edge). Every such request collapses into this ONE shared
 * bucket.
 *
 * This fails CLOSED on purpose: an absent or untrusted header must never let
 * each request mint a fresh per-request key, because that would turn the
 * limiter into a no-op an attacker could bypass simply by omitting or
 * garbling the header.
 */
const NO_TRUSTED_CLIENT_IP = 'shared:no-trusted-client-ip';

/**
 * Longest textual IPv6 address (an IPv4-mapped form such as
 * `0000:0000:0000:0000:0000:ffff:255.255.255.255`) is 45 characters. Anything
 * longer is not an IP literal — reject it before validation so a hostile,
 * oversized header value can never become a Map key.
 */
const MAX_IP_TEXT_LENGTH = 45;

/**
 * Coerce a raw header value into a trusted bucket key, or `undefined` if it is
 * not a single, well-formed IP literal we are willing to trust.
 *
 * Guards, in order: trim; reject empty / over-length (memory-DoS cap); reject
 * anything containing a comma (a multi-hop chain we cannot attribute to a
 * single trusted hop — see trust-boundary note); require `node:net`'s `isIP`
 * to recognise it as IPv4 or IPv6. Non-IP-shaped input falls through to the
 * shared sentinel, so garbage strings can never become bucket keys. Note the
 * limit of this guard: where these headers are client-controlled (non-Vercel
 * deployments), a caller can still rotate VALID IP literals to mint distinct
 * keys — the real boundary is running behind a trusted edge (see the
 * residual-risks note below).
 */
function asTrustedIp(raw: string | undefined): string | undefined {
  if (!raw) return undefined;
  const value = raw.trim();
  if (value.length === 0 || value.length > MAX_IP_TEXT_LENGTH) return undefined;
  if (value.includes(',')) return undefined;
  return isIP(value) === 0 ? undefined : value;
}

/**
 * Resolve the rate-limit bucket key from a client-identity signal the Vercel
 * edge establishes — never a value the client can set directly.
 *
 * TRUST BOUNDARY — Vercel request headers.
 *
 *   - `x-real-ip` (PRIMARY): Vercel's own first-party SDK keys on this header
 *     and this header ONLY. The `@vercel/functions` package's `ipAddress()`
 *     helper reads `IP_HEADER_NAME = 'x-real-ip'` and nothing else (verified
 *     against `@vercel/functions` 3.7.1). It is the single strongest signal
 *     of the edge-observed client IP, so we prefer it.
 *   - `x-vercel-forwarded-for` (SECONDARY): a platform-prefixed variant, used
 *     ONLY as a single opaque IP fallback. We do NOT split it or read any
 *     "rightmost hop" — that chain-position trust model is UNVERIFIED (the
 *     header appears nowhere in Vercel's shipped SDK), so treating a chain as
 *     trustworthy could hand an attacker the very control we are removing. If
 *     the value contains a comma it is rejected outright.
 *   - `x-forwarded-for`: the classic client-suppliable header. NEVER keyed on.
 *     This is the CIB-140 regression this fix removes — the old code used
 *     `c.req.header('x-forwarded-for').split(',')[0]`, i.e. the attacker's own
 *     prefix, which let a caller evade its limit (rotate fake prefixes) or
 *     frame another client (pin their IP).
 *
 * Every candidate is IP-shape validated (`asTrustedIp`); anything that is not
 * a bare IPv4/IPv6 literal falls through to the shared `NO_TRUSTED_CLIENT_IP`
 * sentinel (fail closed).
 *
 * RESIDUAL RISKS (accepted by design):
 *   - A customer proxy placed IN FRONT of Vercel can rewrite `x-real-ip`, so
 *     its clients collapse into one shared bucket. That degrades to a coarse
 *     shared limit on the auth endpoints — an availability trade-off we accept
 *     over trusting a spoofable value.
 *   - On NON-Vercel deployments these headers are fully client-controlled. The
 *     app's own `dev` script runs under `wrangler`, where nothing sets or
 *     strips them. There a caller can rotate VALID IP literals to mint
 *     distinct bucket keys and defeat the throttle (and grow the store until
 *     the window sweep evicts them); the IP-shape validation only blocks
 *     non-IP / comma-bearing / over-length garbage. Per-client limiting is
 *     only meaningful behind a trusted edge that owns these headers.
 *
 * REGRESSION TRAP: only ever key on a header the platform OVERWRITES with the
 * edge-observed client IP. Never re-introduce chain-splitting or trust a
 * header the client can append to.
 */
function resolveTrustedClientKey(c: Context): string {
  // PRIMARY: x-real-ip — the header Vercel's own SDK trusts exclusively.
  const realIp = asTrustedIp(c.req.header('x-real-ip'));
  if (realIp) return realIp;

  // SECONDARY: x-vercel-forwarded-for as a single opaque IP only (no chain
  // parsing); a comma-bearing value is rejected by `asTrustedIp`.
  const vercelForwarded = asTrustedIp(c.req.header('x-vercel-forwarded-for'));
  if (vercelForwarded) return vercelForwarded;

  return NO_TRUSTED_CLIENT_IP;
}

/**
 * Simple in-memory sliding-window rate limiter.
 * Best-effort on serverless (per-instance), but provides basic DoS protection.
 *
 * Keyed on a Vercel-established, IP-shape-validated client identity via
 * `resolveTrustedClientKey` (primary `x-real-ip`), NOT on client-suppliable
 * `x-forwarded-for` — see that function's trust boundary note (CIB-140).
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
    const ip = resolveTrustedClientKey(c);
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
      // The client IP identifies a person; the counters carry the operational
      // signal on their own (CIB-214).
      debug('rate limit exceeded', { count: entry.count, max });
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
