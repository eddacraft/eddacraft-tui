/**
 * Anti-pattern detection types
 *
 * Defines the Warning schema and related types for anti-pattern detection
 * and architecture boundary violations. Follows Zod-first approach per ADR-0001.
 */

import { z } from 'zod';

// =============================================================================
// Location Schema
// =============================================================================

/**
 * Source code location for a warning
 */
export const LocationSchema = z.object({
  file: z.string().describe('File path relative to workspace root'),
  line: z.number().int().positive().describe('Line number (1-based)'),
  column: z.number().int().nonnegative().optional().describe('Column number (0-based)'),
  endLine: z.number().int().positive().optional().describe('End line for multi-line spans'),
  endColumn: z.number().int().nonnegative().optional().describe('End column for multi-line spans'),
});

export type Location = z.infer<typeof LocationSchema>;

// =============================================================================
// Drift Schema
// =============================================================================

/**
 * Drift context for boundary violations
 */
export const DriftSchema = z.object({
  isNew: z.boolean().describe('Whether this is a NEW violation vs existing'),
  existingCount: z
    .number()
    .int()
    .nonnegative()
    .optional()
    .describe('Number of existing violations of this type'),
  baselineId: z.string().optional().describe('ID of the baseline violation if existing'),
});

export type Drift = z.infer<typeof DriftSchema>;

// =============================================================================
// Suppression Schema
// =============================================================================

/**
 * Suppression metadata when a warning is suppressed via @anvil-ignore
 */
export const SuppressionSchema = z.object({
  reason: z.string().min(1).describe('Human-provided reason for suppression'),
  author: z.string().optional().describe('Author who added the suppression (e.g., @jane)'),
  timestamp: z.string().datetime().optional().describe('When the suppression was added'),
  scope: z.enum(['statement', 'import', 'file', 'line']).describe('Scope of the suppression'),
});

export type Suppression = z.infer<typeof SuppressionSchema>;

// =============================================================================
// Warning Schema (Core)
// =============================================================================

/**
 * Warning categories
 */
export const WarningCategorySchema = z.enum(['anti-pattern', 'boundary', 'architecture']);

export type WarningCategory = z.infer<typeof WarningCategorySchema>;

/**
 * Warning severity levels
 */
export const WarningSeveritySchema = z.enum(['error', 'warning', 'info']);

export type WarningSeverity = z.infer<typeof WarningSeveritySchema>;

/**
 * Confidence levels for detection
 */
export const ConfidenceSchema = z.enum(['high', 'medium', 'low']);

export type Confidence = z.infer<typeof ConfidenceSchema>;

/**
 * Core Warning schema - the primary output of anti-pattern and boundary detection
 */
export const WarningSchema = z.object({
  // Identification
  id: z
    .string()
    .regex(/^(AP|ARCH|BOUND)-\d{3}$/)
    .describe('Warning ID (e.g., AP-001, ARCH-001, BOUND-001)'),
  fingerprint: z
    .string()
    .optional()
    .describe('Unique fingerprint for deduplication (hash of location + pattern)'),

  // Classification
  category: WarningCategorySchema.describe('Warning category'),
  severity: WarningSeveritySchema.describe('Severity level'),
  confidence: ConfidenceSchema.describe('Detection confidence'),

  // Display
  title: z.string().describe('Short title (e.g., "Broad eslint-disable added")'),
  message: z.string().describe('Primary message - what happened'),
  explanation: z.string().describe('Why this matters'),
  suggestion: z.string().describe('What to do instead'),

  // Location
  location: LocationSchema.describe('Source code location'),

  // Context
  pattern: z.string().optional().describe('Named pattern or rule that triggered this'),
  drift: DriftSchema.optional().describe('Drift context for boundary violations'),

  // Suppression
  suppressed: SuppressionSchema.optional().describe('Suppression metadata if suppressed'),
});

export type Warning = z.infer<typeof WarningSchema>;

// =============================================================================
// Anti-Pattern Definition Schema
// =============================================================================

/**
 * Regex-based detection configuration
 */
export const RegexDetectionConfigSchema = z.object({
  type: z.literal('regex').describe('Regex-based detection'),
  pattern: z.string().min(1).describe('Regex pattern string'),
});

/**
 * AST-based detection configuration
 */
export const AstDetectionConfigSchema = z.object({
  type: z.literal('ast').describe('AST-based detection'),
  astQuery: z.string().min(1).describe('AST query description'),
});

/**
 * Detection method configuration - discriminated union ensures
 * regex type requires pattern, ast type requires astQuery
 */
export const DetectionConfigSchema = z.discriminatedUnion('type', [
  RegexDetectionConfigSchema,
  AstDetectionConfigSchema,
]);

export type DetectionConfig = z.infer<typeof DetectionConfigSchema>;
export type RegexDetectionConfig = z.infer<typeof RegexDetectionConfigSchema>;
export type AstDetectionConfig = z.infer<typeof AstDetectionConfigSchema>;

/**
 * Anti-pattern definition - defines a detectable pattern
 */
