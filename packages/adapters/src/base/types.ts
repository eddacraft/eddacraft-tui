/**
 * Base Adapter Framework Types
 *
 * Defines the core interfaces for format adapters that convert between
 * external planning formats (SpecKit, BMAD, etc.) and APS.
 */

import type { APSPlan, ValidationResult } from '@eddacraft/anvil-core';

/**
 * Result of format detection
 */
export interface DetectionResult {
  /** Whether this adapter can handle the content */
  detected: boolean;
  /** Confidence score (0-100) */
  confidence: number;
  /** Reason for detection result */
  reason?: string;
}

/**
 * Result of parsing external format to APS
 */
export interface ParseResult {
  /** Whether parsing succeeded */
  success: boolean;
  /** Parsed APS plan (if successful) */
  data?: APSPlan;
  /** Errors encountered during parsing */
  errors?: AdapterError[];
  /** Non-fatal warnings */
  warnings?: AdapterWarning[];
}

/**
 * Result of serializing APS to external format
 */
export interface SerializeResult {
  /** Whether serialization succeeded */
  success: boolean;
  /** Serialized content (if successful) */
  content?: string;
  /** Errors encountered during serialization */
  errors?: AdapterError[];
  /** Non-fatal warnings */
  warnings?: AdapterWarning[];
  /** Additional metadata about serialization */
  metadata?: Record<string, unknown>;
}

/**
 * Error encountered during adapter operation
 */
export interface AdapterError {
  /** Error code for programmatic handling */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Path to the problematic element (e.g., "changes[0].path") */
  path?: string;
  /** Line number in source content (if applicable) */
  line?: number;
  /** Column number in source content (if applicable) */
  column?: number;
  /** Additional error details */
  details?: unknown;
}

/**
 * Warning encountered during adapter operation
 */
export interface AdapterWarning {
  /** Warning code for programmatic handling */
  code: string;
  /** Human-readable warning message */
  message: string;
  /** Path to the element (e.g., "changes[0].description") */
  path?: string;
  /** Line number in source content (if applicable) */
  line?: number;
  /** Column number in source content (if applicable) */
  column?: number;
  /** Additional warning details */
  details?: unknown;
}

/**
 * Context provided when parsing external formats
 */
export interface ParseContext {
  /** Repository path or URL */
  repositoryPath?: string;
  /** Git branch */
  branch?: string;
  /** Git commit hash */
  commit?: string;
  /** Author information */
  author?: string;
  /** Timestamp for provenance */
  timestamp?: string;
  /** Additional context metadata */
  metadata?: Record<string, unknown>;
  /** Pre-generated plan ID (for deterministic parsing) */
  planId?: string;
}

/**
 * Options for adapter operations
 */
export interface AdapterOptions {
  /** Preserve comments from source */
  preserveComments?: boolean;
  /** Preserve metadata from source */
  preserveMetadata?: boolean;
  /** Strict validation mode */
  strict?: boolean;
  /** Format-specific options */
  formatOptions?: Record<string, unknown>;
}

/**
 * Metadata about an adapter
 */
export interface AdapterMetadata {
  /** Adapter name (e.g., "speckit", "bmad") */
  name: string;
  /** Adapter version */
  version: string;
  /** Human-readable display name */
  displayName: string;
  /** Short description of what this adapter handles */
  description: string;
  /** File extensions supported (e.g., [".md", ".spec.md"]) */
  extensions: readonly string[];
  /** Format identifiers (e.g., ["speckit", "spec"]) */
  formats: readonly string[];
}

/**
 * Core interface for format adapters
 *
 * Adapters convert between external planning formats and APS.
 * Each adapter should handle one external format (e.g., SpecKit, BMAD).
 */
export interface FormatAdapter {
  /** Adapter metadata */
  readonly metadata: AdapterMetadata;

  /**
   * Detect if this adapter can handle the given content
   *
   * @param content - Raw content to analyze
   * @returns Detection result with confidence score
   */
  detect(content: string): DetectionResult;

