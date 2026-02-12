/**
 * Contracts — schemas, types, and events.
 * Formerly @eddacraft/anvil-contracts, now co-located inside @eddacraft/anvil-core.
 *
 * Warning-related types (Warning, WarningResult, Location, etc.) are NOT
 * re-exported here because they are already provided by the antipattern module
 * with additional scanner-specific extensions.
 */

// APS schemas & helpers
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
} from './schemas/aps.schema.js';

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
} from './schemas/aps.schema.js';

// JSON Schema generation
export { generateJSONSchema } from './schemas/json-schema.js';

// Gate types (pure interfaces, no overlap with antipattern)
export type {
  GateConfig,
  GateCheck,
  GateResult,
  GateResultDetails,
  GateRunResult,
  CheckContext,
  PlanData,
  WatchConfig,
  PolicyConfig,
  PolicyBundleConfig,
  PolicyVerificationConfig,
  SignatureAlgorithm,
  StackConfig,
  StackLayerConfig,
  StackValidationConfig,
  ArchitectureContextBase,
  NormaliseFilesOptions,
} from './types/gate.types.js';

export { getWarningsFromResult, hasBlockingWarnings } from './types/gate.types.js';
