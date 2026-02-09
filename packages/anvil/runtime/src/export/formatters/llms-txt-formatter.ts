/**
 * llms.txt Formatter
 *
 * Formats Anvil constraints as llms.txt markdown for AI tool consumption.
 *
 * llms.txt is an emerging community standard for providing AI-readable
 * documentation and constraints. See: https://mintlify.com/blog/simplifying-docs-with-llms-txt
 *
 * @module export/formatters/llms-txt-formatter
 */

import type { Constraints } from '../constraint-collector.js';

// =============================================================================
// Formatter Options
// =============================================================================

/**
 * Configuration for llms.txt formatting
 */
export interface LlmsTxtFormatterOptions {
  /** Include metadata header */
  includeMetadata?: boolean;
  /** Include architecture boundaries section */
  includeBoundaries?: boolean;
  /** Include anti-patterns section */
  includeAntiPatterns?: boolean;
  /** Include conventions section */
  includeConventions?: boolean;
  /** Include layer definitions section */
  includeLayers?: boolean;
  /** Include active suppressions section */
  includeSuppressions?: boolean;
}

// =============================================================================
// llms.txt Formatter
// =============================================================================

/**
 * Formats constraints as llms.txt markdown
 */
export class LlmsTxtFormatter {
  private readonly options: Required<LlmsTxtFormatterOptions>;

  constructor(options: LlmsTxtFormatterOptions = {}) {
    this.options = {
      includeMetadata: true,
      includeBoundaries: true,
      includeAntiPatterns: true,
      includeConventions: true,
      includeLayers: true,
      includeSuppressions: true,
      ...options,
    };
  }

  /**
   * Format constraints as llms.txt markdown
   *
   * @param constraints - Constraints to format
   * @returns llms.txt markdown string
   */
  format(constraints: Constraints): string {
    const sections: string[] = [];

    // Title
    sections.push('# Anvil Architecture Constraints\n');

    // Metadata
    if (this.options.includeMetadata) {
      sections.push(this.formatMetadata(constraints));
    }

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

    return sections.join('\n');
  }

  /**
   * Format metadata section
   */
  private formatMetadata(constraints: Constraints): string {
    const lines: string[] = [
      '> **Generated:** ' + new Date(constraints.metadata.collectedAt).toLocaleString(),
      '> **Workspace:** ' + constraints.metadata.workspaceRoot,
      '> **Has Baseline:** ' + (constraints.metadata.hasBaseline ? 'Yes' : 'No'),
      '',
    ];

    return lines.join('\n');
  }

  /**
   * Format architecture boundaries section
   */
  private formatBoundaries(constraints: Constraints): string {
    const lines: string[] = ['## Boundary Rules\n'];

    lines.push(
      'These architectural boundaries must be respected. Violations will be flagged as warnings or errors.\n'
    );

    for (const boundary of constraints.boundaries) {
      const severityEmoji = this.getSeverityEmoji(boundary.severity);
      lines.push(`- ${severityEmoji} **${boundary.name}**`);
      lines.push(`  - From: \`${boundary.from}\``);
      lines.push(`  - To: \`${boundary.to}\``);
      lines.push(`  - Rule: ${boundary.message}`);
      lines.push('');
    }

    return lines.join('\n');
  }

  /**
   * Format layer definitions section
   */
  private formatLayers(constraints: Constraints): string {
    const lines: string[] = ['## Layer Definitions\n'];

    lines.push(
      'The codebase is organised into architectural layers. Each layer has specific responsibilities and dependencies.\n'
    );

    for (const layer of constraints.layers) {
      lines.push(`### ${layer.name}\n`);

      if (layer.description) {
        lines.push(`${layer.description}\n`);
      }

      lines.push('**Patterns:**');
      for (const pattern of layer.patterns) {
        lines.push(`- \`${pattern}\``);
      }
      lines.push('');

      if (layer.dependsOn.length > 0) {
        lines.push('**Can depend on:**');
        for (const dep of layer.dependsOn) {
          lines.push(`- ${dep}`);
        }
      } else {
        lines.push('**Dependencies:** None (leaf layer)');
      }

      lines.push('');
    }

    return lines.join('\n');
  }

