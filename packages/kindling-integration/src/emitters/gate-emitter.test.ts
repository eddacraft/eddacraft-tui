/**
 * Gate Emitter Tests (KINDLING-004)
 *
 * Covers emitGateEvaluated.
 * Gate evaluations form the governance record — why things were
 * allowed or blocked.
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from '../kindling-service.js';
import { KindlingConfigSchema } from '../config.js';
import type { Observation } from '../observation-contract.js';
import type { QueryRequest, QueryResponse } from '../query-contract.js';
import { emitGateEvaluated, type GateResult } from './gate-emitter.js';

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

function makeGateResult(overrides: Partial<GateResult> = {}): GateResult {
  return {
    session_id: '550e8400-e29b-41d4-a716-446655440000',
    gate_id: 'architecture',
    inputs: {
      file_count: 5,
      changed_files: ['src/index.ts'],
    },
    outcome: 'pass',
    rules_evaluated: ['no-circular-deps', 'layer-boundaries'],
    enforcement: 'blocking',
    duration_ms: 250,
    ...overrides,
  };
}

// =============================================================================
// emitGateEvaluated
// =============================================================================

describe('emitGateEvaluated', () => {
  it('returns a valid UUID gate_eval_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const gateEvalId = emitGateEvaluated(svc, makeGateResult());
    expect(gateEvalId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
    );
  });

  it('returns a different gate_eval_id on each call', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const id1 = emitGateEvaluated(svc, makeGateResult());
    const id2 = emitGateEvaluated(svc, makeGateResult());
    expect(id1).not.toBe(id2);
  });

  it('emits an observation with kind gate_evaluated', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('gate_evaluated');
  });

  it('emits gate_eval_id matching the returned value', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const gateEvalId = emitGateEvaluated(svc, makeGateResult());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.gate_eval_id).toBe(gateEvalId);
  });

  it('emits the provided session_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ session_id: '550e8400-e29b-41d4-a716-446655440000' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.session_id).toBe('550e8400-e29b-41d4-a716-446655440000');
  });

  it('emits the provided gate_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ gate_id: 'coverage' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.gate_id).toBe('coverage');
  });

  it('emits the optional gate_version when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ gate_version: '2.1.0' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.gate_version).toBe('2.1.0');
  });

  it('omits gate_version when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.gate_version).toBeUndefined();
  });

  it('emits the inputs with file_count', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ inputs: { file_count: 42 } }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.inputs.file_count).toBe(42);
  });

  it('emits the inputs with changed_files', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const files = ['src/a.ts', 'src/b.ts'];
    emitGateEvaluated(svc, makeGateResult({ inputs: { changed_files: files } }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.inputs.changed_files).toEqual(files);
  });

  it('emits the inputs with baseline_hash', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ inputs: { baseline_hash: 'sha256:abc123' } }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.inputs.baseline_hash).toBe('sha256:abc123');
  });

  it('emits the outcome', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ outcome: 'fail' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.outcome).toBe('fail');
  });

  it('supports all valid outcomes', async () => {
    const outcomes = ['pass', 'fail', 'error', 'skipped'] as const;
    for (const outcome of outcomes) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitGateEvaluated(svc, makeGateResult({ outcome }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
      expect(obs.outcome).toBe(outcome);
    }
  });

  it('emits rules_evaluated', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const rules = ['rule-A', 'rule-B', 'rule-C'];
    emitGateEvaluated(svc, makeGateResult({ rules_evaluated: rules }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.rules_evaluated).toEqual(rules);
  });

  it('emits rules_violated when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(
      svc,
      makeGateResult({ outcome: 'fail', rules_violated: ['no-circular-deps'] })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.rules_violated).toEqual(['no-circular-deps']);
  });

  it('omits rules_violated when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.rules_violated).toBeUndefined();
  });

  it('emits the enforcement level', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ enforcement: 'warning' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.enforcement).toBe('warning');
  });

  it('supports all valid enforcement values', async () => {
    const enforcements = ['blocking', 'warning', 'informational'] as const;
    for (const enforcement of enforcements) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitGateEvaluated(svc, makeGateResult({ enforcement }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
      expect(obs.enforcement).toBe(enforcement);
    }
  });

  it('emits duration_ms', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ duration_ms: 987 }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.duration_ms).toBe(987);
  });

  it('emits violation_count when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ violation_count: 7 }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.violation_count).toBe(7);
  });

  it('emits warning_count when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult({ warning_count: 3 }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'gate_evaluated' }>;
    expect(obs.warning_count).toBe(3);
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitGateEvaluated(svc, makeGateResult());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z$/);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitGateEvaluated(svc, makeGateResult());
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
    expect(() => emitGateEvaluated(svc, makeGateResult())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});
