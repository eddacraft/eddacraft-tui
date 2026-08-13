/**
 * KindlingService lifecycle tests (TCOV-018)
 *
 * Covers: service construction, enabled/disabled state, emit lifecycle,
 * query lifecycle, close behaviour, shouldCapture gating, and factory function.
 * Does NOT duplicate redaction enforcement cases from kindling-service.redaction.test.ts.
 */

import { afterEach, describe, it, expect, vi } from 'vitest';
import {
  KindlingService,
  NoOpKindlingStore,
  ObservationValidationError,
  QueryValidationError,
  createKindlingService,
  type IKindlingStore,
} from './kindling-service.js';
import type { Observation } from './observation-contract.js';
import type { QueryRequest, QueryResponse } from './query-contract.js';
import { KindlingConfigSchema, DEFAULT_KINDLING_CONFIG } from './config.js';

// =============================================================================
// Test helpers
// =============================================================================

const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const SESSION_UUID = 'aaaabbbb-cccc-4ddd-aeee-ffffffffffff';
const VALID_TIMESTAMP = '2026-02-15T10:00:00.000Z';

const enabledConfig = KindlingConfigSchema.parse({ enabled: true });
const disabledConfig = KindlingConfigSchema.parse({ enabled: false });

afterEach(() => {
  vi.restoreAllMocks();
});

function makeSessionStart(): Observation {
  return {
    kind: 'session_start',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    context: {
      working_directory: '/home/user/project',
      anvil_version: '0.8.0',
      command: 'check',
      args: [],
      environment: 'development',
    },
  };
}

function makeSessionEnd(): Observation {
  return {
    kind: 'session_end',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    outcome: 'success',
    exit_code: 0,
    duration_ms: 1200,
    summary: {
      gates_evaluated: 1,
      gates_passed: 1,
      gates_failed: 0,
      actions_executed: 0,
      errors_encountered: 0,
    },
  };
}

function makeGateEvaluated(): Observation {
  return {
    kind: 'gate_evaluated',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    gate_eval_id: 'ge-001',
    gate_id: 'architecture',
    inputs: {},
    outcome: 'pass',
    rules_evaluated: ['rule-a'],
    enforcement: 'blocking',
    duration_ms: 50,
  };
}

function makeError(): Observation {
  return {
    kind: 'error',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    error_id: 'err-001',
    error_type: 'command_failure',
    context: { component: 'gate:architecture' },
    error_message: 'Gate check failed',
    recoverable: false,
  };
}

function makeActionExecuted(): Observation {
  return {
    kind: 'action_executed',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    action_id: 'action-001',
    action_type: 'command',
    details: { command: 'ls -la', working_directory: '/home/user' },
    outcome: 'success',
    duration_ms: 100,
  };
}

function makeConstraintApplied(): Observation {
  return {
    kind: 'constraint_applied',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    constraint_id: 'cst-001',
    constraint_type: 'policy',
    prevented_action: { action_type: 'file_write' },
    reason: 'readonly-policy',
  };
}

function makeHumanInput(): Observation {
  return {
    kind: 'human_input',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    input_type: 'approval',
    context: {},
    decision: 'approved',
    user_identifier: 'test-user',
  };
}

function makePlanCreated(): Observation {
  return {
    kind: 'plan_created',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    plan_id: 'plan-001',
    plan_version: '1.0',
    plan_path: 'plans/my-plan.aps.md',
    plan_hash: 'abc123',
    created_by: 'human',
  };
}

/** Returns a spy store recording every emitted observation */
function makeSpyStore(): { store: IKindlingStore; writes: Observation[]; queries: QueryRequest[] } {
  const writes: Observation[] = [];
  const queries: QueryRequest[] = [];
  const store: IKindlingStore = {
    emit: async (o) => {
      writes.push(o);
    },
    query: async (r): Promise<QueryResponse> => {
      queries.push(r);
      return {
        metadata: {
          query_id: VALID_UUID,
          executed_at: VALID_TIMESTAMP,
          contract_version: '1.0.0',
          result_count: 0,
          truncated: false,
          truncation_reason: 'none',
        },
        observations: [],
      };
    },
    close: async () => {},
  };
  return { store, writes, queries };
}

