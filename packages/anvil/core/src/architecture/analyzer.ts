/**
 * Architecture analyzer
 *
 * Main entry point for architecture analysis. Combines layer detection,
 * entry point detection, and dependency analysis.
 */

import { readdirSync, statSync } from 'fs';
import { join } from 'path';
import { minimatch } from 'minimatch';
import { LayerDetector, createLayerDetector } from './layer-detector.js';
import { EntryPointDetector, createEntryPointDetector } from './entry-detector.js';
import { BaselineManager, createBaselineManager } from './baseline.js';
import {
  type ArchitectureBaseline,
  type Layers,
  type EntryPoint,
  type LayerAssignment,
  type BoundaryViolation,
  type BaselineViolation,
  createViolationId,
  createDefaultLayers,
  createDefaultBoundaries,
} from './types.js';

/**
 * Analysis result
 */
export interface AnalysisResult {
  /** Detected entry points */
  entryPoints: EntryPoint[];
  /** Suggested layer structure */
  layers: Layers;
  /** Layer assignments for all files */
  assignments: LayerAssignment[];
  /** Files with ambiguous layer assignments */
  ambiguous: LayerAssignment[];
  /** Total modules analysed */
  moduleCount: number;
  /** Detected violations */
  violations: BoundaryViolation[];
  /** New violations (not in baseline) */
  newViolations: BoundaryViolation[];
  /** Existing violations (in baseline) */
  existingViolations: BoundaryViolation[];
}

/**
 * Analyzer options
 */
export interface AnalyzerOptions {
  /** Custom layers (overrides detection) */
  layers?: Layers;
  /** Include test files in analysis */
  includeTests?: boolean;
  /** File patterns to include */
  includePatterns?: string[];
  /** File patterns to exclude */
  excludePatterns?: string[];
}

/**
 * Default exclude patterns
 */
const DEFAULT_EXCLUDE_PATTERNS = [
  '**/node_modules/**',
  '**/.git/**',
  '**/dist/**',
  '**/build/**',
  '**/coverage/**',
  '**/*.d.ts',
];

/**
 * Default include patterns
 */
const DEFAULT_INCLUDE_PATTERNS = [
  '**/*.ts',
  '**/*.tsx',
  '**/*.js',
  '**/*.jsx',
  '**/*.mjs',
  '**/*.cjs',
];

/**
 * Architecture analyzer
 */
export class ArchitectureAnalyzer {
  private layerDetector: LayerDetector;
  private entryPointDetector: EntryPointDetector;
  private baselineManager: BaselineManager;
  private options: Required<AnalyzerOptions>;

  constructor(workspaceRoot: string, options: AnalyzerOptions = {}) {
    this.options = {
      layers: options.layers ?? createDefaultLayers(),
      includeTests: options.includeTests ?? false,
      includePatterns: options.includePatterns ?? DEFAULT_INCLUDE_PATTERNS,
      excludePatterns: options.excludePatterns ?? DEFAULT_EXCLUDE_PATTERNS,
    };

    this.layerDetector = createLayerDetector(this.options.layers);
    this.entryPointDetector = createEntryPointDetector(workspaceRoot);
    this.baselineManager = createBaselineManager(workspaceRoot);
  }

  /**
   * Analyse the codebase and return results
   */
  async analyse(filePaths: string[]): Promise<AnalysisResult> {
    // Filter files
    const filteredPaths = this.filterFiles(filePaths);

    // Detect entry points
    let entryPoints = this.entryPointDetector.detectEntryPoints(filteredPaths);
    if (!this.options.includeTests) {
      entryPoints = this.entryPointDetector.filterNonTestEntryPoints(entryPoints);
    }

    // Detect layers
    const assignments = this.layerDetector.detectLayers(filteredPaths);
    const ambiguous = this.layerDetector.findAmbiguousAssignments(filteredPaths);

    // Suggest layers based on detected patterns
    const suggestedLayers = this.layerDetector.suggestLayers(filteredPaths);

    // Use suggested layers if no custom layers provided
    const layers =
      Object.keys(this.options.layers).length > 0 ? this.options.layers : suggestedLayers;

    // Detect violations (would need dependency graph - placeholder for now)
    const violations: BoundaryViolation[] = [];

    // Classify violations as new or existing
    const baseline = this.baselineManager.load();
    const { newViolations, existingViolations } = this.classifyViolations(violations, baseline);

    return {
      entryPoints,
      layers,
      assignments,
      ambiguous,
      moduleCount: filteredPaths.length,
      violations,
      newViolations,
      existingViolations,
    };
  }

  /**
   * Filter files based on include/exclude patterns
   */
  private filterFiles(filePaths: string[]): string[] {
    return filePaths.filter((path) => {
      // Check excludes
      for (const pattern of this.options.excludePatterns) {
        if (minimatch(path, pattern, { matchBase: true })) {
          return false;
        }
      }

      // Check includes
      for (const pattern of this.options.includePatterns) {
        if (minimatch(path, pattern, { matchBase: true })) {
          return true;
        }
      }

      return false;
    });
  }

