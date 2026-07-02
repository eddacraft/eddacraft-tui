import { afterEach, describe, expect, it, vi } from 'vitest';
import type { MemoryObject } from '../contracts/edda-memory.js';
import { createMemoryId, createObservationId, createSessionId } from '../contracts/index.js';
import type { CreateMemoryInput } from '../contracts/ports/index.js';
import type { IMemoryStoreOperations, IVersionTracker } from './store-interfaces.js';
import { EvolutionService } from './evolution-service.js';

afterEach(() => {
  vi.restoreAllMocks();
});

describe('EvolutionService', () => {
  it('supersedes old memory by creating a new memory and retiring the old one', async () => {
    const oldMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655440001'));
    const store = createInMemoryStore([oldMemory]);
    const versionTracker = createVersionTrackerMock();
    const service = new EvolutionService({ store, versionTracker });

    const result = await service.supersedeMemory(oldMemory.id, createCreateMemoryInput());

    expect(result.new.evolution.supersedes).toEqual([oldMemory.id]);
    expect(result.old.status).toBe('superseded');
    expect(result.old.evolution.superseded_by).toBe(result.new.id);
    expect(versionTracker.trackChange).toHaveBeenCalledTimes(1);
  });

  it('falls back to a plain retirement when the replacement memory cannot be saved', async () => {
    const oldMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655440003'));
    const store = createInMemoryStore([oldMemory]);
    const service = new EvolutionService({ store });
    const originalSaveMemory = store.saveMemory;
    let saveAttempts = 0;
    store.saveMemory = vi.fn(async (memory) => {
      saveAttempts += 1;
      if (saveAttempts === 2) {
        throw new Error('disk full');
      }

      await originalSaveMemory(memory);
    });

    await expect(service.supersedeMemory(oldMemory.id, createCreateMemoryInput())).rejects.toThrow(
      'disk full'
    );

    const retiredMemory = await store.getMemory(oldMemory.id);
    expect(retiredMemory?.status).toBe('retired');
    expect(retiredMemory?.evolution.superseded_by).toBeUndefined();
  });

  it("throws when superseding a retired memory because status must be 'active'", async () => {
    const oldMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655440031'), {
      status: 'retired',
    });
    const service = new EvolutionService({ store: createInMemoryStore([oldMemory]) });

    await expect(service.supersedeMemory(oldMemory.id, createCreateMemoryInput())).rejects.toThrow(
      "must be 'active'"
    );
  });

  it("throws when superseding a superseded memory because status must be 'active'", async () => {
    const oldMemory = createMemory(createMemoryId('550e8400-e29b-41d4-a716-446655440032'), {
      status: 'superseded',
    });
    const service = new EvolutionService({ store: createInMemoryStore([oldMemory]) });

    await expect(service.supersedeMemory(oldMemory.id, createCreateMemoryInput())).rejects.toThrow(
      "must be 'active'"
    );
  });

  it('retires a memory with explicit reason and actor', async () => {
    const id = createMemoryId('550e8400-e29b-41d4-a716-446655440002');
    const store = createInMemoryStore([createMemory(id)]);
    const service = new EvolutionService({ store });

    const retired = await service.retireMemory(id, {
      reason: 'This guidance no longer applies',
      retired_by: 'joshua',
    });

    expect(retired?.status).toBe('retired');
    expect(retired?.evolution.retired_reason).toBe('This guidance no longer applies');
    expect(retired?.evolution.retired_by).toBe('joshua');
  });

  it('builds evolution chain from oldest to latest', async () => {
    const id1 = createMemoryId('550e8400-e29b-41d4-a716-446655440011');
    const id2 = createMemoryId('550e8400-e29b-41d4-a716-446655440012');
    const id3 = createMemoryId('550e8400-e29b-41d4-a716-446655440013');
    const m1 = createMemory(id1, {
      status: 'superseded',
      evolution: { supersedes: [], superseded_by: id2, retired_reason: 'v2', retired_by: 'joshua' },
    });
    const m2 = createMemory(id2, {
      status: 'superseded',
      evolution: {
        supersedes: [id1],
        superseded_by: id3,
        retired_reason: 'v3',
        retired_by: 'joshua',
      },
    });
    const m3 = createMemory(id3, {
      status: 'active',
      evolution: { supersedes: [id2] },
    });
    const service = new EvolutionService({ store: createInMemoryStore([m1, m2, m3]) });

    const chain = await service.getEvolutionChain(id2);

    expect(chain.map((memory) => memory.id)).toEqual([id1, id2, id3]);
  });

  it('terminates evolution chain traversal when supersedes links form a cycle', async () => {
    const idA = createMemoryId('550e8400-e29b-41d4-a716-446655440041');
    const idB = createMemoryId('550e8400-e29b-41d4-a716-446655440042');
    const memoryA = createMemory(idA, {
      status: 'superseded',
      evolution: { supersedes: [idB], superseded_by: idB },
    });
    const memoryB = createMemory(idB, {
      status: 'superseded',
      evolution: { supersedes: [idA], superseded_by: idA },
    });
    const service = new EvolutionService({ store: createInMemoryStore([memoryA, memoryB]) });

    const chain = await service.getEvolutionChain(idA);

    expect(chain).not.toHaveLength(0);
    expect(chain.map((memory) => memory.id)).toEqual([idB, idA]);
  });

  it('follows superseded_by links to find latest version', async () => {
    const id1 = createMemoryId('550e8400-e29b-41d4-a716-446655440021');
    const id2 = createMemoryId('550e8400-e29b-41d4-a716-446655440022');
    const id3 = createMemoryId('550e8400-e29b-41d4-a716-446655440023');
    const service = new EvolutionService({
      store: createInMemoryStore([
        createMemory(id1, {
          status: 'superseded',
          evolution: { supersedes: [], superseded_by: id2 },
        }),
        createMemory(id2, {
          status: 'superseded',
          evolution: { supersedes: [id1], superseded_by: id3 },
        }),
        createMemory(id3, { status: 'active', evolution: { supersedes: [id2] } }),
      ]),
    });

    const latest = await service.getLatestVersion(id1);

    expect(latest?.id).toBe(id3);
    expect(latest?.status).toBe('active');
  });

  it('throws when superseding a memory that does not exist', async () => {
    const nonExistentId = createMemoryId('550e8400-e29b-41d4-a716-446655440099');
    const service = new EvolutionService({ store: createInMemoryStore([]) });

    await expect(service.supersedeMemory(nonExistentId, createCreateMemoryInput())).rejects.toThrow(
      'Memory not found'
    );
  });

  it('returns null when retiring a memory that does not exist', async () => {
    const nonExistentId = createMemoryId('550e8400-e29b-41d4-a716-446655440098');
    const service = new EvolutionService({ store: createInMemoryStore([]) });

    const result = await service.retireMemory(nonExistentId, {
      reason: 'Test retirement',
      retired_by: 'joshua',
    });

    expect(result).toBeNull();
  });

  it('returns empty array when getting evolution chain for non-existent memory', async () => {
    const nonExistentId = createMemoryId('550e8400-e29b-41d4-a716-446655440097');
    const service = new EvolutionService({ store: createInMemoryStore([]) });

    const chain = await service.getEvolutionChain(nonExistentId);

    expect(chain).toEqual([]);
  });

  it('returns null when getting latest version of a memory that does not exist', async () => {
    const nonExistentId = createMemoryId('550e8400-e29b-41d4-a716-446655440096');
    const service = new EvolutionService({ store: createInMemoryStore([]) });

    const latest = await service.getLatestVersion(nonExistentId);

    expect(latest).toBeNull();
  });

  it('breaks cycle detection when following superseded_by forms a loop', async () => {
    const idA = createMemoryId('550e8400-e29b-41d4-a716-446655440051');
    const idB = createMemoryId('550e8400-e29b-41d4-a716-446655440052');
    const memoryA = createMemory(idA, {
      status: 'superseded',
      evolution: { supersedes: [], superseded_by: idB },
    });
    const memoryB = createMemory(idB, {
      status: 'superseded',
      evolution: { supersedes: [idA], superseded_by: idA },
    });
    const service = new EvolutionService({ store: createInMemoryStore([memoryA, memoryB]) });

    const latest = await service.getLatestVersion(idA);

    expect(latest?.id).toBe(idB);
  });

  it('stops traversal when superseded_by target does not exist in store', async () => {
    const idA = createMemoryId('550e8400-e29b-41d4-a716-446655440061');
    const idB = createMemoryId('550e8400-e29b-41d4-a716-446655440062');
    const memoryA = createMemory(idA, {
      status: 'superseded',
      evolution: { supersedes: [], superseded_by: idB },
    });
    const service = new EvolutionService({ store: createInMemoryStore([memoryA]) });

    const latest = await service.getLatestVersion(idA);

    expect(latest?.id).toBe(idA);
  });

  describe('terminal-state immutability and race safety (CIB-118)', () => {
    it('retiring an already retired memory is a no-op that preserves the original retirement record', async () => {
      const id = createMemoryId('550e8400-e29b-41d4-a716-446655440071');
      const retired = createMemory(id, {
        status: 'retired',
        evolution: {
          supersedes: [],
          retired_at: '2026-03-01T10:00:00.000Z',
          retired_reason: 'original reason',
          retired_by: 'joshua',
        },
      });
      const store = createInMemoryStore([retired]);
      const service = new EvolutionService({ store });

      const result = await service.retireMemory(id, {
        reason: 'second reason',
        retired_by: 'someone-else',
      });

      expect(result?.status).toBe('retired');
      expect(result?.evolution.retired_reason).toBe('original reason');
      expect(result?.evolution.retired_by).toBe('joshua');
      expect(store.saveMemory).not.toHaveBeenCalled();
    });

    it('retiring a superseded memory preserves the superseded_by link', async () => {
      const idA = createMemoryId('550e8400-e29b-41d4-a716-446655440072');
      const idB = createMemoryId('550e8400-e29b-41d4-a716-446655440073');
      const superseded = createMemory(idA, {
        status: 'superseded',
        evolution: { supersedes: [], superseded_by: idB },
      });
      const store = createInMemoryStore([superseded]);
      const service = new EvolutionService({ store });

      const result = await service.retireMemory(idA, {
        reason: 'cleanup sweep',
        retired_by: 'joshua',
      });

      expect(result?.status).toBe('superseded');
      expect(result?.evolution.superseded_by).toBe(idB);
      expect(store.saveMemory).not.toHaveBeenCalled();
    });

    it('retireMemoryById is a no-op for memories already in a terminal state', async () => {
      const id = createMemoryId('550e8400-e29b-41d4-a716-446655440074');
      const retired = createMemory(id, {
        status: 'retired',
        evolution: {
          supersedes: [],
          retired_at: '2026-03-01T10:00:00.000Z',
          retired_reason: 'original reason',
          retired_by: 'joshua',
        },
      });
      const store = createInMemoryStore([retired]);
      const service = new EvolutionService({ store });

      await service.retireMemoryById(id, undefined, 'second reason', 'someone-else');

      expect(store.saveMemory).not.toHaveBeenCalled();
      const current = await store.getMemory(id);
      expect(current?.evolution.retired_reason).toBe('original reason');
    });

    it('serialises concurrent supersedes of the same memory so only one replacement wins', async () => {
      const id = createMemoryId('550e8400-e29b-41d4-a716-446655440075');
      const store = createInMemoryStore([createMemory(id)]);
      const service = new EvolutionService({ store });

      // Deterministic interleaving: both calls are started before either
      // completes, so without serialisation both read status 'active'.
      const outcomes = await Promise.allSettled([
        service.supersedeMemory(id, createCreateMemoryInput()),
        service.supersedeMemory(id, createCreateMemoryInput()),
      ]);

      const fulfilled = outcomes.filter((outcome) => outcome.status === 'fulfilled');
      const rejected = outcomes.filter((outcome) => outcome.status === 'rejected');
      expect(fulfilled).toHaveLength(1);
      expect(rejected).toHaveLength(1);
      expect((rejected[0] as PromiseRejectedResult).reason.message).toContain("must be 'active'");

      const replacements = (await store.getActiveMemories()).filter((memory) =>
        memory.evolution.supersedes.includes(id)
      );
      expect(replacements).toHaveLength(1);
    });
  });
});

