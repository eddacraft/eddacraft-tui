/**
 * Provenance Link Schema (STACK-004)
 *
 * Defines cross-layer reference schemas for tracing data lineage.
 * Provenance links enable traversal from Edda → Ember → Kindling.
 *
 * Key principle: Every memory can be traced back to its source observations.
 *
 * @module @anvil/edda-stack/contracts/provenance
 */

import { z } from 'zod';
import {
  ObservationIdSchema,
  SessionIdSchema,
  ProposalIdSchema,
  PlanIdSchema,
  GateIdSchema,
  ActionIdSchema,
  type ObservationId,
  type SessionId,
} from './identifiers.js';
import { TimestampSchema, type Timestamp } from './temporal.js';

// =============================================================================
// Kindling Source References
// =============================================================================

/**
 * Reference to a Kindling observation
 * The atomic unit of provenance — links to a single recorded fact
 */
export const KindlingRefSchema = z.object({
  observation_id: ObservationIdSchema,
  session_id: SessionIdSchema,
  kind: z.string().describe('Observation kind (e.g., "gate_evaluated", "action_executed")'),
  timestamp: TimestampSchema.describe('When the observation was recorded'),
});

export type KindlingRef = z.infer<typeof KindlingRefSchema>;

/**
 * Collection of Kindling references
 * Used when multiple observations contribute to a proposal or memory
 */
export const KindlingRefsSchema = z.array(KindlingRefSchema).min(1);

export type KindlingRefs = z.infer<typeof KindlingRefsSchema>;

// =============================================================================
// Ember Source References
// =============================================================================

/**
 * Reference to an Ember proposal
 * Links an Edda memory back to its candidate proposal
 */
export const EmberRefSchema = z.object({
  proposal_id: ProposalIdSchema,
  proposal_type: z.string().describe('Proposal type (e.g., "pattern", "decision")'),
  confidence: z.number().min(0).max(1).describe('Ember confidence score at promotion time'),
  created_at: TimestampSchema.describe('When the proposal was created'),
});

export type EmberRef = z.infer<typeof EmberRefSchema>;

// =============================================================================
// Full Provenance Chain
// =============================================================================

/**
 * Complete provenance chain for an Edda memory
 *
 * Traces: Memory → Proposal → Observations
 *
 * This is the full audit trail showing how a memory came to exist.
 */
export const ProvenanceChainSchema = z.object({
  /** The Ember proposal this memory was promoted from (if any) */
  ember_source: EmberRefSchema.optional().describe('Ember proposal source (if promoted)'),

  /** Direct Kindling observation references */
  kindling_sources: KindlingRefsSchema.describe('Source observations from Kindling'),

  /** Sessions that contributed observations */
  source_sessions: z.array(SessionIdSchema).describe('Sessions that contributed'),

  /** Plans involved (if any) */
  related_plans: z.array(PlanIdSchema).optional().describe('Related plan IDs'),

  /** Gates involved (if any) */
  related_gates: z.array(GateIdSchema).optional().describe('Related gate IDs'),

  /** Actions involved (if any) */
  related_actions: z.array(ActionIdSchema).optional().describe('Related action IDs'),
});

export type ProvenanceChain = z.infer<typeof ProvenanceChainSchema>;

// =============================================================================
// Provenance Metadata
// =============================================================================

/**
 * Provenance attribution — who created/promoted and when
 */
export const AttributionSchema = z.object({
  /** Who performed this action */
  actor: z.string().describe('User identifier (username, email, etc.)'),

  /** When this action occurred */
  timestamp: TimestampSchema,

  /** How the action was performed */
  method: z
    .enum(['cli_command', 'api_call', 'automatic', 'manual_edit'])
    .describe('How the action was triggered'),

  /** Why this action was taken (human-provided) */
  reason: z.string().optional().describe('Human-provided rationale'),
});

export type Attribution = z.infer<typeof AttributionSchema>;

/**
 * Promotion provenance — specific to Ember → Edda promotion
 */
