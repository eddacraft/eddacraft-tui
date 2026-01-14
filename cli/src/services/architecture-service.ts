/**
 * Architecture Service
 *
 * Provides architecture analysis for CLI commands.
 * Wraps core architecture module with CLI-friendly formatting.
 */

import { readdirSync } from 'fs';
import { join, relative } from 'path';
import {
  createArchitectureAnalyzer,
  createBaselineManager,
  createBaseline,
  type ArchitectureBaseline,
  type EntryPoint,
  type Layers,
  type Layer,
} from '@anvil/core';

/**
 * Architecture analysis summary for display
 */
export interface ArchitectureSummary {
  moduleCount: number;
  entryPoints: EntryPoint[];
  layers: Layers;
  layerAssignments: Map<string, string[]>;
}

/**
 * Collect all source files from a directory
 */
function collectSourceFiles(
  dir: string,
  baseDir: string,
  patterns: string[] = ['**/*.ts', '**/*.js']
): string[] {
  const files: string[] = [];
  const excludeDirs = ['node_modules', 'dist', 'build', '.git', '.anvil', 'coverage'];

  try {
    const entries = readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = join(dir, entry.name);

      if (entry.isDirectory()) {
        if (!excludeDirs.includes(entry.name)) {
          files.push(...collectSourceFiles(fullPath, baseDir, patterns));
        }
      } else if (entry.isFile()) {
        const ext = entry.name.split('.').pop();
        if (ext && ['ts', 'js', 'tsx', 'jsx', 'mjs', 'cjs'].includes(ext)) {
          // Skip test files and declaration files
          if (
            !entry.name.includes('.test.') &&
            !entry.name.includes('.spec.') &&
            !entry.name.endsWith('.d.ts')
          ) {
            files.push(relative(baseDir, fullPath));
          }
        }
      }
    }
  } catch (error) {
    // Log directory access errors to stderr for debugging
    console.error(`[ArchitectureService] Failed to read directory ${dir}:`, error);
  }

  return files;
}

/**
 * Analyse project architecture
 */
export async function analyseProjectArchitecture(
  projectRoot: string
): Promise<ArchitectureSummary> {
  // Collect source files
  const sourceFiles = collectSourceFiles(projectRoot, projectRoot);

  // Create analyzer and run analysis
  const analyzer = createArchitectureAnalyzer(projectRoot);
  const result = await analyzer.analyse(sourceFiles);

  // Group files by layer (result.assignments is LayerAssignment[])
  const layerAssignments = new Map<string, string[]>();
  for (const assignment of result.assignments) {
    // Skip unassigned files (layer is null)
    if (assignment.layer === null) continue;

    const existing = layerAssignments.get(assignment.layer) || [];
    existing.push(assignment.file);
    layerAssignments.set(assignment.layer, existing);
  }

  return {
    moduleCount: sourceFiles.length,
    entryPoints: result.entryPoints,
    layers: result.layers,
    layerAssignments,
  };
}

/**
 * Entry point type display configuration
 */
const ENTRY_POINT_TYPE_INFO: Record<string, { label: string; description: string }> = {
  package: { label: 'Package', description: 'Library exports (index.ts, main entry)' },
  application: { label: 'Application', description: 'App entry points (main.ts, app.ts)' },
  http: { label: 'HTTP', description: 'HTTP handlers (routes, controllers)' },
  api: { label: 'API', description: 'API endpoints' },
  cli: { label: 'CLI', description: 'Command-line interfaces' },
};

/**
 * Maximum examples to show per entry point type
 */
const MAX_EXAMPLES_PER_TYPE = 3;

/**
 * Format entry points for display.
 *
 * Groups entry points by type with summary counts, showing only a few
 * examples per type to avoid overwhelming users with long lists.
 *
 * @param entryPoints - Array of detected entry points
 * @param options - Formatting options
 * @returns Array of formatted lines for display
 */
