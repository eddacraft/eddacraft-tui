/**
 * Generic Markdown Format Adapter
 *
 * FormatAdapter implementation for generic markdown planning documents.
 * Serves as a fallback adapter for documents that don't match specific formats.
 */

import { generateHash, type APSPlan, type ValidationResult } from '@eddacraft/anvil-core';
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
import { analyzeContent, calculateConfidenceScore, buildDetectionReason } from './utils.js';
import { parseGeneric } from './parser.js';
import { serializeToGeneric } from './serializer.js';

const MIN_CONTENT_LENGTH = 50;
const FALLBACK_DETECTION_THRESHOLD = 30;
const MAX_FALLBACK_CONFIDENCE = 45;

/**
 * Generic Markdown FormatAdapter implementation
 *
 * Converts between generic markdown documents and APS plans.
 * Designed to work as a fallback for documents that don't match
 * specific formats like BMAD or SpecKit.
 */
export class GenericMarkdownAdapter extends BaseFormatAdapter {
  readonly metadata: AdapterMetadata = {
    name: 'generic-markdown',
    version: '1.0.0',
    displayName: 'Generic Markdown',
    description: 'Generic markdown adapter for PRDs, plans, todos, and other planning documents',
    formats: ['generic', 'markdown', 'prd', 'plan', 'todo', 'rfc', 'adr'],
    extensions: ['.md', '.markdown'],
  };

  /**
   * Detect if content is generic markdown planning document
   *
   * Uses lower confidence threshold (30-40%) to serve as fallback.
   * This adapter should be registered last so specific adapters
   * (BMAD, SpecKit) take precedence.
   *
   * @param content - Document content to analyze
   * @returns Detection result with confidence score
   */
  detect(content: string): DetectionResult {
    const indicators = analyzeContent(content);
    const confidence = calculateConfidenceScore(indicators);
    const reason = buildDetectionReason(indicators);

    const cappedConfidence = Math.min(MAX_FALLBACK_CONFIDENCE, confidence);

    return createDetection(
      cappedConfidence >= FALLBACK_DETECTION_THRESHOLD,
      cappedConfidence,
      reason
    );
  }

  /**
   * Parse generic markdown content to APS plan
   *
   * @param content - Markdown content
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
      // Parse content to APS plan
      const plan = parseGeneric(content, context);

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
          message:
            error instanceof Error ? error.message : 'Failed to parse generic markdown content',
          details: error,
        },
      ]);
    }
  }

  /**
   * Serialize APS plan to generic markdown format
   *
   * @param plan - APS plan to serialize
   * @param options - Adapter options
   * @returns Serialize result with markdown content
   */
  async serialize(plan: APSPlan, _options?: AdapterOptions): Promise<SerializeResult> {
    try {
      const content = serializeToGeneric(plan);
      return this.createSerializeSuccess(content);
    } catch (error) {
      return this.createSerializeError([
        {
          code: 'SERIALIZE_ERROR',
          message:
            error instanceof Error
              ? error.message
              : 'Failed to serialize to generic markdown format',
          details: error,
        },
      ]);
    }
  }

  /**
   * Validate generic markdown content
   *
   * Checks for basic markdown structure without strict requirements.
   *
   * @param content - Content to validate
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

    if (content.trim().length < MIN_CONTENT_LENGTH) {
      issues.push({
        code: 'CONTENT_TOO_SHORT',
        path: 'content',
        message: 'Content is too short to be a valid planning document',
        severity: 'error',
      });
    }

    const indicators = analyzeContent(content);
    const confidence = calculateConfidenceScore(indicators);

    if (confidence < FALLBACK_DETECTION_THRESHOLD) {
      issues.push({
        code: 'LOW_CONFIDENCE',
        path: 'content',
        message: `Content does not appear to be a planning document (confidence: ${confidence}%)`,
        severity: 'error',
      });
    }

    // Warn if missing common planning sections
    if (
      !indicators.hasRequirementsSection &&
      !indicators.hasTasksSection &&
      !indicators.hasFeaturesSection
    ) {
      issues.push({
        code: 'NO_PLANNING_SECTIONS',
        path: 'content',
        message: 'Document lacks common planning sections (requirements, tasks, or features)',
        severity: 'warning',
      });
    }

    return {
      valid: issues.filter((i) => i.severity === 'error').length === 0,
      issues: issues.length > 0 ? issues : undefined,
      summary:
        issues.length === 0
          ? 'Generic markdown document is valid'
          : `Found ${issues.length} validation issue${issues.length > 1 ? 's' : ''}`,
    };
  }
}

/**
 * Create a new generic markdown adapter instance
 *
 * @param options - Adapter options
 * @returns Generic markdown adapter instance
 */
export function createGenericMarkdownAdapter(options?: AdapterOptions): GenericMarkdownAdapter {
  return new GenericMarkdownAdapter(options);
}
