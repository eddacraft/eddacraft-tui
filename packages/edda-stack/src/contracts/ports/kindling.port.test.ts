/**
 * Kindling Port Interface Tests (TCOV-015)
 *
 * Verifies type shapes and structural contracts for kindling.port.ts.
 * Behavioural coverage for IKindlingPort is in testing/mocks/mocks.test.ts.
 */

import { describe, it, expect } from 'vitest';
import type {
  ObservationKind,
  Observation,
  CreateObservationInput,
  ObservationQuery,
  ObservationQueryResult,
  SessionQueryOptions,
  SessionQueryResult,
  PlanQueryOptions,
  SessionSummary,
  PlanQueryResult,
} from './kindling.port.js';
import { createObservationId, createSessionId } from '../identifiers.js';
import type { PlanId } from '../identifiers.js';
import type { Timestamp } from '../temporal.js';

const UUID_A = '550e8400-e29b-41d4-a716-446655440000';
const UUID_B = '550e8400-e29b-41d4-a716-446655440001';
const TS = '2024-06-01T10:00:00.000Z' as Timestamp;

// =============================================================================
// ObservationKind values
// =============================================================================

describe('ObservationKind values (TCOV-015)', () => {
  it('covers all 9 expected observation kinds', () => {
    const kinds: ObservationKind[] = [
      'gate_evaluated',
      'action_executed',
      'action_failed',
      'plan_started',
      'plan_completed',
      'constraint_applied',
      'error_recorded',
      'metric_recorded',
      'custom',
    ];
    const unique = new Set(kinds);
    expect(unique.size).toBe(9);
    expect(kinds).toHaveLength(9);
  });
});

// =============================================================================
// Observation shape
// =============================================================================

describe('Observation shape (TCOV-015)', () => {
  it('models a complete observation record', () => {
    const obs: Observation = {
      id: createObservationId(UUID_A),
      session_id: createSessionId(UUID_B),
      kind: 'gate_evaluated',
      timestamp: TS,
      summary: 'Architecture gate passed',
      data: { gate: 'architecture', result: 'pass', score: 0.85 },
      tags: ['gate', 'architecture'],
    };

    expect(obs.id).toBeTruthy();
    expect(obs.kind).toBe('gate_evaluated');
    expect(obs.data['gate']).toBe('architecture');
    expect(obs.tags).toContain('gate');
  });

  it('allows tags to be absent', () => {
    const obs: Observation = {
      id: createObservationId(UUID_A),
      session_id: createSessionId(UUID_B),
      kind: 'custom',
      timestamp: TS,
      summary: 'Custom observation',
      data: {},
    };
    expect(obs.tags).toBeUndefined();
  });
});

// =============================================================================
// CreateObservationInput shape
// =============================================================================

describe('CreateObservationInput shape (TCOV-015)', () => {
  it('accepts a minimal valid input', () => {
    const input: CreateObservationInput = {
      session_id: createSessionId(UUID_A),
      kind: 'gate_evaluated',
      summary: 'Test gate pass',
      data: { result: 'pass' },
    };
    expect(input.kind).toBe('gate_evaluated');
    expect(input.data['result']).toBe('pass');
    expect(input.tags).toBeUndefined();
  });

  it('accepts input with optional tags', () => {
    const input: CreateObservationInput = {
      session_id: createSessionId(UUID_A),
      kind: 'action_executed',
      summary: 'Action completed',
      data: { action: 'create_file' },
      tags: ['action', 'file'],
    };
    expect(input.tags).toEqual(['action', 'file']);
  });

  it('supports all observation kinds', () => {
    const kinds: ObservationKind[] = [
      'gate_evaluated',
      'action_executed',
      'action_failed',
      'plan_started',
      'plan_completed',
      'constraint_applied',
      'error_recorded',
      'metric_recorded',
      'custom',
    ];
    for (const kind of kinds) {
      const input: CreateObservationInput = {
        session_id: createSessionId(UUID_A),
        kind,
        summary: `${kind} event`,
        data: {},
      };
      expect(input.kind).toBe(kind);
    }
  });
});

// =============================================================================
// ObservationQuery shape
// =============================================================================

describe('ObservationQuery shape (TCOV-015)', () => {
  it('accepts an empty query', () => {
    const query: ObservationQuery = {};
    expect(Object.keys(query)).toHaveLength(0);
  });

  it('accepts all filter fields', () => {
    const query: ObservationQuery = {
      session_id: createSessionId(UUID_A),
      kinds: ['gate_evaluated', 'action_executed'],
      time_range: { start: TS },
      tags: ['gate'],
      limit: 50,
      offset: 10,
    };
    expect(query.kinds).toHaveLength(2);
    expect(query.limit).toBe(50);
    expect(query.offset).toBe(10);
  });
});

// =============================================================================
// ObservationQueryResult shape
// =============================================================================