  /**
   * Classify violations as new or existing
   */
  private classifyViolations(
    violations: BoundaryViolation[],
    baseline: ArchitectureBaseline | null
  ): {
    newViolations: BoundaryViolation[];
    existingViolations: BoundaryViolation[];
  } {
    if (!baseline) {
      // No baseline = all violations are new
      return {
        newViolations: violations.map((v) => ({ ...v, is_new: true })),
        existingViolations: [],
      };
    }

    const baselineIds = new Set(baseline.baseline_snapshot.violations.map((v) => v.id));

    const newViolations: BoundaryViolation[] = [];
    const existingViolations: BoundaryViolation[] = [];

    for (const violation of violations) {
      const id = createViolationId(violation.edge.from, violation.edge.to, violation.edge.line);

      if (baselineIds.has(id)) {
        existingViolations.push({
          ...violation,
          is_new: false,
          baseline_id: id,
        });
      } else {
        newViolations.push({
          ...violation,
          is_new: true,
        });
      }
    }

    return { newViolations, existingViolations };
  }

  /**
   * Create a baseline from analysis results
   */
  createBaseline(result: AnalysisResult): ArchitectureBaseline {
    const violations: BaselineViolation[] = result.violations.map((v) => ({
      id: createViolationId(v.edge.from, v.edge.to, v.edge.line),
      from_layer: v.edge.from_layer ?? 'unknown',
      to_layer: v.edge.to_layer ?? 'unknown',
      from_file: v.edge.from,
      to_file: v.edge.to,
      import_line: v.edge.line,
    }));

    return this.baselineManager.create({
      entryPoints: result.entryPoints,
      layers: result.layers,
      boundaries: createDefaultBoundaries(result.layers),
      violations,
      moduleCount: result.moduleCount,
    });
  }

  /**
   * Update baseline with new analysis
   */
  updateBaseline(result: AnalysisResult): ArchitectureBaseline | null {
    const violations: BaselineViolation[] = result.violations.map((v) => ({
      id: createViolationId(v.edge.from, v.edge.to, v.edge.line),
      from_layer: v.edge.from_layer ?? 'unknown',
      to_layer: v.edge.to_layer ?? 'unknown',
      from_file: v.edge.from,
      to_file: v.edge.to,
      import_line: v.edge.line,
    }));

    return this.baselineManager.update({
      entryPoints: result.entryPoints,
      layers: result.layers,
      violations,
      moduleCount: result.moduleCount,
    });
  }

  /**
   * Get the baseline manager
   */
  getBaselineManager(): BaselineManager {
    return this.baselineManager;
  }

  /**
   * Check if baseline exists
   */
  hasBaseline(): boolean {
    return this.baselineManager.exists();
  }

  /**
   * Load existing baseline
   */
  loadBaseline(): ArchitectureBaseline | null {
    return this.baselineManager.load();
  }
}

/**
 * Create an architecture analyzer
 */
export function createArchitectureAnalyzer(
  workspaceRoot: string,
  options?: AnalyzerOptions
): ArchitectureAnalyzer {
  return new ArchitectureAnalyzer(workspaceRoot, options);
}

/**
 * Quick analysis helper - analyse and optionally create baseline
 */
export async function analyseArchitecture(
  workspaceRoot: string,
  filePaths: string[],
  options?: AnalyzerOptions & { createBaseline?: boolean }
): Promise<AnalysisResult & { baseline?: ArchitectureBaseline }> {
  const analyzer = createArchitectureAnalyzer(workspaceRoot, options);
  const result = await analyzer.analyse(filePaths);

  if (options?.createBaseline) {
    const baseline = analyzer.createBaseline(result);
    return { ...result, baseline };
  }

  return result;
}

/**
 * Options for baseline inference
 */
export interface InferBaselineOptions extends AnalyzerOptions {
  /** Save baseline to .anvil/architecture.json (default: true) */
  save?: boolean;
}

/**
 * Infer architecture baseline from codebase
 *
 * Scans the workspace, detects layers and entry points, and creates a baseline.
 * This is the primary entry point for `anvil init` architecture setup.
 */
export async function inferBaseline(
  workspaceRoot: string,
  options?: InferBaselineOptions
): Promise<{ result: AnalysisResult; baseline: ArchitectureBaseline }> {
  const analyzer = createArchitectureAnalyzer(workspaceRoot, options);
  const filePaths = collectSourceFiles(workspaceRoot, options);
  const result = await analyzer.analyse(filePaths);
  const baseline = analyzer.createBaseline(result);

  if (options?.save !== false) {
    analyzer.getBaselineManager().save(baseline);
  }

  return { result, baseline };
}

/**
 * Collect source files from workspace
 */
function collectSourceFiles(workspaceRoot: string, options?: AnalyzerOptions): string[] {
  const includePatterns = options?.includePatterns ?? DEFAULT_INCLUDE_PATTERNS;
  const excludePatterns = options?.excludePatterns ?? DEFAULT_EXCLUDE_PATTERNS;
  const files: string[] = [];

  function walk(dir: string, relativePath: string = ''): void {
    let entries: string[];
    try {
      entries = readdirSync(dir);
    } catch {
      return;
    }

    for (const entry of entries) {
      const fullPath = join(dir, entry);
      const relPath = relativePath ? `${relativePath}/${entry}` : entry;

      if (excludePatterns.some((p) => minimatch(relPath, p, { matchBase: true }))) {
        continue;
      }

      let stat;
      try {
        stat = statSync(fullPath);
      } catch {
        continue;
      }

      if (stat.isDirectory()) {
        walk(fullPath, relPath);
      } else if (stat.isFile()) {
        if (includePatterns.some((p) => minimatch(relPath, p, { matchBase: true }))) {
          files.push(relPath);
        }
      }
    }
  }

  walk(workspaceRoot);
  return files;
}
