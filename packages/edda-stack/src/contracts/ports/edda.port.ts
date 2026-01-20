/**
 * Edda Port Interface (STACK-007, STACK-009)
 *
 * Defines the interface for Edda memory storage adapters.
 * Implementations can use SQLite, PostgreSQL, file-based, or in-memory storage.
 *
 * @module @eddacraft/anvil-edda-stack/contracts/ports/edda
 */

import type { MemoryId, ProposalId } from '../identifiers.js';
import type { Timestamp } from '../temporal.js';
import type {
  MemoryObject,
  PromoteProposalInput,
  MemoryQuery,
  MemoryQueryResult,
  MemoryStatus,
  MemoryType,
  MemoryContext,
} from '../edda-memory.js';
import type { EddaConfidenceLevel } from '../confidence.js';
import type { ProvenanceChain } from '../provenance.js';
import type { CandidateProposal } from '../ember-proposal.js';

// =============================================================================
// Input Types
// =============================================================================

/**
 * Input for creating a memory directly (without promotion from Ember)
 */
export interface CreateMemoryInput {
  /** Memory type */
  type: MemoryType;

  /** The remembered truth */
  statement: string;

  /** Memory context */
  context: MemoryContext;

  /** Confidence level */
  confidence: EddaConfidenceLevel;

  /** Confidence rationale */
  confidence_rationale?: string;

  /** Provenance chain */
  provenance: ProvenanceChain;

  /** Who is creating this memory */
  created_by: string;

  /** Why this memory is being created */
  reason: string;

  /** Additional metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Input for updating a memory
 */
export interface UpdateMemoryInput {
  /** Updated statement */
  statement?: string;

  /** Updated context */
  context?: Partial<MemoryContext>;

  /** Updated confidence */
  confidence?: EddaConfidenceLevel;

  /** Updated confidence rationale */
  confidence_rationale?: string;

  /** Updated metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Input for retiring a memory
 */
export interface RetireMemoryInput {
  /** Why this memory is being retired */
  reason: string;

  /** Who is retiring it */
  retired_by: string;

  /** Optional: Memory ID that supersedes this one */
  superseded_by?: MemoryId;
}

// =============================================================================
// Provenance Resolution Types (STACK-007)
// =============================================================================

/**
 * Result of resolving a provenance chain
 *
 * When tracing back through provenance, we may encounter broken links
 * (e.g., observations that have been pruned). This result captures
 * what was successfully resolved and what was missing.
 */
export interface ProvenanceResolutionResult {
  /** Whether the full chain was successfully resolved */
  complete: boolean;

  /** Number of links successfully resolved */
  resolved_count: number;

  /** Total links in the chain */
  total_count: number;

  /** IDs of any missing/unresolvable links */
  missing_links: string[];

  /** Resolved provenance data (partial if incomplete) */
  resolved_data?: {
    /** Sessions that were found */
    sessions: string[];
    /** Observations that were found */
    observations: string[];
    /** Proposal that was found (if any) */
    proposal_id?: string;
  };

  /** Warnings encountered during resolution */
  warnings: string[];
}

// =============================================================================
// Statistics Types (STACK-007)
// =============================================================================

/**
 * Statistics about memories by type
 */
export interface MemoryTypeStats {
  type: MemoryType;
  count: number;
}

/**
 * Statistics about memories by status
 */
export interface MemoryStatusStats {
  status: MemoryStatus;
  count: number;
}

/**
 * Statistics about memories by confidence level
 */
export interface ConfidenceLevelStats {
  level: EddaConfidenceLevel;
  count: number;
}

/**
 * Overall statistics for the Edda memory store
 */
export interface EddaStats {
  /** Total number of memories */
  total_memories: number;

  /** Memories by status */
  by_status: MemoryStatusStats[];

  /** Memories by type */
  by_type: MemoryTypeStats[];

  /** Memories by confidence level */
  by_confidence: ConfidenceLevelStats[];

  /** Number of active memories */
  active_count: number;

  /** Number of superseded memories */
  superseded_count: number;

  /** Number of retired memories */
  retired_count: number;

  /** Timestamp of oldest memory */
  oldest_memory?: Timestamp;

  /** Timestamp of most recent memory */
  most_recent?: Timestamp;

  /** Number of unique tags used */
  unique_tags_count: number;
}

// =============================================================================
// Edda Port Interface
// =============================================================================

/**
 * Port interface for Edda memory storage
 *
 * This is the primary abstraction for reading/writing memories.
 * Implementations should be stateless and thread-safe.
 */
export interface IEddaPort {
  // ─────────────────────────────────────────────────────────────────────────
  // Write Operations
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Promote an Ember proposal to an Edda memory
   */
  promoteProposal(input: PromoteProposalInput): Promise<MemoryObject>;

