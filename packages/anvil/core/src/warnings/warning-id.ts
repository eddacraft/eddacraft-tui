/**
 * Warning ID System
 *
 * Provides unique, stable warning identifiers for the explain command.
 * Warning IDs encode the rule, file, and line number to enable precise lookup.
 *
 * Format: {rule}-{file}:{line}
 * Example: AP-003-src/utils/helpers.ts:42
 *
 * @module warnings/warning-id
 */

import { z } from 'zod';
import type { Warning } from './types.js';

// =============================================================================
// Warning ID Schema
// =============================================================================

/**
 * Pattern for valid warning IDs
 * Format: {RULE}-{FILE}:{LINE}
 * Examples:
 * - AP-003-src/utils/helpers.ts:42
 * - ARCH-001-src/api/handler.ts:15
 * - BOUND-001-src/db/queries.ts:100
 */
export const WARNING_ID_PATTERN = /^(AP|ARCH|BOUND)-\d{3}-[^:]+:\d+$/;

/**
 * Schema for parsed warning ID components
 */
export const ParsedWarningIdSchema = z.object({
  rule: z
    .string()
    .regex(/^(AP|ARCH|BOUND)-\d{3}$/)
    .describe('The rule ID (e.g., AP-003, ARCH-001)'),
  file: z.string().min(1).describe('File path relative to workspace root'),
  line: z.number().int().positive().describe('Line number (1-based)'),
});

export type ParsedWarningId = z.infer<typeof ParsedWarningIdSchema>;

// =============================================================================
// Warning ID Generation
// =============================================================================

/**
 * Generate a unique warning ID from a warning object
 *
 * The ID format is: {rule}-{file}:{line}
 * This ensures uniqueness per warning instance while remaining human-readable.
 *
 * @param warning - The warning object
 * @returns A unique warning ID string
 *
 * @example
 * ```ts
 * const warning = {
 *   id: 'AP-003',
 *   location: { file: 'src/utils/helpers.ts', line: 42 }
 * };
 * const warningId = generateWarningId(warning);
 * // Returns: 'AP-003-src/utils/helpers.ts:42'
 * ```
 */
export function generateWarningId(warning: Pick<Warning, 'id' | 'location'>): string {
  const { id, location } = warning;
  return `${id}-${location.file}:${location.line}`;
}

/**
 * Generate a warning ID from components
 *
 * @param rule - The rule ID (e.g., 'AP-003', 'ARCH-001')
 * @param file - File path relative to workspace root
 * @param line - Line number (1-based)
 * @returns A unique warning ID string
 *
 * @example
 * ```ts
 * const warningId = createWarningId('AP-003', 'src/utils/helpers.ts', 42);
 * // Returns: 'AP-003-src/utils/helpers.ts:42'
 * ```
 */
export function createWarningId(rule: string, file: string, line: number): string {
  return `${rule}-${file}:${line}`;
}

// =============================================================================
// Warning ID Parsing
// =============================================================================

/**
 * Parse a warning ID into its components
 *
 * @param warningId - The warning ID to parse
 * @returns Parsed components (rule, file, line) or null if invalid
 *
 * @example
 * ```ts
 * const parsed = parseWarningId('AP-003-src/utils/helpers.ts:42');
 * // Returns: { rule: 'AP-003', file: 'src/utils/helpers.ts', line: 42 }
 *
 * const invalid = parseWarningId('invalid-id');
 * // Returns: null
 * ```
 */
export function parseWarningId(warningId: string): ParsedWarningId | null {
  // Match the pattern: RULE-FILE:LINE
  // RULE is like AP-003 or ARCH-001 (letters-digits)
  // FILE can contain any characters except :
  // LINE is a positive integer
  const match = warningId.match(/^((?:AP|ARCH|BOUND)-\d{3})-(.+):(\d+)$/);

  if (!match) {
    return null;
  }

  const [, rule, file, lineStr] = match;
  const line = parseInt(lineStr, 10);

  if (isNaN(line) || line <= 0) {
    return null;
  }

  return { rule, file, line };
}

/**
 * Validate a warning ID string
 *
 * @param warningId - The warning ID to validate
 * @returns true if the ID is valid, false otherwise
 *
 * @example
 * ```ts
 * isValidWarningId('AP-003-src/utils/helpers.ts:42'); // true
 * isValidWarningId('invalid'); // false
 * isValidWarningId('AP-003'); // false (missing file:line)
 * ```
 */
