import { existsSync, mkdirSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, posix } from 'node:path';
import type {
  EddaConfidenceLevel,
  MemoryId,
  MemoryObject,
  MemoryQuery,
  MemoryQueryResult,
  MemoryStatus,
  MemoryType,
  ProposalId,
} from '../contracts/index.js';
import { MemoryObjectSchema } from '../contracts/index.js';
import type {
  ConfidenceLevelStats,
  EddaStats,
  MemoryStatusStats,
  MemoryTypeStats,
} from '../contracts/ports/edda.port.js';
import type { EddaStorageConfig } from './config.js';
import {
  deserialiseIndex,
  deserialiseMemory,
  serialiseIndex,
  serialiseMemory,
  type MemoryIndex,
  type MemoryIndexEntry,
} from './serialisation.js';

const ALL_MEMORY_TYPES: MemoryType[] = [
  'decision',
  'pattern',
  'constraint',
  'warning',
  'doctrine',
  'lesson',
];

const ALL_MEMORY_STATUSES: MemoryStatus[] = ['active', 'superseded', 'retired'];
const ALL_CONFIDENCE_LEVELS: EddaConfidenceLevel[] = ['low', 'medium', 'high'];

/**
 * YAML-file-backed memory store.
 *
 * Concurrency note (CIB-118): index.yaml and memory files are written with
 * plain non-atomic writes and there is no cross-PROCESS locking. Service-level
 * protections (EvolutionService's per-memory transition lock, the promotion
 * CAS on the proposal store) assume a SINGLE process owns this storage path.
 * Concurrent writers from multiple processes can interleave index writes and
 * lose updates — supporting that deployment shape needs a store-level
 * redesign (atomic rename + file locking), which is out of scope here.
 */
export class MemoryStore {
  private readonly storagePath: string;
  private readonly memoriesPath: string;
  private readonly indexPath: string;

  constructor(config: EddaStorageConfig) {
    this.storagePath = config.path;
    this.memoriesPath = join(this.storagePath, 'memories');
    this.indexPath = join(this.storagePath, 'index.yaml');
  }

  async getMemory(id: MemoryId): Promise<MemoryObject | null> {
    const index = this.loadIndex();
    const entry = index.memories.find((item) => item.id === id);
    if (!entry) {
      return null;
    }

    return this.readMemoryFromEntry(entry);
  }

  async getMemoryByProposalId(proposalId: ProposalId): Promise<MemoryObject | null> {
    const index = this.loadIndex();
    const entry = index.memories.find((item) => item.proposal_id === proposalId);
    if (!entry) {
      return null;
    }

    return this.readMemoryFromEntry(entry);
  }

  async queryMemories(query: MemoryQuery): Promise<MemoryQueryResult> {
    const index = this.loadIndex();
    const includeSuperseded = query.include_superseded;

    const filteredEntries = index.memories.filter((entry) => {
      if (query.types && query.types.length > 0 && !query.types.includes(entry.type)) {
        return false;
      }

      if (query.statuses && query.statuses.length > 0 && !query.statuses.includes(entry.status)) {
        return false;
      }

      if (!query.statuses && !includeSuperseded && entry.status === 'superseded') {
        return false;
      }

      if (
        query.confidence_levels &&
        query.confidence_levels.length > 0 &&
        (!entry.confidence || !query.confidence_levels.includes(entry.confidence))
      ) {
        return false;
      }

      if (query.created_after) {
        if (!entry.created_at || entry.created_at <= query.created_after) {
          return false;
        }
      }

      if (query.created_before) {
        if (!entry.created_at || entry.created_at >= query.created_before) {
          return false;
        }
      }

      if (query.tags && query.tags.length > 0) {
        const tags = entry.tags ?? [];
        if (!query.tags.some((tag) => tags.includes(tag))) {
          return false;
        }
      }

      if (query.search && entry.statement) {
        const searchLower = query.search.toLowerCase();
        if (!entry.statement.toLowerCase().includes(searchLower)) {
          return false;
        }
      }

      return true;
    });

    const memories = filteredEntries
      .map((entry) => this.readMemoryFromEntry(entry))
      .filter((memory): memory is MemoryObject => memory !== null);

    const result = memories;

    const sortBy = query.sort_by ?? 'created_at';
    const sortOrder = query.sort_order ?? 'desc';
    const sortDirection = sortOrder === 'asc' ? 1 : -1;

    result.sort((a, b) => {
      if (sortBy === 'type') {
        return a.type.localeCompare(b.type) * sortDirection;
      }

      const aValue = sortBy === 'updated_at' ? (a.updated_at ?? a.created_at) : a.created_at;
      const bValue = sortBy === 'updated_at' ? (b.updated_at ?? b.created_at) : b.created_at;

      if (aValue === bValue) {
        return a.id.localeCompare(b.id) * sortDirection;
      }

      return (aValue < bValue ? -1 : 1) * sortDirection;
    });

    const offset = query.offset;
    const limit = query.limit;
    const paged = result.slice(offset, offset + limit);

    return {
      memories: paged,
      total: result.length,
      limit,
      offset,
      has_more: offset + paged.length < result.length,
    };
  }

