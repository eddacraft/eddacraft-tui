/**
 * Edda Port Mock (STACK-010)
 *
 * Mock implementation of IEddaPort for testing.
 * Uses in-memory storage and vitest mock functions.
 *
 * @module @eddacraft/anvil-edda-stack/testing/mocks/edda
 */

import { vi, type Mock } from 'vitest';
import { v4 as uuidv4 } from 'uuid';
import type {
  IEddaPort,
  CreateMemoryInput,
  UpdateMemoryInput,
  RetireMemoryInput,
  ProvenanceResolutionResult,
  MemoryTypeStats,
  MemoryStatusStats,
  ConfidenceLevelStats,
  EddaStats,
} from '../../contracts/ports/edda.port.js';
import type { EddaConfidenceLevel } from '../../contracts/confidence.js';
import type {
  MemoryObject,
  PromoteProposalInput,
  MemoryQuery,
  MemoryQueryResult,
  MemoryStatus,
  MemoryType,
} from '../../contracts/edda-memory.js';
import type { CandidateProposal } from '../../contracts/ember-proposal.js';
import type { ProvenanceChain } from '../../contracts/provenance.js';
import type { MemoryId, ProposalId, Timestamp } from '../../contracts/index.js';
import { now } from '../../contracts/temporal.js';
import { createMemoryId, createProposalId, createSessionId } from '../../contracts/identifiers.js';
import { MEMORY_SCHEMA_VERSION } from '../../contracts/edda-memory.js';

// =============================================================================
// Mock Options
// =============================================================================

/**
 * Options for creating a mock Edda port
 */
export interface MockEddaPortOptions {
  /** Initial memories to populate the store */
  initialMemories?: MemoryObject[];

  /** Whether to auto-generate IDs (default: true) */
  autoGenerateIds?: boolean;
}

// =============================================================================
// Mock Implementation
// =============================================================================

/**
 * In-memory implementation of IEddaPort for testing
 */
export interface MockEddaPort extends IEddaPort {
  /** Access to the underlying memory store */
  _store: Map<MemoryId, MemoryObject>;

  /** Index of proposal ID to memory ID */
  _proposalIndex: Map<string, MemoryId>;

  /** Reset the mock to initial state */
  _reset: () => void;

  /** Get all memories (for assertions) */
  _getAll: () => MemoryObject[];

  /** Mock function references for verification */
  _mocks: {
    promoteProposal: Mock;
    createMemory: Mock;
    updateMemory: Mock;
    retireMemory: Mock;
    supersedeMemory: Mock;
    getMemory: Mock;
    getMemoryByProposalId: Mock;
    queryMemories: Mock;
    getActiveMemories: Mock;
    getMemoriesByType: Mock;
    searchMemories: Mock;
    memoryExists: Mock;
    getEvolutionChain: Mock;
    getLatestVersion: Mock;
    countMemories: Mock;
    exportMemories: Mock;
    importMemories: Mock;
  };
}

/**
 * Create a mock Edda port for testing
 */
