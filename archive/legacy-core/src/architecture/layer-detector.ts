/**
 * Layer detection heuristics
 *
 * Detects architectural layers from directory structure and file patterns.
 */

import { minimatch } from 'minimatch';
import type { Layers, LayerAssignment, DetectionConfidence } from './types.js';

/**
 * Layer detection pattern with priority
 */
interface LayerPattern {
  layer: string;
  patterns: string[];
  priority: number; // Lower = higher priority (more specific)
}

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
 * Layer detector using directory heuristics
 */
export class LayerDetector {
  private patterns: LayerPattern[];
  private customLayers: Layers | null = null;

  constructor(customLayers?: Layers) {
    if (customLayers) {
      this.customLayers = customLayers;
      this.patterns = this.layersToPatterns(customLayers);
    } else {
      this.patterns = DEFAULT_LAYER_PATTERNS;
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
   */
  detectLayer(filePath: string): LayerAssignment {
    const matches: Array<{
      layer: string;
      pattern: string;
      priority: number;
    }> = [];

    // Normalise path separators
    const normalisedPath = filePath.replace(/\\/g, '/');

    for (const layerPattern of this.patterns) {
      for (const pattern of layerPattern.patterns) {
        if (minimatch(normalisedPath, pattern, { matchBase: true })) {
          matches.push({
            layer: layerPattern.layer,
            pattern,
            priority: layerPattern.priority,
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

    // Sort by priority (lower = higher priority)
    matches.sort((a, b) => a.priority - b.priority);

    const bestMatch = matches[0];

    // Determine confidence based on match quality
    let confidence: DetectionConfidence = 'high';

    // If multiple layers matched, reduce confidence
    const uniqueLayers = new Set(matches.map((m) => m.layer));
    if (uniqueLayers.size > 1) {
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