export function formatEntryPoints(
  entryPoints: EntryPoint[],
  options: { verbose?: boolean } = {}
): string[] {
  if (entryPoints.length === 0) {
    return ['  (no entry points detected)'];
  }

  // Group by type
  const byType = new Map<string, EntryPoint[]>();
  for (const ep of entryPoints) {
    const existing = byType.get(ep.type) || [];
    existing.push(ep);
    byType.set(ep.type, existing);
  }

  const lines: string[] = [];
  const maxExamples = options.verbose ? Infinity : MAX_EXAMPLES_PER_TYPE;

  // Order types by priority
  const typeOrder = ['package', 'application', 'http', 'api', 'cli'];
  const sortedTypes = Array.from(byType.keys()).sort(
    (a, b) => (typeOrder.indexOf(a) ?? 99) - (typeOrder.indexOf(b) ?? 99)
  );

  for (const type of sortedTypes) {
    const eps = byType.get(type) || [];
    const typeInfo = ENTRY_POINT_TYPE_INFO[type] || { label: type, description: '' };

    // Type header with count
    lines.push(`  ${typeInfo.label} (${eps.length})`);
    if (typeInfo.description) {
      lines.push(`    ${typeInfo.description}`);
    }

    // Show examples (limited unless verbose)
    const examples = eps.slice(0, maxExamples);
    for (const ep of examples) {
      const confidence = ep.confidence === 'high' ? '' : ` [${ep.confidence}]`;
      lines.push(`    • ${ep.path}${confidence}`);
    }

    // Show "and N more" if truncated
    const remaining = eps.length - examples.length;
    if (remaining > 0) {
      lines.push(`    ... and ${remaining} more`);
    }

    lines.push(''); // Blank line between types
  }

  // Remove trailing blank line
  if (lines[lines.length - 1] === '') {
    lines.pop();
  }

  return lines;
}

/**
 * Format layer structure for display (ASCII box diagram)
 */
export function formatLayerDiagram(layers: Layers, assignments: Map<string, string[]>): string[] {
  const lines: string[] = [];
  const layerOrder = ['presentation', 'application', 'domain', 'infrastructure', 'shared'];

  // Find the widest layer name for box sizing
  const maxWidth = Math.max(
    ...Object.entries(layers).map(([name, layer]: [string, Layer]) => {
      const patterns = layer.patterns.join(', ');
      return `${name} (${patterns})`.length;
    }),
    40
  );

  const boxWidth = Math.min(maxWidth + 4, 60);

  // Top border
  lines.push('  ┌' + '─'.repeat(boxWidth) + '┐');

  // Layers in order
  const orderedLayers = layerOrder.filter((l) => layers[l]);
  const unorderedLayers = Object.keys(layers).filter((l) => !layerOrder.includes(l));
  const allLayers = [...orderedLayers, ...unorderedLayers];

  for (let i = 0; i < allLayers.length; i++) {
    const layerName = allLayers[i];
    const layer = layers[layerName];
    const fileCount = assignments.get(layerName)?.length || 0;
    const patterns = layer.patterns.slice(0, 2).join(', ');
    const truncatedPatterns =
      patterns.length > boxWidth - 10 ? patterns.slice(0, boxWidth - 13) + '...' : patterns;

    const content = `${layerName} (${truncatedPatterns}) [${fileCount} files]`;
    const padding = boxWidth - content.length;
    const leftPad = Math.floor(padding / 2);
    const rightPad = padding - leftPad;

    lines.push('  │' + ' '.repeat(leftPad) + content + ' '.repeat(rightPad) + '│');

    // Add separator between layers (except last)
    if (i < allLayers.length - 1) {
      lines.push('  ├' + '─'.repeat(boxWidth) + '┤');
    }
  }

  // Bottom border
  lines.push('  └' + '─'.repeat(boxWidth) + '┘');

  return lines;
}

/**
 * Detected project structure type
 */
export type ProjectStructure = 'monorepo' | 'workspace' | 'single-app' | 'library';

/**
 * Architecture pattern type
 */
export type ArchitecturePattern = 'layered' | 'feature-based' | 'flat' | 'mixed';

/**
 * Architecture explanation result
 */