export function createMockEddaPort(options: MockEddaPortOptions = {}): MockEddaPort {
  const { initialMemories = [], autoGenerateIds = true } = options;

  // In-memory store
  const store = new Map<MemoryId, MemoryObject>();
  const proposalIndex = new Map<string, MemoryId>();

  // Populate initial memories
  for (const memory of initialMemories) {
    store.set(memory.id, memory);
    if (memory.provenance.ember_source?.proposal_id) {
      proposalIndex.set(memory.provenance.ember_source.proposal_id, memory.id);
    }
  }

  // Promote proposal implementation
  const promoteProposalImpl = async (input: PromoteProposalInput): Promise<MemoryObject> => {
    const id = autoGenerateIds ? createMemoryId(uuidv4()) : ('' as MemoryId);
    const timestamp = now();

    const memory: MemoryObject = {
      id,
      type: input.type,
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: input.statement ?? '',
      context: input.context,
      confidence: input.confidence,
      confidence_rationale: input.confidence_rationale,
      provenance: {
        ember_source: {
          proposal_id: input.proposal_id,
          proposal_type: input.type, // Simplified - in reality would get from proposal
          confidence: 0.75, // Simplified - in reality would get from proposal
          created_at: timestamp,
        },
        kindling_sources: [
          {
            observation_id: uuidv4() as any,
            session_id: uuidv4() as any,
            kind: 'gate_evaluated',
            timestamp,
          },
        ],
        source_sessions: [uuidv4() as any],
      },
      attribution: {
        actor: input.promoted_by,
        timestamp,
        method: 'cli_command',
        reason: input.reason,
      },
      evolution: { supersedes: [] },
      created_at: timestamp,
      metadata: input.metadata,
    };

    store.set(id, memory);
    proposalIndex.set(input.proposal_id, id);
    return memory;
  };

  // Create memory implementation
  const createMemoryImpl = async (input: CreateMemoryInput): Promise<MemoryObject> => {
    const id = autoGenerateIds ? createMemoryId(uuidv4()) : ('' as MemoryId);
    const timestamp = now();

    const memory: MemoryObject = {
      id,
      type: input.type,
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: input.statement,
      context: input.context,
      confidence: input.confidence,
      confidence_rationale: input.confidence_rationale,
      provenance: input.provenance,
      attribution: {
        actor: input.created_by,
        timestamp,
        method: 'cli_command',
        reason: input.reason,
      },
      evolution: { supersedes: [] },
      created_at: timestamp,
      metadata: input.metadata,
    };

    store.set(id, memory);
    return memory;
  };

  // Update memory implementation
  const updateMemoryImpl = async (
    id: MemoryId,
    input: UpdateMemoryInput
  ): Promise<MemoryObject | null> => {
    const memory = store.get(id);
    if (!memory) return null;

    const updated: MemoryObject = {
      ...memory,
      ...(input.statement !== undefined && { statement: input.statement }),
      ...(input.context !== undefined && { context: { ...memory.context, ...input.context } }),
      ...(input.confidence !== undefined && { confidence: input.confidence }),
      ...(input.confidence_rationale !== undefined && {
        confidence_rationale: input.confidence_rationale,
      }),
      ...(input.metadata !== undefined && { metadata: input.metadata }),
      updated_at: now(),
    };

    store.set(id, updated);
    return updated;
  };

  // Retire memory implementation
  const retireMemoryImpl = async (
    id: MemoryId,
    input: RetireMemoryInput
  ): Promise<MemoryObject | null> => {
    const memory = store.get(id);
    if (!memory) return null;

    const retired: MemoryObject = {
      ...memory,
      status: 'retired',
      evolution: {
        ...memory.evolution,
        retired_at: now(),
        retired_reason: input.reason,
        retired_by: input.retired_by,
        superseded_by: input.superseded_by,
      },
      updated_at: now(),
    };

    store.set(id, retired);
    return retired;
  };

  // Supersede memory implementation
  const supersedeMemoryImpl = async (
    oldMemoryId: MemoryId,
    newMemoryInput: CreateMemoryInput
  ): Promise<{ old: MemoryObject; new: MemoryObject }> => {
    const newMemory = await createMemoryImpl({
      ...newMemoryInput,
    });

    // Update new memory to reference old one
    const updatedNew: MemoryObject = {
      ...newMemory,
      evolution: {
        ...newMemory.evolution,
        supersedes: [oldMemoryId],
      },
    };
    store.set(newMemory.id, updatedNew);

    // Retire old memory
    const oldMemory = await retireMemoryImpl(oldMemoryId, {
      reason: 'Superseded by new memory',
      retired_by: newMemoryInput.created_by,
      superseded_by: newMemory.id,
    });

    return { old: oldMemory!, new: updatedNew };
  };

  // Get memory implementation
  const getMemoryImpl = async (id: MemoryId): Promise<MemoryObject | null> => {
    return store.get(id) ?? null;
  };

  // Get memory by proposal ID implementation
  const getMemoryByProposalIdImpl = async (
    proposalId: ProposalId
  ): Promise<MemoryObject | null> => {
    const memoryId = proposalIndex.get(proposalId);
    if (!memoryId) return null;
    return store.get(memoryId) ?? null;
  };

  // Query memories implementation
  const queryMemoriesImpl = async (query: MemoryQuery): Promise<MemoryQueryResult> => {
    let memories = Array.from(store.values());

    // Apply filters
    if (query.types && query.types.length > 0) {
      memories = memories.filter((m) => query.types!.includes(m.type));
    }

    if (query.statuses && query.statuses.length > 0) {
      memories = memories.filter((m) => query.statuses!.includes(m.status));
    }

    if (query.confidence_levels && query.confidence_levels.length > 0) {
      memories = memories.filter((m) => query.confidence_levels!.includes(m.confidence));
    }

    if (query.created_after) {
      const after = new Date(query.created_after).getTime();
      memories = memories.filter((m) => new Date(m.created_at).getTime() > after);
    }

    if (query.created_before) {
      const before = new Date(query.created_before).getTime();
      memories = memories.filter((m) => new Date(m.created_at).getTime() < before);
    }

    if (query.tags && query.tags.length > 0) {
      memories = memories.filter(
        (m) => m.context.tags && m.context.tags.some((t) => query.tags!.includes(t))
      );
    }

    if (query.search) {
      const searchLower = query.search.toLowerCase();
      memories = memories.filter((m) => m.statement.toLowerCase().includes(searchLower));
    }

    if (!query.include_superseded) {
      memories = memories.filter((m) => m.status !== 'superseded');
    }

    // Sort
    const sortBy = query.sort_by ?? 'created_at';
    const sortOrder = query.sort_order ?? 'desc';

    memories.sort((a, b) => {
      let cmp = 0;
      if (sortBy === 'created_at') {
        cmp = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
      } else if (sortBy === 'updated_at') {
        const aTime = a.updated_at ? new Date(a.updated_at).getTime() : 0;
        const bTime = b.updated_at ? new Date(b.updated_at).getTime() : 0;
        cmp = aTime - bTime;
      } else if (sortBy === 'type') {
        cmp = a.type.localeCompare(b.type);
      }
      return sortOrder === 'desc' ? -cmp : cmp;
    });

    const total = memories.length;
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 100;

    memories = memories.slice(offset, offset + limit);

    return {
      memories,
      total,
      limit,
      offset,
      has_more: offset + memories.length < total,
    };
  };

  // Get active memories implementation
  const getActiveMemoriesImpl = async (): Promise<MemoryObject[]> => {
    return Array.from(store.values()).filter((m) => m.status === 'active');
  };

  // Get memories by type implementation
  const getMemoriesByTypeImpl = async (type: MemoryType): Promise<MemoryObject[]> => {
    return Array.from(store.values()).filter((m) => m.type === type && m.status === 'active');
  };

  // Search memories implementation
  const searchMemoriesImpl = async (searchText: string): Promise<MemoryObject[]> => {
    const searchLower = searchText.toLowerCase();
    return Array.from(store.values()).filter(
      (m) => m.status === 'active' && m.statement.toLowerCase().includes(searchLower)
    );
  };

  // Memory exists implementation
  const memoryExistsImpl = async (id: MemoryId): Promise<boolean> => {
    return store.has(id);
  };

  // Get evolution chain implementation
  const getEvolutionChainImpl = async (id: MemoryId): Promise<MemoryObject[]> => {
    const chain: MemoryObject[] = [];
    const memory = store.get(id);
    if (!memory) return chain;

    chain.push(memory);

    // Follow supersedes links recursively
    const supersedes = memory.evolution.supersedes ?? [];
    for (const oldId of supersedes) {
      const oldChain = await getEvolutionChainImpl(oldId);
      chain.push(...oldChain);
    }

    return chain;
  };

  // Get latest version implementation
  const getLatestVersionImpl = async (id: MemoryId): Promise<MemoryObject | null> => {
    let current = store.get(id);
    if (!current) return null;

    while (current.evolution.superseded_by) {
      const next = store.get(current.evolution.superseded_by);
      if (!next) break;
      current = next;
    }

    return current;
  };

  // Count memories implementation
  const countMemoriesImpl = async (filter?: {
    status?: MemoryStatus;
    type?: MemoryType;
  }): Promise<number> => {
    let memories = Array.from(store.values());

    if (filter?.status) {
      memories = memories.filter((m) => m.status === filter.status);
    }

    if (filter?.type) {
      memories = memories.filter((m) => m.type === filter.type);
    }

    return memories.length;
  };

  // Export memories implementation
  const exportMemoriesImpl = async (): Promise<MemoryObject[]> => {
    return Array.from(store.values());
  };

  // Import memories implementation
  const importMemoriesImpl = async (memories: MemoryObject[]): Promise<number> => {
    let count = 0;
    for (const memory of memories) {
      if (!store.has(memory.id)) {
        store.set(memory.id, memory);
        if (memory.provenance.ember_source?.proposal_id) {
          proposalIndex.set(memory.provenance.ember_source.proposal_id, memory.id);
        }
        count++;
      }
    }
    return count;
  };

  // Create memory from proposal (STACK-007)
  const createMemoryFromProposalImpl = async (
    input: PromoteProposalInput,
    _proposal: CandidateProposal
  ): Promise<MemoryObject> => {
    return promoteProposalImpl(input);
  };

  // Retire memory by ID (STACK-007)
  const retireMemoryByIdImpl = async (
    id: MemoryId,
    supersededBy: MemoryId | undefined,
    reason: string,
    retiredBy: string
  ): Promise<void> => {
    await retireMemoryImpl(id, { reason, retired_by: retiredBy, superseded_by: supersededBy });
  };

  // Resolve provenance (STACK-007)
  const resolveProvenanceImpl = async (
    chain: ProvenanceChain
  ): Promise<ProvenanceResolutionResult> => {
    const totalCount =
      chain.kindling_sources.length + chain.source_sessions.length + (chain.ember_source ? 1 : 0);
    return {
      complete: true,
      resolved_count: totalCount,
      total_count: totalCount,
      missing_links: [],
      resolved_data: {
        sessions: chain.source_sessions as string[],
        observations: chain.kindling_sources.map((s) => s.observation_id as string),
        proposal_id: chain.ember_source?.proposal_id as string | undefined,
      },
      warnings: [],
    };
  };

  // Is available (STACK-007)
  const isAvailableImpl = async (): Promise<boolean> => {
    return true;
  };

  // Get stats (STACK-007)
  const getStatsImpl = async (): Promise<EddaStats> => {
    const memories = Array.from(store.values());

    // Calculate type stats
    const typeCounts = new Map<string, number>();
    for (const m of memories) {
      typeCounts.set(m.type, (typeCounts.get(m.type) ?? 0) + 1);
    }
    const byType: MemoryTypeStats[] = Array.from(typeCounts.entries()).map(([type, count]) => ({
      type: type as MemoryType,
      count,
    }));

    // Calculate status stats
    const statusCounts = new Map<string, number>();
    for (const m of memories) {
      statusCounts.set(m.status, (statusCounts.get(m.status) ?? 0) + 1);
    }
    const byStatus: MemoryStatusStats[] = Array.from(statusCounts.entries()).map(
      ([status, count]) => ({
        status: status as MemoryStatus,
        count,
      })
    );

    // Calculate confidence stats
    const confidenceCounts = new Map<string, number>();
    for (const m of memories) {
      confidenceCounts.set(m.confidence, (confidenceCounts.get(m.confidence) ?? 0) + 1);
    }
    const byConfidence: ConfidenceLevelStats[] = Array.from(confidenceCounts.entries()).map(
      ([level, count]) => ({
        level: level as EddaConfidenceLevel,
        count,
      })
    );

    // Calculate unique tags
    const uniqueTags = new Set<string>();
    for (const m of memories) {
      if (m.context.tags) {
        for (const tag of m.context.tags) {
          uniqueTags.add(tag);
        }
      }
    }

    // Find oldest and most recent
    let oldest: Timestamp | undefined;
    let mostRecent: Timestamp | undefined;
    for (const m of memories) {
      if (!oldest || m.created_at < oldest) {
        oldest = m.created_at;
      }
      if (!mostRecent || m.created_at > mostRecent) {
        mostRecent = m.created_at;
      }
    }

    return {
      total_memories: memories.length,
      by_type: byType,
      by_status: byStatus,
      by_confidence: byConfidence,
      active_count: memories.filter((m) => m.status === 'active').length,
      superseded_count: memories.filter((m) => m.status === 'superseded').length,
      retired_count: memories.filter((m) => m.status === 'retired').length,
      oldest_memory: oldest,
      most_recent: mostRecent,
      unique_tags_count: uniqueTags.size,
    };
  };

  // Create mock functions
  const mocks = {
    promoteProposal: vi.fn(promoteProposalImpl),
    createMemory: vi.fn(createMemoryImpl),
    createMemoryFromProposal: vi.fn(createMemoryFromProposalImpl),
    updateMemory: vi.fn(updateMemoryImpl),
    retireMemory: vi.fn(retireMemoryImpl),
    retireMemoryById: vi.fn(retireMemoryByIdImpl),
    supersedeMemory: vi.fn(supersedeMemoryImpl),
    getMemory: vi.fn(getMemoryImpl),
    getMemoryByProposalId: vi.fn(getMemoryByProposalIdImpl),
    queryMemories: vi.fn(queryMemoriesImpl),
    getActiveMemories: vi.fn(getActiveMemoriesImpl),
    getMemoriesByType: vi.fn(getMemoriesByTypeImpl),
    searchMemories: vi.fn(searchMemoriesImpl),
    memoryExists: vi.fn(memoryExistsImpl),
    getEvolutionChain: vi.fn(getEvolutionChainImpl),
    getLatestVersion: vi.fn(getLatestVersionImpl),
    resolveProvenance: vi.fn(resolveProvenanceImpl),
    isAvailable: vi.fn(isAvailableImpl),
    getStats: vi.fn(getStatsImpl),
    countMemories: vi.fn(countMemoriesImpl),
    exportMemories: vi.fn(exportMemoriesImpl),
    importMemories: vi.fn(importMemoriesImpl),
  };

  return {
    // IEddaPort implementation
    promoteProposal: mocks.promoteProposal,
    createMemory: mocks.createMemory,
    createMemoryFromProposal: mocks.createMemoryFromProposal,
    updateMemory: mocks.updateMemory,
    retireMemory: mocks.retireMemory,
    retireMemoryById: mocks.retireMemoryById,
    supersedeMemory: mocks.supersedeMemory,
    getMemory: mocks.getMemory,
    getMemoryByProposalId: mocks.getMemoryByProposalId,
    queryMemories: mocks.queryMemories,
    getActiveMemories: mocks.getActiveMemories,
    getMemoriesByType: mocks.getMemoriesByType,
    searchMemories: mocks.searchMemories,
    memoryExists: mocks.memoryExists,
    getEvolutionChain: mocks.getEvolutionChain,
    getLatestVersion: mocks.getLatestVersion,
    resolveProvenance: mocks.resolveProvenance,
    isAvailable: mocks.isAvailable,
    getStats: mocks.getStats,
    countMemories: mocks.countMemories,
    exportMemories: mocks.exportMemories,
    importMemories: mocks.importMemories,

    // Test utilities
    _store: store,
    _proposalIndex: proposalIndex,
    _reset: () => {
      store.clear();
      proposalIndex.clear();
      for (const memory of initialMemories) {
        store.set(memory.id, memory);
        if (memory.provenance.ember_source?.proposal_id) {
          proposalIndex.set(memory.provenance.ember_source.proposal_id, memory.id);
        }
      }
      Object.values(mocks).forEach((mock) => mock.mockClear());
    },
    _getAll: () => Array.from(store.values()),
    _mocks: mocks,
  };
}

