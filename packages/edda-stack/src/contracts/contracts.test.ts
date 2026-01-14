/**
 * Edda Stack Contracts Tests
 *
 * Tests for all shared type definitions and schemas.
 */

import { describe, it, expect } from 'vitest';
import {
  // Identifiers
  UuidSchema,
  ContentHashSchema,
  createObservationId,
  createSessionId,
  createProposalId,
  createMemoryId,
  isValidUuid,
  isValidContentHash,
  type ObservationId,
  type SessionId,
  type Timestamp,
  // Temporal
  TimestampSchema,
  DurationMsSchema,
  TimeRangeSchema,
  TtlConfigSchema,
  now,
  parseTimestamp,
  isValidTimestamp,
  calculateExpiry,
  isExpired,
  remainingTtlMs,
  durationBetween,
  lastNDays,
  createExpiryInfo,
  // Confidence
  EmberConfidenceSchema,
  EddaConfidenceLevelSchema,
  suggestEddaConfidence,
  meetsThreshold,
  clampConfidence,
  averageConfidence,
  weightedConfidence,
  formatEmberConfidence,
  formatEddaConfidence,
  // Provenance
  KindlingRefSchema,
  ProvenanceChainSchema,
  createKindlingRef,
  validateProvenanceIntegrity,
  type ProvenanceChain,
  // Ember Proposals
  ProposalTypeSchema,
  ProposalStatusSchema,
  CandidateProposalSchema,
  ProposalQuerySchema,
  proposalTypeDescriptions,
  // Edda Memory
  MemoryTypeSchema,
  MemoryStatusSchema,
  MemoryObjectSchema,
  MemoryQuerySchema,
  MEMORY_SCHEMA_VERSION,
  memoryTypeDescriptions,
  suggestMemoryType,
} from './index.js';

// =============================================================================
// Identifiers Tests (STACK-001)
// =============================================================================

describe('Identifiers (STACK-001)', () => {
  const validUuid = '550e8400-e29b-41d4-a716-446655440000';
  const invalidUuid = 'not-a-uuid';
  const validHash = 'a'.repeat(64);
  const invalidHash = 'abc123';

  describe('UuidSchema', () => {
    it('accepts valid UUIDs', () => {
      expect(UuidSchema.safeParse(validUuid).success).toBe(true);
    });

    it('rejects invalid UUIDs', () => {
      expect(UuidSchema.safeParse(invalidUuid).success).toBe(false);
    });
  });

  describe('ContentHashSchema', () => {
    it('accepts valid SHA-256 hashes', () => {
      expect(ContentHashSchema.safeParse(validHash).success).toBe(true);
    });

    it('rejects invalid hashes', () => {
      expect(ContentHashSchema.safeParse(invalidHash).success).toBe(false);
    });
  });

  describe('Branded identifiers', () => {
    it('creates branded ObservationId', () => {
      const id = createObservationId(validUuid);
      expect(id).toBe(validUuid);
    });

    it('creates branded SessionId', () => {
      const id = createSessionId(validUuid);
      expect(id).toBe(validUuid);
    });

    it('creates branded ProposalId', () => {
      const id = createProposalId(validUuid);
      expect(id).toBe(validUuid);
    });

    it('creates branded MemoryId', () => {
      const id = createMemoryId(validUuid);
      expect(id).toBe(validUuid);
    });

    it('throws on invalid UUID', () => {
      expect(() => createObservationId(invalidUuid)).toThrow();
    });
  });

  describe('Utility functions', () => {
    it('isValidUuid returns true for valid', () => {
      expect(isValidUuid(validUuid)).toBe(true);
    });

    it('isValidUuid returns false for invalid', () => {
      expect(isValidUuid(invalidUuid)).toBe(false);
    });

    it('isValidContentHash returns true for valid', () => {
      expect(isValidContentHash(validHash)).toBe(true);
    });

    it('isValidContentHash returns false for invalid', () => {
      expect(isValidContentHash(invalidHash)).toBe(false);
    });
  });
});

