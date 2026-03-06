import { describe, expect, it, vi } from 'vitest';
import { createMockEmberPort } from '../testing/mocks/ember.mock.js';
import { createMockKindlingPort } from '../testing/mocks/kindling.mock.js';
import type {
  CreateProposalInput,
  ProposalQuery,
  ProposalQueryResult,
} from '../contracts/ember-proposal.js';
import type { EmberStats } from '../contracts/ports/ember.port.js';
import type { Observation, ObservationKind } from '../contracts/ports/kindling.port.js';
import type { IStackEventBus, StackEvent } from '../contracts/events.js';
import { createObservationId, createSessionId } from '../contracts/identifiers.js';
import type { Timestamp } from '../contracts/temporal.js';
import { now } from '../contracts/temporal.js';
import { CandidateService } from './candidate-service.js';
import { DEFAULT_PRUNE_DAYS } from './decay-service.js';

function createInput(ttlDays = 30): CreateProposalInput {
  return {
    type: 'pattern',
    summary: 'Repeated observation pattern',
    rationale: 'Pattern appears multiple times in one session',
    confidence: 0.72,
    ttl_days: ttlDays,
    provenance: {
      observation_ids: ['550e8400-e29b-41d4-a716-446655440001'],
      session_ids: ['550e8400-e29b-41d4-a716-446655440002'],
      earliest_observation: '2026-01-10T10:00:00.000Z',
      latest_observation: '2026-01-10T10:10:00.000Z',
    },
  };
}

function createObservation(
  id: string,
  sessionId: string,
  kind: ObservationKind,
  timestamp: string,
  summary = 'Observation summary'
): Observation {
  return {
    id: createObservationId(id),
    session_id: createSessionId(sessionId),
    kind,
    timestamp: timestamp as Timestamp,
    summary,
    data: { source: 'test' },
  };
}