export interface ArchitectureExplanation {
  /** Detected project structure (monorepo, single-app, etc.) */
  structure: ProjectStructure;
  /** Detected architecture pattern */
  pattern: ArchitecturePattern;
  /** Human-readable summary */
  summary: string;
  /** Layers that were detected */
  detectedLayers: string[];
  /** Layers with the most files */
  dominantLayers: string[];
  /** Actionable insights */
  insights: string[];
}

/**
 * Detect project structure from entry points and file paths
 */
function detectProjectStructure(entryPoints: EntryPoint[], moduleCount: number): ProjectStructure {
  const packageEntries = entryPoints.filter((ep) => ep.type === 'package');
  const hasMultiplePackages = packageEntries.length > 3;
  const hasWorkspacePattern = entryPoints.some(
    (ep) => ep.path.includes('packages/') || ep.path.includes('apps/')
  );

  if (hasWorkspacePattern || hasMultiplePackages) {
    // Distinguish between monorepo and workspace
    const hasAppsDir = entryPoints.some((ep) => ep.path.startsWith('apps/'));
    return hasAppsDir ? 'monorepo' : 'workspace';
  }

  // Check if this is primarily a library
  const libEntries = packageEntries.length;
  const appEntries = entryPoints.filter((ep) => ep.type === 'application').length;

  if (libEntries > 0 && appEntries === 0 && moduleCount < 50) {
    return 'library';
  }

  return 'single-app';
}

/**
 * Detect architecture pattern from layer assignments
 */
function detectArchitecturePattern(
  layerAssignments: Map<string, string[]>,
  moduleCount: number
): ArchitecturePattern {
  const assignedCount = Array.from(layerAssignments.values()).reduce(
    (sum, files) => sum + files.length,
    0
  );
  const assignedRatio = moduleCount > 0 ? assignedCount / moduleCount : 0;

  // If very few files are assigned to layers, structure is flat
  if (assignedRatio < 0.1) {
    return 'flat';
  }

  // Count layers with files
  const layersWithFiles = Array.from(layerAssignments.entries()).filter(
    ([, files]) => files.length > 0
  );

  // If files are spread across multiple layers
  if (layersWithFiles.length >= 3) {
    return 'layered';
  }

  // If mostly in one or two layers
  if (layersWithFiles.length <= 2 && assignedRatio > 0.3) {
    return 'feature-based';
  }

  return 'mixed';
}

/**
 * Generate architecture explanation.
 *
 * Provides a human-readable explanation of the detected architecture,
 * helping users understand what Anvil has detected about their project.
 */
export function generateArchitectureExplanation(
  summary: ArchitectureSummary
): ArchitectureExplanation {
  const { entryPoints, layerAssignments, moduleCount } = summary;

  // Detect structure and pattern
  const structure = detectProjectStructure(entryPoints, moduleCount);
  const pattern = detectArchitecturePattern(layerAssignments, moduleCount);

  // Find detected layers (those with assigned files)
  const detectedLayers = Array.from(layerAssignments.entries())
    .filter(([, files]) => files.length > 0)
    .map(([layer]) => layer);

  // Find dominant layers (top 2 by file count)
  const dominantLayers = Array.from(layerAssignments.entries())
    .sort(([, a], [, b]) => b.length - a.length)
    .slice(0, 2)
    .filter(([, files]) => files.length > 0)
    .map(([layer]) => layer);

  // Generate summary
  const summary_text = generateSummaryText(structure, pattern, detectedLayers, moduleCount);

  // Generate insights
  const insights = generateInsights(structure, pattern, layerAssignments, moduleCount, entryPoints);

  return {
    structure,
    pattern,
    summary: summary_text,
    detectedLayers,
    dominantLayers,
    insights,
  };
}

/**
 * Generate summary text for the architecture
 */
