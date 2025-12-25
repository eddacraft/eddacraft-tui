/**
 * Architecture baseline types
 *
 * Defines the schema for .anvil/architecture.json - the architecture baseline
 * that enables NEW vs existing violation detection.
 */

import { z } from 'zod';

// =============================================================================
// Entry Point Schema
// =============================================================================

/**
 * Entry point types detected in the codebase
 */
export const EntryPointTypeSchema = z.enum([
  'package', // Package entry (index.ts, main export)
  'application', // Application entry (main.ts, app.ts)
  'http', // HTTP handlers (routes, controllers)
  'api', // API handlers
  'cli', // CLI commands
  'worker', // Background workers/jobs
  'test', // Test entry points
  'unknown', // Could not determine type
]);

export type EntryPointType = z.infer<typeof EntryPointTypeSchema>;

/**
 * Confidence level for detection
 */
export const DetectionConfidenceSchema = z.enum(['high', 'medium', 'low']);

export type DetectionConfidence = z.infer<typeof DetectionConfidenceSchema>;

/**
 * A detected entry point in the codebase
 */
export const EntryPointSchema = z.object({
  path: z.string().describe('File path relative to workspace root'),
  type: EntryPointTypeSchema.describe('Type of entry point'),
  confidence: DetectionConfidenceSchema.describe('Detection confidence'),
  exports: z.array(z.string()).optional().describe('Named exports if applicable'),
});

export type EntryPoint = z.infer<typeof EntryPointSchema>;

// =============================================================================
// Layer Schema
// =============================================================================

/**
 * Standard layer names (can be extended)
 */
export const StandardLayerSchema = z.enum([
  'presentation',
  'application',
  'domain',
  'infrastructure',
  'shared',
]);

export type StandardLayer = z.infer<typeof StandardLayerSchema>;

/**
 * Layer definition with dependency rules
 */
export const LayerSchema = z.object({
  patterns: z.array(z.string()).describe('Glob patterns matching files in this layer'),
  depends_on: z.array(z.string()).describe('Layers this layer is allowed to depend on'),
  description: z.string().optional().describe('Human-readable description'),
});

export type Layer = z.infer<typeof LayerSchema>;

/**
 * Map of layer name to layer definition
 */
export const LayersSchema = z.record(z.string(), LayerSchema);

export type Layers = z.infer<typeof LayersSchema>;

// =============================================================================
// Boundary Schema
// =============================================================================

/**
 * Explicit boundary rule
 */
export const BoundarySchema = z.object({
  name: z.string().describe('Unique boundary name'),
  from: z.string().describe('Source layer'),
  to: z.string().describe('Target layer'),
  severity: z.enum(['error', 'warning', 'info']).describe('Violation severity'),
  message: z.string().describe('Human-readable message when violated'),
  confidence: DetectionConfidenceSchema.optional().describe(
    'Inference confidence (for auto-detected boundaries)'
  ),
});

export type Boundary = z.infer<typeof BoundarySchema>;

// =============================================================================
// Violation Schema (for baseline snapshot)
// =============================================================================

/**
 * A recorded violation in the baseline
 */
export const BaselineViolationSchema = z.object({
  id: z.string().describe('Unique violation ID (hash of from+to+line)'),
  from_layer: z.string().describe('Source layer'),
  to_layer: z.string().describe('Target layer'),
  from_file: z.string().describe('File containing the import'),
  to_file: z.string().describe('File being imported'),
  import_line: z.number().int().positive().describe('Line number of import'),
  rule: z.string().optional().describe('Rule name that was violated (for matching new violations)'),
});

export type BaselineViolation = z.infer<typeof BaselineViolationSchema>;

// =============================================================================
// Baseline Snapshot Schema
// =============================================================================

/**
 * Snapshot of the architecture state at baseline time
 */
export const BaselineSnapshotSchema = z.object({
  module_count: z.number().int().nonnegative().describe('Total modules analysed'),
  timestamp: z.string().datetime().describe('When baseline was created'),
  violations: z.array(BaselineViolationSchema).describe('Existing violations at baseline time'),
});

export type BaselineSnapshot = z.infer<typeof BaselineSnapshotSchema>;

// =============================================================================
// Architecture Baseline Schema (Full)
// =============================================================================

/**
 * Complete architecture baseline stored in .anvil/architecture.json
 */
export const ArchitectureBaselineSchema = z.object({
  // Metadata
  schema_version: z.literal('0.1.0').describe('Schema version'),
  created_at: z.string().datetime().describe('When baseline was created'),
  updated_at: z.string().datetime().describe('When baseline was last updated'),

  // Entry points
  entry_points: z.array(EntryPointSchema).describe('Detected entry points'),

  // Layer structure
  layers: LayersSchema.describe('Layer definitions with dependency rules'),

  // Explicit boundaries
  boundaries: z.array(BoundarySchema).describe('Explicit boundary rules'),

  // Baseline snapshot for NEW vs existing detection
  baseline_snapshot: BaselineSnapshotSchema.describe('Snapshot of violations at baseline time'),
});

