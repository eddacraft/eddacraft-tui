import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { describe, expect, it } from 'vitest';
import {
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
} from '../contracts/identifiers.js';
import type { MemoryObject, MemoryQuery } from '../contracts/index.js';
import { MemoryStore } from './memory-store.js';
import { deserialiseIndex } from './serialisation.js';

function uuidFromNumber(value: number): string {
  return `550e8400-e29b-41d4-a716-${value.toString().padStart(12, '0')}`;
}

function createMemory(overrides: Partial<MemoryObject> = {}, number = 1): MemoryObject {
  const memoryId = createMemoryId(uuidFromNumber(number));
  const sessionId = createSessionId(uuidFromNumber(100 + number));
  const observationId = createObservationId(uuidFromNumber(200 + number));
  const proposalId = createProposalId(uuidFromNumber(300 + number));

  return {
    id: memoryId,
    type: 'pattern',
    status: 'active',
    schema_version: 1,
    statement: `Memory statement ${number}`,
    context: {
      when: '2026-Q1',
      why: 'Regression prevention',
      conditions: ['changes touch runtime behaviour'],
      scope: 'monorepo',
      tags: ['quality', 'test'],
    },
    confidence: 'high',
    confidence_rationale: 'Repeated evidence from production incidents.',
    provenance: {
      ember_source: {
        proposal_id: proposalId,
        proposal_type: 'pattern',
        confidence: 0.8,
        created_at: '2026-02-01T09:00:00.000Z',
      },
      kindling_sources: [
        {
          observation_id: observationId,
          session_id: sessionId,
          kind: 'gate_evaluated',
          timestamp: '2026-02-01T09:05:00.000Z',
        },
      ],
      source_sessions: [sessionId],
    },
    attribution: {
      actor: 'agent/curator',
      timestamp: '2026-02-01T09:06:00.000Z',
      method: 'cli_command',
      reason: 'Promoted by reviewer',
    },
    evolution: {
      supersedes: [],
    },
    created_at: `2026-02-0${Math.min(number, 9)}T09:06:00.000Z`,
    ...overrides,
  };
}

function queryDefaults(overrides: Partial<MemoryQuery> = {}): MemoryQuery {
  return {
    types: undefined,
    statuses: undefined,
    confidence_levels: undefined,
    created_after: undefined,
    created_before: undefined,
    tags: undefined,
    search: undefined,
    include_superseded: false,
    limit: 100,
    offset: 0,
    sort_by: 'created_at',
    sort_order: 'desc',
    ...overrides,
  };
}