export function isValidWarningId(warningId: string): boolean {
  return parseWarningId(warningId) !== null;
}

// =============================================================================
// Warning Lookup
// =============================================================================

/**
 * Find a warning in a list by its warning ID
 *
 * @param warnings - Array of warnings to search
 * @param warningId - The warning ID to find
 * @returns The matching warning or undefined
 *
 * @example
 * ```ts
 * const warnings = [...]; // From check results
 * const warning = findWarningById(warnings, 'AP-003-src/utils/helpers.ts:42');
 * if (warning) {
 *   console.log(warning.message);
 * }
 * ```
 */
export function findWarningById(warnings: Warning[], warningId: string): Warning | undefined {
  const parsed = parseWarningId(warningId);
  if (!parsed) {
    return undefined;
  }

  return warnings.find(
    (w) =>
      w.id === parsed.rule && w.location.file === parsed.file && w.location.line === parsed.line
  );
}

/**
 * Find all warnings matching a rule ID (partial match)
 *
 * @param warnings - Array of warnings to search
 * @param ruleId - The rule ID to match (e.g., 'AP-003')
 * @returns Array of matching warnings
 *
 * @example
 * ```ts
 * const warnings = [...]; // From check results
 * const anyWarnings = findWarningsByRule(warnings, 'AP-003');
 * console.log(`Found ${anyWarnings.length} explicit any types`);
 * ```
 */
export function findWarningsByRule(warnings: Warning[], ruleId: string): Warning[] {
  return warnings.filter((w) => w.id === ruleId);
}

/**
 * Find all warnings in a specific file
 *
 * @param warnings - Array of warnings to search
 * @param file - File path to match
 * @returns Array of matching warnings
 */
export function findWarningsByFile(warnings: Warning[], file: string): Warning[] {
  return warnings.filter((w) => w.location.file === file);
}

// =============================================================================
// Warning ID Collection
// =============================================================================

/**
 * Generate warning IDs for all warnings in a list
 *
 * @param warnings - Array of warnings
 * @returns Map of warning ID to warning
 *
 * @example
 * ```ts
 * const warnings = [...]; // From check results
 * const warningMap = indexWarningsById(warnings);
 *
 * // Quick lookup by ID
 * const warning = warningMap.get('AP-003-src/utils/helpers.ts:42');
 * ```
 */
export function indexWarningsById(warnings: Warning[]): Map<string, Warning> {
  const map = new Map<string, Warning>();
  for (const warning of warnings) {
    const id = generateWarningId(warning);
    map.set(id, warning);
  }
  return map;
}

/**
 * Get all warning IDs from a list of warnings
 *
 * @param warnings - Array of warnings
 * @returns Array of warning IDs
 */
export function getWarningIds(warnings: Warning[]): string[] {
  return warnings.map(generateWarningId);
}

// =============================================================================
// Short ID Support
// =============================================================================

/**
 * Generate a short ID for display purposes
 *
 * Useful when the full warning ID is too long for terminal display.
 * Format: {rule}:{line} (e.g., AP-003:42)
 *
 * @param warning - The warning object
 * @returns A shortened warning ID
 */
export function generateShortId(warning: Pick<Warning, 'id' | 'location'>): string {
  return `${warning.id}:${warning.location.line}`;
}

/**
 * Attempt to resolve a short ID to a full warning ID
 *
 * Searches warnings by rule and line, returns full ID if unique match found.
 *
 * @param warnings - Array of warnings to search
 * @param shortId - Short ID in format RULE:LINE (e.g., 'AP-003:42')
 * @returns Full warning ID, array of matches if ambiguous, or null if not found
 */
export function resolveShortId(warnings: Warning[], shortId: string): string | string[] | null {
  const match = shortId.match(/^((?:AP|ARCH|BOUND)-\d{3}):(\d+)$/);
  if (!match) {
    return null;
  }

  const [, rule, lineStr] = match;
  const line = parseInt(lineStr, 10);

  const matches = warnings.filter((w) => w.id === rule && w.location.line === line);

  if (matches.length === 0) {
    return null;
  }

  if (matches.length === 1) {
    return generateWarningId(matches[0]);
  }

  // Multiple matches - return all full IDs for disambiguation
  return matches.map(generateWarningId);
}
