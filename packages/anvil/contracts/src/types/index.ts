/**
 * Type exports for @eddacraft/anvil-contracts
 *
 * Re-exports types from schemas for convenience.
 * All types are inferred from Zod schemas.
 */

// Re-export all types from schemas
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
  Warning,
  WarningResult,
  WarningCategory,
  WarningSeverity,
  Confidence,
  Location,
  Drift,
  Suppression,
  WarningSummary,
  FlagClass,
  FlagStatus,
  FlagValueType,
  FlagVariant,
  FeatureFlagDefinition,
  FeatureFlagManifest,
  EnvironmentName,
  Channel,
  EnvironmentContext,
  AudienceContext,
  EvaluationContext,
  TargetingOperator,
  TargetingCondition,
  TargetingRule,
} from '../schemas/index.js';

// Re-export gate types
export * from './gate.types.js';
