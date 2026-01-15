/**
 * Anti-pattern scanner
 *
 * Scans source files for anti-patterns using regex-based detection.
 */

import { minimatch } from 'minimatch';
import type { Warning, AntiPattern } from './types.js';
import { getDefaultPatterns, getEnabledPatterns, getPattern } from './patterns.js';

export interface ScanOptions {
  /** Pattern IDs to check (default: all default patterns) */
  patterns?: string[];
  /** Include opt-in patterns (default: false) */
  includeOptIn?: boolean;
}

export interface ScanResult {
  /** File path that was scanned */
  file: string;
  /** Warnings found in the file */
  warnings: Warning[];
  /** Pattern IDs that were checked */
  patternsChecked: string[];
}

function getPatternsToCheck(options: ScanOptions = {}): AntiPattern[] {
  const { patterns: patternIds, includeOptIn = false } = options;

  if (patternIds && patternIds.length > 0) {
    return patternIds.map((id) => getPattern(id)).filter((p): p is AntiPattern => p !== undefined);
  }

  return includeOptIn ? getEnabledPatterns() : getDefaultPatterns();
}

function isFileAllowlisted(filePath: string, allowlist: string[] | undefined): boolean {
  if (!allowlist || allowlist.length === 0) return false;
  return allowlist.some((pattern) => minimatch(filePath, pattern, { matchBase: true }));
}

function createWarningFromMatch(
  pattern: AntiPattern,
  filePath: string,
  line: number,
  column: number
): Warning {
  return {
    id: pattern.id,
    category: 'anti-pattern',
    severity: pattern.severity,
    confidence: pattern.confidence,
    title: pattern.title,
    message: `Found ${pattern.name} at line ${line}`,
    explanation: pattern.explanation,
    suggestion: pattern.suggestion,
    location: {
      file: filePath,
      line,
      column,
    },
    pattern: pattern.id,
  };
}

/**
 * Scan a file's content for anti-patterns
 */
export function scanFile(filePath: string, content: string, options?: ScanOptions): ScanResult {
  const patterns = getPatternsToCheck(options);
  const warnings: Warning[] = [];
  const lines = content.split('\n');

  for (const pattern of patterns) {
    if (pattern.detection.type !== 'regex') continue;
    if (isFileAllowlisted(filePath, pattern.allowlist)) continue;

    const regexPattern = pattern.detection.pattern;
    if (!regexPattern) continue;

    const regex = new RegExp(regexPattern, 'g');

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
      const line = lines[lineIndex];
      const lineNumber = lineIndex + 1;

      let match: RegExpExecArray | null;
      regex.lastIndex = 0;

      while ((match = regex.exec(line)) !== null) {
        warnings.push(createWarningFromMatch(pattern, filePath, lineNumber, match.index));
      }
    }
  }

  return {
    file: filePath,
    warnings,
    patternsChecked: patterns.map((p) => p.id),
  };
}

/**
 * Scan multiple files for anti-patterns
 */
export function scanFiles(
  files: Array<{ path: string; content: string }>,
  options?: ScanOptions
): ScanResult[] {
  return files.map((file) => scanFile(file.path, file.content, options));
}
