/**
 * Prompt Fragment Formatter
 *
 * Formats Anvil constraints as copy-paste system prompt text for manual
 * AI tool configuration. Optimised for Claude, ChatGPT, and similar tools.
 *
 * @module export/formatters/prompt-formatter
 */

import type { Constraints } from '../constraint-collector.js';

// =============================================================================
// Formatter Options
// =============================================================================

/**
 * Configuration for prompt formatting
 */
export interface PromptFormatterOptions {
  /** Include boundaries section */
  includeBoundaries?: boolean;
  /** Include layers section */
  includeLayers?: boolean;
  /** Include anti-patterns section */
  includeAntiPatterns?: boolean;
  /** Include conventions section */
  includeConventions?: boolean;
  /** Include active suppressions section */
  includeSuppressions?: boolean;
  /** Use concise format (shorter, less detail) */
  concise?: boolean;
}

// =============================================================================
// Prompt Formatter
// =============================================================================

/**
 * Formats constraints as system prompt text
 */
export class PromptFormatter {
  private readonly options: Required<PromptFormatterOptions>;

  constructor(options: PromptFormatterOptions = {}) {
    this.options = {
      includeBoundaries: true,
      includeLayers: true,
      includeAntiPatterns: true,
      includeConventions: true,
      includeSuppressions: true,
      concise: false,
      ...options,
    };
  }

  /**
   * Format constraints as prompt text
   *
   * @param constraints - Constraints to format
   * @returns System prompt text
   */
  format(constraints: Constraints): string {
    const sections: string[] = [];

    // Opening instruction
    sections.push(this.formatOpening());

    // Architecture boundaries
    if (this.options.includeBoundaries && constraints.boundaries.length > 0) {
      sections.push(this.formatBoundaries(constraints));
    }

    // Layer definitions
    if (this.options.includeLayers && constraints.layers.length > 0) {
      sections.push(this.formatLayers(constraints));
    }

    // Anti-patterns
    if (this.options.includeAntiPatterns && constraints.antiPatterns.length > 0) {
      sections.push(this.formatAntiPatterns(constraints));
    }

    // Conventions
    if (this.options.includeConventions && constraints.conventions.length > 0) {
      sections.push(this.formatConventions(constraints));
    }

    // Suppressions
    if (this.options.includeSuppressions && constraints.suppressions.length > 0) {
      sections.push(this.formatSuppressions(constraints));
    }

    return sections.join('\n\n');
  }

  /**
   * Format opening instruction
   */
  private formatOpening(): string {
    if (this.options.concise) {
      return (
        'When working on this codebase, follow these architecture rules and conventions. ' +
        'Flag violations during code generation.'
      );
    }

    return (
      'This codebase has specific architecture boundaries, anti-patterns, and conventions that must be followed. ' +
      'When generating or modifying code:\n\n' +
      '1. Respect all architectural boundaries and layer dependencies\n' +
      '2. Avoid all listed anti-patterns\n' +
      '3. Follow project conventions consistently\n' +
      '4. If a boundary violation or anti-pattern is necessary, explain why and suggest alternatives first'
    );
  }

  /**
   * Format architecture boundaries section
   */
  private formatBoundaries(constraints: Constraints): string {
    const lines: string[] = ['**Architecture Boundaries**'];

    if (!this.options.concise) {
      lines.push('These boundaries define which layers can depend on each other:');
    }

    lines.push('');

    for (const boundary of constraints.boundaries) {
      if (this.options.concise) {
        lines.push(`- Layer "${boundary.from}" must not depend on "${boundary.to}"`);
      } else {
        lines.push(
          `- **${boundary.name}**: Layer "${boundary.from}" must not depend on "${boundary.to}"`
        );
        lines.push(`  ${boundary.message}`);
        lines.push(`  Severity: ${boundary.severity}`);
      }
    }

    return lines.join('\n');
  }

