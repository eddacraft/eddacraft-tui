/**
 * Provenance Chain Validator (STACK-011)
 *
 * Validation utilities for provenance chains across the Edda Stack.
 * Validates references exist and temporal ordering is correct.
 *
 * @module @eddacraft/anvil-edda-stack/testing/validators/provenance-chain
 */

import type { ProvenanceChain, KindlingRef, EmberRef } from '../../contracts/provenance.js';
import type { IKindlingPort } from '../../contracts/ports/kindling.port.js';
import type { IEmberPort } from '../../contracts/ports/ember.port.js';
import type { SessionId, Timestamp } from '../../contracts/index.js';

// =============================================================================
// Validation Result Types
// =============================================================================

/**
 * Validation codes for provenance chain issues
 */
export enum ProvenanceValidationCode {
  /** No issues found */
  VALID = 'VALID',

  /** A referenced observation was not found in Kindling */
  MISSING_OBSERVATION = 'MISSING_OBSERVATION',

  /** A referenced session was not found */
  MISSING_SESSION = 'MISSING_SESSION',

  /** The referenced Ember proposal was not found */
  MISSING_PROPOSAL = 'MISSING_PROPOSAL',

  /** An observation references a session not in source_sessions */
  SESSION_MISMATCH = 'SESSION_MISMATCH',

  /** Observation timestamp is before proposal creation */
  TEMPORAL_OBSERVATION_BEFORE_PROPOSAL = 'TEMPORAL_OBSERVATION_BEFORE_PROPOSAL',

  /** Observation timestamp is after promotion time */
  TEMPORAL_OBSERVATION_AFTER_PROMOTION = 'TEMPORAL_OBSERVATION_AFTER_PROMOTION',

  /** Earliest observation is after latest observation */
  TEMPORAL_RANGE_INVALID = 'TEMPORAL_RANGE_INVALID',

  /** No Kindling sources in the chain */
  NO_KINDLING_SOURCES = 'NO_KINDLING_SOURCES',

  /** Empty source_sessions array */
  NO_SOURCE_SESSIONS = 'NO_SOURCE_SESSIONS',

  /** Duplicate observation IDs */
  DUPLICATE_OBSERVATIONS = 'DUPLICATE_OBSERVATIONS',

  /** Ember source missing proposal_id */
  EMBER_SOURCE_INCOMPLETE = 'EMBER_SOURCE_INCOMPLETE',
}

/**
 * A single validation issue
 */
export interface ProvenanceValidationIssue {
  /** The validation code */
  code: ProvenanceValidationCode;

  /** Human-readable description */
  message: string;

  /** Additional context about the issue */
  context?: Record<string, unknown>;
}

/**
 * Result of provenance chain validation
 */
export interface ProvenanceValidationResult {
  /** Whether the chain is valid */
  valid: boolean;

  /** List of issues found (empty if valid) */
  issues: ProvenanceValidationIssue[];

  /** Summary statistics */
  stats: {
    /** Number of Kindling refs checked */
    kindlingRefsChecked: number;

    /** Number of Kindling refs found */
    kindlingRefsFound: number;

    /** Number of sessions checked */
    sessionsChecked: number;

    /** Whether Ember ref was checked */
    emberRefChecked: boolean;

    /** Whether Ember ref was found */
    emberRefFound: boolean;
  };
}

// =============================================================================
// Main Validator
// =============================================================================

/**
 * Validate a complete provenance chain
 *
 * Checks:
 * - All Kindling observation references exist
 * - All sessions are consistent
 * - Ember proposal reference exists (if present)
 * - Temporal ordering is valid
 */
