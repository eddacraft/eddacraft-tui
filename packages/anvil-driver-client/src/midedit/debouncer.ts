/**
 * Mid-edit debouncer with content-hash dedup.
 *
 * Per RTAI-004: any TS driver should be able to emit mid-edit
 * validation requests with a built-in debouncer + content-hash dedup,
 * without re-implementing either in each surface.
 *
 * Responsibilities:
 *   - Coalesce a typing burst into a single request: a fresh call
 *     within `debounceMs` of the previous one (per `key`) replaces the
 *     pending request and rebuilds the timer.
 *   - Identical-content short-circuit: if the SHA-256 of the content
 *     matches the most recent successfully-resolved request for the
 *     same key, return the cached result without a round-trip.
 *   - Cancel-in-flight semantics: when a new request supersedes a
 *     pending one (debounce coalesce) the pending promise resolves to
 *     a `coalesced` outcome — the consumer's caller sees the LATER
 *     result, never an exception.
 *   - Per-call debounce override (e.g. `debounceMs: 0` for tests).
 *
 * Cooperative layer only — the daemon-side INTD-016 RPS bucket is the
 * protective layer per ADR-031. The debounce default of 80ms is the
 * typing cycle; ADR-031's mid-edit budget (50ms `validation.service`,
 * 80ms `validation.roundtrip`) lives on the daemon path.
 *
 * @see plans/decisions/031-validation-latency-rubric.md
 * @see plans/modules/realtime-ai-validation.aps.md (RTAI-004)
 */

import { createHash } from 'node:crypto';

/**
 * Default debounce window. The brief pins this at 80ms (a typing
 * cycle); consumers override per-call via `debounceMs`. Documented but
 * NOT enforced — the daemon-side rate limiter (INTD-016) is the
 * protective layer.
 */
export const DEFAULT_DEBOUNCE_MS = 80;

/**
 * Default content-hash dedup window. Within this sliding window after
 * a successful round-trip, an identical-content request short-circuits
 * with the cached result. Larger than the debounce because dedup
 * benefits paste-and-hold scenarios.
 */
export const DEFAULT_DEDUP_WINDOW_MS = 1_000;

export interface DebouncerScheduler {
  setTimeout: (cb: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
  /** Monotonic clock used for the dedup window. Defaults to
   *  `performance.now()` style, but consumers can inject a fake. */
  now: () => number;
}

const DEFAULT_SCHEDULER: DebouncerScheduler = {
  setTimeout: (cb, ms) => globalThis.setTimeout(cb, ms),
  clearTimeout: (handle) =>
    globalThis.clearTimeout(handle as ReturnType<typeof globalThis.setTimeout>),
  // Monotonic clock — Date.now() can move backwards under NTP / leap-second
  // adjustments, which would treat aged cache entries as still fresh
  // (negative duration < window) and contradict the docstring above.
  // performance.now() is available in Node 16+ and all modern browsers.
  now: () => globalThis.performance.now(),
};

export interface DebouncerOptions {
  /** Default debounce window. Per-call `debounceMs` wins. */
  debounceMs?: number;
  /** Sliding window after a successful response during which an
   *  identical-content request short-circuits with the cached result. */
  dedupWindowMs?: number;
  /** Test hook: replace the timer + clock. */
  scheduler?: DebouncerScheduler;
}

/**
 * Outcome of a debounced call. Either we ran the dispatcher and got
 * its return value (`fresh`), the call was dedup-short-circuited and
 * the cached value is returned (`cached`), or the call was superseded
 * by a later call within the debounce window (`coalesced`).
 *
 * `coalesced` is the cooperative-cancel outcome — the consumer's
 * caller sees the LATER call's result through their later promise.
 * The earlier promise resolves to `coalesced` so the caller knows
 * its dispatch was suppressed; this is intentional and not an error.
 */
export type DebouncedOutcome<R> =
  | { kind: 'fresh'; value: R }
  | { kind: 'cached'; value: R }
  | { kind: 'coalesced' };

/**
 * Compute the SHA-256 hex digest of `content`. Exposed so consumers
 * can short-circuit upstream (e.g. cache-key lookups) using the same
 * hash the debouncer uses.
 *
 * SHA-256 is chosen over a faster non-cryptographic hash because
 * dedup is keyed on user-controlled content; collisions would cause
 * stale cached results to be served. SHA-256 collision risk is
 * negligible for any realistic editor buffer.
 */
export function contentHashSha256(content: string): string {
  return createHash('sha256').update(content, 'utf8').digest('hex');
}

interface PendingDispatch<R> {
  /** Resolves with the dispatcher's return value once the timer fires
   *  AND the dispatcher resolves. */
  resolve: (outcome: DebouncedOutcome<R>) => void;
  /** Rejects when the dispatcher itself throws. */
  reject: (err: unknown) => void;
  /** Active timer handle. */
  timeoutHandle: unknown;
  /** Content of the pending request. */
  content: string;
  /** Memoised hash so we can compare cheaply on coalesce. */
  hash: string;
}

interface DedupCacheEntry<R> {
  hash: string;
  value: R;
  /** Monotonic timestamp of the cache write. */
  recordedAt: number;
}

export interface DebouncedRequest<R> {
  /** Promise resolving to the outcome — `fresh`, `cached`, or
   *  `coalesced`. Never rejects on coalesce; only on dispatcher
   *  failure. */
  promise: Promise<DebouncedOutcome<R>>;
}

/**
 * Per-key debouncer + content-hash dedup cache.
 *
 * The "key" is opaque to the debouncer; consumers use the request's
 * uri (or whatever stable identity their surface has) so that distinct
 * documents debounce independently. Concurrent requests for the SAME
 * key coalesce; concurrent requests for DIFFERENT keys run in parallel.
 *
 * Thread-safety: this class is JavaScript-single-threaded; concurrent
 * calls within the same event-loop turn are well-defined because
 * dispatcher invocation is awaited. There is no cross-thread
 * synchronisation requirement.
 */
export class MidEditDebouncer<R> {
  private readonly defaultDebounceMs: number;
  private readonly dedupWindowMs: number;
  private readonly scheduler: DebouncerScheduler;
  private readonly pending = new Map<string, PendingDispatch<R>>();
  private readonly dedupCache = new Map<string, DedupCacheEntry<R>>();

