/**
 * BMAD Format Adapter
 *
 * FormatAdapter implementation for BMAD (Breakthrough Method for Agile AI-Driven Development) format.
 * Handles PRD, Architecture, Epic, and Story documents.
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
import { parseBMAD } from './parser.js';
import { serializeToBMAD } from './serializer.js';

/**
 * BMAD FormatAdapter implementation
 *
 * Converts between BMAD format documents and APS plans.
 */
export class BMADFormatAdapter extends BaseFormatAdapter {
  readonly metadata: AdapterMetadata = {
    name: 'bmad',
    version: '1.0.0',
    displayName: 'BMAD (Breakthrough Method for Agile AI-Driven Development)',
    description: 'BMAD PRD and architecture document adapter',
    formats: ['bmad', 'prd', 'architecture'],
    extensions: ['.md'],
  };

  /**
   * Detect if content is BMAD format
   *
   * Uses confidence scoring based on multiple indicators:
   * - YAML front-matter (30 points)
   * - Requirement identifiers FR/NFR/US (25 points)
   * - User story format (20 points)
   * - Change log table (15 points)
   * - Document title (10 points)
   *
   * @param content - Document content to analyze
   * @returns Detection result with confidence score
   */
  detect(content: string): DetectionResult {
    const indicators = analyzeContent(content);
    const confidence = calculateConfidenceScore(indicators);
    const reason = buildDetectionReason(indicators);

    // Detection threshold: 50% confidence
    return createDetection(confidence >= 50, confidence, reason);
  }

  /**
   * Parse BMAD content to APS plan
   *
   * @param content - BMAD markdown content
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
      const plan = parseBMAD(content, context);

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
          message: error instanceof Error ? error.message : 'Failed to parse BMAD content',
          details: error,
        },
      ]);
    }
  }

  /**
   * Serialize APS plan to BMAD format
   *
   * @param plan - APS plan to serialize
   * @param options - Adapter options
   * @returns Serialize result with BMAD markdown
   */
  async serialize(plan: APSPlan, _options?: AdapterOptions): Promise<SerializeResult> {
    try {
      const content = serializeToBMAD(plan);
      return this.createSerializeSuccess(content);
    } catch (error) {
      return this.createSerializeError([
        {
          code: 'SERIALIZE_ERROR',
          message: error instanceof Error ? error.message : 'Failed to serialize to BMAD format',
          details: error,
        },
      ]);
    }
  }

  /**
   * Validate BMAD content
   *
   * Checks for required BMAD elements without full conversion.
   *
   * @param content - BMAD content to validate
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
        message: 'Content is too short to be a valid BMAD document',
        severity: 'error',
      });
    }

    // Analyze content for BMAD indicators
    const indicators = analyzeContent(content);
    const confidence = calculateConfidenceScore(indicators);

    // Low confidence suggests invalid BMAD format
    if (confidence < 50) {
      issues.push({
        code: 'LOW_CONFIDENCE',
        path: 'content',
        message: `Content does not appear to be a valid BMAD document (confidence: ${confidence}%)`,
        severity: 'error',
      });
    }

    // Check for at least some requirements or stories
    if (indicators.requirementCount === 0) {
      issues.push({
        code: 'NO_REQUIREMENTS',
        path: 'content',
        message: 'No requirements (FR/NFR/US) found in document',
        severity: 'warning',
      });
    }

    return {
      valid: issues.length === 0,
      issues: issues.length > 0 ? issues : undefined,
      summary:
        issues.length === 0
          ? 'BMAD document is valid'
          : `Found ${issues.length} validation issue${issues.length > 1 ? 's' : ''}`,
    };
  }
}

/**
 * Create a new BMAD format adapter instance
 *
 * @param options - Adapter options
 * @returns BMAD adapter instance
 */
export function createBMADAdapter(options?: AdapterOptions): BMADFormatAdapter {
  return new BMADFormatAdapter(options);
}