// =============================================================================
// Temporal Tests (STACK-002)
// =============================================================================

describe('Temporal (STACK-002)', () => {
  const validTimestamp = '2024-01-15T14:30:00.000Z';
  const invalidTimestamp = 'not-a-timestamp';

  describe('TimestampSchema', () => {
    it('accepts valid ISO8601 timestamps', () => {
      expect(TimestampSchema.safeParse(validTimestamp).success).toBe(true);
    });

    it('rejects invalid timestamps', () => {
      expect(TimestampSchema.safeParse(invalidTimestamp).success).toBe(false);
    });
  });

  describe('DurationMsSchema', () => {
    it('accepts non-negative integers', () => {
      expect(DurationMsSchema.safeParse(1000).success).toBe(true);
      expect(DurationMsSchema.safeParse(0).success).toBe(true);
    });

    it('rejects negative values', () => {
      expect(DurationMsSchema.safeParse(-1).success).toBe(false);
    });

    it('rejects floats', () => {
      expect(DurationMsSchema.safeParse(1.5).success).toBe(false);
    });
  });

  describe('TimeRangeSchema', () => {
    it('accepts valid range with start before end', () => {
      const range = {
        start: '2024-01-01T00:00:00.000Z',
        end: '2024-01-02T00:00:00.000Z',
      };
      expect(TimeRangeSchema.safeParse(range).success).toBe(true);
    });

    it('accepts range without end', () => {
      const range = { start: '2024-01-01T00:00:00.000Z' };
      expect(TimeRangeSchema.safeParse(range).success).toBe(true);
    });

    it('rejects range with end before start', () => {
      const range = {
        start: '2024-01-02T00:00:00.000Z',
        end: '2024-01-01T00:00:00.000Z',
      };
      expect(TimeRangeSchema.safeParse(range).success).toBe(false);
    });
  });

  describe('TtlConfigSchema', () => {
    it('uses defaults', () => {
      const result = TtlConfigSchema.parse({});
      expect(result.default_ttl_days).toBe(30);
      expect(result.min_ttl_days).toBe(7);
      expect(result.max_ttl_days).toBe(90);
    });
  });

  describe('Utility functions', () => {
    it('now() returns valid timestamp', () => {
      const timestamp = now();
      expect(isValidTimestamp(timestamp)).toBe(true);
    });

    it('parseTimestamp validates and returns', () => {
      const result = parseTimestamp(validTimestamp);
      expect(result).toBe(validTimestamp);
    });

    it('calculateExpiry adds days correctly', () => {
      const created = '2024-01-15T00:00:00.000Z';
      const expiry = calculateExpiry(created, 30);
      expect(expiry).toBe('2024-02-14T00:00:00.000Z');
    });

    it('isExpired returns true for past timestamps', () => {
      const past = '2020-01-01T00:00:00.000Z';
      expect(isExpired(past)).toBe(true);
    });

    it('isExpired returns false for future timestamps', () => {
      const future = '2030-01-01T00:00:00.000Z';
      expect(isExpired(future)).toBe(false);
    });

    it('remainingTtlMs returns 0 for expired', () => {
      const past = '2020-01-01T00:00:00.000Z';
      expect(remainingTtlMs(past)).toBe(0);
    });

    it('durationBetween calculates correctly', () => {
      const start = '2024-01-01T00:00:00.000Z';
      const end = '2024-01-01T01:00:00.000Z';
      expect(durationBetween(start, end)).toBe(3600000); // 1 hour in ms
    });

    it('lastNDays returns valid range', () => {
      const range = lastNDays(7);
      expect(TimeRangeSchema.safeParse(range).success).toBe(true);
    });

    it('createExpiryInfo creates valid object', () => {
      const info = createExpiryInfo('2024-01-15T00:00:00.000Z', 30);
      expect(info.created_at).toBe('2024-01-15T00:00:00.000Z');
      expect(info.expires_at).toBe('2024-02-14T00:00:00.000Z');
      expect(info.ttl_days).toBe(30);
    });
  });
});

