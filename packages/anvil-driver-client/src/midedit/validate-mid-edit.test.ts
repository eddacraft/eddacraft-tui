/**
 * `validateMidEdit` unit tests — RTAI-004.
 *
 * Coverage matrix (per the brief):
 *   - Debounce coalesces a typing burst into one round-trip.
 *   - Identical content within window short-circuits client-side
 *     (cached, no round-trip).
 *   - Transport drop cancels in-flight cleanly with structured
 *     `anvil-daemon-transport-drop` (retriable: true).
 *   - Daemon error surfaces structured (not as a thrown exception).
 *   - Per-call debounce override works (`debounceMs: 0`).
 *   - Truncated response surfaces the `truncated` flag.
 *   - Each error code from `crates/anvil-intercept/tests/midedit_contract.rs`
 *     round-trips correctly: -32602 (over-cap, malformed),
 *     -32001/-32603 (transport timeout), -32000 (busy invariant).
 *   - `validateMidEdit` reuses `DriverClient.request` (no parallel
 *     transport path).
 *
 * Most tests use the real Node timer (`debounceMs: 0` fires on the
 * next tick, well within the 15s vitest default). The
 * debounce-coalescing test uses an injected manual scheduler so it
 * can drive synthetic time without sleeping.
 */

import { describe, expect, it } from 'vitest';

import { makeFakeTransportFactory } from '../__fixtures__/fake-transport.js';
import { DriverClient } from '../client/driver-client.js';
import type { Diagnostic } from '../diagnostics/types.js';
import type { DebouncerScheduler } from './debouncer.js';
import { SCAN_BUFFER_METHOD, SCAN_BUFFER_MODE_MID_EDIT } from './validate-mid-edit.js';

interface ManualScheduler extends DebouncerScheduler {
  setTimeout: (cb: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
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

function makeDiagnostic(overrides: Partial<Diagnostic> = {}): Diagnostic {
  return {
    schema_version: 'anvil.diagnostic.v1',
    id: '01HXAMPLE',
    severity: 'warning',
    summary: 'sample',
    location: { file: 'src/x.ts', line: 1 },
    category: 'secret',
    source: { rule_id: 'sample-rule', source_module: 'anvil-checks::sample' },
    mode: 'mid-edit',
    ...overrides,
  };
}

describe('validateMidEdit — wire shape', () => {
  it('builds a scan_buffer request with mode=midEdit', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string; method: string; params: Record<string, unknown> };
          expect(env.method).toBe(SCAN_BUFFER_METHOD);
          expect(env.params.mode).toBe(SCAN_BUFFER_MODE_MID_EDIT);
          expect(env.params.path).toBe('src/foo.ts');
          expect(env.params.text).toBe('const x = 1;\n');
          expect(env.params.version).toBe(7);
          push({
            jsonrpc: '2.0',
            id: env.id,
            result: { version: 7, diagnostics: [], truncated: false },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    const result = await client.validateMidEdit({
      uri: 'src/foo.ts',
      content: 'const x = 1;\n',
      workspaceRoot: '/tmp/ws',
      version: 7,
    });
    expect(result).toEqual({
      kind: 'diagnostics',
      version: 7,
      diagnostics: [],
      truncated: false,
      fromCache: false,
    });
    await client.close();
  });

  it('reuses DriverClient.request (single transport path, only one frame on the wire)', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            result: { version: 1, diagnostics: [], truncated: false },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    const sent = tf.lastInstance!.outboundJson() as Array<{ method: string }>;
    expect(sent).toHaveLength(1);
    expect(sent[0]?.method).toBe(SCAN_BUFFER_METHOD);
    await client.close();
  });

  it('surfaces truncated=true so the caller can warn the user', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            result: {
              version: 1,
              diagnostics: [makeDiagnostic()],
              truncated: true,
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    const result = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(result.kind).toBe('diagnostics');
    if (result.kind === 'diagnostics') {
      expect(result.truncated).toBe(true);
      expect(result.diagnostics).toHaveLength(1);
    }
    await client.close();
  });
});

describe('validateMidEdit — debounce + dedup', () => {
  it('coalesces a 5-event typing burst into ONE round-trip', async () => {
    const sched = manualScheduler();
    let dispatchCount = 0;
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          dispatchCount += 1;
          const env = line as { id: string; params: { version: number } };
          push({
            jsonrpc: '2.0',
            id: env.id,
            result: {
              version: env.params.version,
              diagnostics: [],
              truncated: false,
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { scheduler: sched, debounceMs: 80 },
    });
    await client.connect();

    const promises = [];
    for (let i = 0; i < 5; i += 1) {
      const p = client.validateMidEdit({
        uri: 'a.ts',
        content: `c${i}`,
        workspaceRoot: '/x',
        version: i + 1,
      });
      promises.push(p);
      await sched.advance(20);
    }
    expect(dispatchCount).toBe(0);
    await sched.advance(80);
    const outcomes = await Promise.all(promises);

    expect(dispatchCount).toBe(1);
    expect(outcomes.slice(0, 4).every((o) => o.kind === 'coalesced')).toBe(true);
    expect(outcomes[4]?.kind).toBe('diagnostics');
    await client.close();
  });

  it('identical content within window short-circuits (cached, no round-trip)', async () => {
    let dispatchCount = 0;
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          dispatchCount += 1;
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            result: {
              version: 1,
              diagnostics: [makeDiagnostic({ id: 'first' })],
              truncated: false,
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0, dedupWindowMs: 1_000 },
    });
    await client.connect();

    const first = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'hello',
      workspaceRoot: '/x',
    });
    expect(first.kind).toBe('diagnostics');
    expect(dispatchCount).toBe(1);

    // Within window — same content — must short-circuit.
    const second = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'hello',
      workspaceRoot: '/x',
    });
    expect(second.kind).toBe('cached');
    expect(dispatchCount).toBe(1);
    if (second.kind === 'cached') {
      expect(second.diagnostics).toHaveLength(1);
      expect(second.fromCache).toBe(true);
    }
    await client.close();
  });

  it('per-call debounceMs override works (debounceMs: 0 fires immediately)', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            result: { version: 1, diagnostics: [], truncated: false },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { scheduler: sched, debounceMs: 80 },
    });
    await client.connect();

    // The default would wait 80ms; the per-call override fires now.
    const p = client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
      debounceMs: 0,
    });
    await sched.advance(0);
    const outcome = await p;
    expect(outcome.kind).toBe('diagnostics');
    await client.close();
  });
});

