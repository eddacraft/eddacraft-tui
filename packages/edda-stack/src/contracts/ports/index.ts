/**
 * Port Interfaces (STACK-007, STACK-009)
 *
 * Defines the interfaces for storage adapters across the Edda Stack.
 *
 * @module @anvil/edda-stack/contracts/ports
 */

// =============================================================================
// Kindling Port
// =============================================================================

export type {
  // Core types
  ObservationKind,
  Observation,
  CreateObservationInput,
  ObservationQuery,
  ObservationQueryResult,
  // Session query types (STACK-007)
  SessionQueryOptions,
  SessionQueryResult,
  // Plan query types (STACK-007)
  PlanQueryOptions,
  SessionSummary,
  PlanQueryResult,
  // Interface
  IKindlingPort,
} from './kindling.port.js';

// =============================================================================
// Ember Port
// =============================================================================

export type {
  // Input types
  UpdateProposalInput,
  ResolveProposalInput,
  // Statistics types (STACK-007)
  ProposalTypeStats,
  ProposalStatusStats,
  EmberStats,
  // Interface
  IEmberPort,
} from './ember.port.js';

// =============================================================================
// Edda Port
// =============================================================================

export type {
  // Input types
  CreateMemoryInput,
  UpdateMemoryInput,
  RetireMemoryInput,
  // Provenance resolution types (STACK-007)
  ProvenanceResolutionResult,
  // Statistics types (STACK-007)
  MemoryTypeStats,
  MemoryStatusStats,
  ConfidenceLevelStats,
  EddaStats,
  // Interface
  IEddaPort,
} from './edda.port.js';