  /**
   * Parse external format content into APS
   *
   * @param content - Raw content in external format
   * @param context - Context for provenance tracking
   * @param options - Parsing options
   * @returns Parse result with APS plan or errors
   */
  parse(content: string, context?: ParseContext, options?: AdapterOptions): Promise<ParseResult>;

  /**
   * Serialize APS plan to external format
   *
   * @param plan - APS plan to serialize
   * @param options - Serialization options
   * @returns Serialize result with content or errors
   */
  serialize(plan: APSPlan, options?: AdapterOptions): Promise<SerializeResult>;

  /**
   * Validate external format content
   *
   * Validates the content without full conversion to APS.
   * Useful for fast validation feedback.
   *
   * @param content - Raw content to validate
   * @param options - Validation options
   * @returns Validation result
   */
  validate(content: string, options?: AdapterOptions): Promise<ValidationResult>;

  /**
   * Check if this adapter can import from a given format
   *
   * @param format - Format identifier or file extension
   * @returns True if adapter supports importing from this format
   */
  canImport(format: string): boolean;

  /**
   * Check if this adapter can export to a given format
   *
   * @param format - Format identifier or file extension
   * @returns True if adapter supports exporting to this format
   */
  canExport(format: string): boolean;
}

/**
 * Abstract base class for format adapters
 *
 * Provides common functionality and structure for concrete adapters.
 */
export abstract class BaseFormatAdapter implements FormatAdapter {
  abstract readonly metadata: AdapterMetadata;

  constructor(protected options: AdapterOptions = {}) {}

  abstract detect(content: string): DetectionResult;
  abstract parse(
    content: string,
    context?: ParseContext,
    options?: AdapterOptions
  ): Promise<ParseResult>;
  abstract serialize(plan: APSPlan, options?: AdapterOptions): Promise<SerializeResult>;
  abstract validate(content: string, options?: AdapterOptions): Promise<ValidationResult>;

  canImport(format: string): boolean {
    const normalized = format.toLowerCase().replace(/^\./, '');
    return (
      this.metadata.formats.includes(normalized) ||
      this.metadata.extensions.some((ext) => ext.replace(/^\./, '') === normalized)
    );
  }

  canExport(format: string): boolean {
    return this.canImport(format);
  }

  /**
   * Helper to create a successful parse result
   */
  protected createParseSuccess(data: APSPlan, warnings?: AdapterWarning[]): ParseResult {
    return {
      success: true,
      data,
      warnings: warnings && warnings.length > 0 ? warnings : undefined,
    };
  }

  /**
   * Helper to create a failed parse result
   */
  protected createParseError(errors: AdapterError[], warnings?: AdapterWarning[]): ParseResult {
    return {
      success: false,
      errors,
      warnings: warnings && warnings.length > 0 ? warnings : undefined,
    };
  }

  /**
   * Helper to create a successful serialize result
   */
  protected createSerializeSuccess(
    content: string,
    warnings?: AdapterWarning[],
    metadata?: Record<string, unknown>
  ): SerializeResult {
    return {
      success: true,
      content,
      warnings: warnings && warnings.length > 0 ? warnings : undefined,
      metadata,
    };
  }

  /**
   * Helper to create a failed serialize result
   */
  protected createSerializeError(
    errors: AdapterError[],
    warnings?: AdapterWarning[]
  ): SerializeResult {
    return {
      success: false,
      errors,
      warnings: warnings && warnings.length > 0 ? warnings : undefined,
    };
  }

  /**
   * Helper to add an error
   */
  protected addError(
    errors: AdapterError[],
    code: string,
    message: string,
    path?: string,
    details?: unknown
  ): void {
    errors.push({ code, message, path, details });
  }

  /**
   * Helper to add a warning
   */
  protected addWarning(
    warnings: AdapterWarning[],
    code: string,
    message: string,
    path?: string,
    details?: unknown
  ): void {
    warnings.push({ code, message, path, details });
  }
}
