/**
 * Type Mappings Tests (STACK-005)
 *
 * Tests for proposal to memory type mapping utilities.
 */

import { describe, it, expect } from 'vitest';
import {
  PROPOSAL_TO_MEMORY_TYPE_MAPPING,
  mapProposalToMemoryType,
  mapProposalConfidence,
  createPromotionInput,
  expandProvenanceSummary,
} from './type-mappings.js';
import type { CandidateProposal, ProposalType } from './ember-proposal.js';
import type { MemoryType } from './edda-memory.js';
import type { ProvenanceSummary } from './provenance.js';
import type { Timestamp } from './temporal.js';
import type { ProposalId } from './identifiers.js';
import type { EmberConfidence } from './confidence.js';

// =============================================================================
// Test Fixtures
// =============================================================================

const validUuid = '550e8400-e29b-41d4-a716-446655440000';
const validUuid2 = '550e8400-e29b-41d4-a716-446655440001';
const validTimestamp = '2024-01-15T14:30:00.000Z' as Timestamp;
const laterTimestamp = '2024-01-16T14:30:00.000Z' as Timestamp;

const createTestProposal = (overrides: Partial<CandidateProposal> = {}): CandidateProposal => ({
  id: validUuid as ProposalId,
  type: 'pattern',
  status: 'active',
  summary: 'Test pattern observed',
  rationale: 'This pattern appears multiple times across sessions',
  confidence: 0.75 as EmberConfidence,
  signals: [],
  provenance: {
    observation_ids: [validUuid],
    session_ids: [validUuid],
    earliest_observation: validTimestamp,
    latest_observation: validTimestamp,
  },
  created_at: validTimestamp,
  expires_at: '2024-02-14T14:30:00.000Z' as Timestamp,
  ttl_days: 30,
  ...overrides,
});

// =============================================================================
// PROPOSAL_TO_MEMORY_TYPE_MAPPING Tests
// =============================================================================

describe('PROPOSAL_TO_MEMORY_TYPE_MAPPING', () => {
  it('maps decision to decision', () => {
    expect(PROPOSAL_TO_MEMORY_TYPE_MAPPING.decision).toBe('decision');
  });

  it('maps pattern to pattern', () => {
    expect(PROPOSAL_TO_MEMORY_TYPE_MAPPING.pattern).toBe('pattern');
  });

  it('maps warning to warning', () => {
    expect(PROPOSAL_TO_MEMORY_TYPE_MAPPING.warning).toBe('warning');
  });

  it('maps lesson to lesson', () => {
    expect(PROPOSAL_TO_MEMORY_TYPE_MAPPING.lesson).toBe('lesson');
  });

  it('maps constraint to constraint', () => {
    expect(PROPOSAL_TO_MEMORY_TYPE_MAPPING.constraint).toBe('constraint');
  });

  it('maps anomaly to null (requires human choice)', () => {
    expect(PROPOSAL_TO_MEMORY_TYPE_MAPPING.anomaly).toBe(null);
  });

  it('contains exactly 6 mappings', () => {
    expect(Object.keys(PROPOSAL_TO_MEMORY_TYPE_MAPPING)).toHaveLength(6);
  });
});

// =============================================================================
// mapProposalToMemoryType Tests
// =============================================================================

describe('mapProposalToMemoryType', () => {
  const directMappings: Array<[ProposalType, MemoryType]> = [
    ['decision', 'decision'],
    ['pattern', 'pattern'],
    ['warning', 'warning'],
    ['lesson', 'lesson'],
    ['constraint', 'constraint'],
  ];

  it.each(directMappings)('maps %s to %s', (proposalType, expectedMemoryType) => {
    expect(mapProposalToMemoryType(proposalType)).toBe(expectedMemoryType);
  });

  it('returns null for anomaly', () => {
    expect(mapProposalToMemoryType('anomaly')).toBe(null);
  });
});

// =============================================================================
// mapProposalConfidence Tests
// =============================================================================