export async function validateProvenanceChain(
  chain: ProvenanceChain,
  kindlingPort: IKindlingPort,
  emberPort: IEmberPort
): Promise<ProvenanceValidationResult> {
  const issues: ProvenanceValidationIssue[] = [];
  const stats = {
    kindlingRefsChecked: 0,
    kindlingRefsFound: 0,
    sessionsChecked: 0,
    emberRefChecked: false,
    emberRefFound: false,
  };

  // Structural validation
  if (!chain.kindling_sources || chain.kindling_sources.length === 0) {
    issues.push({
      code: ProvenanceValidationCode.NO_KINDLING_SOURCES,
      message: 'Provenance chain has no Kindling sources',
    });
  }

  if (!chain.source_sessions || chain.source_sessions.length === 0) {
    issues.push({
      code: ProvenanceValidationCode.NO_SOURCE_SESSIONS,
      message: 'Provenance chain has no source sessions',
    });
  }

  // Validate Kindling references
  const kindlingResult = await validateKindlingRefs(chain.kindling_sources, kindlingPort);
  issues.push(...kindlingResult.issues);
  stats.kindlingRefsChecked = kindlingResult.checked;
  stats.kindlingRefsFound = kindlingResult.found;

  // Validate session consistency
  const sessionResult = validateSessionConsistency(chain.kindling_sources, chain.source_sessions);
  issues.push(...sessionResult.issues);
  stats.sessionsChecked = chain.source_sessions?.length ?? 0;

  // Validate Ember reference if present
  if (chain.ember_source) {
    stats.emberRefChecked = true;
    const emberResult = await validateEmberRef(chain.ember_source, emberPort);
    issues.push(...emberResult.issues);
    stats.emberRefFound = emberResult.found;

    // Validate temporal ordering with Ember
    if (emberResult.found && chain.kindling_sources) {
      const temporalResult = validateTemporalOrdering(
        chain.kindling_sources,
        chain.ember_source.created_at
      );
      issues.push(...temporalResult.issues);
    }
  }

  // Check for duplicate observations
  const duplicateResult = validateNoDuplicates(chain.kindling_sources);
  issues.push(...duplicateResult.issues);

  return {
    valid: issues.length === 0,
    issues,
    stats,
  };
}

// =============================================================================
// Individual Validators
// =============================================================================

/**
 * Validate that all Kindling observation references exist
 */
export async function validateKindlingRefs(
  refs: KindlingRef[] | undefined,
  kindlingPort: IKindlingPort
): Promise<{
  issues: ProvenanceValidationIssue[];
  checked: number;
  found: number;
}> {
  const issues: ProvenanceValidationIssue[] = [];
  let found = 0;

  if (!refs || refs.length === 0) {
    return { issues, checked: 0, found: 0 };
  }

  for (const ref of refs) {
    const exists = await kindlingPort.observationExists(ref.observation_id);
    if (exists) {
      found++;
    } else {
      issues.push({
        code: ProvenanceValidationCode.MISSING_OBSERVATION,
        message: `Observation ${ref.observation_id} not found in Kindling`,
        context: {
          observation_id: ref.observation_id,
          session_id: ref.session_id,
          kind: ref.kind,
          timestamp: ref.timestamp,
        },
      });
    }
  }

  return { issues, checked: refs.length, found };
}

/**
 * Validate that the Ember proposal reference exists
 */
export async function validateEmberRef(
  ref: EmberRef,
  emberPort: IEmberPort
): Promise<{
  issues: ProvenanceValidationIssue[];
  found: boolean;
}> {
  const issues: ProvenanceValidationIssue[] = [];

  if (!ref.proposal_id) {
    issues.push({
      code: ProvenanceValidationCode.EMBER_SOURCE_INCOMPLETE,
      message: 'Ember source is missing proposal_id',
      context: { ref },
    });
    return { issues, found: false };
  }

  const exists = await emberPort.proposalExists(ref.proposal_id);
  if (!exists) {
    issues.push({
      code: ProvenanceValidationCode.MISSING_PROPOSAL,
      message: `Proposal ${ref.proposal_id} not found in Ember`,
      context: {
        proposal_id: ref.proposal_id,
        proposal_type: ref.proposal_type,
        confidence: ref.confidence,
        created_at: ref.created_at,
      },
    });
    return { issues, found: false };
  }

  return { issues, found: true };
}

/**
 * Validate temporal ordering of observations
 *
 * Checks that observations are not after the proposal creation time
 * (observations should happen before or during proposal creation)
 */
export function validateTemporalOrdering(
  refs: KindlingRef[],
  proposalCreatedAt: Timestamp
): { issues: ProvenanceValidationIssue[] } {
  const issues: ProvenanceValidationIssue[] = [];
  const proposalTime = new Date(proposalCreatedAt).getTime();

  // Allow a small grace period (5 minutes) for clock skew
  const gracePeriodMs = 5 * 60 * 1000;

  for (const ref of refs) {
    const obsTime = new Date(ref.timestamp).getTime();

    // Observation should not be significantly after proposal creation
    if (obsTime > proposalTime + gracePeriodMs) {
      issues.push({
        code: ProvenanceValidationCode.TEMPORAL_OBSERVATION_AFTER_PROMOTION,
        message: `Observation ${ref.observation_id} timestamp is after proposal creation`,
        context: {
          observation_id: ref.observation_id,
          observation_timestamp: ref.timestamp,
          proposal_created_at: proposalCreatedAt,
          difference_ms: obsTime - proposalTime,
        },
      });
    }
  }

  // Validate that observation range is valid
  if (refs.length > 1) {
    const timestamps = refs.map((r) => new Date(r.timestamp).getTime());
    const earliest = Math.min(...timestamps);
    const latest = Math.max(...timestamps);

    // This is a sanity check - earliest should be <= latest
    if (earliest > latest) {
      issues.push({
        code: ProvenanceValidationCode.TEMPORAL_RANGE_INVALID,
        message: 'Earliest observation timestamp is after latest observation timestamp',
        context: {
          earliest: new Date(earliest).toISOString(),
          latest: new Date(latest).toISOString(),
        },
      });
    }
  }

  return { issues };
}

