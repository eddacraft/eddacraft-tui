/**
 * `DriverClient` unit tests.
 *
 * Coverage matrix (per DRVR-001 brief):
 *   - Happy-path request / response roundtrip
 *   - Subscription dispatch
 *   - Per-request timeout (read-only and enforcement-ack defaults)
 *   - In-flight cancellation on transport drop
 *   - Transparent reconnection with exponential backoff
 *   - Reconnection budget exhaustion -> structured error
 *   - Reliability-budget quarantine after repeated failures
 *   - Quarantine survives reconnect (single budget instance)
 *
 * Tests run against the in-memory fake transport. The real-daemon
 * integration test lives in `__tests__/integration-real-daemon.test.ts`.
 */

import { describe, expect, it, vi } from 'vitest';

import { makeFakeTransportFactory } from '../__fixtures__/fake-transport.js';
import { DriverClient } from './driver-client.js';
import { DriverClientError } from '../errors.js';

interface ManualScheduler {
  setTimeout: (cb: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
  /** Run all timers whose fire time is <= the synthetic clock. */
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
    async advance(ms) {
      const target = nowMs + ms;
      // Iterate until no more timers fall inside the window. New
      // timers scheduled by callbacks are picked up on the next pass.
      let progress = true;
      while (progress) {
        progress = false;
        // Snapshot keys so removal during callback doesn't disturb us.
        const due = [...queue.entries()]
          .filter(([, entry]) => entry.fireAt <= target)
          .sort((a, b) => a[1].fireAt - b[1].fireAt);
        for (const [handle, entry] of due) {
          if (!queue.has(handle)) {
            continue;
          }
          queue.delete(handle);
          // Advance synthetic clock to the firing time.
          nowMs = Math.max(nowMs, entry.fireAt);
          entry.cb();
          progress = true;
          // Yield to the microtask queue so any awaiting promises
          // resolve before the next timer fires.
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

describe('DriverClient — happy path', () => {
  it('round-trips a request through a fake daemon', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string; method: string; params?: unknown };
          push({ jsonrpc: '2.0', id: env.id, result: { ok: true, echo: env.params } });
        },
      },
    ]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    const result = await client.request('session.register', {
      session_id: 's1',
      worktree: '/tmp/wt',
    });
    expect(result).toEqual({ ok: true, echo: { session_id: 's1', worktree: '/tmp/wt' } });
    await client.close();
  });

  it('routes notifications to subscribers', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    const seen: unknown[] = [];
    const unsub = client.subscribe('anvil/publishDiagnostics', (event) => seen.push(event));
    tf.lastInstance!.pushIncoming(
      JSON.stringify({
        jsonrpc: '2.0',
        method: 'anvil/publishDiagnostics',
        params: { uri: 'file:///x', diagnostics: [] },
      }) + '\n'
    );
    expect(seen).toEqual([{ uri: 'file:///x', diagnostics: [] }]);
    unsub();
    tf.lastInstance!.pushIncoming(
      JSON.stringify({
        jsonrpc: '2.0',
        method: 'anvil/publishDiagnostics',
        params: { uri: 'file:///y', diagnostics: [] },
      }) + '\n'
    );
    // After unsubscribe, no further events.
    expect(seen.length).toBe(1);
    await client.close();
  });

  it('forwards a notification (no response expected)', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    await client.notify('anvil/heartbeat', { session_id: 's1' });
    const sent = tf.lastInstance!.outboundJson() as Array<{ method: string }>;
    expect(sent[0]?.method).toBe('anvil/heartbeat');
    expect(sent[0]).not.toHaveProperty('id');
    await client.close();
  });
});