/** Returns a minimal valid session-scope QueryRequest */
function makeSessionQuery(): QueryRequest {
  return {
    scope: 'session',
    session_id: SESSION_UUID,
    shape: 'timeline',
    format: 'json',
    max_results: 10,
    max_payload_bytes: 1024,
  };
}

// =============================================================================
// NoOpKindlingStore
// =============================================================================

describe('NoOpKindlingStore', () => {
  it('emit resolves without writing', async () => {
    const store = new NoOpKindlingStore();
    await expect(store.emit(makeSessionStart())).resolves.toBeUndefined();
  });

  it('query returns empty response with valid metadata', async () => {
    const store = new NoOpKindlingStore();
    const resp = await store.query(makeSessionQuery());
    expect(resp.observations).toHaveLength(0);
    expect(resp.metadata.result_count).toBe(0);
    expect(resp.metadata.truncated).toBe(false);
    expect(resp.metadata.truncation_reason).toBe('none');
    expect(resp.metadata.contract_version).toBe('1.0.0');
    // query_id is a fresh UUID each call
    expect(resp.metadata.query_id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/
    );
  });

  it('close resolves without error', async () => {
    const store = new NoOpKindlingStore();
    await expect(store.close()).resolves.toBeUndefined();
  });
});

// =============================================================================
// KindlingService — construction and properties
// =============================================================================

describe('KindlingService — construction', () => {
  it('exposes enabled=true when config says enabled', () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    expect(svc.enabled).toBe(true);
  });

  it('exposes enabled=false when config says disabled', () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    expect(svc.enabled).toBe(false);
  });

  it('exposes the configuration as a readonly snapshot', () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    expect(svc.configuration.enabled).toBe(true);
    expect(svc.configuration.query_limits.max_results).toBe(100);
  });
});

// =============================================================================
// KindlingService.emit — lifecycle
// =============================================================================

describe('KindlingService.emit — enabled', () => {
  it('delegates a valid observation to the store', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    const obs = makeSessionStart();
    await svc.emit(obs);
    expect(writes).toHaveLength(1);
    expect(writes[0]).toEqual(obs);
  });

  it('throws ObservationValidationError for malformed observations', async () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    // Missing required fields
    await expect(svc.emit({ kind: 'session_start' } as unknown as Observation)).rejects.toThrow(
      ObservationValidationError
    );
  });

  it('ObservationValidationError carries issues array', async () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    try {
      await svc.emit({ kind: 'session_start' } as unknown as Observation);
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(ObservationValidationError);
      expect((e as ObservationValidationError).issues.length).toBeGreaterThan(0);
    }
  });

  it('is a no-op after close() is called', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.close();
    await svc.emit(makeSessionStart());
    expect(writes).toHaveLength(0);
  });

  it('skips emit when shouldCapture returns false (kind not captured)', async () => {
    // Disable gate capture
    const config = KindlingConfigSchema.parse({
      enabled: true,
      capture: {
        sessions: true,
        gates: false, // <-- disabled
        actions: true,
        plans: true,
        human_inputs: true,
        constraints: true,
        errors: true,
      },
    });
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, config);
    await svc.emit(makeGateEvaluated());
    expect(writes).toHaveLength(0);
  });

  it('captures session_start when sessions capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makeSessionStart());
    expect(writes).toHaveLength(1);
  });

  it('captures session_end when sessions capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makeSessionEnd());
    expect(writes).toHaveLength(1);
  });

  it('captures gate_evaluated when gates capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makeGateEvaluated());
    expect(writes).toHaveLength(1);
  });

  it('captures action_executed when actions capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makeActionExecuted());
    expect(writes).toHaveLength(1);
  });

  it('captures error observations when errors capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makeError());
    expect(writes).toHaveLength(1);
  });

  it('captures constraint_applied when constraints capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makeConstraintApplied());
    expect(writes).toHaveLength(1);
  });

  it('captures human_input when human_inputs capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makeHumanInput());
    expect(writes).toHaveLength(1);
  });

  it('captures plan_created when plans capture enabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.emit(makePlanCreated());
    expect(writes).toHaveLength(1);
  });

  it('does not write to store when disabled', async () => {
    const { store, writes } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    await svc.emit(makeSessionStart());
    expect(writes).toHaveLength(0);
  });
});