// =============================================================================
// Confidence Tests (STACK-003)
// =============================================================================

describe('Confidence (STACK-003)', () => {
  describe('EmberConfidenceSchema', () => {
    it('accepts values in range 0-1', () => {
      expect(EmberConfidenceSchema.safeParse(0).success).toBe(true);
      expect(EmberConfidenceSchema.safeParse(0.5).success).toBe(true);
      expect(EmberConfidenceSchema.safeParse(1).success).toBe(true);
    });

    it('rejects values outside range', () => {
      expect(EmberConfidenceSchema.safeParse(-0.1).success).toBe(false);
      expect(EmberConfidenceSchema.safeParse(1.1).success).toBe(false);
    });
  });

  describe('EddaConfidenceLevelSchema', () => {
    it('accepts valid levels', () => {
      expect(EddaConfidenceLevelSchema.safeParse('low').success).toBe(true);
      expect(EddaConfidenceLevelSchema.safeParse('medium').success).toBe(true);
      expect(EddaConfidenceLevelSchema.safeParse('high').success).toBe(true);
    });

    it('rejects invalid levels', () => {
      expect(EddaConfidenceLevelSchema.safeParse('very_high').success).toBe(false);
    });
  });

  describe('suggestEddaConfidence', () => {
    it('suggests low for scores < 0.5', () => {
      expect(suggestEddaConfidence(0.3)).toBe('low');
    });

    it('suggests medium for scores 0.5-0.75', () => {
      expect(suggestEddaConfidence(0.6)).toBe('medium');
    });

    it('suggests high for scores >= 0.75', () => {
      expect(suggestEddaConfidence(0.8)).toBe('high');
    });
  });

  describe('Utility functions', () => {
    it('meetsThreshold compares correctly', () => {
      expect(meetsThreshold(0.6, 0.5)).toBe(true);
      expect(meetsThreshold(0.4, 0.5)).toBe(false);
    });

    it('clampConfidence clamps values', () => {
      expect(clampConfidence(-0.5)).toBe(0);
      expect(clampConfidence(1.5)).toBe(1);
      expect(clampConfidence(0.5)).toBe(0.5);
    });

    it('averageConfidence calculates correctly', () => {
      expect(averageConfidence([0.5, 0.7])).toBe(0.6);
      expect(averageConfidence([])).toBe(0);
    });

    it('weightedConfidence calculates correctly', () => {
      const result = weightedConfidence([
        { score: 0.8, weight: 2 },
        { score: 0.4, weight: 1 },
      ]);
      // (0.8*2 + 0.4*1) / 3 = 2/3 ≈ 0.667
      expect(result).toBeCloseTo(0.667, 2);
    });

    it('formatEmberConfidence formats correctly', () => {
      expect(formatEmberConfidence(0.75)).toBe('75%');
    });

    it('formatEddaConfidence formats correctly', () => {
      expect(formatEddaConfidence('high')).toBe('High confidence');
    });
  });
});

// =============================================================================
// Provenance Tests (STACK-004)
// =============================================================================

