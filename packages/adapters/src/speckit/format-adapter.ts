/**
 * SpecKit Format Adapter
 *
 * FormatAdapter implementation for GitHub spec-kit format.
 * Handles spec.md, plan.md, and tasks.md documents.
 *
 * This adapter wraps the existing SpecKitImportAdapterV2 and SpecKitExportAdapter
 * to provide the unified FormatAdapter interface with format detection.
 */

import { generateHash, type APSPlan, type ValidationResult } from '@anvil/core';
import {
  BaseFormatAdapter,
  type AdapterMetadata,
  type DetectionResult,
  type ParseResult,
  type SerializeResult,
  type ParseContext,
  type AdapterOptions,
  type ExternalSpec,
} from '../base/types.js';
import { createDetection } from '../base/utils.js';
import { SpecKitImportAdapterV2 } from './import-v2.js';
import { SpecKitExportAdapter } from './export.js';

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

  private importAdapter: SpecKitImportAdapterV2;
  private exportAdapter: SpecKitExportAdapter;

  constructor(options?: AdapterOptions) {
    super(options);
    this.importAdapter = new SpecKitImportAdapterV2();
    this.exportAdapter = new SpecKitExportAdapter();
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
      // Create ExternalSpec from content in the format expected by SpecKitImportAdapterV2
      const externalSpec: ExternalSpec = {
        format: 'speckit',
        version: '1.0.0',
        content: {
          spec: {
            content: content,
          },
        },
        metadata: {
          filePath: context?.filePath,
        },
      };

      // Convert to APS using import adapter
      const conversionResult = await this.importAdapter.convertToAPS(externalSpec);

      if (!conversionResult.success) {
        return this.createParseError(
          conversionResult.errors?.map((err) => ({
            code: err.code,
            message: err.message,
            details: err.details,
          })) || [{ code: 'PARSE_ERROR', message: 'Failed to parse SpecKit content' }]
        );
      }

      // Apply context overrides
      let plan = conversionResult.data!;

      if (context) {
        plan = {
          ...plan,
          provenance: {
            ...plan.provenance,
            author: context.author || plan.provenance.author,
            repository: context.repositoryPath || plan.provenance.repository,
            branch: context.branch || plan.provenance.branch,
            commit: context.commit || plan.provenance.commit,
            timestamp: context.timestamp || plan.provenance.timestamp,
          },
        };
      }

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
   * Serialize APS plan to SpecKit format
   *
   * @param plan - APS plan to serialize
   * @param options - Adapter options
   * @returns Serialize result with SpecKit markdown
   */
  async serialize(plan: APSPlan, _options?: AdapterOptions): Promise<SerializeResult> {
    try {
      // Convert from APS using export adapter
      const conversionResult = await this.exportAdapter.convertFromAPS(plan);

      if (!conversionResult.success) {
        return this.createSerializeError(
          conversionResult.errors?.map((err) => ({
            code: err.code,
            message: err.message,
            details: err.details,
          })) || [{ code: 'SERIALIZE_ERROR', message: 'Failed to serialize to SpecKit format' }]
        );
      }

      // Extract spec.md content
      const externalSpec = conversionResult.data!;
      const content =
        typeof externalSpec.content === 'string'
          ? externalSpec.content
          : (externalSpec.content as { specContent?: string }).specContent || '';

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
