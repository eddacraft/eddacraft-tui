import { describe, expect, it } from 'vitest';
import {
  EvolutionLinkSchema,
  EvolutionGraphSchema,
  createEvolutionLink,
  buildEvolutionGraph,
  findRootMemory,
  findLatestMemory,
  getEvolutionPath,
  validateEvolutionGraph,
} from './evolution.js';
import { createMemoryId } from './identifiers.js';

const ID_1 = '550e8400-e29b-41d4-a716-446655440000';
const ID_2 = '550e8400-e29b-41d4-a716-446655440001';
const ID_3 = '550e8400-e29b-41d4-a716-446655440002';
const ID_4 = '550e8400-e29b-41d4-a716-446655440003';
const VALID_TIMESTAMP = '2024-01-15T14:30:00.000Z';

function buildMemory(
  id: string,
  status: 'active' | 'superseded' | 'retired',
  supersedes: string[],
  supersededBy?: string
) {
  return {
    id,
    type: 'decision' as const,
    status,
    schema_version: 1,
    statement: `Memory ${id}`,
    context: {
      when: 'During implementation',
      why: 'Track evolution behaviour',
      conditions: ['Memory migration'],
      tags: ['evolution'],
    },
    metadata: {
      decision_point: 'Testing evolution graph',
    },
    confidence: 'high' as const,
    provenance: {
      kindling_sources: [
        {
          observation_id: ID_1,
          session_id: ID_1,
          kind: 'action_executed',
          timestamp: VALID_TIMESTAMP,
        },
      ],
      source_sessions: [ID_1],
    },
    attribution: {
      actor: 'maintainer@eddacraft.dev',
      timestamp: VALID_TIMESTAMP,
      method: 'manual_edit' as const,
      reason: 'Evolution test setup',
    },
    evolution: {
      supersedes,
      superseded_by: supersededBy,
      retired_at: status !== 'active' ? VALID_TIMESTAMP : undefined,
      retired_reason: status !== 'active' ? 'Replaced by newer memory' : undefined,
      retired_by: status !== 'active' ? 'maintainer@eddacraft.dev' : undefined,
    },
    created_at: VALID_TIMESTAMP,
    updated_at: VALID_TIMESTAMP,
  };
}

describe('Evolution links (EDDA-004)', () => {
  it('createEvolutionLink creates a valid link', () => {
    const link = createEvolutionLink(
      createMemoryId(ID_1),
      createMemoryId(ID_2),
      'Refined decision after production feedback',
      'maintainer@eddacraft.dev'
    );

    expect(EvolutionLinkSchema.safeParse(link).success).toBe(true);
    expect(link.old_memory_id).toBe(ID_1);
    expect(link.new_memory_id).toBe(ID_2);
    expect(link.reason).toBe('Refined decision after production feedback');
  });

  it('rejects self-referential links', () => {
    expect(() =>
      createEvolutionLink(
        createMemoryId(ID_1),
        createMemoryId(ID_1),
        'Invalid self-link',
        'maintainer@eddacraft.dev'
      )
    ).toThrow();
  });
});

describe('Evolution graph utilities (EDDA-004)', () => {
  it('builds an evolution graph from memories', () => {
    const graph = buildEvolutionGraph([
      buildMemory(ID_1, 'superseded', [], ID_2),
      buildMemory(ID_2, 'superseded', [ID_1], ID_3),
      buildMemory(ID_3, 'active', [ID_2]),
    ]);

    expect(EvolutionGraphSchema.safeParse(graph).success).toBe(true);
    expect(graph.nodes).toHaveLength(3);
    expect(graph.links).toHaveLength(2);
  });

  it('finds root and latest memory in a chain', () => {
    const graph = buildEvolutionGraph([
      buildMemory(ID_1, 'superseded', [], ID_2),
      buildMemory(ID_2, 'superseded', [ID_1], ID_3),
      buildMemory(ID_3, 'active', [ID_2]),
    ]);

    const root = findRootMemory(graph);
    const latest = findLatestMemory(graph);

    expect(root?.memory_id).toBe(ID_1);
    expect(latest?.memory_id).toBe(ID_3);
  });

  it('returns evolution path between two memories', () => {
    const graph = buildEvolutionGraph([
      buildMemory(ID_1, 'superseded', [], ID_2),
      buildMemory(ID_2, 'superseded', [ID_1], ID_3),
      buildMemory(ID_3, 'active', [ID_2]),
    ]);

    const path = getEvolutionPath(graph, createMemoryId(ID_1), createMemoryId(ID_3));
    expect(path).toEqual([ID_1, ID_2, ID_3]);
  });

  it('returns null when no path exists', () => {
    const graph = buildEvolutionGraph([
      buildMemory(ID_1, 'active', []),
      buildMemory(ID_2, 'active', []),
    ]);

    const path = getEvolutionPath(graph, createMemoryId(ID_1), createMemoryId(ID_2));
    expect(path).toBeNull();
  });
});

describe('Evolution graph validation (EDDA-004)', () => {
  it('validates a consistent evolution graph', () => {
    const graph = buildEvolutionGraph([
      buildMemory(ID_1, 'superseded', [], ID_2),
      buildMemory(ID_2, 'active', [ID_1]),
    ]);

    const result = validateEvolutionGraph(graph);
    expect(result.valid).toBe(true);
    expect(result.issues).toEqual([]);
  });

  it('detects cycles in graph links', () => {
    const graph = EvolutionGraphSchema.parse({
      nodes: [
        {
          memory_id: ID_1,
          status: 'active',
          position_in_chain: 0,
          supersedes: [],
        },
        {
          memory_id: ID_2,
          status: 'superseded',
          position_in_chain: 1,
          supersedes: [ID_1],
        },
      ],
      links: [
        {
          old_memory_id: ID_1,
          new_memory_id: ID_2,
          reason: 'v2',
          actor: 'maintainer@eddacraft.dev',
          linked_at: VALID_TIMESTAMP,
        },
        {
          old_memory_id: ID_2,
          new_memory_id: ID_1,
          reason: 'invalid back-edge',
          actor: 'maintainer@eddacraft.dev',
          linked_at: VALID_TIMESTAMP,
        },
      ],
    });

    const result = validateEvolutionGraph(graph);
    expect(result.valid).toBe(false);
    expect(result.issues).toContain('Evolution graph contains a cycle');
  });

  it('detects orphan nodes in multi-node graphs', () => {
    const graph = EvolutionGraphSchema.parse({
      nodes: [
        {
          memory_id: ID_1,
          status: 'active',
          position_in_chain: 0,
          supersedes: [],
        },
        {
          memory_id: ID_2,
          status: 'superseded',
          position_in_chain: 1,
          supersedes: [ID_1],
        },
        {
          memory_id: ID_4,
          status: 'retired',
          position_in_chain: 0,
          supersedes: [],
        },
      ],
      links: [
        {
          old_memory_id: ID_1,
          new_memory_id: ID_2,
          reason: 'Refinement',
          actor: 'maintainer@eddacraft.dev',
          linked_at: VALID_TIMESTAMP,
        },
      ],
    });

    const result = validateEvolutionGraph(graph);
    expect(result.valid).toBe(false);
    expect(result.issues).toContain(`Orphan memory node: ${ID_4}`);
  });
});
