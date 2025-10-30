/**
 * SpecKit Format Adapter
 *
 * FormatAdapter implementation for GitHub spec-kit format.
 * Handles simple SpecKit specification documents with Intent, Overview, Goals,
 * Requirements, and Changes sections.
 *
 * Uses SpecKitParser to parse markdown with ## sections and convert to/from APS format.
 */

import {
  generateHash,
  type APSPlan,
  type ValidationResult,
  type Change,
  createPlan,
} from '@anvil/core';
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
import { SpecKitParser } from './parser.js';

/**
 * Detection indicators for SpecKit format
 */
interface SpecKitIndicators {
  hasSpecificationHeader: boolean;
  hasIntentSection: boolean;
  hasOverviewSection: boolean;
  hasGoalsSection: boolean;
  hasRequirementsSection: boolean;
  hasChangesSection: boolean;
  hasFilesToCreateSection: boolean;
  hasFilesToUpdateSection: boolean;
  hasCodeBlocks: boolean;
  sectionCount: number;
}

/**
 * SpecKit FormatAdapter implementation
 *
 * Converts between SpecKit format documents and APS plans.
 */
export class SpecKitFormatAdapter extends BaseFormatAdapter {
  readonly metadata: AdapterMetadata = {
    name: 'speckit',
    version: '2.0.0',
    displayName: 'GitHub SpecKit',
    description: 'GitHub spec-kit format adapter (spec.md, plan.md, tasks.md)',
    formats: ['speckit', 'spec-kit', 'spec.md', 'plan.md', 'tasks.md'],
    extensions: ['.md'],
  };

  private parser: SpecKitParser;

  constructor(options?: AdapterOptions) {
    super(options);
    this.parser = new SpecKitParser();
  }

  /**
   * Detect if content is SpecKit format
   *
   * Uses confidence scoring based on multiple indicators:
   * - Specification header (20 points)
   * - Intent section (15 points)
   * - Overview section (10 points)
   * - Goals section (10 points)
   * - Requirements section (10 points)
   * - Changes section (20 points)
   * - Files to Create/Update sections (10 points)
   * - Code blocks (5 points)
   *
   * @param content - Document content to analyze
   * @returns Detection result with confidence score
   */
  detect(content: string): DetectionResult {
    const indicators = this.analyzeContent(content);
    const confidence = this.calculateConfidence(indicators);
    const reason = this.buildDetectionReason(indicators);

    // Detection threshold: 50% confidence
    // Lower threshold than BMAD to accommodate minimal SpecKit documents
    return createDetection(confidence >= 50, confidence, reason);
  }

  /**
   * Parse SpecKit content to APS plan
   *
   * @param content - SpecKit markdown content
   * @param context - Parse context for provenance
   * @param options - Adapter options
   * @returns Parse result with APS plan
   */
  async parse(
    content: string,
    context?: ParseContext,
    _options?: AdapterOptions
  ): Promise<ParseResult> {
    try {
      // Parse SpecKit markdown using SpecKitParser
      const parsed = this.parser.parseSpecMarkdown(content);

      // Build intent from parsed content
      const intent = parsed.intent || 'Implement Feature';

      // Build proposed changes from parsed changes
      const changes: Change[] = [];
      if (parsed.changes && parsed.changes.length > 0) {
        for (const change of parsed.changes) {
          const changeType = this.inferChangeType(change.type);
          changes.push({
            type: changeType,
            path: change.path || this.inferPathFromDescription(change.description),
            description: change.description,
            content: change.content,
          });
        }
      }

      // Build provenance
      const provenance = {
        timestamp: context?.timestamp || new Date().toISOString(),
        source: 'cli' as const,
        version: this.metadata.version,
        author: context?.author,
        repository: context?.repositoryPath,
        branch: context?.branch,
        commit: context?.commit,
      };

      // Generate plan ID
      const planId = `aps-${Date.now().toString(16).substring(0, 8)}`;

      // Create APS plan
      const plan: APSPlan = {
        ...createPlan({
          id: planId,
          intent,
          provenance,
          changes,
        }),
        schema_version: '0.1.0' as const,
        hash: '0'.repeat(64), // Temporary, will be replaced
        metadata: {
          source_format: 'speckit',
          overview: parsed.overview,
          goals: parsed.goals,
          requirements: parsed.requirements,
          ...parsed.metadata,
        },
      };

      // Generate hash for the plan
      const planWithHash = {
        ...plan,
        hash: generateHash(plan),
      };

      return this.createParseSuccess(planWithHash);
    } catch (error) {
      return this.createParseError([
        {
          code: 'PARSE_ERROR',
          message: error instanceof Error ? error.message : 'Failed to parse SpecKit content',
          details: error,
        },
      ]);
    }
  }

  /**
   * Infer APS change type from SpecKit change type
   */
  private inferChangeType(type: string): Change['type'] {
    const typeLower = type.toLowerCase();
    if (typeLower.includes('create')) return 'file_create';
    if (typeLower.includes('update') || typeLower.includes('modify')) return 'file_update';
    if (typeLower.includes('delete') || typeLower.includes('remove')) return 'file_delete';
    return 'file_update'; // Default to update
  }

