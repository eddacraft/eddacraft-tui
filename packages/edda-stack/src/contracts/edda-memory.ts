/**
 * Memory Object Schema (EDDA-001)
 *
 * Defines the data model for Edda canonical memory objects.
 * Memory objects are durable, versioned, human-curated knowledge.
 *
 * Key characteristics:
 * - Permanent by design (no automatic expiry)
 * - Human-asserted confidence (judgemental, not computed)
 * - Versioned and auditable
 * - Requires human decision to create
 * - Links back to Ember proposals and Kindling observations
 *
 * @module @anvil/edda-stack/contracts/edda-memory
 */

import { z } from 'zod';
import { MemoryIdSchema, ProposalIdSchema } from './identifiers.js';
import { TimestampSchema } from './temporal.js';
import { EddaConfidenceLevelSchema } from './confidence.js';
import { ProvenanceChainSchema, AttributionSchema } from './provenance.js';

// =============================================================================
// Memory Types
// =============================================================================

/**
 * The 6 types of canonical memory objects
 *
 * These map to (but are not identical to) Ember proposal types.
 * The mapping is intentional to enable promotion workflow.
 */
export const MemoryTypeSchema = z.enum([
  'decision', // A choice made with context and consequences
  'pattern', // A recurring structure worth codifying
  'constraint', // A boundary or limitation to respect
  'warning', // A persistent caution or known risk
  'doctrine', // An organisational principle or belief
  'lesson', // A learning from experience
]);

export type MemoryType = z.infer<typeof MemoryTypeSchema>;

/**
 * Human-readable descriptions of memory types
 */
export const memoryTypeDescriptions: Record<MemoryType, string> = {
  decision: 'A choice made with context and consequences',
  pattern: 'A recurring structure worth codifying',
  constraint: 'A boundary or limitation to respect',
  warning: 'A persistent caution or known risk',
  doctrine: 'An organisational principle or belief',
  lesson: 'A learning from experience',
};

// =============================================================================
// Memory Status
// =============================================================================

/**
 * Lifecycle status of a memory object
 */
export const MemoryStatusSchema = z.enum([
  'active', // Memory is current and applicable
  'superseded', // Memory has been replaced by newer memory
  'retired', // Memory is no longer applicable (but preserved)
]);

export type MemoryStatus = z.infer<typeof MemoryStatusSchema>;

// =============================================================================
// Memory Context
// =============================================================================

/**
 * Context describing when/why/under what conditions this memory applies
 */
export const MemoryContextSchema = z.object({
  /** When this became true or was decided */
  when: z.string().describe('When this became true (timestamp or description)'),

  /** Why this is worth remembering */
  why: z.string().describe('Rationale for preserving this memory'),

  /** Conditions under which this applies */
  conditions: z.array(z.string()).default([]).describe('Applicability conditions'),

  /** Scope limitations */
  scope: z.string().optional().describe('Where this applies (e.g., "monorepo only")'),

  /** Related concepts or domains */
  tags: z.array(z.string()).default([]).describe('Tags for categorisation'),
});

export type MemoryContext = z.infer<typeof MemoryContextSchema>;

// =============================================================================
// Evolution Graph
// =============================================================================

/**
 * Evolution tracking — supersedes/superseded_by relationships
 */
export const EvolutionSchema = z.object({
  /** Memory IDs this memory supersedes (replaces) */
  supersedes: z.array(MemoryIdSchema).default([]),

  /** Memory ID that superseded this memory (if retired) */
  superseded_by: MemoryIdSchema.optional(),

  /** When this memory was retired */
  retired_at: TimestampSchema.optional(),

  /** Why this memory was retired */
  retired_reason: z.string().optional(),

  /** Who retired this memory */
  retired_by: z.string().optional(),
});

export type Evolution = z.infer<typeof EvolutionSchema>;

// =============================================================================
// Core Memory Object Schema
// =============================================================================

/**
 * Schema version for migration support
 */
