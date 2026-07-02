/**
 * Candidate Memory Proposal Schema (EMBER-001)
 *
 * Defines the data model for Ember candidate memory proposals.
 * Proposals are ephemeral suggestions that decay unless promoted to Edda.
 *
 * Key characteristics:
 * - Ephemeral by design (TTL-based expiry)
 * - Heuristic confidence (algorithmic, not authoritative)
 * - Allowed to be wrong (probabilistic thinking)
 * - Links back to Kindling observations
 *
 * @module @eddacraft/anvil-edda-stack/contracts/ember-proposal
 */

import { z } from 'zod';
import { ProposalIdSchema } from './identifiers.js';
import { TimestampSchema, DurationDaysSchema } from './temporal.js';
import { EmberConfidenceSchema } from './confidence.js';
import { ProvenanceSummarySchema } from './provenance.js';

// =============================================================================
// Proposal Types
// =============================================================================

/**
 * The 6 types of candidate memory proposals
 *
 * Each type represents a different kind of pattern or insight
 * that Ember has detected from Kindling observations.
 */
export const ProposalTypeSchema = z.enum([
  'decision', // A choice made with consequences
  'pattern', // A recurring structure or behaviour
  'warning', // A signal of potential problems
  'lesson', // A learning from failure or success
  'anomaly', // An unexpected deviation
  'constraint', // A discovered limitation or boundary
]);

export type ProposalType = z.infer<typeof ProposalTypeSchema>;

/**
 * Human-readable descriptions of proposal types
 */
export const proposalTypeDescriptions: Record<ProposalType, string> = {
  decision: 'A choice made with consequences',
  pattern: 'A recurring structure or behaviour',
  warning: 'A signal of potential problems',
  lesson: 'A learning from failure or success',
  anomaly: 'An unexpected deviation from expected behaviour',
  constraint: 'A discovered limitation or boundary',
};

// =============================================================================
// Proposal Status
// =============================================================================

/**
 * Lifecycle status of a proposal
 */
export const ProposalStatusSchema = z.enum([
  'active', // Proposal is active and can be reviewed
  'promoted', // Proposal was promoted to Edda
  'expired', // Proposal expired without action
  'dismissed', // Proposal was explicitly dismissed
]);

export type ProposalStatus = z.infer<typeof ProposalStatusSchema>;

/**
 * Thrown when a state transition is attempted on a proposal that has already
 * reached a terminal status (promoted, expired, or dismissed). Terminal
 * proposal states are immutable: the first resolution wins and later
 * transitions are refused rather than overwriting the recorded resolution
 * (CIB-118).
 */
export class ProposalAlreadyResolvedError extends Error {
  constructor(
    public readonly proposalId: string,
    public readonly status: ProposalStatus
  ) {
    super(
      `Proposal ${proposalId} is already resolved (status: '${status}'); terminal proposal states are immutable`
    );
    this.name = 'ProposalAlreadyResolvedError';
  }
}

// =============================================================================
// Evaluation Signals
// =============================================================================

/**
 * Signals that contributed to this proposal's confidence score
 *
 * These record which heuristics fired and their individual contributions.
 */
export const EvaluationSignalSchema = z.object({
  /** Name of the evaluation rule that fired */
  rule: z.string().describe('Evaluation rule name (e.g., "repetition", "escalation")'),

  /** Contribution to overall confidence (0.0-1.0) */
  contribution: EmberConfidenceSchema.describe("This signal's contribution to confidence"),

  /** Weight of this rule in overall evaluation */
  weight: z.number().positive().describe('Rule weight in evaluation'),

  /** Additional context from the rule */
  context: z.record(z.string(), z.unknown()).optional().describe('Rule-specific context'),
});

export type EvaluationSignal = z.infer<typeof EvaluationSignalSchema>;

// =============================================================================
// Core Proposal Schema
// =============================================================================

/**
 * Candidate Memory Proposal
 *
 * The core data model for Ember. A proposal represents a candidate
 * piece of memory that might be worth promoting to Edda.
 */
