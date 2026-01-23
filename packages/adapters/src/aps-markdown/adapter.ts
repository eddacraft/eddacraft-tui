/**
 * APS Markdown Format Adapter
 *
 * FormatAdapter implementation for Anvil Plan Spec (APS) markdown format.
 * Handles both leaf specs (.aps.md with Tasks section) and index files
 * (.aps.md with Modules section).
 */

import type { APSPlan, ValidationResult } from '@eddacraft/anvil-core';
import {
  BaseFormatAdapter,
  type AdapterMetadata,
  type DetectionResult,
  type ParseResult,
  type SerializeResult,
  type ParseContext,
  type AdapterOptions,
} from '../base/types.js';
import { createDetection } from '../base/utils.js';

/**
 * Detection indicators for APS markdown format
 */
interface APSMarkdownIndicators {
  /** Has H1 title */
  hasH1Title: boolean;
  /** Has **Scope:** field */
  hasScopeField: boolean;
  /** Has ## Tasks section */
  hasTasksSection: boolean;
  /** Has ## Modules section */
  hasModulesSection: boolean;
  /** Has SCOPE-NNN task ID pattern (e.g., AUTH-001) */
  hasScopeTaskPattern: boolean;
  /** Has **Intent:** field in tasks */
  hasIntentField: boolean;
  /** Has **Path:** with .aps.md links */
  hasAPSModuleLinks: boolean;
  /** Has **Confidence:** field */
  hasConfidenceField: boolean;
  /** Has **Owner:** field */
  hasOwnerField: boolean;
  /** Has **Priority:** field */
  hasPriorityField: boolean;
  /** Count of SCOPE-NNN patterns found */
  taskPatternCount: number;
  /** Count of .aps.md links found */
  apsLinkCount: number;
}

/**
 * APS Markdown FormatAdapter implementation
 *
 * Converts between APS markdown documents and APS plans.
 */
export class APSMarkdownAdapter extends BaseFormatAdapter {
  readonly metadata: AdapterMetadata = {
    name: 'aps-markdown',
    version: '1.0.0',
    displayName: 'Anvil Plan Spec Markdown',
    description: 'APS markdown format adapter for plan specs (.aps.md)',
    formats: ['aps', 'aps-markdown', 'aps.md'],
    extensions: ['.aps.md'],
  };

  /**
   * Detect if content is APS markdown format
   *
   * Uses confidence scoring based on multiple indicators:
   * - Tasks section with SCOPE-NNN headings (30 points)
   * - **Intent:** field in tasks (20 points)
   * - Modules section (25 points)
   * - .aps.md path links (20 points)
   * - **Scope:** field (10 points)
   * - **Confidence:** field (10 points)
   * - **Owner:** field (5 points)
   * - **Priority:** field (5 points)
   *
   * @param content - Document content to analyze
   * @returns Detection result with confidence score
   */
  detect(content: string): DetectionResult {
    const indicators = this.analyzeContent(content);
    const confidence = this.calculateConfidence(indicators);
    const reason = this.buildDetectionReason(indicators);

    // Detection threshold: 50% confidence
    return createDetection(confidence >= 50, confidence, reason);
  }

  /**
   * Parse APS markdown content to APS plan
   *
   * NOTE: Not yet implemented - returns NOT_IMPLEMENTED error.
   *
   * @param _content - APS markdown content
   * @param _context - Parse context for provenance
   * @param _options - Adapter options
   * @returns Parse result with APS plan
   */
  async parse(
    _content: string,
    _context?: ParseContext,
    _options?: AdapterOptions
  ): Promise<ParseResult> {
    return this.createParseError([
      {
        code: 'NOT_IMPLEMENTED',
        message: 'APSMarkdownAdapter.parse() is not yet implemented',
      },
    ]);
  }

  /**
   * Serialize APS plan to APS markdown format
   *
   * NOTE: Not yet implemented - returns NOT_IMPLEMENTED error.
   *
   * @param _plan - APS plan to serialize
   * @param _options - Adapter options
   * @returns Serialize result with APS markdown
   */
  async serialize(_plan: APSPlan, _options?: AdapterOptions): Promise<SerializeResult> {
    return this.createSerializeError([
      {
        code: 'NOT_IMPLEMENTED',
        message: 'APSMarkdownAdapter.serialize() is not yet implemented',
      },
    ]);
  }

  /**
   * Validate APS markdown content
   *
   * NOTE: Not yet implemented - returns NOT_IMPLEMENTED error.
   *
   * @param _content - APS markdown content to validate
   * @param _options - Validation options
   * @returns Validation result
   */
  async validate(_content: string, _options?: AdapterOptions): Promise<ValidationResult> {
    return {
      valid: false,
      issues: [
        {
          code: 'NOT_IMPLEMENTED',
          path: '',
          message: 'APSMarkdownAdapter.validate() is not yet implemented',
          severity: 'error',
        },
      ],
      summary: 'Validation not yet implemented',
    };
  }

  /**
   * Override canImport to handle the compound extension .aps.md
   */
  canImport(format: string): boolean {
    const normalized = format.toLowerCase().replace(/^\./, '');
    return (
      this.metadata.formats.includes(normalized) ||
      this.metadata.extensions.some((ext) => ext.replace(/^\./, '') === normalized) ||
      normalized === 'aps.md'
    );
  }

