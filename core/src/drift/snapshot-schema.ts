/**
 * Drift snapshot schema
 *
 * Defines the structure for point-in-time drift snapshots used to track
 * architecture evolution over time. Follows Zod-first approach per ADR-0001.
 */

import { z } from 'zod';

// =============================================================================
// Snapshot Version
// =============================================================================

export const SNAPSHOT_SCHEMA_VERSION = '1.0.0';

// =============================================================================
// Violation Record Schema
// =============================================================================

/**
 * A recorded boundary violation in the snapshot
 */
export const SnapshotViolationSchema = z.object({
  id: z.string().describe('Unique violation ID (hash of from+to+line)'),
  type: z.enum(['boundary', 'architecture']).describe('Violation type'),
  from_file: z.string().describe('File containing the import'),
  to_file: z.string().describe('File being imported'),
  from_layer: z.string().nullable().describe('Source layer'),
  to_layer: z.string().nullable().describe('Target layer'),
  line: z.number().int().positive().describe('Line number of import'),
  rule: z.string().optional().describe('Rule name that was violated'),
  message: z.string().optional().describe('Human-readable violation message'),
});

export type SnapshotViolation = z.infer<typeof SnapshotViolationSchema>;

// =============================================================================
// Anti-pattern Record Schema
// =============================================================================

/**
 * A recorded anti-pattern occurrence in the snapshot
 */
export const SnapshotAntiPatternSchema = z.object({
  id: z.string().describe('Pattern ID (e.g., AP-003)'),
  file: z.string().describe('File containing the pattern'),
  line: z.number().int().positive().describe('Line number'),
  pattern: z.string().describe('Pattern name'),
  severity: z.enum(['error', 'warning', 'info']).describe('Severity level'),
});

export type SnapshotAntiPattern = z.infer<typeof SnapshotAntiPatternSchema>;

// =============================================================================
// Suppression Record Schema
// =============================================================================

/**
 * A recorded suppression in the snapshot
 */
export const SnapshotSuppressionSchema = z.object({
  id: z.string().describe('Unique suppression ID'),
  pattern_id: z.string().describe('Pattern ID being suppressed'),
  file: z.string().describe('File containing the suppression'),
  line: z.number().int().positive().describe('Line number of suppression'),
  reason: z.string().describe('Reason provided for suppression'),
  scope: z.enum(['statement', 'import', 'file', 'line']).describe('Suppression scope'),
  expires_at: z.string().datetime().optional().describe('Expiry date if time-boxed'),
  is_expired: z.boolean().optional().describe('Whether suppression has expired'),
});

export type SnapshotSuppression = z.infer<typeof SnapshotSuppressionSchema>;

// =============================================================================
// Metrics Schema
// =============================================================================

/**
 * Aggregate metrics for quick comparison
 */
export const SnapshotMetricsSchema = z.object({
  boundary_violations: z.number().int().nonnegative().describe('Total boundary violations'),
  antipattern_count: z.number().int().nonnegative().describe('Total anti-pattern occurrences'),
  suppression_count: z.number().int().nonnegative().describe('Total active suppressions'),
  expired_suppressions: z.number().int().nonnegative().describe('Expired suppressions'),
  files_analysed: z.number().int().nonnegative().describe('Number of files analysed'),
});

export type SnapshotMetrics = z.infer<typeof SnapshotMetricsSchema>;

// =============================================================================
// Anti-pattern Breakdown Schema
// =============================================================================

/**
 * Breakdown of anti-patterns by type
 */
export const AntiPatternBreakdownSchema = z
  .record(z.string(), z.number().int().nonnegative())
  .describe('Count of each anti-pattern type (e.g., { "AP-003": 5, "AP-004": 2 })');

export type AntiPatternBreakdown = z.infer<typeof AntiPatternBreakdownSchema>;

// =============================================================================
// Hotspot Schema
// =============================================================================

/**
 * A file or directory with high violation concentration
 */
export const HotspotSchema = z.object({
  path: z.string().describe('File or directory path'),
  violation_count: z.number().int().positive().describe('Number of violations'),
  types: z.array(z.string()).describe('Types of violations found'),
});

