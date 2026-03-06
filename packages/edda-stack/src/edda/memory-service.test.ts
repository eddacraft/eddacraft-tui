import { afterEach, describe, expect, it, vi } from 'vitest';
import type {
  CandidateProposal,
  MemoryObject,
  MemoryQuery,
  MemoryQueryResult,
  PromoteProposalInput,
  ProvenanceChain,
} from '../contracts/index.js';
import {
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
} from '../contracts/index.js';
import type {
  CreateMemoryInput,
  EddaStats,
  ProvenanceResolutionResult,
} from '../contracts/ports/index.js';
import type { IMemoryStoreOperations, IVersionTracker } from './store-interfaces.js';
import { EvolutionService } from './evolution-service.js';
import { MemoryService } from './memory-service.js';
import { PromotionService } from './promotion-service.js';
import { ProvenanceService } from './provenance-service.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('MemoryService', () => {
  it('delegates write operations to specialised services', async () => {
    const store = createStoreMock();
    const services = createServiceDeps(store);
    const memoryService = new MemoryService({
      store,
      promotionService: services.promotionService,
      provenanceService: services.provenanceService,
      evolutionService: services.evolutionService,
    });

    const promotedMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441001'));
    const createdMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441002'));
    const retiredMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441003'), {
      status: 'retired',
    });
    const supersededResult = {
      old: createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441004'), {
        status: 'superseded',
      }),
      new: createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441005')),
    };

    vi.spyOn(services.promotionService, 'promoteProposal').mockResolvedValue(promotedMemory);
    vi.spyOn(services.promotionService, 'createMemory').mockResolvedValue(createdMemory);
    vi.spyOn(services.promotionService, 'createMemoryFromProposal').mockResolvedValue(
      createdMemory
    );
    vi.spyOn(services.evolutionService, 'retireMemory').mockResolvedValue(retiredMemory);
    vi.spyOn(services.evolutionService, 'retireMemoryById').mockResolvedValue(undefined);
    vi.spyOn(services.evolutionService, 'supersedeMemory').mockResolvedValue(supersededResult);

    await expect(memoryService.promoteProposal(createPromotionInput())).resolves.toEqual(
      promotedMemory
    );
    await expect(memoryService.createMemory(createCreateMemoryInput())).resolves.toEqual(
      createdMemory
    );
    await expect(
      memoryService.createMemoryFromProposal(createPromotionInput(), createCandidateProposal())
    ).resolves.toEqual(createdMemory);
    await expect(
      memoryService.retireMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441006'), {
        reason: 'Retire this memory',
        retired_by: 'joshua',
      })
    ).resolves.toEqual(retiredMemory);
    await expect(
      memoryService.supersedeMemory(
        createMemoryId('550e8400-e29b-41d4-a716-446655441007'),
        createCreateMemoryInput()
      )
    ).resolves.toEqual(supersededResult);
    await memoryService.retireMemoryById(
      createMemoryId('550e8400-e29b-41d4-a716-446655441008'),
      createMemoryId('550e8400-e29b-41d4-a716-446655441009'),
      'Superseded',
      'joshua'
    );

    expect(services.promotionService.promoteProposal).toHaveBeenCalledTimes(1);
    expect(services.promotionService.createMemory).toHaveBeenCalledTimes(1);
    expect(services.promotionService.createMemoryFromProposal).toHaveBeenCalledTimes(1);
    expect(services.evolutionService.retireMemory).toHaveBeenCalledTimes(1);
    expect(services.evolutionService.retireMemoryById).toHaveBeenCalledTimes(1);
    expect(services.evolutionService.supersedeMemory).toHaveBeenCalledTimes(1);
  });

  it('delegates read, provenance, evolution, and maintenance operations', async () => {
    const store = createStoreMock();
    const services = createServiceDeps(store);
    const memoryService = new MemoryService({
      store,
      promotionService: services.promotionService,
      provenanceService: services.provenanceService,
      evolutionService: services.evolutionService,
    });

    const memory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441101'));
    const queryResult: MemoryQueryResult = {
      memories: [memory],
      total: 1,
      limit: 10,
      offset: 0,
      has_more: false,
    };
    const stats: EddaStats = {
      total_memories: 1,
      by_status: [],
      by_type: [],
      by_confidence: [],
      active_count: 1,
      superseded_count: 0,
      retired_count: 0,
      unique_tags_count: 1,
    };
    const provenanceResolution: ProvenanceResolutionResult = {
      complete: true,
      resolved_count: 2,
      total_count: 2,
      missing_links: [],
      warnings: [],
      resolved_data: {
        sessions: ['550e8400-e29b-41d4-a716-446655441111'],
        observations: ['550e8400-e29b-41d4-a716-446655441112'],
      },
    };

    vi.mocked(store.getMemory).mockResolvedValue(memory);
    vi.mocked(store.getMemoryByProposalId).mockResolvedValue(memory);
    vi.mocked(store.queryMemories).mockResolvedValue(queryResult);
    vi.mocked(store.getActiveMemories).mockResolvedValue([memory]);
    vi.mocked(store.getMemoriesByType).mockResolvedValue([memory]);
    vi.mocked(store.searchMemories).mockResolvedValue([memory]);
    vi.mocked(store.memoryExists).mockResolvedValue(true);
    vi.mocked(store.isAvailable).mockResolvedValue(true);
    vi.mocked(store.getStats).mockResolvedValue(stats);
    vi.mocked(store.countMemories).mockResolvedValue(1);
    vi.mocked(store.exportMemories).mockResolvedValue([memory]);
    vi.mocked(store.importMemories).mockResolvedValue(1);
    vi.spyOn(services.provenanceService, 'resolveProvenance').mockResolvedValue(
      provenanceResolution
    );
    vi.spyOn(services.evolutionService, 'getEvolutionChain').mockResolvedValue([memory]);
    vi.spyOn(services.evolutionService, 'getLatestVersion').mockResolvedValue(memory);

    const query: MemoryQuery = {
      include_superseded: false,
      limit: 10,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    };

    await expect(memoryService.getMemory(memory.id)).resolves.toEqual(memory);
    await expect(
      memoryService.getMemoryByProposalId(createProposalId('550e8400-e29b-41d4-a716-446655441120'))
    ).resolves.toEqual(memory);
    await expect(memoryService.queryMemories(query)).resolves.toEqual(queryResult);
    await expect(memoryService.getActiveMemories()).resolves.toEqual([memory]);
    await expect(memoryService.getMemoriesByType('decision')).resolves.toEqual([memory]);
    await expect(memoryService.searchMemories('Base')).resolves.toEqual([memory]);
    await expect(memoryService.memoryExists(memory.id)).resolves.toBe(true);
    await expect(memoryService.getEvolutionChain(memory.id)).resolves.toEqual([memory]);
    await expect(memoryService.getLatestVersion(memory.id)).resolves.toEqual(memory);
    await expect(memoryService.resolveProvenance(memory.provenance)).resolves.toEqual(
      provenanceResolution
    );
    await expect(memoryService.isAvailable()).resolves.toBe(true);
    await expect(memoryService.getStats()).resolves.toEqual(stats);
    await expect(memoryService.countMemories({ status: 'active' })).resolves.toBe(1);
    await expect(memoryService.exportMemories()).resolves.toEqual([memory]);
    await expect(memoryService.importMemories([memory])).resolves.toBe(1);

    expect(store.getMemory).toHaveBeenCalledTimes(1);
    expect(services.provenanceService.resolveProvenance).toHaveBeenCalledTimes(1);
    expect(services.evolutionService.getLatestVersion).toHaveBeenCalledTimes(1);
  });

  it('updates memory and records version change', async () => {
    const existing = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441201'));
    const store = createStoreMock();
    const versionTracker = createVersionTrackerMock();
    const services = createServiceDeps(store);
    const memoryService = new MemoryService({
      store,
      promotionService: services.promotionService,
      provenanceService: services.provenanceService,
      evolutionService: services.evolutionService,
      versionTracker,
    });

    vi.mocked(store.getMemory).mockResolvedValue(existing);

    const updated = await memoryService.updateMemory(existing.id, {
      statement: 'Updated memory statement',
      context: { scope: 'edda-only' },
      confidence: 'medium',
      confidence_rationale: 'New evidence reduced certainty',
      metadata: { reviewed: true },
    });

    expect(updated?.statement).toBe('Updated memory statement');
    expect(updated?.context.scope).toBe('edda-only');
    expect(updated?.confidence).toBe('medium');
    expect(store.saveMemory).toHaveBeenCalledTimes(1);
    expect(versionTracker.trackChange).toHaveBeenCalledTimes(1);
  });

  it('returns null for updateMemory when memory does not exist', async () => {
    const store = createStoreMock();
    const services = createServiceDeps(store);
    const memoryService = new MemoryService({
      store,
      promotionService: services.promotionService,
      provenanceService: services.provenanceService,
      evolutionService: services.evolutionService,
    });

    vi.mocked(store.getMemory).mockResolvedValue(null);

    await expect(
      memoryService.updateMemory(createMemoryId('550e8400-e29b-41d4-a716-446655441301'), {
        statement: 'No-op',
      })
    ).resolves.toBeNull();
  });
});

