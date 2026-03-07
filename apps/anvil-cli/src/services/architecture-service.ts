/**
 * Architecture Service
 *
 * Provides architecture analysis for CLI commands.
 * Wraps core architecture module with CLI-friendly formatting.
 */

import { readdirSync } from 'node:fs';
import { join, relative } from 'node:path';
import {
  createArchitectureAnalyser,
  createBaselineManager,
  createBaseline,
  type ArchitectureBaseline,
  type EntryPoint,
  type Layers,
  type Layer,
} from '@eddacraft/anvil-core';
import { print } from '../utils/output.js';

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
    print(`[ArchitectureService] Failed to read directory ${dir}:`, error);
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

  // Create analyser and run analysis
  const analyser = createArchitectureAnalyser(projectRoot);
  const result = await analyser.analyse(sourceFiles);

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
const ENTRY_POINT_TYPE_INFO: Record<string, { emoji: string; label: string; description: string }> =
  {
    package: {
      emoji: '[PKG]',
      label: 'Packages',
      description: 'Library exports (index.ts, main entry)',
    },
    application: {
      emoji: '[APP]',
      label: 'Applications',
      description: 'App entry points (main.ts, app.ts)',
    },
    http: { emoji: '[HTTP]', label: 'HTTP Handlers', description: 'Routes and controllers' },
    api: { emoji: '[API]', label: 'API Endpoints', description: 'API handlers' },
    cli: { emoji: '[CLI]', label: 'CLI Tools', description: 'Command-line interfaces' },
    worker: { emoji: '[WRK]', label: 'Workers', description: 'Background jobs and workers' },
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
  options: { verbose?: boolean; showExamples?: boolean } = {}
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
  const showExamples = options.showExamples ?? true;
  const maxExamples = options.verbose ? Infinity : MAX_EXAMPLES_PER_TYPE;

  // Order types by priority
  const typeOrder = ['package', 'application', 'http', 'api', 'cli', 'worker'];
  const sortedTypes = Array.from(byType.keys()).sort(
    (a, b) => (typeOrder.indexOf(a) ?? 99) - (typeOrder.indexOf(b) ?? 99)
  );

  for (const type of sortedTypes) {
    const eps = byType.get(type) || [];
    const typeInfo = ENTRY_POINT_TYPE_INFO[type] || { emoji: '[?]', label: type, description: '' };

    // Get example names for summary line (first few paths, shortened)
    const exampleNames = eps.slice(0, 3).map((ep) => {
      // Extract just the filename or last path segment for brevity
      const parts = ep.path.split('/');
      return parts[parts.length - 1].replace(/\.(ts|js|mjs|cjs)$/, '');
    });
    const examplesStr = exampleNames.join(', ');
    const hasMore = eps.length > 3;

    // Type header with count and brief examples
    lines.push(
      `  ${typeInfo.emoji} ${typeInfo.label}: ${eps.length} (${examplesStr}${hasMore ? ', ...' : ''})`
    );

    // Show detailed examples only if requested
    if (showExamples && options.verbose) {
      if (typeInfo.description) {
        lines.push(`      ${typeInfo.description}`);
      }

      const examples = eps.slice(0, maxExamples);
      for (const ep of examples) {
        const confidence = ep.confidence === 'high' ? '' : ` [${ep.confidence}]`;
        lines.push(`      - ${ep.path}${confidence}`);
      }

      // Show "and N more" if truncated
      const remaining = eps.length - examples.length;
      if (remaining > 0) {
        lines.push(`      ... and ${remaining} more`);
      }
    }
  }

  return lines;
}

/**
 * Format entry points summary header.
 *
 * Returns a one-line summary of entry points with total count.
 * Used as a header before the detailed entry point list.
 *
 * @param entryPoints - Array of detected entry points
 * @returns Summary header string
 */
