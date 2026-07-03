import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

interface ThrottleEntry {
  count: number;
  resetAt: number;
}

export interface EmailThrottleOptions {
  /** Length of the fixed window in milliseconds. */
  windowMs?: number;
  /** Maximum submissions allowed per email per window. */
  max?: number;
  /** Hard cap on the number of tracked SUB-CAP (evictable) buckets. */
  maxKeys?: number;
  /** Hard cap on the number of tracked penalised (eviction-exempt) buckets. */
  maxPenalisedKeys?: number;
}

export interface EmailThrottleResult {
  limited: boolean;
  retryAfterSeconds: number;
}

export interface EmailThrottle {
  /**
   * Record one submission for `email` and report whether that email is now
   * over the cap. Both successful and failed submissions are meant to call
   * this — the throttle counts intent to submit, not outcome.
   */
  consume(email: string): EmailThrottleResult;
  /** Test-only: clear all tracked windows so suites don't leak state. */
  reset(): void;
}

const GMAIL_DOMAINS = new Set(['gmail.com', 'googlemail.com']);

/**
 * Canonicalise an email into a throttle bucket key so that addresses which
 * deliver to the SAME mailbox share ONE bucket. This only ever MERGES buckets,
 * never splits them, so it can only make the throttle stricter — it can never
 * loosen the cap.
 *
 * Rules, in order:
 *   1. Trim surrounding whitespace, then lowercase (case/whitespace variants).
 *   2. Strip a sub-address `+tag` — everything from the first `+` in the local
 *      part up to the `@` — for ALL domains. Plus-addressing is widely
 *      supported and, crucially, stripping it can only merge distinct-looking
 *      addresses that hit one inbox (`victim+1@x` and `victim+2@x` → one
 *      bucket), so it is always safe. This closes the `+tag` iteration bypass.
 *   3. For Gmail only (`gmail.com` / `googlemail.com`, which ignore dots in the
 *      local part), strip dots from the local part so `vic.tim@gmail.com` and
 *      `victim@gmail.com` share a bucket. Deliberately Gmail-scoped: most other
 *      providers treat dots as significant, so stripping dots globally could
 *      merge two genuinely-different mailboxes and over-throttle an innocent
 *      third party.
 *
 * Degenerate case: if stripping the `+tag` (or Gmail dots) collapses the local
 * part to empty (e.g. `+tag@x.com` → `@x.com`), the key becomes a single
 * per-domain shared bucket. This is merge-only — several already-degenerate
 * addresses on one domain share a throttle — so it can only tighten the cap,
 * never split a bucket or bypass it, and is left as-is.
 */
export function normaliseEmailKey(email: string): string {
  const trimmed = email.trim().toLowerCase();
  const at = trimmed.lastIndexOf('@');
  // No usable local part (`@x`, `x`, ``): key on the trimmed value as-is.
  if (at <= 0) return trimmed;

  let local = trimmed.slice(0, at);
  const domain = trimmed.slice(at + 1);

  const plus = local.indexOf('+');
  if (plus !== -1) local = local.slice(0, plus);

  if (GMAIL_DOMAINS.has(domain)) local = local.replaceAll('.', '');

  return `${local}@${domain}`;
}

/**
 * Per-email, IP-independent submission throttle for the public waitlist
 * signup endpoint.
 *
 * WHY IT SITS ALONGSIDE THE GLOBAL LIMITER (CIB-140): the shared
 * `rateLimiter` keys on a Vercel-established client IP (`x-real-ip`). That
 * protects against a single source flooding the API, but it does NOTHING to
 * stop signup abuse / email-bombing of a specific mailbox from many source
 * IPs, and on non-Vercel deployments the IP header is client-controlled. This
 * throttle deliberately keys on the SUBMITTED email — a client-supplied value
 * — because the abuse we're closing is repeated targeting of one mailbox
 * regardless of source. It is a complement to, not a replacement for, the
 * per-IP limiter.
 *
 * EVICTION / MEMORY MODEL (CIB-142 adversarial review): buckets are split into
 * two stores so eviction can never reset an active penalty —
 *   - `active`   : buckets still under the cap (`count < max`). Insertion-
 *                  ordered, so eviction is O(1) FIFO (`keys().next()`), and
 *                  evicting one only forfeits a partial sub-cap count.
 *   - `penalised`: buckets at/over the cap (`count >= max`). NEVER evicted by
 *                  the `active` FIFO, so an attacker CANNOT spray distinct
 *                  emails to push a victim's penalised bucket out of the store
 *                  and reset its count. Bounded independently.
 * There is no O(n) scan on the hot path (the earlier "evict globally-oldest"
 * design was both a CPU-amplification vector and a FIFO cap-reset bypass).
 *
 * POSTURE: in-memory and best-effort, exactly like `rateLimiter` — per
 * serverless instance, reset on cold start, fixed window, and defeatable at
 * scale by an attacker who can spread load across many warm instances. It
 * raises the cost of the naive email-bomb without pretending to be a durable,
 * cross-instance quota.
 *
 * ACKNOWLEDGED RESIDUAL (victim-email griefing): the key is client-supplied
 * with no proof of mailbox ownership, so anyone can spend a victim address's
 * budget and briefly (`windowMs`) throttle that address's own signup. Accepted:
 * the endpoint is an unauthenticated marketing form, the effect is bounded and
 * self-healing, and the alternative (no per-email throttle) leaves the mailbox
 * open to bombing — the higher-stakes harm.
 */
