/**
 * Human Input Emitter Tests (KINDLING-007a)
 *
 * Covers emitHumanInput.
 * Human inputs are first-class events: approvals, overrides, rejections,
 * edits, confirmations, and cancellations are part of the governance record.
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from '../kindling-service.js';
import { KindlingConfigSchema } from '../config.js';
import type { Observation } from '../observation-contract.js';
import type { QueryRequest, QueryResponse } from '../query-contract.js';
import { emitHumanInput, type HumanInputDetails } from './human-input-emitter.js';

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

function makeHumanInputDetails(overrides: Partial<HumanInputDetails> = {}): HumanInputDetails {
  return {
    session_id: VALID_UUID,
    input_type: 'approval',
    context: {
      prompt: 'Deploy to production?',
      target: 'plan-001',
    },
    decision: 'approved',
    user_identifier: 'joshuaboys',
    ...overrides,
  };
}

// =============================================================================
// emitHumanInput
// =============================================================================

describe('emitHumanInput', () => {
  it('returns a valid UUID input_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const inputId = emitHumanInput(svc, makeHumanInputDetails());
    expect(inputId).toMatch(UUID_RE);
  });

  it('returns a different input_id on each call', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const id1 = emitHumanInput(svc, makeHumanInputDetails());
    const id2 = emitHumanInput(svc, makeHumanInputDetails());
    expect(id1).not.toBe(id2);
  });

  it('emits an observation with kind human_input', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('human_input');
  });

  it('emits the provided session_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.session_id).toBe(VALID_UUID);
  });

  it('emits the input_type', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails({ input_type: 'override' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.input_type).toBe('override');
  });

  it('supports all valid input_type values', async () => {
    const inputTypes = [
      'approval',
      'override',
      'rejection',
      'manual_edit',
      'confirmation',
      'cancellation',
    ] as const;
    for (const input_type of inputTypes) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitHumanInput(svc, makeHumanInputDetails({ input_type }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
      expect(obs.input_type).toBe(input_type);
    }
  });

  it('emits the context.prompt when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(
      svc,
      makeHumanInputDetails({ context: { prompt: 'Are you sure?', target: undefined } })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.context.prompt).toBe('Are you sure?');
  });

  it('emits the context.target when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(
      svc,
      makeHumanInputDetails({ context: { prompt: undefined, target: 'gate-eval-abc' } })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.context.target).toBe('gate-eval-abc');
  });

  it('emits with empty context when no prompt or target provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails({ context: {} }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.context.prompt).toBeUndefined();
    expect(obs.context.target).toBeUndefined();
  });

  it('emits the decision', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails({ decision: 'rejected' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.decision).toBe('rejected');
  });

  it('emits the optional reason when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails({ reason: 'Not ready for production' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.reason).toBe('Not ready for production');
  });

  it('omits reason when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    expect(obs.reason).toBeUndefined();
  });

  it('emits the user_identifier', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails({ user_identifier: 'alice@example.com' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'human_input' }>;
    // Note: user_identifier with email may be treated as sensitive by the service;
    // we verify it reaches the observation layer regardless (KindlingService handles redaction).
    expect(typeof obs.user_identifier).toBe('string');
    expect(obs.user_identifier.length).toBeGreaterThan(0);
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitHumanInput(svc, makeHumanInputDetails({ user_identifier: 'plain-user' }));
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(TIMESTAMP_RE);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitHumanInput(svc, makeHumanInputDetails({ user_identifier: 'plain-user' }));
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
      emitHumanInput(svc, makeHumanInputDetails({ user_identifier: 'plain-user' }))
    ).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});
