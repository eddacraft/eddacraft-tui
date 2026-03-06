import { describe, expect, it, vi } from 'vitest';
import { createObservationId, createSessionId } from '../contracts/identifiers.js';
import { parseTimestamp } from '../contracts/temporal.js';
import type {
  CreateObservationInput,
  IKindlingPort,
  Observation,
  ObservationQuery,
  ObservationQueryResult,
  PlanQueryOptions,
  PlanQueryResult,
  SessionQueryOptions,
  SessionQueryResult,
} from '../contracts/ports/kindling.port.js';
import { AggregatorService } from './aggregator-service.js';

const SESSION_A = createSessionId('11111111-1111-4111-8111-111111111111');
const SESSION_B = createSessionId('22222222-2222-4222-8222-222222222222');

function buildObservation(
  id: string,
  sessionId: typeof SESSION_A,
  kind: Observation['kind'],
  timestamp: string,
  summary: string,
  data: Record<string, unknown>
): Observation {
  return {
    id: createObservationId(id),
    session_id: sessionId,
    kind,
    timestamp: parseTimestamp(timestamp),
    summary,
    data,
  };
}

const CANNED_OBSERVATIONS: Observation[] = [
  buildObservation(
    'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1',
    SESSION_A,
    'error_recorded',
    '2026-01-01T10:00:00.000Z',
    'Network timeout',
    { severity: 'medium' }
  ),
  buildObservation(
    'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2',
    SESSION_A,
    'action_failed',
    '2026-01-01T10:01:00.000Z',
    'Network timeout',
    { severity: 'high' }
  ),
  buildObservation(
    'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa3',
    SESSION_A,
    'action_executed',
    '2026-01-01T10:03:00.000Z',
    'Retry succeeded',
    { severity: 'low' }
  ),
  buildObservation(
    'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa4',
    SESSION_A,
    'error_recorded',
    '2026-01-01T10:20:00.000Z',
    'Network timeout',
    { severity: 'high' }
  ),
  buildObservation(
    'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1',
    SESSION_B,
    'action_executed',
    '2026-01-01T11:00:00.000Z',
    'Build succeeded',
    { severity: 'low' }
  ),
];

function createKindlingMock(observations: Observation[]): IKindlingPort {
  const queryObservations = vi.fn(
    async (query: ObservationQuery): Promise<ObservationQueryResult> => {
      const filtered = observations.filter((item) => {
        if (query.session_id && item.session_id !== query.session_id) {
          return false;
        }
        if (query.kinds && !query.kinds.includes(item.kind)) {
          return false;
        }
        return true;
      });

      return {
        observations: filtered,
        total: filtered.length,
        has_more: false,
      };
    }
  );

  const getSessionObservations = vi.fn(async (sessionId: typeof SESSION_A) => {
    return observations.filter((item) => item.session_id === sessionId);
  });

  const notImplemented = (): never => {
    throw new Error('not implemented in test');
  };

  return {
    createObservation: async (_input: CreateObservationInput) => notImplemented(),
    createObservationBatch: async (_inputs: CreateObservationInput[]) => notImplemented(),
    getObservation: async (_id) => notImplemented(),
    queryObservations,
    getSessionObservations,
    observationExists: async (_id) => notImplemented(),
    querySession: async (_sessionId, _options?: SessionQueryOptions): Promise<SessionQueryResult> =>
      notImplemented(),
    queryByPlan: async (_planId, _options?: PlanQueryOptions): Promise<PlanQueryResult> =>
      notImplemented(),
    getObservationsBySession: async (_sessionId) => notImplemented(),
    getObservationsByTimeRange: async (_range) => notImplemented(),
    getObservationsAsRefs: async (_ids) => notImplemented(),
    isAvailable: async () => true,
    countObservations: async (_sessionId?) => observations.length,
    pruneObservations: async (_olderThan) => 0,
  };
}

describe('AggregatorService', () => {
  it('groupBySession groups observations from one session', async () => {
    const service = new AggregatorService(createKindlingMock(CANNED_OBSERVATIONS));

    const groups = await service.groupBySession(SESSION_A);

    expect(groups).toHaveLength(1);
    expect(groups[0].grouping_type).toBe('session');
    expect(groups[0].count).toBe(4);
    expect(groups[0].session_ids).toEqual([SESSION_A]);
  });

  it('groupByTemporalProximity creates temporal windows', async () => {
    const service = new AggregatorService(createKindlingMock(CANNED_OBSERVATIONS));

    const groups = await service.groupByTemporalProximity(
      CANNED_OBSERVATIONS.slice(0, 4),
      5 * 60 * 1000
    );

    expect(groups).toHaveLength(2);
    expect(groups[0].count).toBe(3);
    expect(groups[1].count).toBe(1);
  });

  it('groupByKind categorises observations by kind', async () => {
    const service = new AggregatorService(createKindlingMock(CANNED_OBSERVATIONS));

    const groups = await service.groupByKind(CANNED_OBSERVATIONS.slice(0, 4));

    const kinds = groups.map((group) => group.signals.find((signal) => signal.startsWith('kind_')));
    expect(kinds).toContain('kind_error_recorded');
    expect(kinds).toContain('kind_action_failed');
    expect(kinds).toContain('kind_action_executed');
  });

  it('detectRepetitions finds repeated patterns using threshold', async () => {
    const service = new AggregatorService(createKindlingMock(CANNED_OBSERVATIONS));

    const groups = await service.detectRepetitions(CANNED_OBSERVATIONS.slice(0, 4), 2);

    expect(groups).toHaveLength(1);
    expect(groups[0].count).toBe(2);
    expect(groups[0].signals).toContain('repetition_detected');
    expect(groups[0].suggested_type).toBe('pattern');
  });

  it('aggregate runs pipeline and returns sorted merged groups', async () => {
    const service = new AggregatorService(createKindlingMock(CANNED_OBSERVATIONS));

    const groups = await service.aggregate(SESSION_A);

    expect(groups.length).toBeGreaterThan(0);
    expect(groups[0].count).toBe(4);
    expect(groups[0].observation_ids).toHaveLength(4);
    expect(groups.every((group) => group.session_ids.includes(SESSION_A))).toBe(true);
  });
});