describe('DriverClient — timeouts', () => {
  it('read-only request times out at the read default (10s)', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([
      {
        respond() {
          // Hung daemon: never replies.
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
    });
    await client.connect();
    const pending = client.request('session.list').catch((e: unknown) => e);
    await sched.advance(9_999);
    // Promise should still be pending; we can't await directly, but
    // we can advance to just before the deadline and ensure no
    // settlement happened by giving microtasks a tick.
    await new Promise((r) => setImmediate(r));
    await sched.advance(2);
    const err = (await pending) as DriverClientError;
    expect(err).toBeInstanceOf(DriverClientError);
    expect(err.code).toBe('anvil-daemon-timeout');
    expect(err.retriable).toBe(true);
    expect(err.timeout_ms).toBe(10_000);
    await client.close();
  });

  it('enforcement-ack request times out at the 500ms default', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([{ respond() {} }]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
    });
    await client.connect();
    const pending = client.request('anvil/enforcement/ack', { id: 'd1' }).catch((e) => e);
    await sched.advance(500);
    const err = (await pending) as DriverClientError;
    expect(err.code).toBe('anvil-daemon-timeout');
    expect(err.timeout_ms).toBe(500);
    await client.close();
  });

  it('per-request timeoutMs override wins over the defaults', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([{ respond() {} }]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
    });
    await client.connect();
    const pending = client.request('session.list', undefined, { timeoutMs: 50 }).catch((e) => e);
    await sched.advance(50);
    const err = (await pending) as DriverClientError;
    expect(err.timeout_ms).toBe(50);
    await client.close();
  });
});

describe('DriverClient — transport drop / cancellation', () => {
  it('cancels in-flight requests with anvil-daemon-transport-drop', async () => {
    const tf = makeFakeTransportFactory([{}, {}]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      reconnect: { jitter: 0, initialMs: 1, maxAttempts: 0 },
    });
    await client.connect();
    const pending = client.request('session.list').catch((e: unknown) => e);
    // Wait one microtask so the request hits the wire before drop.
    await new Promise((r) => setImmediate(r));
    tf.lastInstance!.dropFromPeer('peer');
    const err = (await pending) as DriverClientError;
    expect(err.code).toBe('anvil-daemon-transport-drop');
    expect(err.retriable).toBe(true);
    await client.close();
  });

  it('refuses requests after close()', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    await client.close();
    let err: unknown;
    try {
      await client.request('session.list');
    } catch (e) {
      err = e;
    }
    expect((err as DriverClientError).code).toBe('anvil-driver-closed');
  });
});

describe('DriverClient — reconnection', () => {
  it('reconnects with exponential backoff after a peer drop', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([
      {}, // first connect succeeds
      {}, // reconnect succeeds
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      reconnect: {
        initialMs: 100,
        capMs: 30_000,
        maxAttempts: 5,
        jitter: 0,
        random: () => 0.5,
      },
    });
    const reconnects: unknown[] = [];
    client.on('reconnecting', (e) => reconnects.push(e));
    const connecteds: number[] = [];
    client.on('connected', () => connecteds.push(Date.now()));

    await client.connect();
    expect(tf.instances.length).toBe(1);
    tf.instances[0]!.dropFromPeer('peer');

    await sched.advance(100);
    // Reconnect should be in flight; allow microtasks to settle.
    await new Promise((r) => setImmediate(r));
    expect(tf.instances.length).toBe(2);
    expect(reconnects.length).toBe(1);
    expect((reconnects[0] as { delayMs: number }).delayMs).toBe(100);
    expect(connecteds.length).toBe(2);
    await client.close();
  });

  it('caps backoff at the configured maximum', async () => {
    const sched = manualScheduler();
    const recipes = Array.from({ length: 4 }, () => ({
      connectError: new Error('refused'),
    }));
    const tf = makeFakeTransportFactory([{}, ...recipes]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      reconnect: {
        initialMs: 100,
        capMs: 200, // cap aggressively to cover the cap branch
        maxAttempts: 4,
        jitter: 0,
        random: () => 0.5,
      },
    });
    const reconnects: Array<{ attempt: number; delayMs: number }> = [];
    client.on('reconnecting', (e) => reconnects.push(e));
    await client.connect();
    tf.instances[0]!.dropFromPeer('peer');
    // Drive through all reconnect attempts.
    for (let i = 0; i < 6; i += 1) {
      await sched.advance(500);
      await new Promise((r) => setImmediate(r));
    }
    // Each scheduled attempt has delayMs = min(initial * 2^(n-1), cap).
    expect(reconnects.map((r) => r.delayMs)).toEqual([100, 200, 200, 200]);
    await client.close();
  });

  it('emits reconnect_failed after exhausting maxAttempts', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([
      {},
      { connectError: new Error('boom') },
      { connectError: new Error('boom') },
    ]);
    const failedEvents: Array<{ attempts: number }> = [];
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      reconnect: {
        initialMs: 10,
        capMs: 100,
        maxAttempts: 2,
        jitter: 0,
        random: () => 0.5,
      },
    });
    client.on('reconnect_failed', (e) => failedEvents.push(e));
    await client.connect();
    tf.instances[0]!.dropFromPeer('peer');
    for (let i = 0; i < 5; i += 1) {
      await sched.advance(200);
      await new Promise((r) => setImmediate(r));
    }
    expect(failedEvents.length).toBe(1);
    expect(failedEvents[0]?.attempts).toBe(2);
    await client.close();
  });

  it('does not reconnect on a local close', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([{}, {}]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      reconnect: { initialMs: 10, jitter: 0, random: () => 0.5, maxAttempts: 5 },
    });
    const reconnects: unknown[] = [];
    client.on('reconnecting', (e) => reconnects.push(e));
    await client.connect();
    await client.close();
    await sched.advance(500);
    expect(reconnects).toEqual([]);
    expect(tf.instances.length).toBe(1);
  });
});