function generateSummaryText(
  structure: ProjectStructure,
  pattern: ArchitecturePattern,
  detectedLayers: string[],
  moduleCount: number
): string {
  const structureLabels: Record<ProjectStructure, string> = {
    monorepo: 'a monorepo with multiple apps/packages',
    workspace: 'a workspace with multiple packages',
    'single-app': 'a single application',
    library: 'a library/package',
  };

  const patternLabels: Record<ArchitecturePattern, string> = {
    layered: 'layered architecture',
    'feature-based': 'feature-based organisation',
    flat: 'flat structure',
    mixed: 'mixed organisation',
  };

  const structureDesc = structureLabels[structure];
  const patternDesc = patternLabels[pattern];

  if (detectedLayers.length === 0) {
    return (
      `Detected ${structureDesc} with ${moduleCount} modules. ` +
      `No standard architectural layers were identified — this project may use ` +
      `a custom organisation or feature-based structure.`
    );
  }

  return (
    `Detected ${structureDesc} with ${patternDesc}. ` +
    `Found ${moduleCount} modules organised across ${detectedLayers.length} layer(s): ` +
    `${detectedLayers.join(', ')}.`
  );
}

/**
 * Generate actionable insights
 */
function generateInsights(
  structure: ProjectStructure,
  pattern: ArchitecturePattern,
  layerAssignments: Map<string, string[]>,
  moduleCount: number,
  entryPoints: EntryPoint[]
): string[] {
  const insights: string[] = [];

  // Insight: Layer coverage
  const assignedCount = Array.from(layerAssignments.values()).reduce(
    (sum, files) => sum + files.length,
    0
  );
  const assignedRatio = moduleCount > 0 ? assignedCount / moduleCount : 0;

  if (assignedRatio < 0.3 && pattern !== 'flat') {
    insights.push(
      `Only ${Math.round(assignedRatio * 100)}% of files match layer patterns. ` +
        `Consider customising layer patterns in architecture.yaml to match your project structure.`
    );
  }

  // Insight: Monorepo-specific
  if (structure === 'monorepo' || structure === 'workspace') {
    const packageCount = entryPoints.filter((ep) => ep.type === 'package').length;
    insights.push(
      `Detected ${packageCount} package entry points. ` +
        `Anvil will track dependencies between packages to enforce boundaries.`
    );
  }

  // Insight: Missing layers
  const standardLayers = ['presentation', 'application', 'domain', 'infrastructure'];
  const missingLayers = standardLayers.filter(
    (layer) => !layerAssignments.has(layer) || (layerAssignments.get(layer)?.length ?? 0) === 0
  );

  if (missingLayers.length > 0 && missingLayers.length < standardLayers.length) {
    insights.push(
      `Standard layers not detected: ${missingLayers.join(', ')}. ` +
        `This is normal for smaller projects or feature-based architectures.`
    );
  }

  // Insight: Flat structure guidance
  if (pattern === 'flat') {
    insights.push(
      `Your project has a flat structure without clear layers. ` +
        `As the project grows, consider organising code by architectural concern.`
    );
  }

  // Default insight if no specific insights
  if (insights.length === 0) {
    insights.push(
      `Architecture baseline will be saved. Anvil will warn on new cross-boundary dependencies.`
    );
  }

  return insights;
}

/**
 * Format architecture explanation for display
 */
export function formatArchitectureExplanation(explanation: ArchitectureExplanation): string[] {
  const lines: string[] = [];

  // Summary
  lines.push(explanation.summary);
  lines.push('');

  // Insights
  if (explanation.insights.length > 0) {
    for (const insight of explanation.insights) {
      lines.push(`  💡 ${insight}`);
    }
  }

  return lines;
}

/**
 * Create and save architecture baseline
 */
export function saveArchitectureBaseline(
  projectRoot: string,
  summary: ArchitectureSummary
): ArchitectureBaseline {
  const baseline = createBaseline({
    entryPoints: summary.entryPoints,
    layers: summary.layers,
    moduleCount: summary.moduleCount,
  });

  const manager = createBaselineManager(projectRoot);
  manager.save(baseline);

  return baseline;
}

/**
 * Check if architecture baseline already exists
 */
export function hasExistingBaseline(projectRoot: string): boolean {
  const manager = createBaselineManager(projectRoot);
  return manager.exists();
}

/**
 * Load existing architecture baseline
 */
export function loadExistingBaseline(projectRoot: string): ArchitectureBaseline | null {
  const manager = createBaselineManager(projectRoot);
  return manager.load();
}