describe('CandidateService', () => {
  it('createProposal delegates to store and returns proposal', async () => {
    const store = createMockEmberPort();
    const publish = vi.fn<(_event: StackEvent) => Promise<void>>().mockResolvedValue(undefined);
    const eventBus = {
      publish,
      subscribe: vi.fn(),
      subscribeAll: vi.fn(),
    } as unknown as IStackEventBus;
    const service = new CandidateService({ store, eventBus });

    const proposal = await service.createProposal(createInput(14));

    expect(store._mocks.createProposal).toHaveBeenCalledTimes(1);
    expect(proposal.ttl_days).toBe(14);
    expect(publish).toHaveBeenCalledTimes(1);
  });

  it('clamps ttl_days below min and above max bounds', async () => {
    const store = createMockEmberPort();
    const service = new CandidateService({ store });

    const lowTtlProposal = await service.createProposal(createInput(1));
    const highTtlProposal = await service.createProposal(createInput(500));

    expect(lowTtlProposal.ttl_days).toBe(7);
    expect(highTtlProposal.ttl_days).toBe(90);
    expect(store._mocks.createProposal.mock.calls[0][0].ttl_days).toBe(7);
    expect(store._mocks.createProposal.mock.calls[1][0].ttl_days).toBe(90);
  });

  it('enforces max_candidates limit', async () => {
    const store = createMockEmberPort();
    store._mocks.countProposals.mockResolvedValue(1);
    const service = new CandidateService({
      store,
      config: {
        evaluation: {
          min_confidence: 0.3,
          repetition_threshold: 3,
          escalation_window_hours: 24,
        },
        decay: {
          default_ttl_days: 30,
          min_ttl_days: 7,
          max_ttl_days: 90,
        },
        limits: {
          max_candidates: 1,
        },
      },
    });

    await expect(service.createProposal(createInput(14))).rejects.toThrow(
      'Maximum candidate limit reached'
    );
    expect(store._mocks.createProposal).not.toHaveBeenCalled();
  });

  it('delegates getProposal, queryProposals, and getActiveProposals', async () => {
    const store = createMockEmberPort();
    const service = new CandidateService({ store });
    const existing = await store.createProposal(createInput(21));
    const query: ProposalQuery = {
      include_expired: false,
      limit: 10,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    };

    const fetched = await service.getProposal(existing.id);
    const queried = await service.queryProposals(query);
    const active = await service.getActiveProposals();

    expect(store._mocks.getProposal).toHaveBeenCalledWith(existing.id);
    expect(store._mocks.queryProposals).toHaveBeenCalledWith(query);
    expect(store._mocks.getActiveProposals).toHaveBeenCalledTimes(1);
    expect(fetched?.id).toBe(existing.id);
    expect(queried.proposals.length).toBeGreaterThanOrEqual(1);
    expect(active.length).toBeGreaterThanOrEqual(1);
  });

  it('promoteProposal delegates to markPromoted', async () => {
    const store = createMockEmberPort();
    const service = new CandidateService({ store });

    await service.promoteProposal(
      '550e8400-e29b-41d4-a716-446655440010',
      '550e8400-e29b-41d4-a716-446655440011',
      'joshua'
    );

    expect(store._mocks.markPromoted).toHaveBeenCalledWith(
      '550e8400-e29b-41d4-a716-446655440010',
      '550e8400-e29b-41d4-a716-446655440011',
      'joshua'
    );
  });

  it('dismissProposal delegates to markDismissed', async () => {
    const store = createMockEmberPort();
    const service = new CandidateService({ store });

    await service.dismissProposal('550e8400-e29b-41d4-a716-446655440012', 'Not relevant', 'joshua');

    expect(store._mocks.markDismissed).toHaveBeenCalledWith(
      '550e8400-e29b-41d4-a716-446655440012',
      'Not relevant',
      'joshua'
    );
  });

  it('runDecayCycle processes expired and prunes resolved proposals', async () => {
    const store = createMockEmberPort();
    store._mocks.processExpiredProposals.mockResolvedValue(3);
    store._mocks.pruneProposals.mockResolvedValue(5);
    const service = new CandidateService({ store });
    const before = Date.now();

    const result = await service.runDecayCycle();

    expect(store._mocks.processExpiredProposals).toHaveBeenCalledTimes(1);
    expect(store._mocks.pruneProposals).toHaveBeenCalledTimes(1);
    expect(result).toEqual({ expired: 3, pruned: 5 });

    const pruneArg = store._mocks.pruneProposals.mock.calls[0][0] as string;
    const prunedAt = new Date(pruneArg).getTime();
    const pruneDaysMs = DEFAULT_PRUNE_DAYS * 24 * 60 * 60 * 1000;
    expect(Math.abs(before - pruneDaysMs - prunedAt)).toBeLessThan(30_000);
  });

  it('delegates getStats', async () => {
    const store = createMockEmberPort();
    const stats: EmberStats = {
      total_proposals: 42,
      by_status: [],
      by_type: [],
      expiring_soon: 3,
    };
    store._mocks.getStats.mockResolvedValue(stats);
    const service = new CandidateService({ store });

    const result = await service.getStats();

    expect(store._mocks.getStats).toHaveBeenCalledTimes(1);
    expect(result).toEqual(stats);
  });

  it('delegates isAvailable', async () => {
    const store = createMockEmberPort();
    store._mocks.isAvailable.mockResolvedValue(true);
    const service = new CandidateService({ store });

    const available = await service.isAvailable();

    expect(store._mocks.isAvailable).toHaveBeenCalledTimes(1);
    expect(available).toBe(true);
  });

  it('processSession returns empty array when kindlingPort is missing', async () => {
    const store = createMockEmberPort();
    const service = new CandidateService({ store });

    const proposals = await service.processSession('550e8400-e29b-41d4-a716-446655440099');

    expect(proposals).toEqual([]);
  });

  it('processSession aggregates by kind, evaluates built-in groups, and creates proposals', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440101';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440111',
        sessionId,
        'action_failed',
        '2026-01-11T09:00:00.000Z',
        'Action failed once'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440112',
        sessionId,
        'action_failed',
        '2026-01-11T09:01:00.000Z',
        'Action failed twice'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440113',
        sessionId,
        'gate_evaluated',
        '2026-01-11T09:02:00.000Z',
        'Gate evaluated'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    const service = new CandidateService({ store, kindlingPort });

    const proposals = await service.processSession(sessionId);

    expect(kindlingPort._mocks.getObservationsBySession).toHaveBeenCalledWith(
      createSessionId(sessionId)
    );
    expect(store._mocks.createProposal).toHaveBeenCalledTimes(2);
    expect(proposals).toHaveLength(2);
    expect(proposals.map((proposal) => proposal.type).sort()).toEqual(['pattern', 'warning']);
    expect(proposals.find((proposal) => proposal.type === 'warning')?.metadata).toMatchObject({
      observation_kind: 'action_failed',
      occurrence_count: 2,
    });
  });

  it('processSession uses custom aggregator when provided', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440121';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440131',
        sessionId,
        'error_recorded',
        '2026-01-12T10:00:00.000Z'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440132',
        sessionId,
        'action_executed',
        '2026-01-12T10:01:00.000Z'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    let aggregateCalls = 0;
    let receivedSessionId = '';
    let receivedLength = 0;
    const aggregator = {
      async aggregateSession(
        aggregatedSessionId: ReturnType<typeof createSessionId>,
        obs: Observation[]
      ) {
        aggregateCalls++;
        receivedSessionId = aggregatedSessionId;
        receivedLength = obs.length;
        return [
          {
            key: 'error_recorded' as const,
            observations: [obs[0]],
          },
        ];
      },
    };
    const service = new CandidateService({ store, kindlingPort, aggregator });

    const proposals = await service.processSession(sessionId);

    expect(aggregateCalls).toBe(1);
    expect(receivedSessionId).toBe(sessionId);
    expect(receivedLength).toBe(2);
    expect(proposals).toHaveLength(1);
    expect(proposals[0]?.type).toBe('warning');
  });

  it('processSession uses custom evaluator when provided', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440141';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440151',
        sessionId,
        'gate_evaluated',
        '2026-01-13T08:00:00.000Z'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    let evaluateCalls = 0;
    const evaluator = {
      async evaluateGroup(group: { key: ObservationKind; observations: Observation[] }) {
        evaluateCalls++;
        return {
          should_propose: true,
          confidence: 0.95,
          type: 'anomaly' as const,
          summary: `Custom evaluation for ${group.key}`,
          rationale: 'Custom evaluator recognised this group',
          metadata: { evaluator: 'custom' },
          ttl_days: 12,
        };
      },
    };
    const service = new CandidateService({ store, kindlingPort, evaluator });

    const proposals = await service.processSession(sessionId);

    expect(evaluateCalls).toBe(1);
    expect(proposals).toHaveLength(1);
    expect(proposals[0]).toMatchObject({
      type: 'anomaly',
      summary: 'Custom evaluation for gate_evaluated',
      rationale: 'Custom evaluator recognised this group',
      ttl_days: 12,
      metadata: { evaluator: 'custom' },
    });
  });

  it('processSession skips a group when evaluator returns null', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440161';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440171',
        sessionId,
        'gate_evaluated',
        now(),
        'Single gate event'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    const evaluator = {
      async evaluateGroup() {
        return null;
      },
    };
    const service = new CandidateService({ store, kindlingPort, evaluator });

    const proposals = await service.processSession(sessionId);

    expect(proposals).toEqual([]);
    expect(store._mocks.createProposal).not.toHaveBeenCalled();
  });

  it('processSession skips a group when evaluator says should_propose is false', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440181';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440191',
        sessionId,
        'action_failed',
        now(),
        'Failed action'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    const evaluator = {
      async evaluateGroup() {
        return {
          should_propose: false,
          confidence: 0.99,
          type: 'warning' as const,
          summary: 'Should be skipped',
          rationale: 'Custom logic filtered this out',
        };
      },
    };
    const service = new CandidateService({ store, kindlingPort, evaluator });

    const proposals = await service.processSession(sessionId);

    expect(proposals).toEqual([]);
    expect(store._mocks.createProposal).not.toHaveBeenCalled();
  });

  it('processSession skips proposals below minimum confidence after clamping', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440201';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440211',
        sessionId,
        'error_recorded',
        '2026-01-15T10:00:00.000Z'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    const evaluator = {
      async evaluateGroup() {
        return {
          should_propose: true,
          confidence: -5,
          type: 'warning' as const,
          summary: 'Too low confidence',
          rationale: 'Will be clamped to zero',
        };
      },
    };
    const service = new CandidateService({
      store,
      kindlingPort,
      evaluator,
      config: {
        evaluation: {
          min_confidence: 0.2,
          repetition_threshold: 3,
          escalation_window_hours: 24,
        },
        decay: {
          default_ttl_days: 30,
          min_ttl_days: 7,
          max_ttl_days: 90,
        },
        limits: {
          max_candidates: 1000,
        },
      },
    });

    const proposals = await service.processSession(sessionId);

    expect(proposals).toEqual([]);
    expect(store._mocks.createProposal).not.toHaveBeenCalled();
  });

  it('processSession returns empty array when Kindling returns no observations', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440221';
    const kindlingPort = createMockKindlingPort();
    const store = createMockEmberPort();
    const service = new CandidateService({ store, kindlingPort });

    const proposals = await service.processSession(sessionId);

    expect(proposals).toEqual([]);
    expect(store._mocks.createProposal).not.toHaveBeenCalled();
  });

  it('processSession skips groups with no observations', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440231';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440241',
        sessionId,
        'gate_evaluated',
        '2026-01-16T09:00:00.000Z'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    let evaluateCalls = 0;
    const aggregator = {
      async aggregateSession() {
        return [
          { key: 'plan_started' as const, observations: [] },
          { key: 'gate_evaluated' as const, observations },
        ];
      },
    };
    const evaluator = {
      async evaluateGroup(group: { key: ObservationKind; observations: Observation[] }) {
        evaluateCalls++;
        return {
          should_propose: true,
          confidence: 0.9,
          type: group.key === 'gate_evaluated' ? ('pattern' as const) : ('decision' as const),
          summary: 'Valid group only',
          rationale: 'Groups without observations are ignored',
        };
      },
    };
    const service = new CandidateService({ store, kindlingPort, aggregator, evaluator });

    const proposals = await service.processSession(sessionId);

    expect(evaluateCalls).toBe(1);
    expect(proposals).toHaveLength(1);
    expect(proposals[0]?.type).toBe('pattern');
  });

  it('processSession maps observation kinds to expected proposal types', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440251';
    const observations: Observation[] = [
      createObservation(
        '550e8400-e29b-41d4-a716-446655440261',
        sessionId,
        'error_recorded',
        '2026-01-17T11:00:00.000Z'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440262',
        sessionId,
        'action_failed',
        '2026-01-17T11:01:00.000Z'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440263',
        sessionId,
        'constraint_applied',
        '2026-01-17T11:02:00.000Z'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440264',
        sessionId,
        'plan_completed',
        '2026-01-17T11:03:00.000Z'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440265',
        sessionId,
        'plan_started',
        '2026-01-17T11:04:00.000Z'
      ),
      createObservation(
        '550e8400-e29b-41d4-a716-446655440266',
        sessionId,
        'gate_evaluated',
        '2026-01-17T11:05:00.000Z'
      ),
    ];
    const kindlingPort = createMockKindlingPort({ initialObservations: observations });
    const store = createMockEmberPort();
    const service = new CandidateService({ store, kindlingPort });

    const proposals = await service.processSession(sessionId);

    const byKind = new Map(
      proposals.map((proposal) => [proposal.metadata?.observation_kind as string, proposal.type])
    );
    expect(byKind.get('error_recorded')).toBe('warning');
    expect(byKind.get('action_failed')).toBe('warning');
    expect(byKind.get('constraint_applied')).toBe('constraint');
    expect(byKind.get('plan_completed')).toBe('lesson');
    expect(byKind.get('plan_started')).toBe('decision');
    expect(byKind.get('gate_evaluated')).toBe('pattern');
  });

  it('processSession builds provenance with IDs and earliest/latest timestamps', async () => {
    const sessionId = '550e8400-e29b-41d4-a716-446655440271';
    const observationA = createObservation(
      '550e8400-e29b-41d4-a716-446655440281',
      sessionId,
      'gate_evaluated',
      '2026-01-18T12:05:00.000Z'
    );
    const observationB = createObservation(
      '550e8400-e29b-41d4-a716-446655440282',
      sessionId,
      'gate_evaluated',
      '2026-01-18T12:01:00.000Z'
    );
    const observationC = createObservation(
      '550e8400-e29b-41d4-a716-446655440283',
      sessionId,
      'gate_evaluated',
      '2026-01-18T12:09:00.000Z'
    );
    const kindlingPort = createMockKindlingPort({
      initialObservations: [observationA, observationB, observationC],
    });
    const store = createMockEmberPort();
    const service = new CandidateService({ store, kindlingPort });

    const proposals = await service.processSession(sessionId);

    expect(proposals).toHaveLength(1);
    expect(proposals[0]?.provenance).toEqual({
      observation_ids: [observationA.id, observationB.id, observationC.id],
      session_ids: [sessionId],
      earliest_observation: '2026-01-18T12:01:00.000Z',
      latest_observation: '2026-01-18T12:09:00.000Z',
    });
  });

  it('updateProposal delegates to store and returns updated proposal', async () => {
    const store = createMockEmberPort();
    const service = new CandidateService({ store });
    const created = await store.createProposal(createInput(10));

    const updated = await service.updateProposal(created.id, { summary: 'Updated summary' });

    expect(store._mocks.updateProposal).toHaveBeenCalledWith(created.id, {
      summary: 'Updated summary',
    });
    expect(updated?.summary).toBe('Updated summary');
  });

  it('queryProposals returns store response unchanged', async () => {
    const store = createMockEmberPort();
    const expected: ProposalQueryResult = {
      proposals: [],
      total: 0,
      limit: 5,
      offset: 0,
      has_more: false,
    };
    store._mocks.queryProposals.mockResolvedValue(expected);
    const service = new CandidateService({ store });

    const result = await service.queryProposals({
      include_expired: false,
      limit: 5,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    expect(result).toEqual(expected);
  });
});
