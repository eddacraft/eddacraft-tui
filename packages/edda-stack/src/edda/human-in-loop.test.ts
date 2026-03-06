import { afterEach, describe, expect, expectTypeOf, it, vi } from 'vitest';
import type {
  CandidateProposal,
  MemoryObject,
  PromoteProposalInput,
  RetireMemoryInput,
} from '../contracts/index.js';
import {
  MemoryObjectSchema,
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
} from '../contracts/index.js';
import type { CreateMemoryInput, IEmberPort } from '../contracts/ports/index.js';
import { EvolutionService } from './evolution-service.js';
import { MemoryService } from './memory-service.js';
import { PromotionService } from './promotion-service.js';
import { ProvenanceService } from './provenance-service.js';
import type { IMemoryStoreOperations, IVersionTracker } from './store-interfaces.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('Human-in-loop memory boundaries', () => {
  it('requires non-empty promoted_by for proposal promotion', async () => {
    const proposal = createProposal();
    const service = new PromotionService({
      store: createStoreMock(),
      emberPort: createEmberPortMock(proposal),
      config: {
        require_reason: true,
        require_attribution: true,
        min_ember_confidence: 0.5,
      },
    });

    const input = createPromotionInput(proposal.id);
    input.promoted_by = '   ';

    await expect(service.promoteProposal(input)).rejects.toThrow(
      'Promotion attribution is required'
    );
  });

  it('requires non-empty reason for proposal promotion', async () => {
    const proposal = createProposal();
    const service = new PromotionService({
      store: createStoreMock(),
      emberPort: createEmberPortMock(proposal),
      config: {
        require_reason: true,
        require_attribution: true,
        min_ember_confidence: 0.5,
      },
    });

    const input = createPromotionInput(proposal.id);
    input.reason = '  ';

    await expect(service.promoteProposal(input)).rejects.toThrow('Promotion reason is required');
  });

  it('requires created_by in direct memory creation input', async () => {
    const service = new PromotionService({ store: createStoreMock() });
    const rawInput = createDirectMemoryInput() as unknown as Record<string, unknown>;
    delete rawInput.created_by;

    await expect(service.createMemory(rawInput as unknown as CreateMemoryInput)).rejects.toThrow();
  });

  it('does not expose an auto-promote public API on MemoryService', () => {
    const store = createStoreMock();
    const memoryService = new MemoryService({
      store,
      promotionService: new PromotionService({ store }),
      provenanceService: new ProvenanceService({ store }),
      evolutionService: new EvolutionService({ store }),
    });

    const publicMethods = Object.getOwnPropertyNames(Object.getPrototypeOf(memoryService));

    expect(publicMethods).not.toContain('autoPromote');
    expect(publicMethods).not.toContain('promoteEligibleProposals');
    expect(publicMethods).toContain('promoteProposal');
  });

  it('produces non-empty attribution.actor across human-driven creation paths', async () => {
    const store = createStoreMock();
    const oldMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655440091'));
    vi.mocked(store.getMemory).mockResolvedValue(oldMemory);

    const promotionService = new PromotionService({
      store,
      emberPort: createEmberPortMock(createProposal()),
      versionTracker: createVersionTrackerMock(),
    });
    const evolutionService = new EvolutionService({
      store,
      versionTracker: createVersionTrackerMock(),
    });

    const promoted = await promotionService.promoteProposal(
      createPromotionInput(createProposalId('550e8400-e29b-41d4-a716-446655440001'))
    );
    const created = await promotionService.createMemory(createDirectMemoryInput());
    const superseded = await evolutionService.supersedeMemory(
      oldMemory.id,
      createDirectMemoryInput()
    );

    expect(promoted.attribution.actor.trim().length).toBeGreaterThan(0);
    expect(created.attribution.actor.trim().length).toBeGreaterThan(0);
    expect(superseded.new.attribution.actor.trim().length).toBeGreaterThan(0);
  });

  it('rejects MemoryObjectSchema payloads that omit attribution.actor', () => {
    const memory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655440092'));
    const payload = {
      ...memory,
      attribution: {
        timestamp: memory.attribution.timestamp,
        method: memory.attribution.method,
        reason: memory.attribution.reason,
      },
    };

    const result = MemoryObjectSchema.safeParse(payload);

    expect(result.success).toBe(false);
  });

  it('requires retired_by in RetireMemoryInput contract', () => {
    expectTypeOf<RetireMemoryInput>().toMatchTypeOf<{ reason: string; retired_by: string }>();
  });

  it('requires created_by in CreateMemoryInput used by supersedeMemory', () => {
    type SupersedeInput = Parameters<EvolutionService['supersedeMemory']>[1];
    expectTypeOf<SupersedeInput>().toMatchTypeOf<{ created_by: string }>();
    expectTypeOf<CreateMemoryInput>().toMatchTypeOf<{ created_by: string }>();
  });
});

