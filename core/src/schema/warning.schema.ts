/**
 * Warning schema exports for planless checks.
 *
 * Kept here to match APS module expectations while delegating to the
 * anti-pattern schema definitions.
 */

export {
  WarningSchema,
  type Warning,
  WarningResultSchema,
  type WarningResult,
  WarningCategorySchema,
  type WarningCategory,
  WarningSeveritySchema,
  type WarningSeverity,
  ConfidenceSchema,
  type Confidence,
  LocationSchema,
  type Location,
  DriftSchema,
  type Drift,
  SuppressionSchema,
  type Suppression,
  createWarningFingerprint,
  createWarningResult,
  countBySeverity,
  isBlockingWarning,
  validateWarningResultConsistency,
} from '../antipattern/types.js';
