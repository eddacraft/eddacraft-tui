/**
 * Mock Implementations Test Suite (TCOV-015)
 *
 * Verifies that each mock in testing/mocks/ satisfies its corresponding port
 * interface contract both structurally (all methods present) and
 * behaviourally (correct return values, state transitions, edge cases).
 *
 * Scope: testing/mocks/edda.mock.ts, ember.mock.ts, kindling.mock.ts, index.ts
 * Out of scope: store-interfaces.ts, migration/ (owned by TCOV-016)
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { v4 as uuidv4 } from 'uuid';
import {
  createMockEddaPort,
  mockEddaWithMemories,
  mockEddaEmpty,
  mockEddaWithEvolutionChain,
  type MockEddaPort,
} from './edda.mock.js';
import {
  createMockEmberPort,
  mockEmberWithProposals,
  mockEmberEmpty,
  mockEmberWithMixedStatuses,
  type MockEmberPort,
} from './ember.mock.js';
import {
  createMockKindlingPort,
  mockKindlingWithObservations,
  mockKindlingEmpty,
  mockKindlingMultipleSessions,
  type MockKindlingPort,
} from './kindling.mock.js';
import {
  createMemoryId,
  createProposalId,
  createSessionId,
  createObservationId,
} from '../../contracts/identifiers.js';
import { calculateExpiry } from '../../contracts/temporal.js';
import type { MemoryId, Timestamp } from '../../contracts/index.js';
import { MEMORY_SCHEMA_VERSION } from '../../contracts/edda-memory.js';
import type { ProvenanceChain } from '../../contracts/provenance.js';

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const UUID_A = '550e8400-e29b-41d4-a716-446655440000';
const UUID_B = '550e8400-e29b-41d4-a716-446655440001';
const UUID_C = '550e8400-e29b-41d4-a716-446655440002';
const TS = '2024-06-01T10:00:00.000Z' as Timestamp;

function makeProvenanceChain(sessionUuid = UUID_A, obsUuid = UUID_B): ProvenanceChain {
  return {
    kindling_sources: [
      {
        observation_id: createObservationId(sessionUuid),
        session_id: createSessionId(obsUuid),
        kind: 'gate_evaluated',
        timestamp: TS,
      },
    ],
    source_sessions: [createSessionId(obsUuid)],
  };
}

function makeCreateMemoryInput(overrides: Record<string, unknown> = {}) {
  return {
    type: 'decision' as const,
    statement: 'Use TypeScript strict mode',
    context: { when: '2024-01-01', why: 'Type safety', conditions: [], tags: ['typescript'] },
    confidence: 'high' as const,
    provenance: makeProvenanceChain(),
    created_by: 'user@example.com',
    reason: 'Team decided',
    ...overrides,
  };
}

function makeCreateProposalInput(overrides: Record<string, unknown> = {}) {
  return {
    type: 'pattern' as const,
    summary: 'Factory pattern observed',
    rationale: 'Used in 5 files this week',
    confidence: 0.75,
    provenance: {
      observation_ids: [uuidv4()],
      session_ids: [createSessionId(UUID_A)],
      earliest_observation: TS,
      latest_observation: TS,
    },
    ...overrides,
  };
}

function makeCreateObservationInput(overrides: Record<string, unknown> = {}) {
  return {
    session_id: createSessionId(UUID_A),
    kind: 'gate_evaluated' as const,
    summary: 'Architecture gate passed',
    data: { gate: 'architecture', result: 'pass' },
    ...overrides,
  };
}

// =============================================================================
// EDDA MOCK TESTS
// =============================================================================

describe('MockEddaPort — createMockEddaPort (TCOV-015)', () => {
  let port: MockEddaPort;

  beforeEach(() => {
    port = createMockEddaPort();
  });

  // ─── write operations ───────────────────────────────────────────────────────

  describe('createMemory', () => {
    it('creates a memory and returns it with a generated id', async () => {
      const input = makeCreateMemoryInput();
      const memory = await port.createMemory(input);

      expect(memory.id).toBeTruthy();
      expect(memory.type).toBe('decision');
      expect(memory.statement).toBe('Use TypeScript strict mode');
      expect(memory.status).toBe('active');
      expect(memory.confidence).toBe('high');
      expect(memory.schema_version).toBe(MEMORY_SCHEMA_VERSION);
    });

    it('persists the memory in the internal store', async () => {
      const memory = await port.createMemory(makeCreateMemoryInput());
      expect(port._store.has(memory.id)).toBe(true);
    });

    it('records the mock call', async () => {
      await port.createMemory(makeCreateMemoryInput());
      expect(port._mocks.createMemory).toHaveBeenCalledOnce();
    });
  });

  describe('promoteProposal', () => {
    it('promotes a proposal and returns a memory', async () => {
      const proposalId = createProposalId(UUID_A);
      const memory = await port.promoteProposal({
        proposal_id: proposalId,
        type: 'pattern',
        confidence: 'medium',
        context: { when: '2024-01-01', why: 'Recurring usage', conditions: [], tags: [] },
        promoted_by: 'user@example.com',
        reason: 'Worth remembering',
      });

      expect(memory.status).toBe('active');
      expect(memory.type).toBe('pattern');
      expect(memory.provenance.ember_source?.proposal_id).toBe(proposalId);
    });

    it('indexes the promoted memory by proposal id', async () => {
      const proposalId = createProposalId(UUID_A);
      const memory = await port.promoteProposal({
        proposal_id: proposalId,
        type: 'decision',
        confidence: 'high',
        context: { when: '2024-01-01', why: 'Team decision', conditions: [] },
        promoted_by: 'user@example.com',
        reason: 'Codifying decision',
      });

      const retrieved = await port.getMemoryByProposalId(proposalId);
      expect(retrieved?.id).toBe(memory.id);
    });
  });

  describe('createMemoryFromProposal', () => {
    it('creates a memory from a proposal object', async () => {
      const proposalId = createProposalId(UUID_A);
      const sessionId = createSessionId(UUID_B);
      const fakeProposal = {
        id: proposalId,
        type: 'decision' as const,
        status: 'active' as const,
        summary: 'Use factory pattern',
        rationale: 'Observed in 5 files',
        confidence: 0.8,
        signals: [],
        provenance: {
          observation_ids: [UUID_A],
          session_ids: [sessionId],
          earliest_observation: TS,
          latest_observation: TS,
        },
        created_at: TS,
        expires_at: calculateExpiry(TS, 30),
        ttl_days: 30,
      };

      const memory = await port.createMemoryFromProposal(
        {
          proposal_id: proposalId,
          type: 'decision',
          confidence: 'high',
          context: { when: '2024-01-01', why: 'Decision made', conditions: [] },
          promoted_by: 'user@example.com',
          reason: 'Promoting to memory',
        },
        fakeProposal
      );

      expect(memory.status).toBe('active');
      expect(memory.provenance.ember_source?.proposal_id).toBe(proposalId);
    });
  });

  describe('updateMemory', () => {
    it('updates statement and returns updated memory', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      const updated = await port.updateMemory(created.id, { statement: 'Updated statement' });

      expect(updated?.statement).toBe('Updated statement');
      expect(updated?.type).toBe(created.type); // unchanged fields preserved
    });

    it('updates confidence_rationale', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      const updated = await port.updateMemory(created.id, {
        confidence_rationale: 'Explicitly validated',
      });
      expect(updated?.confidence_rationale).toBe('Explicitly validated');
    });

    it('merges context partial update', async () => {
      const created = await port.createMemory(
        makeCreateMemoryInput({ context: { when: '2024-01-01', why: 'original', conditions: [] } })
      );
      const updated = await port.updateMemory(created.id, {
        context: { why: 'updated reason' },
      });
      expect(updated?.context.why).toBe('updated reason');
      expect(updated?.context.when).toBe('2024-01-01'); // preserved
    });

    it('returns null when memory does not exist', async () => {
      const result = await port.updateMemory(createMemoryId(UUID_A), { statement: 'x' });
      expect(result).toBeNull();
    });

    it('persists the update in the store', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      await port.updateMemory(created.id, { statement: 'Persisted' });
      const refetched = await port.getMemory(created.id);
      expect(refetched?.statement).toBe('Persisted');
    });
  });

  describe('retireMemory', () => {
    it('marks memory as retired', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      const retired = await port.retireMemory(created.id, {
        reason: 'No longer applicable',
        retired_by: 'user@example.com',
      });

      expect(retired?.status).toBe('retired');
      expect(retired?.evolution.retired_reason).toBe('No longer applicable');
      expect(retired?.evolution.retired_by).toBe('user@example.com');
    });

    it('returns null when memory does not exist', async () => {
      const result = await port.retireMemory(createMemoryId(UUID_A), {
        reason: 'missing',
        retired_by: 'user@example.com',
      });
      expect(result).toBeNull();
    });

    it('records optional superseded_by link', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      const supersedingId = createMemoryId(UUID_B);
      const retired = await port.retireMemory(created.id, {
        reason: 'Replaced',
        retired_by: 'user@example.com',
        superseded_by: supersedingId,
      });
      expect(retired?.evolution.superseded_by).toBe(supersedingId);
    });
  });

  describe('retireMemoryById', () => {
    it('retires by id without returning the object', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      await port.retireMemoryById(created.id, undefined, 'Obsolete', 'user@example.com');
      const fetched = await port.getMemory(created.id);
      expect(fetched?.status).toBe('retired');
    });

    it('records superseded_by when provided', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      const supersedingId = createMemoryId(UUID_C);
      await port.retireMemoryById(created.id, supersedingId, 'Replaced', 'user@example.com');
      const fetched = await port.getMemory(created.id);
      expect(fetched?.evolution.superseded_by).toBe(supersedingId);
    });
  });

  describe('supersedeMemory', () => {
    it('creates new memory and retires old one', async () => {
      const old = await port.createMemory(makeCreateMemoryInput({ statement: 'Old approach' }));
      const { old: retiredOld, new: newMem } = await port.supersedeMemory(
        old.id,
        makeCreateMemoryInput({ statement: 'New approach' })
      );

      expect(retiredOld.status).toBe('retired');
      expect(newMem.status).toBe('active');
      expect(newMem.statement).toBe('New approach');
      expect(newMem.evolution.supersedes).toContain(old.id);
    });

    it('records superseded_by on old memory', async () => {
      const old = await port.createMemory(makeCreateMemoryInput());
      const { old: retiredOld, new: newMem } = await port.supersedeMemory(
        old.id,
        makeCreateMemoryInput({ statement: 'Replacement' })
      );
      expect(retiredOld.evolution.superseded_by).toBe(newMem.id);
    });
  });

  // ─── read operations ─────────────────────────────────────────────────────────

  describe('getMemory', () => {
    it('retrieves a memory by id', async () => {
      const created = await port.createMemory(makeCreateMemoryInput());
      const fetched = await port.getMemory(created.id);
      expect(fetched?.id).toBe(created.id);
    });

    it('returns null for missing id', async () => {
      expect(await port.getMemory(createMemoryId(UUID_A))).toBeNull();
    });
  });

  describe('getMemoryByProposalId', () => {
    it('returns null when no memory is indexed for proposal id', async () => {
      const result = await port.getMemoryByProposalId(createProposalId(UUID_A));
      expect(result).toBeNull();
    });

    it('returns memory after promote', async () => {
      const proposalId = createProposalId(UUID_A);
      const memory = await port.promoteProposal({
        proposal_id: proposalId,
        type: 'decision',
        confidence: 'high',
        context: { when: '2024-01-01', why: 'Decided', conditions: [] },
        promoted_by: 'u@example.com',
        reason: 'Reason',
      });
      const found = await port.getMemoryByProposalId(proposalId);
      expect(found?.id).toBe(memory.id);
    });
  });

  describe('getActiveMemories', () => {
    it('returns only active memories', async () => {
      const active = await port.createMemory(makeCreateMemoryInput());
      await port.retireMemory(active.id, { reason: 'Done', retired_by: 'u' });
      const active2 = await port.createMemory(makeCreateMemoryInput({ statement: 'Still active' }));

      const results = await port.getActiveMemories();
      expect(results.some((m) => m.id === active2.id)).toBe(true);
      expect(results.every((m) => m.status === 'active')).toBe(true);
    });

    it('returns empty array when store is empty', async () => {
      expect(await port.getActiveMemories()).toEqual([]);
    });
  });

  describe('getMemoriesByType', () => {
    it('returns only active memories of the requested type', async () => {
      await port.createMemory(makeCreateMemoryInput({ type: 'decision' }));
      await port.createMemory(makeCreateMemoryInput({ type: 'pattern', statement: 'A pattern' }));

      const decisions = await port.getMemoriesByType('decision');
      expect(decisions.every((m) => m.type === 'decision')).toBe(true);
      expect(decisions.every((m) => m.status === 'active')).toBe(true);
    });
  });

  describe('searchMemories', () => {
    it('returns memories matching search text case-insensitively', async () => {
      await port.createMemory(makeCreateMemoryInput({ statement: 'Use TypeScript strict mode' }));
      await port.createMemory(
        makeCreateMemoryInput({ statement: 'Avoid any type', confidence: 'medium' as const })
      );

      const results = await port.searchMemories('typescript');
      expect(results).toHaveLength(1);
      expect(results[0].statement).toContain('TypeScript');
    });

    it('returns empty array when no matches', async () => {
      await port.createMemory(makeCreateMemoryInput());
      expect(await port.searchMemories('zzznomatch')).toEqual([]);
    });

    it('excludes retired memories from search', async () => {
      const m = await port.createMemory(makeCreateMemoryInput({ statement: 'Retired content' }));
      await port.retireMemory(m.id, { reason: 'Done', retired_by: 'u' });
      const results = await port.searchMemories('Retired');
      expect(results).toHaveLength(0);
    });
  });

  describe('memoryExists', () => {
    it('returns true for existing memory', async () => {
      const m = await port.createMemory(makeCreateMemoryInput());
      expect(await port.memoryExists(m.id)).toBe(true);
    });

    it('returns false for non-existent memory', async () => {
      expect(await port.memoryExists(createMemoryId(UUID_A))).toBe(false);
    });
  });

  describe('queryMemories', () => {
    it('returns all memories with empty query', async () => {
      await port.createMemory(makeCreateMemoryInput());
      await port.createMemory(makeCreateMemoryInput({ statement: 'Second memory' }));
      const result = await port.queryMemories({});
      expect(result.total).toBe(2);
      expect(result.memories).toHaveLength(2);
    });

    it('filters by type', async () => {
      await port.createMemory(makeCreateMemoryInput({ type: 'decision' }));
      await port.createMemory(makeCreateMemoryInput({ type: 'pattern', statement: 'Pattern' }));
      const result = await port.queryMemories({ types: ['pattern'] });
      expect(result.memories.every((m) => m.type === 'pattern')).toBe(true);
    });

    it('filters by status', async () => {
      const m = await port.createMemory(makeCreateMemoryInput());
      await port.retireMemory(m.id, { reason: 'Done', retired_by: 'u' });
      await port.createMemory(makeCreateMemoryInput({ statement: 'Active memory' }));

      const result = await port.queryMemories({ statuses: ['retired'] });
      expect(result.memories.every((m) => m.status === 'retired')).toBe(true);
    });

    it('filters by confidence_levels', async () => {
      await port.createMemory(makeCreateMemoryInput({ confidence: 'high' as const }));
      await port.createMemory(
        makeCreateMemoryInput({ statement: 'Low conf', confidence: 'low' as const })
      );
      const result = await port.queryMemories({ confidence_levels: ['low'] });
      expect(result.memories.every((m) => m.confidence === 'low')).toBe(true);
    });

    it('filters by tags', async () => {
      await port.createMemory(
        makeCreateMemoryInput({
          context: { when: '2024-01-01', why: 'tagged', conditions: [], tags: ['alpha'] },
        })
      );
      await port.createMemory(
        makeCreateMemoryInput({
          statement: 'Other',
          context: { when: '2024-01-01', why: 'untagged', conditions: [], tags: ['beta'] },
        })
      );
      const result = await port.queryMemories({ tags: ['alpha'] });
      expect(result.memories).toHaveLength(1);
    });

    it('filters by search text', async () => {
      await port.createMemory(makeCreateMemoryInput({ statement: 'TypeScript strict' }));
      await port.createMemory(makeCreateMemoryInput({ statement: 'Python loose' }));
      const result = await port.queryMemories({ search: 'python' });
      expect(result.memories).toHaveLength(1);
    });

    it('excludes superseded by default', async () => {
      const old = await port.createMemory(makeCreateMemoryInput());
      await port.supersedeMemory(old.id, makeCreateMemoryInput({ statement: 'Replacement' }));
      // After supersedeMemory, old is 'retired' not 'superseded' — both should be excluded from active
      const result = await port.queryMemories({});
      expect(result.memories.every((m) => m.status !== 'superseded')).toBe(true);
    });

    it('paginates results with limit and offset', async () => {
      for (let i = 0; i < 5; i++) {
        await port.createMemory(makeCreateMemoryInput({ statement: `Memory ${i}` }));
      }
      const page1 = await port.queryMemories({ limit: 2, offset: 0 });
      const page2 = await port.queryMemories({ limit: 2, offset: 2 });

      expect(page1.memories).toHaveLength(2);
      expect(page2.memories).toHaveLength(2);
      expect(page1.has_more).toBe(true);
      expect(page1.memories[0].id).not.toBe(page2.memories[0].id);
    });

    it('sorts by created_at asc', async () => {
      await port.createMemory(makeCreateMemoryInput({ statement: 'First' }));
      await port.createMemory(makeCreateMemoryInput({ statement: 'Second' }));
      const result = await port.queryMemories({ sort_by: 'created_at', sort_order: 'asc' });
      expect(result.memories[0].statement).toBe('First');
    });

    it('sorts by type', async () => {
      await port.createMemory(makeCreateMemoryInput({ type: 'warning', statement: 'Warning' }));
      await port.createMemory(makeCreateMemoryInput({ type: 'decision', statement: 'Decision' }));
      const result = await port.queryMemories({ sort_by: 'type', sort_order: 'asc' });
      expect(result.memories[0].type).toBe('decision');
    });

    it('returns has_more false when all results fit', async () => {
      await port.createMemory(makeCreateMemoryInput());
      const result = await port.queryMemories({ limit: 10 });
      expect(result.has_more).toBe(false);
    });
  });

  // ─── evolution graph ─────────────────────────────────────────────────────────

  describe('getEvolutionChain', () => {
    it('returns single memory when no supersedes links', async () => {
      const m = await port.createMemory(makeCreateMemoryInput());
      const chain = await port.getEvolutionChain(m.id);
      expect(chain).toHaveLength(1);
      expect(chain[0].id).toBe(m.id);
    });

    it('returns empty array for unknown id', async () => {
      const chain = await port.getEvolutionChain(createMemoryId(UUID_A));
      expect(chain).toEqual([]);
    });

    it('follows supersedes links recursively', async () => {
      const portWithChain = mockEddaWithEvolutionChain();
      const all = portWithChain._getAll();
      const newest = all.find((m) => m.evolution.supersedes.length > 0);
      if (!newest) throw new Error('No newest found in fixture');
      const chain = await portWithChain.getEvolutionChain(newest.id);
      expect(chain.length).toBeGreaterThanOrEqual(2);
    });
  });

  describe('getLatestVersion', () => {
    it('returns the same memory when no superseded_by', async () => {
      const m = await port.createMemory(makeCreateMemoryInput());
      const latest = await port.getLatestVersion(m.id);
      expect(latest?.id).toBe(m.id);
    });

    it('returns null for unknown id', async () => {
      expect(await port.getLatestVersion(createMemoryId(UUID_A))).toBeNull();
    });

    it('follows superseded_by chain to newest', async () => {
      const portWithChain = mockEddaWithEvolutionChain();
      const all = portWithChain._getAll();
      // The oldest has superseded_by pointing to the new one
      const oldest = all.find((m) => m.evolution.superseded_by !== undefined);
      if (!oldest) throw new Error('No oldest found in fixture');
      const latest = await portWithChain.getLatestVersion(oldest.id);
      expect(latest?.evolution.supersedes).toBeDefined();
      expect(latest?.status).toBe('active');
    });
  });

  // ─── provenance ───────────────────────────────────────────────────────────────

  describe('resolveProvenance', () => {
    it('resolves a chain and marks it complete', async () => {
      const chain = makeProvenanceChain();
      const result = await port.resolveProvenance(chain);

      expect(result.complete).toBe(true);
      expect(result.missing_links).toEqual([]);
      expect(result.warnings).toEqual([]);
    });

    it('counts total links correctly (kindling + sessions + ember source)', async () => {
      const chain: ProvenanceChain = {
        ember_source: {
          proposal_id: createProposalId(UUID_A),
          proposal_type: 'decision',
          confidence: 0.8,
          created_at: TS,
        },
        kindling_sources: [
          {
            observation_id: createObservationId(UUID_B),
            session_id: createSessionId(UUID_C),
            kind: 'gate_evaluated',
            timestamp: TS,
          },
        ],
        source_sessions: [createSessionId(UUID_C)],
      };
      const result = await port.resolveProvenance(chain);
      // 1 kindling + 1 session + 1 ember = 3
      expect(result.total_count).toBe(3);
      expect(result.resolved_count).toBe(3);
    });

    it('extracts sessions and observations into resolved_data', async () => {
      const sessionId = createSessionId(UUID_A);
      const obsId = createObservationId(UUID_B);
      const chain: ProvenanceChain = {
        kindling_sources: [
          { observation_id: obsId, session_id: sessionId, kind: 'action_executed', timestamp: TS },
        ],
        source_sessions: [sessionId],
      };
      const result = await port.resolveProvenance(chain);
      expect(result.resolved_data?.sessions).toContain(sessionId);
      expect(result.resolved_data?.observations).toContain(obsId);
    });
  });

  // ─── maintenance / status ─────────────────────────────────────────────────────

  describe('isAvailable', () => {
    it('returns true', async () => {
      expect(await port.isAvailable()).toBe(true);
    });
  });

  describe('getStats', () => {
    it('returns zeroed stats for empty store', async () => {
      const stats = await port.getStats();
      expect(stats.total_memories).toBe(0);
      expect(stats.active_count).toBe(0);
      expect(stats.retired_count).toBe(0);
      expect(stats.superseded_count).toBe(0);
      expect(stats.unique_tags_count).toBe(0);
    });

    it('counts memories by status', async () => {
      await port.createMemory(makeCreateMemoryInput());
      const m2 = await port.createMemory(makeCreateMemoryInput({ statement: 'Will be retired' }));
      await port.retireMemory(m2.id, { reason: 'Done', retired_by: 'u' });

      const stats = await port.getStats();
      expect(stats.total_memories).toBe(2);
      expect(stats.active_count).toBe(1);
      expect(stats.retired_count).toBe(1);
    });

    it('counts unique tags', async () => {
      await port.createMemory(
        makeCreateMemoryInput({
          context: { when: 't', why: 'w', conditions: [], tags: ['ts', 'quality'] },
        })
      );
      await port.createMemory(
        makeCreateMemoryInput({
          statement: 'B',
          context: { when: 't', why: 'w', conditions: [], tags: ['ts'] },
        })
      );
      const stats = await port.getStats();
      expect(stats.unique_tags_count).toBe(2); // 'ts' and 'quality'
    });

    it('populates by_type array', async () => {
      await port.createMemory(makeCreateMemoryInput({ type: 'decision' }));
      await port.createMemory(makeCreateMemoryInput({ type: 'pattern', statement: 'Pattern' }));
      const stats = await port.getStats();
      const decisionStat = stats.by_type.find((s) => s.type === 'decision');
      expect(decisionStat?.count).toBe(1);
    });
  });

  describe('countMemories', () => {
    it('counts all memories with no filter', async () => {
      await port.createMemory(makeCreateMemoryInput());
      await port.createMemory(makeCreateMemoryInput({ statement: 'Second' }));
      expect(await port.countMemories()).toBe(2);
    });

    it('counts by status filter', async () => {
      const m = await port.createMemory(makeCreateMemoryInput());
      await port.retireMemory(m.id, { reason: 'Done', retired_by: 'u' });
      await port.createMemory(makeCreateMemoryInput({ statement: 'Active' }));
      expect(await port.countMemories({ status: 'active' })).toBe(1);
      expect(await port.countMemories({ status: 'retired' })).toBe(1);
    });

    it('counts by type filter', async () => {
      await port.createMemory(makeCreateMemoryInput({ type: 'decision' }));
      await port.createMemory(makeCreateMemoryInput({ type: 'pattern', statement: 'Pattern' }));
      expect(await port.countMemories({ type: 'decision' })).toBe(1);
    });
  });

  describe('exportMemories / importMemories', () => {
    it('exports all memories in the store', async () => {
      await port.createMemory(makeCreateMemoryInput());
      await port.createMemory(makeCreateMemoryInput({ statement: 'Second' }));
      const exported = await port.exportMemories();
      expect(exported).toHaveLength(2);
    });

    it('imports new memories and returns count', async () => {
      const m = await port.createMemory(makeCreateMemoryInput());
      const newMem = { ...m, id: createMemoryId(UUID_B), statement: 'Imported' };

      const count = await port.importMemories([newMem]);
      expect(count).toBe(1);
      expect(await port.memoryExists(newMem.id)).toBe(true);
    });

    it('skips already-existing memories on import', async () => {
      const m = await port.createMemory(makeCreateMemoryInput());
      const count = await port.importMemories([m]); // already exists
      expect(count).toBe(0);
    });

    it('round-trips: export then import into empty port', async () => {
      await port.createMemory(makeCreateMemoryInput());
      const exported = await port.exportMemories();

      const freshPort = createMockEddaPort();
      const count = await freshPort.importMemories(exported);
      expect(count).toBe(exported.length);
      const freshExport = await freshPort.exportMemories();
      expect(freshExport).toHaveLength(exported.length);
    });
  });

  // ─── test utilities ────────────────────────────────────────────────────────────

  describe('_reset', () => {
    it('clears the store and resets mock call counts', async () => {
      await port.createMemory(makeCreateMemoryInput());
      expect(port._store.size).toBe(1);
      expect(port._mocks.createMemory).toHaveBeenCalledOnce();

      port._reset();
      expect(port._store.size).toBe(0);
      expect(port._mocks.createMemory).not.toHaveBeenCalled();
    });

    it('restores initial memories on reset', async () => {
      const sessionId = createSessionId(UUID_A);
      const initial = [
        {
          id: createMemoryId(UUID_A),
          type: 'decision' as const,
          status: 'active' as const,
          schema_version: MEMORY_SCHEMA_VERSION,
          statement: 'Initial memory',
          context: { when: 'today', why: 'testing', conditions: [] },
          confidence: 'high' as const,
          provenance: {
            kindling_sources: [
              {
                observation_id: createObservationId(UUID_B),
                session_id: sessionId,
                kind: 'gate_evaluated' as const,
                timestamp: TS,
              },
            ],
            source_sessions: [sessionId],
          },
          attribution: {
            actor: 'u',
            timestamp: TS,
            method: 'cli_command' as const,
            reason: 'init',
          },
          evolution: { supersedes: [] },
          created_at: TS,
        },
      ];

      const portWithInit = createMockEddaPort({ initialMemories: initial });
      await portWithInit.createMemory(makeCreateMemoryInput({ statement: 'Added after init' }));
      expect(portWithInit._store.size).toBe(2);

      portWithInit._reset();
      expect(portWithInit._store.size).toBe(1);
      expect(portWithInit._store.has(initial[0].id)).toBe(true);
    });
  });

  describe('_getAll', () => {
    it('returns all memories in the store', async () => {
      await port.createMemory(makeCreateMemoryInput());
      await port.createMemory(makeCreateMemoryInput({ statement: 'Second' }));
      expect(port._getAll()).toHaveLength(2);
    });
  });
});

// ─── pre-built scenario helpers ───────────────────────────────────────────────

describe('MockEddaPort pre-built scenarios', () => {
  describe('mockEddaWithMemories', () => {
    it('creates a port with 3 initial memories', () => {
      const p = mockEddaWithMemories();
      expect(p._getAll()).toHaveLength(3);
    });

    it('has memories across multiple types', () => {
      const p = mockEddaWithMemories();
      const types = new Set(p._getAll().map((m) => m.type));
      expect(types.size).toBeGreaterThan(1);
    });

    it('proposal index is populated for ember-sourced memories', () => {
      const p = mockEddaWithMemories();
      // At least one memory with ember_source should exist
      const hasEmberSource = p._getAll().some((m) => m.provenance.ember_source);
      if (hasEmberSource) {
        expect(p._proposalIndex.size).toBeGreaterThan(0);
      }
    });
  });

  describe('mockEddaEmpty', () => {
    it('creates a port with no memories', () => {
      const p = mockEddaEmpty();
      expect(p._getAll()).toHaveLength(0);
    });
  });

  describe('mockEddaWithEvolutionChain', () => {
    it('creates a port with exactly 2 memories (old and new)', () => {
      const p = mockEddaWithEvolutionChain();
      expect(p._getAll()).toHaveLength(2);
    });

    it('has a superseded memory pointing to the active one', () => {
      const p = mockEddaWithEvolutionChain();
      const superseded = p._getAll().find((m) => m.status === 'superseded');
      const active = p._getAll().find((m) => m.status === 'active');
      expect(superseded).toBeDefined();
      expect(active).toBeDefined();
      expect(superseded?.evolution.superseded_by).toBe(active?.id);
    });
  });
});

// =============================================================================
// EMBER MOCK TESTS
// =============================================================================

describe('MockEmberPort — createMockEmberPort (TCOV-015)', () => {
  let port: MockEmberPort;

  beforeEach(() => {
    port = createMockEmberPort();
  });

  describe('createProposal', () => {
    it('creates a proposal with generated id and active status', async () => {
      const input = makeCreateProposalInput();
      const proposal = await port.createProposal(input);

      expect(proposal.id).toBeTruthy();
      expect(proposal.status).toBe('active');
      expect(proposal.type).toBe('pattern');
      expect(proposal.summary).toBe('Factory pattern observed');
      expect(proposal.confidence).toBe(0.75);
    });

    it('sets expires_at based on ttl_days', async () => {
      const proposal = await port.createProposal(makeCreateProposalInput({ ttl_days: 7 }));
      const created = new Date(proposal.created_at).getTime();
      const expires = new Date(proposal.expires_at).getTime();
      const diff = (expires - created) / (1000 * 60 * 60 * 24);
      expect(Math.round(diff)).toBe(7);
    });

    it('records the call on the mock', async () => {
      await port.createProposal(makeCreateProposalInput());
      expect(port._mocks.createProposal).toHaveBeenCalledOnce();
    });

    it('persists the proposal in the store', async () => {
      const p = await port.createProposal(makeCreateProposalInput());
      expect(port._store.has(p.id)).toBe(true);
    });
  });

  describe('updateProposal', () => {
    it('updates summary and rationale', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const updated = await port.updateProposal(created.id, {
        summary: 'Updated summary',
        rationale: 'Updated rationale',
      });
      expect(updated?.summary).toBe('Updated summary');
      expect(updated?.rationale).toBe('Updated rationale');
    });

    it('updates confidence', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const updated = await port.updateProposal(created.id, { confidence: 0.9 });
      expect(updated?.confidence).toBe(0.9);
    });

    it('returns null for unknown proposal', async () => {
      const result = await port.updateProposal(createProposalId(UUID_A), { summary: 'x' });
      expect(result).toBeNull();
    });

    it('persists the update', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      await port.updateProposal(created.id, { summary: 'Persisted update' });
      const fetched = await port.getProposal(created.id);
      expect(fetched?.summary).toBe('Persisted update');
    });
  });

  describe('resolveProposal', () => {
    it('marks proposal as promoted with memory_id', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const memoryId = uuidv4();
      const resolved = await port.resolveProposal(created.id, {
        status: 'promoted',
        resolved_by: 'user@example.com',
        resolution_reason: 'Worth remembering',
        memory_id: memoryId as MemoryId,
      });
      expect(resolved?.status).toBe('promoted');
      expect(resolved?.resolution?.memory_id).toBe(memoryId);
    });

    it('marks proposal as dismissed', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const resolved = await port.resolveProposal(created.id, {
        status: 'dismissed',
        resolution_reason: 'Not relevant',
      });
      expect(resolved?.status).toBe('dismissed');
    });

    it('returns null for unknown proposal', async () => {
      const result = await port.resolveProposal(createProposalId(UUID_A), { status: 'expired' });
      expect(result).toBeNull();
    });
  });

  describe('getProposal', () => {
    it('returns proposal by id', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const fetched = await port.getProposal(created.id);
      expect(fetched?.id).toBe(created.id);
    });

    it('returns null for unknown id', async () => {
      expect(await port.getProposal(createProposalId(UUID_A))).toBeNull();
    });
  });

  describe('getActiveProposals', () => {
    it('returns only active non-expired proposals', async () => {
      // Create a far-future proposal (active)
      await port.createProposal(makeCreateProposalInput({ ttl_days: 365 }));
      const results = await port.getActiveProposals();
      expect(results.every((p) => p.status === 'active')).toBe(true);
    });

    it('returns empty array for empty store', async () => {
      expect(await port.getActiveProposals()).toEqual([]);
    });
  });

  describe('getProposalsBySession', () => {
    it('returns proposals for a given session id', async () => {
      const sessionId = createSessionId(UUID_A);
      await port.createProposal(makeCreateProposalInput());
      // the default proposal has session UUID_A in provenance
      const results = await port.getProposalsBySession(sessionId);
      expect(results.length).toBeGreaterThanOrEqual(1);
    });

    it('returns empty array when no proposals for session', async () => {
      const otherSession = createSessionId(UUID_B);
      await port.createProposal(makeCreateProposalInput());
      const results = await port.getProposalsBySession(otherSession);
      expect(results).toHaveLength(0);
    });
  });

  describe('proposalExists', () => {
    it('returns true for existing proposal', async () => {
      const p = await port.createProposal(makeCreateProposalInput());
      expect(await port.proposalExists(p.id)).toBe(true);
    });

    it('returns false for non-existent proposal', async () => {
      expect(await port.proposalExists(createProposalId(UUID_A))).toBe(false);
    });
  });

  describe('markPromoted', () => {
    it('marks proposal status as promoted', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const memoryId = createMemoryId(UUID_B);
      await port.markPromoted(created.id, memoryId, 'user@example.com');
      const fetched = await port.getProposal(created.id);
      expect(fetched?.status).toBe('promoted');
      expect(fetched?.resolution?.memory_id).toBe(memoryId);
      expect(fetched?.resolution?.resolved_by).toBe('user@example.com');
    });

    it('is a no-op for unknown proposal id', async () => {
      await expect(
        port.markPromoted(createProposalId(UUID_A), createMemoryId(UUID_B), 'u')
      ).resolves.toBeUndefined();
    });
  });

  describe('markDismissed', () => {
    it('marks proposal status as dismissed with reason', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      await port.markDismissed(created.id, 'Not applicable', 'user@example.com');
      const fetched = await port.getProposal(created.id);
      expect(fetched?.status).toBe('dismissed');
      expect(fetched?.resolution?.resolution_reason).toBe('Not applicable');
    });

    it('is a no-op for unknown proposal id', async () => {
      await expect(
        port.markDismissed(createProposalId(UUID_A), 'reason', 'u')
      ).resolves.toBeUndefined();
    });
  });

  describe('getExpiredProposals', () => {
    it('returns active proposals past their expiry', async () => {
      // Create a proposal that is already past expiry by using a past expires_at
      const past = new Date(Date.now() - 86400000).toISOString() as Timestamp;
      const created = await port.createProposal(makeCreateProposalInput());
      // Manually mutate expires_at in the store to simulate expiry
      const stored = port._store.get(created.id)!;
      port._store.set(created.id, { ...stored, expires_at: past });

      const expired = await port.getExpiredProposals();
      expect(expired.some((p) => p.id === created.id)).toBe(true);
    });

    it('returns empty array when no proposals have expired', async () => {
      await port.createProposal(makeCreateProposalInput({ ttl_days: 365 }));
      expect(await port.getExpiredProposals()).toHaveLength(0);
    });
  });

  describe('processExpiredProposals', () => {
    it('marks expired proposals and returns count', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const past = new Date(Date.now() - 86400000).toISOString() as Timestamp;
      port._store.set(created.id, { ...port._store.get(created.id)!, expires_at: past });

      const count = await port.processExpiredProposals();
      expect(count).toBe(1);
      const fetched = await port.getProposal(created.id);
      expect(fetched?.status).toBe('expired');
    });

    it('returns 0 when nothing to expire', async () => {
      await port.createProposal(makeCreateProposalInput({ ttl_days: 365 }));
      expect(await port.processExpiredProposals()).toBe(0);
    });
  });

  describe('expireStaleProposals', () => {
    it('mirrors processExpiredProposals behaviour', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      const past = new Date(Date.now() - 86400000).toISOString() as Timestamp;
      port._store.set(created.id, { ...port._store.get(created.id)!, expires_at: past });

      const count = await port.expireStaleProposals();
      expect(count).toBe(1);
    });
  });

  describe('queryProposals', () => {
    it('returns all proposals with empty query', async () => {
      await port.createProposal(makeCreateProposalInput());
      await port.createProposal(
        makeCreateProposalInput({ summary: 'Second', type: 'decision' as const })
      );
      const result = await port.queryProposals({});
      expect(result.total).toBe(2);
    });

    it('filters by types', async () => {
      await port.createProposal(makeCreateProposalInput({ type: 'pattern' as const }));
      await port.createProposal(
        makeCreateProposalInput({ summary: 'Dec', type: 'decision' as const })
      );
      const result = await port.queryProposals({ types: ['pattern'] });
      expect(result.proposals.every((p) => p.type === 'pattern')).toBe(true);
    });

    it('filters by statuses', async () => {
      const p = await port.createProposal(makeCreateProposalInput());
      await port.markDismissed(p.id, 'reason', 'u');
      await port.createProposal(makeCreateProposalInput({ summary: 'Active' }));
      const result = await port.queryProposals({ statuses: ['dismissed'] });
      expect(result.proposals.every((p) => p.status === 'dismissed')).toBe(true);
    });

    it('filters by min_confidence', async () => {
      await port.createProposal(makeCreateProposalInput({ confidence: 0.3 }));
      await port.createProposal(makeCreateProposalInput({ confidence: 0.9, summary: 'High conf' }));
      const result = await port.queryProposals({ min_confidence: 0.8 });
      expect(result.proposals.every((p) => p.confidence >= 0.8)).toBe(true);
    });

    it('excludes expired by default', async () => {
      const p = await port.createProposal(makeCreateProposalInput());
      port._store.set(p.id, { ...port._store.get(p.id)!, status: 'expired' });
      await port.createProposal(makeCreateProposalInput({ summary: 'Active' }));
      const result = await port.queryProposals({});
      expect(result.proposals.every((p) => p.status !== 'expired')).toBe(true);
    });

    it('includes expired when include_expired is true', async () => {
      const p = await port.createProposal(makeCreateProposalInput());
      port._store.set(p.id, { ...port._store.get(p.id)!, status: 'expired' });
      const result = await port.queryProposals({ include_expired: true });
      expect(result.proposals.some((p) => p.status === 'expired')).toBe(true);
    });

    it('filters by session_id', async () => {
      const sessionA = createSessionId(UUID_A);
      const sessionB = createSessionId(UUID_B);
      await port.createProposal(makeCreateProposalInput()); // uses UUID_A
      await port.createProposal({
        ...makeCreateProposalInput({ summary: 'Other session' }),
        provenance: {
          observation_ids: [UUID_C],
          session_ids: [sessionB],
          earliest_observation: TS,
          latest_observation: TS,
        },
      });
      const result = await port.queryProposals({ session_id: sessionA });
      expect(result.proposals.every((p) => p.provenance.session_ids.includes(sessionA))).toBe(true);
    });

    it('sorts by confidence desc', async () => {
      await port.createProposal(makeCreateProposalInput({ confidence: 0.3 }));
      await port.createProposal(makeCreateProposalInput({ confidence: 0.9, summary: 'High' }));
      const result = await port.queryProposals({ sort_by: 'confidence', sort_order: 'desc' });
      expect(result.proposals[0].confidence).toBeGreaterThan(result.proposals[1].confidence);
    });

    it('paginates results', async () => {
      for (let i = 0; i < 5; i++) {
        await port.createProposal(makeCreateProposalInput({ summary: `Summary ${i}` }));
      }
      const page = await port.queryProposals({ limit: 2, offset: 0 });
      expect(page.proposals).toHaveLength(2);
      expect(page.has_more).toBe(true);
    });
  });

  describe('countProposals', () => {
    it('counts all proposals', async () => {
      await port.createProposal(makeCreateProposalInput());
      await port.createProposal(makeCreateProposalInput({ summary: 'Second' }));
      expect(await port.countProposals()).toBe(2);
    });

    it('counts by status', async () => {
      const p = await port.createProposal(makeCreateProposalInput());
      await port.markDismissed(p.id, 'reason', 'u');
      await port.createProposal(makeCreateProposalInput({ summary: 'Active' }));
      expect(await port.countProposals('active')).toBe(1);
      expect(await port.countProposals('dismissed')).toBe(1);
    });
  });

  describe('pruneProposals', () => {
    it('removes resolved proposals older than threshold', async () => {
      const created = await port.createProposal(makeCreateProposalInput());
      await port.markDismissed(created.id, 'reason', 'u');

      // Future threshold — should prune anything resolved before now
      const future = new Date(Date.now() + 86400000).toISOString() as Timestamp;
      const count = await port.pruneProposals(future);
      expect(count).toBeGreaterThan(0);
      expect(await port.proposalExists(created.id)).toBe(false);
    });

    it('does not prune active proposals', async () => {
      await port.createProposal(makeCreateProposalInput());
      const future = new Date(Date.now() + 86400000).toISOString() as Timestamp;
      await port.pruneProposals(future);
      expect(await port.countProposals('active')).toBe(1);
    });
  });

  describe('isAvailable', () => {
    it('returns true', async () => {
      expect(await port.isAvailable()).toBe(true);
    });
  });

  describe('getStats', () => {
    it('returns zeroed stats for empty store', async () => {
      const stats = await port.getStats();
      expect(stats.total_proposals).toBe(0);
      expect(stats.expiring_soon).toBe(0);
    });

    it('counts by status correctly', async () => {
      await port.createProposal(makeCreateProposalInput({ ttl_days: 365 }));
      const p = await port.createProposal(
        makeCreateProposalInput({ summary: 'Will be dismissed' })
      );
      await port.markDismissed(p.id, 'reason', 'u');

      const stats = await port.getStats();
      expect(stats.total_proposals).toBe(2);
      const activeStatus = stats.by_status.find((s) => s.status === 'active');
      const dismissedStatus = stats.by_status.find((s) => s.status === 'dismissed');
      expect(activeStatus?.count).toBe(1);
      expect(dismissedStatus?.count).toBe(1);
    });

    it('computes avg_confidence for active proposals', async () => {
      await port.createProposal(makeCreateProposalInput({ confidence: 0.6, ttl_days: 365 }));
      await port.createProposal(
        makeCreateProposalInput({ confidence: 0.8, summary: 'High', ttl_days: 365 })
      );
      const stats = await port.getStats();
      expect(stats.avg_confidence).toBeCloseTo(0.7, 5);
    });

    it('computes promotion_rate', async () => {
      const p1 = await port.createProposal(makeCreateProposalInput());
      await port.markPromoted(p1.id, createMemoryId(UUID_C), 'u');
      const p2 = await port.createProposal(makeCreateProposalInput({ summary: 'Dismissed' }));
      await port.markDismissed(p2.id, 'reason', 'u');

      const stats = await port.getStats();
      // 1 promoted / (1 promoted + 1 dismissed) = 0.5
      expect(stats.promotion_rate).toBeCloseTo(0.5, 5);
    });
  });

  describe('_reset', () => {
    it('clears the store and resets mock calls', async () => {
      await port.createProposal(makeCreateProposalInput());
      port._reset();
      expect(port._store.size).toBe(0);
      expect(port._mocks.createProposal).not.toHaveBeenCalled();
    });
  });

  describe('_getAll', () => {
    it('returns all proposals', async () => {
      await port.createProposal(makeCreateProposalInput());
      await port.createProposal(makeCreateProposalInput({ summary: 'Second' }));
      expect(port._getAll()).toHaveLength(2);
    });
  });
});

// ─── pre-built scenario helpers (Ember) ────────────────────────────────────────

describe('MockEmberPort pre-built scenarios', () => {
  describe('mockEmberWithProposals', () => {
    it('creates a port with 3 initial proposals', () => {
      const p = mockEmberWithProposals();
      expect(p._getAll()).toHaveLength(3);
    });

    it('all initial proposals are active', () => {
      const p = mockEmberWithProposals();
      expect(p._getAll().every((pr) => pr.status === 'active')).toBe(true);
    });
  });

  describe('mockEmberEmpty', () => {
    it('creates a port with no proposals', () => {
      expect(mockEmberEmpty()._getAll()).toHaveLength(0);
    });
  });

  describe('mockEmberWithMixedStatuses', () => {
    it('has proposals in at least 3 different statuses', () => {
      const p = mockEmberWithMixedStatuses();
      const statuses = new Set(p._getAll().map((pr) => pr.status));
      expect(statuses.size).toBeGreaterThanOrEqual(3);
    });
  });
});

// =============================================================================
// KINDLING MOCK TESTS
// =============================================================================

describe('MockKindlingPort — createMockKindlingPort (TCOV-015)', () => {
  let port: MockKindlingPort;

  beforeEach(() => {
    port = createMockKindlingPort();
  });

  describe('createObservation', () => {
    it('creates an observation with generated id', async () => {
      const input = makeCreateObservationInput();
      const obs = await port.createObservation(input);

      expect(obs.id).toBeTruthy();
      expect(obs.kind).toBe('gate_evaluated');
      expect(obs.summary).toBe('Architecture gate passed');
      expect(obs.session_id).toBe(createSessionId(UUID_A));
    });

    it('persists observation in the store', async () => {
      const obs = await port.createObservation(makeCreateObservationInput());
      expect(port._store.has(obs.id)).toBe(true);
    });

    it('records the mock call', async () => {
      await port.createObservation(makeCreateObservationInput());
      expect(port._mocks.createObservation).toHaveBeenCalledOnce();
    });

    it('includes optional tags', async () => {
      const obs = await port.createObservation(
        makeCreateObservationInput({ tags: ['gate', 'arch'] })
      );
      expect(obs.tags).toEqual(['gate', 'arch']);
    });
  });

  describe('createObservationBatch', () => {
    it('creates multiple observations', async () => {
      const inputs = [
        makeCreateObservationInput({ summary: 'Obs 1' }),
        makeCreateObservationInput({ summary: 'Obs 2', kind: 'action_executed' as const }),
      ];
      const results = await port.createObservationBatch(inputs);
      expect(results).toHaveLength(2);
      expect(results[0].summary).toBe('Obs 1');
      expect(results[1].summary).toBe('Obs 2');
    });

    it('returns empty array for empty input', async () => {
      expect(await port.createObservationBatch([])).toEqual([]);
    });

    it('all created observations are persisted', async () => {
      const results = await port.createObservationBatch([
        makeCreateObservationInput(),
        makeCreateObservationInput({ summary: 'B' }),
      ]);
      for (const obs of results) {
        expect(port._store.has(obs.id)).toBe(true);
      }
    });
  });

  describe('getObservation', () => {
    it('retrieves observation by id', async () => {
      const created = await port.createObservation(makeCreateObservationInput());
      const fetched = await port.getObservation(created.id);
      expect(fetched?.id).toBe(created.id);
    });

    it('returns null for unknown id', async () => {
      expect(await port.getObservation(createObservationId(UUID_A))).toBeNull();
    });
  });

  describe('observationExists', () => {
    it('returns true for existing observation', async () => {
      const obs = await port.createObservation(makeCreateObservationInput());
      expect(await port.observationExists(obs.id)).toBe(true);
    });

    it('returns false for non-existent observation', async () => {
      expect(await port.observationExists(createObservationId(UUID_A))).toBe(false);
    });
  });

  describe('queryObservations', () => {
    it('returns all observations with empty query', async () => {
      await port.createObservation(makeCreateObservationInput());
      await port.createObservation(makeCreateObservationInput({ summary: 'Second' }));
      const result = await port.queryObservations({});
      expect(result.total).toBe(2);
    });

    it('filters by session_id', async () => {
      const sessionA = createSessionId(UUID_A);
      const sessionB = createSessionId(UUID_B);
      await port.createObservation(makeCreateObservationInput({ session_id: sessionA }));
      await port.createObservation(
        makeCreateObservationInput({ session_id: sessionB, summary: 'B session' })
      );
      const result = await port.queryObservations({ session_id: sessionA });
      expect(result.observations.every((o) => o.session_id === sessionA)).toBe(true);
    });

    it('filters by kinds', async () => {
      await port.createObservation(makeCreateObservationInput({ kind: 'gate_evaluated' as const }));
      await port.createObservation(
        makeCreateObservationInput({ kind: 'action_executed' as const, summary: 'Action' })
      );
      const result = await port.queryObservations({ kinds: ['action_executed'] });
      expect(result.observations.every((o) => o.kind === 'action_executed')).toBe(true);
    });

    it('filters by time_range', async () => {
      const obs1 = await port.createObservation(makeCreateObservationInput());
      // Store a specific timestamp
      const pastTs = '2020-01-01T00:00:00.000Z' as Timestamp;
      port._store.set(obs1.id, { ...port._store.get(obs1.id)!, timestamp: pastTs });
      await port.createObservation(makeCreateObservationInput({ summary: 'Recent' }));

      const result = await port.queryObservations({
        time_range: { start: '2023-01-01T00:00:00.000Z' as Timestamp },
      });
      // Only the recent one should match
      expect(result.observations.every((o) => o.summary !== obs1.summary)).toBe(true);
    });

    it('filters by tags', async () => {
      await port.createObservation(makeCreateObservationInput({ tags: ['gate'] }));
      await port.createObservation(
        makeCreateObservationInput({ tags: ['action'], summary: 'Tagged action' })
      );
      const result = await port.queryObservations({ tags: ['gate'] });
      expect(result.observations.every((o) => o.tags?.includes('gate'))).toBe(true);
    });

    it('paginates results', async () => {
      for (let i = 0; i < 5; i++) {
        await port.createObservation(makeCreateObservationInput({ summary: `Obs ${i}` }));
      }
      const page = await port.queryObservations({ limit: 2, offset: 0 });
      expect(page.observations).toHaveLength(2);
      expect(page.has_more).toBe(true);
    });

    it('returns has_more false when all fit', async () => {
      await port.createObservation(makeCreateObservationInput());
      const result = await port.queryObservations({ limit: 10 });
      expect(result.has_more).toBe(false);
    });
  });

  describe('getSessionObservations', () => {
    it('returns observations for a session', async () => {
      const sid = createSessionId(UUID_A);
      await port.createObservation(makeCreateObservationInput({ session_id: sid }));
      await port.createObservation(
        makeCreateObservationInput({ session_id: createSessionId(UUID_B), summary: 'Other' })
      );
      const results = await port.getSessionObservations(sid);
      expect(results.every((o) => o.session_id === sid)).toBe(true);
    });

    it('returns empty for unknown session', async () => {
      expect(await port.getSessionObservations(createSessionId(UUID_A))).toEqual([]);
    });
  });

  describe('getObservationsBySession (STACK-007)', () => {
    it('is an alias for getSessionObservations', async () => {
      const sid = createSessionId(UUID_A);
      await port.createObservation(makeCreateObservationInput({ session_id: sid }));
      const bySession = await port.getSessionObservations(sid);
      const byMethod = await port.getObservationsBySession(sid);
      expect(byMethod.map((o) => o.id)).toEqual(bySession.map((o) => o.id));
    });
  });

  describe('querySession (STACK-007)', () => {
    it('returns observations for the given session', async () => {
      const sid = createSessionId(UUID_A);
      await port.createObservation(makeCreateObservationInput({ session_id: sid }));
      const result = await port.querySession(sid);
      expect(result.session_id).toBe(sid);
      expect(result.observations.every((o) => o.session_id === sid)).toBe(true);
    });

    it('populates session_metadata with start/end timestamps', async () => {
      const sid = createSessionId(UUID_A);
      await port.createObservation(makeCreateObservationInput({ session_id: sid }));
      const result = await port.querySession(sid);
      expect(result.session_metadata?.started_at).toBeDefined();
      expect(result.session_metadata?.ended_at).toBeDefined();
    });

    it('returns empty session_metadata for empty session', async () => {
      const result = await port.querySession(createSessionId(UUID_A));
      expect(result.session_metadata).toBeUndefined();
    });

    it('filters by kinds option', async () => {
      const sid = createSessionId(UUID_A);
      await port.createObservation(
        makeCreateObservationInput({ session_id: sid, kind: 'gate_evaluated' as const })
      );
      await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          kind: 'action_executed' as const,
          summary: 'Action',
        })
      );
      const result = await port.querySession(sid, { kinds: ['gate_evaluated'] });
      expect(result.observations.every((o) => o.kind === 'gate_evaluated')).toBe(true);
    });

    it('strips payloads when include_payloads is false', async () => {
      const sid = createSessionId(UUID_A);
      await port.createObservation(
        makeCreateObservationInput({ session_id: sid, data: { secret: 'data' } })
      );
      const result = await port.querySession(sid, { include_payloads: false });
      expect(result.observations.every((o) => Object.keys(o.data).length === 0)).toBe(true);
    });

    it('sorts observations by timestamp asc', async () => {
      const sid = createSessionId(UUID_A);
      const obs1 = await port.createObservation(
        makeCreateObservationInput({ session_id: sid, summary: 'First' })
      );
      const obs2 = await port.createObservation(
        makeCreateObservationInput({ session_id: sid, summary: 'Second' })
      );
      // Give obs2 a later timestamp
      port._store.set(obs2.id, {
        ...port._store.get(obs2.id)!,
        timestamp: new Date(Date.now() + 1000).toISOString() as Timestamp,
      });

      const result = await port.querySession(sid, { sort_order: 'asc' });
      expect(result.observations[0].id).toBe(obs1.id);
    });
  });

  describe('queryByPlan (STACK-007)', () => {
    it('returns plan query result with empty sessions for no matching observations', async () => {
      const planId = 'plan-001' as ReturnType<typeof createSessionId>;
      const result = await port.queryByPlan(
        planId as unknown as Parameters<typeof port.queryByPlan>[0]
      );
      expect(result.sessions).toHaveLength(0);
      expect(result.total_sessions).toBe(0);
      expect(result.has_more).toBe(false);
    });

    it('groups observations by session and returns summaries', async () => {
      const sid = createSessionId(UUID_A);
      const planId = 'plan-abc';
      await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          data: { plan: planId },
        })
      );
      await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          summary: 'Second obs for plan',
          data: { plan: planId },
        })
      );

      const result = await port.queryByPlan(
        planId as unknown as Parameters<typeof port.queryByPlan>[0]
      );
      expect(result.sessions).toHaveLength(1);
      expect(result.sessions[0].observation_count).toBe(2);
      expect(result.sessions[0].session_id).toBe(sid);
    });

    it('includes observations when include_observations is true', async () => {
      const sid = createSessionId(UUID_A);
      const planId = 'plan-xyz';
      await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          data: { plan: planId },
        })
      );

      const result = await port.queryByPlan(
        planId as unknown as Parameters<typeof port.queryByPlan>[0],
        { include_observations: true }
      );
      expect(result.observations).toBeDefined();
      expect(result.observations?.length).toBeGreaterThan(0);
    });
  });

  describe('getObservationsByTimeRange (STACK-007)', () => {
    it('returns observations within time range', async () => {
      // Force timestamps into the store so the time-range filter is predictable.
      // The range end defaults to Date.now() (exclusive), so we use explicit past ts values.
      const recentTs = '2025-01-01T00:00:00.000Z' as Timestamp;
      const oldTs = '2020-01-01T00:00:00.000Z' as Timestamp;
      const future = new Date(Date.now() + 5000).toISOString() as Timestamp;

      const recent = await port.createObservation(makeCreateObservationInput());
      port._store.set(recent.id, { ...port._store.get(recent.id)!, timestamp: recentTs });
      const old = await port.createObservation(makeCreateObservationInput({ summary: 'Old obs' }));
      port._store.set(old.id, { ...port._store.get(old.id)!, timestamp: oldTs });

      const results = await port.getObservationsByTimeRange({
        start: '2023-01-01T00:00:00.000Z' as Timestamp,
        end: future,
      });
      expect(results.some((o) => o.id === recent.id)).toBe(true);
      expect(results.every((o) => o.id !== old.id)).toBe(true);
    });

    it('returns all observations when range covers all', async () => {
      const ts = '2024-06-01T12:00:00.000Z' as Timestamp;
      const obs = await port.createObservation(makeCreateObservationInput());
      port._store.set(obs.id, { ...port._store.get(obs.id)!, timestamp: ts });

      const results = await port.getObservationsByTimeRange({
        start: '2000-01-01T00:00:00.000Z' as Timestamp,
        end: '2030-01-01T00:00:00.000Z' as Timestamp,
      });
      expect(results).toHaveLength(1);
    });
  });

  describe('getObservationsAsRefs', () => {
    it('converts observations to KindlingRef format', async () => {
      const obs = await port.createObservation(makeCreateObservationInput());
      const refs = await port.getObservationsAsRefs([obs.id]);
      expect(refs).toHaveLength(1);
      expect(refs[0].observation_id).toBe(obs.id);
      expect(refs[0].session_id).toBe(obs.session_id);
      expect(refs[0].kind).toBe(obs.kind);
      expect(refs[0].timestamp).toBe(obs.timestamp);
    });

    it('skips unknown ids', async () => {
      const refs = await port.getObservationsAsRefs([createObservationId(UUID_A)]);
      expect(refs).toHaveLength(0);
    });

    it('returns empty array for empty input', async () => {
      expect(await port.getObservationsAsRefs([])).toEqual([]);
    });
  });

  describe('countObservations', () => {
    it('counts all observations', async () => {
      await port.createObservation(makeCreateObservationInput());
      await port.createObservation(makeCreateObservationInput({ summary: 'Second' }));
      expect(await port.countObservations()).toBe(2);
    });

    it('counts by session id', async () => {
      const sid = createSessionId(UUID_A);
      await port.createObservation(makeCreateObservationInput({ session_id: sid }));
      await port.createObservation(
        makeCreateObservationInput({ session_id: createSessionId(UUID_B), summary: 'Other' })
      );
      expect(await port.countObservations(sid)).toBe(1);
    });
  });

  describe('pruneObservations', () => {
    it('removes observations older than threshold', async () => {
      const obs = await port.createObservation(makeCreateObservationInput());
      // Force a past timestamp
      port._store.set(obs.id, {
        ...port._store.get(obs.id)!,
        timestamp: '2020-01-01T00:00:00.000Z' as Timestamp,
      });

      const future = new Date(Date.now() + 86400000).toISOString() as Timestamp;
      const count = await port.pruneObservations(future);
      expect(count).toBe(1);
      expect(await port.observationExists(obs.id)).toBe(false);
    });

    it('keeps recent observations', async () => {
      await port.createObservation(makeCreateObservationInput());
      const past = '2000-01-01T00:00:00.000Z' as Timestamp;
      const count = await port.pruneObservations(past);
      expect(count).toBe(0);
      expect(port._store.size).toBe(1);
    });
  });

  describe('isAvailable', () => {
    it('returns true', async () => {
      expect(await port.isAvailable()).toBe(true);
    });
  });

  describe('_reset', () => {
    it('clears the store and resets mock calls', async () => {
      await port.createObservation(makeCreateObservationInput());
      port._reset();
      expect(port._store.size).toBe(0);
      expect(port._mocks.createObservation).not.toHaveBeenCalled();
    });

    it('restores initial observations on reset', async () => {
      const initial: import('../../contracts/ports/kindling.port.js').Observation[] = [
        {
          id: createObservationId(UUID_A),
          session_id: createSessionId(UUID_B),
          kind: 'gate_evaluated',
          timestamp: TS,
          summary: 'Initial obs',
          data: { initial: true },
        },
      ];
      const portWithInit = createMockKindlingPort({ initialObservations: initial });
      await portWithInit.createObservation(makeCreateObservationInput({ summary: 'Added' }));
      expect(portWithInit._store.size).toBe(2);

      portWithInit._reset();
      expect(portWithInit._store.size).toBe(1);
      expect(portWithInit._store.has(initial[0].id)).toBe(true);
    });
  });

  describe('_getAll', () => {
    it('returns all observations in the store', async () => {
      await port.createObservation(makeCreateObservationInput());
      await port.createObservation(makeCreateObservationInput({ summary: 'Second' }));
      expect(port._getAll()).toHaveLength(2);
    });
  });
});

// ─── pre-built scenario helpers (Kindling) ──────────────────────────────────────

describe('MockKindlingPort pre-built scenarios', () => {
  describe('mockKindlingWithObservations', () => {
    it('creates a port with 4 initial observations', () => {
      const p = mockKindlingWithObservations();
      expect(p._getAll()).toHaveLength(4);
    });

    it('all observations share the same session', () => {
      const p = mockKindlingWithObservations();
      const sessionIds = new Set(p._getAll().map((o) => o.session_id));
      expect(sessionIds.size).toBe(1);
    });

    it('has multiple observation kinds', () => {
      const p = mockKindlingWithObservations();
      const kinds = new Set(p._getAll().map((o) => o.kind));
      expect(kinds.size).toBeGreaterThan(1);
    });
  });

  describe('mockKindlingEmpty', () => {
    it('creates a port with no observations', () => {
      expect(mockKindlingEmpty()._getAll()).toHaveLength(0);
    });
  });

  describe('mockKindlingMultipleSessions', () => {
    it('creates a port with observations from 2 sessions', () => {
      const p = mockKindlingMultipleSessions();
      const sessions = new Set(p._getAll().map((o) => o.session_id));
      expect(sessions.size).toBe(2);
    });
  });
});

// =============================================================================
// Branch coverage boosters — uncovered paths identified by v8 coverage
// =============================================================================

describe('MockEddaPort — branch coverage (TCOV-015)', () => {
  describe('queryMemories sort_by updated_at', () => {
    it('sorts by updated_at when memories have been updated', async () => {
      const port = createMockEddaPort();
      await port.createMemory(makeCreateMemoryInput({ statement: 'First' }));
      const m2 = await port.createMemory(makeCreateMemoryInput({ statement: 'Second' }));
      // Update m2 so it has an updated_at
      await port.updateMemory(m2.id, { statement: 'Second (updated)' });

      const result = await port.queryMemories({ sort_by: 'updated_at', sort_order: 'desc' });
      // m2 now has updated_at, m1 doesn't — m2 should come first in desc
      expect(result.memories[0].id).toBe(m2.id);
    });

    it('handles memories without updated_at (treated as 0)', async () => {
      const port = createMockEddaPort();
      await port.createMemory(makeCreateMemoryInput({ statement: 'Never updated' }));
      const result = await port.queryMemories({ sort_by: 'updated_at', sort_order: 'asc' });
      // Single memory — just verifies the branch is reached without error
      expect(result.memories).toHaveLength(1);
    });
  });

  describe('importMemories with ember_source', () => {
    it('indexes proposal_id from imported memories with ember_source', async () => {
      const port = createMockEddaPort();
      const proposalId = createProposalId(UUID_A);
      const sessionId = createSessionId(UUID_B);
      const memoryWithEmber = {
        id: createMemoryId(UUID_C),
        type: 'decision' as const,
        status: 'active' as const,
        schema_version: MEMORY_SCHEMA_VERSION,
        statement: 'Imported with ember source',
        context: { when: 'now', why: 'test', conditions: [] as string[] },
        confidence: 'high' as const,
        provenance: {
          ember_source: {
            proposal_id: proposalId,
            proposal_type: 'decision',
            confidence: 0.85,
            created_at: TS,
          },
          kindling_sources: [
            {
              observation_id: createObservationId(UUID_B),
              session_id: sessionId,
              kind: 'gate_evaluated' as const,
              timestamp: TS,
            },
          ],
          source_sessions: [sessionId],
        },
        attribution: {
          actor: 'u',
          timestamp: TS,
          method: 'cli_command' as const,
          reason: 'import',
        },
        evolution: { supersedes: [] as string[] },
        created_at: TS,
      };

      await port.importMemories([
        memoryWithEmber as Parameters<typeof port.importMemories>[0][number],
      ]);
      const found = await port.getMemoryByProposalId(proposalId);
      expect(found?.id).toBe(memoryWithEmber.id);
    });
  });

  describe('_reset restores ember_source index', () => {
    it('repopulates the proposal index after reset', async () => {
      const proposalId = createProposalId(UUID_A);
      const sessionId = createSessionId(UUID_B);
      const initial = [
        {
          id: createMemoryId(UUID_C),
          type: 'decision' as const,
          status: 'active' as const,
          schema_version: MEMORY_SCHEMA_VERSION,
          statement: 'Memory with ember source',
          context: { when: 'now', why: 'test', conditions: [] as string[] },
          confidence: 'high' as const,
          provenance: {
            ember_source: {
              proposal_id: proposalId,
              proposal_type: 'decision',
              confidence: 0.85,
              created_at: TS,
            },
            kindling_sources: [
              {
                observation_id: createObservationId(UUID_B),
                session_id: sessionId,
                kind: 'gate_evaluated' as const,
                timestamp: TS,
              },
            ],
            source_sessions: [sessionId],
          },
          attribution: {
            actor: 'u',
            timestamp: TS,
            method: 'cli_command' as const,
            reason: 'test',
          },
          evolution: { supersedes: [] as string[] },
          created_at: TS,
        },
      ];

      const port = createMockEddaPort({
        initialMemories: initial as Parameters<typeof createMockEddaPort>[0]['initialMemories'],
      });
      // Clear the index by adding a memory then resetting
      await port.createMemory(makeCreateMemoryInput({ statement: 'Added' }));
      port._reset();

      // After reset, the ember_source index should be restored
      expect(port._proposalIndex.has(proposalId)).toBe(true);
    });
  });
});

describe('MockEmberPort — branch coverage (TCOV-015)', () => {
  describe('queryProposals created_before filter', () => {
    it('excludes proposals created after the before-timestamp', async () => {
      const port = createMockEmberPort();
      const p = await port.createProposal(makeCreateProposalInput());
      // Force a past created_at
      const pastTs = '2020-01-01T00:00:00.000Z' as Timestamp;
      port._store.set(p.id, { ...port._store.get(p.id)!, created_at: pastTs });

      // This proposal was created in 2020; filter for things before 2019 — should exclude it
      const result = await port.queryProposals({
        created_before: '2019-01-01T00:00:00.000Z' as Timestamp,
      });
      expect(result.proposals).toHaveLength(0);
    });

    it('includes proposals created before the threshold', async () => {
      const port = createMockEmberPort();
      const p = await port.createProposal(makeCreateProposalInput());
      const pastTs = '2020-01-01T00:00:00.000Z' as Timestamp;
      port._store.set(p.id, { ...port._store.get(p.id)!, created_at: pastTs });

      const result = await port.queryProposals({
        created_before: '2021-01-01T00:00:00.000Z' as Timestamp,
      });
      expect(result.proposals.some((pr) => pr.id === p.id)).toBe(true);
    });
  });

  describe('queryProposals sort_by expires_at', () => {
    it('sorts by expires_at', async () => {
      const port = createMockEmberPort();
      const p1 = await port.createProposal(makeCreateProposalInput({ ttl_days: 7 }));
      await port.createProposal(makeCreateProposalInput({ summary: 'Long TTL', ttl_days: 90 }));

      const result = await port.queryProposals({ sort_by: 'expires_at', sort_order: 'asc' });
      // p1 expires sooner, so it should be first
      expect(result.proposals[0].id).toBe(p1.id);
    });
  });

  describe('_reset restores initial proposals', () => {
    it('repopulates the store with initial proposals after reset', async () => {
      const port = createMockEmberPort({ initialProposals: [] });
      await port.createProposal(makeCreateProposalInput());
      expect(port._store.size).toBe(1);
      port._reset();
      // Resets to empty (initial was empty)
      expect(port._store.size).toBe(0);
      expect(port._mocks.createProposal).not.toHaveBeenCalled();
    });
  });
});

describe('MockEddaPort — additional branch coverage (TCOV-015)', () => {
  describe('queryMemories created_after / created_before filters', () => {
    it('filters by created_after', async () => {
      const port = createMockEddaPort();
      const m = await port.createMemory(makeCreateMemoryInput());
      // Force an old timestamp
      const oldTs = '2020-01-01T00:00:00.000Z' as Timestamp;
      port._store.set(m.id, { ...port._store.get(m.id)!, created_at: oldTs });

      const result = await port.queryMemories({
        created_after: '2021-01-01T00:00:00.000Z' as Timestamp,
      });
      expect(result.memories).toHaveLength(0);
    });

    it('filters by created_before', async () => {
      const port = createMockEddaPort();
      const m = await port.createMemory(makeCreateMemoryInput());
      const futureTs = '2030-01-01T00:00:00.000Z' as Timestamp;
      port._store.set(m.id, { ...port._store.get(m.id)!, created_at: futureTs });

      const result = await port.queryMemories({
        created_before: '2025-01-01T00:00:00.000Z' as Timestamp,
      });
      expect(result.memories).toHaveLength(0);
    });
  });
});

describe('MockEmberPort — additional branch coverage (TCOV-015)', () => {
  describe('queryProposals created_after filter', () => {
    it('filters by created_after', async () => {
      const port = createMockEmberPort();
      const p = await port.createProposal(makeCreateProposalInput());
      const oldTs = '2020-01-01T00:00:00.000Z' as Timestamp;
      port._store.set(p.id, { ...port._store.get(p.id)!, created_at: oldTs });

      const result = await port.queryProposals({
        created_after: '2021-01-01T00:00:00.000Z' as Timestamp,
      });
      expect(result.proposals.some((pr) => pr.id === p.id)).toBe(false);
    });
  });
});

describe('MockKindlingPort — branch coverage (TCOV-015)', () => {
  describe('querySession time_range and tags filter', () => {
    it('filters by time_range in querySession', async () => {
      const port = createMockKindlingPort();
      const sid = createSessionId(UUID_A);
      const oldTs = '2020-01-01T00:00:00.000Z' as Timestamp;
      const recentTs = '2025-01-01T00:00:00.000Z' as Timestamp;

      const oldObs = await port.createObservation(
        makeCreateObservationInput({ session_id: sid, summary: 'Old' })
      );
      port._store.set(oldObs.id, { ...port._store.get(oldObs.id)!, timestamp: oldTs });
      const recentObs = await port.createObservation(
        makeCreateObservationInput({ session_id: sid, summary: 'Recent' })
      );
      port._store.set(recentObs.id, { ...port._store.get(recentObs.id)!, timestamp: recentTs });

      const result = await port.querySession(sid, {
        time_range: {
          start: '2024-01-01T00:00:00.000Z' as Timestamp,
          end: '2026-01-01T00:00:00.000Z' as Timestamp,
        },
      });
      expect(result.observations.some((o) => o.id === recentObs.id)).toBe(true);
      expect(result.observations.every((o) => o.id !== oldObs.id)).toBe(true);
    });

    it('filters by tags in querySession', async () => {
      const port = createMockKindlingPort();
      const sid = createSessionId(UUID_A);
      await port.createObservation(makeCreateObservationInput({ session_id: sid, tags: ['gate'] }));
      await port.createObservation(
        makeCreateObservationInput({ session_id: sid, tags: ['action'], summary: 'Action obs' })
      );

      const result = await port.querySession(sid, { tags: ['gate'] });
      expect(result.observations.every((o) => o.tags?.includes('gate'))).toBe(true);
      expect(result.observations).toHaveLength(1);
    });
  });

  describe('queryByPlan with kinds filter', () => {
    it('filters plan observations by kind', async () => {
      const port = createMockKindlingPort();
      const sid = createSessionId(UUID_A);
      const planId = 'plan-filtered';
      await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          kind: 'gate_evaluated' as const,
          data: { plan: planId },
          summary: 'Gate obs',
        })
      );
      await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          kind: 'action_executed' as const,
          data: { plan: planId },
          summary: 'Action obs',
        })
      );

      const result = await port.queryByPlan(
        planId as unknown as Parameters<typeof port.queryByPlan>[0],
        { kinds: ['gate_evaluated'] }
      );
      // sessions are grouped; both from same session but kinds filter applied to planObs
      expect(result.sessions[0].observation_count).toBe(1);
    });
  });

  describe('queryByPlan with session_time_range filter', () => {
    it('filters sessions by session_time_range', async () => {
      const port = createMockKindlingPort();
      const sid = createSessionId(UUID_A);
      const planId = 'plan-time-filtered';
      const futureTs = new Date(Date.now() + 5000).toISOString() as Timestamp;

      const obs = await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          data: { plan: planId },
          summary: 'Future plan obs',
        })
      );
      // Force a far-future timestamp on this observation
      port._store.set(obs.id, { ...port._store.get(obs.id)!, timestamp: futureTs });

      // Range that excludes the future
      const result = await port.queryByPlan(
        planId as unknown as Parameters<typeof port.queryByPlan>[0],
        {
          session_time_range: {
            start: '2000-01-01T00:00:00.000Z' as Timestamp,
            end: '2020-01-01T00:00:00.000Z' as Timestamp,
          },
        }
      );
      // Session started_at is the future timestamp, which is outside 2000-2020 range
      expect(result.sessions).toHaveLength(0);
    });

    it('includes sessions within session_time_range', async () => {
      const port = createMockKindlingPort();
      const sid = createSessionId(UUID_A);
      const planId = 'plan-in-range';
      const midTs = '2024-06-01T12:00:00.000Z' as Timestamp;

      const obs = await port.createObservation(
        makeCreateObservationInput({
          session_id: sid,
          data: { plan: planId },
          summary: 'In-range plan obs',
        })
      );
      port._store.set(obs.id, { ...port._store.get(obs.id)!, timestamp: midTs });

      const result = await port.queryByPlan(
        planId as unknown as Parameters<typeof port.queryByPlan>[0],
        {
          session_time_range: {
            start: '2024-01-01T00:00:00.000Z' as Timestamp,
            end: '2025-01-01T00:00:00.000Z' as Timestamp,
          },
        }
      );
      expect(result.sessions).toHaveLength(1);
    });
  });
});
