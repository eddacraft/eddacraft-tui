/**
 * Kindling Integration Contracts (v1)
 *
 * This package defines the mechanical contract between Anvil and Kindling.
 * Two surfaces:
 *
 * 1. Observation Contract (Write-Only)
 *    - What Anvil must emit to be "Kindling-complete"
 *    - 11 observation kinds covering session, plan, gate, action, constraint, human, error
 *    - Immutable, timestamped, linked facts
 *
 * 2. Query Contract (Read-Only)
 *    - How to retrieve observations (bounded, explicit, no inference)
 *    - 4 query scopes: session, plan, gate, action
 *    - Mandatory constraints, throttling, output guarantees
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

// =============================================================================
// Re-export Observation from query-contract for convenience
// =============================================================================

export { ObservationSchema as QueryObservationSchema } from './query-contract.js';
