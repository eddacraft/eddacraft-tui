/**
 * Tests for IMemoryStoreOperations and IVersionTracker contracts.
 *
 * These tests validate each method of the two store interfaces defined in
 * store-interfaces.ts by exercising concrete in-memory implementations.
 * store-interfaces.ts is type-only (no runtime code), so coverage is
 * demonstrated through usage, not line metrics.
 */
import { describe, expect, it, vi } from 'vitest';
import {
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
} from '../contracts/identifiers.js';
import type {
  MemoryObject,
  MemoryQuery,
  MemoryQueryResult,
  MemoryStatus,
  MemoryType,
} from '../contracts/edda-memory.js';
import type { MemoryId, ProposalId } from '../contracts/identifiers.js';
import type { EddaStats } from '../contracts/ports/edda.port.js';
import type { IMemoryStoreOperations, IVersionTracker, VersionEntry } from './store-interfaces.js';

// =============================================================================
// Helpers
// =============================================================================

function uuidFromSuffix(suffix: string): string {
  return `550e8400-e29b-41d4-a716-${suffix.padStart(12, '0')}`;
}

function buildMemory(overrides: Partial<MemoryObject> = {}, n = 1): MemoryObject {
  const memoryId = createMemoryId(uuidFromSuffix(String(n)));
  const sessionId = createSessionId(uuidFromSuffix(String(100 + n)));
  const observationId = createObservationId(uuidFromSuffix(String(200 + n)));
  const proposalId = createProposalId(uuidFromSuffix(String(300 + n)));

  return {
    id: memoryId,
    type: 'pattern',
    status: 'active',
    schema_version: 1,
    statement: `Memory statement ${n}`,
    context: {
      when: '2026-Q1',
      why: 'Test coverage',
      conditions: [],
      tags: ['test'],
    },
    confidence: 'high',
    provenance: {
      ember_source: {
        proposal_id: proposalId,
        proposal_type: 'pattern',
        confidence: 0.8,
        created_at: '2026-01-01T00:00:00.000Z',
      },
      kindling_sources: [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: '2026-01-01T00:01:00.000Z',
        },
      ],
      source_sessions: [sessionId],
    },
    attribution: {
      actor: 'agent/test',
      timestamp: '2026-01-01T00:02:00.000Z',
      method: 'cli_command',
      reason: 'Test memory',
    },
    evolution: { supersedes: [] },
    created_at: `2026-01-0${Math.min(n, 9)}T00:00:00.000Z`,
    ...overrides,
  };
}

// =============================================================================
// In-memory implementation of IMemoryStoreOperations
// =============================================================================

/**
 * Minimal concrete implementation used to verify that the IMemoryStoreOperations
 * contract can be satisfied and each method behaves as specified.
 */
class InMemoryStore implements IMemoryStoreOperations {
  private readonly data = new Map<MemoryId, MemoryObject>();
  private readonly proposalIndex = new Map<ProposalId, MemoryId>();

  async getMemory(id: MemoryId): Promise<MemoryObject | null> {
    return this.data.get(id) ?? null;
  }

  async saveMemory(memory: MemoryObject): Promise<void> {
    this.data.set(memory.id, memory);
    const proposalId = memory.provenance.ember_source?.proposal_id;
    if (proposalId) {
      this.proposalIndex.set(proposalId, memory.id);
    }
  }

  async getMemoryByProposalId(proposalId: ProposalId): Promise<MemoryObject | null> {
    const memoryId = this.proposalIndex.get(proposalId);
    if (!memoryId) return null;
    return this.data.get(memoryId) ?? null;
  }

  async queryMemories(query: MemoryQuery): Promise<MemoryQueryResult> {
    let results = Array.from(this.data.values());

    if (query.statuses && query.statuses.length > 0) {
      results = results.filter((m) => query.statuses!.includes(m.status));
    }
    if (query.types && query.types.length > 0) {
      results = results.filter((m) => query.types!.includes(m.type));
    }
    if (!query.include_superseded) {
      results = results.filter((m) => m.status !== 'superseded');
    }

    const total = results.length;
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 100;
    const page = results.slice(offset, offset + limit);

    return {
      memories: page,
      total,
      limit,
      offset,
      has_more: offset + limit < total,
    };
  }