export type Hotspot = z.infer<typeof HotspotSchema>;

// =============================================================================
// Drift Snapshot Schema (Full)
// =============================================================================

/**
 * Complete drift snapshot for point-in-time state capture
 */
export const DriftSnapshotSchema = z.object({
  // Metadata
  schema_version: z.string().describe('Snapshot schema version'),
  created_at: z.string().datetime().describe('When snapshot was created'),
  name: z.string().optional().describe('Optional snapshot name (e.g., "release-1.0")'),

  // Aggregate metrics
  metrics: SnapshotMetricsSchema.describe('Aggregate counts for quick comparison'),

  // Anti-pattern breakdown
  antipattern_breakdown: AntiPatternBreakdownSchema.optional().describe(
    'Count by anti-pattern type'
  ),

  // Hotspots
  hotspots: z.array(HotspotSchema).optional().describe('Files/directories with most violations'),

  // Detailed records
  violations: z.array(SnapshotViolationSchema).describe('All boundary violations'),
  antipatterns: z.array(SnapshotAntiPatternSchema).describe('All anti-pattern occurrences'),
  suppressions: z.array(SnapshotSuppressionSchema).describe('All suppressions'),

  // Baseline reference
  baseline_hash: z.string().optional().describe('Hash of architecture baseline used'),

  // Git context
  git_ref: z.string().optional().describe('Git commit SHA or branch at snapshot time'),
});

export type DriftSnapshot = z.infer<typeof DriftSnapshotSchema>;

// =============================================================================
// Snapshot File Metadata
// =============================================================================

/**
 * Metadata for listing snapshots
 */
export const SnapshotMetadataSchema = z.object({
  filename: z.string().describe('Snapshot filename'),
  name: z.string().optional().describe('Optional snapshot name'),
  created_at: z.string().datetime().describe('When snapshot was created'),
  metrics: SnapshotMetricsSchema.describe('Aggregate metrics'),
});

export type SnapshotMetadata = z.infer<typeof SnapshotMetadataSchema>;

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Generate a snapshot filename from timestamp
 */
export function generateSnapshotFilename(date: Date = new Date()): string {
  const timestamp = date.toISOString().replace(/[:.]/g, '-').replace('Z', '');
  return `snapshot-${timestamp}.json`;
}

/**
 * Generate a snapshot filename from a custom name
 */
export function generateNamedSnapshotFilename(name: string): string {
  const safeName = name.replace(/[^a-zA-Z0-9-_]/g, '-').toLowerCase();
  return `snapshot-${safeName}.json`;
}

/**
 * Parse snapshot name from filename
 */
export function parseSnapshotFilename(filename: string): {
  isNamed: boolean;
  nameOrTimestamp: string;
} {
  const match = filename.match(/^snapshot-(.+)\.json$/);
  if (!match) {
    throw new Error(`Invalid snapshot filename: ${filename}`);
  }

  const value = match[1];
  // Check if it looks like a timestamp (contains T and multiple dashes)
  const isTimestamp = value.includes('T') && (value.match(/-/g) || []).length > 3;

  return {
    isNamed: !isTimestamp,
    nameOrTimestamp: value,
  };
}

/**
 * Create an empty snapshot with default values
 */
export function createEmptySnapshot(options?: {
  name?: string;
  baselineHash?: string;
  gitRef?: string;
}): DriftSnapshot {
  return {
    schema_version: SNAPSHOT_SCHEMA_VERSION,
    created_at: new Date().toISOString(),
    name: options?.name,
    metrics: {
      boundary_violations: 0,
      antipattern_count: 0,
      suppression_count: 0,
      expired_suppressions: 0,
      files_analysed: 0,
    },
    violations: [],
    antipatterns: [],
    suppressions: [],
    baseline_hash: options?.baselineHash,
    git_ref: options?.gitRef,
  };
}

/**
 * Validate a snapshot against the schema
 */
export function validateSnapshot(data: unknown): {
  success: boolean;
  data?: DriftSnapshot;
  error?: string;
} {
  const result = DriftSnapshotSchema.safeParse(data);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, error: result.error.format()._errors.join(', ') };
}
