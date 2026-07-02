/**
 * Edda Stack Contracts
 *
 * Shared type definitions and schemas for the Kindling · Ember · Edda stack.
 *
 * @module @eddacraft/anvil-edda-stack/contracts
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
  // Errors
  ProposalAlreadyResolvedError,
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

// =============================================================================
// Memory Type Definitions (EDDA-002)
// =============================================================================

export {
  // Typed memory schemas (discriminated unions)
  DecisionMemorySchema,
  PatternMemorySchema,
  ConstraintMemorySchema,
  WarningMemorySchema,
  DoctrineMemorySchema,
  LessonMemorySchema,
  TypedMemorySchema,
  // Types
  type DecisionMemory,
  type PatternMemory,
  type ConstraintMemory,
  type WarningMemory,
  type DoctrineMemory,
  type LessonMemory,
  type TypedMemory,
  type MemoryMetadataByType,
  // Utilities
  validateMemoryMetadata,
  createTypedMemory,
  parseTypedMemory,
} from './memory-types.js';

// =============================================================================
// Evolution Graph (EDDA-004)
// =============================================================================

export {
  EvolutionLinkSchema,
  EvolutionNodeSchema,
  EvolutionGraphSchema,
  createEvolutionLink,
  buildEvolutionGraph,
  findRootMemory,
  findLatestMemory,
  getEvolutionPath,
  validateEvolutionGraph,
  type EvolutionLink,
  type EvolutionNode,
  type EvolutionGraph,
} from './evolution.js';

// =============================================================================
// Typed Proposals — Discriminated Unions (EMBER-002)
// =============================================================================

export {
  // Schemas
  DecisionProposalSchema,
  PatternProposalSchema,
  WarningProposalSchema,
  LessonProposalSchema,
  AnomalyProposalSchema,
  ConstraintProposalSchema,
  TypedProposalSchema,
  // Types
  type DecisionProposal,
  type PatternProposal,
  type WarningProposal,
  type LessonProposal,
  type AnomalyProposal,
  type ConstraintProposal,
  type TypedProposal,
  type ProposalMetadataByType,
  // Utilities
  validateProposalMetadata,
  createTypedProposal,
  parseTypedProposal,
} from './proposal-types.js';

// =============================================================================
// Type Mappings (STACK-005)
// =============================================================================

export {
  // Constants
  PROPOSAL_TO_MEMORY_TYPE_MAPPING,
  // Utilities
  mapProposalToMemoryType,
  mapProposalConfidence,
  createPromotionInput,
  expandProvenanceSummary,
} from './type-mappings.js';

export {
  // Constants
  OBSERVATION_KIND_TO_PROPOSAL_TYPE_MAPPING,
  // Utilities
  mapObservationKindToProposalType,
  mapObservationKindsToProposalType,
} from './observation-mappings.js';

// =============================================================================
// Events (STACK-008)
// =============================================================================

export {
  // Schemas
  SourceLayerSchema,
  BaseEventSchema,
  EventTypeSchema,
  ObservationRecordedPayloadSchema,
  SessionCompletedPayloadSchema,
  ProposalCreatedPayloadSchema,
  ProposalNearExpiryPayloadSchema,
  MemoryPromotedPayloadSchema,
  MemoryRetiredPayloadSchema,
  ObservationRecordedEventSchema,
  SessionCompletedEventSchema,
  ProposalCreatedEventSchema,
  ProposalNearExpiryEventSchema,
  MemoryPromotedEventSchema,
  MemoryRetiredEventSchema,
  StackEventSchema,
  // Types
  type SourceLayer,
  type BaseEvent,
  type EventType,
  type ObservationRecordedPayload,
  type SessionCompletedPayload,
  type ProposalCreatedPayload,
  type ProposalNearExpiryPayload,
  type MemoryPromotedPayload,
  type MemoryRetiredPayload,
  type ObservationRecordedEvent,
  type SessionCompletedEvent,
  type ProposalCreatedEvent,
  type ProposalNearExpiryEvent,
  type MemoryPromotedEvent,
  type MemoryRetiredEvent,
  type StackEvent,
  type EventHandler,
  type Unsubscribe,
  type IStackEventBus,
  // Factory functions
  createEvent,
  createObservationRecordedEvent,
  createSessionCompletedEvent,
  createProposalCreatedEvent,
  createProposalNearExpiryEvent,
  createMemoryPromotedEvent,
  createMemoryRetiredEvent,
  // Type guards
  isObservationRecordedEvent,
  isSessionCompletedEvent,
  isProposalCreatedEvent,
  isProposalNearExpiryEvent,
  isMemoryPromotedEvent,
  isMemoryRetiredEvent,
  isFromLayer,
  isKindlingEvent,
  isEmberEvent,
  isEddaEvent,
  // Constants
  eventTypeDescriptions,
} from './events.js';

// =============================================================================
// Ports (STACK-007, STACK-009)
// =============================================================================

export type {
  // Kindling Port
  ObservationKind,
  Observation,
  CreateObservationInput,
  ObservationQuery,
  ObservationQueryResult,
  SessionQueryOptions,
  SessionQueryResult,
  PlanQueryOptions,
  SessionSummary,
  PlanQueryResult,
  IKindlingPort,
  // Ember Port
  UpdateProposalInput,
  ResolveProposalInput,
  ProposalTypeStats,
  ProposalStatusStats,
  EmberStats,
  IEmberPort,
  // Edda Port
  CreateMemoryInput,
  UpdateMemoryInput,
  RetireMemoryInput,
  ProvenanceResolutionResult,
  MemoryTypeStats,
  MemoryStatusStats,
  ConfidenceLevelStats,
  EddaStats,
  IEddaPort,
} from './ports/index.js';