  async getActiveMemories(): Promise<MemoryObject[]> {
    return Array.from(this.data.values()).filter((m) => m.status === 'active');
  }

  async getMemoriesByType(type: MemoryType): Promise<MemoryObject[]> {
    return Array.from(this.data.values()).filter((m) => m.type === type);
  }

  async searchMemories(searchText: string): Promise<MemoryObject[]> {
    const lower = searchText.toLowerCase();
    return Array.from(this.data.values()).filter((m) => m.statement.toLowerCase().includes(lower));
  }

  async memoryExists(id: MemoryId): Promise<boolean> {
    return this.data.has(id);
  }

  async countMemories(filter?: { status?: MemoryStatus; type?: MemoryType }): Promise<number> {
    if (!filter) return this.data.size;

    let count = 0;
    for (const m of this.data.values()) {
      const statusMatch = !filter.status || m.status === filter.status;
      const typeMatch = !filter.type || m.type === filter.type;
      if (statusMatch && typeMatch) count++;
    }
    return count;
  }

  async getStats(): Promise<EddaStats> {
    const memories = Array.from(this.data.values());
    const byType = new Map<MemoryType, number>();
    const byConfidence = new Map<string, number>();
    const allTags = new Set<string>();

    let activeCount = 0;
    let supersededCount = 0;
    let retiredCount = 0;
    let oldest: string | undefined;
    let mostRecent: string | undefined;

    for (const m of memories) {
      if (m.status === 'active') activeCount++;
      else if (m.status === 'superseded') supersededCount++;
      else if (m.status === 'retired') retiredCount++;

      byType.set(m.type, (byType.get(m.type) ?? 0) + 1);
      byConfidence.set(m.confidence, (byConfidence.get(m.confidence) ?? 0) + 1);
      for (const tag of m.context.tags ?? []) allTags.add(tag);

      if (!oldest || m.created_at < oldest) oldest = m.created_at;
      if (!mostRecent || m.created_at > mostRecent) mostRecent = m.created_at;
    }

    return {
      total_memories: memories.length,
      by_status: [
        { status: 'active', count: activeCount },
        { status: 'superseded', count: supersededCount },
        { status: 'retired', count: retiredCount },
      ],
      by_type: Array.from(byType.entries()).map(([type, count]) => ({ type, count })),
      by_confidence: Array.from(byConfidence.entries()).map(([level, count]) => ({
        level: level as 'high' | 'medium' | 'low',
        count,
      })),
      active_count: activeCount,
      superseded_count: supersededCount,
      retired_count: retiredCount,
      oldest_memory: oldest,
      most_recent: mostRecent,
      unique_tags_count: allTags.size,
    };
  }

  async isAvailable(): Promise<boolean> {
    return true;
  }

  async exportMemories(): Promise<MemoryObject[]> {
    return Array.from(this.data.values());
  }

  async importMemories(memories: MemoryObject[]): Promise<number> {
    for (const m of memories) {
      await this.saveMemory(m);
    }
    return memories.length;
  }
}

// =============================================================================
// In-memory implementation of IVersionTracker
// =============================================================================

class InMemoryVersionTracker implements IVersionTracker {
  private initialised = false;
  private readonly history = new Map<string, VersionEntry[]>();

  async init(): Promise<void> {
    this.initialised = true;
  }

  async trackChange(filePaths: string[], message: string, author: string): Promise<string> {
    if (!this.initialised) {
      await this.init();
    }
    const hash = `hash-${Date.now()}`;
    const entry: VersionEntry = {
      hash,
      message,
      author,
      timestamp: new Date().toISOString(),
    };
    for (const path of filePaths) {
      const existing = this.history.get(path) ?? [];
      this.history.set(path, [entry, ...existing]);
    }
    return hash;
  }

  async getHistory(filePath: string, limit = 10): Promise<VersionEntry[]> {
    const entries = this.history.get(filePath) ?? [];
    return entries.slice(0, limit);
  }

  async isInitialised(): Promise<boolean> {
    return this.initialised;
  }
}

// =============================================================================
// IMemoryStoreOperations tests
// =============================================================================