  /**
   * Infer file path from change description
   */
  private inferPathFromDescription(description: string): string {
    // Try to extract path from common patterns like "at path/to/file" or "`path/to/file`"
    const pathMatch =
      description.match(/at\s+`([^`]+)`/) ||
      description.match(/at\s+(\S+\.\w+)/) ||
      description.match(/`([^`]+\.\w+)`/);

    if (pathMatch) {
      return pathMatch[1];
    }

    // Fallback: generate a generic path
    return 'src/generated-file.ts';
  }

  /**
   * Serialize APS plan to SpecKit format
   *
   * @param plan - APS plan to serialize
   * @param options - Adapter options
   * @returns Serialize result with SpecKit markdown
   */
  async serialize(plan: APSPlan, _options?: AdapterOptions): Promise<SerializeResult> {
    try {
      const sections: string[] = [];

      // Header
      sections.push('# Specification');
      sections.push('');

      // Intent section
      sections.push('## Intent');
      sections.push('');
      sections.push(plan.intent);
      sections.push('');

      // Overview section (if available in metadata)
      if (plan.metadata?.overview) {
        sections.push('## Overview');
        sections.push('');
        sections.push(plan.metadata.overview as string);
        sections.push('');
      }

      // Goals section (if available in metadata)
      if (plan.metadata?.goals && Array.isArray(plan.metadata.goals)) {
        sections.push('## Goals');
        sections.push('');
        for (const goal of plan.metadata.goals as string[]) {
          sections.push(`- ${goal}`);
        }
        sections.push('');
      }

      // Requirements section (if available in metadata)
      if (plan.metadata?.requirements && Array.isArray(plan.metadata.requirements)) {
        sections.push('## Requirements');
        sections.push('');
        for (const req of plan.metadata.requirements as string[]) {
          sections.push(`- ${req}`);
        }
        sections.push('');
      }

      // Changes section
      if (plan.proposed_changes.length > 0) {
        sections.push('## Changes');
        sections.push('');

        // Group changes by type
        const fileCreates = plan.proposed_changes.filter((c) => c.type === 'file_create');
        const fileUpdates = plan.proposed_changes.filter((c) => c.type === 'file_update');
        const fileDeletes = plan.proposed_changes.filter((c) => c.type === 'file_delete');

        // Files to Create
        if (fileCreates.length > 0) {
          sections.push('### Files to Create');
          sections.push('');
          for (const change of fileCreates) {
            sections.push(`#### Create ${change.path}`);
            sections.push('');
            sections.push(change.description || 'No description provided');
            sections.push('');
            if (change.content) {
              sections.push('```typescript');
              sections.push(change.content);
              sections.push('```');
              sections.push('');
            }
          }
        }

        // Files to Update
        if (fileUpdates.length > 0) {
          sections.push('### Files to Update');
          sections.push('');
          for (const change of fileUpdates) {
            sections.push(`#### Update ${change.path}`);
            sections.push('');
            sections.push(change.description || 'No description provided');
            sections.push('');
            if (change.content) {
              sections.push('```typescript');
              sections.push(change.content);
              sections.push('```');
              sections.push('');
            }
          }
        }

        // Files to Delete
        if (fileDeletes.length > 0) {
          sections.push('### Files to Delete');
          sections.push('');
          for (const change of fileDeletes) {
            sections.push(`#### Delete ${change.path}`);
            sections.push('');
            sections.push(change.description || 'No description provided');
            sections.push('');
          }
        }
      }

      // Metadata section (if additional metadata exists)
      const metadataKeys = Object.keys(plan.metadata || {}).filter(
        (k) => !['source_format', 'overview', 'goals', 'requirements'].includes(k)
      );
      if (metadataKeys.length > 0) {
        sections.push('## Metadata');
        sections.push('');
        sections.push('```json');
        const filteredMetadata: Record<string, unknown> = {};
        for (const key of metadataKeys) {
          filteredMetadata[key] = plan.metadata?.[key];
        }
        sections.push(JSON.stringify(filteredMetadata, null, 2));
        sections.push('```');
        sections.push('');
      }

      const content = sections.join('\n');
      return this.createSerializeSuccess(content);
    } catch (error) {
      return this.createSerializeError([
        {
          code: 'SERIALIZE_ERROR',
          message: error instanceof Error ? error.message : 'Failed to serialize to SpecKit format',
          details: error,
        },
      ]);
    }
  }

