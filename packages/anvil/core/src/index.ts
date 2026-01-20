/**
 * @eddacraft/anvil-core
 *
 * Pure domain logic for the Anvil system.
 * Contains antipattern detection, architecture analysis, drift detection,
 * suppression management, validation, and other core functionality.
 *
 * This package has NO I/O operations - all I/O is handled by @eddacraft/anvil-runtime.
 *
 * @module @eddacraft/anvil-core
 */

// Antipattern detection
export * from './antipattern/index.js';

// Suppression management
export * from './suppression/index.js';

// Architecture analysis
export * from './architecture/index.js';

// Drift detection
export * from './drift/index.js';

// Provenance tracking
export * from './provenance/index.js';

// Warning utilities
export * from './warnings/index.js';

// Explain functionality
export * from './explain/index.js';

// Validation
export * from './validation/index.js';

// Crypto utilities
export * from './crypto/index.js';

// General utilities
export * from './utils/index.js';

// Re-export from @eddacraft/anvil-contracts for backward compatibility
// (only types - no circular dependency)
export {
  APSPlanSchema,
  APS_SCHEMA_VERSION,
  ChangeTypeSchema,
  ChangeSchema,
  ProvenanceSchema,
  ValidationSchema,
  EvidenceEntrySchema,
  EvidenceSchema,
  ApprovalSchema,
  ExecutionResultSchema,
  validatePlan,
  createPlan,
} from '@eddacraft/anvil-contracts';

export type {
  APSPlan,
  Change,
  ChangeType,
  Provenance,
  Validation,
  EvidenceEntry,
  Evidence,
  Approval,
  ExecutionResult,
  SchemaValidationResult,
  GateConfig,
  GateCheck,
  GateResult,
  GateRunResult,
  CheckContext,
  PlanData,
  WatchConfig,
  PolicyConfig,
  StackConfig,
  ArchitectureContextBase,
} from '@eddacraft/anvil-contracts';