describe('mapProposalConfidence', () => {
  describe('low confidence (0.0-0.49)', () => {
    it('maps 0 to low', () => {
      expect(mapProposalConfidence(0 as EmberConfidence)).toBe('low');
    });

    it('maps 0.3 to low', () => {
      expect(mapProposalConfidence(0.3 as EmberConfidence)).toBe('low');
    });

    it('maps 0.49 to low', () => {
      expect(mapProposalConfidence(0.49 as EmberConfidence)).toBe('low');
    });
  });

  describe('medium confidence (0.5-0.74)', () => {
    it('maps 0.5 to medium', () => {
      expect(mapProposalConfidence(0.5 as EmberConfidence)).toBe('medium');
    });

    it('maps 0.6 to medium', () => {
      expect(mapProposalConfidence(0.6 as EmberConfidence)).toBe('medium');
    });

    it('maps 0.74 to medium', () => {
      expect(mapProposalConfidence(0.74 as EmberConfidence)).toBe('medium');
    });
  });

  describe('high confidence (0.75-1.0)', () => {
    it('maps 0.75 to high', () => {
      expect(mapProposalConfidence(0.75 as EmberConfidence)).toBe('high');
    });

    it('maps 0.85 to high', () => {
      expect(mapProposalConfidence(0.85 as EmberConfidence)).toBe('high');
    });

    it('maps 1.0 to high', () => {
      expect(mapProposalConfidence(1.0 as EmberConfidence)).toBe('high');
    });
  });
});

// =============================================================================
// createPromotionInput Tests
// =============================================================================

describe('createPromotionInput', () => {
  const promotedBy = 'user@example.com';
  const reason = 'This pattern is valuable and well-established';

  describe('basic promotion', () => {
    it('creates valid promotion input from proposal', () => {
      const proposal = createTestProposal();
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.proposal_id).toBe(proposal.id);
      expect(input.statement).toBe(proposal.summary);
      expect(input.type).toBe('pattern');
      expect(input.confidence).toBe('high'); // 0.75 maps to high
      expect(input.promoted_by).toBe(promotedBy);
      expect(input.reason).toBe(reason);
    });

    it('includes context with when and why', () => {
      const proposal = createTestProposal();
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.context.when).toBe(proposal.created_at);
      expect(input.context.why).toBe(proposal.rationale);
    });

    it('includes confidence rationale with percentage', () => {
      const proposal = createTestProposal({ confidence: 0.75 as EmberConfidence });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.confidence_rationale).toContain('75%');
    });

    it('preserves proposal metadata', () => {
      const metadata = { pattern_name: 'Test Pattern', occurrence_count: 5 };
      const proposal = createTestProposal({ metadata });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.metadata).toEqual(metadata);
    });
  });

  describe('type mapping', () => {
    it('maps decision proposal to decision memory', () => {
      const proposal = createTestProposal({ type: 'decision' });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.type).toBe('decision');
    });

    it('maps warning proposal to warning memory', () => {
      const proposal = createTestProposal({ type: 'warning' });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.type).toBe('warning');
    });

    it('maps lesson proposal to lesson memory', () => {
      const proposal = createTestProposal({ type: 'lesson' });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.type).toBe('lesson');
    });

    it('maps constraint proposal to constraint memory', () => {
      const proposal = createTestProposal({ type: 'constraint' });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.type).toBe('constraint');
    });

    it('throws for anomaly proposal without explicit type', () => {
      const proposal = createTestProposal({ type: 'anomaly' });

      expect(() => createPromotionInput(proposal, promotedBy, reason)).toThrow(
        /Cannot create promotion input for proposal type 'anomaly'/
      );
    });

    it('accepts anomaly proposal with explicit memoryType', () => {
      const proposal = createTestProposal({ type: 'anomaly' });
      const input = createPromotionInput(proposal, promotedBy, reason, {
        memoryType: 'warning',
      });

      expect(input.type).toBe('warning');
    });
  });

  describe('confidence mapping', () => {
    it('maps low confidence to low level', () => {
      const proposal = createTestProposal({ confidence: 0.3 as EmberConfidence });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.confidence).toBe('low');
    });

    it('maps medium confidence to medium level', () => {
      const proposal = createTestProposal({ confidence: 0.6 as EmberConfidence });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.confidence).toBe('medium');
    });

    it('maps high confidence to high level', () => {
      const proposal = createTestProposal({ confidence: 0.85 as EmberConfidence });
      const input = createPromotionInput(proposal, promotedBy, reason);

      expect(input.confidence).toBe('high');
    });
  });

  describe('option overrides', () => {
    it('allows overriding memory type', () => {
      const proposal = createTestProposal({ type: 'pattern' });
      const input = createPromotionInput(proposal, promotedBy, reason, {
        memoryType: 'doctrine',
      });

      expect(input.type).toBe('doctrine');
    });

    it('allows overriding confidence level', () => {
      const proposal = createTestProposal({ confidence: 0.3 as EmberConfidence });
      const input = createPromotionInput(proposal, promotedBy, reason, {
        confidence: 'high',
      });

      expect(input.confidence).toBe('high');
    });

    it('allows overriding statement', () => {
      const proposal = createTestProposal();
      const customStatement = 'Custom statement for memory';
      const input = createPromotionInput(proposal, promotedBy, reason, {
        statement: customStatement,
      });

      expect(input.statement).toBe(customStatement);
    });

    it('allows adding conditions', () => {
      const proposal = createTestProposal();
      const conditions = ['When deploying to production', 'For critical services'];
      const input = createPromotionInput(proposal, promotedBy, reason, { conditions });

      expect(input.context.conditions).toEqual(conditions);
    });

    it('allows setting scope', () => {
      const proposal = createTestProposal();
      const input = createPromotionInput(proposal, promotedBy, reason, {
        scope: 'Backend services only',
      });

      expect(input.context.scope).toBe('Backend services only');
    });

    it('allows adding tags', () => {
      const proposal = createTestProposal();
      const tags = ['architecture', 'performance'];
      const input = createPromotionInput(proposal, promotedBy, reason, { tags });

      expect(input.context.tags).toEqual(tags);
    });
  });
});