function createServiceDeps(store: IMemoryStoreOperations): {
  promotionService: PromotionService;
  provenanceService: ProvenanceService;
  evolutionService: EvolutionService;
} {
  return {
    promotionService: new PromotionService({ store }),
    provenanceService: new ProvenanceService({ store }),
    evolutionService: new EvolutionService({ store }),
  };
}

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

function createPromotionInput(): PromoteProposalInput {
  return {
    proposal_id: createProposalId('550e8400-e29b-41d4-a716-446655442001'),
    type: 'decision',
    statement: 'Promote this decision',
    confidence: 'high',
    context: {
      when: '2026-03-03T10:00:00.000Z',
      why: 'It is stable',
      conditions: ['Service API decisions'],
      tags: ['decision'],
    },
    promoted_by: 'joshua',
    reason: 'Human-reviewed and approved',
  };
}

function createCandidateProposal(): CandidateProposal {
  return {
    id: createProposalId('550e8400-e29b-41d4-a716-446655442002'),
    type: 'decision',
    status: 'active',
    summary: 'Candidate decision summary',
    rationale: 'Candidate rationale',
    confidence: 0.9,
    signals: [],
    provenance: {
      observation_ids: ['550e8400-e29b-41d4-a716-446655442010'],
      session_ids: ['550e8400-e29b-41d4-a716-446655442011'],
      earliest_observation: '2026-03-03T09:00:00.000Z',
      latest_observation: '2026-03-03T10:00:00.000Z',
      proposal_id: '550e8400-e29b-41d4-a716-446655442002',
    },
    created_at: '2026-03-03T10:00:00.000Z',
    expires_at: '2026-04-02T10:00:00.000Z',
    ttl_days: 30,
  };
}

