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
    // Log directory access errors for debugging
    console.debug(`[ArchitectureService] Failed to read directory ${dir}:`, error);
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
 * Format entry points for display
 */
export function formatEntryPoints(entryPoints: EntryPoint[]): string[] {
  if (entryPoints.length === 0) {
    return ['  (no entry points detected)'];
  }

  return entryPoints.map((ep) => {
    const confidence = ep.confidence === 'high' ? '' : ` (${ep.confidence} confidence)`;
    return `  • ${ep.path} (${ep.type})${confidence}`;
  });
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