export function formatEntryPointsSummary(entryPoints: EntryPoint[]): string {
  if (entryPoints.length === 0) {
    return 'No entry points detected';
  }

  // Group by type for summary
  const byType = new Map<string, number>();
  for (const ep of entryPoints) {
    byType.set(ep.type, (byType.get(ep.type) || 0) + 1);
  }

  // Build type breakdown
  const typeOrder = ['package', 'application', 'http', 'api', 'cli', 'worker'];
  const typeCounts: string[] = [];

  for (const type of typeOrder) {
    const count = byType.get(type);
    if (count && count > 0) {
      const typeInfo = ENTRY_POINT_TYPE_INFO[type];
      if (typeInfo) {
        typeCounts.push(`${count} ${typeInfo.label.toLowerCase()}`);
      }
    }
  }

  // Handle any types not in the standard order
  for (const [type, count] of byType.entries()) {
    if (!typeOrder.includes(type) && count > 0) {
      typeCounts.push(`${count} ${type}`);
    }
  }

  return `Entry Points (${entryPoints.length} total): ${typeCounts.join(', ')}`;
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

    const fileLabel = fileCount === 1 ? 'file' : 'files';
    const content = `${layerName} (${truncatedPatterns}) [${fileCount} ${fileLabel}]`;
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
 * Convert layer definitions to a Mermaid flowchart showing dependency arrows.
 *
 * Generates a `graph TD` definition from the Layers structure, using each
 * layer's `depends_on` list to draw directed edges. Optionally includes
 * file counts as node labels.
 */
export function layersToMermaid(layers: Layers, assignments?: Map<string, string[]>): string {
  const layerOrder = ['presentation', 'application', 'domain', 'infrastructure', 'shared'];
  const ordered = layerOrder.filter((l) => layers[l]);
  const unordered = Object.keys(layers).filter((l) => !layerOrder.includes(l));
  const all = [...ordered, ...unordered];

  const lines = ['graph TD'];

  // Declare nodes with labels (include file count when available)
  for (const name of all) {
    const fileCount = assignments?.get(name)?.length;
    if (fileCount !== undefined) {
      const fileLabel = fileCount === 1 ? 'file' : 'files';
      lines.push(`  ${name}["${name} (${fileCount} ${fileLabel})"]`);
    }
  }

  // Draw dependency edges
  const edgesAdded = new Set<string>();
  for (const name of all) {
    const layer = layers[name];
    for (const dep of layer.depends_on) {
      if (layers[dep]) {
        const edgeKey = `${name}->${dep}`;
        if (!edgesAdded.has(edgeKey)) {
          lines.push(`  ${name} --> ${dep}`);
          edgesAdded.add(edgeKey);
        }
      }
    }
  }

  return lines.join('\n');
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
 * Confidence level for architecture detection
 */
export type DetectionConfidenceLevel = 'high' | 'medium' | 'low';

/**
 * Recommended architecture template
 */
export type RecommendedTemplate =
  | 'monorepo'
  | 'nx-workspace'
  | 'layered'
  | 'hexagonal'
  | 'clean'
  | 'starter'
  | 'custom';

/**
 * Workspace structure details (for monorepos)
 */
export interface WorkspaceDetails {
  /** Number of apps in apps/ directory */
  appsCount: number;
  /** Number of packages in packages/ directory */
  packagesCount: number;
  /** Number of libs in libs/ directory */
  libsCount: number;
  /** Whether shared/ directory exists */
  hasShared: boolean;
}

/**
 * Architecture explanation result
 */
export interface ArchitectureExplanation {
  /** Detected project structure (monorepo, single-app, etc.) */
  structure: ProjectStructure;
  /** Detected architecture pattern */
  pattern: ArchitecturePattern;
  /** Human-readable pattern name for display */
  patternDisplayName: string;
  /** Confidence level of the detection */
  confidence: DetectionConfidenceLevel;
  /** Human-readable summary */
  summary: string;
  /** Brief description of what was understood about the project */
  organizationDescription: string;
  /** Layers that were detected */
  detectedLayers: string[];
  /** Layers with the most files */
  dominantLayers: string[];
  /** Recommended architecture template */
  recommendedTemplate: RecommendedTemplate;
  /** Reason for template recommendation */
  templateReason: string;
  /** Workspace details (for monorepos) */
  workspaceDetails?: WorkspaceDetails;
  /** Actionable insights */
  insights: string[];
  /** Actionable next steps */
  nextSteps: string[];
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
 * Pattern display names for user-friendly output
 */
const PATTERN_DISPLAY_NAMES: Record<ArchitecturePattern, string> = {
  layered: 'Layered Architecture',
  'feature-based': 'Feature-Based Structure',
  flat: 'Flat Structure',
  mixed: 'Mixed Organization',
};

/**
 * Structure display names for user-friendly output
 */
const STRUCTURE_DISPLAY_NAMES: Record<ProjectStructure, string> = {
  monorepo: 'Monorepo',
  workspace: 'Workspace (Multi-Package)',
  'single-app': 'Single Application',
  library: 'Library/Package',
};

/**
 * Detect workspace details from entry points
 */
function detectWorkspaceDetails(entryPoints: EntryPoint[]): WorkspaceDetails {
  const appsCount = entryPoints.filter((ep) => ep.path.startsWith('apps/')).length;
  const packagesCount = entryPoints.filter((ep) => ep.path.startsWith('packages/')).length;
  const libsCount = entryPoints.filter((ep) => ep.path.startsWith('libs/')).length;
  const hasShared = entryPoints.some((ep) => ep.path.includes('shared/'));

  return { appsCount, packagesCount, libsCount, hasShared };
}

/**
 * Calculate confidence level based on detection quality
 */
function calculateConfidence(
  structure: ProjectStructure,
  pattern: ArchitecturePattern,
  layerAssignments: Map<string, string[]>,
  moduleCount: number,
  entryPoints: EntryPoint[]
): DetectionConfidenceLevel {
  // Calculate layer assignment ratio
  const assignedCount = Array.from(layerAssignments.values()).reduce(
    (sum, files) => sum + files.length,
    0
  );
  const assignedRatio = moduleCount > 0 ? assignedCount / moduleCount : 0;

  // Calculate entry point confidence
  const highConfidenceEntryPoints = entryPoints.filter((ep) => ep.confidence === 'high').length;
  const entryPointConfidenceRatio =
    entryPoints.length > 0 ? highConfidenceEntryPoints / entryPoints.length : 0;

  // High confidence: good layer coverage and high-confidence entry points
  if (assignedRatio > 0.5 && entryPointConfidenceRatio > 0.7) {
    return 'high';
  }

  // Medium confidence: some layer coverage or good entry point detection
  if (assignedRatio > 0.2 || entryPointConfidenceRatio > 0.5) {
    return 'medium';
  }

  // Low confidence: poor coverage
  return 'low';
}

/**
 * Determine recommended template based on detected structure and pattern
 */
function determineRecommendedTemplate(
  structure: ProjectStructure,
  pattern: ArchitecturePattern,
  workspaceDetails?: WorkspaceDetails
): { template: RecommendedTemplate; reason: string } {
  // Monorepo/Workspace detection
  if (structure === 'monorepo') {
    // Check for Nx-style workspace (has libs/ directory)
    if (workspaceDetails?.libsCount && workspaceDetails.libsCount > 0) {
      return {
        template: 'nx-workspace',
        reason:
          'Detected Nx-style workspace with libs/ directory. This template provides optimal package boundary rules.',
      };
    }
    return {
      template: 'monorepo',
      reason:
        'Detected monorepo with apps/ and packages/ directories. This template enforces inter-package dependencies.',
    };
  }

  if (structure === 'workspace') {
    return {
      template: 'monorepo',
      reason:
        'Detected multi-package workspace. This template helps maintain clean package boundaries.',
    };
  }

  // Single app or library detection
  if (pattern === 'layered') {
    return {
      template: 'layered',
      reason:
        'Detected clear architectural layers. This template enforces presentation -> business -> data flow.',
    };
  }

  if (pattern === 'feature-based') {
    return {
      template: 'hexagonal',
      reason:
        'Feature-based structure works well with hexagonal architecture for domain isolation.',
    };
  }

  if (structure === 'library') {
    return {
      template: 'starter',
      reason: 'Small library structure. Starter template provides minimal boundary enforcement.',
    };
  }

  // Default for flat or mixed
  return {
    template: 'starter',
    reason:
      'Project structure is flexible. Starter template lets you define custom boundaries as you grow.',
  };
}

/**
 * Generate organization description based on detected structure
 */
function generateOrganizationDescription(
  structure: ProjectStructure,
  pattern: ArchitecturePattern,
  detectedLayers: string[],
  dominantLayers: string[],
  workspaceDetails?: WorkspaceDetails
): string {
  if (structure === 'monorepo' || structure === 'workspace') {
    const parts: string[] = [];

    if (workspaceDetails) {
      if (workspaceDetails.appsCount > 0) {
        parts.push(`${workspaceDetails.appsCount} apps in apps/`);
      }
      if (workspaceDetails.packagesCount > 0) {
        parts.push(`${workspaceDetails.packagesCount} packages in packages/`);
      }
      if (workspaceDetails.libsCount > 0) {
        parts.push(`${workspaceDetails.libsCount} libs in libs/`);
      }
    }

    if (parts.length > 0) {
      return parts.join(', ');
    }
    return 'Multiple packages with shared dependencies';
  }

  if (detectedLayers.length > 0) {
    if (dominantLayers.length > 0) {
      return `Code concentrated in ${dominantLayers.join(' and ')} layers`;
    }
    return `Organized across ${detectedLayers.join(', ')} layers`;
  }

  if (pattern === 'flat') {
    return 'Flat directory structure without clear layer separation';
  }

  return 'Custom organization pattern';
}

/**
 * Generate actionable next steps
 */
function generateNextSteps(
  structure: ProjectStructure,
  pattern: ArchitecturePattern,
  recommendedTemplate: RecommendedTemplate,
  confidence: DetectionConfidenceLevel
): string[] {
  const steps: string[] = [];

  // Template recommendation
  if (recommendedTemplate !== 'custom') {
    steps.push(
      `Run 'anvil architecture:init --template ${recommendedTemplate}' to apply optimized boundary rules.`
    );
  }

  // Confidence-based suggestions
  if (confidence === 'low') {
    steps.push(
      "Review the detected structure and consider adding an 'architecture.yaml' with custom layer patterns."
    );
  }

  // Structure-specific suggestions
  if (structure === 'monorepo' || structure === 'workspace') {
    steps.push("Use 'anvil gate:boundaries' to check for unauthorized cross-package dependencies.");
  } else if (pattern === 'flat') {
    steps.push(
      'As your project grows, consider organizing code by layer (e.g., services/, domain/, infrastructure/).'
    );
  }

  // Default next step
  if (steps.length === 0) {
    steps.push("Run 'anvil gate' to validate your codebase against the detected boundaries.");
  }

  return steps;
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

  // Detect workspace details for monorepos
  const workspaceDetails =
    structure === 'monorepo' || structure === 'workspace'
      ? detectWorkspaceDetails(entryPoints)
      : undefined;

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

  // Calculate confidence
  const confidence = calculateConfidence(
    structure,
    pattern,
    layerAssignments,
    moduleCount,
    entryPoints
  );

  // Determine recommended template
  const { template: recommendedTemplate, reason: templateReason } = determineRecommendedTemplate(
    structure,
    pattern,
    workspaceDetails
  );

  // Generate display name
  const patternDisplayName = `${STRUCTURE_DISPLAY_NAMES[structure]} - ${PATTERN_DISPLAY_NAMES[pattern]}`;

  // Generate organization description
  const organizationDescription = generateOrganizationDescription(
    structure,
    pattern,
    detectedLayers,
    dominantLayers,
    workspaceDetails
  );

  // Generate summary
  const summary_text = generateSummaryText(structure, pattern, detectedLayers, moduleCount);

  // Generate insights
  const insights = generateInsights(structure, pattern, layerAssignments, moduleCount, entryPoints);

  // Generate next steps
  const nextSteps = generateNextSteps(structure, pattern, recommendedTemplate, confidence);

  return {
    structure,
    pattern,
    patternDisplayName,
    confidence,
    summary: summary_text,
    organizationDescription,
    detectedLayers,
    dominantLayers,
    recommendedTemplate,
    templateReason,
    workspaceDetails,
    insights,
    nextSteps,
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
 * Confidence level display configuration
 */
const CONFIDENCE_DISPLAY: Record<DetectionConfidenceLevel, { label: string; indicator: string }> = {
  high: { label: 'High', indicator: '[***]' },
  medium: { label: 'Medium', indicator: '[** ]' },
  low: { label: 'Low', indicator: '[*  ]' },
};

/**
 * Format architecture explanation for display.
 *
 * Produces a structured output with clear sections:
 * - Architecture Analysis header with pattern and confidence
 * - Structure breakdown (for monorepos)
 * - Organization description
 * - Recommended template with rationale
 * - Actionable insights
 * - Next steps
 */
export function formatArchitectureExplanation(explanation: ArchitectureExplanation): string[] {
  const lines: string[] = [];
  const confidenceInfo = CONFIDENCE_DISPLAY[explanation.confidence];

  // Architecture Analysis header
  lines.push('Architecture Analysis:');
  lines.push(`  Pattern:    ${explanation.patternDisplayName}`);

  // Structure details (for monorepos)
  if (explanation.workspaceDetails) {
    const { appsCount, packagesCount, libsCount } = explanation.workspaceDetails;
    const structureParts: string[] = [];
    if (appsCount > 0) structureParts.push(`${appsCount} apps`);
    if (packagesCount > 0) structureParts.push(`${packagesCount} packages`);
    if (libsCount > 0) structureParts.push(`${libsCount} libs`);

    if (structureParts.length > 0) {
      lines.push(`  Structure:  ${structureParts.join(', ')}`);
    }
  }

  // Organization description
  lines.push(`  Organization: ${explanation.organizationDescription}`);

  // Confidence indicator
  lines.push(`  Confidence: ${confidenceInfo.indicator} ${confidenceInfo.label}`);
  lines.push('');

  // Recommended template section
  lines.push(`  Recommended Template: ${explanation.recommendedTemplate}`);
  lines.push(`    ${explanation.templateReason}`);
  lines.push('');

  // Insights (if any beyond the defaults)
  if (explanation.insights.length > 0) {
    lines.push('  Insights:');
    for (const insight of explanation.insights) {
      // Wrap long insights
      const wrappedInsight = wrapText(insight, 70);
      lines.push(`    - ${wrappedInsight[0]}`);
      for (let i = 1; i < wrappedInsight.length; i++) {
        lines.push(`      ${wrappedInsight[i]}`);
      }
    }
    lines.push('');
  }

  // Next steps
  if (explanation.nextSteps.length > 0) {
    lines.push('  Next Steps:');
    for (const step of explanation.nextSteps) {
      lines.push(`    - ${step}`);
    }
  }

  return lines;
}

/**
 * Wrap text to a maximum width
 */
function wrapText(text: string, maxWidth: number): string[] {
  if (text.length <= maxWidth) {
    return [text];
  }

  const words = text.split(' ');
  const lines: string[] = [];
  let currentLine = '';

  for (const word of words) {
    if (currentLine.length + word.length + 1 <= maxWidth) {
      currentLine = currentLine ? `${currentLine} ${word}` : word;
    } else {
      if (currentLine) {
        lines.push(currentLine);
      }
      currentLine = word;
    }
  }

  if (currentLine) {
    lines.push(currentLine);
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