  /**
   * Format anti-patterns section
   */
  private formatAntiPatterns(constraints: Constraints): string {
    const lines: string[] = ['## Anti-patterns (Blocked)\n'];

    lines.push(
      'These code patterns are considered anti-patterns and should be avoided. ' +
        'Anvil will flag them during code review.\n'
    );

    // Group by category
    const byCategory = new Map<string, typeof constraints.antiPatterns>();
    for (const pattern of constraints.antiPatterns) {
      const category = pattern.category;
      if (!byCategory.has(category)) {
        byCategory.set(category, []);
      }
      byCategory.get(category)!.push(pattern);
    }

    // Format each category
    for (const [category, patterns] of byCategory) {
      lines.push(`### ${this.formatCategoryName(category)}\n`);

      for (const pattern of patterns) {
        const severityEmoji = this.getSeverityEmoji(pattern.severity);
        lines.push(`#### ${severityEmoji} ${pattern.name} (\`${pattern.id}\`)\n`);
        lines.push(`**Why it's problematic:** ${pattern.explanation}\n`);
        lines.push(`**What to do instead:** ${pattern.suggestion}\n`);
      }
    }

    return lines.join('\n');
  }

  /**
   * Format conventions section
   */
  private formatConventions(constraints: Constraints): string {
    const lines: string[] = ['## Conventions\n'];

    lines.push(
      'These conventions should be followed throughout the codebase for consistency and maintainability.\n'
    );

    for (const convention of constraints.conventions) {
      lines.push(`### ${this.formatCategoryName(convention.category)}\n`);
      lines.push(`${convention.description}\n`);

      if (convention.examples && convention.examples.length > 0) {
        lines.push('**Examples:**');
        for (const example of convention.examples) {
          lines.push(`- ${example}`);
        }
        lines.push('');
      }
    }

    return lines.join('\n');
  }

  /**
   * Format active suppressions section
   */
  private formatSuppressions(constraints: Constraints): string {
    const lines: string[] = ['## Active Suppressions\n'];

    lines.push(
      'These patterns are intentionally suppressed in specific locations. ' +
        'Do not flag or attempt to fix these.\n'
    );

    // Group by pattern ID
    const byPattern = new Map<string, typeof constraints.suppressions>();
    for (const suppression of constraints.suppressions) {
      if (!byPattern.has(suppression.patternId)) {
        byPattern.set(suppression.patternId, []);
      }
      byPattern.get(suppression.patternId)!.push(suppression);
    }

    for (const [patternId, suppressions] of byPattern) {
      lines.push(`### \`${patternId}\`\n`);

      for (const suppression of suppressions) {
        lines.push(`- **\`${suppression.file}\`** (${suppression.scope})`);
        lines.push(`  - Reason: ${suppression.reason}`);
        if (suppression.expiresAt) {
          lines.push(`  - Expires: ${new Date(suppression.expiresAt).toLocaleDateString()}`);
        }
        lines.push('');
      }
    }

    return lines.join('\n');
  }

  /**
   * Get emoji for severity level
   */
  private getSeverityEmoji(severity: string): string {
    switch (severity) {
      case 'error':
        return '🚫';
      case 'warning':
        return '⚠️';
      case 'info':
        return 'ℹ️';
      default:
        return '•';
    }
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
 * Format constraints as llms.txt with default options
 *
 * @param constraints - Constraints to format
 * @returns llms.txt markdown string
 */
export function formatAsLlmsTxt(constraints: Constraints): string {
  const formatter = new LlmsTxtFormatter();
  return formatter.format(constraints);
}

/**
 * Format constraints as llms.txt without metadata
 *
 * @param constraints - Constraints to format
 * @returns llms.txt markdown string
 */
export function formatAsLlmsTxtWithoutMetadata(constraints: Constraints): string {
  const formatter = new LlmsTxtFormatter({ includeMetadata: false });
  return formatter.format(constraints);
}