  /** Returns active memories. Results capped at 1000; use queryMemories() for pagination. */
  async getActiveMemories(): Promise<MemoryObject[]> {
    const result = await this.queryMemories({
      types: undefined,
      statuses: ['active'],
      confidence_levels: undefined,
      created_after: undefined,
      created_before: undefined,
      tags: undefined,
      search: undefined,
      include_superseded: false,
      limit: 1000,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });
    return result.memories;
  }

  /** Returns memories for a type. Results capped at 1000; use queryMemories() for pagination. */
  async getMemoriesByType(type: MemoryType): Promise<MemoryObject[]> {
    const result = await this.queryMemories({
      types: [type],
      statuses: undefined,
      confidence_levels: undefined,
      created_after: undefined,
      created_before: undefined,
      tags: undefined,
      search: undefined,
      include_superseded: false,
      limit: 1000,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });
    return result.memories;
  }

  /** Returns search-matched memories. Results capped at 1000; use queryMemories() for pagination. */
  async searchMemories(searchText: string): Promise<MemoryObject[]> {
    const result = await this.queryMemories({
      types: undefined,
      statuses: undefined,
      confidence_levels: undefined,
      created_after: undefined,
      created_before: undefined,
      tags: undefined,
      search: searchText,
      include_superseded: false,
      limit: 1000,
      offset: 0,
      sort_by: 'created_at',
      sort_order: 'desc',
    });
    return result.memories;
  }

  async memoryExists(id: MemoryId): Promise<boolean> {
    const index = this.loadIndex();
    return index.memories.some((entry) => entry.id === id);
  }

  async countMemories(filter?: { status?: MemoryStatus; type?: MemoryType }): Promise<number> {
    const index = this.loadIndex();
    return index.memories.filter((entry) => {
      if (filter?.status && entry.status !== filter.status) {
        return false;
      }
      if (filter?.type && entry.type !== filter.type) {
        return false;
      }
      return true;
    }).length;
  }

  async getStats(): Promise<EddaStats> {
    const memories = await this.exportMemories();

    const byStatusMap = new Map<MemoryStatus, number>();
    const byTypeMap = new Map<MemoryType, number>();
    const byConfidenceMap = new Map<EddaConfidenceLevel, number>();
    const tags = new Set<string>();

    for (const memory of memories) {
      byStatusMap.set(memory.status, (byStatusMap.get(memory.status) ?? 0) + 1);
      byTypeMap.set(memory.type, (byTypeMap.get(memory.type) ?? 0) + 1);
      byConfidenceMap.set(memory.confidence, (byConfidenceMap.get(memory.confidence) ?? 0) + 1);

      for (const tag of memory.context.tags) {
        tags.add(tag);
      }
    }

    const sortedByCreatedAt = [...memories].sort((a, b) =>
      a.created_at.localeCompare(b.created_at)
    );

    const byStatus: MemoryStatusStats[] = ALL_MEMORY_STATUSES.map((status) => ({
      status,
      count: byStatusMap.get(status) ?? 0,
    }));

    const byType: MemoryTypeStats[] = ALL_MEMORY_TYPES.map((type) => ({
      type,
      count: byTypeMap.get(type) ?? 0,
    }));

    const byConfidence: ConfidenceLevelStats[] = ALL_CONFIDENCE_LEVELS.map((level) => ({
      level,
      count: byConfidenceMap.get(level) ?? 0,
    }));

    return {
      total_memories: memories.length,
      by_status: byStatus,
      by_type: byType,
      by_confidence: byConfidence,
      active_count: byStatusMap.get('active') ?? 0,
      superseded_count: byStatusMap.get('superseded') ?? 0,
      retired_count: byStatusMap.get('retired') ?? 0,
      oldest_memory: sortedByCreatedAt[0]?.created_at,
      most_recent: sortedByCreatedAt[sortedByCreatedAt.length - 1]?.created_at,
      unique_tags_count: tags.size,
    };
  }

