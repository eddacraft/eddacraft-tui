/**
 * Session Emitter Tests (KINDLING-003)
 *
 * Covers emitSessionStart and emitSessionEnd.
 * Sessions are the spine — every other observation links to a session_id.
 *
 * Strategy: inject a recording IKindlingStore at the store boundary to capture
 * the observation the emitter writes, then assert event shape, required fields,
 * optional fields, and return value.
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from '../kindling-service.js';
import { KindlingConfigSchema } from '../config.js';
import type { Observation } from '../observation-contract.js';
import type { QueryRequest, QueryResponse } from '../query-contract.js';
import {
  emitSessionStart,
  emitSessionEnd,
  type SessionStartContext,
  type SessionEndOutcome,
} from './session-emitter.js';

// =============================================================================
// Test Helpers
// =============================================================================

const enabledConfig = KindlingConfigSchema.parse({ enabled: true });

function makeSpyStore(): { store: IKindlingStore; emits: Observation[] } {
  const emits: Observation[] = [];
  const store: IKindlingStore = {
    emit: async (o) => {
      emits.push(o);
    },
    query: async (_req: QueryRequest): Promise<QueryResponse> => {
      throw new Error('query not used in emitter tests');
    },
    close: async () => {},
  };
  return { store, emits };
}

function makeService(store: IKindlingStore): KindlingService {
  return new KindlingService(store, enabledConfig);
}

function makeSessionStartContext(
  overrides: Partial<SessionStartContext> = {}
): SessionStartContext {
  return {
    working_directory: '/home/user/project',
    anvil_version: '1.0.0',
    command: 'check',
    args: ['--watch'],
    environment: 'development',
    ...overrides,
  };
}

function makeSessionEndOutcome(overrides: Partial<SessionEndOutcome> = {}): SessionEndOutcome {
  return {
    outcome: 'success',
    exit_code: 0,
    duration_ms: 1200,
    summary: {
      gates_evaluated: 3,
      gates_passed: 3,
      gates_failed: 0,
      actions_executed: 5,
      errors_encountered: 0,
    },
    ...overrides,
  };
}

// =============================================================================
// emitSessionStart
// =============================================================================

describe('emitSessionStart', () => {
  it('returns a valid UUID session_id', async () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const sessionId = emitSessionStart(svc, makeSessionStartContext());
    expect(sessionId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
    );
  });

  it('returns a different session_id on each call', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const id1 = emitSessionStart(svc, makeSessionStartContext());
    const id2 = emitSessionStart(svc, makeSessionStartContext());
    expect(id1).not.toBe(id2);
  });

  it('emits an observation with kind session_start', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext());
    // Allow fire-and-forget to resolve
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('session_start');
  });

  it('emits session_id that matches the returned value', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const sessionId = emitSessionStart(svc, makeSessionStartContext());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].session_id).toBe(sessionId);
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z$/);
  });

  it('includes working_directory in context', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ working_directory: '/srv/app' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.context.working_directory).toBe('/srv/app');
  });

  it('includes anvil_version in context', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ anvil_version: '2.3.1' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.context.anvil_version).toBe('2.3.1');
  });

  it('includes command in context', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ command: 'watch' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.context.command).toBe('watch');
  });

  it('includes args in context', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ args: ['--json', '--ci'] }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.context.args).toEqual(['--json', '--ci']);
  });

  it('includes environment in context', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ environment: 'ci' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.context.environment).toBe('ci');
  });

  it('includes optional git_ref when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ git_ref: 'main' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.context.git_ref).toBe('main');
  });

  it('includes optional git_dirty when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ git_dirty: true }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.context.git_dirty).toBe(true);
  });

  it('includes optional plan_id when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext({ plan_id: 'plan-abc-123' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.plan_id).toBe('plan-abc-123');
  });

  it('omits plan_id when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionStart(svc, makeSessionStartContext());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_start' }>;
    expect(obs.plan_id).toBeUndefined();
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitSessionStart(svc, makeSessionStartContext());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(0);
  });

  it('swallows store errors silently (fire-and-forget)', async () => {
    const failStore: IKindlingStore = {
      emit: async () => {
        throw new Error('store exploded');
      },
      query: async (_req: QueryRequest): Promise<QueryResponse> => {
        throw new Error('not used');
      },
      close: async () => {},
    };
    const svc = makeService(failStore);
    // Should not throw
    expect(() => emitSessionStart(svc, makeSessionStartContext())).not.toThrow();
    // Allow promise to settle
    await new Promise((r) => setImmediate(r));
  });
});

// =============================================================================
// emitSessionEnd
// =============================================================================

describe('emitSessionEnd', () => {
  it('returns the same session_id that was passed in', async () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const inputId = '550e8400-e29b-41d4-a716-446655440000';
    const returned = emitSessionEnd(svc, inputId, makeSessionEndOutcome());
    expect(returned).toBe(inputId);
  });

  it('emits an observation with kind session_end', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionEnd(svc, '550e8400-e29b-41d4-a716-446655440000', makeSessionEndOutcome());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('session_end');
  });

  it('emits with the provided session_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const sid = '550e8400-e29b-41d4-a716-446655440000';
    emitSessionEnd(svc, sid, makeSessionEndOutcome());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].session_id).toBe(sid);
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionEnd(svc, '550e8400-e29b-41d4-a716-446655440000', makeSessionEndOutcome());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z$/);
  });

  it('emits the outcome value', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionEnd(
      svc,
      '550e8400-e29b-41d4-a716-446655440000',
      makeSessionEndOutcome({ outcome: 'failure' })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_end' }>;
    expect(obs.outcome).toBe('failure');
  });

  it('emits the exit_code', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionEnd(
      svc,
      '550e8400-e29b-41d4-a716-446655440000',
      makeSessionEndOutcome({ exit_code: 1 })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_end' }>;
    expect(obs.exit_code).toBe(1);
  });

  it('emits the duration_ms', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitSessionEnd(
      svc,
      '550e8400-e29b-41d4-a716-446655440000',
      makeSessionEndOutcome({ duration_ms: 9876 })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_end' }>;
    expect(obs.duration_ms).toBe(9876);
  });

  it('emits the summary counts', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const summary = {
      gates_evaluated: 10,
      gates_passed: 8,
      gates_failed: 2,
      actions_executed: 15,
      errors_encountered: 3,
    };
    emitSessionEnd(svc, '550e8400-e29b-41d4-a716-446655440000', makeSessionEndOutcome({ summary }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_end' }>;
    expect(obs.summary).toEqual(summary);
  });

  it('supports all valid outcome values', async () => {
    const outcomes = ['success', 'failure', 'partial', 'cancelled'] as const;
    for (const outcome of outcomes) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitSessionEnd(
        svc,
        '550e8400-e29b-41d4-a716-446655440000',
        makeSessionEndOutcome({ outcome })
      );
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'session_end' }>;
      expect(obs.outcome).toBe(outcome);
    }
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitSessionEnd(svc, '550e8400-e29b-41d4-a716-446655440000', makeSessionEndOutcome());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(0);
  });

  it('swallows store errors silently (fire-and-forget)', async () => {
    const failStore: IKindlingStore = {
      emit: async () => {
        throw new Error('store exploded');
      },
      query: async (_req: QueryRequest): Promise<QueryResponse> => {
        throw new Error('not used');
      },
      close: async () => {},
    };
    const svc = makeService(failStore);
    expect(() =>
      emitSessionEnd(svc, '550e8400-e29b-41d4-a716-446655440000', makeSessionEndOutcome())
    ).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});
