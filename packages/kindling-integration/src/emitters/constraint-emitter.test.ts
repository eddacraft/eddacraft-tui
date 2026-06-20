/**
 * Constraint Emitter Tests (KINDLING-007b)
 *
 * Covers emitConstraintApplied.
 * Constraint observations record why an action was prevented — policy,
 * scope, environment, or approval requirements.
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from '../kindling-service.js';
import { KindlingConfigSchema } from '../config.js';
import type { Observation } from '../observation-contract.js';
import type { QueryRequest, QueryResponse } from '../query-contract.js';
import { emitConstraintApplied, type ConstraintDetails } from './constraint-emitter.js';

// =============================================================================
// Test Helpers
// =============================================================================

const enabledConfig = KindlingConfigSchema.parse({ enabled: true });
const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const TIMESTAMP_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z$/;

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

function makeConstraintDetails(overrides: Partial<ConstraintDetails> = {}): ConstraintDetails {
  return {
    session_id: VALID_UUID,
    constraint_type: 'policy',
    prevented_action: {
      action_type: 'file_write',
      action_target: '/etc/passwd',
    },
    reason: 'write-to-system-files-denied',
    ...overrides,
  };
}

// =============================================================================
// emitConstraintApplied
// =============================================================================

describe('emitConstraintApplied', () => {
  it('returns a valid UUID constraint_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const constraintId = emitConstraintApplied(svc, makeConstraintDetails());
    expect(constraintId).toMatch(UUID_RE);
  });

  it('returns a different constraint_id on each call', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const id1 = emitConstraintApplied(svc, makeConstraintDetails());
    const id2 = emitConstraintApplied(svc, makeConstraintDetails());
    expect(id1).not.toBe(id2);
  });

  it('emits an observation with kind constraint_applied', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('constraint_applied');
  });

  it('emits constraint_id matching the returned value', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const constraintId = emitConstraintApplied(svc, makeConstraintDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.constraint_id).toBe(constraintId);
  });

  it('emits the provided session_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.session_id).toBe(VALID_UUID);
  });

  it('emits the constraint_type', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails({ constraint_type: 'scope' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.constraint_type).toBe('scope');
  });

  it('supports all valid constraint_type values', async () => {
    const types = ['policy', 'rule', 'scope', 'environment', 'approval_required'] as const;
    for (const constraint_type of types) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitConstraintApplied(svc, makeConstraintDetails({ constraint_type }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
      expect(obs.constraint_type).toBe(constraint_type);
    }
  });

  it('emits prevented_action.action_type', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(
      svc,
      makeConstraintDetails({
        prevented_action: { action_type: 'deploy', action_target: 'production' },
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.prevented_action.action_type).toBe('deploy');
  });

  it('emits prevented_action.action_target when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(
      svc,
      makeConstraintDetails({
        prevented_action: { action_type: 'file_delete', action_target: '/src/core.ts' },
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.prevented_action.action_target).toBe('/src/core.ts');
  });

  it('omits prevented_action.action_target when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(
      svc,
      makeConstraintDetails({
        prevented_action: { action_type: 'command' },
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.prevented_action.action_target).toBeUndefined();
  });

  it('emits the reason', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails({ reason: 'no-production-writes' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.reason).toBe('no-production-writes');
  });

  it('emits the optional scope when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails({ scope: 'src/ only' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.scope).toBe('src/ only');
  });

  it('omits scope when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.scope).toBeUndefined();
  });

  it('emits the optional environment when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails({ environment: 'not in production' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.environment).toBe('not in production');
  });

  it('omits environment when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.environment).toBeUndefined();
  });

  it('emits options_available when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(
      svc,
      makeConstraintDetails({ options_available: ['staging', 'development', 'production'] })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.options_available).toEqual(['staging', 'development', 'production']);
  });

  it('emits options_allowed when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(
      svc,
      makeConstraintDetails({ options_allowed: ['staging', 'development'] })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.options_allowed).toEqual(['staging', 'development']);
  });

  it('omits options_available and options_allowed when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'constraint_applied' }>;
    expect(obs.options_available).toBeUndefined();
    expect(obs.options_allowed).toBeUndefined();
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitConstraintApplied(svc, makeConstraintDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(TIMESTAMP_RE);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitConstraintApplied(svc, makeConstraintDetails());
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
    expect(() => emitConstraintApplied(svc, makeConstraintDetails())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});