export const PromotionProvenanceSchema = z.object({
  /** The proposal that was promoted */
  proposal_id: ProposalIdSchema,

  /** Ember confidence at promotion time */
  ember_confidence: z.number().min(0).max(1),

  /** Who promoted and when */
  attribution: AttributionSchema,

  /** Original proposal rationale (copied from Ember) */
  original_rationale: z.string().describe('Rationale from Ember proposal'),
});

export type PromotionProvenance = z.infer<typeof PromotionProvenanceSchema>;

// =============================================================================
// Lightweight Provenance (for embedded use)
// =============================================================================

/**
 * Minimal provenance for embedding in other schemas
 * Used when full chain is too heavy
 */
export const ProvenanceSummarySchema = z.object({
  /** Observation IDs (just the IDs, not full refs) */
  observation_ids: z.array(z.string().uuid()).min(1),

  /** Session IDs that contributed */
  session_ids: z.array(z.string().uuid()),

  /** Ember proposal ID if promoted */
  proposal_id: z.string().uuid().optional(),

  /** Primary timestamp (earliest observation) */
  earliest_observation: TimestampSchema,

  /** Latest timestamp */
  latest_observation: TimestampSchema,
});

export type ProvenanceSummary = z.infer<typeof ProvenanceSummarySchema>;

// =============================================================================
// Provenance Utilities
// =============================================================================

/**
 * Create a minimal KindlingRef
 */
export function createKindlingRef(
  observationId: ObservationId,
  sessionId: SessionId,
  kind: string,
  timestamp: Timestamp
): KindlingRef {
  return {
    observation_id: observationId,
    session_id: sessionId,
    kind,
    timestamp,
  };
}

/**
 * Create a provenance summary from a chain
 */
export function summariseProvenance(chain: ProvenanceChain): ProvenanceSummary {
  const observationIds = chain.kindling_sources.map((ref) => ref.observation_id);
  const timestamps = chain.kindling_sources.map((ref) => new Date(ref.timestamp).getTime());

  return {
    observation_ids: observationIds,
    session_ids: chain.source_sessions,
    proposal_id: chain.ember_source?.proposal_id,
    earliest_observation: new Date(Math.min(...timestamps)).toISOString() as Timestamp,
    latest_observation: new Date(Math.max(...timestamps)).toISOString() as Timestamp,
  };
}

/**
 * Merge multiple provenance chains (for aggregation)
 */
export function mergeProvenanceChains(chains: ProvenanceChain[]): ProvenanceChain {
  const allKindlingSources = chains.flatMap((c) => c.kindling_sources);
  const allSessions = [...new Set(chains.flatMap((c) => c.source_sessions))];
  const allPlans = [...new Set(chains.flatMap((c) => c.related_plans ?? []))];
  const allGates = [...new Set(chains.flatMap((c) => c.related_gates ?? []))];
  const allActions = [...new Set(chains.flatMap((c) => c.related_actions ?? []))];

  // Deduplicate kindling sources by observation_id
  const uniqueKindling = allKindlingSources.reduce((acc, ref) => {
    if (!acc.find((r) => r.observation_id === ref.observation_id)) {
      acc.push(ref);
    }
    return acc;
  }, [] as KindlingRef[]);

  return {
    kindling_sources: uniqueKindling,
    source_sessions: allSessions as SessionId[],
    related_plans: allPlans.length > 0 ? allPlans : undefined,
    related_gates: allGates.length > 0 ? allGates : undefined,
    related_actions: allActions.length > 0 ? allActions : undefined,
    // ember_source is not merged — only set during promotion
  };
}

/**
 * Validate that all observation IDs in a chain are valid UUIDs
 */
export function validateProvenanceIntegrity(chain: ProvenanceChain): {
  valid: boolean;
  issues: string[];
} {
  const issues: string[] = [];

  // Check kindling sources
  if (chain.kindling_sources.length === 0) {
    issues.push('Provenance chain has no Kindling sources');
  }

  // Check session consistency
  const sessionSet = new Set(chain.source_sessions);
  for (const ref of chain.kindling_sources) {
    if (!sessionSet.has(ref.session_id)) {
      issues.push(`Observation ${ref.observation_id} references session not in source_sessions`);
    }
  }

  return {
    valid: issues.length === 0,
    issues,
  };
}