describe('MemoryStore (EDDA-006)', () => {
  it('supports save/get/delete memory with index updates', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-store-'));
    const store = new MemoryStore({ type: 'git', path: storagePath, format: 'yaml' });

    try {
      const memory = createMemory({}, 1);
      await store.saveMemory(memory);

      expect(await store.isAvailable()).toBe(true);
      expect(await store.memoryExists(memory.id)).toBe(true);

      const fetched = await store.getMemory(memory.id);
      expect(fetched).toEqual(memory);

      const deleted = await store.deleteMemory(memory.id);
      expect(deleted).toBe(true);
      expect(await store.memoryExists(memory.id)).toBe(false);
      expect(await store.getMemory(memory.id)).toBeNull();
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('filters and paginates queries from index-backed metadata', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-store-'));
    const store = new MemoryStore({ type: 'git', path: storagePath, format: 'yaml' });

    try {
      const first = createMemory(
        {
          type: 'pattern',
          status: 'active',
          confidence: 'high',
          context: {
            when: '2026-Q1',
            why: 'Pattern codification',
            conditions: ['pattern repeats'],
            scope: 'runtime',
            tags: ['quality', 'repeat'],
          },
          created_at: '2026-02-01T10:00:00.000Z',
        },
        2
      );
      const second = createMemory(
        {
          type: 'warning',
          status: 'retired',
          confidence: 'medium',
          statement: 'Avoid unstable third-party APIs in critical flows.',
          context: {
            when: '2026-Q1',
            why: 'Downtime incidents',
            conditions: ['external API risk'],
            scope: 'network',
            tags: ['risk', 'api'],
          },
          created_at: '2026-02-02T10:00:00.000Z',
        },
        3
      );
      const third = createMemory(
        {
          type: 'decision',
          status: 'superseded',
          confidence: 'low',
          statement: 'Prefer deterministic adapters for planner workflows.',
          context: {
            when: '2026-Q1',
            why: 'Design evolution',
            conditions: ['planner rewrites'],
            scope: 'planning',
            tags: ['quality', 'planning'],
          },
          created_at: '2026-02-03T10:00:00.000Z',
        },
        4
      );

      await store.importMemories([first, second, third]);

      const result = await store.queryMemories(
        queryDefaults({
          types: ['pattern', 'warning'],
          statuses: ['active', 'retired'],
          confidence_levels: ['high', 'medium'],
          tags: ['quality'],
          created_after: '2026-02-01T00:00:00.000Z',
          created_before: '2026-02-03T00:00:00.000Z',
          sort_by: 'created_at',
          sort_order: 'asc',
          limit: 1,
          offset: 0,
        })
      );

      expect(result.total).toBe(1);
      expect(result.memories).toHaveLength(1);
      expect(result.memories[0]?.id).toBe(first.id);
      expect(result.has_more).toBe(false);
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('supports search, getActiveMemories, and getMemoriesByType', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-store-'));
    const store = new MemoryStore({ type: 'git', path: storagePath, format: 'yaml' });

    try {
      const pattern = createMemory(
        { statement: 'Use deterministic plan checks in CI.', type: 'pattern', status: 'active' },
        5
      );
      const warning = createMemory(
        { statement: 'Do not bypass gate checks.', type: 'warning', status: 'active' },
        6
      );
      const superseded = createMemory(
        { statement: 'Legacy search baseline', type: 'pattern', status: 'superseded' },
        7
      );

      await store.importMemories([pattern, warning, superseded]);

      const search = await store.searchMemories('deterministic');
      expect(search.map((item) => item.id)).toEqual([pattern.id]);

      const active = await store.getActiveMemories();
      expect(active).toHaveLength(2);

      const patterns = await store.getMemoriesByType('pattern');
      expect(patterns).toHaveLength(1);
      expect(patterns[0]?.id).toBe(pattern.id);
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('returns counts and aggregate stats', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-store-'));
    const store = new MemoryStore({ type: 'git', path: storagePath, format: 'yaml' });

    try {
      await store.importMemories([
        createMemory({ type: 'pattern', status: 'active', confidence: 'high' }, 8),
        createMemory({ type: 'warning', status: 'retired', confidence: 'medium' }, 9),
        createMemory({ type: 'decision', status: 'superseded', confidence: 'low' }, 10),
      ]);

      expect(await store.countMemories()).toBe(3);
      expect(await store.countMemories({ status: 'active' })).toBe(1);
      expect(await store.countMemories({ type: 'warning' })).toBe(1);

      const stats = await store.getStats();
      expect(stats.total_memories).toBe(3);
      expect(stats.active_count).toBe(1);
      expect(stats.retired_count).toBe(1);
      expect(stats.superseded_count).toBe(1);
      expect(stats.by_type.find((item) => item.type === 'pattern')?.count).toBe(1);
      expect(stats.by_confidence.find((item) => item.level === 'high')?.count).toBe(1);
      expect(stats.unique_tags_count).toBeGreaterThan(0);
      expect(stats.oldest_memory).toBeDefined();
      expect(stats.most_recent).toBeDefined();
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('exports and imports memories preserving structural equality', async () => {
    const sourcePath = mkdtempSync(join(tmpdir(), 'edda-store-source-'));
    const targetPath = mkdtempSync(join(tmpdir(), 'edda-store-target-'));
    const sourceStore = new MemoryStore({ type: 'git', path: sourcePath, format: 'yaml' });
    const targetStore = new MemoryStore({ type: 'git', path: targetPath, format: 'yaml' });

    try {
      const memories = [createMemory({}, 11), createMemory({ type: 'lesson' }, 12)];
      await sourceStore.importMemories(memories);

      const exported = await sourceStore.exportMemories();
      const importedCount = await targetStore.importMemories(exported);

      expect(importedCount).toBe(2);
      expect(await targetStore.exportMemories()).toEqual(memories);
    } finally {
      rmSync(sourcePath, { recursive: true, force: true });
      rmSync(targetPath, { recursive: true, force: true });
    }
  });

  it('rejects invalid memory objects before writing them to disk', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-store-'));
    const store = new MemoryStore({ type: 'git', path: storagePath, format: 'yaml' });
    const invalidMemory = {
      ...createMemory({}, 14),
      id: '../outside',
    } as unknown as MemoryObject;

    try {
      await expect(store.importMemories([invalidMemory])).rejects.toThrow();
      expect(await store.exportMemories()).toEqual([]);
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });

  it('keeps index consistency when updating existing memory IDs', async () => {
    const storagePath = mkdtempSync(join(tmpdir(), 'edda-store-'));
    const store = new MemoryStore({ type: 'git', path: storagePath, format: 'yaml' });

    try {
      const original = createMemory(
        {
          statement: 'x'.repeat(140),
          type: 'pattern',
        },
        13
      );
      await store.saveMemory(original);

      const updated = {
        ...original,
        type: 'doctrine' as const,
        statement: 'Updated doctrine statement for index consistency checks.',
      };
      await store.saveMemory(updated);

      const indexPath = join(storagePath, 'index.yaml');
      expect(existsSync(indexPath)).toBe(true);

      const index = deserialiseIndex(readFileSync(indexPath, 'utf8'));
      expect(index.memories).toHaveLength(1);
      expect(index.memories[0]?.type).toBe('doctrine');
      expect(index.memories[0]?.path).toContain('memories/doctrine');
      expect(index.memories[0]?.statement?.length).toBeLessThanOrEqual(100);
      expect(index.memories[0]?.proposal_id).toBe(original.provenance.ember_source?.proposal_id);

      const oldPath = join(storagePath, 'memories', 'pattern', `${original.id}.yaml`);
      const newPath = join(storagePath, 'memories', 'doctrine', `${original.id}.yaml`);
      expect(existsSync(oldPath)).toBe(false);
      expect(existsSync(newPath)).toBe(true);
    } finally {
      rmSync(storagePath, { recursive: true, force: true });
    }
  });
});
