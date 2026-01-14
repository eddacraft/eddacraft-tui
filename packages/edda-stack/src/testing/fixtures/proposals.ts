/**
 * Proposal Fixtures (STACK-010)
 *
 * Factory functions for creating valid CandidateProposal test fixtures.
 * All fixtures pass Zod validation.
 *
 * @module @anvil/edda-stack/testing/fixtures/proposals
 */

import { v4 as uuidv4 } from 'uuid';
import type {
  CandidateProposal,
  ProposalType,
  ProposalStatus,
  EvaluationSignal,
} from '../../contracts/ember-proposal.js';
import { CandidateProposalSchema } from '../../contracts/ember-proposal.js';
import type { ProposalId, Timestamp } from '../../contracts/index.js';
import type { ProvenanceSummary } from '../../contracts/provenance.js';
import { createProposalId, createSessionId } from '../../contracts/identifiers.js';
import { now, calculateExpiry } from '../../contracts/temporal.js';

// =============================================================================
// Types
// =============================================================================

/**
 * Override options for proposal fixtures
 */
export interface ProposalFixtureOverrides {
  id?: ProposalId;
  type?: ProposalType;
  status?: ProposalStatus;
  summary?: string;
  rationale?: string;
  confidence?: number;
  signals?: EvaluationSignal[];
  provenance?: Partial<ProvenanceSummary>;
  created_at?: Timestamp;
  expires_at?: Timestamp;
  ttl_days?: number;
  metadata?: Record<string, unknown>;
  resolution?: {
    resolved_at?: Timestamp;
    resolved_by?: string;
    resolution_reason?: string;
    memory_id?: string;
  };
}

// =============================================================================
// Generic Factory
// =============================================================================

/**
 * Create a valid proposal fixture
 */
export function createProposalFixture(
  type: ProposalType,
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  const id = overrides.id ?? createProposalId(uuidv4());
  const sessionId = createSessionId(uuidv4());
  const observationId = uuidv4();
  const createdAt = overrides.created_at ?? now();
  const ttlDays = overrides.ttl_days ?? 30;

  const baseProposal = {
    id,
    type: overrides.type ?? type,
    status: overrides.status ?? 'active',
    summary: overrides.summary ?? getDefaultSummary(type),
    rationale: overrides.rationale ?? getDefaultRationale(type),
    confidence: overrides.confidence ?? 0.7,
    signals: overrides.signals ?? getDefaultSignals(type),
    provenance: {
      observation_ids: [observationId],
      session_ids: [sessionId],
      earliest_observation: createdAt,
      latest_observation: createdAt,
      ...overrides.provenance,
    },
    created_at: createdAt,
    expires_at: overrides.expires_at ?? calculateExpiry(createdAt, ttlDays),
    ttl_days: ttlDays,
    metadata: overrides.metadata ?? getDefaultMetadata(type),
    ...(overrides.resolution && { resolution: overrides.resolution }),
  };

  // Validate and return
  return CandidateProposalSchema.parse(baseProposal);
}

// =============================================================================
// Type-Specific Factories
// =============================================================================

/**
 * Create a valid decision proposal
 */
export function createValidDecisionProposal(
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  return createProposalFixture('decision', {
    summary: 'Team decided to adopt new testing framework',
    rationale: 'The current framework lacks TypeScript support and modern features',
    confidence: 0.85,
    metadata: {
      decision_point: 'Testing framework selection',
      alternatives_considered: ['Jest', 'Vitest', 'Mocha'],
      outcome_observed: 'Chose Vitest for better ESM support',
    },
    ...overrides,
  });
}

/**
 * Create a valid pattern proposal
 */
export function createValidPatternProposal(
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  return createProposalFixture('pattern', {
    summary: 'Repository pattern used for data access layer',
    rationale: 'Consistent abstraction pattern observed across multiple services',
    confidence: 0.75,
    metadata: {
      pattern_name: 'Repository Pattern',
      occurrence_count: 5,
      first_seen: new Date(Date.now() - 86400000 * 7).toISOString(),
      last_seen: now(),
    },
    ...overrides,
  });
}

