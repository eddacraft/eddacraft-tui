/**
 * Edda Stack Contracts
 *
 * Shared type definitions and schemas for the Kindling · Ember · Edda stack.
 *
 * @module @anvil/edda-stack/contracts
 */

// =============================================================================
// Identifiers (STACK-001)
// =============================================================================

export {
  // Schemas
  UuidSchema,
  ContentHashSchema,
  ObservationIdSchema,
  SessionIdSchema,
  ProposalIdSchema,
  MemoryIdSchema,
  PlanIdSchema,
  GateIdSchema,
  ActionIdSchema,
  GateEvalIdSchema,
  ErrorIdSchema,
  ConstraintIdSchema,
  // Types
  type ObservationId,
  type SessionId,
  type ProposalId,
  type MemoryId,
  type PlanId,
  type GateId,
  type ActionId,
  type GateEvalId,
  type ErrorId,
  type ConstraintId,
  // Utilities
  createObservationId,
  createSessionId,
  createProposalId,
  createMemoryId,
  createActionId,
  createGateEvalId,
  createErrorId,
  isValidUuid,
  isValidContentHash,
} from './identifiers.js';

// =============================================================================
// Temporal (STACK-002)
// =============================================================================

export {
  // Schemas
  TimestampSchema,
  DurationMsSchema,
  DurationSecondsSchema,
  DurationDaysSchema,
  TimeRangeSchema,
  TtlConfigSchema,
  ExpiryInfoSchema,
  // Types
  type Timestamp,
  type DurationMs,
  type DurationSeconds,
  type DurationDays,
  type TimeRange,
  type TtlConfig,
  type ExpiryInfo,
  // Utilities
  now,
  parseTimestamp,
  isValidTimestamp,
  calculateExpiry,
  isExpired,
  remainingTtlMs,
  durationBetween,
  rangeFromStart,
  lastNDays,
  createExpiryInfo,
} from './temporal.js';

// =============================================================================
// Confidence (STACK-003)
// =============================================================================

export {
  // Schemas
  EmberConfidenceSchema,
  EmberConfidenceThresholdsSchema,
  EddaConfidenceLevelSchema,
  EddaConfidenceSchema,
  // Types
  type EmberConfidence,
  type EmberConfidenceThresholds,
  type EddaConfidenceLevel,
  type EddaConfidence,
  // Constants
  confidenceMappingDefaults,
  // Utilities
  suggestEddaConfidence,
  meetsThreshold,
  clampConfidence,
  averageConfidence,
  maxConfidence,
  weightedConfidence,
  formatEmberConfidence,
  formatEddaConfidence,
} from './confidence.js';

// =============================================================================
// Provenance (STACK-004)
// =============================================================================

export {
  // Schemas
  KindlingRefSchema,
  KindlingRefsSchema,
  EmberRefSchema,
  ProvenanceChainSchema,
  AttributionSchema,
  PromotionProvenanceSchema,
  ProvenanceSummarySchema,
  // Types
  type KindlingRef,
  type KindlingRefs,
  type EmberRef,
  type ProvenanceChain,
  type Attribution,
  type PromotionProvenance,
  type ProvenanceSummary,
  // Utilities
  createKindlingRef,
  summariseProvenance,
  mergeProvenanceChains,
  validateProvenanceIntegrity,
} from './provenance.js';

// =============================================================================
// Ember Proposals (EMBER-001)
// =============================================================================

export {
  // Schemas
  ProposalTypeSchema,
  ProposalStatusSchema,
  EvaluationSignalSchema,
  CandidateProposalSchema,
  CreateProposalInputSchema,
  ProposalQuerySchema,
  ProposalQueryResultSchema,
  // Type-specific metadata schemas
  DecisionMetadataSchema,
  PatternMetadataSchema,
  WarningMetadataSchema,
  LessonMetadataSchema,
  AnomalyMetadataSchema,
  ConstraintMetadataSchema,
  // Types
  type ProposalType,
  type ProposalStatus,
  type EvaluationSignal,
  type CandidateProposal,
  type CreateProposalInput,
  type ProposalQuery,
  type ProposalQueryResult,
  // Constants
  proposalTypeDescriptions,
} from './ember-proposal.js';

// =============================================================================
// Edda Memory (EDDA-001)
// =============================================================================

export {
  // Schemas
  MemoryTypeSchema,
  MemoryStatusSchema,
  MemoryContextSchema,
  EvolutionSchema,
  MemoryObjectSchema,
  PromoteProposalInputSchema,
  MemoryQuerySchema,
  MemoryQueryResultSchema,
  // Type-specific metadata schemas
  DecisionMemoryMetadataSchema,
  PatternMemoryMetadataSchema,
  ConstraintMemoryMetadataSchema,
  WarningMemoryMetadataSchema,
  DoctrineMemoryMetadataSchema,
  LessonMemoryMetadataSchema,
  // Types
  type MemoryType,
  type MemoryStatus,
  type MemoryContext,
  type Evolution,
  type MemoryObject,
  type PromoteProposalInput,
  type MemoryQuery,
  type MemoryQueryResult,
  // Constants
  MEMORY_SCHEMA_VERSION,
  memoryTypeDescriptions,
  proposalToMemoryTypeMapping,
  // Utilities
  suggestMemoryType,
} from './edda-memory.js';