  /**
   * Format layer definitions section
   */
  private formatLayers(constraints: Constraints): string {
    const lines: string[] = ['**Layer Definitions**'];

    if (!this.options.concise) {
      lines.push('The codebase is organised into these architectural layers:');
    }

    lines.push('');

    for (const layer of constraints.layers) {
      if (this.options.concise) {
        lines.push(`- **${layer.name}**: ${layer.patterns.join(', ')}`);
        if (layer.dependsOn.length > 0) {
          lines.push(`  Can depend on: ${layer.dependsOn.join(', ')}`);
        }
      } else {
        lines.push(`- **${layer.name}**`);
        if (layer.description) {
          lines.push(`  ${layer.description}`);
        }
        lines.push(`  Files: ${layer.patterns.join(', ')}`);
        if (layer.dependsOn.length > 0) {
          lines.push(`  Allowed dependencies: ${layer.dependsOn.join(', ')}`);
        } else {
          lines.push(`  No dependencies (leaf layer)`);
        }
        lines.push('');
      }
    }

    return lines.join('\n');
  }

  /**
   * Format anti-patterns section
   */
  private formatAntiPatterns(constraints: Constraints): string {
    const lines: string[] = ['**Forbidden Anti-patterns**'];

    if (!this.options.concise) {
      lines.push('Never introduce these code patterns:');
    }

    lines.push('');

    // Group by category for better organisation
    const byCategory = new Map<string, typeof constraints.antiPatterns>();
    for (const pattern of constraints.antiPatterns) {
      if (!byCategory.has(pattern.category)) {
        byCategory.set(pattern.category, []);
      }
      byCategory.get(pattern.category)!.push(pattern);
    }

    for (const [category, patterns] of byCategory) {
      if (!this.options.concise) {
        lines.push(`${this.formatCategoryName(category)}:`);
      }

      for (const pattern of patterns) {
        if (this.options.concise) {
          lines.push(`- ${pattern.name}: ${pattern.suggestion}`);
        } else {
          lines.push(`- **${pattern.name}** (${pattern.id})`);
          lines.push(`  Problem: ${pattern.explanation}`);
          lines.push(`  Instead: ${pattern.suggestion}`);
          lines.push('');
        }
      }
    }

    return lines.join('\n');
  }

  /**
   * Format conventions section
   */
  private formatConventions(constraints: Constraints): string {
    const lines: string[] = ['**Project Conventions**'];

    if (!this.options.concise) {
      lines.push('Follow these conventions for consistency:');
    }

    lines.push('');

    for (const convention of constraints.conventions) {
      lines.push(
        `- **${this.formatCategoryName(convention.category)}**: ${convention.description}`
      );

      if (!this.options.concise && convention.examples && convention.examples.length > 0) {
        for (const example of convention.examples) {
          lines.push(`  • ${example}`);
        }
      }
    }

    return lines.join('\n');
  }

  /**
   * Format active suppressions section
   */
  private formatSuppressions(constraints: Constraints): string {
    const lines: string[] = ['**Active Suppressions**'];

    if (!this.options.concise) {
      lines.push('These violations are intentionally suppressed. Do not flag or fix them:');
    }

    lines.push('');

    // Group by pattern ID
    const byPattern = new Map<string, typeof constraints.suppressions>();
    for (const suppression of constraints.suppressions) {
      if (!byPattern.has(suppression.patternId)) {
        byPattern.set(suppression.patternId, []);
      }
      byPattern.get(suppression.patternId)!.push(suppression);
    }

    for (const [patternId, suppressions] of byPattern) {
      if (this.options.concise) {
        const files = suppressions.map((s) => s.file).join(', ');
        lines.push(`- ${patternId}: suppressed in ${files}`);
      } else {
        lines.push(`${patternId}:`);
        for (const suppression of suppressions) {
          lines.push(`- **${suppression.file}** (${suppression.scope}): ${suppression.reason}`);
          if (suppression.expiresAt) {
            lines.push(`  Expires: ${new Date(suppression.expiresAt).toISOString().slice(0, 10)}`);
          }
        }
        lines.push('');
      }
    }

    return lines.join('\n');
  }

  /**
   * Format category name for display
   */
  private formatCategoryName(category: string): string {
    return category
      .split('-')
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Format constraints as prompt fragment with default options
 *
 * @param constraints - Constraints to format
 * @returns Prompt text
 */
export function formatAsPrompt(constraints: Constraints): string {
  const formatter = new PromptFormatter();
  return formatter.format(constraints);
}

/**
 * Format constraints as concise prompt fragment
 *
 * @param constraints - Constraints to format
 * @returns Concise prompt text
 */
export function formatAsConcisePrompt(constraints: Constraints): string {
  const formatter = new PromptFormatter({ concise: true });
  return formatter.format(constraints);
}