export const CandidateProposalSchema = z.object({
  // ─────────────────────────────────────────────────────────────────────────
  // Identity
  // ─────────────────────────────────────────────────────────────────────────

  /** Unique proposal identifier */
  id: ProposalIdSchema,

  /** Type of proposal */
  type: ProposalTypeSchema,

  /** Current status */
  status: ProposalStatusSchema.default('active'),

  // ─────────────────────────────────────────────────────────────────────────
  // Content
  // ─────────────────────────────────────────────────────────────────────────

  /** Brief summary of the candidate (what was observed) */
  summary: z.string().min(1).max(500).describe('Brief summary of the candidate observation'),

  /** Detailed rationale (why this might be worth remembering) */
  rationale: z.string().min(1).max(2000).describe('Why this might be worth remembering'),

  /** Type-specific metadata */
  metadata: z.record(z.string(), z.unknown()).optional().describe('Type-specific additional data'),

  // ─────────────────────────────────────────────────────────────────────────
  // Confidence & Evaluation
  // ─────────────────────────────────────────────────────────────────────────

  /** Overall confidence score (heuristic) */
  confidence: EmberConfidenceSchema.describe('Overall confidence score'),

  /** Individual signals that contributed to confidence */
  signals: z.array(EvaluationSignalSchema).default([]).describe('Evaluation signals'),

  // ─────────────────────────────────────────────────────────────────────────
  // Provenance
  // ─────────────────────────────────────────────────────────────────────────

  /** Links back to source observations */
  provenance: ProvenanceSummarySchema.describe('Source observation links'),

  // ─────────────────────────────────────────────────────────────────────────
  // Temporal
  // ─────────────────────────────────────────────────────────────────────────

  /** When this proposal was created */
  created_at: TimestampSchema,

  /** When this proposal expires (TTL) */
  expires_at: TimestampSchema,

  /** Original TTL in days */
  ttl_days: DurationDaysSchema,

  /** When this proposal was last evaluated/updated */
  updated_at: TimestampSchema.optional(),

  // ─────────────────────────────────────────────────────────────────────────
  // Resolution (if promoted/dismissed)
  // ─────────────────────────────────────────────────────────────────────────

  /** Resolution information (when status changes from active) */
  resolution: z
    .object({
      resolved_at: TimestampSchema,
      resolved_by: z.string().optional().describe('User who resolved'),
      resolution_reason: z.string().optional().describe('Why resolved this way'),
      memory_id: z.string().uuid().optional().describe('Edda memory ID if promoted'),
    })
    .optional(),
});

export type CandidateProposal = z.infer<typeof CandidateProposalSchema>;

// =============================================================================
// Type-Specific Metadata Schemas
// =============================================================================

/**
 * Metadata for "decision" proposals
 */
export const DecisionMetadataSchema = z.object({
  decision_point: z.string().describe('What decision was made'),
  alternatives_considered: z.array(z.string()).optional(),
  outcome_observed: z.string().optional(),
});

/**
 * Metadata for "pattern" proposals
 */
export const PatternMetadataSchema = z.object({
  pattern_name: z.string().optional(),
  occurrence_count: z.number().int().positive(),
  first_seen: TimestampSchema,
  last_seen: TimestampSchema,
});

/**
 * Metadata for "warning" proposals
 */
export const WarningMetadataSchema = z.object({
  warning_type: z.string(),
  severity: z.enum(['low', 'medium', 'high']),
  affected_areas: z.array(z.string()).optional(),
});

/**
 * Metadata for "lesson" proposals
 */
export const LessonMetadataSchema = z.object({
  lesson_type: z.enum(['success', 'failure', 'mixed']),
  context: z.string(),
  applicable_to: z.array(z.string()).optional(),
});

/**
 * Metadata for "anomaly" proposals
 */
export const AnomalyMetadataSchema = z.object({
  expected_behaviour: z.string(),
  actual_behaviour: z.string(),
  deviation_magnitude: z.number().optional(),
});

/**
 * Metadata for "constraint" proposals
 */
export const ConstraintMetadataSchema = z.object({
  constraint_type: z.string(),
  scope: z.string(),
  discovered_via: z.string().optional(),
});

// =============================================================================
// Proposal Creation Utilities
// =============================================================================

/**
 * Input for creating a new proposal
 */
export const CreateProposalInputSchema = z.object({
  type: ProposalTypeSchema,
  summary: z.string().min(1).max(500),
  rationale: z.string().min(1).max(2000),
  confidence: EmberConfidenceSchema,
  provenance: ProvenanceSummarySchema,
  ttl_days: DurationDaysSchema.optional().default(30),
  metadata: z.record(z.string(), z.unknown()).optional(),
  signals: z.array(EvaluationSignalSchema).optional(),
});

export type CreateProposalInput = z.infer<typeof CreateProposalInputSchema>;

// =============================================================================
// Query Schemas
// =============================================================================

/**
 * Filters for querying proposals
 */
export const ProposalQuerySchema = z.object({
  /** Filter by type(s) */
  types: z.array(ProposalTypeSchema).optional(),

  /** Filter by status(es) */
  statuses: z.array(ProposalStatusSchema).optional(),

  /** Minimum confidence threshold */
  min_confidence: EmberConfidenceSchema.optional(),

  /** Created after this timestamp */
  created_after: TimestampSchema.optional(),

  /** Created before this timestamp */
  created_before: TimestampSchema.optional(),

  /** Include expired proposals */
  include_expired: z.boolean().optional().default(false),

  /** Session ID filter */
  session_id: z.string().uuid().optional(),

  /** Limit results */
  limit: z.number().int().positive().optional().default(100),

  /** Offset for pagination */
  offset: z.number().int().nonnegative().optional().default(0),

  /** Sort by field */
  sort_by: z.enum(['created_at', 'confidence', 'expires_at']).optional().default('created_at'),

  /** Sort direction */
  sort_order: z.enum(['asc', 'desc']).optional().default('desc'),
});

export type ProposalQuery = z.infer<typeof ProposalQuerySchema>;

/**
 * Query result wrapper
 */
export const ProposalQueryResultSchema = z.object({
  proposals: z.array(CandidateProposalSchema),
  total: z.number().int().nonnegative(),
  limit: z.number().int().positive(),
  offset: z.number().int().nonnegative(),
  has_more: z.boolean(),
});

export type ProposalQueryResult = z.infer<typeof ProposalQueryResultSchema>;