/**
 * Create a valid warning proposal
 */
export function createValidWarningProposal(
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  return createProposalFixture('warning', {
    summary: 'Increasing cyclomatic complexity in core module',
    rationale: 'Complexity metrics have exceeded threshold in recent commits',
    confidence: 0.65,
    metadata: {
      warning_type: 'complexity',
      severity: 'medium',
      affected_areas: ['src/core/processor.ts', 'src/core/evaluator.ts'],
    },
    ...overrides,
  });
}

/**
 * Create a valid lesson proposal
 */
export function createValidLessonProposal(
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  return createProposalFixture('lesson', {
    summary: 'Premature optimization led to unmaintainable code',
    rationale: 'Refactoring effort was needed after over-optimizing the hot path',
    confidence: 0.7,
    metadata: {
      lesson_type: 'failure',
      context: 'Performance optimization sprint',
      applicable_to: ['performance work', 'optimization tasks'],
    },
    ...overrides,
  });
}

/**
 * Create a valid anomaly proposal
 */
export function createValidAnomalyProposal(
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  return createProposalFixture('anomaly', {
    summary: 'Unusual spike in API response times',
    rationale: 'Response times deviated significantly from baseline',
    confidence: 0.6,
    metadata: {
      expected_behaviour: 'Response time < 100ms',
      actual_behaviour: 'Response time averaging 450ms',
      deviation_magnitude: 3.5,
    },
    ...overrides,
  });
}

/**
 * Create a valid constraint proposal
 */
export function createValidConstraintProposal(
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  return createProposalFixture('constraint', {
    summary: 'Maximum file size limit for uploads is 10MB',
    rationale: 'Discovered through trial and error during file upload feature',
    confidence: 0.9,
    metadata: {
      constraint_type: 'technical',
      scope: 'file-upload-service',
      discovered_via: 'Error during large file upload',
    },
    ...overrides,
  });
}

// =============================================================================
// Status Variant Factories
// =============================================================================

/**
 * Create an active proposal (default)
 */
export function createActiveProposal(
  type: ProposalType = 'pattern',
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  return createProposalFixture(type, {
    status: 'active',
    ...overrides,
  });
}

/**
 * Create an expired proposal
 */
export function createExpiredProposal(
  type: ProposalType = 'pattern',
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  const expiredAt = new Date(Date.now() - 86400000).toISOString() as Timestamp; // 1 day ago
  const createdAt = new Date(Date.now() - 86400000 * 31).toISOString() as Timestamp; // 31 days ago

  return createProposalFixture(type, {
    status: 'expired',
    created_at: createdAt,
    expires_at: expiredAt,
    ttl_days: 30,
    resolution: {
      resolved_at: expiredAt,
      resolution_reason: 'TTL expired',
    },
    ...overrides,
  });
}

/**
 * Create a promoted proposal
 */
export function createPromotedProposal(
  type: ProposalType = 'decision',
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  const resolvedAt = now();

  return createProposalFixture(type, {
    status: 'promoted',
    confidence: 0.85,
    resolution: {
      resolved_at: resolvedAt,
      resolved_by: 'user@example.com',
      resolution_reason: 'Valuable insight worth preserving',
      memory_id: uuidv4(),
    },
    ...overrides,
  });
}

/**
 * Create a dismissed proposal
 */
export function createDismissedProposal(
  type: ProposalType = 'anomaly',
  overrides: ProposalFixtureOverrides = {}
): CandidateProposal {
  const resolvedAt = now();

  return createProposalFixture(type, {
    status: 'dismissed',
    confidence: 0.4,
    resolution: {
      resolved_at: resolvedAt,
      resolved_by: 'user@example.com',
      resolution_reason: 'False positive - not a real anomaly',
    },
    ...overrides,
  });
}

// =============================================================================
// Batch Factories
// =============================================================================

