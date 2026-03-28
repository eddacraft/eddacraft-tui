/**
 * Format detection service for automatic format identification
 * @module cli/services/format-detection
 */

import { readFile } from 'node:fs/promises';
import { AdapterRegistry } from '@eddacraft/anvil-adapters';
import type {
  FormatDetectionService as IFormatDetectionService,
  FormatDetectionResult,
  FormatDetectionError,
} from '../types/services.js';

/**
 * Default minimum confidence threshold for format detection
 */
const DEFAULT_MIN_CONFIDENCE = 50;

/**
 * Implementation of format detection service using adapter registry
 */
export class FormatDetectionService implements IFormatDetectionService {
  private registry: AdapterRegistry;
  private minConfidence: number;

  constructor(options?: { minConfidence?: number }) {
    this.registry = AdapterRegistry.getInstance();
    this.minConfidence = options?.minConfidence ?? DEFAULT_MIN_CONFIDENCE;
  }

  /**
   * Detect format from file content
   * @param content - Raw file content
   * @param filePath - Optional file path for extension-based hints
   * @returns Detection result with selected adapter, or null if no match
   */
  async detectFormat(content: string, filePath?: string): Promise<FormatDetectionResult | null> {
    // Use adapter registry's detection mechanism
    const detected = this.registry.detectAdapter(content, this.minConfidence);

    if (!detected) {
      return null;
    }

    return {
      format: detected.adapter.metadata.name,
      adapter: detected.adapter,
      detection: detected.detection,
      filePath,
    };
  }

  /**
   * Detect format from file path
   * @param filePath - Absolute path to the file
   * @returns Detection result with selected adapter, or null if no match
   */
  async detectFormatFromFile(filePath: string): Promise<FormatDetectionResult | null> {
    try {
      const content = await readFile(filePath, 'utf-8');
      return this.detectFormat(content, filePath);
    } catch (error) {
      const err = error as NodeJS.ErrnoException;
      throw new FormatDetectionErrorImpl(`Failed to read file: ${err.message}`, filePath);
    }
  }

  /**
   * Get all possible formats for given content
   * @param content - Raw file content
   * @returns Array of all matching adapters sorted by confidence (descending)
   */
  async detectAllFormats(content: string): Promise<FormatDetectionResult[]> {
    const results: FormatDetectionResult[] = [];
    const adapters = this.registry.listAdapters();

    for (const adapter of adapters) {
      const detection = adapter.detect(content);
      if (detection.detected && detection.confidence >= this.minConfidence) {
        results.push({
          format: adapter.metadata.name,
          adapter,
          detection,
        });
      }
    }

    // Sort by confidence (descending)
    return results.sort((a, b) => b.detection.confidence - a.detection.confidence);
  }
}

/**
 * Error implementation for format detection failures
 */
class FormatDetectionErrorImpl extends Error implements FormatDetectionError {
  constructor(
    message: string,
    public readonly filePath?: string,
    public readonly triedFormats?: string[]
  ) {
    super(message);
    this.name = 'FormatDetectionError';
    Error.captureStackTrace?.(this, FormatDetectionErrorImpl);
  }
}
