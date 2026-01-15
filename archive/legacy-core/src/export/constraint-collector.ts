/**
 * Constraint Collector
 *
 * Aggregates constraints from architecture baselines, anti-pattern catalogues,
 * and project configuration into a single exportable data structure.
 *
 * This module provides the foundation for exporting constraints in various
 * formats (llms.txt, MCP resources, prompt fragments) for AI tool consumption.
 *
 * @module export/constraint-collector
 */

import { PATTERNS } from '../antipattern/patterns.js';
import type { AntiPattern } from '../antipattern/types.js';
import { loadBaseline } from '../architecture/baseline.js';
import type { ArchitectureBaseline, Boundary, Layer } from '../architecture/types.js';

// =============================================================================
// Types
// =============================================================================

/**
 * Architecture boundary constraint
 */
export interface BoundaryConstraint {
  /** Human-readable name */
  name: string;
  /** Source layer */
  from: string;
  /** Target layer */
  to: string;
  /** Violation message */
  message: string;
  /** Severity level */
  severity: 'error' | 'warning' | 'info';
}

/**
 * Layer definition constraint
 */
export interface LayerConstraint {
  /** Layer name */
  name: string;
  /** Glob patterns matching files in this layer */
  patterns: string[];
  /** Layers this layer can depend on */
  dependsOn: string[];
  /** Human-readable description */
  description?: string;
}

/**
 * Anti-pattern constraint
 */
export interface AntiPatternConstraint {
  /** Pattern ID (e.g., AP-001) */
  id: string;
  /** Pattern name */
  name: string;
  /** Pattern category */
  category: string;
  /** Why this pattern is problematic */
  explanation: string;
  /** What to do instead */
  suggestion: string;
  /** Default severity */
  severity: 'error' | 'warning' | 'info';
  /** Whether enabled by default */
  enabled: boolean;
}

/**
 * Project convention constraint
 */
export interface ConventionConstraint {
  /** Convention category */
  category: string;
  /** Human-readable description */
  description: string;
  /** Examples */
  examples?: string[];
}

/**
 * Aggregated constraints from all sources
 */
export interface Constraints {
  /** Architecture boundaries */
  boundaries: BoundaryConstraint[];
  /** Layer definitions */
  layers: LayerConstraint[];
  /** Anti-pattern rules */
  antiPatterns: AntiPatternConstraint[];
  /** Project conventions */
  conventions: ConventionConstraint[];
  /** Metadata */
  metadata: {
    /** When constraints were collected */
    collectedAt: string;
    /** Workspace root */
    workspaceRoot: string;
    /** Whether architecture baseline exists */
    hasBaseline: boolean;
  };
}

// =============================================================================
// Constraint Collector
// =============================================================================

/**
 * Configuration for constraint collection
 */
export interface ConstraintCollectorConfig {
  /** Workspace root directory */
  workspaceRoot: string;
  /** Include opt-in anti-patterns */
  includeOptInPatterns?: boolean;
  /** Include disabled patterns */
  includeDisabledPatterns?: boolean;
}

/**
 * Collects and aggregates constraints from various sources
 */
export class ConstraintCollector {
  private readonly config: Required<ConstraintCollectorConfig>;

  constructor(config: ConstraintCollectorConfig) {
    this.config = {
      includeOptInPatterns: false,
      includeDisabledPatterns: false,
      ...config,
    };
  }

  /**
   * Collect all constraints
   */
  async collect(): Promise<Constraints> {
    const baseline = loadBaseline(this.config.workspaceRoot);
    const hasBaseline = baseline !== null;

    return {
      boundaries: this.collectBoundaries(baseline),
      layers: this.collectLayers(baseline),
      antiPatterns: this.collectAntiPatterns(),
      conventions: this.collectConventions(),
      metadata: {
        collectedAt: new Date().toISOString(),
        workspaceRoot: this.config.workspaceRoot,
        hasBaseline,
      },
    };
  }

  /**
   * Collect architecture boundaries
   */
  private collectBoundaries(baseline: ArchitectureBaseline | null): BoundaryConstraint[] {
    if (!baseline) {
      return [];
    }

    return baseline.boundaries.map((boundary: Boundary) => ({
      name: boundary.name,
      from: boundary.from,
      to: boundary.to,
      message: boundary.message,
      severity: boundary.severity,
    }));
  }

  /**
   * Collect layer definitions
   */
  private collectLayers(baseline: ArchitectureBaseline | null): LayerConstraint[] {
    if (!baseline) {
      return [];
    }

    return Object.entries(baseline.layers).map(([name, layer]: [string, Layer]) => ({
      name,
      patterns: layer.patterns,
      dependsOn: layer.depends_on,
      description: layer.description,
    }));
  }

  /**
   * Collect anti-pattern rules
   */
  private collectAntiPatterns(): AntiPatternConstraint[] {
    let patterns = PATTERNS.slice();

    // Filter based on configuration
    if (!this.config.includeDisabledPatterns) {
      patterns = patterns.filter((p) => p.enabled);
    }

    if (!this.config.includeOptInPatterns) {
      patterns = patterns.filter((p) => !p.optIn);
    }

    return patterns.map((pattern: AntiPattern) => ({
      id: pattern.id,
      name: pattern.name,
      category: pattern.category,
      explanation: pattern.explanation,
      suggestion: pattern.suggestion,
      severity: pattern.severity,
      enabled: pattern.enabled,
    }));
  }

  /**
   * Collect project conventions
   *
   * These are static conventions for the Anvil project itself.
   * In a more generic system, these could be loaded from configuration.
   */
  private collectConventions(): ConventionConstraint[] {
    return [
      {
        category: 'spelling',
        description: 'Use UK English spelling',
        examples: ['organised (not organized)', 'behaviour (not behavior)', 'colour (not color)'],
      },
      {
        category: 'imports',
        description: 'ESM imports require .js extensions',
        examples: ["import { foo } from './bar.js'", "NOT: import { foo } from './bar'"],
      },
      {
        category: 'schemas',
        description: 'Zod schemas as source of truth for types',
        examples: [
          'export const FooSchema = z.object({ ... })',
          'export type Foo = z.infer<typeof FooSchema>',
        ],
      },
      {
        category: 'naming',
        description: 'Kebab-case for file names',
        examples: ['gate-runner.ts', 'format-detection.ts'],
      },
      {
        category: 'type-safety',
        description: 'No type assertions without runtime validation',
        examples: ['Use Zod parse, not "as" casts', 'Avoid @ts-ignore and @ts-expect-error'],
      },
    ];
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Collect constraints with default configuration
 *
 * @param workspaceRoot - Workspace root directory
 * @returns Aggregated constraints
 */
export async function collectConstraints(workspaceRoot: string): Promise<Constraints> {
  const collector = new ConstraintCollector({ workspaceRoot });
  return collector.collect();
}

/**
 * Check if any constraints exist
 *
 * @param constraints - Constraints to check
 * @returns true if any constraints exist
 */
export function hasAnyConstraints(constraints: Constraints): boolean {
  return (
    constraints.boundaries.length > 0 ||
    constraints.layers.length > 0 ||
    constraints.antiPatterns.length > 0 ||
    constraints.conventions.length > 0
  );
}

/**
 * Count total constraints
 *
 * @param constraints - Constraints to count
 * @returns Total number of constraints
 */
export function countConstraints(constraints: Constraints): number {
  return (
    constraints.boundaries.length +
    constraints.layers.length +
    constraints.antiPatterns.length +
    constraints.conventions.length
  );
}
