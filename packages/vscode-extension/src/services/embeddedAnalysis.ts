/**
 * Embedded Analysis Service
 *
 * Provides fast-path analysis using @eddacraft/anvil-core directly (no CLI subprocess).
 * Used for operations that need sub-100ms response times:
 * - Anti-pattern detection
 * - Format detection (basic)
 *
 * Heavy operations (gates, OPA, coverage) remain CLI-based via AnvilService.
 *
 * Includes content-hash LRU caching to skip re-analysis of unchanged files.
 */

import * as crypto from 'crypto';
import {
  scanFile,
  type ScanResult,
  type Warning,
  type ScanOptions,
  getPattern,
  getDefaultPatterns,
  type AntiPattern,
} from '@eddacraft/anvil-core/antipattern';

/**
 * Cache entry for analysis results
 */
interface CacheEntry {
  /** Content hash (SHA-256) */
  hash: string;
  /** Cached analysis result */
  result: AnalysisResult;
  /** Timestamp when cached */
  cachedAt: number;
}

/**
 * Analysis result for a single file
 */
export interface AnalysisResult {
  /** File path that was analysed */
  file: string;
  /** Warnings found in the file */
  warnings: AnalysisWarning[];
  /** Time taken for analysis in milliseconds */
  duration: number;
  /** Pattern IDs that were checked */
  patternsChecked: string[];
}

/**
 * Warning with VS Code-friendly location
 */
export interface AnalysisWarning {
  /** Warning ID (e.g., AP-001) */
  id: string;
  /** Warning title */
  title: string;
  /** Primary message */
  message: string;
  /** Why this matters */
  explanation: string;
  /** What to do instead */
  suggestion: string;
  /** Severity level */
  severity: 'error' | 'warning' | 'info';
  /** Detection confidence */
  confidence: 'high' | 'medium' | 'low';
  /** Source location (1-based line, 0-based column) */
  location: {
    file: string;
    line: number;
    column: number;
    endLine?: number;
    endColumn?: number;
  };
  /** Pattern ID that triggered this warning */
  pattern: string;
  /** Link to documentation */
  documentationUrl?: string;
}

/**
 * Options for embedded analysis
 */
export interface EmbeddedAnalysisOptions {
  /** Pattern IDs to check (default: all default patterns) */
  patterns?: string[];
  /** Include opt-in patterns like console detection (default: false) */
  includeOptIn?: boolean;
}

const DEFAULT_CACHE_TTL_MS = 5 * 60 * 1000;
const MAX_CACHE_SIZE = 100;

export class EmbeddedAnalysisService {
  private patternCache: Map<string, AntiPattern> = new Map();
  private analysisCache: Map<string, CacheEntry> = new Map();
  private cacheTTL: number;

  constructor(cacheTTL: number = DEFAULT_CACHE_TTL_MS) {
    this.cacheTTL = cacheTTL;

    for (const pattern of getDefaultPatterns()) {
      this.patternCache.set(pattern.id, pattern);
    }
  }

  analyseFile(
    filePath: string,
    content: string,
    options?: EmbeddedAnalysisOptions
  ): AnalysisResult {
    const contentHash = this.hashContent(content);
    const cacheKey = this.getCacheKey(filePath, options);

    const cached = this.analysisCache.get(cacheKey);
    if (cached && cached.hash === contentHash && !this.isCacheExpired(cached)) {
      this.touchCacheEntry(cacheKey, cached);
      return { ...cached.result, duration: 0 };
    }

    const startTime = performance.now();

    const scanOptions: ScanOptions = {
      patterns: options?.patterns,
      includeOptIn: options?.includeOptIn ?? false,
    };

    const scanResult: ScanResult = scanFile(filePath, content, scanOptions);
    const warnings = scanResult.warnings.map((w) => this.mapWarning(w));

    const result: AnalysisResult = {
      file: filePath,
      warnings,
      duration: Math.round(performance.now() - startTime),
      patternsChecked: scanResult.patternsChecked,
    };

    this.cacheResult(cacheKey, contentHash, result);

    return result;
  }

  private hashContent(content: string): string {
    return crypto.createHash('sha256').update(content).digest('hex').slice(0, 16);
  }

  private getCacheKey(filePath: string, options?: EmbeddedAnalysisOptions): string {
    const optionsKey = options?.patterns?.join(',') ?? 'default';
    const optInKey = options?.includeOptIn ? '+optin' : '';
    return `${filePath}:${optionsKey}${optInKey}`;
  }

  private isCacheExpired(entry: CacheEntry): boolean {
    return Date.now() - entry.cachedAt > this.cacheTTL;
  }

  private cacheResult(key: string, hash: string, result: AnalysisResult): void {
    if (this.analysisCache.size >= MAX_CACHE_SIZE) {
      const lruKey = this.analysisCache.keys().next().value;
      if (lruKey) {
        this.analysisCache.delete(lruKey);
      }
    }

    this.analysisCache.set(key, {
      hash,
      result,
      cachedAt: Date.now(),
    });
  }

  private touchCacheEntry(key: string, entry: CacheEntry): void {
    this.analysisCache.delete(key);
    this.analysisCache.set(key, entry);
  }

  invalidateCache(filePath?: string): void {
    if (filePath) {
      for (const key of this.analysisCache.keys()) {
        if (key.startsWith(filePath + ':')) {
          this.analysisCache.delete(key);
        }
      }
    } else {
      this.analysisCache.clear();
    }
  }

  getCacheStats(): { size: number; maxSize: number; ttlMs: number } {
    return {
      size: this.analysisCache.size,
      maxSize: MAX_CACHE_SIZE,
      ttlMs: this.cacheTTL,
    };
  }

  /**
   * Check if a file should be analysed based on extension
   *
   * @param filePath - Path to check
   * @returns true if file should be analysed
   */
  shouldAnalyse(filePath: string): boolean {
    const analysableExtensions = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.mts', '.cts'];

    const ext = filePath.substring(filePath.lastIndexOf('.'));
    return analysableExtensions.includes(ext.toLowerCase());
  }

  /**
   * Get pattern details by ID
   *
   * @param id - Pattern ID (e.g., AP-001)
   * @returns Pattern details or undefined
   */
  getPatternInfo(id: string): AntiPattern | undefined {
    // Check cache first
    if (this.patternCache.has(id)) {
      return this.patternCache.get(id);
    }

    // Fetch and cache
    const pattern = getPattern(id);
    if (pattern) {
      this.patternCache.set(id, pattern);
    }
    return pattern;
  }

  /**
   * Get all available pattern IDs
   */
  getAvailablePatterns(): string[] {
    return getDefaultPatterns().map((p) => p.id);
  }

  /**
   * Map core Warning to VS Code-friendly AnalysisWarning
   */
  private mapWarning(warning: Warning): AnalysisWarning {
    const pattern = this.getPatternInfo(warning.id);

    return {
      id: warning.id,
      title: warning.title,
      message: warning.message,
      explanation: warning.explanation,
      suggestion: warning.suggestion,
      severity: warning.severity,
      confidence: warning.confidence,
      location: {
        file: warning.location.file,
        line: warning.location.line,
        column: warning.location.column ?? 0,
        endLine: warning.location.endLine,
        endColumn: warning.location.endColumn,
      },
      pattern: warning.pattern ?? warning.id,
      documentationUrl: pattern?.documentation,
    };
  }
}

/**
 * Singleton instance for use across the extension
 */
let instance: EmbeddedAnalysisService | undefined;

export function getEmbeddedAnalysisService(): EmbeddedAnalysisService {
  if (!instance) {
    instance = new EmbeddedAnalysisService();
  }
  return instance;
}
