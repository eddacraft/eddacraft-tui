/**
 * Layer detection heuristics
 *
 * Detects architectural layers from directory structure and file patterns.
 * Supports both single-app and monorepo project structures.
 */

import { minimatch } from 'minimatch';
import type { Layers, LayerAssignment, DetectionConfidence } from './types.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('architecture');

/**
 * Layer detection pattern with priority
 */
interface LayerPattern {
  layer: string;
  patterns: string[];
  priority: number; // Lower = higher priority (more specific)
}

/**
 * Common source root directories that indicate where the actual source code starts
 */
const SOURCE_ROOT_PATTERNS = new Set(['src', 'lib', 'source', 'app']);

/**
 * Default layer patterns with priorities
 * Priority determines which layer wins when patterns overlap
 */
const DEFAULT_LAYER_PATTERNS: LayerPattern[] = [
  {
    layer: 'presentation',
    patterns: [
      '**/controllers/**',
      '**/routes/**',
      '**/api/**',
      '**/handlers/**',
      '**/endpoints/**',
      '**/views/**',
      '**/pages/**',
    ],
    priority: 1,
  },
  {
    layer: 'application',
    patterns: [
      '**/services/**',
      '**/use-cases/**',
      '**/usecases/**',
      '**/application/**',
      '**/interactors/**',
    ],
    priority: 2,
  },
  {
    layer: 'domain',
    patterns: ['**/domain/**', '**/entities/**', '**/models/**', '**/core/**', '**/business/**'],
    priority: 3,
  },
  {
    layer: 'infrastructure',
    patterns: [
      '**/repositories/**',
      '**/data/**',
      '**/infrastructure/**',
      '**/db/**',
      '**/database/**',
      '**/adapters/**',
      '**/external/**',
      '**/clients/**',
    ],
    priority: 4,
  },
  {
    layer: 'shared',
    patterns: [
      '**/utils/**',
      '**/lib/**',
      '**/common/**',
      '**/shared/**',
      '**/helpers/**',
      '**/types/**',
      '**/constants/**',
      '**/config/**',
    ],
    priority: 5,
  },
];

/**
 * Extract the directory name that a pattern would match
 * e.g., "**\/controllers\/**" -> "controllers"
 */
