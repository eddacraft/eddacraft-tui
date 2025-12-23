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
  RegexDetectionConfigSchema,
  type RegexDetectionConfig,
  AstDetectionConfigSchema,
  type AstDetectionConfig,
  DetectionConfigSchema,
  type DetectionConfig,
  AntiPatternSchema,
  type AntiPattern,
  // Results
  WarningResultSchema,
  type WarningResult,
  type WarningSummary,
  // Suppression records
  SuppressionRecordSchema,
  type SuppressionRecord,
  // Utilities
  createWarningFingerprint,
  isBlockingWarning,
  countBySeverity,
  createWarningResult,
  validateWarningResultConsistency,
} from './types.js';

// Pattern catalogue
export {
  PATTERNS,
  type PatternCategory,
  getPattern,
  getPatternsByCategory,
  getEnabledPatterns,
  getDefaultPatterns,
  getPatternIds,
  isValidPatternId,
} from './patterns.js';
