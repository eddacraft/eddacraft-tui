/**
 * Type Mappings (STACK-005)
 *
 * Maps between Ember proposal types and Edda memory types.
 * Provides utilities for the promotion workflow from Ember to Edda.
 *
 * Key functions:
 * - Type mapping: ProposalType -> MemoryType (with null for anomaly)
 * - Confidence mapping: EmberConfidence -> EddaConfidenceLevel
 * - Promotion input creation: CandidateProposal -> PromoteProposalInput
 * - Provenance expansion: ProvenanceSummary -> ProvenanceChain
 *
 * @module @eddacraft/anvil-edda-stack/contracts/type-mappings
 */

import type { ProposalType, CandidateProposal } from './ember-proposal.js';
import type { MemoryType, PromoteProposalInput, MemoryContext } from './edda-memory.js';
import type { EmberConfidence, EddaConfidenceLevel } from './confidence.js';
import type { ProvenanceSummary, ProvenanceChain, KindlingRef } from './provenance.js';
import { suggestEddaConfidence } from './confidence.js';
import type { Timestamp } from './temporal.js';
import type { ObservationId, SessionId, ProposalId } from './identifiers.js';

// =============================================================================
// Type Mapping Constant
// =============================================================================

/**
 * Mapping from Ember ProposalType to Edda MemoryType
 *
 * Most proposal types map directly to memory types with the same name.
 * The exception is 'anomaly' which requires human choice (returns null).
 *
 * This mapping is used during the promotion workflow to suggest
 * an appropriate memory type for a given proposal.
 */
export const PROPOSAL_TO_MEMORY_TYPE_MAPPING: Record<ProposalType, MemoryType | null> = {
  decision: 'decision',
  pattern: 'pattern',
  warning: 'warning',
  lesson: 'lesson',
  constraint: 'constraint',
  anomaly: null, // Anomalies require human choice - no direct mapping
} as const;

// =============================================================================
// Type Mapping Functions
// =============================================================================

/**
 * Map a proposal type to a memory type
 *
 * Returns the suggested memory type for a given proposal type.
 * Returns null for 'anomaly' as it requires human decision.
 *
 * @param proposalType - The Ember proposal type
 * @returns The suggested Edda memory type, or null if human choice required
 *
 * @example
 * ```ts
 * mapProposalToMemoryType('decision') // => 'decision'
 * mapProposalToMemoryType('anomaly')  // => null (requires human choice)
 * ```
 */
export function mapProposalToMemoryType(proposalType: ProposalType): MemoryType | null {
  return PROPOSAL_TO_MEMORY_TYPE_MAPPING[proposalType];
}

/**
 * Map Ember confidence score to Edda confidence level
 *
 * Uses the existing suggestEddaConfidence function to convert
 * a numeric Ember confidence (0.0-1.0) to a categorical Edda level.
 *
 * Mapping:
 * - 0.00-0.49 -> 'low'
 * - 0.50-0.74 -> 'medium'
 * - 0.75-1.00 -> 'high'
 *
 * @param emberConfidence - The numeric Ember confidence score (0.0-1.0)
 * @returns The suggested Edda confidence level
 *
 * @example
 * ```ts
 * mapProposalConfidence(0.3)  // => 'low'
 * mapProposalConfidence(0.6)  // => 'medium'
 * mapProposalConfidence(0.85) // => 'high'
 * ```
 */
export function mapProposalConfidence(emberConfidence: EmberConfidence): EddaConfidenceLevel {
  return suggestEddaConfidence(emberConfidence);
}

// =============================================================================
// Promotion Input Creation
// =============================================================================

/**
 * Create a PromoteProposalInput from a CandidateProposal
 *
 * Generates the input structure needed to promote an Ember proposal
 * to an Edda memory. This provides sensible defaults while allowing
 * the caller to provide promotion-specific information.
 *
 * Note: For 'anomaly' proposals, the caller must provide an explicit
 * memoryType in the options since there is no default mapping.
 *
 * @param proposal - The Ember candidate proposal to promote
 * @param promotedBy - User identifier performing the promotion
 * @param reason - Human-provided reason for promotion
 * @param options - Optional overrides for type and confidence
 * @returns The promotion input ready for the Edda promotion API
 *
 * @throws {Error} If proposal type is 'anomaly' and no memoryType override provided
 *
 * @example
 * ```ts
 * const input = createPromotionInput(
 *   proposal,
 *   'user@example.com',
 *   'This pattern is well-established and valuable'
 * );
 * ```
 */