function createStoreMock(): IMemoryStoreOperations {
  return {
    getMemory: vi.fn(async () => null),
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

function createVersionTrackerMock(): IVersionTracker {
  return {
    init: vi.fn(async () => undefined),
    trackChange: vi.fn(async () => 'hash-1'),
    getHistory: vi.fn(async () => []),
    isInitialised: vi.fn(async () => true),
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

function createProposal(overrides?: Partial<CandidateProposal>): CandidateProposal {
  return {
    id: createProposalId('550e8400-e29b-41d4-a716-446655440001'),
    type: 'decision',
    status: 'active',
    summary: 'Adopt strict linting for memory services',
    rationale: 'Prevents drift and enforces quality expectations',
    confidence: 0.8,
    signals: [],
    provenance: {
      observation_ids: ['550e8400-e29b-41d4-a716-446655440010'],
      session_ids: ['550e8400-e29b-41d4-a716-446655440020'],
      proposal_id: '550e8400-e29b-41d4-a716-446655440001',
      earliest_observation: '2026-03-01T10:00:00.000Z',
      latest_observation: '2026-03-01T11:00:00.000Z',
    },
    created_at: '2026-03-01T12:00:00.000Z',
    expires_at: '2026-03-31T12:00:00.000Z',
    ttl_days: 30,
    ...overrides,
  };
}

function createPromotionInput(
  proposalId: PromoteProposalInput['proposal_id']
): PromoteProposalInput {
  return {
    proposal_id: proposalId,
    type: 'decision',
    statement: 'Use strict linting for Edda service implementations',
    confidence: 'high',
    confidence_rationale: 'Directly affirmed during review',
    context: {
      when: '2026-03-01T12:00:00.000Z',
      why: 'Consistency and auditability are required',
      conditions: ['Service layer changes'],
      tags: ['quality', 'edda'],
    },
    promoted_by: 'joshua',
    reason: 'This guidance is now stable enough for canonical memory',
  };
}

function createDirectMemoryInput(): CreateMemoryInput {
  return {
    type: 'decision',
    statement: 'Always use explicit dependency interfaces in Edda services',
    context: {
      when: '2026-03-02T09:00:00.000Z',
      why: 'Prevents coupling during phased delivery',
      conditions: ['Parallel implementation phases'],
      tags: ['architecture', 'interfaces'],
    },
    confidence: 'high',
    confidence_rationale: 'Repeatedly validated in integration work',
    provenance: {
      kindling_sources: [
        {
          observation_id: createObservationId('550e8400-e29b-41d4-a716-446655440100'),
          session_id: createSessionId('550e8400-e29b-41d4-a716-446655440101'),
          kind: 'action_executed',
          timestamp: '2026-03-02T09:00:00.000Z',
        },
      ],
      source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655440101')],
    },
    created_by: 'joshua',
    reason: 'Core implementation boundary should be explicit and durable',
  };
}

function createMemory(
  id: ReturnType<typeof createMemoryId>,
  overrides?: Partial<MemoryObject>
): MemoryObject {
  return {
    id,
    type: 'decision',
    status: 'active',
    schema_version: 1,
    statement: 'Base memory statement',
    context: {
      when: '2026-03-01T10:00:00.000Z',
      why: 'Track evolution lifecycle',
      conditions: ['Edda service changes'],
      tags: ['evolution'],
    },
    confidence: 'high',
    provenance: {
      kindling_sources: [
        {
          observation_id: createObservationId('550e8400-e29b-41d4-a716-446655440101'),
          session_id: createSessionId('550e8400-e29b-41d4-a716-446655440102'),
          kind: 'action_executed',
          timestamp: '2026-03-01T10:00:00.000Z',
        },
      ],
      source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655440102')],
    },
    attribution: {
      actor: 'joshua',
      timestamp: '2026-03-01T10:00:00.000Z',
      method: 'cli_command',
      reason: 'Initial memory',
    },
    evolution: { supersedes: [] },
    created_at: '2026-03-01T10:00:00.000Z',
    ...overrides,
  };
}
