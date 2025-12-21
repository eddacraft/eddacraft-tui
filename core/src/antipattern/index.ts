/**
 * Anti-pattern detection module
 *
 * Provides detection of code anti-patterns and architecture boundary violations.
 */

// Types and schemas
export {
  // Location
  LocationSchema,
  type Location,
  // Drift
  DriftSchema,
  type Drift,
  // Suppression
  SuppressionSchema,
  type Suppression,
  // Warning
  WarningCategorySchema,
  type WarningCategory,
  WarningSeveritySchema,
  type WarningSeverity,
  ConfidenceSchema,
  type Confidence,
  WarningSchema,
  type Warning,
  // Anti-pattern definition
  DetectionConfigSchema,
  type DetectionConfig,
  AntiPatternSchema,
  type AntiPattern,
  // Results
  WarningResultSchema,
  type WarningResult,
  // Suppression records
  SuppressionRecordSchema,
  type SuppressionRecord,
  // Utilities
  createWarningFingerprint,
  isBlockingWarning,
  countBySeverity,
} from './types.js';