describe('validateMidEdit — transport drop / cancellation', () => {
  it('returns structured anvil-daemon-transport-drop on transport drop', async () => {
    // Use a manual scheduler for the debouncer so we can fire the
    // dispatch deterministically before dropping the transport.
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([
      {
        // No respond — daemon hangs. We drop the transport after
        // the request hits the wire.
      },
      // Slot for the post-drop reconnect attempt; fail the connect
      // so reconnect does not race with the assertion.
      { connectError: new Error('refused') },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { scheduler: sched, debounceMs: 0 },
      reconnect: { initialMs: 1, jitter: 0, maxAttempts: 0 },
    });
    await client.connect();

    const pending = client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    // Fire the debouncer timer and let the dispatch reach the wire.
    await sched.advance(0);
    // One extra microtask drain so the async dispatcher's `await
    // client.request(...)` reaches `transport.send`.
    await new Promise((r) => setImmediate(r));
    expect(tf.lastInstance!.outbound.length).toBe(1);
    tf.lastInstance!.dropFromPeer('peer');
    const result = await pending;

    expect(result.kind).toBe('error');
    if (result.kind === 'error') {
      expect(result.error.error).toBe('anvil-daemon-transport-drop');
      expect(result.error.retriable).toBe(true);
    }
    await client.close();
  });

  it('cancels in-flight pending requests cleanly when the client is closed', async () => {
    const sched = manualScheduler();
    const tf = makeFakeTransportFactory([{}]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { scheduler: sched, debounceMs: 80 },
    });
    await client.connect();

    // Submit a request that would fire after 80ms; close the client
    // before the timer fires.
    const pending = client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    await client.close();
    // Advance time past the debounce; the cancelled pending must
    // resolve cleanly (no hang) — the debouncer's timer was cleared
    // by close-time teardown semantics, but even if it weren't, the
    // dispatcher would catch `anvil-driver-closed` and surface it.
    await sched.advance(200);
    // The promise must settle — this assertion fails if the dispatch
    // hangs.
    const result = await pending;
    // Either coalesced (debouncer cancelled) or error (post-close
    // dispatch). Both are valid no-hang outcomes. The "no hang" is
    // the contract.
    expect(['coalesced', 'error']).toContain(result.kind);
  });
});

