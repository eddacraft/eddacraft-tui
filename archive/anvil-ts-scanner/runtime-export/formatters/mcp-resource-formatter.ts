/**
 * MCP Resource Formatter
 *
 * Formats Anvil constraints as Model Context Protocol (MCP) compatible resources.
 *
 * MCP is a protocol for exposing contextual information to AI models during generation.
 * See: https://modelcontextprotocol.io/
 *
 * @module export/formatters/mcp-resource-formatter
 */

import type { Constraints } from '../constraint-collector.js';

// =============================================================================
// MCP Types
// =============================================================================

/**
 * MCP resource representation of constraints
 */
export interface McpResource {
  /** Resource URI (e.g., anvil://constraints) */
  uri: string;
  /** Resource name */
  name: string;
  /** Resource description */
  description: string;
  /** MIME type */
  mimeType: string;
  /** Resource contents */
  contents: McpResourceContents;
}

/**
 * Contents of an MCP resource
 */
export interface McpResourceContents {
  /** Architecture boundaries */
  boundaries?: McpBoundary[];
  /** Layer definitions */
  layers?: McpLayer[];
  /** Anti-pattern rules */
  antiPatterns?: McpAntiPattern[];
  /** Project conventions */
  conventions?: McpConvention[];
  /** Active suppression policies */
  suppressions?: McpSuppression[];
  /** Metadata */
  metadata: {
    /** When resource was generated */
    generatedAt: string;
    /** Workspace root */
    workspaceRoot: string;
    /** Whether baseline exists */
    hasBaseline: boolean;
    /** Schema version */
    version: string;
  };
}

/**
 * MCP representation of a boundary
 */
export interface McpBoundary {
  /** Boundary identifier */
  id: string;
  /** Boundary name */
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
 * MCP representation of a layer
 */
export interface McpLayer {
  /** Layer name */
  name: string;
  /** File patterns */
  patterns: string[];
  /** Dependencies */
  dependsOn: string[];
  /** Description */
  description?: string;
}

/**
 * MCP representation of an anti-pattern
 */
export interface McpAntiPattern {
  /** Pattern ID */
  id: string;
  /** Pattern name */
  name: string;
  /** Pattern category */
  category: string;
  /** Why it's problematic */
  explanation: string;
  /** What to do instead */
  suggestion: string;
  /** Severity level */
  severity: 'error' | 'warning' | 'info';
  /** Whether enabled */
  enabled: boolean;
}

/**
 * MCP representation of a convention
 */
export interface McpConvention {
  /** Convention category */
  category: string;
  /** Description */
  description: string;
  /** Examples */
  examples?: string[];
}

/**
 * MCP representation of a suppression
 */
export interface McpSuppression {
  /** Pattern ID being suppressed */
  patternId: string;
  /** File path */
  file: string;
  /** Suppression scope */
  scope: string;
  /** Reason for suppression */
  reason: string;
  /** Expiry date */
  expiresAt?: string;
}

// =============================================================================
// Formatter Options
// =============================================================================

/**
 * Configuration for MCP resource formatting
 */
export interface McpResourceFormatterOptions {
  /** Base URI for resources */
  baseUri?: string;
  /** Include boundaries */
  includeBoundaries?: boolean;
  /** Include layers */
  includeLayers?: boolean;
  /** Include anti-patterns */
  includeAntiPatterns?: boolean;
  /** Include conventions */
  includeConventions?: boolean;
  /** Include active suppressions */
  includeSuppressions?: boolean;
}

// =============================================================================
// MCP Resource Formatter
// =============================================================================

/**
 * Formats constraints as MCP-compatible resources
 */
export class McpResourceFormatter {
  private readonly options: Required<McpResourceFormatterOptions>;

  constructor(options: McpResourceFormatterOptions = {}) {
    this.options = {
      baseUri: 'anvil://constraints',
      includeBoundaries: true,
      includeLayers: true,
      includeAntiPatterns: true,
      includeConventions: true,
      includeSuppressions: true,
      ...options,
    };
  }

  /**
   * Format constraints as MCP resource
   *
   * @param constraints - Constraints to format
   * @returns MCP resource object
   */
  format(constraints: Constraints): McpResource {
    return {
      uri: this.options.baseUri,
      name: 'Anvil Architecture Constraints',
      description: 'Architecture rules, anti-patterns, and conventions for this codebase',
      mimeType: 'application/json',
      contents: this.formatContents(constraints),
    };
  }

  /**
   * Format constraints as JSON string
   *
   * @param constraints - Constraints to format
   * @param pretty - Whether to pretty-print JSON
   * @returns JSON string
   */
  formatAsJson(constraints: Constraints, pretty = true): string {
    const resource = this.format(constraints);
    return JSON.stringify(resource, null, pretty ? 2 : 0);
  }

  /**
   * Format resource contents
   */
  private formatContents(constraints: Constraints): McpResourceContents {
    const contents: McpResourceContents = {
      metadata: {
        generatedAt: new Date().toISOString(),
        workspaceRoot: constraints.metadata.workspaceRoot,
        hasBaseline: constraints.metadata.hasBaseline,
        version: '1.0.0',
      },
    };

    if (this.options.includeBoundaries && constraints.boundaries.length > 0) {
      contents.boundaries = constraints.boundaries.map((boundary, index) => ({
        id: `boundary-${index + 1}`,
        name: boundary.name,
        from: boundary.from,
        to: boundary.to,
        message: boundary.message,
        severity: boundary.severity,
      }));
    }

    if (this.options.includeLayers && constraints.layers.length > 0) {
      contents.layers = constraints.layers.map((layer) => ({
        name: layer.name,
        patterns: layer.patterns,
        dependsOn: layer.dependsOn,
        description: layer.description,
      }));
    }

    if (this.options.includeAntiPatterns && constraints.antiPatterns.length > 0) {
      contents.antiPatterns = constraints.antiPatterns.map((pattern) => ({
        id: pattern.id,
        name: pattern.name,
        category: pattern.category,
        explanation: pattern.explanation,
        suggestion: pattern.suggestion,
        severity: pattern.severity,
        enabled: pattern.enabled,
      }));
    }

    if (this.options.includeConventions && constraints.conventions.length > 0) {
      contents.conventions = constraints.conventions.map((convention) => ({
        category: convention.category,
        description: convention.description,
        examples: convention.examples,
      }));
    }

    if (this.options.includeSuppressions && constraints.suppressions.length > 0) {
      contents.suppressions = constraints.suppressions.map((suppression) => ({
        patternId: suppression.patternId,
        file: suppression.file,
        scope: suppression.scope,
        reason: suppression.reason,
        expiresAt: suppression.expiresAt,
      }));
    }

    return contents;
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Format constraints as MCP resource with default options
 *
 * @param constraints - Constraints to format
 * @returns MCP resource object
 */
export function formatAsMcpResource(constraints: Constraints): McpResource {
  const formatter = new McpResourceFormatter();
  return formatter.format(constraints);
}

/**
 * Format constraints as MCP resource JSON
 *
 * @param constraints - Constraints to format
 * @param pretty - Whether to pretty-print JSON
 * @returns JSON string
 */
export function formatAsMcpResourceJson(constraints: Constraints, pretty = true): string {
  const formatter = new McpResourceFormatter();
  return formatter.formatAsJson(constraints, pretty);
}
