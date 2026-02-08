/**
 * Adapter Registry
 *
 * Central registry for format adapters with auto-detection and lookup capabilities.
 */

import type { FormatAdapter, DetectionResult, PathDetectionHint } from './types.js';

/**
 * Registry for managing format adapters
 *
 * Provides:
 * - Adapter registration and lookup
 * - Auto-detection of formats from content
 * - Extension-based adapter selection
 */
export class AdapterRegistry {
  private static instance: AdapterRegistry | null = null;
  private adapters: Map<string, FormatAdapter> = new Map();

  private constructor() {}

  /**
   * Get singleton instance
   */
  static getInstance(): AdapterRegistry {
    if (!AdapterRegistry.instance) {
      AdapterRegistry.instance = new AdapterRegistry();
    }
    return AdapterRegistry.instance;
  }

  /**
   * Reset singleton instance (useful for testing)
   */
  static resetInstance(): void {
    AdapterRegistry.instance = null;
  }

  /**
   * Register an adapter
   *
   * @param adapter - Adapter to register
   * @throws Error if adapter with same name already registered
   */
  register(adapter: FormatAdapter): void {
    if (this.adapters.has(adapter.metadata.name)) {
      throw new Error(`Adapter '${adapter.metadata.name}' is already registered`);
    }
    this.adapters.set(adapter.metadata.name, adapter);
  }

  /**
   * Unregister an adapter by name
   *
   * @param name - Name of adapter to unregister
   * @returns True if adapter was removed, false if not found
   */
  unregister(name: string): boolean {
    return this.adapters.delete(name);
  }

  /**
   * Get adapter by name
   *
   * @param name - Adapter name
   * @returns Adapter instance or undefined
   */
  getAdapter(name: string): FormatAdapter | undefined {
    return this.adapters.get(name);
  }

  /**
   * Get adapter for a specific format identifier or extension
   *
   * @param format - Format identifier (e.g., "speckit") or extension (e.g., ".md")
   * @returns First matching adapter or undefined
   */
  getAdapterForFormat(format: string): FormatAdapter | undefined {
    for (const adapter of this.adapters.values()) {
      if (adapter.canImport(format)) {
        return adapter;
      }
    }
    return undefined;
  }

  /**
   * Detect adapter from content
   *
   * Runs detection on all registered adapters and returns the best match.
   *
   * @param content - Content to analyze
   * @param minConfidence - Minimum confidence score (0-100) to accept
   * @returns Best matching adapter and detection result, or undefined
   */
  detectAdapter(
    content: string,
    minConfidence: number = 50
  ): { adapter: FormatAdapter; detection: DetectionResult } | undefined {
    let bestMatch: { adapter: FormatAdapter; detection: DetectionResult } | undefined;
    let bestConfidence = minConfidence - 1;

    for (const adapter of this.adapters.values()) {
      const detection = adapter.detect(content);
      if (detection.detected && detection.confidence > bestConfidence) {
        bestConfidence = detection.confidence;
        bestMatch = { adapter, detection };
      }
    }

    return bestMatch;
  }

  /**
   * Detect adapter from content with path hints
   *
   * Like detectAdapter but passes file path information to adapters
   * that implement detectWithPath for improved accuracy.
   *
   * @param content - Content to analyze
   * @param hint - File path and directory information
   * @param minConfidence - Minimum confidence score (0-100) to accept
   * @returns Best matching adapter and detection result, or undefined
   */
  detectAdapterWithPath(
    content: string,
    hint: PathDetectionHint,
    minConfidence: number = 50
  ): { adapter: FormatAdapter; detection: DetectionResult } | undefined {
    let bestMatch: { adapter: FormatAdapter; detection: DetectionResult } | undefined;
    let bestConfidence = minConfidence - 1;

    for (const adapter of this.adapters.values()) {
      const detection = adapter.detectWithPath
        ? adapter.detectWithPath(content, hint)
        : adapter.detect(content);

      if (detection.detected && detection.confidence > bestConfidence) {
        bestConfidence = detection.confidence;
        bestMatch = { adapter, detection };
      }
    }

    return bestMatch;
  }

  /**
   * Get all detection results for content
   *
   * Useful for debugging or showing user multiple format possibilities.
   *
   * @param content - Content to analyze
   * @returns Array of detection results for all adapters, sorted by confidence
   */
  detectAll(content: string): Array<{ adapter: FormatAdapter; detection: DetectionResult }> {
    const results = Array.from(this.adapters.values()).map((adapter) => ({
      adapter,
      detection: adapter.detect(content),
    }));

    return results.sort((a, b) => b.detection.confidence - a.detection.confidence);
  }

  /**
   * List all registered adapters
   *
   * @returns Array of all adapters
   */
  listAdapters(): ReadonlyArray<FormatAdapter> {
    return Array.from(this.adapters.values());
  }

  /**
   * List all adapter names
   *
   * @returns Array of adapter names
   */
  listAdapterNames(): ReadonlyArray<string> {
    return Array.from(this.adapters.keys());
  }

  /**
   * List all supported formats across all adapters
   *
   * @returns Array of unique format identifiers
   */
  listSupportedFormats(): ReadonlyArray<string> {
    const formats = new Set<string>();
    for (const adapter of this.adapters.values()) {
      for (const format of adapter.metadata.formats) {
        formats.add(format);
      }
    }
    return Array.from(formats).sort();
  }

  /**
   * List all supported extensions across all adapters
   *
   * @returns Array of unique file extensions
   */
  listSupportedExtensions(): ReadonlyArray<string> {
    const extensions = new Set<string>();
    for (const adapter of this.adapters.values()) {
      for (const ext of adapter.metadata.extensions) {
        extensions.add(ext);
      }
    }
    return Array.from(extensions).sort();
  }

  /**
   * Get adapters that can import a specific format
   *
   * @param format - Format identifier or extension
   * @returns Array of adapters that support this format for import
   */
  getImportAdapters(format: string): ReadonlyArray<FormatAdapter> {
    return Array.from(this.adapters.values()).filter((adapter) => adapter.canImport(format));
  }

  /**
   * Get adapters that can export to a specific format
   *
   * @param format - Format identifier or extension
   * @returns Array of adapters that support this format for export
   */
  getExportAdapters(format: string): ReadonlyArray<FormatAdapter> {
    return Array.from(this.adapters.values()).filter((adapter) => adapter.canExport(format));
  }

  /**
   * Check if any adapter supports a format
   *
   * @param format - Format identifier or extension
   * @returns True if at least one adapter supports this format
   */
  isFormatSupported(format: string): boolean {
    return this.getAdapterForFormat(format) !== undefined;
  }

  /**
   * Clear all registered adapters
   *
   * Mainly useful for testing.
   */
  clear(): void {
    this.adapters.clear();
  }

  /**
   * Get number of registered adapters
   */
  get size(): number {
    return this.adapters.size;
  }
}

/**
 * Default singleton instance
 */
export const registry = AdapterRegistry.getInstance();
