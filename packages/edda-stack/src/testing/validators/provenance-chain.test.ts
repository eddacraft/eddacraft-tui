/**
 * Provenance Chain Validator Tests (STACK-011)
 *
 * Tests for the provenance chain validation utilities.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { v4 as uuidv4 } from 'uuid';
import {
  validateProvenanceChain,
  validateKindlingRefs,
  validateEmberRef,
  validateTemporalOrdering,
  validateSessionConsistency,
  validateNoDuplicates,
  hasValidationCode,
  getIssuesByCode,
  formatValidationResult,
  ProvenanceValidationCode,
} from './provenance-chain.js';
import { createMockKindlingPort, createMockEmberPort } from '../mocks/index.js';
import type { MockKindlingPort } from '../mocks/kindling.mock.js';
import type { MockEmberPort } from '../mocks/ember.mock.js';
import type { ProvenanceChain, KindlingRef, EmberRef } from '../../contracts/provenance.js';
import type { Timestamp } from '../../contracts/index.js';
import {
  createObservationId,
  createSessionId,
  createProposalId,
} from '../../contracts/identifiers.js';
import { now, calculateExpiry } from '../../contracts/temporal.js';
import type { CandidateProposal } from '../../contracts/ember-proposal.js';
import type { Observation } from '../../contracts/ports/kindling.port.js';

describe('Provenance Chain Validator', () => {
  let kindlingPort: MockKindlingPort;
  let emberPort: MockEmberPort;

  // Test data
  const sessionId = createSessionId(uuidv4());
  const observationId = createObservationId(uuidv4());
  const proposalId = createProposalId(uuidv4());
  const timestamp = now();

  beforeEach(() => {
    kindlingPort = createMockKindlingPort();
    emberPort = createMockEmberPort();
  });

  describe('validateProvenanceChain', () => {
    it('validates a correct provenance chain', async () => {
      // Set up Kindling observation
      const observation: Observation = {
        id: observationId,
        session_id: sessionId,
        kind: 'gate_evaluated',
        timestamp,
        summary: 'Test observation',
        data: {},
      };
      kindlingPort._store.set(observationId, observation);

      // Set up Ember proposal
      const proposal: CandidateProposal = {
        id: proposalId,
        type: 'pattern',
        status: 'active',
        summary: 'Test proposal',
        rationale: 'Test rationale',
        confidence: 0.75,
        signals: [],
        provenance: {
          observation_ids: [observationId],
          session_ids: [sessionId],
          earliest_observation: timestamp,
          latest_observation: timestamp,
        },
        created_at: timestamp,
        expires_at: calculateExpiry(timestamp, 30),
        ttl_days: 30,
      };
      emberPort._store.set(proposalId, proposal);

      const chain: ProvenanceChain = {
        ember_source: {
          proposal_id: proposalId,
          proposal_type: 'pattern',
          confidence: 0.75,
          created_at: timestamp,
        },
        kindling_sources: [
          {
            observation_id: observationId,
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp,
          },
        ],
        source_sessions: [sessionId],
      };

      const result = await validateProvenanceChain(chain, kindlingPort, emberPort);

      expect(result.valid).toBe(true);
      expect(result.issues).toHaveLength(0);
      expect(result.stats.kindlingRefsChecked).toBe(1);
      expect(result.stats.kindlingRefsFound).toBe(1);
      expect(result.stats.emberRefChecked).toBe(true);
      expect(result.stats.emberRefFound).toBe(true);
    });

    it('detects missing Kindling observation', async () => {
      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: observationId, // Not in store
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp,
          },
        ],
        source_sessions: [sessionId],
      };

      const result = await validateProvenanceChain(chain, kindlingPort, emberPort);

      expect(result.valid).toBe(false);
      expect(hasValidationCode(result, ProvenanceValidationCode.MISSING_OBSERVATION)).toBe(true);
    });

    it('detects missing Ember proposal', async () => {
      // Set up Kindling observation
      const observation: Observation = {
        id: observationId,
        session_id: sessionId,
        kind: 'gate_evaluated',
        timestamp,
        summary: 'Test observation',
        data: {},
      };
      kindlingPort._store.set(observationId, observation);

      const chain: ProvenanceChain = {
        ember_source: {
          proposal_id: proposalId, // Not in store
          proposal_type: 'pattern',
          confidence: 0.75,
          created_at: timestamp,
        },
        kindling_sources: [
          {
            observation_id: observationId,
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp,
          },
        ],
        source_sessions: [sessionId],
      };

      const result = await validateProvenanceChain(chain, kindlingPort, emberPort);

      expect(result.valid).toBe(false);
      expect(hasValidationCode(result, ProvenanceValidationCode.MISSING_PROPOSAL)).toBe(true);
    });

    it('detects empty kindling sources', async () => {
      const chain: ProvenanceChain = {
        kindling_sources: [],
        source_sessions: [sessionId],
      };

      const result = await validateProvenanceChain(chain, kindlingPort, emberPort);

      expect(result.valid).toBe(false);
      expect(hasValidationCode(result, ProvenanceValidationCode.NO_KINDLING_SOURCES)).toBe(true);
    });

    it('detects empty source sessions', async () => {
      const observation: Observation = {
        id: observationId,
        session_id: sessionId,
        kind: 'gate_evaluated',
        timestamp,
        summary: 'Test observation',
        data: {},
      };
      kindlingPort._store.set(observationId, observation);

      const chain: ProvenanceChain = {
        kindling_sources: [
          {
            observation_id: observationId,
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp,
          },
        ],
        source_sessions: [],
      };

      const result = await validateProvenanceChain(chain, kindlingPort, emberPort);

      expect(result.valid).toBe(false);
      expect(hasValidationCode(result, ProvenanceValidationCode.NO_SOURCE_SESSIONS)).toBe(true);
    });
  });

  describe('validateKindlingRefs', () => {
    it('returns success when all refs exist', async () => {
      const observation: Observation = {
        id: observationId,
        session_id: sessionId,
        kind: 'gate_evaluated',
        timestamp,
        summary: 'Test observation',
        data: {},
      };
      kindlingPort._store.set(observationId, observation);

      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
      ];

      const result = await validateKindlingRefs(refs, kindlingPort);

      expect(result.issues).toHaveLength(0);
      expect(result.checked).toBe(1);
      expect(result.found).toBe(1);
    });

    it('reports missing observations', async () => {
      const refs: KindlingRef[] = [
        {
          observation_id: observationId, // Not in store
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
      ];

      const result = await validateKindlingRefs(refs, kindlingPort);

      expect(result.issues).toHaveLength(1);
      expect(result.issues[0].code).toBe(ProvenanceValidationCode.MISSING_OBSERVATION);
      expect(result.checked).toBe(1);
      expect(result.found).toBe(0);
    });

    it('handles empty refs array', async () => {
      const result = await validateKindlingRefs([], kindlingPort);

      expect(result.issues).toHaveLength(0);
      expect(result.checked).toBe(0);
      expect(result.found).toBe(0);
    });

    it('handles undefined refs', async () => {
      const result = await validateKindlingRefs(undefined, kindlingPort);

      expect(result.issues).toHaveLength(0);
      expect(result.checked).toBe(0);
      expect(result.found).toBe(0);
    });
  });

  describe('validateEmberRef', () => {
    it('returns success when proposal exists', async () => {
      const proposal: CandidateProposal = {
        id: proposalId,
        type: 'pattern',
        status: 'active',
        summary: 'Test proposal',
        rationale: 'Test rationale',
        confidence: 0.75,
        signals: [],
        provenance: {
          observation_ids: [observationId],
          session_ids: [sessionId],
          earliest_observation: timestamp,
          latest_observation: timestamp,
        },
        created_at: timestamp,
        expires_at: calculateExpiry(timestamp, 30),
        ttl_days: 30,
      };
      emberPort._store.set(proposalId, proposal);

      const ref: EmberRef = {
        proposal_id: proposalId,
        proposal_type: 'pattern',
        confidence: 0.75,
        created_at: timestamp,
      };

      const result = await validateEmberRef(ref, emberPort);

      expect(result.issues).toHaveLength(0);
      expect(result.found).toBe(true);
    });

    it('reports missing proposal', async () => {
      const ref: EmberRef = {
        proposal_id: proposalId, // Not in store
        proposal_type: 'pattern',
        confidence: 0.75,
        created_at: timestamp,
      };

      const result = await validateEmberRef(ref, emberPort);

      expect(result.issues).toHaveLength(1);
      expect(result.issues[0].code).toBe(ProvenanceValidationCode.MISSING_PROPOSAL);
      expect(result.found).toBe(false);
    });

    it('reports incomplete ember source', async () => {
      const ref = {
        proposal_id: '', // Empty proposal ID
        proposal_type: 'pattern',
        confidence: 0.75,
        created_at: timestamp,
      } as EmberRef;

      const result = await validateEmberRef(ref, emberPort);

      expect(result.issues).toHaveLength(1);
      expect(result.issues[0].code).toBe(ProvenanceValidationCode.EMBER_SOURCE_INCOMPLETE);
      expect(result.found).toBe(false);
    });
  });

  describe('validateTemporalOrdering', () => {
    it('accepts observations before proposal creation', () => {
      const proposalCreatedAt = '2024-01-15T12:00:00.000Z' as Timestamp;
      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: '2024-01-15T11:00:00.000Z' as Timestamp, // 1 hour before
        },
      ];

      const result = validateTemporalOrdering(refs, proposalCreatedAt);

      expect(result.issues).toHaveLength(0);
    });

    it('accepts observations at proposal creation (within grace period)', () => {
      const proposalCreatedAt = '2024-01-15T12:00:00.000Z' as Timestamp;
      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: '2024-01-15T12:03:00.000Z' as Timestamp, // 3 minutes after (within 5 min grace)
        },
      ];

      const result = validateTemporalOrdering(refs, proposalCreatedAt);

      expect(result.issues).toHaveLength(0);
    });

    it('detects observations significantly after proposal creation', () => {
      const proposalCreatedAt = '2024-01-15T12:00:00.000Z' as Timestamp;
      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: '2024-01-15T13:00:00.000Z' as Timestamp, // 1 hour after
        },
      ];

      const result = validateTemporalOrdering(refs, proposalCreatedAt);

      expect(result.issues).toHaveLength(1);
      expect(result.issues[0].code).toBe(
        ProvenanceValidationCode.TEMPORAL_OBSERVATION_AFTER_PROMOTION
      );
    });

    it('validates observation range correctly', () => {
      const proposalCreatedAt = '2024-01-15T12:00:00.000Z' as Timestamp;
      const refs: KindlingRef[] = [
        {
          observation_id: createObservationId(uuidv4()),
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: '2024-01-15T10:00:00.000Z' as Timestamp,
        },
        {
          observation_id: createObservationId(uuidv4()),
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: '2024-01-15T11:00:00.000Z' as Timestamp,
        },
      ];

      const result = validateTemporalOrdering(refs, proposalCreatedAt);

      expect(result.issues).toHaveLength(0);
    });
  });

  describe('validateSessionConsistency', () => {
    it('accepts when all sessions match', () => {
      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
      ];

      const result = validateSessionConsistency(refs, [sessionId]);

      expect(result.issues).toHaveLength(0);
    });

    it('detects session mismatch', () => {
      const differentSession = createSessionId(uuidv4());
      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: differentSession, // Not in source_sessions
          kind: 'gate_evaluated',
          timestamp,
        },
      ];

      const result = validateSessionConsistency(refs, [sessionId]);

      expect(result.issues).toHaveLength(1);
      expect(result.issues[0].code).toBe(ProvenanceValidationCode.SESSION_MISMATCH);
    });

    it('handles undefined refs', () => {
      const result = validateSessionConsistency(undefined, [sessionId]);

      expect(result.issues).toHaveLength(0);
    });

    it('handles undefined sessions', () => {
      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
      ];

      const result = validateSessionConsistency(refs, undefined);

      expect(result.issues).toHaveLength(0);
    });
  });

  describe('validateNoDuplicates', () => {
    it('accepts unique observation IDs', () => {
      const refs: KindlingRef[] = [
        {
          observation_id: createObservationId(uuidv4()),
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
        {
          observation_id: createObservationId(uuidv4()),
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
      ];

      const result = validateNoDuplicates(refs);

      expect(result.issues).toHaveLength(0);
    });

    it('detects duplicate observation IDs', () => {
      const refs: KindlingRef[] = [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
        {
          observation_id: observationId, // Duplicate
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp,
        },
      ];

      const result = validateNoDuplicates(refs);

      expect(result.issues).toHaveLength(1);
      expect(result.issues[0].code).toBe(ProvenanceValidationCode.DUPLICATE_OBSERVATIONS);
    });

    it('handles empty refs', () => {
      const result = validateNoDuplicates([]);

      expect(result.issues).toHaveLength(0);
    });

    it('handles undefined refs', () => {
      const result = validateNoDuplicates(undefined);

      expect(result.issues).toHaveLength(0);
    });
  });

  describe('Utility functions', () => {
    describe('hasValidationCode', () => {
      it('returns true when code is present', () => {
        const result = {
          valid: false,
          issues: [
            {
              code: ProvenanceValidationCode.MISSING_OBSERVATION,
              message: 'Test',
            },
          ],
          stats: {
            kindlingRefsChecked: 0,
            kindlingRefsFound: 0,
            sessionsChecked: 0,
            emberRefChecked: false,
            emberRefFound: false,
          },
        };

        expect(hasValidationCode(result, ProvenanceValidationCode.MISSING_OBSERVATION)).toBe(true);
      });

      it('returns false when code is not present', () => {
        const result = {
          valid: false,
          issues: [
            {
              code: ProvenanceValidationCode.MISSING_OBSERVATION,
              message: 'Test',
            },
          ],
          stats: {
            kindlingRefsChecked: 0,
            kindlingRefsFound: 0,
            sessionsChecked: 0,
            emberRefChecked: false,
            emberRefFound: false,
          },
        };

        expect(hasValidationCode(result, ProvenanceValidationCode.MISSING_PROPOSAL)).toBe(false);
      });
    });

    describe('getIssuesByCode', () => {
      it('returns matching issues', () => {
        const result = {
          valid: false,
          issues: [
            {
              code: ProvenanceValidationCode.MISSING_OBSERVATION,
              message: 'Test 1',
            },
            {
              code: ProvenanceValidationCode.MISSING_OBSERVATION,
              message: 'Test 2',
            },
            {
              code: ProvenanceValidationCode.MISSING_PROPOSAL,
              message: 'Test 3',
            },
          ],
          stats: {
            kindlingRefsChecked: 0,
            kindlingRefsFound: 0,
            sessionsChecked: 0,
            emberRefChecked: false,
            emberRefFound: false,
          },
        };

        const issues = getIssuesByCode(result, ProvenanceValidationCode.MISSING_OBSERVATION);

        expect(issues).toHaveLength(2);
        expect(issues[0].message).toBe('Test 1');
        expect(issues[1].message).toBe('Test 2');
      });

      it('returns empty array when no matches', () => {
        const result = {
          valid: true,
          issues: [],
          stats: {
            kindlingRefsChecked: 0,
            kindlingRefsFound: 0,
            sessionsChecked: 0,
            emberRefChecked: false,
            emberRefFound: false,
          },
        };

        const issues = getIssuesByCode(result, ProvenanceValidationCode.MISSING_OBSERVATION);

        expect(issues).toHaveLength(0);
      });
    });

    describe('formatValidationResult', () => {
      it('formats valid result', () => {
        const result = {
          valid: true,
          issues: [],
          stats: {
            kindlingRefsChecked: 3,
            kindlingRefsFound: 3,
            sessionsChecked: 1,
            emberRefChecked: true,
            emberRefFound: true,
          },
        };

        const formatted = formatValidationResult(result);

        expect(formatted).toBe('Provenance chain is valid');
      });

      it('formats invalid result with issues', () => {
        const result = {
          valid: false,
          issues: [
            {
              code: ProvenanceValidationCode.MISSING_OBSERVATION,
              message: 'Observation not found',
            },
            {
              code: ProvenanceValidationCode.SESSION_MISMATCH,
              message: 'Session mismatch',
            },
          ],
          stats: {
            kindlingRefsChecked: 2,
            kindlingRefsFound: 1,
            sessionsChecked: 1,
            emberRefChecked: true,
            emberRefFound: false,
          },
        };

        const formatted = formatValidationResult(result);

        expect(formatted).toContain('Provenance chain validation failed');
        expect(formatted).toContain('[MISSING_OBSERVATION]');
        expect(formatted).toContain('[SESSION_MISMATCH]');
        expect(formatted).toContain('Kindling refs: 1/2 found');
        expect(formatted).toContain('Ember ref: not found');
      });
    });
  });
});