  /**
   * Create a memory directly (without Ember proposal)
   */
  createMemory(input: CreateMemoryInput): Promise<MemoryObject>;

  /**
   * Create a new memory from a promoted proposal (STACK-007)
   *
   * @param input - Promotion input with human-provided context
   * @param proposal - The Ember proposal being promoted
   * @returns The created memory object
   */
  createMemoryFromProposal(
    input: PromoteProposalInput,
    proposal: CandidateProposal
  ): Promise<MemoryObject>;

  /**
   * Update an existing memory
   */
  updateMemory(id: MemoryId, input: UpdateMemoryInput): Promise<MemoryObject | null>;

  /**
   * Retire a memory (mark as no longer applicable)
   */
  retireMemory(id: MemoryId, input: RetireMemoryInput): Promise<MemoryObject | null>;

  /**
   * Retire a memory with explicit parameters (STACK-007)
   *
   * Retired memories are preserved but marked as no longer applicable.
   * If superseded by another memory, the link is recorded.
   *
   * @param id - The memory ID to retire
   * @param supersededBy - ID of the memory that supersedes this (if any)
   * @param reason - Why the memory is being retired
   * @param retiredBy - Who is retiring the memory
   */
  retireMemoryById(
    id: MemoryId,
    supersededBy: MemoryId | undefined,
    reason: string,
    retiredBy: string
  ): Promise<void>;

  /**
   * Supersede a memory with a new one
   * Creates the new memory and retires the old one atomically
   */
  supersedeMemory(
    oldMemoryId: MemoryId,
    newMemoryInput: CreateMemoryInput
  ): Promise<{ old: MemoryObject; new: MemoryObject }>;

  // ─────────────────────────────────────────────────────────────────────────
  // Read Operations
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get a single memory by ID
   */
  getMemory(id: MemoryId): Promise<MemoryObject | null>;

  /**
   * Get a memory by the proposal ID it was promoted from
   */
  getMemoryByProposalId(proposalId: ProposalId): Promise<MemoryObject | null>;

  /**
   * Query memories with filters
   */
  queryMemories(query: MemoryQuery): Promise<MemoryQueryResult>;

  /**
   * Get all active memories
   */
  getActiveMemories(): Promise<MemoryObject[]>;

  /**
   * Get memories by type
   */
  getMemoriesByType(type: MemoryType): Promise<MemoryObject[]>;

  /**
   * Search memories by statement text
   */
  searchMemories(searchText: string): Promise<MemoryObject[]>;

  /**
   * Check if a memory exists
   */
  memoryExists(id: MemoryId): Promise<boolean>;

  // ─────────────────────────────────────────────────────────────────────────
  // Evolution Graph
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get the evolution chain for a memory
   *
   * Returns all memories in the supersedes/superseded_by chain,
   * ordered from oldest to newest.
   *
   * @param id - The memory ID to trace
   * @returns Array of memories in the evolution chain
   */
  getEvolutionChain(id: MemoryId): Promise<MemoryObject[]>;

  /**
   * Get the latest version of a memory
   * Follows superseded_by links to find the current active version
   */
  getLatestVersion(id: MemoryId): Promise<MemoryObject | null>;

  // ─────────────────────────────────────────────────────────────────────────
  // Provenance Resolution (STACK-007)
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Resolve a provenance chain to verify all links exist
   *
   * This validates that the observations, sessions, and proposals
   * referenced in a provenance chain still exist and are accessible.
   *
   * @param chain - The provenance chain to resolve
   * @returns Resolution result with any missing links
   */
  resolveProvenance(chain: ProvenanceChain): Promise<ProvenanceResolutionResult>;

  // ─────────────────────────────────────────────────────────────────────────
  // Maintenance & Status
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Check if the Edda store is available and operational (STACK-007)
   *
   * @returns True if the store is available
   */
  isAvailable(): Promise<boolean>;

  /**
   * Get statistics about the memory store (STACK-007)
   *
   * @returns Current statistics
   */
  getStats(): Promise<EddaStats>;

  /**
   * Get total memory count (optionally filtered by status or type)
   */
  countMemories(filter?: { status?: MemoryStatus; type?: MemoryType }): Promise<number>;

  /**
   * Export all memories (for backup)
   */
  exportMemories(): Promise<MemoryObject[]>;

  /**
   * Import memories (for restore)
   */
  importMemories(memories: MemoryObject[]): Promise<number>;
}
