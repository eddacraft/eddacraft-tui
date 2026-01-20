/**
 * Kindling Port Mock (STACK-009)
 *
 * Mock implementation of IKindlingPort for testing.
 * Uses in-memory storage and vitest mock functions.
 *
 * @module @eddacraft/anvil-edda-stack/testing/mocks/kindling
 */

import { vi, type Mock } from 'vitest';
import { v4 as uuidv4 } from 'uuid';
import type {
  IKindlingPort,
  Observation,
  CreateObservationInput,
  ObservationQuery,
  ObservationQueryResult,
  SessionQueryOptions,
  SessionQueryResult,
  PlanQueryOptions,
  PlanQueryResult,
  SessionSummary,
} from '../../contracts/ports/kindling.port.js';
import type { ObservationId, SessionId, PlanId, Timestamp } from '../../contracts/index.js';
import type { KindlingRef, TimeRange } from '../../contracts/index.js';
import { now } from '../../contracts/temporal.js';
import { createObservationId, createSessionId } from '../../contracts/identifiers.js';

// =============================================================================
// Mock Options
// =============================================================================

/**
 * Options for creating a mock Kindling port
 */
export interface MockKindlingPortOptions {
  /** Initial observations to populate the store */
  initialObservations?: Observation[];

  /** Default session ID for new observations */
  defaultSessionId?: SessionId;

  /** Whether to auto-generate IDs (default: true) */
  autoGenerateIds?: boolean;
}

// =============================================================================
// Mock Implementation
// =============================================================================

/**
 * In-memory implementation of IKindlingPort for testing
 */
export interface MockKindlingPort extends IKindlingPort {
  /** Access to the underlying observation store */
  _store: Map<ObservationId, Observation>;

  /** Reset the mock to initial state */
  _reset: () => void;

  /** Get all observations (for assertions) */
  _getAll: () => Observation[];

  /** Mock function references for verification */
  _mocks: {
    createObservation: Mock;
    createObservationBatch: Mock;
    getObservation: Mock;
    queryObservations: Mock;
    getSessionObservations: Mock;
    observationExists: Mock;
    querySession: Mock;
    queryByPlan: Mock;
    getObservationsBySession: Mock;
    getObservationsByTimeRange: Mock;
    getObservationsAsRefs: Mock;
    isAvailable: Mock;
    countObservations: Mock;
    pruneObservations: Mock;
  };
}

/**
 * Create a mock Kindling port for testing
 */
