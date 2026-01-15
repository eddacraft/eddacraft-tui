/**
 * Warning types for @anvil/contracts
 *
 * Minimal type definitions for warnings used across packages.
 * Full Zod schemas and utilities are in @anvil/core.
 */

import { z } from 'zod';

// =============================================================================
// Location Schema
// =============================================================================

export const LocationSchema = z.object({
  file: z.string(),
  line: z.number().int().positive(),
  column: z.number().int().nonnegative().optional(),
  endLine: z.number().int().positive().optional(),
  endColumn: z.number().int().nonnegative().optional(),
});

export type Location = z.infer<typeof LocationSchema>;

// =============================================================================
// Drift Schema
// =============================================================================

export const DriftSchema = z.object({
  isNew: z.boolean(),
  existingCount: z.number().int().nonnegative().optional(),
  baselineId: z.string().optional(),
});

export type Drift = z.infer<typeof DriftSchema>;

// =============================================================================
// Suppression Schema
// =============================================================================

export const SuppressionSchema = z.object({
  reason: z.string().min(1),
  author: z.string().optional(),
  timestamp: z.string().datetime().optional(),
  scope: z.enum(['statement', 'import', 'file', 'line']),
});

export type Suppression = z.infer<typeof SuppressionSchema>;

// =============================================================================
// Warning Schema
// =============================================================================

export const WarningCategorySchema = z.enum(['anti-pattern', 'boundary', 'architecture']);
export type WarningCategory = z.infer<typeof WarningCategorySchema>;

export const WarningSeveritySchema = z.enum(['error', 'warning', 'info']);
export type WarningSeverity = z.infer<typeof WarningSeveritySchema>;

export const ConfidenceSchema = z.enum(['high', 'medium', 'low']);
export type Confidence = z.infer<typeof ConfidenceSchema>;

export const WarningSchema = z.object({
  id: z.string().regex(/^(AP|ARCH|BOUND)-\d{3}$/),
  fingerprint: z.string().optional(),
  category: WarningCategorySchema,
  severity: WarningSeveritySchema,
  confidence: ConfidenceSchema,
  title: z.string(),
  message: z.string(),
  explanation: z.string(),
  suggestion: z.string(),
  location: LocationSchema,
  pattern: z.string().optional(),
  drift: DriftSchema.optional(),
  suppressed: SuppressionSchema.optional(),
});

export type Warning = z.infer<typeof WarningSchema>;

// =============================================================================
// Warning Result Schema
// =============================================================================

export const WarningResultSchema = z.object({
  warnings: z.array(WarningSchema),
  summary: z.object({
    total: z.number().int().nonnegative(),
    byCategory: z.record(z.string(), z.number().int().nonnegative()),
    bySeverity: z.record(z.string(), z.number().int().nonnegative()),
    suppressed: z.number().int().nonnegative(),
    blocking: z.number().int().nonnegative(),
  }),
});

export type WarningResult = z.infer<typeof WarningResultSchema>;

export interface WarningSummary {
  total: number;
  byCategory: Record<string, number>;
  bySeverity: Record<string, number>;
  suppressed: number;
  blocking: number;
}

// =============================================================================
// Helper Functions
// =============================================================================

export function createWarningFingerprint(warning: Warning): string {
  const parts = [warning.id, warning.location.file, warning.location.line.toString()];
  if (warning.pattern) {
    parts.push(warning.pattern);
  }
  return parts.join(':');
}

export function isBlockingWarning(warning: Warning): boolean {
  return warning.severity === 'error' && !warning.suppressed;
}

export function countBySeverity(warnings: Warning[]): Record<string, number> {
  const counts: Record<string, number> = { error: 0, warning: 0, info: 0 };
  for (const w of warnings) {
    counts[w.severity] = (counts[w.severity] || 0) + 1;
  }
  return counts;
}

export function createWarningResult(warnings: Warning[]): WarningResult {
  const byCategory: Record<string, number> = {};
  const bySeverity = countBySeverity(warnings);
  let suppressed = 0;
  let blocking = 0;

  for (const w of warnings) {
    byCategory[w.category] = (byCategory[w.category] || 0) + 1;
    if (w.suppressed) suppressed++;
    if (isBlockingWarning(w)) blocking++;
  }

  return {
    warnings,
    summary: {
      total: warnings.length,
      byCategory,
      bySeverity,
      suppressed,
      blocking,
    },
  };
}

export function validateWarningResultConsistency(result: WarningResult): boolean {
  const actualTotal = result.warnings.length;
  if (result.summary.total !== actualTotal) return false;

  const actualBySeverity = countBySeverity(result.warnings);
  for (const [severity, count] of Object.entries(result.summary.bySeverity)) {
    if (actualBySeverity[severity] !== count) return false;
  }

  return true;
}
