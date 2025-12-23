/**
 * Architecture analysis module
 *
 * Provides architecture baseline management, layer detection,
 * entry point detection, and boundary violation analysis.
 */

// Types
export {
  // Entry points
  EntryPointTypeSchema,
  type EntryPointType,
  DetectionConfidenceSchema,
  type DetectionConfidence,
  EntryPointSchema,
  type EntryPoint,
  // Layers
  StandardLayerSchema,
  type StandardLayer,
  LayerSchema,
  type Layer,
  LayersSchema,
  type Layers,
  // Boundaries
  BoundarySchema,
  type Boundary,
  // Violations
  BaselineViolationSchema,
  type BaselineViolation,
  // Baseline
  BaselineSnapshotSchema,
  type BaselineSnapshot,
  ArchitectureBaselineSchema,
  type ArchitectureBaseline,
  // Analysis results
  LayerAssignmentSchema,
  type LayerAssignment,
  DependencyEdgeSchema,
  type DependencyEdge,
  BoundaryViolationSchema,
  type BoundaryViolation,
  // Utilities
  createViolationId,
  isExistingViolation,
  createDefaultLayers,
  createDefaultBoundaries,
} from './types.js';

// Layer detection
export { LayerDetector, createLayerDetector } from './layer-detector.js';

// Entry point detection
export { EntryPointDetector, createEntryPointDetector } from './entry-detector.js';

// Baseline management
export {
  BASELINE_FILENAME,
  ANVIL_DIR,
  getBaselinePath,
  baselineExists,
  loadBaseline,
  saveBaseline,
  createBaseline,
  updateBaseline,
  mergeViolations,
  findNewViolations,
  findFixedViolations,
  BaselineManager,
  createBaselineManager,
} from './baseline.js';

// Architecture analyzer
export {
  type AnalysisResult,
  type AnalyzerOptions,
  ArchitectureAnalyzer,
  createArchitectureAnalyzer,
  analyseArchitecture,
} from './analyzer.js';
