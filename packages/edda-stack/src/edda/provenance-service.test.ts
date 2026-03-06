import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CandidateProposal, MemoryObject, ProvenanceChain } from '../contracts/index.js';
import {
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
} from '../contracts/index.js';
import type { IEmberPort } from '../contracts/ports/index.js';
import type { IMemoryStoreOperations } from './store-interfaces.js';
import { ProvenanceService } from './provenance-service.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('ProvenanceService', () => {
  it('resolves a complete provenance chain when proposal exists', async () => {
    const proposal = createProposal();
    const service = new ProvenanceService({
      store: createStoreMock(),
      emberPort: createEmberPortMock(proposal),
    });

    const result = await service.resolveProvenance(createProvenanceChain(proposal.id));

    expect(result.complete).toBe(true);
    expect(result.total_count).toBe(3);
    expect(result.resolved_count).toBe(3);
    expect(result.missing_links).toEqual([]);
    expect(result.resolved_data?.proposal_id).toBe(proposal.id);
  });

  it('reports missing links when ember proposal cannot be resolved', async () => {
    const proposalId = createProposalId('550e8400-e29b-41d4-a716-446655440999');
    const service = new ProvenanceService({
      store: createStoreMock(),
      emberPort: createEmberPortMock(null),
    });

    const result = await service.resolveProvenance(createProvenanceChain(proposalId));

    expect(result.complete).toBe(false);
    expect(result.missing_links).toContain(`proposal:${proposalId}`);
    expect(result.resolved_count).toBe(2);
  });

  it('warns when ember proposal exists but emberPort is unavailable', async () => {
    const service = new ProvenanceService({
      store: createStoreMock(),
    });

    const result = await service.resolveProvenance(
      createProvenanceChain(createProposalId('550e8400-e29b-41d4-a716-446655440998'))
    );

    expect(result.complete).toBe(false);
    expect(result.warnings).toContain('Cannot validate Ember proposal reference without emberPort');
  });

  it('returns memory provenance resolution when memory exists', async () => {
    const proposal = createProposal();
    const chain = createProvenanceChain(proposal.id);
    const memory = createMemory(chain);
    const store = createStoreMock(memory);
    const service = new ProvenanceService({
      store,
      emberPort: createEmberPortMock(proposal),
    });

    const result = await service.getMemoryProvenance(memory.id);

    expect(result?.memory.id).toBe(memory.id);
    expect(result?.resolution.complete).toBe(true);
  });

  it('validates provenance integrity via contract utility', () => {
    const service = new ProvenanceService({ store: createStoreMock() });
    const invalidChain: ProvenanceChain = {
      kindling_sources: [
        {
          observation_id: createObservationId('550e8400-e29b-41d4-a716-446655440010'),
          session_id: createSessionId('550e8400-e29b-41d4-a716-446655440011'),
          kind: 'action_executed',
          timestamp: '2026-03-01T10:00:00.000Z',
        },
      ],
      source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655440099')],
    };

    const result = service.validateProvenanceIntegrity(invalidChain);

    expect(result.valid).toBe(false);
    expect(result.issues[0]).toContain('references session not in source_sessions');
  });
});

function createStoreMock(memory?: MemoryObject): IMemoryStoreOperations {
  return {
    getMemory: vi.fn(async () => memory ?? null),
    saveMemory: vi.fn(async (_memory: MemoryObject) => undefined),
    getMemoryByProposalId: vi.fn(async () => null),
    queryMemories: vi.fn(async () => ({
      memories: [],
      total: 0,
      limit: 100,
      offset: 0,
      has_more: false,
    })),
    getActiveMemories: vi.fn(async () => []),
    getMemoriesByType: vi.fn(async () => []),
    searchMemories: vi.fn(async () => []),
    memoryExists: vi.fn(async () => false),
    countMemories: vi.fn(async () => 0),
    getStats: vi.fn(async () => ({
      total_memories: 0,
      by_status: [],
      by_type: [],
      by_confidence: [],
      active_count: 0,
      superseded_count: 0,
      retired_count: 0,
      unique_tags_count: 0,
    })),
    isAvailable: vi.fn(async () => true),
    exportMemories: vi.fn(async () => []),
    importMemories: vi.fn(async () => 0),
  };
}

