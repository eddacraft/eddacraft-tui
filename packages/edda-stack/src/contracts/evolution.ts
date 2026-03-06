import { z } from 'zod';
import { EvolutionSchema, MemoryObjectSchema, MemoryStatusSchema } from './edda-memory.js';
import { MemoryIdSchema, type MemoryId } from './identifiers.js';
import { TimestampSchema, type Timestamp } from './temporal.js';

export const EvolutionLinkSchema = z
  .object({
    old_memory_id: MemoryIdSchema,
    new_memory_id: MemoryIdSchema,
    reason: z.string().min(1),
    actor: z.string().min(1),
    linked_at: TimestampSchema,
  })
  .refine((value) => value.old_memory_id !== value.new_memory_id, {
    message: 'Evolution link must connect two different memories',
    path: ['new_memory_id'],
  });

export type EvolutionLink = z.infer<typeof EvolutionLinkSchema>;

export const EvolutionNodeSchema = z.object({
  memory_id: MemoryIdSchema,
  status: MemoryStatusSchema,
  position_in_chain: z.number().int().nonnegative(),
  supersedes: z.array(MemoryIdSchema).default([]),
  superseded_by: MemoryIdSchema.optional(),
});

export type EvolutionNode = z.infer<typeof EvolutionNodeSchema>;

export const EvolutionGraphSchema = z.object({
  nodes: z.array(EvolutionNodeSchema),
  links: z.array(EvolutionLinkSchema),
});

export type EvolutionGraph = z.infer<typeof EvolutionGraphSchema>;

export function createEvolutionLink(
  oldId: MemoryId,
  newId: MemoryId,
  reason: string,
  actor: string
): EvolutionLink {
  return EvolutionLinkSchema.parse({
    old_memory_id: oldId,
    new_memory_id: newId,
    reason,
    actor,
    linked_at: new Date().toISOString() as Timestamp,
  });
}

export function buildEvolutionGraph(
  memories: z.input<typeof MemoryObjectSchema>[]
): EvolutionGraph {
  const parsedMemories = z.array(MemoryObjectSchema).parse(memories);

  const nodes = parsedMemories.map((memory) =>
    EvolutionNodeSchema.parse({
      memory_id: memory.id,
      status: memory.status,
      position_in_chain: 0,
      supersedes: memory.evolution.supersedes,
      superseded_by: memory.evolution.superseded_by,
    })
  );

  const links: EvolutionLink[] = [];
  const linkKeys = new Set<string>();

  for (const memory of parsedMemories) {
    const actor = memory.evolution.retired_by ?? memory.attribution.actor;
    const reason = memory.evolution.retired_reason ?? 'Superseded by newer memory';
    const linkedAt = memory.evolution.retired_at ?? memory.updated_at ?? memory.created_at;

    for (const supersededId of memory.evolution.supersedes) {
      const key = `${supersededId}:${memory.id}`;
      if (linkKeys.has(key)) {
        continue;
      }

      links.push(
        EvolutionLinkSchema.parse({
          old_memory_id: supersededId,
          new_memory_id: memory.id,
          reason,
          actor,
          linked_at: linkedAt,
        })
      );
      linkKeys.add(key);
    }

    if (memory.evolution.superseded_by !== undefined) {
      const key = `${memory.id}:${memory.evolution.superseded_by}`;
      if (!linkKeys.has(key)) {
        links.push(
          EvolutionLinkSchema.parse({
            old_memory_id: memory.id,
            new_memory_id: memory.evolution.superseded_by,
            reason,
            actor,
            linked_at: linkedAt,
          })
        );
        linkKeys.add(key);
      }
    }
  }

  const positionedNodes = assignNodePositions(nodes, links);

  return EvolutionGraphSchema.parse({
    nodes: positionedNodes,
    links,
  });
}

export function findRootMemory(graph: EvolutionGraph): EvolutionNode | null {
  const parsedGraph = EvolutionGraphSchema.parse(graph);
  const incoming = new Map<MemoryId, number>();

  for (const node of parsedGraph.nodes) {
    incoming.set(node.memory_id, 0);
  }

  for (const link of parsedGraph.links) {
    incoming.set(link.new_memory_id, (incoming.get(link.new_memory_id) ?? 0) + 1);
  }

  const roots = parsedGraph.nodes
    .filter((node) => (incoming.get(node.memory_id) ?? 0) === 0)
    .sort((a, b) => a.position_in_chain - b.position_in_chain);

  return roots[0] ?? null;
}

export function findLatestMemory(graph: EvolutionGraph): EvolutionNode | null {
  const parsedGraph = EvolutionGraphSchema.parse(graph);
  const outgoing = new Map<MemoryId, number>();

  for (const node of parsedGraph.nodes) {
    outgoing.set(node.memory_id, 0);
  }

  for (const link of parsedGraph.links) {
    outgoing.set(link.old_memory_id, (outgoing.get(link.old_memory_id) ?? 0) + 1);
  }

  const latest = parsedGraph.nodes
    .filter((node) => (outgoing.get(node.memory_id) ?? 0) === 0)
    .sort((a, b) => b.position_in_chain - a.position_in_chain);

  return latest[0] ?? null;
}