describe('IMemoryStoreOperations contract (store-interfaces)', () => {
  it('saveMemory and getMemory round-trip preserves the full object', async () => {
    const store = new InMemoryStore();
    const memory = buildMemory({}, 1);

    await store.saveMemory(memory);
    const fetched = await store.getMemory(memory.id);

    expect(fetched).toEqual(memory);
  });

  it('getMemory returns null for unknown id', async () => {
    const store = new InMemoryStore();
    const unknownId = createMemoryId(uuidFromSuffix('999'));

    expect(await store.getMemory(unknownId)).toBeNull();
  });

  it('getMemoryByProposalId finds memory via ember_source proposal_id', async () => {
    const store = new InMemoryStore();
    const memory = buildMemory({}, 2);
    const proposalId = memory.provenance.ember_source!.proposal_id;

    await store.saveMemory(memory);
    const found = await store.getMemoryByProposalId(proposalId);

    expect(found).toEqual(memory);
  });

  it('getMemoryByProposalId returns null when proposal_id is not indexed', async () => {
    const store = new InMemoryStore();
    const unknownProposalId = createProposalId(uuidFromSuffix('888'));

    expect(await store.getMemoryByProposalId(unknownProposalId)).toBeNull();
  });

  it('queryMemories filters by status', async () => {
    const store = new InMemoryStore();
    const active = buildMemory({ status: 'active' }, 3);
    const retired = buildMemory({ status: 'retired' }, 4);

    await store.importMemories([active, retired]);

    const result = await store.queryMemories({
      statuses: ['retired'],
      include_superseded: false,
      limit: 100,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    expect(result.memories).toHaveLength(1);
    expect(result.memories[0]?.id).toBe(retired.id);
  });

  it('queryMemories filters by type', async () => {
    const store = new InMemoryStore();
    const pattern = buildMemory({ type: 'pattern', status: 'active' }, 5);
    const decision = buildMemory({ type: 'decision', status: 'active' }, 6);

    await store.importMemories([pattern, decision]);

    const result = await store.queryMemories({
      types: ['decision'],
      statuses: ['active'],
      include_superseded: false,
      limit: 100,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    expect(result.memories).toHaveLength(1);
    expect(result.memories[0]?.id).toBe(decision.id);
  });

  it('queryMemories excludes superseded memories when include_superseded is false', async () => {
    const store = new InMemoryStore();
    const active = buildMemory({ status: 'active' }, 7);
    const superseded = buildMemory({ status: 'superseded' }, 8);

    await store.importMemories([active, superseded]);

    const result = await store.queryMemories({
      include_superseded: false,
      limit: 100,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    expect(result.memories.every((m) => m.status !== 'superseded')).toBe(true);
  });

  it('queryMemories includes superseded memories when include_superseded is true', async () => {
    const store = new InMemoryStore();
    const active = buildMemory({ status: 'active' }, 9);
    const superseded = buildMemory({ status: 'superseded' }, 10);

    await store.importMemories([active, superseded]);

    const result = await store.queryMemories({
      include_superseded: true,
      limit: 100,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    expect(result.total).toBe(2);
  });

  it('queryMemories paginates correctly', async () => {
    const store = new InMemoryStore();
    const memories = [
      buildMemory({ status: 'active' }, 11),
      buildMemory({ status: 'active' }, 12),
      buildMemory({ status: 'active' }, 13),
    ];

    await store.importMemories(memories);

    const page1 = await store.queryMemories({
      include_superseded: false,
      limit: 2,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    expect(page1.memories).toHaveLength(2);
    expect(page1.has_more).toBe(true);
    expect(page1.total).toBe(3);

    const page2 = await store.queryMemories({
      include_superseded: false,
      limit: 2,
      offset: 2,
      sort_by: 'created_at',
      sort_order: 'desc',
    });

    expect(page2.memories).toHaveLength(1);
    expect(page2.has_more).toBe(false);
  });

  it('getActiveMemories returns only active-status memories', async () => {
    const store = new InMemoryStore();
    const active = buildMemory({ status: 'active' }, 14);
    const retired = buildMemory({ status: 'retired' }, 15);
    const superseded = buildMemory({ status: 'superseded' }, 16);

    await store.importMemories([active, retired, superseded]);

    const result = await store.getActiveMemories();

    expect(result).toHaveLength(1);
    expect(result[0]?.id).toBe(active.id);
  });

  it('getMemoriesByType returns only memories of the specified type', async () => {
    const store = new InMemoryStore();
    const pattern = buildMemory({ type: 'pattern' }, 17);
    const decision = buildMemory({ type: 'decision' }, 18);
    const warning = buildMemory({ type: 'warning' }, 19);

    await store.importMemories([pattern, decision, warning]);

    const patterns = await store.getMemoriesByType('pattern');
    expect(patterns).toHaveLength(1);
    expect(patterns[0]?.id).toBe(pattern.id);

    const decisions = await store.getMemoriesByType('decision');
    expect(decisions).toHaveLength(1);
    expect(decisions[0]?.id).toBe(decision.id);

    const constraints = await store.getMemoriesByType('constraint');
    expect(constraints).toHaveLength(0);
  });

  it('searchMemories returns memories matching text case-insensitively', async () => {
    const store = new InMemoryStore();
    const m1 = buildMemory({ statement: 'Always run tests before merging.' }, 20);
    const m2 = buildMemory({ statement: 'Use deterministic checks.' }, 21);
    const m3 = buildMemory({ statement: 'Prefer small atomic commits.' }, 22);

    await store.importMemories([m1, m2, m3]);

    const results = await store.searchMemories('TESTS');
    expect(results).toHaveLength(1);
    expect(results[0]?.id).toBe(m1.id);
  });

  it('searchMemories returns empty array when no matches exist', async () => {
    const store = new InMemoryStore();
    await store.saveMemory(buildMemory({ statement: 'Run lints.' }, 23));

    expect(await store.searchMemories('nonexistent keyword')).toHaveLength(0);
  });

  it('memoryExists returns true for saved memories and false otherwise', async () => {
    const store = new InMemoryStore();
    const memory = buildMemory({}, 24);

    expect(await store.memoryExists(memory.id)).toBe(false);
    await store.saveMemory(memory);
    expect(await store.memoryExists(memory.id)).toBe(true);
  });

  it('countMemories with no filter returns total', async () => {
    const store = new InMemoryStore();
    await store.importMemories([
      buildMemory({ status: 'active', type: 'pattern' }, 25),
      buildMemory({ status: 'retired', type: 'warning' }, 26),
      buildMemory({ status: 'superseded', type: 'decision' }, 27),
    ]);

    expect(await store.countMemories()).toBe(3);
  });

  it('countMemories filters by status', async () => {
    const store = new InMemoryStore();
    await store.importMemories([
      buildMemory({ status: 'active' }, 28),
      buildMemory({ status: 'active' }, 29),
      buildMemory({ status: 'retired' }, 30),
    ]);

    expect(await store.countMemories({ status: 'active' })).toBe(2);
    expect(await store.countMemories({ status: 'retired' })).toBe(1);
    expect(await store.countMemories({ status: 'superseded' })).toBe(0);
  });

  it('countMemories filters by type', async () => {
    const store = new InMemoryStore();
    await store.importMemories([
      buildMemory({ type: 'pattern' }, 31),
      buildMemory({ type: 'pattern' }, 32),
      buildMemory({ type: 'decision' }, 33),
    ]);

    expect(await store.countMemories({ type: 'pattern' })).toBe(2);
    expect(await store.countMemories({ type: 'decision' })).toBe(1);
    expect(await store.countMemories({ type: 'constraint' })).toBe(0);
  });

  it('countMemories filters by both status and type', async () => {
    const store = new InMemoryStore();
    await store.importMemories([
      buildMemory({ type: 'pattern', status: 'active' }, 34),
      buildMemory({ type: 'pattern', status: 'retired' }, 35),
      buildMemory({ type: 'decision', status: 'active' }, 36),
    ]);

    expect(await store.countMemories({ type: 'pattern', status: 'active' })).toBe(1);
    expect(await store.countMemories({ type: 'pattern', status: 'retired' })).toBe(1);
  });

  it('getStats returns accurate counts across statuses and types', async () => {
    const store = new InMemoryStore();
    await store.importMemories([
      buildMemory({ type: 'pattern', status: 'active', confidence: 'high' }, 37),
      buildMemory({ type: 'warning', status: 'retired', confidence: 'medium' }, 38),
      buildMemory({ type: 'decision', status: 'superseded', confidence: 'low' }, 39),
    ]);

    const stats = await store.getStats();

    expect(stats.total_memories).toBe(3);
    expect(stats.active_count).toBe(1);
    expect(stats.retired_count).toBe(1);
    expect(stats.superseded_count).toBe(1);
    expect(stats.by_type.find((s) => s.type === 'pattern')?.count).toBe(1);
    expect(stats.by_type.find((s) => s.type === 'warning')?.count).toBe(1);
    expect(stats.by_confidence.find((s) => s.level === 'high')?.count).toBe(1);
    expect(stats.unique_tags_count).toBeGreaterThanOrEqual(0);
    expect(stats.oldest_memory).toBeDefined();
    expect(stats.most_recent).toBeDefined();
  });

  it('getStats returns empty stats for an empty store', async () => {
    const store = new InMemoryStore();

    const stats = await store.getStats();

    expect(stats.total_memories).toBe(0);
    expect(stats.active_count).toBe(0);
    expect(stats.superseded_count).toBe(0);
    expect(stats.retired_count).toBe(0);
    expect(stats.oldest_memory).toBeUndefined();
    expect(stats.most_recent).toBeUndefined();
  });

  it('isAvailable returns true when the store is operational', async () => {
    const store = new InMemoryStore();

    expect(await store.isAvailable()).toBe(true);
  });

  it('exportMemories returns all saved memories', async () => {
    const store = new InMemoryStore();
    const memories = [buildMemory({}, 40), buildMemory({}, 41)];
    await store.importMemories(memories);

    const exported = await store.exportMemories();

    expect(exported).toHaveLength(2);
  });

  it('importMemories returns the count of imported items', async () => {
    const store = new InMemoryStore();
    const memories = [buildMemory({}, 42), buildMemory({}, 43), buildMemory({}, 44)];

    const count = await store.importMemories(memories);

    expect(count).toBe(3);
    expect(await store.countMemories()).toBe(3);
  });

  it('importMemories overwrites existing memories with matching ids', async () => {
    const store = new InMemoryStore();
    const memory = buildMemory({ statement: 'Original statement.' }, 45);
    await store.saveMemory(memory);

    const updated = { ...memory, statement: 'Updated statement.' };
    await store.importMemories([updated]);

    const fetched = await store.getMemory(memory.id);
    expect(fetched?.statement).toBe('Updated statement.');
    expect(await store.countMemories()).toBe(1);
  });
});

// =============================================================================
// IVersionTracker tests
// =============================================================================

describe('IVersionTracker contract (store-interfaces)', () => {
  it('isInitialised returns false before init is called', async () => {
    const tracker = new InMemoryVersionTracker();

    expect(await tracker.isInitialised()).toBe(false);
  });

  it('init marks tracker as initialised', async () => {
    const tracker = new InMemoryVersionTracker();

    await tracker.init();

    expect(await tracker.isInitialised()).toBe(true);
  });

  it('init is idempotent when called multiple times', async () => {
    const tracker = new InMemoryVersionTracker();

    await tracker.init();
    await tracker.init();

    expect(await tracker.isInitialised()).toBe(true);
  });

  it('trackChange returns a non-empty hash string', async () => {
    const tracker = new InMemoryVersionTracker();
    await tracker.init();

    const hash = await tracker.trackChange(['memories/pattern/test.yaml'], 'Save memory', 'agent');

    expect(typeof hash).toBe('string');
    expect(hash.length).toBeGreaterThan(0);
  });

  it('trackChange auto-initialises the tracker if not yet initialised', async () => {
    const tracker = new InMemoryVersionTracker();

    const hash = await tracker.trackChange(['index.yaml'], 'Initial commit', 'agent');

    expect(await tracker.isInitialised()).toBe(true);
    expect(hash.length).toBeGreaterThan(0);
  });

  it('getHistory returns entries in reverse-chronological order (latest first)', async () => {
    const tracker = new InMemoryVersionTracker();
    await tracker.init();

    await tracker.trackChange(['index.yaml'], 'First change', 'alice');
    await tracker.trackChange(['index.yaml'], 'Second change', 'bob');

    const history = await tracker.getHistory('index.yaml');

    expect(history).toHaveLength(2);
    expect(history[0]?.message).toBe('Second change');
    expect(history[1]?.message).toBe('First change');
  });

  it('getHistory respects the limit parameter', async () => {
    const tracker = new InMemoryVersionTracker();
    await tracker.init();

    await tracker.trackChange(['index.yaml'], 'Change 1', 'agent');
    await tracker.trackChange(['index.yaml'], 'Change 2', 'agent');
    await tracker.trackChange(['index.yaml'], 'Change 3', 'agent');

    const limited = await tracker.getHistory('index.yaml', 2);

    expect(limited).toHaveLength(2);
  });

  it('getHistory returns empty array for a path with no tracked changes', async () => {
    const tracker = new InMemoryVersionTracker();
    await tracker.init();

    const history = await tracker.getHistory('memories/unknown.yaml');

    expect(history).toEqual([]);
  });

  it('getHistory returns empty array when limit is 0', async () => {
    const tracker = new InMemoryVersionTracker();
    await tracker.init();
    await tracker.trackChange(['index.yaml'], 'A change', 'agent');

    const result = await tracker.getHistory('index.yaml', 0);

    expect(result).toEqual([]);
  });

  it('trackChange records history independently per file path', async () => {
    const tracker = new InMemoryVersionTracker();
    await tracker.init();

    await tracker.trackChange(['memories/a.yaml', 'memories/b.yaml'], 'Batch commit', 'agent');

    const histA = await tracker.getHistory('memories/a.yaml');
    const histB = await tracker.getHistory('memories/b.yaml');
    const histC = await tracker.getHistory('memories/c.yaml');

    expect(histA).toHaveLength(1);
    expect(histB).toHaveLength(1);
    expect(histC).toHaveLength(0);
  });

  it('VersionEntry shape satisfies the interface (hash, message, author, timestamp)', async () => {
    const tracker = new InMemoryVersionTracker();
    await tracker.init();
    await tracker.trackChange(['index.yaml'], 'Verify shape', 'joshua');

    const [entry] = await tracker.getHistory('index.yaml');

    expect(entry).toMatchObject({
      hash: expect.any(String) as string,
      message: 'Verify shape',
      author: 'joshua',
      timestamp: expect.any(String) as string,
    });
  });
});

// =============================================================================
// Mock interchangeability
// =============================================================================

describe('IMemoryStoreOperations used as vi.fn() mock (store-interfaces)', () => {
  it('fulfils the interface when constructed entirely from vi.fn spies', async () => {
    const store: IMemoryStoreOperations = {
      getMemory: vi.fn(async () => null),
      saveMemory: vi.fn(async () => undefined),
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

    const memoryId = createMemoryId(uuidFromSuffix('999'));

    expect(await store.getMemory(memoryId)).toBeNull();
    expect(await store.memoryExists(memoryId)).toBe(false);
    expect(await store.isAvailable()).toBe(true);
    expect(await store.countMemories()).toBe(0);
    expect(await store.exportMemories()).toEqual([]);

    const stats = await store.getStats();
    expect(stats.total_memories).toBe(0);
  });

  it('fulfils IVersionTracker when constructed from vi.fn spies', async () => {
    const tracker: IVersionTracker = {
      init: vi.fn(async () => undefined),
      trackChange: vi.fn(async () => 'mock-hash'),
      getHistory: vi.fn(async () => []),
      isInitialised: vi.fn(async () => false),
    };

    await tracker.init();
    const hash = await tracker.trackChange(['file.yaml'], 'msg', 'agent');
    const history = await tracker.getHistory('file.yaml', 5);
    const ready = await tracker.isInitialised();

    expect(hash).toBe('mock-hash');
    expect(history).toEqual([]);
    expect(ready).toBe(false);
    expect(tracker.init).toHaveBeenCalledOnce();
    expect(tracker.trackChange).toHaveBeenCalledWith(['file.yaml'], 'msg', 'agent');
  });
});