// =============================================================================
// KindlingService.query
// =============================================================================

describe('KindlingService.query', () => {
  it('delegates a valid query to the store', async () => {
    const { store, queries } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    const q = makeSessionQuery();
    const resp = await svc.query(q);
    expect(queries).toHaveLength(1);
    expect(resp.observations).toHaveLength(0);
  });

  it('throws QueryValidationError for invalid query request', async () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await expect(svc.query({ scope: 'session' } as unknown as QueryRequest)).rejects.toThrow(
      QueryValidationError
    );
  });

  it('caps max_results to config limit when request exceeds it', async () => {
    const config = KindlingConfigSchema.parse({
      enabled: true,
      query_limits: { max_results: 5, max_payload_bytes: 1024 * 1024 },
    });
    const { store, queries } = makeSpyStore();
    const svc = new KindlingService(store, config);
    await svc.query({
      scope: 'session',
      session_id: SESSION_UUID,
      shape: 'timeline',
      format: 'json',
      max_results: 500, // exceeds config cap of 5
      max_payload_bytes: 1024 * 1024,
    });
    expect(queries[0].max_results).toBe(5);
  });

  it('caps max_payload_bytes to config limit when request exceeds it', async () => {
    const config = KindlingConfigSchema.parse({
      enabled: true,
      query_limits: { max_results: 100, max_payload_bytes: 512 },
    });
    const { store, queries } = makeSpyStore();
    const svc = new KindlingService(store, config);
    await svc.query({
      scope: 'session',
      session_id: SESSION_UUID,
      shape: 'timeline',
      format: 'json',
      max_results: 10,
      max_payload_bytes: 10 * 1024 * 1024, // exceeds config cap of 512
    });
    expect(queries[0].max_payload_bytes).toBe(512);
  });

  it('honours request limits when below config cap', async () => {
    const { store, queries } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.query({
      scope: 'session',
      session_id: SESSION_UUID,
      shape: 'timeline',
      format: 'json',
      max_results: 3, // below config cap of 100
      max_payload_bytes: 1024,
    });
    expect(queries[0].max_results).toBe(3);
  });

  it('truncates a store response that exceeds config max_results', async () => {
    const config = KindlingConfigSchema.parse({
      enabled: true,
      query_limits: { max_results: 1, max_payload_bytes: 1024 * 1024 },
    });
    const store: IKindlingStore = {
      emit: async () => {},
      query: async (): Promise<QueryResponse> => ({
        metadata: {
          query_id: VALID_UUID,
          executed_at: VALID_TIMESTAMP,
          contract_version: '1.0.0',
          result_count: 10,
          truncated: false,
          truncation_reason: 'none',
        },
        observations: Array.from({ length: 10 }, (_, i) => ({
          id: `550e8400-e29b-41d4-a716-44665544${String(i).padStart(4, '0')}`,
          kind: 'session_start' as const,
          timestamp: VALID_TIMESTAMP,
          session_id: SESSION_UUID,
          provenance: [],
          payload: {},
        })),
      }),
      close: async () => {},
    };
    const svc = new KindlingService(store, config);
    const resp = await svc.query({
      scope: 'session',
      session_id: SESSION_UUID,
      shape: 'timeline',
      format: 'json',
      max_results: 10,
      max_payload_bytes: 1024 * 1024,
    });
    expect(resp.observations).toHaveLength(1);
    expect(resp.metadata.result_count).toBe(1);
    expect(resp.metadata.truncated).toBe(true);
    expect(resp.metadata.truncation_reason).toBe('max_results');
  });

  it('truncates a store response that exceeds config max_payload_bytes', async () => {
    const largePayload = 'x'.repeat(600);
    const config = KindlingConfigSchema.parse({
      enabled: true,
      query_limits: { max_results: 100, max_payload_bytes: 100 },
    });
    const store: IKindlingStore = {
      emit: async () => {},
      query: async (): Promise<QueryResponse> => ({
        metadata: {
          query_id: VALID_UUID,
          executed_at: VALID_TIMESTAMP,
          contract_version: '1.0.0',
          result_count: 3,
          truncated: false,
          truncation_reason: 'none',
        },
        observations: Array.from({ length: 3 }, (_, i) => ({
          id: `550e8400-e29b-41d4-a716-44665544${String(i).padStart(4, '0')}`,
          kind: 'session_start' as const,
          timestamp: VALID_TIMESTAMP,
          session_id: SESSION_UUID,
          provenance: [],
          payload: { data: largePayload },
        })),
      }),
      close: async () => {},
    };
    const svc = new KindlingService(store, config);
    const resp = await svc.query({
      scope: 'session',
      session_id: SESSION_UUID,
      shape: 'timeline',
      format: 'json',
      max_results: 100,
      max_payload_bytes: 10 * 1024 * 1024,
    });
    expect(resp.observations.length).toBeLessThan(3);
    expect(resp.metadata.truncated).toBe(true);
    expect(resp.metadata.truncation_reason).toBe('max_payload_bytes');
    expect(resp.metadata.result_count).toBe(resp.observations.length);
    const payloadBytes = new TextEncoder().encode(JSON.stringify(resp.observations)).byteLength;
    expect(payloadBytes).toBeLessThanOrEqual(config.query_limits.max_payload_bytes);
  });

  it('throws QueryValidationError after service is closed', async () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.close();
    await expect(svc.query(makeSessionQuery())).rejects.toThrow(QueryValidationError);
  });

  it('QueryValidationError has the correct name', async () => {
    const { store } = makeSpyStore();
    const svc = new KindlingService(store, enabledConfig);
    await svc.close();
    try {
      await svc.query(makeSessionQuery());
      expect.fail('should have thrown');
    } catch (e) {
      expect((e as Error).name).toBe('QueryValidationError');
    }
  });
});

