/**
 * APS Schema Module
 *
 * This module exports the Anvil Plan Specification (APS) schema definitions,
 * TypeScript types, validation utilities, and JSON Schema generation.
 */

// Re-export all Zod schemas
export {
  APSPlanSchema,
  ChangeSchema,
  ChangeTypeSchema,
  ProvenanceSchema,
  ValidationSchema,
  EvidenceSchema,
  EvidenceEntrySchema,
  ApprovalSchema,
  ExecutionResultSchema,
} from './aps.schema.js';

// Re-export all TypeScript types
export type {
  APSPlan,
  Change,
  ChangeType,
  Provenance,
  Validation,
  Evidence,
  EvidenceEntry,
  Approval,
  ExecutionResult,
  SchemaValidationResult,
} from './aps.schema.js';

// Re-export utility functions
export { validatePlan, createPlan, APS_SCHEMA_VERSION } from './aps.schema.js';

// Warning schema (planless checks)
export * from './warning.schema.js';

// Export JSON Schema generation utilities
export { generateJSONSchema } from './json-schema.js';