  /**
   * Validate SpecKit content
   *
   * Checks for required SpecKit elements without full conversion.
   *
   * @param content - SpecKit content to validate
   * @param options - Validation options
   * @returns Validation result
   */
  async validate(content: string, _options?: AdapterOptions): Promise<ValidationResult> {
    const issues: Array<{
      path: string;
      message: string;
      code: string;
      severity: 'error' | 'warning';
    }> = [];

    // Check for minimum content length
    if (content.trim().length < 100) {
      issues.push({
        code: 'CONTENT_TOO_SHORT',
        path: 'content',
        message: 'Content is too short to be a valid SpecKit document',
        severity: 'error',
      });
    }

    // Analyze content for SpecKit indicators
    const indicators = this.analyzeContent(content);
    const confidence = this.calculateConfidence(indicators);

    // Low confidence suggests invalid SpecKit format
    if (confidence < 50) {
      issues.push({
        code: 'LOW_CONFIDENCE',
        path: 'content',
        message: `Content does not appear to be a valid SpecKit document (confidence: ${confidence}%)`,
        severity: 'error',
      });
    }

    // Check for required sections
    if (!indicators.hasSpecificationHeader && !indicators.hasChangesSection) {
      issues.push({
        code: 'MISSING_REQUIRED_SECTIONS',
        path: 'content',
        message: 'Missing required sections (Specification header or Changes section)',
        severity: 'error',
      });
    }

    // Warn if missing recommended sections
    if (!indicators.hasIntentSection) {
      issues.push({
        code: 'MISSING_INTENT',
        path: 'content',
        message: 'Missing recommended Intent section',
        severity: 'warning',
      });
    }

    return {
      valid: issues.filter((i) => i.severity === 'error').length === 0,
      issues: issues.length > 0 ? issues : undefined,
      summary:
        issues.length === 0
          ? 'SpecKit document is valid'
          : `Found ${issues.length} validation issue${issues.length > 1 ? 's' : ''}`,
    };
  }

  /**
   * Analyze content for SpecKit indicators
   */
  private analyzeContent(content: string): SpecKitIndicators {
    const lowerContent = content.toLowerCase();

    return {
      hasSpecificationHeader: /^#\s+(specification|spec)\s*$/im.test(content),
      hasIntentSection: /^##\s+intent\s*$/im.test(content),
      hasOverviewSection: /^##\s+overview\s*$/im.test(content),
      hasGoalsSection: /^##\s+goals?\s*$/im.test(content),
      hasRequirementsSection: /^##\s+requirements?\s*$/im.test(content),
      hasChangesSection: /^##\s+changes?\s*$/im.test(content),
      hasFilesToCreateSection:
        lowerContent.includes('files to create') || lowerContent.includes('create file'),
      hasFilesToUpdateSection:
        lowerContent.includes('files to update') || lowerContent.includes('update file'),
      hasCodeBlocks: /```[\s\S]*?```/.test(content),
      sectionCount: (content.match(/^##\s+/gim) || []).length,
    };
  }

  /**
   * Calculate confidence score
   */
  private calculateConfidence(indicators: SpecKitIndicators): number {
    let score = 0;

    // Specification header (20 points)
    if (indicators.hasSpecificationHeader) {
      score += 20;
    }

    // Intent section (15 points)
    if (indicators.hasIntentSection) {
      score += 15;
    }

    // Overview section (10 points)
    if (indicators.hasOverviewSection) {
      score += 10;
    }

    // Goals section (10 points)
    if (indicators.hasGoalsSection) {
      score += 10;
    }

    // Requirements section (10 points)
    if (indicators.hasRequirementsSection) {
      score += 10;
    }

    // Changes section (20 points)
    if (indicators.hasChangesSection) {
      score += 20;
    }

    // Files to Create/Update sections (10 points)
    if (indicators.hasFilesToCreateSection || indicators.hasFilesToUpdateSection) {
      score += 10;
    }

    // Code blocks (5 points)
    if (indicators.hasCodeBlocks) {
      score += 5;
    }

    // Bonus: If has both Specification header AND Intent section, ensure at least 50% confidence
    // This accommodates minimal but valid SpecKit documents
    if (indicators.hasSpecificationHeader && indicators.hasIntentSection && score < 50) {
      score = 50;
    }

    return Math.min(100, score);
  }

  /**
   * Build detection reason message
   */
  private buildDetectionReason(indicators: SpecKitIndicators): string {
    const reasons: string[] = [];

    if (indicators.hasSpecificationHeader) {
      reasons.push('specification-header');
    }
    if (indicators.hasIntentSection) {
      reasons.push('intent-section');
    }
    if (indicators.hasGoalsSection) {
      reasons.push('goals-section');
    }
    if (indicators.hasRequirementsSection) {
      reasons.push('requirements-section');
    }
    if (indicators.hasChangesSection) {
      reasons.push('changes-section');
    }
    if (indicators.hasFilesToCreateSection || indicators.hasFilesToUpdateSection) {
      reasons.push('file-changes');
    }
    if (indicators.hasCodeBlocks) {
      reasons.push('code-blocks');
    }
    if (indicators.sectionCount >= 3) {
      reasons.push(`${indicators.sectionCount} sections`);
    }

    return reasons.length > 0 ? reasons.join(', ') : 'no strong indicators';
  }
}

/**
 * Create a new SpecKit format adapter instance
 *
 * @param options - Adapter options
 * @returns SpecKit adapter instance
 */
export function createSpecKitAdapter(options?: AdapterOptions): SpecKitFormatAdapter {
  return new SpecKitFormatAdapter(options);
}