export const MEMORY_SCHEMA_VERSION = 1;

/**
 * Memory Object
 *
 * The core data model for Edda. A memory object represents a piece
 * of institutional knowledge that has been curated and preserved.
 */
export const MemoryObjectSchema = z.object({
  // ─────────────────────────────────────────────────────────────────────────
  // Identity & Versioning
  // ─────────────────────────────────────────────────────────────────────────

  /** Unique memory identifier (stable, referable) */
  id: MemoryIdSchema,

  /** Type of memory */
  type: MemoryTypeSchema,

  /** Current status */
  status: MemoryStatusSchema.default('active'),

  /** Schema version for migration support */
  schema_version: z.number().int().positive().default(MEMORY_SCHEMA_VERSION),

  // ─────────────────────────────────────────────────────────────────────────
  // Content
  // ─────────────────────────────────────────────────────────────────────────

  /** The remembered truth (the core statement) */
  statement: z.string().min(1).max(2000).describe('The remembered truth'),

  /** Context describing when/why/conditions */
  context: MemoryContextSchema,

  /** Type-specific metadata */
  metadata: z.record(z.unknown()).optional(),

  // ─────────────────────────────────────────────────────────────────────────
  // Confidence
  // ─────────────────────────────────────────────────────────────────────────

  /** Human-asserted confidence level */
  confidence: EddaConfidenceLevelSchema,

  /** Rationale for confidence level */
  confidence_rationale: z.string().optional(),

  // ─────────────────────────────────────────────────────────────────────────
  // Provenance
  // ─────────────────────────────────────────────────────────────────────────

  /** Full provenance chain (Kindling → Ember → Edda) */
  provenance: ProvenanceChainSchema,

  // ─────────────────────────────────────────────────────────────────────────
  // Attribution
  // ─────────────────────────────────────────────────────────────────────────

  /** Who promoted this memory and when */
  attribution: AttributionSchema,

  // ─────────────────────────────────────────────────────────────────────────
  // Evolution
  // ─────────────────────────────────────────────────────────────────────────

  /** Evolution tracking (supersedes/superseded_by) */
  evolution: EvolutionSchema.default({}),

  // ─────────────────────────────────────────────────────────────────────────
  // Temporal
  // ─────────────────────────────────────────────────────────────────────────

  /** When this memory was created */
  created_at: TimestampSchema,

  /** When this memory was last updated */
  updated_at: TimestampSchema.optional(),
});

export type MemoryObject = z.infer<typeof MemoryObjectSchema>;

// =============================================================================
// Type-Specific Metadata Schemas
// =============================================================================

/**
 * Metadata for "decision" memories
 */
export const DecisionMemoryMetadataSchema = z.object({
  decision_point: z.string().describe('What decision was made'),
  alternatives_considered: z.array(z.string()).optional(),
  outcome: z.string().optional(),
  reversible: z.boolean().optional(),
});

/**
 * Metadata for "pattern" memories
 */
export const PatternMemoryMetadataSchema = z.object({
  pattern_name: z.string(),
  applies_to: z.array(z.string()).optional(),
  anti_pattern: z.boolean().optional().default(false),
});

/**
 * Metadata for "constraint" memories
 */
export const ConstraintMemoryMetadataSchema = z.object({
  constraint_type: z.enum(['technical', 'process', 'policy', 'resource']),
  enforcement: z.enum(['hard', 'soft', 'advisory']).optional(),
  workaround: z.string().optional(),
});

/**
 * Metadata for "warning" memories
 */
export const WarningMemoryMetadataSchema = z.object({
  severity: z.enum(['low', 'medium', 'high', 'critical']),
  affected_areas: z.array(z.string()).optional(),
  mitigation: z.string().optional(),
});

/**
 * Metadata for "doctrine" memories
 */
export const DoctrineMemoryMetadataSchema = z.object({
  principle: z.string(),
  source: z.string().optional().describe('Where this doctrine comes from'),
  exceptions: z.array(z.string()).optional(),
});