export function getEvolutionPath(
  graph: EvolutionGraph,
  fromId: MemoryId,
  toId: MemoryId
): MemoryId[] | null {
  const parsedGraph = EvolutionGraphSchema.parse(graph);
  const nodeIds = new Set(parsedGraph.nodes.map((node) => node.memory_id));

  if (!nodeIds.has(fromId) || !nodeIds.has(toId)) {
    return null;
  }

  if (fromId === toId) {
    return [fromId];
  }

  const adjacency = new Map<MemoryId, MemoryId[]>();
  for (const link of parsedGraph.links) {
    const current = adjacency.get(link.old_memory_id) ?? [];
    current.push(link.new_memory_id);
    adjacency.set(link.old_memory_id, current);
  }

  const queue: Array<{ id: MemoryId; path: MemoryId[] }> = [{ id: fromId, path: [fromId] }];
  const visited = new Set<MemoryId>([fromId]);

  while (queue.length > 0) {
    const next = queue.shift();
    if (next === undefined) {
      break;
    }

    const neighbours = adjacency.get(next.id) ?? [];
    for (const neighbour of neighbours) {
      if (visited.has(neighbour)) {
        continue;
      }

      const path = [...next.path, neighbour];
      if (neighbour === toId) {
        return path;
      }

      visited.add(neighbour);
      queue.push({ id: neighbour, path });
    }
  }

  return null;
}

export function validateEvolutionGraph(graph: EvolutionGraph): {
  valid: boolean;
  issues: string[];
} {
  const parsedGraph = EvolutionGraphSchema.parse(graph);
  const issues: string[] = [];

  const nodeIds = new Set(parsedGraph.nodes.map((node) => node.memory_id));
  const incoming = new Map<MemoryId, number>();
  const outgoing = new Map<MemoryId, number>();

  for (const node of parsedGraph.nodes) {
    incoming.set(node.memory_id, 0);
    outgoing.set(node.memory_id, 0);
    EvolutionSchema.parse({
      supersedes: node.supersedes,
      superseded_by: node.superseded_by,
    });
  }

  for (const link of parsedGraph.links) {
    if (!nodeIds.has(link.old_memory_id)) {
      issues.push(`Link references missing old memory: ${link.old_memory_id}`);
    }
    if (!nodeIds.has(link.new_memory_id)) {
      issues.push(`Link references missing new memory: ${link.new_memory_id}`);
    }
    if (link.old_memory_id === link.new_memory_id) {
      issues.push(`Self-referential evolution link: ${link.old_memory_id}`);
    }

    if (nodeIds.has(link.old_memory_id)) {
      outgoing.set(link.old_memory_id, (outgoing.get(link.old_memory_id) ?? 0) + 1);
    }
    if (nodeIds.has(link.new_memory_id)) {
      incoming.set(link.new_memory_id, (incoming.get(link.new_memory_id) ?? 0) + 1);
    }
  }

  const visiting = new Set<MemoryId>();
  const visited = new Set<MemoryId>();

  const adjacency = new Map<MemoryId, MemoryId[]>();
  for (const link of parsedGraph.links) {
    const current = adjacency.get(link.old_memory_id) ?? [];
    current.push(link.new_memory_id);
    adjacency.set(link.old_memory_id, current);
  }

  const hasCycle = (nodeId: MemoryId): boolean => {
    if (visiting.has(nodeId)) {
      return true;
    }
    if (visited.has(nodeId)) {
      return false;
    }

    visiting.add(nodeId);
    for (const next of adjacency.get(nodeId) ?? []) {
      if (hasCycle(next)) {
        return true;
      }
    }
    visiting.delete(nodeId);
    visited.add(nodeId);

    return false;
  };

  for (const node of parsedGraph.nodes) {
    if (hasCycle(node.memory_id)) {
      issues.push('Evolution graph contains a cycle');
      break;
    }
  }

  if (parsedGraph.nodes.length > 1) {
    const orphanNodes = parsedGraph.nodes.filter(
      (node) =>
        (incoming.get(node.memory_id) ?? 0) === 0 && (outgoing.get(node.memory_id) ?? 0) === 0
    );
    for (const orphan of orphanNodes) {
      issues.push(`Orphan memory node: ${orphan.memory_id}`);
    }
  }

  return {
    valid: issues.length === 0,
    issues,
  };
}

function assignNodePositions(nodes: EvolutionNode[], links: EvolutionLink[]): EvolutionNode[] {
  const incomingCount = new Map<MemoryId, number>();
  const adjacency = new Map<MemoryId, MemoryId[]>();

  for (const node of nodes) {
    incomingCount.set(node.memory_id, 0);
    adjacency.set(node.memory_id, []);
  }

  for (const link of links) {
    incomingCount.set(link.new_memory_id, (incomingCount.get(link.new_memory_id) ?? 0) + 1);
    const current = adjacency.get(link.old_memory_id) ?? [];
    current.push(link.new_memory_id);
    adjacency.set(link.old_memory_id, current);
  }

  const depths = new Map<MemoryId, number>();
  const queue: MemoryId[] = [];

  for (const node of nodes) {
    if ((incomingCount.get(node.memory_id) ?? 0) === 0) {
      queue.push(node.memory_id);
      depths.set(node.memory_id, 0);
    }
  }

  while (queue.length > 0) {
    const nodeId = queue.shift();
    if (nodeId === undefined) {
      break;
    }

    const currentDepth = depths.get(nodeId) ?? 0;
    for (const next of adjacency.get(nodeId) ?? []) {
      const existingDepth = depths.get(next);
      const nextDepth = currentDepth + 1;

      if (existingDepth === undefined || nextDepth > existingDepth) {
        depths.set(next, nextDepth);
      }

      incomingCount.set(next, (incomingCount.get(next) ?? 1) - 1);
      if ((incomingCount.get(next) ?? 0) === 0) {
        queue.push(next);
      }
    }
  }

  return nodes.map((node) => ({
    ...node,
    position_in_chain: depths.get(node.memory_id) ?? 0,
  }));
}
