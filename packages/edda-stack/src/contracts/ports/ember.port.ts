/**
 * Ember Port Interface (STACK-007, STACK-009)
 *
 * Defines the interface for Ember proposal storage adapters.
 * Implementations can use SQLite, PostgreSQL, or in-memory storage.
 *
 * @module @eddacraft/anvil-edda-stack/contracts/ports/ember
 */

import type { ProposalId, MemoryId, SessionId } from '../identifiers.js';
import type { Timestamp } from '../temporal.js';
import type {
  CandidateProposal,
  CreateProposalInput,
  ProposalQuery,
  ProposalQueryResult,
  ProposalStatus,
  ProposalType,
} from '../ember-proposal.js';

// =============================================================================
// Input Types
// =============================================================================

/**
 * Input for updating a proposal
 */
export interface UpdateProposalInput {
  /** Updated summary */
  summary?: string;

  /** Updated rationale */
  rationale?: string;

  /** Updated confidence score */
  confidence?: number;

  /** Updated metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Input for resolving a proposal (promotion/dismissal)
 */
export interface ResolveProposalInput {
  /** New status (promoted, dismissed, or expired) */
  status: Exclude<ProposalStatus, 'active'>;

  /** Who resolved it */
  resolved_by?: string;

  /** Why it was resolved this way */
  resolution_reason?: string;

  /** Memory ID if promoted */
  memory_id?: MemoryId;
}

// =============================================================================
// Statistics Types (STACK-007)
// =============================================================================

/**
 * Statistics about proposals by type
 */
export interface ProposalTypeStats {
  type: ProposalType;
  count: number;
  avg_confidence: number;
}

/**
 * Statistics about proposals by status
 */
export interface ProposalStatusStats {
  status: ProposalStatus;
  count: number;
}

/**
 * Overall statistics for the Ember proposal store
 */
export interface EmberStats {
  /** Total number of proposals */
  total_proposals: number;

  /** Proposals by status */
  by_status: ProposalStatusStats[];

  /** Proposals by type */
  by_type: ProposalTypeStats[];

  /** Number of proposals expiring within 24 hours */
  expiring_soon: number;

  /** Average confidence across all active proposals */
  avg_confidence?: number;

  /** Timestamp of oldest active proposal */
  oldest_active?: Timestamp;

  /** Timestamp of most recent proposal */
  most_recent?: Timestamp;

  /** Promotion rate (promoted / (promoted + expired + dismissed)) */
  promotion_rate?: number;
}

// =============================================================================
// Ember Port Interface
// =============================================================================

/**
 * Port interface for Ember proposal storage
 *
 * This is the primary abstraction for reading/writing proposals.
 * Implementations should be stateless and thread-safe.
 */
export interface IEmberPort {
  // ─────────────────────────────────────────────────────────────────────────
  // Write Operations
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Create a new proposal
   */
  createProposal(input: CreateProposalInput): Promise<CandidateProposal>;

  /**
   * Update an existing proposal
   */
  updateProposal(id: ProposalId, input: UpdateProposalInput): Promise<CandidateProposal | null>;

  /**
   * Resolve a proposal (promote, dismiss, or mark expired)
   *
   * Only active proposals can be resolved. Returns null when the proposal
   * does not exist; throws ProposalAlreadyResolvedError when the proposal is
   * already in a terminal state (terminal states are immutable, CIB-118).
   */
  resolveProposal(id: ProposalId, input: ResolveProposalInput): Promise<CandidateProposal | null>;

  // ─────────────────────────────────────────────────────────────────────────
  // Read Operations
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get a single proposal by ID
   */
  getProposal(id: ProposalId): Promise<CandidateProposal | null>;

  /**
   * Query proposals with filters
   */
  queryProposals(query: ProposalQuery): Promise<ProposalQueryResult>;

  /**
   * Get all active proposals (not expired, promoted, or dismissed)
   */
  getActiveProposals(): Promise<CandidateProposal[]>;

  /**
   * Get proposals by session ID
   */
  getProposalsBySession(sessionId: SessionId): Promise<CandidateProposal[]>;

  /**
   * Check if a proposal exists
   */
  proposalExists(id: ProposalId): Promise<boolean>;

  // ─────────────────────────────────────────────────────────────────────────
  // Resolution Operations (STACK-007)
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Mark a proposal as promoted to Edda
   *
   * This updates the proposal status to 'promoted' and records
   * the memory ID it was promoted to.
   *
   * The transition is atomic and idempotent: replaying the same promotion
   * (same memory ID) is a no-op, while promoting with a different memory ID
   * or from another terminal state throws ProposalAlreadyResolvedError
   * (CIB-118).
   *
   * @param id - The proposal ID to mark as promoted
   * @param memoryId - The Edda memory ID this was promoted to
   * @param resolvedBy - Who performed the promotion
   */
  markPromoted(id: ProposalId, memoryId: MemoryId, resolvedBy: string): Promise<void>;

  /**
   * Mark a proposal as dismissed
   *
   * This updates the proposal status to 'dismissed' and records
   * the reason for dismissal.
   *
   * Replaying a dismissal is a no-op (the first resolution record wins);
   * dismissing from another terminal state throws
   * ProposalAlreadyResolvedError (CIB-118).
   *
   * @param id - The proposal ID to dismiss
   * @param reason - Why the proposal was dismissed
   * @param resolvedBy - Who dismissed the proposal
   */
  markDismissed(id: ProposalId, reason: string, resolvedBy: string): Promise<void>;

  // ─────────────────────────────────────────────────────────────────────────
  // TTL & Expiry
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Get expired proposals that haven't been marked as expired yet
   */
  getExpiredProposals(): Promise<CandidateProposal[]>;

  /**
   * Mark expired proposals as expired
   * Returns the number of proposals marked
   */
  processExpiredProposals(): Promise<number>;

  /**
   * Expire stale proposals that have passed their TTL (STACK-007)
   *
   * This should be called periodically (e.g., by a background job)
   * to clean up proposals that were neither promoted nor dismissed.
   *
   * @returns Number of proposals that were expired
   */
  expireStaleProposals(): Promise<number>;

  // ─────────────────────────────────────────────────────────────────────────
  // Maintenance & Status
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Check if the Ember store is available and operational (STACK-007)
   *
   * @returns True if the store is available
   */
  isAvailable(): Promise<boolean>;

  /**
   * Get statistics about the proposal store (STACK-007)
   *
   * @returns Current statistics
   */
  getStats(): Promise<EmberStats>;

  /**
   * Get total proposal count (optionally filtered by status)
   */
  countProposals(status?: ProposalStatus): Promise<number>;

  /**
   * Delete resolved proposals older than a given timestamp
   * Returns the number of deleted proposals
   */
  pruneProposals(olderThan: Timestamp): Promise<number>;
}