// =============================================================================
// KindlingService.close
// =============================================================================

describe('KindlingService.close', () => {
  it('calls store.close once', async () => {
    const { store } = makeSpyStore();
    const closespy = vi.spyOn(store, 'close');
    const svc = new KindlingService(store, enabledConfig);
    await svc.close();
    expect(closespy).toHaveBeenCalledTimes(1);
  });

  it('is idempotent — a second close call does not call store.close again', async () => {
    const { store } = makeSpyStore();
    const closespy = vi.spyOn(store, 'close');
    const svc = new KindlingService(store, enabledConfig);
    await svc.close();
    await svc.close();
    expect(closespy).toHaveBeenCalledTimes(1);
  });
});

// =============================================================================
// createKindlingService factory
// =============================================================================

describe('createKindlingService', () => {
  it('returns a KindlingService instance', () => {
    const svc = createKindlingService(enabledConfig);
    expect(svc).toBeInstanceOf(KindlingService);
  });

  it('uses DEFAULT_KINDLING_CONFIG when no config provided', () => {
    const svc = createKindlingService();
    expect(svc.enabled).toBe(DEFAULT_KINDLING_CONFIG.enabled);
  });

  it('uses the provided store when given', async () => {
    const { store, writes } = makeSpyStore();
    const svc = createKindlingService(enabledConfig, store);
    await svc.emit(makeSessionStart());
    expect(writes).toHaveLength(1);
  });

  it('falls back to NoOpKindlingStore when no store provided', async () => {
    // Emit should not throw even with no store
    const svc = createKindlingService(enabledConfig);
    await expect(svc.emit(makeSessionStart())).resolves.toBeUndefined();
  });
});

// =============================================================================
// ObservationValidationError and QueryValidationError
// =============================================================================

describe('error types', () => {
  it('ObservationValidationError has correct name and issues', () => {
    const err = new ObservationValidationError('msg', ['issue-1', 'issue-2']);
    expect(err.name).toBe('ObservationValidationError');
    expect(err.message).toBe('msg');
    expect(err.issues).toEqual(['issue-1', 'issue-2']);
  });

  it('QueryValidationError has correct name', () => {
    const err = new QueryValidationError('bad query');
    expect(err.name).toBe('QueryValidationError');
    expect(err.message).toBe('bad query');
  });
});