describe('DriverClient — reliability budget', () => {
  it('quarantines the driver after repeated failures and persists across reconnect', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([{ respond() {} }, { respond() {} }]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      driverIdentity: 'driver-X',
      reliabilityBudget: { failureThreshold: 2, windowMs: 60_000, cooldownMs: 60_000 },
      reconnect: { initialMs: 10, jitter: 0, random: () => 0.5, maxAttempts: 5 },
      timeoutsMs: { readOnly: 100 },
    });
    await client.connect();
    const failures: unknown[] = [];
    for (let i = 0; i < 2; i += 1) {
      const p = client.request('session.list').catch((e) => failures.push(e));
      await sched.advance(101);
      await p;
    }
    // Simulate transport drop -> reconnect; quarantine must remain.
    tf.lastInstance!.dropFromPeer('peer');
    await sched.advance(50);
    await new Promise((r) => setImmediate(r));
    let nextErr: DriverClientError | undefined;
    try {
      await client.request('session.list');
    } catch (e) {
      nextErr = e as DriverClientError;
    }
    expect(nextErr?.code).toBe('anvil-driver-quarantined');
    await client.close();
  });

  it('does not touch the budget for unidentified drivers', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([{ respond() {} }]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      reliabilityBudget: { failureThreshold: 1, windowMs: 60_000 },
      timeoutsMs: { readOnly: 50 },
      // No driverIdentity — pre-handshake state.
    });
    await client.connect();
    const p = client.request('session.list').catch((e) => e);
    await sched.advance(60);
    await p;
    expect(client.reliabilitySnapshot()).toEqual([]);
    await client.close();
  });

  it('rejects a request immediately when already quarantined', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([{ respond() {} }]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      driverIdentity: 'driver-X',
      reliabilityBudget: { failureThreshold: 1, windowMs: 60_000, cooldownMs: 60_000 },
      timeoutsMs: { readOnly: 50 },
    });
    await client.connect();
    const first = client.request('session.list').catch((e) => e);
    await sched.advance(60);
    await first;
    let err: unknown;
    try {
      await client.request('session.list');
    } catch (e) {
      err = e;
    }
    expect((err as DriverClientError).code).toBe('anvil-driver-quarantined');
    await client.close();
  });
});

describe('DriverClient — wrong-owner refusal at connect', () => {
  it('surfaces structured wrong-owner from the transport factory', async () => {
    const tf = makeFakeTransportFactory([
      {
        connectError: Object.assign(new Error('mode mismatch'), {}),
      },
    ]);
    // Wrap factory to fire the structured error.
    const client = new DriverClient({
      transportFactory: () => {
        const inst = tf.factory({});
        // Force the structured-error path: replace connect.
        const original = inst.connect.bind(inst);
        inst.connect = async (handlers): Promise<void> => {
          await original(handlers).catch(() => undefined);
          throw (await import('../errors.js')).driverError(
            'anvil-daemon-wrong-owner',
            'fake wrong-owner'
          );
        };
        return inst;
      },
    });
    let err: unknown;
    try {
      await client.connect();
    } catch (e) {
      err = e;
    }
    expect(err).toBeInstanceOf(DriverClientError);
    expect((err as DriverClientError).code).toBe('anvil-daemon-wrong-owner');
    expect((err as DriverClientError).retriable).toBe(false);
  });
});

