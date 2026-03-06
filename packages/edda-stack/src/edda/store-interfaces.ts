import type {
  MemoryObject,
  MemoryQuery,
  MemoryQueryResult,
  MemoryStatus,
  MemoryType,
} from '../contracts/edda-memory.js';
import type { MemoryId, ProposalId } from '../contracts/identifiers.js';
import type { Timestamp } from '../contracts/temporal.js';
import type { EddaStats } from '../contracts/ports/edda.port.js';

/** Minimal store interface that services depend on */
export interface IMemoryStoreOperations {
  getMemory(id: MemoryId): Promise<MemoryObject | null>;
  saveMemory(memory: MemoryObject): Promise<void>;
  getMemoryByProposalId(proposalId: ProposalId): Promise<MemoryObject | null>;
  queryMemories(query: MemoryQuery): Promise<MemoryQueryResult>;
  getActiveMemories(): Promise<MemoryObject[]>;
  getMemoriesByType(type: MemoryType): Promise<MemoryObject[]>;
  searchMemories(searchText: string): Promise<MemoryObject[]>;
  memoryExists(id: MemoryId): Promise<boolean>;
  countMemories(filter?: { status?: MemoryStatus; type?: MemoryType }): Promise<number>;
  getStats(): Promise<EddaStats>;
  isAvailable(): Promise<boolean>;
  exportMemories(): Promise<MemoryObject[]>;
  importMemories(memories: MemoryObject[]): Promise<number>;
}

export interface VersionEntry {
  hash: string;
  message: string;
  author: string;
  timestamp: Timestamp;
}

export interface IVersionTracker {
  init(): Promise<void>;
  trackChange(filePaths: string[], message: string, author: string): Promise<string>;
  getHistory(filePath: string, limit?: number): Promise<VersionEntry[]>;
  isInitialised(): Promise<boolean>;
}