function createEmberPortMock(proposal: CandidateProposal | null): IEmberPort {
  return {
    createProposal: vi.fn(async () => {
      throw new Error('Not used in this test');
    }),
    updateProposal: vi.fn(async () => null),
    resolveProposal: vi.fn(async () => null),
    getProposal: vi.fn(async () => proposal),
    queryProposals: vi.fn(async () => ({
      proposals: [],
      total: 0,
      limit: 100,
      offset: 0,
      has_more: false,
    })),
    getActiveProposals: vi.fn(async () => []),
    getProposalsBySession: vi.fn(async () => []),
    proposalExists: vi.fn(async () => proposal !== null),
    markPromoted: vi.fn(async () => undefined),
    markDismissed: vi.fn(async () => undefined),
    getExpiredProposals: vi.fn(async () => []),
    processExpiredProposals: vi.fn(async () => 0),
    expireStaleProposals: vi.fn(async () => 0),
    isAvailable: vi.fn(async () => true),
    getStats: vi.fn(async () => ({
      total_proposals: 0,
      by_status: [],
      by_type: [],
      expiring_soon: 0,
    })),
    countProposals: vi.fn(async () => 0),
    pruneProposals: vi.fn(async () => 0),
  };
}

function createProposal(): CandidateProposal {
  return {
    id: createProposalId('550e8400-e29b-41d4-a716-446655440001'),
    type: 'pattern',
    status: 'active',
    summary: 'Pattern is stable',
    rationale: 'Observed repeatedly',
    confidence: 0.9,
    signals: [],
    provenance: {
      observation_ids: ['550e8400-e29b-41d4-a716-446655440010'],
      session_ids: ['550e8400-e29b-41d4-a716-446655440020'],
      earliest_observation: '2026-03-01T09:00:00.000Z',
      latest_observation: '2026-03-01T10:00:00.000Z',
      proposal_id: '550e8400-e29b-41d4-a716-446655440001',
    },
    created_at: '2026-03-01T10:00:00.000Z',
    expires_at: '2026-03-31T10:00:00.000Z',
    ttl_days: 30,
  };
}

function createProvenanceChain(proposalId: ReturnType<typeof createProposalId>): ProvenanceChain {
  return {
    ember_source: {
      proposal_id: proposalId,
      proposal_type: 'pattern',
      confidence: 0.9,
      created_at: '2026-03-01T10:00:00.000Z',
    },
    kindling_sources: [
      {
        observation_id: createObservationId('550e8400-e29b-41d4-a716-446655440010'),
        session_id: createSessionId('550e8400-e29b-41d4-a716-446655440020'),
        kind: 'gate_evaluated',
        timestamp: '2026-03-01T09:00:00.000Z',
      },
    ],
    source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655440020')],
  };
}

function createMemory(chain: ProvenanceChain): MemoryObject {
  return {
    id: createMemoryId('550e8400-e29b-41d4-a716-446655440501'),
    type: 'pattern',
    status: 'active',
    schema_version: 1,
    statement: 'Codify the repeated pattern',
    context: {
      when: '2026-03-01T10:00:00.000Z',
      why: 'Pattern is stable',
      conditions: ['Service changes'],
      tags: ['pattern'],
    },
    confidence: 'high',
    provenance: chain,
    attribution: {
      actor: 'joshua',
      timestamp: '2026-03-01T11:00:00.000Z',
      method: 'cli_command',
      reason: 'Promoted by human decision',
    },
    evolution: { supersedes: [] },
    created_at: '2026-03-01T11:00:00.000Z',
  };
}