export function createMockKindlingPort(options: MockKindlingPortOptions = {}): MockKindlingPort {
  const {
    initialObservations = [],
    defaultSessionId = createSessionId(uuidv4()),
    autoGenerateIds = true,
  } = options;

  // In-memory store
  const store = new Map<ObservationId, Observation>();

  // Populate initial observations
  for (const obs of initialObservations) {
    store.set(obs.id, obs);
  }

  // Create observation implementation
  const createObservationImpl = async (input: CreateObservationInput): Promise<Observation> => {
    const id = autoGenerateIds ? createObservationId(uuidv4()) : ('' as ObservationId);
    const observation: Observation = {
      id,
      session_id: input.session_id || defaultSessionId,
      kind: input.kind,
      timestamp: now(),
      summary: input.summary,
      data: input.data,
      tags: input.tags,
    };
    store.set(id, observation);
    return observation;
  };

  // Create batch implementation
  const createObservationBatchImpl = async (
    inputs: CreateObservationInput[]
  ): Promise<Observation[]> => {
    const results: Observation[] = [];
    for (const input of inputs) {
      results.push(await createObservationImpl(input));
    }
    return results;
  };

  // Get observation implementation
  const getObservationImpl = async (id: ObservationId): Promise<Observation | null> => {
    return store.get(id) ?? null;
  };

  // Query observations implementation
  const queryObservationsImpl = async (
    query: ObservationQuery
  ): Promise<ObservationQueryResult> => {
    let observations = Array.from(store.values());

    // Apply filters
    if (query.session_id) {
      observations = observations.filter((o) => o.session_id === query.session_id);
    }

    if (query.kinds && query.kinds.length > 0) {
      observations = observations.filter((o) => query.kinds!.includes(o.kind));
    }

    if (query.time_range) {
      const startTime = new Date(query.time_range.start).getTime();
      const endTime = query.time_range.end ? new Date(query.time_range.end).getTime() : Date.now();
      observations = observations.filter((o) => {
        const obsTime = new Date(o.timestamp).getTime();
        return obsTime >= startTime && obsTime < endTime;
      });
    }

    if (query.tags && query.tags.length > 0) {
      observations = observations.filter(
        (o) => o.tags && o.tags.some((t) => query.tags!.includes(t))
      );
    }

    // Sort by timestamp descending
    observations.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());

    const total = observations.length;
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 100;

    observations = observations.slice(offset, offset + limit);

    return {
      observations,
      total,
      has_more: offset + observations.length < total,
    };
  };

  // Get session observations implementation
  const getSessionObservationsImpl = async (sessionId: SessionId): Promise<Observation[]> => {
    return Array.from(store.values()).filter((o) => o.session_id === sessionId);
  };

  // Observation exists implementation
  const observationExistsImpl = async (id: ObservationId): Promise<boolean> => {
    return store.has(id);
  };

  // Get observations as refs implementation
  const getObservationsAsRefsImpl = async (ids: ObservationId[]): Promise<KindlingRef[]> => {
    const refs: KindlingRef[] = [];
    for (const id of ids) {
      const obs = store.get(id);
      if (obs) {
        refs.push({
          observation_id: obs.id,
          session_id: obs.session_id,
          kind: obs.kind,
          timestamp: obs.timestamp,
        });
      }
    }
    return refs;
  };

  // Count observations implementation
  const countObservationsImpl = async (sessionId?: SessionId): Promise<number> => {
    if (sessionId) {
      return Array.from(store.values()).filter((o) => o.session_id === sessionId).length;
    }
    return store.size;
  };

  // Prune observations implementation
  const pruneObservationsImpl = async (olderThan: Timestamp): Promise<number> => {
    const threshold = new Date(olderThan).getTime();
    let count = 0;
    for (const [id, obs] of store.entries()) {
      if (new Date(obs.timestamp).getTime() < threshold) {
        store.delete(id);
        count++;
      }
    }
    return count;
  };

  // Query session implementation (STACK-007)
  const querySessionImpl = async (
    sessionId: SessionId,
    options?: SessionQueryOptions
  ): Promise<SessionQueryResult> => {
    let observations = Array.from(store.values()).filter((o) => o.session_id === sessionId);

    // Apply filters
    if (options?.kinds && options.kinds.length > 0) {
      observations = observations.filter((o) => options.kinds!.includes(o.kind));
    }

    if (options?.time_range) {
      const startTime = new Date(options.time_range.start).getTime();
      const endTime = options.time_range.end
        ? new Date(options.time_range.end).getTime()
        : Date.now();
      observations = observations.filter((o) => {
        const obsTime = new Date(o.timestamp).getTime();
        return obsTime >= startTime && obsTime < endTime;
      });
    }

    if (options?.tags && options.tags.length > 0) {
      observations = observations.filter(
        (o) => o.tags && o.tags.some((t) => options.tags!.includes(t))
      );
    }

    // Sort
    const sortOrder = options?.sort_order ?? 'desc';
    observations.sort((a, b) => {
      const cmp = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
      return sortOrder === 'desc' ? -cmp : cmp;
    });

    const total = observations.length;
    const offset = options?.offset ?? 0;
    const limit = options?.limit ?? 100;

    // Optionally strip payloads
    if (options?.include_payloads === false) {
      observations = observations.map((o) => ({ ...o, data: {} }));
    }

    observations = observations.slice(offset, offset + limit);

    // Get session metadata
    const sessionObs = Array.from(store.values()).filter((o) => o.session_id === sessionId);
    const timestamps = sessionObs.map((o) => new Date(o.timestamp).getTime());

    return {
      session_id: sessionId,
      observations,
      total,
      has_more: offset + observations.length < total,
      session_metadata:
        sessionObs.length > 0
          ? {
              started_at: new Date(Math.min(...timestamps)).toISOString() as Timestamp,
              ended_at: new Date(Math.max(...timestamps)).toISOString() as Timestamp,
            }
          : undefined,
    };
  };

  // Query by plan implementation (STACK-007)
  const queryByPlanImpl = async (
    planId: PlanId,
    options?: PlanQueryOptions
  ): Promise<PlanQueryResult> => {
    // Find observations that reference this plan (stored in data.plan or data.plan_id)
    let planObs = Array.from(store.values()).filter(
      (o) => o.data?.plan === planId || o.data?.plan_id === planId
    );

    if (options?.kinds && options.kinds.length > 0) {
      planObs = planObs.filter((o) => options.kinds!.includes(o.kind));
    }

    // Group by session
    const sessionMap = new Map<SessionId, Observation[]>();
    for (const obs of planObs) {
      const existing = sessionMap.get(obs.session_id) ?? [];
      sessionMap.set(obs.session_id, [...existing, obs]);
    }

    // Build session summaries
    let sessions: SessionSummary[] = [];
    for (const [sid, obs] of sessionMap.entries()) {
      const timestamps = obs.map((o) => new Date(o.timestamp).getTime());
      const kindsSet = new Set(obs.map((o) => o.kind));
      sessions.push({
        session_id: sid,
        started_at: new Date(Math.min(...timestamps)).toISOString() as Timestamp,
        ended_at: new Date(Math.max(...timestamps)).toISOString() as Timestamp,
        observation_count: obs.length,
        kinds_observed: Array.from(kindsSet),
      });
    }

    // Apply session time range filter
    if (options?.session_time_range) {
      const startTime = new Date(options.session_time_range.start).getTime();
      const endTime = options.session_time_range.end
        ? new Date(options.session_time_range.end).getTime()
        : Date.now();
      sessions = sessions.filter((s) => {
        const sessionTime = new Date(s.started_at).getTime();
        return sessionTime >= startTime && sessionTime < endTime;
      });
    }

    const totalSessions = sessions.length;
    const offset = options?.offset ?? 0;
    const limit = options?.limit ?? 100;

    sessions = sessions.slice(offset, offset + limit);

    return {
      plan_id: planId,
      sessions,
      total_sessions: totalSessions,
      has_more: offset + sessions.length < totalSessions,
      observations: options?.include_observations ? planObs : undefined,
    };
  };

  // Get observations by session implementation (STACK-007)
  const getObservationsBySessionImpl = async (sessionId: SessionId): Promise<Observation[]> => {
    return Array.from(store.values()).filter((o) => o.session_id === sessionId);
  };

  // Get observations by time range implementation (STACK-007)
  const getObservationsByTimeRangeImpl = async (range: TimeRange): Promise<Observation[]> => {
    const startTime = new Date(range.start).getTime();
    const endTime = range.end ? new Date(range.end).getTime() : Date.now();
    return Array.from(store.values()).filter((o) => {
      const obsTime = new Date(o.timestamp).getTime();
      return obsTime >= startTime && obsTime < endTime;
    });
  };

  // Is available implementation (STACK-007)
  const isAvailableImpl = async (): Promise<boolean> => {
    return true;
  };

  // Create mock functions
  const mocks = {
    createObservation: vi.fn(createObservationImpl),
    createObservationBatch: vi.fn(createObservationBatchImpl),
    getObservation: vi.fn(getObservationImpl),
    queryObservations: vi.fn(queryObservationsImpl),
    getSessionObservations: vi.fn(getSessionObservationsImpl),
    observationExists: vi.fn(observationExistsImpl),
    querySession: vi.fn(querySessionImpl),
    queryByPlan: vi.fn(queryByPlanImpl),
    getObservationsBySession: vi.fn(getObservationsBySessionImpl),
    getObservationsByTimeRange: vi.fn(getObservationsByTimeRangeImpl),
    getObservationsAsRefs: vi.fn(getObservationsAsRefsImpl),
    isAvailable: vi.fn(isAvailableImpl),
    countObservations: vi.fn(countObservationsImpl),
    pruneObservations: vi.fn(pruneObservationsImpl),
  };

  return {
    // IKindlingPort implementation
    createObservation: mocks.createObservation,
    createObservationBatch: mocks.createObservationBatch,
    getObservation: mocks.getObservation,
    queryObservations: mocks.queryObservations,
    getSessionObservations: mocks.getSessionObservations,
    observationExists: mocks.observationExists,
    querySession: mocks.querySession,
    queryByPlan: mocks.queryByPlan,
    getObservationsBySession: mocks.getObservationsBySession,
    getObservationsByTimeRange: mocks.getObservationsByTimeRange,
    getObservationsAsRefs: mocks.getObservationsAsRefs,
    isAvailable: mocks.isAvailable,
    countObservations: mocks.countObservations,
    pruneObservations: mocks.pruneObservations,

    // Test utilities
    _store: store,
    _reset: () => {
      store.clear();
      for (const obs of initialObservations) {
        store.set(obs.id, obs);
      }
      Object.values(mocks).forEach((mock) => mock.mockClear());
    },
    _getAll: () => Array.from(store.values()),
    _mocks: mocks,
  };
}