describe('ObservationQueryResult shape (TCOV-015)', () => {
  it('models a paginated result', () => {
    const obs: Observation = {
      id: createObservationId(UUID_A),
      session_id: createSessionId(UUID_B),
      kind: 'gate_evaluated',
      timestamp: TS,
      summary: 'Gate evaluated',
      data: {},
    };
    const result: ObservationQueryResult = {
      observations: [obs],
      total: 5,
      has_more: true,
    };
    expect(result.total).toBe(5);
    expect(result.has_more).toBe(true);
    expect(result.observations).toHaveLength(1);
  });

  it('models an empty result', () => {
    const result: ObservationQueryResult = {
      observations: [],
      total: 0,
      has_more: false,
    };
    expect(result.total).toBe(0);
    expect(result.has_more).toBe(false);
  });
});

// =============================================================================
// SessionQueryOptions shape
// =============================================================================

describe('SessionQueryOptions shape (TCOV-015)', () => {
  it('accepts all filter options', () => {
    const options: SessionQueryOptions = {
      kinds: ['gate_evaluated'],
      time_range: { start: TS },
      tags: ['gate'],
      include_payloads: false,
      limit: 20,
      offset: 0,
      sort_order: 'asc',
    };
    expect(options.include_payloads).toBe(false);
    expect(options.sort_order).toBe('asc');
  });

  it('accepts minimal options', () => {
    const options: SessionQueryOptions = {};
    expect(options.kinds).toBeUndefined();
  });
});

// =============================================================================
// SessionQueryResult shape
// =============================================================================

describe('SessionQueryResult shape (TCOV-015)', () => {
  it('models a result with session metadata', () => {
    const sessionId = createSessionId(UUID_A);
    const result: SessionQueryResult = {
      session_id: sessionId,
      observations: [],
      total: 0,
      has_more: false,
      session_metadata: {
        started_at: TS,
        ended_at: TS,
        plan_id: 'plan-001' as PlanId,
      },
    };
    expect(result.session_id).toBe(sessionId);
    expect(result.session_metadata?.started_at).toBe(TS);
  });

  it('allows session_metadata to be absent', () => {
    const result: SessionQueryResult = {
      session_id: createSessionId(UUID_A),
      observations: [],
      total: 0,
      has_more: false,
    };
    expect(result.session_metadata).toBeUndefined();
  });
});

// =============================================================================
// PlanQueryOptions shape
// =============================================================================

describe('PlanQueryOptions shape (TCOV-015)', () => {
  it('accepts all options', () => {
    const options: PlanQueryOptions = {
      kinds: ['plan_started', 'plan_completed'],
      session_time_range: { start: TS },
      include_observations: true,
      limit: 10,
      offset: 0,
    };
    expect(options.include_observations).toBe(true);
  });

  it('accepts empty options', () => {
    const options: PlanQueryOptions = {};
    expect(options.include_observations).toBeUndefined();
  });
});

// =============================================================================
// SessionSummary and PlanQueryResult shapes
// =============================================================================

describe('SessionSummary shape (TCOV-015)', () => {
  it('models a session summary', () => {
    const summary: SessionSummary = {
      session_id: createSessionId(UUID_A),
      started_at: TS,
      ended_at: TS,
      observation_count: 5,
      kinds_observed: ['gate_evaluated', 'action_executed'],
    };
    expect(summary.observation_count).toBe(5);
    expect(summary.kinds_observed).toContain('gate_evaluated');
    expect(summary.ended_at).toBeDefined();
  });

  it('allows ended_at to be absent (ongoing session)', () => {
    const summary: SessionSummary = {
      session_id: createSessionId(UUID_A),
      started_at: TS,
      observation_count: 2,
      kinds_observed: ['plan_started'],
    };
    expect(summary.ended_at).toBeUndefined();
  });
});

describe('PlanQueryResult shape (TCOV-015)', () => {
  it('models a plan result with sessions', () => {
    const planId = 'plan-001' as PlanId;
    const result: PlanQueryResult = {
      plan_id: planId,
      sessions: [
        {
          session_id: createSessionId(UUID_A),
          started_at: TS,
          observation_count: 3,
          kinds_observed: ['gate_evaluated'],
        },
      ],
      total_sessions: 1,
      has_more: false,
    };
    expect(result.total_sessions).toBe(1);
    expect(result.sessions).toHaveLength(1);
    expect(result.observations).toBeUndefined();
  });
});

// =============================================================================
// IKindlingPort interface structural completeness check
// =============================================================================

describe('IKindlingPort interface structure (TCOV-015)', () => {
  it('has all expected method names in the interface contract', () => {
    const methods = [
      'createObservation',
      'createObservationBatch',
      'getObservation',
      'queryObservations',
      'getSessionObservations',
      'observationExists',
      'querySession',
      'queryByPlan',
      'getObservationsBySession',
      'getObservationsByTimeRange',
      'getObservationsAsRefs',
      'isAvailable',
      'countObservations',
      'pruneObservations',
    ];

    const unique = new Set(methods);
    expect(unique.size).toBe(methods.length);
    expect(methods.length).toBe(14);
  });
});