  async isAvailable(): Promise<boolean> {
    return existsSync(this.storagePath);
  }

  async exportMemories(): Promise<MemoryObject[]> {
    const index = this.loadIndex();

    return index.memories
      .map((entry) => this.readMemoryFromEntry(entry))
      .filter((memory): memory is MemoryObject => memory !== null);
  }

  async importMemories(memories: MemoryObject[]): Promise<number> {
    this.ensureStorageInitialised();

    for (const memory of memories) {
      await this.saveMemory(memory);
    }

    return memories.length;
  }

  async saveMemory(memory: MemoryObject): Promise<void> {
    this.ensureStorageInitialised();

    const validatedMemory = MemoryObjectSchema.parse(memory);
    const index = this.loadIndex();
    const existing = index.memories.find((entry) => entry.id === validatedMemory.id);

    if (
      existing &&
      existing.path !== this.getRelativeMemoryPath(validatedMemory.id, validatedMemory.type)
    ) {
      const oldPath = join(this.storagePath, existing.path);
      if (existsSync(oldPath)) {
        unlinkSync(oldPath);
      }
    }

    const relativePath = this.getRelativeMemoryPath(validatedMemory.id, validatedMemory.type);
    const fullPath = join(this.storagePath, relativePath);
    mkdirSync(dirname(fullPath), { recursive: true });
    writeFileSync(fullPath, serialiseMemory(validatedMemory), 'utf8');

    const nextEntry = this.toIndexEntry(validatedMemory, relativePath);
    const withoutCurrent = index.memories.filter((entry) => entry.id !== validatedMemory.id);

    this.writeIndex({
      memories: [...withoutCurrent, nextEntry],
      updated_at: new Date().toISOString(),
    });
  }

  async deleteMemory(id: MemoryId): Promise<boolean> {
    this.ensureStorageInitialised();

    const index = this.loadIndex();
    const existing = index.memories.find((entry) => entry.id === id);
    if (!existing) {
      return false;
    }

    const targetPath = join(this.storagePath, existing.path);
    if (existsSync(targetPath)) {
      rmSync(targetPath);
    }

    this.writeIndex({
      memories: index.memories.filter((entry) => entry.id !== id),
      updated_at: new Date().toISOString(),
    });

    return true;
  }

  private ensureStorageInitialised(): void {
    mkdirSync(this.storagePath, { recursive: true });
    mkdirSync(this.memoriesPath, { recursive: true });

    for (const type of ALL_MEMORY_TYPES) {
      mkdirSync(join(this.memoriesPath, type), { recursive: true });
    }

    if (!existsSync(this.indexPath)) {
      this.writeIndex({
        memories: [],
        updated_at: new Date().toISOString(),
      });
    }
  }

  private loadIndex(): MemoryIndex {
    if (!existsSync(this.indexPath)) {
      return {
        memories: [],
        updated_at: new Date().toISOString(),
      };
    }

    const raw = readFileSync(this.indexPath, 'utf8');
    return deserialiseIndex(raw);
  }

  private writeIndex(index: MemoryIndex): void {
    writeFileSync(this.indexPath, serialiseIndex(index), 'utf8');
  }

  private readMemoryFromEntry(entry: MemoryIndexEntry): MemoryObject | null {
    const fullPath = join(this.storagePath, entry.path);
    if (!existsSync(fullPath)) {
      return null;
    }

    const raw = readFileSync(fullPath, 'utf8');
    return deserialiseMemory(raw);
  }

  private getRelativeMemoryPath(id: MemoryId, type: MemoryType): string {
    return posix.join('memories', type, `${id}.yaml`);
  }

  private toIndexEntry(memory: MemoryObject, path: string): MemoryIndexEntry {
    const proposalId = memory.provenance.ember_source?.proposal_id;

    return {
      id: memory.id,
      type: memory.type,
      status: memory.status,
      path,
      statement: memory.statement.slice(0, 100),
      confidence: memory.confidence,
      tags: memory.context.tags,
      created_at: memory.created_at,
      proposal_id: proposalId,
    };
  }
}