function getPatternDirectoryName(pattern: string): string | null {
  // Match patterns like "**/controllers/**" or "src/controllers/**"
  const match = pattern.match(/\*\*\/([^/*]+)\/\*\*/);
  if (match) {
    return match[1];
  }
  // Match patterns like "controllers/**"
  const simpleMatch = pattern.match(/^([^/*]+)\/\*\*/);
  if (simpleMatch) {
    return simpleMatch[1];
  }
  return null;
}

/**
 * Find the position of a directory in a path (index from end, 0-based)
 * Returns -1 if not found
 *
 * For "packages/web/src/controllers/user.ts" and "controllers":
 * - parts = ["packages", "web", "src", "controllers", "user.ts"]
 * - "controllers" is at index 3, length is 5
 * - position from end = 5 - 3 - 1 = 1 (1 directory from the file)
 */
function getDirectoryPositionFromEnd(filePath: string, dirName: string): number {
  const parts = filePath.split('/');
  const dirIndex = parts.findIndex((part) => part === dirName);
  if (dirIndex === -1) return -1;
  return parts.length - dirIndex - 1;
}

/**
 * Find the source root position in a path
 * Returns the index of the first source root directory (src, lib, etc.)
 * Returns -1 if no source root is found
 */
function findSourceRootIndex(filePath: string): number {
  const parts = filePath.split('/');
  for (let i = 0; i < parts.length; i++) {
    if (SOURCE_ROOT_PATTERNS.has(parts[i])) {
      return i;
    }
  }
  return -1;
}

/**
 * Check if a directory name is likely a package/app name rather than a layer directory
 * by checking if it appears before the source root
 */
function isBeforeSourceRoot(filePath: string, dirName: string): boolean {
  const parts = filePath.split('/');
  const dirIndex = parts.findIndex((part) => part === dirName);
  const sourceRootIndex = findSourceRootIndex(filePath);

  // If no source root found, can't determine
  if (sourceRootIndex === -1) return false;

  // If the directory appears before the source root, it's likely a package name
  return dirIndex < sourceRootIndex;
}

/**
 * Layer detector using directory heuristics
 */
export class LayerDetector {
  private patterns: LayerPattern[];
  private customLayers: Layers | null = null;

  constructor(customLayers?: Layers) {
    if (customLayers) {
      this.customLayers = customLayers;
      this.patterns = this.layersToPatterns(customLayers);
      debug('LayerDetector created with custom layers', Object.keys(customLayers));
    } else {
      this.patterns = DEFAULT_LAYER_PATTERNS;
      debug('LayerDetector created with default patterns');
    }
  }

  /**
   * Convert Layers config to LayerPattern array
   */
  private layersToPatterns(layers: Layers): LayerPattern[] {
    const result: LayerPattern[] = [];
    let priority = 1;

    for (const [name, layer] of Object.entries(layers)) {
      result.push({
        layer: name,
        patterns: layer.patterns,
        priority: priority++,
      });
    }

    return result;
  }

  /**
   * Detect layer for a single file
   *
   * Uses position-aware matching to handle monorepo structures where package names
   * may match layer patterns (e.g., packages/api/src/services/user.ts should match
   * 'services', not 'api').
   */
  detectLayer(filePath: string): LayerAssignment {
    const matches: Array<{
      layer: string;
      pattern: string;
      priority: number;
      positionScore: number; // Lower = closer to file = better
      isBeforeSrcRoot: boolean;
    }> = [];

    // Normalise path separators
    const normalisedPath = filePath.replace(/\\/g, '/');

    for (const layerPattern of this.patterns) {
      for (const pattern of layerPattern.patterns) {
        if (minimatch(normalisedPath, pattern, { matchBase: true })) {
          const dirName = getPatternDirectoryName(pattern);
          const positionScore = dirName
            ? getDirectoryPositionFromEnd(normalisedPath, dirName)
            : 999;
          const beforeSrcRoot = dirName ? isBeforeSourceRoot(normalisedPath, dirName) : false;

          matches.push({
            layer: layerPattern.layer,
            pattern,
            priority: layerPattern.priority,
            positionScore,
            isBeforeSrcRoot: beforeSrcRoot,
          });
        }
      }
    }

    if (matches.length === 0) {
      return {
        file: filePath,
        layer: null,
        confidence: 'low',
      };
    }

    // Filter out matches that are before the source root (likely package names)
    // unless ALL matches are before the source root
    const afterSrcRootMatches = matches.filter((m) => !m.isBeforeSrcRoot);
    const effectiveMatches = afterSrcRootMatches.length > 0 ? afterSrcRootMatches : matches;

    // Sort matches by:
    // 1. Position score (closer to file = better, i.e., lower score)
    // 2. Layer priority as tiebreaker (lower = higher priority)
    effectiveMatches.sort((a, b) => {
      // First compare position score
      if (a.positionScore !== b.positionScore) {
        return a.positionScore - b.positionScore;
      }
      // Then compare layer priority
      return a.priority - b.priority;
    });

    const bestMatch = effectiveMatches[0];

    // Determine confidence based on match quality
    let confidence: DetectionConfidence = 'high';

    // If multiple layers matched, reduce confidence
    const uniqueLayers = new Set(effectiveMatches.map((m) => m.layer));
    if (uniqueLayers.size > 1) {
      confidence = 'medium';
    }

    // If the best match was before source root, reduce confidence
    if (bestMatch.isBeforeSrcRoot) {
      confidence = 'medium';
    }

    return {
      file: filePath,
      layer: bestMatch.layer,
      confidence,
      matched_pattern: bestMatch.pattern,
    };
  }

  /**
   * Detect layers for multiple files
   */
  detectLayers(filePaths: string[]): LayerAssignment[] {
    return filePaths.map((path) => this.detectLayer(path));
  }

  /**
   * Get all detected layers from a set of files
   */
  getDetectedLayers(filePaths: string[]): Set<string> {
    const layers = new Set<string>();

    for (const path of filePaths) {
      const assignment = this.detectLayer(path);
      if (assignment.layer) {
        layers.add(assignment.layer);
      }
    }

    return layers;
  }

  /**
   * Suggest layer structure based on detected patterns
   */
  suggestLayers(filePaths: string[]): Layers {
    const detectedLayers = this.getDetectedLayers(filePaths);
    const result: Layers = {};

    // Only include layers that were actually detected
    for (const layerPattern of this.patterns) {
      if (detectedLayers.has(layerPattern.layer)) {
        result[layerPattern.layer] = {
          patterns: layerPattern.patterns,
          depends_on: this.getDefaultDependencies(layerPattern.layer),
          description: this.getLayerDescription(layerPattern.layer),
        };
      }
    }

    return result;
  }

  /**
   * Get default dependencies for a layer
   */
  private getDefaultDependencies(layer: string): string[] {
    const deps: Record<string, string[]> = {
      presentation: ['application', 'shared'],
      application: ['domain', 'infrastructure', 'shared'],
      domain: ['shared'],
      infrastructure: ['domain', 'shared'],
      shared: [],
    };

    return deps[layer] ?? [];
  }

  /**
   * Get description for a layer
   */
  private getLayerDescription(layer: string): string {
    const descriptions: Record<string, string> = {
      presentation: 'HTTP handlers, controllers, API routes',
      application: 'Business logic, use cases, services',
      domain: 'Domain entities, value objects, domain logic',
      infrastructure: 'Data access, external services, infrastructure',
      shared: 'Shared utilities, helpers, common code',
    };

    return descriptions[layer] ?? `${layer} layer`;
  }

  /**
   * Check if a dependency from one layer to another is allowed
   */
  isAllowedDependency(fromLayer: string, toLayer: string, layers?: Layers): boolean {
    const layerConfig = layers ?? this.customLayers;

    if (!layerConfig) {
      // Use default rules
      const defaultDeps = this.getDefaultDependencies(fromLayer);
      return fromLayer === toLayer || defaultDeps.includes(toLayer);
    }

    const fromLayerConfig = layerConfig[fromLayer];
    if (!fromLayerConfig) {
      return true; // Unknown layer, allow
    }

    return fromLayer === toLayer || fromLayerConfig.depends_on.includes(toLayer);
  }

  /**
   * Find files with ambiguous layer assignments
   */
  findAmbiguousAssignments(filePaths: string[]): LayerAssignment[] {
    return this.detectLayers(filePaths).filter(
      (a) => a.confidence === 'medium' || a.confidence === 'low'
    );
  }
}

/**
 * Create a layer detector with default patterns
 */
export function createLayerDetector(customLayers?: Layers): LayerDetector {
  return new LayerDetector(customLayers);
}