// =============================================================================
// expandProvenanceSummary Tests
// =============================================================================

describe('expandProvenanceSummary', () => {
  describe('basic expansion', () => {
    it('creates valid provenance chain from summary', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.kindling_sources).toHaveLength(1);
      expect(chain.source_sessions).toEqual([validUuid]);
    });

    it('includes observation_id in kindling refs', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.kindling_sources[0].observation_id).toBe(validUuid);
    });

    it('includes session_id in kindling refs', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.kindling_sources[0].session_id).toBe(validUuid);
    });

    it('uses default kind "observation" when not provided', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.kindling_sources[0].kind).toBe('observation');
    });
  });

  describe('observation kinds', () => {
    it('uses provided observation kinds', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      };
      const kinds = { [validUuid]: 'gate_evaluated' };

      const chain = expandProvenanceSummary(summary, kinds);

      expect(chain.kindling_sources[0].kind).toBe('gate_evaluated');
    });

    it('handles mixed provided and default kinds', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid, validUuid2],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: laterTimestamp,
      };
      const kinds = { [validUuid]: 'action_executed' };

      const chain = expandProvenanceSummary(summary, kinds);

      expect(chain.kindling_sources[0].kind).toBe('action_executed');
      expect(chain.kindling_sources[1].kind).toBe('observation');
    });
  });

  describe('timestamp handling', () => {
    it('uses earliest_observation for single observation', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: laterTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.kindling_sources[0].timestamp).toBe(validTimestamp);
    });

    it('distributes timestamps across multiple observations', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid, validUuid2],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: laterTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.kindling_sources[0].timestamp).toBe(validTimestamp);
      expect(chain.kindling_sources[1].timestamp).toBe(laterTimestamp);
    });
  });

  describe('session handling', () => {
    it('cycles through sessions for multiple observations', () => {
      const session1 = validUuid;
      const session2 = validUuid2;
      const obs1 = validUuid;
      const obs2 = validUuid2;
      const obs3 = '550e8400-e29b-41d4-a716-446655440002';

      const summary: ProvenanceSummary = {
        observation_ids: [obs1, obs2, obs3],
        session_ids: [session1, session2],
        earliest_observation: validTimestamp,
        latest_observation: laterTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.kindling_sources[0].session_id).toBe(session1);
      expect(chain.kindling_sources[1].session_id).toBe(session2);
      expect(chain.kindling_sources[2].session_id).toBe(session1); // cycles back
    });
  });

  describe('ember_source handling', () => {
    it('includes ember_source when proposal_id present', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        proposal_id: validUuid2,
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.ember_source).toBeDefined();
      expect(chain.ember_source?.proposal_id).toBe(validUuid2);
    });

    it('omits ember_source when no proposal_id', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.ember_source).toBeUndefined();
    });

    it('sets ember_source created_at to earliest_observation', () => {
      const summary: ProvenanceSummary = {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        proposal_id: validUuid2,
        earliest_observation: validTimestamp,
        latest_observation: laterTimestamp,
      };

      const chain = expandProvenanceSummary(summary);

      expect(chain.ember_source?.created_at).toBe(validTimestamp);
    });
  });
});