// =============================================================================
// Pre-built Scenarios
// =============================================================================

/**
 * Create a mock Edda port with sample memories
 */
export function mockEddaWithMemories(): MockEddaPort {
  const sessionId = createSessionId(uuidv4());
  const proposalId = createProposalId(uuidv4());
  const baseTimestamp = new Date('2024-01-15T10:00:00.000Z');

  const memories: MemoryObject[] = [
    {
      id: createMemoryId(uuidv4()),
      type: 'decision',
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: 'We use TypeScript strict mode for all new projects',
      context: {
        when: '2024-01-01',
        why: 'Type safety improves maintainability and reduces bugs',
        conditions: ['new projects', 'greenfield development'],
        scope: 'All TypeScript projects',
        tags: ['typescript', 'tooling', 'quality'],
      },
      confidence: 'high',
      confidence_rationale: 'Explicitly decided by team lead',
      provenance: {
        ember_source: {
          proposal_id: proposalId,
          proposal_type: 'decision',
          confidence: 0.85,
          created_at: baseTimestamp.toISOString() as Timestamp,
        },
        kindling_sources: [
          {
            observation_id: uuidv4() as any,
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp: baseTimestamp.toISOString() as Timestamp,
          },
        ],
        source_sessions: [sessionId],
      },
      attribution: {
        actor: 'user@example.com',
        timestamp: baseTimestamp.toISOString() as Timestamp,
        method: 'cli_command',
        reason: 'Codifying team decision',
      },
      evolution: { supersedes: [] },
      created_at: baseTimestamp.toISOString() as Timestamp,
    },
    {
      id: createMemoryId(uuidv4()),
      type: 'pattern',
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: 'Use factory functions for creating complex objects',
      context: {
        when: '2024-01-10',
        why: 'Factories provide better encapsulation and testability',
        conditions: ['complex object creation', 'dependency injection needed'],
        tags: ['pattern', 'factory', 'design'],
      },
      confidence: 'medium',
      provenance: {
        kindling_sources: [
          {
            observation_id: uuidv4() as any,
            session_id: sessionId,
            kind: 'action_executed',
            timestamp: new Date(baseTimestamp.getTime() + 86400000).toISOString() as Timestamp,
          },
        ],
        source_sessions: [sessionId],
      },
      attribution: {
        actor: 'developer@example.com',
        timestamp: new Date(baseTimestamp.getTime() + 172800000).toISOString() as Timestamp,
        method: 'cli_command',
        reason: 'Documenting recurring pattern',
      },
      evolution: { supersedes: [] },
      created_at: new Date(baseTimestamp.getTime() + 172800000).toISOString() as Timestamp,
    },
    {
      id: createMemoryId(uuidv4()),
      type: 'warning',
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: 'Avoid using any type in TypeScript code',
      context: {
        when: '2024-01-12',
        why: 'Using any defeats the purpose of TypeScript',
        conditions: ['TypeScript files'],
        tags: ['typescript', 'warning', 'quality'],
      },
      confidence: 'high',
      confidence_rationale: 'Widely accepted best practice',
      provenance: {
        kindling_sources: [
          {
            observation_id: uuidv4() as any,
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp: new Date(baseTimestamp.getTime() + 259200000).toISOString() as Timestamp,
          },
        ],
        source_sessions: [sessionId],
      },
      attribution: {
        actor: 'user@example.com',
        timestamp: new Date(baseTimestamp.getTime() + 259200000).toISOString() as Timestamp,
        method: 'manual_edit',
        reason: 'Adding team guideline',
      },
      evolution: { supersedes: [] },
      created_at: new Date(baseTimestamp.getTime() + 259200000).toISOString() as Timestamp,
    },
  ];

  return createMockEddaPort({
    initialMemories: memories,
  });
}