/**
 * Validate that all observation sessions are in source_sessions
 */
export function validateSessionConsistency(
  refs: KindlingRef[] | undefined,
  sourceSessions: SessionId[] | undefined
): { issues: ProvenanceValidationIssue[] } {
  const issues: ProvenanceValidationIssue[] = [];

  if (!refs || !sourceSessions) {
    return { issues };
  }

  const sessionSet = new Set(sourceSessions);

  for (const ref of refs) {
    if (!sessionSet.has(ref.session_id)) {
      issues.push({
        code: ProvenanceValidationCode.SESSION_MISMATCH,
        message: `Observation ${ref.observation_id} references session ${ref.session_id} not in source_sessions`,
        context: {
          observation_id: ref.observation_id,
          observation_session: ref.session_id,
          source_sessions: sourceSessions,
        },
      });
    }
  }

  return { issues };
}

/**
 * Validate that there are no duplicate observation IDs
 */
export function validateNoDuplicates(refs: KindlingRef[] | undefined): {
  issues: ProvenanceValidationIssue[];
} {
  const issues: ProvenanceValidationIssue[] = [];

  if (!refs || refs.length === 0) {
    return { issues };
  }

  const seen = new Set<string>();
  const duplicates: string[] = [];

  for (const ref of refs) {
    if (seen.has(ref.observation_id)) {
      duplicates.push(ref.observation_id);
    }
    seen.add(ref.observation_id);
  }

  if (duplicates.length > 0) {
    issues.push({
      code: ProvenanceValidationCode.DUPLICATE_OBSERVATIONS,
      message: `Duplicate observation IDs found: ${duplicates.join(', ')}`,
      context: {
        duplicates,
      },
    });
  }

  return { issues };
}

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Create a validation result indicating success
 */
export function createValidResult(): ProvenanceValidationResult {
  return {
    valid: true,
    issues: [],
    stats: {
      kindlingRefsChecked: 0,
      kindlingRefsFound: 0,
      sessionsChecked: 0,
      emberRefChecked: false,
      emberRefFound: false,
    },
  };
}

/**
 * Create a validation result with a single issue
 */
export function createInvalidResult(
  code: ProvenanceValidationCode,
  message: string,
  context?: Record<string, unknown>
): ProvenanceValidationResult {
  return {
    valid: false,
    issues: [{ code, message, context }],
    stats: {
      kindlingRefsChecked: 0,
      kindlingRefsFound: 0,
      sessionsChecked: 0,
      emberRefChecked: false,
      emberRefFound: false,
    },
  };
}

/**
 * Check if a validation result contains a specific error code
 */
export function hasValidationCode(
  result: ProvenanceValidationResult,
  code: ProvenanceValidationCode
): boolean {
  return result.issues.some((issue) => issue.code === code);
}

/**
 * Get all issues with a specific code
 */
export function getIssuesByCode(
  result: ProvenanceValidationResult,
  code: ProvenanceValidationCode
): ProvenanceValidationIssue[] {
  return result.issues.filter((issue) => issue.code === code);
}

/**
 * Format validation result as a human-readable string
 */
export function formatValidationResult(result: ProvenanceValidationResult): string {
  if (result.valid) {
    return 'Provenance chain is valid';
  }

  const lines = ['Provenance chain validation failed:'];
  for (const issue of result.issues) {
    lines.push(`  - [${issue.code}] ${issue.message}`);
  }

  lines.push('');
  lines.push('Statistics:');
  lines.push(
    `  - Kindling refs: ${result.stats.kindlingRefsFound}/${result.stats.kindlingRefsChecked} found`
  );
  lines.push(`  - Sessions checked: ${result.stats.sessionsChecked}`);
  if (result.stats.emberRefChecked) {
    lines.push(`  - Ember ref: ${result.stats.emberRefFound ? 'found' : 'not found'}`);
  }

  return lines.join('\n');
}