export const AntiPatternSchema = z.object({
  // Identification
  id: z
    .string()
    .regex(/^AP-\d{3}$/)
    .describe('Pattern ID (e.g., AP-001)'),
  name: z.string().describe('Human-readable name'),

  // Classification
  category: z
    .enum(['escape-hatch', 'error-handling', 'code-quality', 'type-safety', 'html', 'css'])
    .describe('Pattern category'),
  severity: WarningSeveritySchema.describe('Default severity'),
  confidence: ConfidenceSchema.describe('Detection confidence'),

  // Detection
  detection: DetectionConfigSchema.describe('How to detect this pattern'),

  // Messaging
  title: z.string().describe('Warning title template'),
  explanation: z.string().describe('Why this pattern is problematic'),
  suggestion: z.string().describe('Recommended alternative'),

  // File targeting
  fileExtensions: z
    .array(z.string())
    .optional()
    .describe('File extensions this pattern applies to (e.g., [".html", ".htm"])'),

  // Customisation
  allowlist: z.array(z.string()).optional().describe('File patterns to skip (glob patterns)'),
  threshold: z
    .number()
    .int()
    .positive()
    .optional()
    .describe('Count threshold for triggering (e.g., >3 per file)'),

  // Enablement
  enabled: z.boolean().default(true).describe('Whether enabled by default'),
  optIn: z.boolean().default(false).describe('If true, disabled by default (noisy patterns)'),

  // Documentation
  documentation: z.string().url().optional().describe('Link to detailed documentation'),
});

export type AntiPattern = z.infer<typeof AntiPatternSchema>;

// =============================================================================
// Warning Result Schema (for GateResult integration)
// =============================================================================

/**
 * Collection of warnings from a check run - embeds in GateResult.details
 */
export const WarningResultSchema = z.object({
  warnings: z.array(WarningSchema).describe('All warnings found'),
  summary: z.object({
    total: z.number().int().nonnegative(),
    errors: z.number().int().nonnegative(),
    warnings: z.number().int().nonnegative(),
    info: z.number().int().nonnegative(),
    suppressed: z.number().int().nonnegative(),
  }),
  patterns_checked: z.array(z.string()).describe('Pattern IDs that were checked'),
});

export type WarningResult = z.infer<typeof WarningResultSchema>;

// =============================================================================
// Suppression Record Schema (for provenance)
// =============================================================================

/**
 * Suppression record for provenance tracking
 */
export const SuppressionRecordSchema = z.object({
  id: z.string().describe('Unique suppression ID'),
  pattern_id: z.string().describe('Pattern ID being suppressed'),
  file: z.string().describe('File containing the suppression'),
  line: z.number().int().positive().describe('Line number of suppression comment'),
  reason: z.string().describe('Reason provided'),
  author: z.string().optional().describe('Author if specified'),
  timestamp: z.string().datetime().describe('When recorded'),
  commit: z.string().optional().describe('Git commit SHA when added'),
  scope: z.enum(['statement', 'import', 'file', 'line']).describe('Suppression scope'),
  expires_at: z.string().datetime().optional().describe('Expiry date for time-boxed suppressions'),
});

export type SuppressionRecord = z.infer<typeof SuppressionRecordSchema>;

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Create a warning fingerprint for deduplication
 */
export function createWarningFingerprint(warning: Omit<Warning, 'fingerprint'>): string {
  const parts = [
    warning.id,
    warning.location.file,
    warning.location.line.toString(),
    warning.pattern ?? '',
  ];
  // Simple hash - in production would use crypto
  return parts.join(':');
}

/**
 * Check if a warning should be considered an error for gate purposes
 */
export function isBlockingWarning(warning: Warning): boolean {
  return warning.severity === 'error' && !warning.suppressed;
}

/**
 * Warning summary type matching WarningResultSchema.summary
 */
export interface WarningSummary {
  total: number;
  errors: number;
  warnings: number;
  info: number;
  suppressed: number;
}

/**
 * Count warnings by severity, including total count.
 * Note: A warning can be both suppressed AND have a severity,
 * so suppressed warnings are counted separately and not in their severity bucket.
 */
export function countBySeverity(warnings: Warning[]): WarningSummary {
  const counts = warnings.reduce(
    (acc, w) => {
      if (w.suppressed) {
        acc.suppressed++;
      } else if (w.severity === 'error') {
        acc.errors++;
      } else if (w.severity === 'warning') {
        acc.warnings++;
      } else {
        acc.info++;
      }
      return acc;
    },
    { errors: 0, warnings: 0, info: 0, suppressed: 0 }
  );

  return {
    total: warnings.length,
    ...counts,
  };
}

/**
 * Create a complete WarningResult summary from a list of warnings.
 * Ensures consistency between warnings array and summary counts.
 *
 * @param warnings - Array of warnings
 * @param patternsChecked - Array of pattern IDs that were checked
 * @returns Complete WarningResult object
 */
export function createWarningResult(warnings: Warning[], patternsChecked: string[]): WarningResult {
  return {
    warnings,
    summary: countBySeverity(warnings),
    patterns_checked: patternsChecked,
  };
}

/**
 * Validate that a WarningResult's summary matches its warnings array.
 * Useful for catching inconsistencies in manually constructed results.
 *
 * @param result - WarningResult to validate
 * @returns true if summary matches warnings, false otherwise
 */
export function validateWarningResultConsistency(result: WarningResult): boolean {
  const computed = countBySeverity(result.warnings);
  return (
    result.summary.total === computed.total &&
    result.summary.errors === computed.errors &&
    result.summary.warnings === computed.warnings &&
    result.summary.info === computed.info &&
    result.summary.suppressed === computed.suppressed
  );
}