/**
 * Create a set of proposals of all types
 */
export function createProposalsOfAllTypes(): CandidateProposal[] {
  return [
    createValidDecisionProposal(),
    createValidPatternProposal(),
    createValidWarningProposal(),
    createValidLessonProposal(),
    createValidAnomalyProposal(),
    createValidConstraintProposal(),
  ];
}

/**
 * Create a set of proposals with all statuses
 */
export function createProposalsOfAllStatuses(): CandidateProposal[] {
  return [
    createActiveProposal('pattern'),
    createExpiredProposal('warning'),
    createPromotedProposal('decision'),
    createDismissedProposal('anomaly'),
  ];
}

// =============================================================================
// Helper Functions
// =============================================================================

function getDefaultSummary(type: ProposalType): string {
  const summaries: Record<ProposalType, string> = {
    decision: 'A decision was made regarding project architecture',
    pattern: 'A recurring code pattern was observed',
    warning: 'A potential issue was detected',
    lesson: 'A lesson was learned from recent work',
    anomaly: 'An unexpected behaviour was observed',
    constraint: 'A constraint or limitation was discovered',
  };
  return summaries[type];
}

function getDefaultRationale(type: ProposalType): string {
  const rationales: Record<ProposalType, string> = {
    decision: 'This decision affects the long-term architecture and should be documented',
    pattern: 'This pattern appears multiple times and may be worth codifying',
    warning: 'This warning indicates potential technical debt or risk',
    lesson: 'This lesson could help avoid similar issues in the future',
    anomaly: 'This anomaly may indicate a bug or unexpected system behaviour',
    constraint: 'This constraint should be documented to avoid wasted effort',
  };
  return rationales[type];
}

function getDefaultSignals(type: ProposalType): EvaluationSignal[] {
  const signals: Record<ProposalType, EvaluationSignal[]> = {
    decision: [
      { rule: 'explicit_decision', contribution: 0.8, weight: 2.0 },
      { rule: 'scope_impact', contribution: 0.6, weight: 1.0 },
    ],
    pattern: [
      { rule: 'repetition', contribution: 0.7, weight: 1.5 },
      { rule: 'consistency', contribution: 0.6, weight: 1.0 },
    ],
    warning: [
      { rule: 'threshold_breach', contribution: 0.65, weight: 1.2 },
      { rule: 'trend_detection', contribution: 0.5, weight: 1.0 },
    ],
    lesson: [
      { rule: 'outcome_correlation', contribution: 0.7, weight: 1.5 },
      { rule: 'retrospective', contribution: 0.6, weight: 1.0 },
    ],
    anomaly: [
      { rule: 'deviation_detection', contribution: 0.6, weight: 1.3 },
      { rule: 'baseline_comparison', contribution: 0.5, weight: 1.0 },
    ],
    constraint: [
      { rule: 'boundary_discovery', contribution: 0.8, weight: 1.5 },
      { rule: 'error_correlation', contribution: 0.7, weight: 1.2 },
    ],
  };
  return signals[type];
}

function getDefaultMetadata(type: ProposalType): Record<string, unknown> {
  const metadata: Record<ProposalType, Record<string, unknown>> = {
    decision: {
      decision_point: 'Generic decision point',
      alternatives_considered: ['Option A', 'Option B'],
    },
    pattern: {
      pattern_name: 'Generic Pattern',
      occurrence_count: 3,
      first_seen: new Date(Date.now() - 86400000 * 7).toISOString(),
      last_seen: now(),
    },
    warning: {
      warning_type: 'generic',
      severity: 'medium',
      affected_areas: ['src/'],
    },
    lesson: {
      lesson_type: 'mixed',
      context: 'General development work',
    },
    anomaly: {
      expected_behaviour: 'Normal operation',
      actual_behaviour: 'Unexpected behaviour',
    },
    constraint: {
      constraint_type: 'technical',
      scope: 'general',
    },
  };
  return metadata[type];
}