/**
 * Metadata for "lesson" memories
 */
export const LessonMemoryMetadataSchema = z.object({
  lesson_type: z.enum(['success', 'failure', 'mixed']),
  applicable_to: z.array(z.string()).optional(),
  key_takeaway: z.string().optional(),
});

// =============================================================================
// Promotion Input (Ember → Edda)
// =============================================================================

/**
 * Input for promoting an Ember proposal to Edda memory
 */
export const PromoteProposalInputSchema = z.object({
  /** The Ember proposal ID to promote */
  proposal_id: ProposalIdSchema,

  /** Override the statement (defaults to proposal summary) */
  statement: z.string().min(1).max(2000).optional(),

  /** Memory type (may differ from proposal type) */
  type: MemoryTypeSchema,

  /** Human-asserted confidence */
  confidence: EddaConfidenceLevelSchema,

  /** Confidence rationale */
  confidence_rationale: z.string().optional(),

  /** Context for the memory */
  context: MemoryContextSchema,

  /** Attribution (who is promoting) */
  promoted_by: z.string().describe('User identifier'),

  /** Reason for promotion */
  reason: z.string().describe('Why this is being promoted'),

  /** Additional metadata */
  metadata: z.record(z.unknown()).optional(),
});

export type PromoteProposalInput = z.infer<typeof PromoteProposalInputSchema>;

// =============================================================================
// Query Schemas
// =============================================================================

/**
 * Filters for querying memories
 */
export const MemoryQuerySchema = z.object({
  /** Filter by type(s) */
  types: z.array(MemoryTypeSchema).optional(),

  /** Filter by status(es) */
  statuses: z.array(MemoryStatusSchema).optional(),

  /** Filter by confidence level(s) */
  confidence_levels: z.array(EddaConfidenceLevelSchema).optional(),

  /** Created after this timestamp */
  created_after: TimestampSchema.optional(),

  /** Created before this timestamp */
  created_before: TimestampSchema.optional(),

  /** Filter by tags (any match) */
  tags: z.array(z.string()).optional(),

  /** Full-text search in statement */
  search: z.string().optional(),

  /** Include superseded memories */
  include_superseded: z.boolean().optional().default(false),

  /** Limit results */
  limit: z.number().int().positive().optional().default(100),

  /** Offset for pagination */
  offset: z.number().int().nonnegative().optional().default(0),

  /** Sort by field */
  sort_by: z.enum(['created_at', 'updated_at', 'type']).optional().default('created_at'),

  /** Sort direction */
  sort_order: z.enum(['asc', 'desc']).optional().default('desc'),
});

export type MemoryQuery = z.infer<typeof MemoryQuerySchema>;

/**
 * Query result wrapper
 */
export const MemoryQueryResultSchema = z.object({
  memories: z.array(MemoryObjectSchema),
  total: z.number().int().nonnegative(),
  limit: z.number().int().positive(),
  offset: z.number().int().nonnegative(),
  has_more: z.boolean(),
});

export type MemoryQueryResult = z.infer<typeof MemoryQueryResultSchema>;

// =============================================================================
// Type Mapping (Ember ProposalType → Edda MemoryType)
// =============================================================================

/**
 * Default mapping from Ember proposal types to Edda memory types
 *
 * Note: Not all proposal types map directly. Humans choose the final type.
 */
export const proposalToMemoryTypeMapping: Record<string, MemoryType | null> = {
  decision: 'decision',
  pattern: 'pattern',
  warning: 'warning',
  lesson: 'lesson',
  anomaly: null, // Anomalies don't map directly — human decides
  constraint: 'constraint',
};

/**
 * Suggest a memory type from a proposal type
 * Returns null if no direct mapping exists
 */
export function suggestMemoryType(proposalType: string): MemoryType | null {
  return proposalToMemoryTypeMapping[proposalType] ?? null;
}
