import { createHash } from 'node:crypto';

import type {
  AdapterError,
  AdapterWarning,
  ParseResult,
  SerializeResult,
  DetectionResult,
} from './types.js';

export function generateDeterministicPlanId(content: string): string {
  const hash = createHash('sha256').update(content, 'utf8').digest('hex');
  return `aps-${hash.substring(0, 8)}`;
}

/**
 * Create a detection result
 *
 * @param detected - Whether format was detected
 * @param confidence - Confidence score (0-100)
 * @param reason - Optional reason
 * @returns Detection result
 */
export function createDetection(
  detected: boolean,
  confidence: number,
  reason?: string
): DetectionResult {
  return {
    detected,
    confidence: Math.max(0, Math.min(100, confidence)),
    reason,
  };
}

/**
 * Create an adapter error
 *
 * @param code - Error code
 * @param message - Error message
 * @param options - Additional options
 * @returns Adapter error
 */
export function createError(
  code: string,
  message: string,
  options?: {
    path?: string;
    line?: number;
    column?: number;
    details?: unknown;
  }
): AdapterError {
  return {
    code,
    message,
    ...options,
  };
}

/**
 * Create an adapter warning
 *
 * @param code - Warning code
 * @param message - Warning message
 * @param options - Additional options
 * @returns Adapter warning
 */
export function createWarning(
  code: string,
  message: string,
  options?: {
    path?: string;
    line?: number;
    column?: number;
    details?: unknown;
  }
): AdapterWarning {
  return {
    code,
    message,
    ...options,
  };
}

/**
 * Check if content matches a pattern
 *
 * Useful for format detection.
 *
 * @param content - Content to check
 * @param patterns - Array of regex patterns or strings
 * @returns Number of patterns matched (0 to patterns.length)
 */
export function matchesPatterns(content: string, patterns: Array<string | RegExp>): number {
  let matches = 0;
  for (const pattern of patterns) {
    if (typeof pattern === 'string') {
      if (content.includes(pattern)) {
        matches++;
      }
    } else {
      if (pattern.test(content)) {
        matches++;
      }
    }
  }
  return matches;
}

/**
 * Calculate confidence score based on pattern matches
 *
 * @param matchCount - Number of patterns matched
 * @param totalPatterns - Total number of patterns checked
 * @param requiredMatches - Minimum matches for 100% confidence
 * @returns Confidence score (0-100)
 */
export function calculateConfidence(
  matchCount: number,
  totalPatterns: number,
  requiredMatches: number = totalPatterns
): number {
  if (matchCount === 0) return 0;
  if (matchCount >= requiredMatches) return 100;

  // Linear interpolation
  return Math.round((matchCount / requiredMatches) * 100);
}

/**
 * Extract file extension from filename or path
 *
 * @param filename - Filename or path
 * @returns Extension (including dot) or empty string
 */
export function getExtension(filename: string): string {
  const match = filename.match(/\.[^.]+$/);
  return match ? match[0] : '';
}

/**
 * Normalize format identifier
 *
 * Removes leading dot and converts to lowercase.
 *
 * @param format - Format identifier or extension
 * @returns Normalized format
 */
export function normalizeFormat(format: string): string {
  return format.toLowerCase().replace(/^\./, '');
}

/**
 * Format errors for display
 *
 * @param errors - Array of errors
 * @returns Formatted error string
 */
export function formatErrors(errors: AdapterError[]): string {
  return errors
    .map((error) => {
      let msg = `[${error.code}] ${error.message}`;
      if (error.path) {
        msg += ` (at ${error.path}`;
        if (error.line !== undefined) {
          msg += `:${error.line}`;
          if (error.column !== undefined) {
            msg += `:${error.column}`;
          }
        }
        msg += ')';
      }
      return msg;
    })
    .join('\n');
}

/**
 * Format warnings for display
 *
 * @param warnings - Array of warnings
 * @returns Formatted warning string
 */
export function formatWarnings(warnings: AdapterWarning[]): string {
  return warnings
    .map((warning) => {
      let msg = `[${warning.code}] ${warning.message}`;
      if (warning.path) {
        msg += ` (at ${warning.path}`;
        if (warning.line !== undefined) {
          msg += `:${warning.line}`;
          if (warning.column !== undefined) {
            msg += `:${warning.column}`;
          }
        }
        msg += ')';
      }
      return msg;
    })
    .join('\n');
}

/**
 * Merge parse results
 *
 * Useful when parsing composite formats.
 *
 * @param results - Array of parse results
 * @returns Merged result
 */
export function mergeParseResults(results: ParseResult[]): ParseResult {
  if (results.length === 0) {
    return {
      success: false,
      errors: [createError('NO_RESULTS', 'No parse results to merge')],
    };
  }

  if (results.length === 1) {
    return results[0];
  }

  const allErrors: AdapterError[] = [];
  const allWarnings: AdapterWarning[] = [];
  let finalData = results[0].data;

  for (const result of results) {
    if (!result.success) {
      if (result.errors) {
        allErrors.push(...result.errors);
      }
    }
    if (result.warnings) {
      allWarnings.push(...result.warnings);
    }
    if (result.data) {
      finalData = result.data;
    }
  }

  if (allErrors.length > 0) {
    return {
      success: false,
      errors: allErrors,
      warnings: allWarnings.length > 0 ? allWarnings : undefined,
    };
  }

  return {
    success: true,
    data: finalData,
    warnings: allWarnings.length > 0 ? allWarnings : undefined,
  };
}

/**
 * Check if result has errors
 *
 * @param result - Parse or serialize result
 * @returns True if result has errors
 */
export function hasErrors(result: ParseResult | SerializeResult): boolean {
  return !result.success || (result.errors !== undefined && result.errors.length > 0);
}

/**
 * Check if result has warnings
 *
 * @param result - Parse or serialize result
 * @returns True if result has warnings
 */
export function hasWarnings(result: ParseResult | SerializeResult): boolean {
  return result.warnings !== undefined && result.warnings.length > 0;
}

/**
 * Extract first line from content (useful for format detection)
 *
 * @param content - Content string
 * @returns First non-empty line or empty string
 */
export function getFirstLine(content: string): string {
  const lines = content.split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed) {
      return trimmed;
    }
  }
  return '';
}

/**
 * Count occurrences of pattern in content
 *
 * @param content - Content to search
 * @param pattern - Pattern to count
 * @returns Number of occurrences
 */
export function countOccurrences(content: string, pattern: string | RegExp): number {
  if (typeof pattern === 'string') {
    return (content.match(new RegExp(pattern, 'g')) || []).length;
  }
  return (content.match(new RegExp(pattern.source, 'g')) || []).length;
}