// =============================================================================
// Pre-built Scenarios
// =============================================================================

/**
 * Create a mock Kindling port with sample observations
 */
export function mockKindlingWithObservations(): MockKindlingPort {
  const sessionId = createSessionId(uuidv4());
  const baseTimestamp = new Date('2024-01-15T10:00:00.000Z');

  const observations: Observation[] = [
    {
      id: createObservationId(uuidv4()),
      session_id: sessionId,
      kind: 'gate_evaluated',
      timestamp: new Date(baseTimestamp.getTime()).toISOString() as Timestamp,
      summary: 'Architecture gate passed',
      data: { gate: 'architecture', result: 'pass', score: 0.85 },
      tags: ['gate', 'architecture'],
    },
    {
      id: createObservationId(uuidv4()),
      session_id: sessionId,
      kind: 'action_executed',
      timestamp: new Date(baseTimestamp.getTime() + 60000).toISOString() as Timestamp,
      summary: 'Created new component',
      data: { action: 'create_component', file: 'src/Button.tsx' },
      tags: ['action', 'component'],
    },
    {
      id: createObservationId(uuidv4()),
      session_id: sessionId,
      kind: 'gate_evaluated',
      timestamp: new Date(baseTimestamp.getTime() + 120000).toISOString() as Timestamp,
      summary: 'Coverage gate failed',
      data: { gate: 'coverage', result: 'fail', score: 0.45 },
      tags: ['gate', 'coverage'],
    },
    {
      id: createObservationId(uuidv4()),
      session_id: sessionId,
      kind: 'action_failed',
      timestamp: new Date(baseTimestamp.getTime() + 180000).toISOString() as Timestamp,
      summary: 'Test run failed',
      data: { action: 'run_tests', error: 'Test timeout', exitCode: 1 },
      tags: ['action', 'test', 'failure'],
    },
  ];

  return createMockKindlingPort({
    initialObservations: observations,
    defaultSessionId: sessionId,
  });
}