export function createPromotionInput(
  proposal: CandidateProposal,
  promotedBy: string,
  reason: string,
  options?: {
    /** Override the memory type (required for anomaly proposals) */
    memoryType?: MemoryType;
    /** Override the confidence level */
    confidence?: EddaConfidenceLevel;
    /** Override the statement (defaults to proposal summary) */
    statement?: string;
    /** Additional context conditions */
    conditions?: string[];
    /** Scope limitation */
    scope?: string;
    /** Tags for categorization */
    tags?: string[];
  }
): PromoteProposalInput {
  // Determine memory type
  const suggestedType = mapProposalToMemoryType(proposal.type);
  const memoryType = options?.memoryType ?? suggestedType;

  if (memoryType === null) {
    throw new Error(
      `Cannot create promotion input for proposal type '${proposal.type}' without explicit memoryType. ` +
        `Anomaly proposals require human choice of memory type.`
    );
  }

  // Determine confidence level
  const confidence = options?.confidence ?? mapProposalConfidence(proposal.confidence);

  // Build context
  const context: MemoryContext = {
    when: proposal.created_at,
    why: proposal.rationale,
    conditions: options?.conditions ?? [],
    scope: options?.scope,
    tags: options?.tags ?? [],
  };

  return {
    proposal_id: proposal.id,
    statement: options?.statement ?? proposal.summary,
    type: memoryType,
    confidence,
    confidence_rationale: `Suggested from Ember confidence of ${(proposal.confidence * 100).toFixed(0)}%`,
    context,
    promoted_by: promotedBy,
    reason,
    metadata: proposal.metadata,
  };
}

// =============================================================================
// Provenance Expansion
// =============================================================================

/**
 * Expand a ProvenanceSummary into a full ProvenanceChain
 *
 * Converts the lightweight summary format (used in proposals) into
 * the full chain format (used in memories). This involves expanding
 * observation IDs into full KindlingRef objects.
 *
 * Note: Since the summary doesn't contain observation kinds, this
 * function uses a generic 'observation' kind. For more precise
 * provenance, the caller should provide the observation kinds.
 *
 * @param summary - The lightweight provenance summary from a proposal
 * @param observationKinds - Optional map of observation IDs to their kinds
 * @returns The expanded provenance chain
 *
 * @example
 * ```ts
 * const chain = expandProvenanceSummary(proposal.provenance);
 * // Use in memory creation with full audit trail
 * ```
 */
export function expandProvenanceSummary(
  summary: ProvenanceSummary,
  observationKinds?: Record<string, string>
): ProvenanceChain {
  // Calculate timestamps for each observation
  // If we have multiple observations, distribute them between earliest and latest
  const observationCount = summary.observation_ids.length;
  const earliestTime = new Date(summary.earliest_observation).getTime();
  const latestTime = new Date(summary.latest_observation).getTime();
  const timeSpan = latestTime - earliestTime;

  // Create KindlingRef for each observation
  const kindling_sources: KindlingRef[] = summary.observation_ids.map((obsId, index) => {
    // Distribute timestamps evenly across the time range
    const timestamp =
      observationCount === 1
        ? summary.earliest_observation
        : (new Date(
            earliestTime + (timeSpan * index) / (observationCount - 1)
          ).toISOString() as Timestamp);

    // Use provided kind or default to 'observation'
    const kind = observationKinds?.[obsId] ?? 'observation';

    // Use first session ID as default, or cycle through available sessions
    const sessionIndex = index % summary.session_ids.length;
    const sessionId = summary.session_ids[sessionIndex];

    return {
      observation_id: obsId as ObservationId,
      session_id: sessionId as SessionId,
      kind,
      timestamp,
    };
  });

  // Build the chain
  const chain: ProvenanceChain = {
    kindling_sources,
    source_sessions: summary.session_ids as SessionId[],
  };

  // Add ember_source if proposal_id exists
  if (summary.proposal_id) {
    chain.ember_source = {
      proposal_id: summary.proposal_id as ProposalId,
      proposal_type: 'unknown', // Summary doesn't include type
      confidence: 0, // Summary doesn't include confidence
      created_at: summary.earliest_observation,
    };
  }

  return chain;
}
