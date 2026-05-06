/**
 * `MidEditDebouncer` unit tests — RTAI-004.
 *
 * Coverage:
 *   - Debounce coalesces a typing burst into one dispatch.
 *   - Identical-content within dedup window short-circuits client-side.
 *   - Cached entries expire after the dedup window.
 *   - Cancellation (`cancel`/`cancelAll`) resolves pending promises
 *     with `coalesced` (no hangs).
 *   - Per-call debounce override.
 *   - SHA-256 content hash is sound (not a weak hash).
 *   - Distinct keys debounce independently.
 *   - Dispatcher rejection propagates and does NOT populate cache.
 */

import { describe, expect, it } from 'vitest';

import {
  contentHashSha256,
  DEFAULT_DEBOUNCE_MS,
  DEFAULT_DEDUP_WINDOW_MS,
  MidEditDebouncer,
  type DebouncerScheduler,
} from './debouncer.js';

interface ManualScheduler extends DebouncerScheduler {
  /** Fast-forward the synthetic clock by `ms` and fire any timers
   *  that come due. */
  advance(ms: number): Promise<void>;
  pending: number;
}

function manualScheduler(): ManualScheduler {
  let nowMs = 0;
  let counter = 0;
  const queue = new Map<number, { fireAt: number; cb: () => void }>();
  const sched: ManualScheduler = {
    setTimeout(cb, ms) {
      counter += 1;
      const handle = counter;
      queue.set(handle, { fireAt: nowMs + ms, cb });
      return handle;
    },
    clearTimeout(handle) {
      queue.delete(handle as number);
    },
    now: () => nowMs,
    async advance(ms) {
      const target = nowMs + ms;
      let progress = true;
      while (progress) {
        progress = false;
        const due = [...queue.entries()]
          .filter(([, entry]) => entry.fireAt <= target)
          .sort((a, b) => a[1].fireAt - b[1].fireAt);
        for (const [handle, entry] of due) {
          if (!queue.has(handle)) {
            continue;
          }
          queue.delete(handle);
          nowMs = Math.max(nowMs, entry.fireAt);
          entry.cb();
          progress = true;
          await new Promise((r) => setImmediate(r));
        }
      }
      nowMs = target;
    },
    get pending() {
      return queue.size;
    },
  };
  return sched;
}

describe('contentHashSha256', () => {
  it('produces a stable 64-character hex digest for identical input', () => {
    const a = contentHashSha256('hello world');
    const b = contentHashSha256('hello world');
    expect(a).toBe(b);
    expect(a).toMatch(/^[0-9a-f]{64}$/);
  });

  it('produces distinct digests for different inputs', () => {
    const a = contentHashSha256('hello world');
    const b = contentHashSha256('hello world!');
    expect(a).not.toBe(b);
  });

  it('handles unicode content without truncation (no weak-hash risk)', () => {
    const a = contentHashSha256('alpha');
    const b = contentHashSha256('αlpha');
    expect(a).not.toBe(b);
    expect(b).toMatch(/^[0-9a-f]{64}$/);
  });
});

describe('MidEditDebouncer — defaults', () => {
  it('exposes the brief-pinned defaults (80ms typing cycle, 1s dedup)', () => {
    expect(DEFAULT_DEBOUNCE_MS).toBe(80);
    expect(DEFAULT_DEDUP_WINDOW_MS).toBe(1_000);
  });

  it('rejects negative debounceMs and dedupWindowMs', () => {
    expect(() => new MidEditDebouncer({ debounceMs: -1 })).toThrow(RangeError);
    expect(() => new MidEditDebouncer({ dedupWindowMs: -1 })).toThrow(RangeError);
  });

  it('rejects per-call debounceMs that is negative', () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({ scheduler: sched });
    expect(() => debouncer.submit('uri', 'content', async () => 'x', { debounceMs: -5 })).toThrow(
      RangeError
    );
  });
});