function createVersionTrackerMock(): IVersionTracker {
  return {
    init: vi.fn(async () => undefined),
    trackChange: vi.fn(async () => 'hash-1'),
    getHistory: vi.fn(async () => []),
    isInitialised: vi.fn(async () => true),
  };
}

function createInMemoryStore(initial: MemoryObject[]): IMemoryStoreOperations {
  const store = new Map(initial.map((memory) => [memory.id, memory]));

  return {
    getMemory: vi.fn(async (id) => store.get(id) ?? null),
    saveMemory: vi.fn(async (memory) => {
      store.set(memory.id, memory);
    }),
    getMemoryByProposalId: vi.fn(async () => null),
    queryMemories: vi.fn(async () => ({
      memories: [],
      total: 0,
      limit: 100,
      offset: 0,
      has_more: false,
    })),
    getActiveMemories: vi.fn(async () =>
      Array.from(store.values()).filter((memory) => memory.status === 'active')
    ),
    getMemoriesByType: vi.fn(async (type) =>
      Array.from(store.values()).filter((memory) => memory.type === type)
    ),
    searchMemories: vi.fn(async (searchText) =>
      Array.from(store.values()).filter((memory) => memory.statement.includes(searchText))
    ),
    memoryExists: vi.fn(async (id) => store.has(id)),
    countMemories: vi.fn(async () => store.size),
    getStats: vi.fn(async () => ({
      total_memories: store.size,
      by_status: [],
      by_type: [],
      by_confidence: [],
      active_count: Array.from(store.values()).filter((memory) => memory.status === 'active')
        .length,
      superseded_count: Array.from(store.values()).filter(
        (memory) => memory.status === 'superseded'
      ).length,
      retired_count: Array.from(store.values()).filter((memory) => memory.status === 'retired')
        .length,
      unique_tags_count: 0,
    })),
    isAvailable: vi.fn(async () => true),
    exportMemories: vi.fn(async () => Array.from(store.values())),
    importMemories: vi.fn(async (memories) => {
      for (const memory of memories) {
        store.set(memory.id, memory);
      }
      return memories.length;
    }),
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

function createCreateMemoryInput(): CreateMemoryInput {
  return {
    type: 'decision',
    statement: 'Updated memory statement',
    context: {
      when: '2026-03-02T10:00:00.000Z',
      why: 'Superseded with better evidence',
      conditions: ['Updated behaviour observed'],
      tags: ['evolution'],
    },
    confidence: 'high',
    confidence_rationale: 'Confirmed by repeated observation',
    provenance: {
      kindling_sources: [
        {
          observation_id: createObservationId('550e8400-e29b-41d4-a716-446655440201'),
          session_id: createSessionId('550e8400-e29b-41d4-a716-446655440202'),
          kind: 'gate_evaluated',
          timestamp: '2026-03-02T10:00:00.000Z',
        },
      ],
      source_sessions: [createSessionId('550e8400-e29b-41d4-a716-446655440202')],
    },
    created_by: 'joshua',
    reason: 'Refined canonical memory based on new evidence',
  };
}
