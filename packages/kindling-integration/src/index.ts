/**
 * Kindling Integration (v1)
 *
 * This package provides the complete integration layer between Anvil and Kindling.
 *
 * Three surfaces:
 *
 * 1. Observation Contract (Write-Only)
 *    - 11 observation kinds covering session, plan, gate, action, constraint, human, error
 *    - Immutable, timestamped, linked facts
 *
 * 2. Query Contract (Read-Only)
 *    - 4 query scopes: session, plan, gate, action
 *    - Mandatory constraints, throttling, output guarantees
 *
 * 3. Service Layer (Orchestration)
 *    - KindlingService: validation, sensitive-data checks, store delegation
 *    - Emitters: fire-and-forget observation emission
 *    - KindlingQueryService: high-level query convenience methods
 *    - Configuration, retention, and query limit enforcement
 *
 * GOVERNING RULE:
 * Kindling is a system of record, not a reasoning engine.
 * Queries may retrieve facts; interpretation is the caller's responsibility.
 *
 * @packageDocumentation
 */

// =============================================================================
// Observation Contract (Write-Only)
// =============================================================================

export {
  // Version
  OBSERVATION_CONTRACT_VERSION,

  // Schemas (individual)
  SessionStartObservationSchema,
  SessionEndObservationSchema,
  PlanCreatedObservationSchema,
  PlanEditedObservationSchema,
  PlanApprovedObservationSchema,
  PlanRejectedObservationSchema,
  ActionExecutedObservationSchema,
  GateEvaluatedObservationSchema,
  ConstraintAppliedObservationSchema,
  HumanInputObservationSchema,
  ErrorObservationSchema,
  CommandInvokedObservationSchema,
  ArgShapeSchema,
  FlagSetEntrySchema,

  // Schema (union)
  ObservationSchema,

  // Types
  type SessionStartObservation,
  type SessionEndObservation,
  type PlanCreatedObservation,
  type PlanEditedObservation,
  type PlanApprovedObservation,
  type PlanRejectedObservation,
  type ActionExecutedObservation,
  type GateEvaluatedObservation,
  type ConstraintAppliedObservation,
  type HumanInputObservation,
  type ErrorObservation,
  type CommandInvokedObservation,
  type ArgShape,
  type FlagSetEntry,
  type Observation,

  // Utilities
  validateObservation,
  containsSensitiveData,
} from './observation-contract.js';

// =============================================================================
// Query Contract (Read-Only)
// =============================================================================

export {
  // Version
  KINDLING_QUERY_CONTRACT_VERSION,

  // Query scopes
  QueryScopeSchema,
  type QueryScope,

  // Result shape
  ResultShapeSchema,
  type ResultShape,

  // Output format
  OutputFormatSchema,
  type OutputFormat,

  // Query requests (individual)
  SessionQuerySchema,
  PlanQuerySchema,
  GateQuerySchema,
  ActionQuerySchema,

  // Query request (union)
  QueryRequestSchema,
  QueryRequestBaseSchema,
  type QueryRequest,
  type QueryRequestBase,
  type SessionQuery,
  type PlanQuery,
  type GateQuery,
  type ActionQuery,

  // Query response
  QueryResponseSchema,
  QueryResponseMetadataSchema,
  ProvenanceLinkSchema,
  type QueryResponse,
  type QueryResponseMetadata,
  type ProvenanceLink,

  // Utilities
  validateQueryRequest,
  validateQueryResponse,
} from './query-contract.js';

// Re-export Observation from query-contract for convenience
export { ObservationSchema as QueryObservationSchema } from './query-contract.js';

// =============================================================================
// Configuration (KINDLING-002)
// =============================================================================

export {
  KindlingConfigSchema,
  CaptureConfigSchema,
  RetentionConfigSchema,
  QueryLimitConfigSchema,
  type KindlingConfig,
  type CaptureConfig,
  type RetentionConfig,
  type QueryLimitConfig,
  DEFAULT_KINDLING_CONFIG,
  loadKindlingConfig,
  shouldCapture,
} from './config.js';

// =============================================================================
// Service Layer (KINDLING-001)
// =============================================================================

export {
  type IKindlingStore,
  NoOpKindlingStore,
  KindlingService,
  ObservationValidationError,
  QueryValidationError,
  createKindlingService,
} from './kindling-service.js';

// =============================================================================
// Sensitive Data Validation (KINDLING-015)
// =============================================================================

export {
  validateNoSensitiveData,
  redactSensitiveFields,
  type SensitiveDataValidationResult,
} from './sensitive-data-validator.js';

// =============================================================================
// Emitters (KINDLING-003 through 008)
// =============================================================================

export {
  // Session
  emitSessionStart,
  emitSessionEnd,
  type SessionStartContext,
  type SessionEndOutcome,

  // Gate
  emitGateEvaluated,
  type GateResult,

  // Action
  emitActionExecuted,
  type ActionDetails,

  // Plan
  emitPlanCreated,
  emitPlanEdited,
  emitPlanApproved,
  emitPlanRejected,
  type PlanCreatedInput,
  type PlanEditedInput,
  type PlanApprovedInput,
  type PlanRejectedInput,

  // Human Input
  emitHumanInput,
  type HumanInputDetails,

  // Constraint
  emitConstraintApplied,
  type ConstraintDetails,

  // Error
  emitError,
  type ErrorDetails,
} from './emitters/index.js';

// =============================================================================
// Query Service (KINDLING-009)
// =============================================================================

export {
  KindlingQueryService,
  type QueryOptions,
  type SessionQueryOptions,
  type PlanQueryOptions,
  type ActionQueryOptions,
} from './query-service.js';

// =============================================================================
// Query Limits (KINDLING-010)
// =============================================================================

export { enforceQueryLimits, limitsFromConfig, type QueryLimits } from './query-limits.js';

// =============================================================================
// Retention (KINDLING-016)
// =============================================================================

export {
  type IRetentionCapableStore,
  type StorageStats,
  type PruneResult,
  isRetentionCapable,
  pruneOldObservations,
  getStorageStats,
} from './retention.js';

// =============================================================================
// Status Utility (KINDLING-014)
// =============================================================================

export {
  getKindlingStatus,
  formatKindlingStatus,
  type KindlingStatus,
  type KindlingStatusConfig,
  type KindlingStatusStore,
} from './status.js';

// =============================================================================
// Adapter (Anvil → Kindling Bridge)
// =============================================================================

export { AnvilKindlingAdapter, type AnvilKindlingAdapterConfig } from './adapter.js';
