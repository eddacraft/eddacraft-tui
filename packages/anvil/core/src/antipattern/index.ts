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

// HTML patterns
export { HTML_PATTERNS } from './patterns-html.js';

// CSS patterns
export { CSS_PATTERNS } from './patterns-css.js';

// Scanner
export { type ScanOptions, type ScanResult, scanFile, scanFiles } from './scanner.js';

// .anvil file format (Phase 1: source tree → compiled pattern registry)
export * from './format/index.js';