describe('MidEditDebouncer — debounce coalescing', () => {
  it('coalesces a 5-event typing burst into ONE dispatch', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 80,
      dedupWindowMs: 0,
      scheduler: sched,
    });

    let dispatchCount = 0;
    const dispatch = async (text: string): Promise<string> => {
      dispatchCount += 1;
      return `result-${text}`;
    };

    // Five rapid keystrokes within the debounce window. Each call
    // sees a pending entry from the previous call and replaces it.
    const promises = [];
    for (let i = 0; i < 5; i += 1) {
      const req = debouncer.submit('file:///x', `content-${i}`, dispatch);
      promises.push(req.promise);
      // Advance just under the debounce window between keystrokes
      // so the timer never fires before the next call replaces it.
      await sched.advance(20);
    }

    // After the burst the timer hasn't fired yet (we've advanced
    // 4 × 20 = 80ms, but the LAST submit re-armed the timer).
    expect(dispatchCount).toBe(0);

    // Fire the final timer.
    await sched.advance(80);
    const outcomes = await Promise.all(promises);

    // Only the LAST dispatch fires.
    expect(dispatchCount).toBe(1);

    // The first four are coalesced; the last is fresh.
    expect(outcomes.slice(0, 4).every((o) => o.kind === 'coalesced')).toBe(true);
    expect(outcomes[4]).toEqual({ kind: 'fresh', value: 'result-content-4' });
  });

  it('per-call debounceMs override wins over the default', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 80,
      dedupWindowMs: 0,
      scheduler: sched,
    });

    let dispatchCount = 0;
    const req = debouncer.submit(
      'file:///x',
      'content',
      async () => {
        dispatchCount += 1;
        return 'ok';
      },
      { debounceMs: 0 }
    );

    await sched.advance(0);
    const outcome = await req.promise;
    expect(dispatchCount).toBe(1);
    expect(outcome).toEqual({ kind: 'fresh', value: 'ok' });
  });

  it('distinct keys debounce independently (no cross-document interference)', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 80,
      dedupWindowMs: 0,
      scheduler: sched,
    });

    let dispatchCount = 0;
    const dispatch = async (text: string): Promise<string> => {
      dispatchCount += 1;
      return text;
    };

    const reqA = debouncer.submit('file:///a.ts', 'A', dispatch);
    const reqB = debouncer.submit('file:///b.ts', 'B', dispatch);

    await sched.advance(80);
    const [outA, outB] = await Promise.all([reqA.promise, reqB.promise]);

    expect(dispatchCount).toBe(2);
    expect(outA).toEqual({ kind: 'fresh', value: 'A' });
    expect(outB).toEqual({ kind: 'fresh', value: 'B' });
  });
});

describe('MidEditDebouncer — content-hash dedup', () => {
  it('identical content within window short-circuits without dispatch', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 0,
      dedupWindowMs: 1_000,
      scheduler: sched,
    });

    let dispatchCount = 0;
    const dispatch = async (text: string): Promise<string> => {
      dispatchCount += 1;
      return `result-${text}`;
    };

    // First call: fresh dispatch.
    const first = debouncer.submit('file:///x', 'hello', dispatch);
    await sched.advance(0);
    const firstOutcome = await first.promise;
    expect(firstOutcome).toEqual({ kind: 'fresh', value: 'result-hello' });
    expect(dispatchCount).toBe(1);

    // Second call: identical content, within window — cached.
    await sched.advance(500);
    const second = debouncer.submit('file:///x', 'hello', dispatch);
    const secondOutcome = await second.promise;
    expect(secondOutcome).toEqual({ kind: 'cached', value: 'result-hello' });
    expect(dispatchCount).toBe(1); // No new dispatch.
  });

  it('cache expires after the dedup window', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 0,
      dedupWindowMs: 1_000,
      scheduler: sched,
    });

    let dispatchCount = 0;
    const dispatch = async (text: string): Promise<string> => {
      dispatchCount += 1;
      return `result-${text}`;
    };

    const first = debouncer.submit('file:///x', 'hello', dispatch);
    await sched.advance(0);
    await first.promise;
    expect(dispatchCount).toBe(1);

    // Advance PAST the dedup window.
    await sched.advance(2_000);

    const second = debouncer.submit('file:///x', 'hello', dispatch);
    await sched.advance(0);
    const outcome = await second.promise;

    expect(outcome).toEqual({ kind: 'fresh', value: 'result-hello' });
    expect(dispatchCount).toBe(2);
  });

  it('different content within window dispatches normally (no false dedup)', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 0,
      dedupWindowMs: 1_000,
      scheduler: sched,
    });

    let dispatchCount = 0;
    const dispatch = async (text: string): Promise<string> => {
      dispatchCount += 1;
      return `result-${text}`;
    };

    const first = debouncer.submit('file:///x', 'hello', dispatch);
    await sched.advance(0);
    await first.promise;

    const second = debouncer.submit('file:///x', 'world', dispatch);
    await sched.advance(0);
    const secondOutcome = await second.promise;

    expect(secondOutcome).toEqual({ kind: 'fresh', value: 'result-world' });
    expect(dispatchCount).toBe(2);
  });
});