  /**
   * Analyze content for APS markdown indicators
   */
  private analyzeContent(content: string): APSMarkdownIndicators {
    // Check for H1 title
    const hasH1Title = /^#\s+.+$/m.test(content);

    // Check for **Scope:** field (inline format: **Scope:** VALUE)
    const hasScopeField = /\*\*Scope:\*\*\s*\S+/i.test(content);

    // Check for ## Tasks section
    const hasTasksSection = /^##\s+Tasks\s*$/im.test(content);

    // Check for ## Modules section
    const hasModulesSection = /^##\s+Modules\s*$/im.test(content);

    // Check for SCOPE-NNN pattern in ### headings (e.g., ### AUTH-001: Description)
    const scopeTaskPattern = /^###\s+[A-Z]{2,10}-\d{3}:/gm;
    const taskPatternMatches = content.match(scopeTaskPattern) || [];
    const hasScopeTaskPattern = taskPatternMatches.length > 0;
    const taskPatternCount = taskPatternMatches.length;

    // Check for **Intent:** field
    const hasIntentField = /\*\*Intent:\*\*/i.test(content);

    // Check for **Path:** with .aps.md links
    const apsLinkPattern = /\*\*Path:\*\*\s*\[.*?\]\([^)]*\.aps\.md\)/gi;
    const apsLinkMatches = content.match(apsLinkPattern) || [];
    const hasAPSModuleLinks = apsLinkMatches.length > 0;
    const apsLinkCount = apsLinkMatches.length;

    // Check for **Confidence:** field
    const hasConfidenceField = /\*\*Confidence:\*\*/i.test(content);

    // Check for **Owner:** field
    const hasOwnerField = /\*\*Owner:\*\*/i.test(content);

    // Check for **Priority:** field
    const hasPriorityField = /\*\*Priority:\*\*/i.test(content);

    return {
      hasH1Title,
      hasScopeField,
      hasTasksSection,
      hasModulesSection,
      hasScopeTaskPattern,
      hasIntentField,
      hasAPSModuleLinks,
      hasConfidenceField,
      hasOwnerField,
      hasPriorityField,
      taskPatternCount,
      apsLinkCount,
    };
  }

  /**
   * Calculate confidence score based on indicators
   */
  private calculateConfidence(indicators: APSMarkdownIndicators): number {
    let score = 0;

    // Leaf spec indicators (Tasks section path)
    if (indicators.hasTasksSection) {
      score += 15; // Has ## Tasks section

      if (indicators.hasScopeTaskPattern) {
        score += 30; // Has SCOPE-NNN task IDs
        // Bonus for multiple tasks
        if (indicators.taskPatternCount > 1) {
          score += 5;
        }
      }

      if (indicators.hasIntentField) {
        score += 20; // Has **Intent:** in tasks
      }
    }

    // Index file indicators (Modules section path)
    if (indicators.hasModulesSection) {
      score += 20; // Has ## Modules section

      if (indicators.hasAPSModuleLinks) {
        score += 25; // Has .aps.md path links
        // Bonus for multiple modules
        if (indicators.apsLinkCount > 1) {
          score += 5;
        }
      }
    }

    // Common APS metadata fields
    if (indicators.hasScopeField) {
      score += 10; // Has **Scope:** field
    }

    if (indicators.hasConfidenceField) {
      score += 10; // Has **Confidence:** field
    }

    if (indicators.hasOwnerField) {
      score += 5; // Has **Owner:** field
    }

    if (indicators.hasPriorityField) {
      score += 5; // Has **Priority:** field
    }

    return Math.min(100, score);
  }

  /**
   * Build detection reason message
   */
  private buildDetectionReason(indicators: APSMarkdownIndicators): string {
    const reasons: string[] = [];

    if (indicators.hasTasksSection) {
      reasons.push('tasks-section');
    }
    if (indicators.hasScopeTaskPattern) {
      reasons.push(`scope-tasks(${indicators.taskPatternCount})`);
    }
    if (indicators.hasIntentField) {
      reasons.push('intent-field');
    }
    if (indicators.hasModulesSection) {
      reasons.push('modules-section');
    }
    if (indicators.hasAPSModuleLinks) {
      reasons.push(`aps-links(${indicators.apsLinkCount})`);
    }
    if (indicators.hasScopeField) {
      reasons.push('scope-field');
    }
    if (indicators.hasConfidenceField) {
      reasons.push('confidence-field');
    }
    if (indicators.hasOwnerField) {
      reasons.push('owner-field');
    }
    if (indicators.hasPriorityField) {
      reasons.push('priority-field');
    }

    return reasons.length > 0 ? reasons.join(', ') : 'no APS indicators';
  }
}

/**
 * Create a new APS markdown adapter instance
 *
 * @param options - Adapter options
 * @returns APS markdown adapter instance
 */
export function createAPSMarkdownAdapter(options?: AdapterOptions): APSMarkdownAdapter {
  return new APSMarkdownAdapter(options);
}
