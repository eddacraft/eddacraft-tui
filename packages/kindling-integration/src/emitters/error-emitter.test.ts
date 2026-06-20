/**
 * Error Emitter Tests (KINDLING-008)
 *
 * Covers emitError.
 * "Errors are not noise, they are data." Error observations record all
 * failures — command failures, tool errors, aborted executions, partial
 * states, and validation failures.
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from '../kindling-service.js';
import { KindlingConfigSchema } from '../config.js';
import type { Observation } from '../observation-contract.js';
import type { QueryRequest, QueryResponse } from '../query-contract.js';
import { emitError, type ErrorDetails } from './error-emitter.js';

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

function makeErrorDetails(overrides: Partial<ErrorDetails> = {}): ErrorDetails {
  return {
    session_id: VALID_UUID,
    error_type: 'command_failure',
    context: {
      component: 'gate:architecture',
    },
    error_message: 'Gate evaluation failed: circular dependency detected',
    recoverable: true,
    ...overrides,
  };
}

// =============================================================================
// emitError
// =============================================================================

describe('emitError', () => {
  it('returns a valid UUID error_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const errorId = emitError(svc, makeErrorDetails());
    expect(errorId).toMatch(UUID_RE);
  });

  it('returns a different error_id on each call', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const id1 = emitError(svc, makeErrorDetails());
    const id2 = emitError(svc, makeErrorDetails());
    expect(id1).not.toBe(id2);
  });

  it('emits an observation with kind error', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('error');
  });

  it('emits error_id matching the returned value', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const errorId = emitError(svc, makeErrorDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.error_id).toBe(errorId);
  });

  it('emits the provided session_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.session_id).toBe(VALID_UUID);
  });

  it('emits the error_type', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ error_type: 'tool_error' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.error_type).toBe('tool_error');
  });

  it('supports all valid error_type values', async () => {
    const types = [
      'command_failure',
      'tool_error',
      'aborted_execution',
      'partial_state',
      'validation_failure',
    ] as const;
    for (const error_type of types) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitError(svc, makeErrorDetails({ error_type }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
      expect(obs.error_type).toBe(error_type);
    }
  });

  it('emits context.component', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ context: { component: 'cli:watch' } }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.context.component).toBe('cli:watch');
  });

  it('emits context.action_id when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(
      svc,
      makeErrorDetails({ context: { component: 'executor', action_id: 'action-abc-123' } })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.context.action_id).toBe('action-abc-123');
  });

  it('omits context.action_id when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ context: { component: 'executor' } }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.context.action_id).toBeUndefined();
  });

  it('emits context.gate_id when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(
      svc,
      makeErrorDetails({ context: { component: 'gate-runner', gate_id: 'coverage' } })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.context.gate_id).toBe('coverage');
  });

  it('omits context.gate_id when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ context: { component: 'gate-runner' } }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.context.gate_id).toBeUndefined();
  });

  it('emits the error_message', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ error_message: 'File not found: config.json' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.error_message).toBe('File not found: config.json');
  });

  it('emits error_code when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ error_code: 'ENOENT' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.error_code).toBe('ENOENT');
  });

  it('omits error_code when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.error_code).toBeUndefined();
  });

  it('emits exit_code when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ exit_code: 1 }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.exit_code).toBe(1);
  });

  it('omits exit_code when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.exit_code).toBeUndefined();
  });

  it('emits recoverable: true for recoverable errors', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ recoverable: true }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.recoverable).toBe(true);
  });

  it('emits recoverable: false for non-recoverable errors', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails({ recoverable: false }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.recoverable).toBe(false);
  });

  it('emits partial_state_description when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(
      svc,
      makeErrorDetails({
        error_type: 'partial_state',
        partial_state_description: '3 of 5 files written before abort',
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.partial_state_description).toBe('3 of 5 files written before abort');
  });

  it('omits partial_state_description when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'error' }>;
    expect(obs.partial_state_description).toBeUndefined();
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitError(svc, makeErrorDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(TIMESTAMP_RE);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitError(svc, makeErrorDetails());
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
    expect(() => emitError(svc, makeErrorDetails())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});
