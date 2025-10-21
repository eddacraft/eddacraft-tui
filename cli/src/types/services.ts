/**
 * Service interface definitions for CLI adapter integration
 * @module cli/types/services
 */

import type {
  FormatAdapter,
  DetectionResult,
  AdapterError,
  AdapterWarning,
  AdapterOptions,
} from '@anvil/adapters';
import type { APSPlan, ValidationResult } from '@anvil/core';

/**
 * Format detection service that identifies the format of input files
 * and selects the appropriate adapter.
 */
export interface FormatDetectionService {
  /**
   * Detect format from file content
   * @param content - Raw file content
   * @param filePath - Optional file path for extension-based detection
   * @returns Detection result with selected adapter
   */
  detectFormat(content: string, filePath?: string): Promise<FormatDetectionResult | null>;

  /**
   * Detect format from file path
   * @param filePath - Absolute path to the file
   * @returns Detection result with selected adapter
   */
  detectFormatFromFile(filePath: string): Promise<FormatDetectionResult | null>;

  /**
   * Get all possible formats for given content
   * @param content - Raw file content
   * @returns Array of all matching adapters sorted by confidence
   */
  detectAllFormats(content: string): Promise<FormatDetectionResult[]>;
}

/**
 * Result of format detection
 */
export interface FormatDetectionResult {
  /** Detected format identifier */
  format: string;
  /** Selected adapter instance */
  adapter: FormatAdapter;
  /** Detection result from adapter */
  detection: DetectionResult;
  /** File path (if provided) */
  filePath?: string;
}

/**
 * Plan loader service that loads plans from any supported format
 */
export interface PlanLoaderService {
  /**
   * Load plan from file (APS or external format)
   * @param filePath - Path to plan file
   * @param options - Loading options
   * @returns Loaded plan with metadata
   */
  loadPlan(filePath: string, options?: LoadPlanOptions): Promise<LoadPlanResult>;

  /**
   * Load plan from content string
   * @param content - Plan content
   * @param options - Loading options
   * @returns Loaded plan with metadata
   */
  loadPlanFromContent(content: string, options?: LoadPlanOptions): Promise<LoadPlanResult>;
}

/**
 * Options for loading plans
 */
export interface LoadPlanOptions {
  /** Explicitly specify input format (bypasses auto-detection) */
  format?: string;
  /** Validate hash integrity */
  validateHash?: boolean;
  /** Use strict validation */
  strict?: boolean;
  /** Adapter-specific options */
  adapterOptions?: AdapterOptions;
}

/**
 * Result of plan loading
 */
export interface LoadPlanResult {
  /** Loaded APS plan */
  plan: APSPlan;
  /** Validation result */
  validation: ValidationResult;
  /** Source format information (if loaded from external format) */
  sourceFormat?: SourceFormatInfo;
  /** Warnings from parsing/conversion */
  warnings?: AdapterWarning[];
}

/**
 * Information about source format
 */
export interface SourceFormatInfo {
  /** Format identifier */
  format: string;
  /** Adapter name */
  adapter: string;
  /** Detection confidence (0-100) */
  confidence: number;
  /** File path */
  filePath?: string;
}

/**
 * Evidence injection service that writes gate results back to source documents
 */
export interface EvidenceInjectionService {
  /**
   * Inject evidence into source document
   * @param sourcePath - Path to source document
   * @param evidence - Evidence bundle from gate execution
   * @param adapter - Format adapter for the source document
   * @param options - Injection options
   * @returns Injection result
   */
  injectEvidence(
    sourcePath: string,
    evidence: unknown,
    adapter: FormatAdapter,
    options?: EvidenceInjectionOptions
  ): Promise<EvidenceInjectionResult>;

  /**
   * Preview evidence injection without writing to file
   * @param content - Source document content
   * @param evidence - Evidence bundle
   * @param adapter - Format adapter
   * @returns Preview of injected content
   */
  previewInjection(content: string, evidence: unknown, adapter: FormatAdapter): Promise<string>;
}

/**
 * Options for evidence injection
 */
export interface EvidenceInjectionOptions {
  /** Create backup before injection */
  createBackup?: boolean;
  /** Backup file suffix */
  backupSuffix?: string;
  /** Preserve original formatting */
  preserveFormatting?: boolean;
  /** Injection strategy */
  strategy?: 'append' | 'replace' | 'merge';
  /** Section to inject into (format-specific) */
  targetSection?: string;
}

/**
 * Result of evidence injection
 */
export interface EvidenceInjectionResult {
  /** Whether injection succeeded */
  success: boolean;
  /** Path to modified file */
  filePath?: string;
  /** Path to backup file (if created) */
  backupPath?: string;
  /** Errors encountered */
  errors?: AdapterError[];
  /** Warnings encountered */
  warnings?: AdapterWarning[];
  /** Statistics about injection */
  stats?: {
    linesAdded: number;
    linesModified: number;
    sectionsAdded: string[];
  };
}

/**
 * Error thrown when format detection fails
 */
export class FormatDetectionError extends Error {
  constructor(
    message: string,
    public readonly filePath?: string,
    public readonly triedFormats?: string[]
  ) {
    super(message);
    this.name = 'FormatDetectionError';
  }
}

/**
 * Error thrown when plan loading fails
 */
export class PlanLoadError extends Error {
  constructor(
    message: string,
    public readonly filePath?: string,
    public readonly cause?: Error
  ) {
    super(message);
    this.name = 'PlanLoadError';
  }
}