describe('MidEditDebouncer — cancellation', () => {
  it('cancel() resolves pending promise with coalesced (no hang)', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 80,
      scheduler: sched,
    });

    let dispatchCount = 0;
    const req = debouncer.submit('file:///x', 'content', async () => {
      dispatchCount += 1;
      return 'unused';
    });

    debouncer.cancel('file:///x');
    const outcome = await req.promise;

    expect(outcome).toEqual({ kind: 'coalesced' });
    expect(dispatchCount).toBe(0);
  });

  it('cancelAll() resolves every pending dispatch with coalesced', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 80,
      scheduler: sched,
    });

    const a = debouncer.submit('file:///a', 'A', async () => 'unused');
    const b = debouncer.submit('file:///b', 'B', async () => 'unused');
    const c = debouncer.submit('file:///c', 'C', async () => 'unused');

    expect(debouncer.pendingCount).toBe(3);
    debouncer.cancelAll();
    expect(debouncer.pendingCount).toBe(0);

    const outcomes = await Promise.all([a.promise, b.promise, c.promise]);
    expect(outcomes).toEqual([{ kind: 'coalesced' }, { kind: 'coalesced' }, { kind: 'coalesced' }]);
  });

  it('cancel() on an unknown key is a no-op', () => {
    const debouncer = new MidEditDebouncer<string>();
    expect(() => debouncer.cancel('file:///nonexistent')).not.toThrow();
  });
});

describe('MidEditDebouncer — dispatcher errors', () => {
  it('dispatcher rejection propagates and does NOT populate cache', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 0,
      dedupWindowMs: 1_000,
      scheduler: sched,
    });

    const failingDispatch = async (): Promise<string> => {
      throw new Error('boom');
    };

    const first = debouncer.submit('file:///x', 'hello', failingDispatch);
    // Attach the rejection handler synchronously BEFORE advancing
    // the scheduler so Node never sees a transient unhandled
    // rejection (the handler is registered before the dispatcher
    // settles).
    const firstResult = first.promise.catch((err: unknown) => err);
    await sched.advance(0);
    const firstErr = (await firstResult) as Error;
    expect(firstErr.message).toBe('boom');

    // Cache MUST be empty — a failed dispatch never seeds dedup.
    let dispatchedAgain = false;
    const next = debouncer.submit('file:///x', 'hello', async () => {
      dispatchedAgain = true;
      return 'ok';
    });
    await sched.advance(0);
    const outcome = await next.promise;

    expect(dispatchedAgain).toBe(true);
    expect(outcome).toEqual({ kind: 'fresh', value: 'ok' });
  });

  it('synchronous dispatcher throw is normalised to a rejection', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 0,
      scheduler: sched,
    });

    const throwingDispatch = (() => {
      throw new Error('sync boom');
    }) as () => Promise<string>;

    const req = debouncer.submit('file:///x', 'content', throwingDispatch);
    const settled = req.promise.catch((err: unknown) => err);
    await sched.advance(0);
    const err = (await settled) as Error;
    expect(err.message).toBe('sync boom');
  });
});

describe('MidEditDebouncer — coalesce semantics', () => {
  it('a coalesced earlier call resolves with coalesced even after the later one settles', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 80,
      scheduler: sched,
    });

    const a = debouncer.submit('file:///x', 'A', async () => 'A-result');
    // Replacement before the timer fires.
    await sched.advance(40);
    const b = debouncer.submit('file:///x', 'B', async () => 'B-result');
    await sched.advance(80);

    const [aOutcome, bOutcome] = await Promise.all([a.promise, b.promise]);
    expect(aOutcome).toEqual({ kind: 'coalesced' });
    expect(bOutcome).toEqual({ kind: 'fresh', value: 'B-result' });
  });

  it('caches the LAST successfully resolved value, not the first', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 0,
      dedupWindowMs: 1_000,
      scheduler: sched,
    });

    const a = debouncer.submit('file:///x', 'first', async () => 'first-result');
    await sched.advance(0);
    await a.promise;

    const b = debouncer.submit('file:///x', 'second', async () => 'second-result');
    await sched.advance(0);
    await b.promise;

    // Re-submit identical-to-second content; should hit second-result, not first.
    const c = debouncer.submit('file:///x', 'second', async () => 'unreachable');
    const cOutcome = await c.promise;
    expect(cOutcome).toEqual({ kind: 'cached', value: 'second-result' });
  });

  it('clearDedupCache() drops cached entries', async () => {
    const sched = manualScheduler();
    const debouncer = new MidEditDebouncer<string>({
      debounceMs: 0,
      scheduler: sched,
    });

    const a = debouncer.submit('file:///x', 'hello', async () => 'cached');
    await sched.advance(0);
    await a.promise;

    debouncer.clearDedupCache();

    let dispatched = false;
    const b = debouncer.submit('file:///x', 'hello', async () => {
      dispatched = true;
      return 'fresh';
    });
    await sched.advance(0);
    const outcome = await b.promise;
    expect(dispatched).toBe(true);
    expect(outcome).toEqual({ kind: 'fresh', value: 'fresh' });
  });
});