function createCreateMemoryInput(): CreateMemoryInput {
  return {
    type: 'decision',
    statement: 'Create memory directly',
    context: {
      when: '2026-03-03T10:00:00.000Z',
      why: 'Manual creation for canonical truth',
      conditions: ['Direct creation flow'],
      tags: ['manual'],
    },
    confidence: 'high',
    provenance: {
      kindling_sources: [
        {
          observation_id: createObservationId('550e8400-e29b-41d4-a716-446655442101'),
          session_id: createSessionId('550e8400-e29b-41d4-a716-446655442102'),
          kind: 'action_executed',
          timestamp: '2026-03-03T10:00:00.000Z',
        },
      ],
      source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655442102')],
    },
    created_by: 'joshua',
    reason: 'Explicit human decision',
  };
}

function createMemory(
  id: ReturnType<typeof createMemoryId>,
  overrides?: Partial<MemoryObject>
): MemoryObject {
  const provenance: ProvenanceChain = {
    kindling_sources: [
      {
        observation_id: createObservationId('550e8400-e29b-41d4-a716-446655443001'),
        session_id: createSessionId('550e8400-e29b-41d4-a716-446655443002'),
        kind: 'gate_evaluated',
        timestamp: '2026-03-03T11:00:00.000Z',
      },
    ],
    source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655443002')],
  };

  return {
    id,
    type: 'decision',
    status: 'active',
    schema_version: 1,
    statement: 'Base statement',
    context: {
      when: '2026-03-03T11:00:00.000Z',
      why: 'Base rationale',
      conditions: ['Base condition'],
      tags: ['base'],
    },
    confidence: 'high',
    provenance,
    attribution: {
      actor: 'joshua',
      timestamp: '2026-03-03T11:00:00.000Z',
      method: 'cli_command',
      reason: 'Created for testing',
    },
    evolution: { supersedes: [] },
    created_at: '2026-03-03T11:00:00.000Z',
    ...overrides,
  };
}
