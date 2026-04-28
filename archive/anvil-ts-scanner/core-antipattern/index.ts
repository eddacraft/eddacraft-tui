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
  getPatternsByFamily,
  getEnabledPatterns,
  getDefaultPatterns,
  getPatternIds,
  isValidPatternId,
  reloadPatterns,
} from './patterns.js';

// Compiled `.anvil` registry loader — backs the pattern catalogue in Phase 2.
export {
  loadCompiledRegistry,
  loadRegistryPatterns,
  compiledToAntiPattern,
  resetRegistryCache,
  type LoadRegistryOptions,
  type LoadRegistryResult,
} from './registry-loader.js';

// Scanner
export {
  type ArtifactKind,
  type Artifact,
  type ScanOptions,
  type ScanResult,
  scanArtifact,
  scanArtifacts,
  scanFile,
  scanFiles,
} from './scanner.js';

// .anvil file format (Phase 1: source tree → compiled pattern registry)
export * from './format/index.js';
