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
  type InferBaselineOptions,
  ArchitectureAnalyzer,
  createArchitectureAnalyzer,
  analyseArchitecture,
  inferBaseline,
} from './analyzer.js';

// Edge detection
export {
  type ImportEdge,
  type BaselineComparison,
  type ExtractOptions,
  createEdgeFingerprint,
  fingerprintEdge,
  resolveImportPath,
  extractImports,
  extractImportsFromFiles,
  compareToBaseline,
  toDependencyEdge,
  deduplicateEdges,
  filterCrossLayerEdges,
} from './edge-detector.js';

// Architecture definition schema
export {
  ArchitectureTemplateSchema,
  type ArchitectureTemplate,
  LayerDefinitionSchema,
  type LayerDefinition,
  BoundedContextSchema,
  type BoundedContext,
  RuleSeveritySchema,
  type RuleSeverity,
  ArchitectureRuleSchema,
  type ArchitectureRule,
  ArchitectureOptionsSchema,
  type ArchitectureOptions,
  ArchitectureDefinitionSchema,
  type ArchitectureDefinition,
  AVAILABLE_TEMPLATES,
  getAvailableTemplates,
  isValidTemplate,
  ARCHITECTURE_DEFINITION_VERSION,
  validateArchitectureDefinition,
  getDefaultOptions,
} from './definition-schema.js';

// YAML parser
export {
  ARCHITECTURE_YAML_FILENAME,
  getArchitectureYamlPath,
  architectureYamlExists,
  parseArchitectureDefinition,
  writeArchitectureYaml,
  getTemplateDefaults,
  mergeWithTemplate,
  createDefinitionFromTemplate,
} from './yaml-parser.js';

// DC config generator
export {
  DC_CONFIG_FILENAME,
  getDCConfigPath,
  dcConfigExists,
  needsRegeneration,
  writeDCConfig,
  generateDCConfig,
} from './dc-generator.js';

// Rego policy generator
export {
  GENERATED_POLICIES_DIR,
  REGO_FILENAME,
  REGO_PACKAGE,
  getRegoPath,
  regoExists,
  needsRegoRegeneration,
  writeRegoPolicy,
  generateRegoPolicy,
} from './rego-generator.js';

// Architecture compiler (DC + Rego orchestration)
export {
  type CompileResult,
  type CompileOptions,
  compileArchitecture,
  needsCompilation,
} from './compiler.js';

// Template loader
export {
  type TemplateFile,
  type LoadedTemplate,
  TemplateLoader,
  getTemplateLoader,
  listTemplates,
  getTemplate as getArchitectureTemplate,
  validateTemplate,
} from './templates/index.js';

// Architecture context (for OPA)
export {
  ArchViolationSeveritySchema,
  type ArchViolationSeverity,
  ArchViolationSchema,
  type ArchViolation,
  ModuleInfoSchema,
  type ModuleInfo,
  LayerStatsSchema,
  type LayerStats,
  ArchitectureContextSchema,
  type ArchitectureContext,
  createEmptyContext,
  ArchitectureContextBuilder,
  createContextBuilder,
} from './context.js';