/**
 * Create an empty mock Kindling port
 */
export function mockKindlingEmpty(): MockKindlingPort {
  return createMockKindlingPort();
}

/**
 * Create a mock Kindling port with observations from multiple sessions
 */
export function mockKindlingMultipleSessions(): MockKindlingPort {
  const session1 = createSessionId(uuidv4());
  const session2 = createSessionId(uuidv4());
  const baseTimestamp = new Date('2024-01-15T10:00:00.000Z');

  const observations: Observation[] = [
    {
      id: createObservationId(uuidv4()),
      session_id: session1,
      kind: 'plan_started',
      timestamp: new Date(baseTimestamp.getTime()).toISOString() as Timestamp,
      summary: 'Plan execution started',
      data: { plan: 'feature-implementation' },
      tags: ['plan'],
    },
    {
      id: createObservationId(uuidv4()),
      session_id: session1,
      kind: 'plan_completed',
      timestamp: new Date(baseTimestamp.getTime() + 3600000).toISOString() as Timestamp,
      summary: 'Plan execution completed',
      data: { plan: 'feature-implementation', success: true },
      tags: ['plan'],
    },
    {
      id: createObservationId(uuidv4()),
      session_id: session2,
      kind: 'error_recorded',
      timestamp: new Date(baseTimestamp.getTime() + 7200000).toISOString() as Timestamp,
      summary: 'Runtime error in production',
      data: { error: 'TypeError', message: 'Cannot read property of undefined' },
      tags: ['error', 'production'],
    },
  ];

  return createMockKindlingPort({
    initialObservations: observations,
    defaultSessionId: session1,
  });
}