  public constructor(options: DebouncerOptions = {}) {
    const debounceMs = options.debounceMs ?? DEFAULT_DEBOUNCE_MS;
    if (debounceMs < 0) {
      throw new RangeError('debounceMs must be non-negative');
    }
    const dedupWindowMs = options.dedupWindowMs ?? DEFAULT_DEDUP_WINDOW_MS;
    if (dedupWindowMs < 0) {
      throw new RangeError('dedupWindowMs must be non-negative');
    }
    this.defaultDebounceMs = debounceMs;
    this.dedupWindowMs = dedupWindowMs;
    this.scheduler = options.scheduler ?? DEFAULT_SCHEDULER;
  }

  /**
   * Submit a request through the debouncer.
   *
   * Behaviour:
   *   1. If the dedup cache has a fresh entry (within
   *      `dedupWindowMs`) whose hash matches `content`, resolve
   *      immediately with `{ kind: 'cached', value }` — no dispatch.
   *   2. Otherwise, if there is a pending dispatch for `key`, cancel
   *      it (resolving its promise with `{ kind: 'coalesced' }`) and
   *      install this request as the new pending one.
   *   3. Set a timer for `debounceMs` (per-call wins over default).
   *      When the timer fires, invoke `dispatch(content)`. The
   *      dispatcher's resolved value populates the dedup cache and
   *      the returned promise resolves with
   *      `{ kind: 'fresh', value }`.
   *
   * The dispatcher MAY return either a successful response value (R)
   * or a structured error type the consumer encodes itself; the
   * debouncer does not interpret the value. If the dispatcher throws,
   * the returned promise rejects — but ONLY for the call whose
   * dispatch actually fired. Coalesced earlier calls always resolve
   * with `{ kind: 'coalesced' }`.
   */
  public submit(
    key: string,
    content: string,
    dispatch: (content: string) => Promise<R>,
    options: { debounceMs?: number } = {}
  ): DebouncedRequest<R> {
    const debounceMs = options.debounceMs ?? this.defaultDebounceMs;
    if (debounceMs < 0) {
      throw new RangeError('debounceMs must be non-negative');
    }

    const hash = contentHashSha256(content);

    // Step 1: dedup-cache lookup.
    const cached = this.dedupCache.get(key);
    const nowMs = this.scheduler.now();
    if (
      cached !== undefined &&
      cached.hash === hash &&
      nowMs - cached.recordedAt <= this.dedupWindowMs
    ) {
      // Identical content within the window — short-circuit AND coalesce
      // any in-flight dispatch for this key. If a pending request exists
      // for newer-but-superseded content, leaving it pending would let
      // a stale daemon call fire later (extra round-trip + a promise
      // resolving with diagnostics for content that is no longer
      // current). Cancel the pending dispatch so only the cached
      // outcome is observable for this `submit` epoch.
      const stalePending = this.pending.get(key);
      if (stalePending !== undefined) {
        this.scheduler.clearTimeout(stalePending.timeoutHandle);
        this.pending.delete(key);
        stalePending.resolve({ kind: 'coalesced' });
      }
      return { promise: Promise.resolve({ kind: 'cached', value: cached.value }) };
    }
    if (cached !== undefined && nowMs - cached.recordedAt > this.dedupWindowMs) {
      // Cache entry has aged out — drop it eagerly so the map does
      // not retain stale entries indefinitely.
      this.dedupCache.delete(key);
    }

    // Step 2: coalesce any in-flight pending dispatch.
    const previous = this.pending.get(key);
    if (previous !== undefined) {
      this.scheduler.clearTimeout(previous.timeoutHandle);
      this.pending.delete(key);
      previous.resolve({ kind: 'coalesced' });
    }

    // Step 3: schedule the new dispatch.
    let resolveOuter: (outcome: DebouncedOutcome<R>) => void;
    let rejectOuter: (err: unknown) => void;
    const promise = new Promise<DebouncedOutcome<R>>((resolve, reject) => {
      resolveOuter = resolve;
      rejectOuter = reject;
    });

    const timeoutHandle = this.scheduler.setTimeout(() => {
      // Fire the dispatch. Re-read the pending entry — a coalesce
      // could only have replaced it (in which case our handle would
      // have been cleared and this callback would not fire). Defensive
      // check anyway so a stale timer never invokes dispatch.
      const entry = this.pending.get(key);
      if (entry === undefined || entry.hash !== hash) {
        // We were replaced; the replacement's outer promise is now
        // owned by a later submit. This branch should be unreachable
        // because clearTimeout cancels the callback — but if a host
        // scheduler fires after clear(), we defensively no-op.
        return;
      }
      this.pending.delete(key);
      // Invoke the dispatcher inside an async IIFE so a synchronous
      // throw is captured into the local `try/catch` before any
      // microtask sees it. Without the IIFE, `Promise.reject(err)`
      // would propagate as an unhandled rejection in the same tick
      // it is constructed (Node detects rejections at construction
      // time, not at handler-attachment time).
      void (async () => {
        try {
          const value = await dispatch(content);
          // Populate the dedup cache. `recordedAt` uses our
          // scheduler clock so injected fake clocks remain monotonic
          // with the window check.
          this.dedupCache.set(key, {
            hash,
            value,
            recordedAt: this.scheduler.now(),
          });
          resolveOuter({ kind: 'fresh', value });
        } catch (err) {
          rejectOuter(err);
        }
      })();
    }, debounceMs);

    this.pending.set(key, {
      resolve: (outcome) => resolveOuter(outcome),
      reject: (err) => rejectOuter(err),
      timeoutHandle,
      content,
      hash,
    });

    return { promise };
  }

  /**
   * Cancel any pending dispatch for `key`. The pending promise (if
   * any) resolves with `{ kind: 'coalesced' }` because cancellation
   * is semantically the same as superseded-by-later. Used internally
   * by `validateMidEdit` on transport-drop teardown so callers see
   * structured outcomes rather than hangs.
   */
  public cancel(key: string): void {
    const entry = this.pending.get(key);
    if (entry === undefined) {
      return;
    }
    this.pending.delete(key);
    this.scheduler.clearTimeout(entry.timeoutHandle);
    entry.resolve({ kind: 'coalesced' });
  }

  /** Cancel every pending dispatch. */
  public cancelAll(): void {
    if (this.pending.size === 0) {
      return;
    }
    const drained = [...this.pending.entries()];
    this.pending.clear();
    for (const [, entry] of drained) {
      this.scheduler.clearTimeout(entry.timeoutHandle);
      entry.resolve({ kind: 'coalesced' });
    }
  }

  /** Drop the dedup cache entirely. Test/teardown helper. */
  public clearDedupCache(): void {
    this.dedupCache.clear();
  }

  /** Snapshot of the pending count, for tests / telemetry. */
  public get pendingCount(): number {
    return this.pending.size;
  }
}