describe('Provenance (STACK-004)', () => {
  const validUuid = '550e8400-e29b-41d4-a716-446655440000';
  const validTimestamp = '2024-01-15T14:30:00.000Z';

  describe('KindlingRefSchema', () => {
    it('accepts valid reference', () => {
      const ref = {
        observation_id: validUuid,
        session_id: validUuid,
        kind: 'gate_evaluated',
        timestamp: validTimestamp,
      };
      expect(KindlingRefSchema.safeParse(ref).success).toBe(true);
    });
  });

  describe('ProvenanceChainSchema', () => {
    it('accepts valid chain', () => {
      const chain = {
        kindling_sources: [
          {
            observation_id: validUuid,
            session_id: validUuid,
            kind: 'gate_evaluated',
            timestamp: validTimestamp,
          },
        ],
        source_sessions: [validUuid],
      };
      expect(ProvenanceChainSchema.safeParse(chain).success).toBe(true);
    });

    it('rejects chain without kindling sources', () => {
      const chain = {
        kindling_sources: [],
        source_sessions: [validUuid],
      };
      expect(ProvenanceChainSchema.safeParse(chain).success).toBe(false);
    });
  });

  describe('createKindlingRef', () => {
    it('creates valid reference', () => {
      const ref = createKindlingRef(
        validUuid as ObservationId,
        validUuid as SessionId,
        'gate_evaluated',
        validTimestamp as Timestamp
      );
      expect(KindlingRefSchema.safeParse(ref).success).toBe(true);
    });
  });

  describe('validateProvenanceIntegrity', () => {
    it('validates consistent chain', () => {
      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: validUuid as ObservationId,
            session_id: validUuid as SessionId,
            kind: 'gate_evaluated',
            timestamp: validTimestamp as Timestamp,
          },
        ],
        source_sessions: [validUuid as SessionId],
      };
      const result = validateProvenanceIntegrity(chain);
      expect(result.valid).toBe(true);
    });

    it('detects missing session', () => {
      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: validUuid as ObservationId,
            session_id: validUuid as SessionId,
            kind: 'gate_evaluated',
            timestamp: validTimestamp as Timestamp,
          },
        ],
        source_sessions: ['550e8400-e29b-41d4-a716-446655440001' as SessionId],
      };
      const result = validateProvenanceIntegrity(chain);
      expect(result.valid).toBe(false);
    });
  });
});

// =============================================================================
// Ember Proposal Tests (EMBER-001)
// =============================================================================

describe('Ember Proposals (EMBER-001)', () => {
  const validUuid = '550e8400-e29b-41d4-a716-446655440000';
  const validTimestamp = '2024-01-15T14:30:00.000Z';

  describe('ProposalTypeSchema', () => {
    it('accepts all 6 types', () => {
      const types = ['decision', 'pattern', 'warning', 'lesson', 'anomaly', 'constraint'];
      for (const type of types) {
        expect(ProposalTypeSchema.safeParse(type).success).toBe(true);
      }
    });

    it('rejects invalid types', () => {
      expect(ProposalTypeSchema.safeParse('invalid').success).toBe(false);
    });
  });

  describe('ProposalStatusSchema', () => {
    it('accepts all statuses', () => {
      const statuses = ['active', 'promoted', 'expired', 'dismissed'];
      for (const status of statuses) {
        expect(ProposalStatusSchema.safeParse(status).success).toBe(true);
      }
    });
  });

  describe('CandidateProposalSchema', () => {
    const validProposal = {
      id: validUuid,
      type: 'pattern',
      status: 'active',
      summary: 'Test pattern observed',
      rationale: 'This pattern appears multiple times',
      confidence: 0.75,
      provenance: {
        observation_ids: [validUuid],
        session_ids: [validUuid],
        earliest_observation: validTimestamp,
        latest_observation: validTimestamp,
      },
      created_at: validTimestamp,
      expires_at: '2024-02-14T14:30:00.000Z',
      ttl_days: 30,
    };

    it('accepts valid proposal', () => {
      expect(CandidateProposalSchema.safeParse(validProposal).success).toBe(true);
    });

    it('rejects proposal without summary', () => {
      const invalid = { ...validProposal, summary: '' };
      expect(CandidateProposalSchema.safeParse(invalid).success).toBe(false);
    });
  });

  describe('ProposalQuerySchema', () => {
    it('accepts empty query with defaults', () => {
      const result = ProposalQuerySchema.parse({});
      expect(result.limit).toBe(100);
      expect(result.offset).toBe(0);
      expect(result.sort_by).toBe('created_at');
      expect(result.sort_order).toBe('desc');
    });
  });

  describe('proposalTypeDescriptions', () => {
    it('has descriptions for all types', () => {
      const types = ['decision', 'pattern', 'warning', 'lesson', 'anomaly', 'constraint'];
      for (const type of types) {
        expect(
          proposalTypeDescriptions[type as keyof typeof proposalTypeDescriptions]
        ).toBeDefined();
      }
    });
  });
});