/**
 * Create an empty mock Edda port
 */
export function mockEddaEmpty(): MockEddaPort {
  return createMockEddaPort();
}

/**
 * Create a mock Edda port with an evolution chain
 */
export function mockEddaWithEvolutionChain(): MockEddaPort {
  const sessionId = createSessionId(uuidv4());
  const baseTimestamp = new Date('2024-01-01T10:00:00.000Z');

  const oldMemoryId = createMemoryId(uuidv4());
  const newMemoryId = createMemoryId(uuidv4());

  const memories: MemoryObject[] = [
    // Old (superseded) memory
    {
      id: oldMemoryId,
      type: 'decision',
      status: 'superseded',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: 'Use npm for package management',
      context: {
        when: '2024-01-01',
        why: 'Team was familiar with npm',
        conditions: [],
        tags: ['tooling', 'npm'],
      },
      confidence: 'medium',
      provenance: {
        kindling_sources: [
          {
            observation_id: uuidv4() as any,
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp: baseTimestamp.toISOString() as Timestamp,
          },
        ],
        source_sessions: [sessionId],
      },
      attribution: {
        actor: 'user@example.com',
        timestamp: baseTimestamp.toISOString() as Timestamp,
        method: 'cli_command',
        reason: 'Initial decision',
      },
      evolution: {
        supersedes: [],
        superseded_by: newMemoryId,
        retired_at: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
        retired_reason: 'Migrated to pnpm for better performance',
        retired_by: 'user@example.com',
      },
      created_at: baseTimestamp.toISOString() as Timestamp,
    },
    // New (current) memory
    {
      id: newMemoryId,
      type: 'decision',
      status: 'active',
      schema_version: MEMORY_SCHEMA_VERSION,
      statement: 'Use pnpm for package management',
      context: {
        when: '2024-02-01',
        why: 'pnpm offers better disk space usage and faster installs',
        conditions: [],
        tags: ['tooling', 'pnpm'],
      },
      confidence: 'high',
      confidence_rationale: 'Proven performance benefits',
      provenance: {
        kindling_sources: [
          {
            observation_id: uuidv4() as any,
            session_id: sessionId,
            kind: 'gate_evaluated',
            timestamp: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
          },
        ],
        source_sessions: [sessionId],
      },
      attribution: {
        actor: 'user@example.com',
        timestamp: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
        method: 'cli_command',
        reason: 'Updating tooling decision',
      },
      evolution: {
        supersedes: [oldMemoryId],
      },
      created_at: new Date(baseTimestamp.getTime() + 2592000000).toISOString() as Timestamp,
    },
  ];

  return createMockEddaPort({
    initialMemories: memories,
  });
}