export function createEmailThrottle(opts?: EmailThrottleOptions): EmailThrottle {
  const windowMs = opts?.windowMs ?? 60 * 60 * 1000; // 1 hour
  const max = opts?.max ?? 3;
  const maxKeys = opts?.maxKeys ?? 10_000;
  const maxPenalisedKeys = opts?.maxPenalisedKeys ?? 10_000;
  debug('waitlist email throttle initialised', { windowMs, max, maxKeys, maxPenalisedKeys });

  // Sub-cap, evictable buckets (count < max), kept in insertion order.
  const active = new Map<string, ThrottleEntry>();
  // At/over-cap, eviction-exempt buckets (count >= max).
  const penalised = new Map<string, ThrottleEntry>();

  // Periodic sweep of expired windows to bound memory, mirroring `rateLimiter`.
  // `unref()` so it never keeps the process alive.
  setInterval(() => {
    const now = Date.now();
    for (const [key, entry] of active) {
      if (now >= entry.resetAt) active.delete(key);
    }
    for (const [key, entry] of penalised) {
      if (now >= entry.resetAt) penalised.delete(key);
    }
  }, windowMs).unref();

  return {
    consume(email: string): EmailThrottleResult {
      const key = normaliseEmailKey(email);
      const now = Date.now();

      let entry = penalised.get(key) ?? active.get(key);
      if (entry && now >= entry.resetAt) {
        // Window elapsed — drop the stale bucket wherever it lived and start
        // fresh (fixed window).
        active.delete(key);
        penalised.delete(key);
        entry = undefined;
      }

      if (!entry) {
        if (!active.has(key) && active.size >= maxKeys) {
          // Evict the OLDEST sub-cap bucket (O(1) FIFO). `active` never holds a
          // penalised bucket, so this can never reset a victim past the cap.
          const oldest = active.keys().next().value;
          if (oldest !== undefined) active.delete(oldest);
        }
        entry = { count: 0, resetAt: now + windowMs };
        active.set(key, entry);
      }

      entry.count += 1;

      if (entry.count >= max && active.has(key)) {
        // Promote into the eviction-exempt penalised set the moment the bucket
        // reaches the cap (one submission before it actually throttles), so it
        // can never be FIFO-evicted and reset.
        active.delete(key);
        if (!penalised.has(key) && penalised.size >= maxPenalisedKeys) {
          // Extreme distributed attack only: >= maxPenalisedKeys DISTINCT
          // victims penalised at once. Shed the oldest-promoted penalty in
          // O(1) via Map insertion order (FIFO). This is a memory bound, not a
          // precise expiry policy: because resetAt is fixed at bucket creation
          // (not promotion), the oldest-promoted entry is not guaranteed to be
          // the strict soonest-to-expire when buckets were created at different
          // times — but it is close, and a true min-resetAt scan would restore
          // the O(n)-per-insert cost this design removed. Documented residual —
          // reaching it requires penalising thousands of distinct mailboxes
          // simultaneously, not a single-target bomb.
          const oldestPenalised = penalised.keys().next().value;
          if (oldestPenalised !== undefined) penalised.delete(oldestPenalised);
        }
        penalised.set(key, entry);
      }

      const limited = entry.count > max;
      if (limited) {
        debug('waitlist email throttle exceeded', { count: entry.count, max });
      }
      return {
        limited,
        retryAfterSeconds: Math.max(1, Math.ceil((entry.resetAt - now) / 1000)),
      };
    },
    reset(): void {
      active.clear();
      penalised.clear();
    },
  };
}

/**
 * Shared instance mounted by the public waitlist signup route. One process-wide
 * budget per email; the route imports this so every request funnels through the
 * same store. Tests import it to `reset()` between cases.
 */
export const waitlistEmailThrottle = createEmailThrottle();