describe('validateMidEdit — errors-as-first-class (RTAI-008 contract)', () => {
  it('surfaces -32602 (Invalid params, over-cap) as structured error, not throw', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            error: {
              code: -32_602,
              message: 'Invalid params',
              data: { reason: 'content exceeds 1 MiB cap' },
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    const result = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(result.kind).toBe('error');
    if (result.kind === 'error') {
      expect(result.error.error).toBe('anvil-daemon-error');
      // -32602 is Invalid params — not retriable.
      expect(result.error.retriable).toBe(false);
      expect(result.error.message).toBe('Invalid params');
      const data = result.error.data as { code: number; daemon_data?: unknown } | undefined;
      expect(data?.code).toBe(-32_602);
    }
    await client.close();
  });

  it('surfaces -32602 (malformed request) as structured error, not throw', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            error: {
              code: -32_602,
              message: 'Invalid params',
              data: { reason: 'missing required field: text' },
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    const result = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(result.kind).toBe('error');
    if (result.kind === 'error') {
      const data = result.error.data as { code: number };
      expect(data.code).toBe(-32_602);
      expect(result.error.retriable).toBe(false);
    }
    await client.close();
  });

  it('surfaces -32001 (transport timeout / scan timed out) as retriable error', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            error: {
              code: -32_001,
              message: 'Scan timed out',
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    const result = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(result.kind).toBe('error');
    if (result.kind === 'error') {
      const data = result.error.data as { code: number };
      expect(data.code).toBe(-32_001);
      // -32001 is in the daemon's retriable mapping.
      expect(result.error.retriable).toBe(true);
    }
    await client.close();
  });

  it('surfaces -32603 (internal error / service unavailable) as not-retriable', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            error: {
              code: -32_603,
              message: 'Internal error',
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    const result = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(result.kind).toBe('error');
    if (result.kind === 'error') {
      const data = result.error.data as { code: number };
      expect(data.code).toBe(-32_603);
      expect(result.error.retriable).toBe(false);
    }
    await client.close();
  });

  it('surfaces -32000 (server busy) as retriable error', async () => {
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          push({
            jsonrpc: '2.0',
            id: env.id,
            error: {
              code: -32_000,
              message: 'Server busy',
            },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0 },
    });
    await client.connect();
    const result = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(result.kind).toBe('error');
    if (result.kind === 'error') {
      const data = result.error.data as { code: number };
      expect(data.code).toBe(-32_000);
      // -32000 (busy) is in the daemon's retriable mapping.
      expect(result.error.retriable).toBe(true);
    }
    await client.close();
  });

  it('does not poison the dedup cache on a daemon error (next identical-content call retries)', async () => {
    let attempt = 0;
    const tf = makeFakeTransportFactory([
      {
        respond(line, push) {
          const env = line as { id: string };
          attempt += 1;
          if (attempt === 1) {
            push({
              jsonrpc: '2.0',
              id: env.id,
              error: { code: -32_000, message: 'Server busy' },
            });
            return;
          }
          push({
            jsonrpc: '2.0',
            id: env.id,
            result: { version: 1, diagnostics: [], truncated: false },
          });
        },
      },
    ]);
    const client = new DriverClient({
      transportFactory: tf.factory,
      midEdit: { debounceMs: 0, dedupWindowMs: 1_000 },
    });
    await client.connect();

    const first = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(first.kind).toBe('error');

    // Same content. Errors must NOT seed the dedup cache; the second
    // call must hit the wire again so a transient -32000 / -32001 can
    // succeed on retry.
    const second = await client.validateMidEdit({
      uri: 'a.ts',
      content: 'A',
      workspaceRoot: '/x',
    });
    expect(second.kind).toBe('diagnostics');
    expect(attempt).toBe(2);
    await client.close();
  });
});