// =============================================================================
// Edda Memory Tests (EDDA-001)
// =============================================================================

describe('Edda Memory (EDDA-001)', () => {
  const validUuid = '550e8400-e29b-41d4-a716-446655440000';
  const validTimestamp = '2024-01-15T14:30:00.000Z';

  describe('MemoryTypeSchema', () => {
    it('accepts all 6 types', () => {
      const types = ['decision', 'pattern', 'constraint', 'warning', 'doctrine', 'lesson'];
      for (const type of types) {
        expect(MemoryTypeSchema.safeParse(type).success).toBe(true);
      }
    });

    it('rejects invalid types', () => {
      expect(MemoryTypeSchema.safeParse('anomaly').success).toBe(false); // anomaly is Ember-only
    });
  });

  describe('MemoryStatusSchema', () => {
    it('accepts all statuses', () => {
      const statuses = ['active', 'superseded', 'retired'];
      for (const status of statuses) {
        expect(MemoryStatusSchema.safeParse(status).success).toBe(true);
      }
    });
  });

  describe('MemoryObjectSchema', () => {
    const validMemory = {
      id: validUuid,
      type: 'decision',
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: 'We decided to use TypeScript for all new code',
      context: {
        when: '2024-01-15',
        why: 'Type safety improves maintainability',
        conditions: ['new projects', 'greenfield code'],
      },
      confidence: 'high',
      provenance: {
        kindling_sources: [
          {
            observation_id: validUuid,
            session_id: validUuid,
            kind: 'gate_evaluated',
            timestamp: validTimestamp,
          },
        ],
        source_sessions: [validUuid],
      },
      attribution: {
        actor: 'user@example.com',
        timestamp: validTimestamp,
        method: 'cli_command',
        reason: 'Codifying team decision',
      },
      created_at: validTimestamp,
    };

    it('accepts valid memory', () => {
      expect(MemoryObjectSchema.safeParse(validMemory).success).toBe(true);
    });

    it('rejects memory without statement', () => {
      const invalid = { ...validMemory, statement: '' };
      expect(MemoryObjectSchema.safeParse(invalid).success).toBe(false);
    });
  });

  describe('MemoryQuerySchema', () => {
    it('accepts empty query with defaults', () => {
      const result = MemoryQuerySchema.parse({});
      expect(result.limit).toBe(100);
      expect(result.offset).toBe(0);
      expect(result.include_superseded).toBe(false);
    });
  });

  describe('suggestMemoryType', () => {
    it('maps decision to decision', () => {
      expect(suggestMemoryType('decision')).toBe('decision');
    });

    it('maps pattern to pattern', () => {
      expect(suggestMemoryType('pattern')).toBe('pattern');
    });

    it('returns null for anomaly', () => {
      expect(suggestMemoryType('anomaly')).toBe(null);
    });
  });

  describe('memoryTypeDescriptions', () => {
    it('has descriptions for all types', () => {
      const types = ['decision', 'pattern', 'constraint', 'warning', 'doctrine', 'lesson'];
      for (const type of types) {
        expect(memoryTypeDescriptions[type as keyof typeof memoryTypeDescriptions]).toBeDefined();
      }
    });
  });

  describe('MEMORY_SCHEMA_VERSION', () => {
    it('is defined', () => {
      expect(MEMORY_SCHEMA_VERSION).toBe(1);
    });
  });
});