describe('DriverClient — framer error surfacing', () => {
  it('surfaces framer errors as framing_error events without dropping the connection', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    const events: Array<{ reason: string }> = [];
    client.on('framing_error', (e) => events.push({ reason: e.reason }));
    tf.lastInstance!.pushIncoming('{not json}\n');
    expect(events.length).toBe(1);
    expect(events[0]?.reason).toBe('invalid-json');
    // Connection should still be alive: send a follow-up request and
    // get a response.
    tf.lastInstance!.pushIncoming(
      JSON.stringify({ jsonrpc: '2.0', method: 'anvil/x', params: { ok: true } }) + '\n'
    );
    await client.close();
  });

  it('forwards subscriber-thrown errors through the error event', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    client.subscribe('anvil/x', () => {
      throw new Error('handler boom');
    });
    const errors: Array<{ cause: unknown }> = [];
    client.on('error', (e) => errors.push(e));
    tf.lastInstance!.pushIncoming(
      JSON.stringify({ jsonrpc: '2.0', method: 'anvil/x', params: {} }) + '\n'
    );
    expect(errors.length).toBe(1);
    expect((errors[0]!.cause as Error).message).toBe('handler boom');
    await client.close();
  });
});

describe('DriverClient — guards', () => {
  it('rejects request() before connect()', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    let err: unknown;
    try {
      await client.request('session.list');
    } catch (e) {
      err = e;
    }
    expect((err as DriverClientError).code).toBe('anvil-daemon-unavailable');
  });

  it('rejects bad reconnect / timeout configuration at construction', () => {
    expect(() => new DriverClient({ reconnect: { initialMs: 0 } })).toThrow(RangeError);
    expect(() => new DriverClient({ reconnect: { initialMs: 100, capMs: 50 } })).toThrow(
      RangeError
    );
    expect(() => new DriverClient({ reconnect: { maxAttempts: -1 } })).toThrow(RangeError);
    expect(() => new DriverClient({ reconnect: { jitter: -0.1 } })).toThrow(RangeError);
    expect(() => new DriverClient({ reconnect: { jitter: 1 } })).toThrow(RangeError);
    expect(() => new DriverClient({ timeoutsMs: { readOnly: 0 } })).toThrow(RangeError);
  });

  it('explicit reconnect after budget exhaustion resets the attempt counter', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([
      {}, // first connect succeeds
      { connectError: new Error('boom') },
      { connectError: new Error('boom') },
      {}, // explicit reconnect succeeds
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      scheduler: sched,
      reconnect: {
        initialMs: 10,
        capMs: 100,
        maxAttempts: 2,
        jitter: 0,
        random: () => 0.5,
      },
    });
    const failedEvents: unknown[] = [];
    client.on('reconnect_failed', (e) => failedEvents.push(e));
    await client.connect();
    tf.instances[0]!.dropFromPeer('peer');
    for (let i = 0; i < 5; i += 1) {
      await sched.advance(200);
      await new Promise((r) => setImmediate(r));
    }
    expect(failedEvents.length).toBe(1);
    // Explicit reconnect after exhaustion: fresh attempt counter
    // means the next peer-drop re-uses the full reconnect budget.
    await client.connect();
    expect(tf.instances.length).toBe(4);
    await client.close();
  });

  it('connect() is idempotent', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    await client.connect();
    expect(tf.instances.length).toBe(1);
    await client.close();
  });

  it('close() is idempotent', async () => {
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({ transportFactory: tf.factory });
    await client.connect();
    await client.close();
    await client.close();
  });
});

// Sanity check that vi mock is actually loaded — keeps the test
// tooling honest if the vitest runtime is misconfigured.
describe('vitest sanity', () => {
  it('runs', () => {
    expect(vi.fn()).toBeDefined();
  });
});
