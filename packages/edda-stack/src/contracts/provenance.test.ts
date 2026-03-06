import { describe, expect, it } from 'vitest';
import {
  AttributionSchema,
  EmberRefSchema,
  KindlingRefsSchema,
  KindlingRefSchema,
  PromotionProvenanceSchema,
  ProvenanceChainSchema,
  ProvenanceSummarySchema,
  createKindlingRef,
  mergeProvenanceChains,
  summariseProvenance,
  validateProvenanceIntegrity,
  type ProvenanceChain,
} from './provenance.js';
import type {
  ActionId,
  ObservationId,
  PlanId,
  ProposalId,
  SessionId,
  GateId,
} from './identifiers.js';
import type { Timestamp } from './temporal.js';

describe('Provenance schemas (EDDA-003)', () => {
  const observationIdA = '550e8400-e29b-41d4-a716-446655440000';
  const observationIdB = '550e8400-e29b-41d4-a716-446655440001';
  const sessionIdA = '550e8400-e29b-41d4-a716-446655440010';
  const sessionIdB = '550e8400-e29b-41d4-a716-446655440011';
  const proposalId = '550e8400-e29b-41d4-a716-446655440020' as ProposalId;
  const planId = '550e8400-e29b-41d4-a716-446655440030' as PlanId;
  const gateId = '550e8400-e29b-41d4-a716-446655440040' as GateId;
  const actionId = '550e8400-e29b-41d4-a716-446655440050' as ActionId;
  const timestampA = '2026-03-01T10:00:00.000Z';
  const timestampB = '2026-03-01T10:05:00.000Z';

  describe('KindlingRefSchema', () => {
    it('accepts a valid Kindling reference', () => {
      const result = KindlingRefSchema.safeParse({
        observation_id: observationIdA,
        session_id: sessionIdA,
        kind: 'gate_evaluated',
        timestamp: timestampA,
      });

      expect(result.success).toBe(true);
    });

    it('rejects invalid observation and session IDs', () => {
      const result = KindlingRefSchema.safeParse({
        observation_id: 'invalid-id',
        session_id: 'invalid-session',
        kind: 'action_executed',
        timestamp: timestampA,
      });

      expect(result.success).toBe(false);
    });
  });

  describe('KindlingRefsSchema', () => {
    it('accepts one or more references', () => {
      const result = KindlingRefsSchema.safeParse([
        {
          observation_id: observationIdA,
          session_id: sessionIdA,
          kind: 'gate_evaluated',
          timestamp: timestampA,
        },
      ]);

      expect(result.success).toBe(true);
    });

    it('rejects an empty references array', () => {
      const result = KindlingRefsSchema.safeParse([]);

      expect(result.success).toBe(false);
    });
  });

  describe('EmberRefSchema', () => {
    it('accepts a valid Ember reference', () => {
      const result = EmberRefSchema.safeParse({
        proposal_id: proposalId,
        proposal_type: 'pattern',
        confidence: 0.75,
        created_at: timestampA,
      });

      expect(result.success).toBe(true);
    });

    it('rejects confidence outside the 0..1 range', () => {
      const result = EmberRefSchema.safeParse({
        proposal_id: proposalId,
        proposal_type: 'decision',
        confidence: 1.5,
        created_at: timestampA,
      });

      expect(result.success).toBe(false);
    });
  });

  describe('ProvenanceChainSchema', () => {
    it('accepts a full chain with optional relation fields', () => {
      const result = ProvenanceChainSchema.safeParse({
        ember_source: {
          proposal_id: proposalId,
          proposal_type: 'pattern',
          confidence: 0.68,
          created_at: timestampA,
        },
        kindling_sources: [
          {
            observation_id: observationIdA,
            session_id: sessionIdA,
            kind: 'gate_evaluated',
            timestamp: timestampA,
          },
        ],
        source_sessions: [sessionIdA],
        related_plans: [planId],
        related_gates: [gateId],
        related_actions: [actionId],
      });

      expect(result.success).toBe(true);
    });

    it('accepts a chain without optional relation fields', () => {
      const result = ProvenanceChainSchema.safeParse({
        kindling_sources: [
          {
            observation_id: observationIdA,
            session_id: sessionIdA,
            kind: 'action_executed',
            timestamp: timestampA,
          },
        ],
        source_sessions: [sessionIdA],
      });

      expect(result.success).toBe(true);
    });

    it('rejects empty kindling_sources', () => {
      const result = ProvenanceChainSchema.safeParse({
        kindling_sources: [],
        source_sessions: [sessionIdA],
      });

      expect(result.success).toBe(false);
    });
  });

  describe('AttributionSchema', () => {
    it('accepts attribution with an optional reason', () => {
      const result = AttributionSchema.safeParse({
        actor: 'joshua',
        timestamp: timestampA,
        method: 'cli_command',
        reason: 'Promoted after repeated evidence',
      });

      expect(result.success).toBe(true);
    });

    it('accepts attribution without a reason', () => {
      const result = AttributionSchema.safeParse({
        actor: 'system',
        timestamp: timestampA,
        method: 'automatic',
      });

      expect(result.success).toBe(true);
    });

    it('rejects unsupported attribution methods', () => {
      const result = AttributionSchema.safeParse({
        actor: 'joshua',
        timestamp: timestampA,
        method: 'script',
      });

      expect(result.success).toBe(false);
    });
  });

  describe('PromotionProvenanceSchema', () => {
    it('accepts valid promotion provenance', () => {
      const result = PromotionProvenanceSchema.safeParse({
        proposal_id: proposalId,
        ember_confidence: 0.82,
        attribution: {
          actor: 'joshua',
          timestamp: timestampA,
          method: 'manual_edit',
          reason: 'Matches confirmed project behaviour',
        },
        original_rationale: 'Repeated across multiple sessions and outcomes',
      });

      expect(result.success).toBe(true);
    });

    it('rejects invalid ember_confidence', () => {
      const result = PromotionProvenanceSchema.safeParse({
        proposal_id: proposalId,
        ember_confidence: -0.2,
        attribution: {
          actor: 'joshua',
          timestamp: timestampA,
          method: 'api_call',
        },
        original_rationale: 'Invalid confidence should fail',
      });

      expect(result.success).toBe(false);
    });
  });

  describe('ProvenanceSummarySchema', () => {
    it('accepts a valid provenance summary', () => {
      const result = ProvenanceSummarySchema.safeParse({
        observation_ids: [observationIdA],
        session_ids: [sessionIdA],
        proposal_id: proposalId,
        earliest_observation: timestampA,
        latest_observation: timestampB,
      });

      expect(result.success).toBe(true);
    });

    it('accepts a summary without optional proposal_id', () => {
      const result = ProvenanceSummarySchema.safeParse({
        observation_ids: [observationIdA],
        session_ids: [sessionIdA],
        earliest_observation: timestampA,
        latest_observation: timestampB,
      });

      expect(result.success).toBe(true);
    });

    it('rejects empty observation IDs', () => {
      const result = ProvenanceSummarySchema.safeParse({
        observation_ids: [],
        session_ids: [sessionIdA],
        earliest_observation: timestampA,
        latest_observation: timestampB,
      });

      expect(result.success).toBe(false);
    });
  });

  describe('createKindlingRef', () => {
    it('creates a valid reference object', () => {
      const ref = createKindlingRef(
        observationIdA as ObservationId,
        sessionIdA as SessionId,
        'gate_evaluated',
        timestampA as Timestamp
      );

      expect(ref).toEqual({
        observation_id: observationIdA,
        session_id: sessionIdA,
        kind: 'gate_evaluated',
        timestamp: timestampA,
      });
      expect(KindlingRefSchema.safeParse(ref).success).toBe(true);
    });
  });

  describe('summariseProvenance', () => {
    it('creates summary with earliest/latest timestamps and proposal ID', () => {
      const chain: ProvenanceChain = {
        ember_source: {
          proposal_id: proposalId,
          proposal_type: 'pattern',
          confidence: 0.7,
          created_at: timestampA,
        },
        kindling_sources: [
          {
            observation_id: observationIdA as ObservationId,
            session_id: sessionIdA as SessionId,
            kind: 'action_executed',
            timestamp: timestampB as Timestamp,
          },
          {
            observation_id: observationIdB as ObservationId,
            session_id: sessionIdB as SessionId,
            kind: 'gate_evaluated',
            timestamp: timestampA as Timestamp,
          },
        ],
        source_sessions: [sessionIdA as SessionId, sessionIdB as SessionId],
      };

      const summary = summariseProvenance(chain);

      expect(summary.observation_ids).toEqual([observationIdA, observationIdB]);
      expect(summary.session_ids).toEqual([sessionIdA, sessionIdB]);
      expect(summary.proposal_id).toBe(proposalId);
      expect(summary.earliest_observation).toBe(timestampA);
      expect(summary.latest_observation).toBe(timestampB);
    });

    it('omits proposal_id when no ember source exists', () => {
      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: observationIdA as ObservationId,
            session_id: sessionIdA as SessionId,
            kind: 'gate_evaluated',
            timestamp: timestampA as Timestamp,
          },
        ],
        source_sessions: [sessionIdA as SessionId],
      };

      const summary = summariseProvenance(chain);

      expect(summary.proposal_id).toBeUndefined();
    });
  });

  describe('mergeProvenanceChains', () => {
    it('merges multiple chains and deduplicates by observation_id', () => {
      const chains: ProvenanceChain[] = [
        {
          kindling_sources: [
            {
              observation_id: observationIdA as ObservationId,
              session_id: sessionIdA as SessionId,
              kind: 'gate_evaluated',
              timestamp: timestampA as Timestamp,
            },
            {
              observation_id: observationIdB as ObservationId,
              session_id: sessionIdB as SessionId,
              kind: 'action_executed',
              timestamp: timestampB as Timestamp,
            },
          ],
          source_sessions: [sessionIdA as SessionId, sessionIdB as SessionId],
          related_plans: [planId],
          related_gates: [gateId],
        },
        {
          kindling_sources: [
            {
              observation_id: observationIdA as ObservationId,
              session_id: sessionIdA as SessionId,
              kind: 'gate_evaluated',
              timestamp: timestampA as Timestamp,
            },
          ],
          source_sessions: [sessionIdA as SessionId],
          related_plans: [planId],
          related_actions: [actionId],
        },
      ];

      const merged = mergeProvenanceChains(chains);

      expect(merged.kindling_sources).toHaveLength(2);
      expect(merged.kindling_sources.map((ref) => ref.observation_id)).toEqual([
        observationIdA,
        observationIdB,
      ]);
      expect(merged.source_sessions).toEqual([sessionIdA, sessionIdB]);
      expect(merged.related_plans).toEqual([planId]);
      expect(merged.related_gates).toEqual([gateId]);
      expect(merged.related_actions).toEqual([actionId]);
      expect(merged.ember_source).toBeUndefined();
    });

    it('returns undefined relation fields when merged values are empty', () => {
      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: observationIdA as ObservationId,
            session_id: sessionIdA as SessionId,
            kind: 'action_executed',
            timestamp: timestampA as Timestamp,
          },
        ],
        source_sessions: [sessionIdA as SessionId],
      };

      const merged = mergeProvenanceChains([chain]);

      expect(merged.related_plans).toBeUndefined();
      expect(merged.related_gates).toBeUndefined();
      expect(merged.related_actions).toBeUndefined();
    });
  });

  describe('validateProvenanceIntegrity', () => {
    it('returns valid for a consistent chain', () => {
      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: observationIdA as ObservationId,
            session_id: sessionIdA as SessionId,
            kind: 'gate_evaluated',
            timestamp: timestampA as Timestamp,
          },
        ],
        source_sessions: [sessionIdA as SessionId],
      };

      const result = validateProvenanceIntegrity(chain);

      expect(result.valid).toBe(true);
      expect(result.issues).toEqual([]);
    });

    it('reports issue for empty kindling sources', () => {
      const chain: ProvenanceChain = {
        kindling_sources: [],
        source_sessions: [sessionIdA as SessionId],
      };

      const result = validateProvenanceIntegrity(chain);

      expect(result.valid).toBe(false);
      expect(result.issues).toContain('Provenance chain has no Kindling sources');
    });

    it('reports issue when source_sessions misses referenced session IDs', () => {
      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: observationIdA as ObservationId,
            session_id: sessionIdA as SessionId,
            kind: 'action_executed',
            timestamp: timestampA as Timestamp,
          },
        ],
        source_sessions: [sessionIdB as SessionId],
      };

      const result = validateProvenanceIntegrity(chain);

      expect(result.valid).toBe(false);
      expect(result.issues).toContain(
        `Observation ${observationIdA} references session not in source_sessions`
      );
    });
  });
});