export type ArchitectureBaseline = z.infer<typeof ArchitectureBaselineSchema>;

// =============================================================================
// Layer Detection Result
// =============================================================================

/**
 * Result of layer detection for a file
 */
export const LayerAssignmentSchema = z.object({
  file: z.string().describe('File path'),
  layer: z.string().nullable().describe('Assigned layer (null if unassigned)'),
  confidence: DetectionConfidenceSchema.describe('Assignment confidence'),
  matched_pattern: z.string().optional().describe('Pattern that matched'),
});

export type LayerAssignment = z.infer<typeof LayerAssignmentSchema>;

// =============================================================================
// Dependency Edge
// =============================================================================

/**
 * A dependency edge between two files
 */
export const DependencyEdgeSchema = z.object({
  from: z.string().describe('Source file'),
  to: z.string().describe('Target file'),
  from_layer: z.string().nullable().describe('Source layer'),
  to_layer: z.string().nullable().describe('Target layer'),
  line: z.number().int().positive().describe('Import line number'),
  type: z.enum(['import', 'require', 'dynamic']).describe('Import type'),
});

export type DependencyEdge = z.infer<typeof DependencyEdgeSchema>;

// =============================================================================
// Boundary Violation
// =============================================================================

/**
 * A detected boundary violation
 */
export const BoundaryViolationSchema = z.object({
  edge: DependencyEdgeSchema.describe('The violating edge'),
  boundary: BoundarySchema.optional().describe('Explicit boundary violated'),
  is_new: z.boolean().describe('Whether this is a NEW violation'),
  baseline_id: z.string().optional().describe('ID in baseline if existing violation'),
});

export type BoundaryViolation = z.infer<typeof BoundaryViolationSchema>;

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Create a violation ID from edge details
 */
export function createViolationId(fromFile: string, toFile: string, line: number): string {
  // Simple deterministic ID - in production would use proper hash
  return `${fromFile}:${toFile}:${line}`.replace(/[^a-zA-Z0-9:]/g, '_');
}

/**
 * Check if a violation exists in the baseline
 */
export function isExistingViolation(
  violation: BoundaryViolation,
  baseline: BaselineSnapshot
): boolean {
  const id = createViolationId(violation.edge.from, violation.edge.to, violation.edge.line);
  return baseline.violations.some((v) => v.id === id);
}

/**
 * Create default layer structure for common patterns
 */
export function createDefaultLayers(): Layers {
  return {
    presentation: {
      patterns: ['src/controllers/**', 'src/routes/**', 'src/api/**', 'src/handlers/**'],
      depends_on: ['application', 'shared'],
      description: 'HTTP handlers, controllers, API routes',
    },
    application: {
      patterns: ['src/services/**', 'src/use-cases/**', 'src/application/**'],
      depends_on: ['domain', 'infrastructure', 'shared'],
      description: 'Business logic, use cases, services',
    },
    domain: {
      patterns: ['src/domain/**', 'src/entities/**', 'src/models/**'],
      depends_on: ['shared'],
      description: 'Domain entities, value objects, domain logic',
    },
    infrastructure: {
      patterns: ['src/repositories/**', 'src/data/**', 'src/infrastructure/**', 'src/db/**'],
      depends_on: ['domain', 'shared'],
      description: 'Data access, external services, infrastructure',
    },
    shared: {
      patterns: ['src/utils/**', 'src/lib/**', 'src/common/**', 'src/shared/**'],
      depends_on: [],
      description: 'Shared utilities, helpers, common code',
    },
  };
}

/**
 * Create default boundaries from layer structure
 */
export function createDefaultBoundaries(layers: Layers): Boundary[] {
  const boundaries: Boundary[] = [];
  const layerNames = Object.keys(layers);

  for (const fromLayer of layerNames) {
    const allowedDeps = layers[fromLayer].depends_on;

    for (const toLayer of layerNames) {
      // Skip self-references
      if (fromLayer === toLayer) continue;

      // If not in allowed deps, create a boundary
      if (!allowedDeps.includes(toLayer)) {
        boundaries.push({
          name: `no-${fromLayer}-to-${toLayer}`,
          from: fromLayer,
          to: toLayer,
          severity: 'error',
          message: `${fromLayer} layer must not directly depend on ${toLayer}`,
          confidence: 'high',
        });
      }
    }
  }

  return boundaries;
}
