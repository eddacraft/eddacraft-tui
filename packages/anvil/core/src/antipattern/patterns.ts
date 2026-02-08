/**
 * Anti-pattern Catalogue
 *
 * Defines the built-in anti-patterns that Anvil detects. Each pattern has:
 * - ID: Unique identifier (AP-001, AP-002, etc.)
 * - Detection: Regex or AST-based detection method
 * - Messaging: Title, explanation, and suggestion for warnings
 * - Configuration: Severity, confidence, allowlist, threshold, etc.
 *
 * @module antipattern/patterns
 */

import type { AntiPattern } from './types.js';
import { HTML_PATTERNS } from './patterns-html.js';
import { CSS_PATTERNS } from './patterns-css.js';

// =============================================================================
// Pattern Definitions
// =============================================================================

/**
 * AP-001: Broad eslint-disable (file-level or block without rule)
 *
 * Detects `eslint-disable` comments that disable all rules, which:
 * - Silences all linting errors in scope
 * - Makes code review harder
 * - Often masks multiple issues
 */
const AP001_BROAD_ESLINT_DISABLE: AntiPattern = {
  id: 'AP-001',
  name: 'Broad eslint-disable',
  category: 'escape-hatch',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    // Matches /* eslint-disable */ without specific rules
    // Does NOT match eslint-disable-next-line or eslint-disable-line
    pattern: String.raw`/\*\s*eslint-disable\s*\*/|//\s*eslint-disable(?!-next-line|-line)\s*$`,
  },
  title: 'Broad eslint-disable added',
  explanation:
    'Disabling all ESLint rules hides legitimate issues and makes code harder to maintain. ' +
    'This pattern indicates technical debt that should be addressed.',
  suggestion:
    'Disable specific rules with /* eslint-disable rule-name */ or fix the underlying issues.',
  enabled: true,
  optIn: false,
};

/**
 * AP-002: Rule-specific eslint-disable
 *
 * Detects `eslint-disable rule-name` comments. These are less problematic than
 * broad disables but still warrant attention during review.
 */
const AP002_RULE_SPECIFIC_ESLINT_DISABLE: AntiPattern = {
  id: 'AP-002',
  name: 'Rule-specific eslint-disable',
  category: 'escape-hatch',
  severity: 'info',
  confidence: 'high',
  detection: {
    type: 'regex',
    // Matches eslint-disable with specific rule(s)
    // e.g., /* eslint-disable @typescript-eslint/no-explicit-any */
    // or eslint-disable-next-line no-console
    pattern: String.raw`eslint-disable(?:-next-line|-line)?\s+[\w@/-]+`,
  },
  title: 'Rule-specific eslint-disable',
  explanation:
    'While better than disabling all rules, targeted disables still indicate code that violates linting standards. ' +
    'Consider if the disable is necessary or if the code can be improved.',
  suggestion: 'Add a comment explaining why this rule needs to be disabled here.',
  enabled: true,
  optIn: true, // Noisy - opt-in only
};

/**
 * AP-003: Explicit `any` type usage
 *
 * Detects explicit use of `any` type in TypeScript. This bypasses type checking
 * and can hide bugs.
 */
const AP003_ANY_TYPE: AntiPattern = {
  id: 'AP-003',
  name: 'Explicit any type',
  category: 'type-safety',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    // Matches `: any`, `as any`, `<any>` type assertions
    // Careful to avoid matching words containing "any" like "company"
    pattern: String.raw`:\s*any\b|as\s+any\b|<any>`,
  },
  title: 'Explicit any type usage',
  explanation:
    'Using `any` defeats the purpose of TypeScript by disabling type checking. ' +
    'This can hide bugs and makes refactoring harder.',
  suggestion:
    'Use `unknown` for truly unknown types, or define a proper interface/type. ' +
    'For third-party libraries, consider using or creating type definitions.',
  enabled: true,
  optIn: false,
  allowlist: ['*.d.ts', '**/__mocks__/**', '**/test/**/*.ts'],
};

/**
 * AP-004: @ts-ignore directive
 *
 * Detects `@ts-ignore` which suppresses ALL TypeScript errors on the next line.
 */
const AP004_TS_IGNORE: AntiPattern = {
  id: 'AP-004',
  name: '@ts-ignore directive',
  category: 'type-safety',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`@ts-ignore`,
  },
  title: '@ts-ignore suppresses all errors',
  explanation:
    '@ts-ignore suppresses ALL TypeScript errors on the next line, including legitimate issues. ' +
    'This can hide bugs introduced by code changes.',
  suggestion:
    'Use @ts-expect-error with a description instead, which fails if the expected error disappears. ' +
    'Better yet, fix the underlying type issue.',
  enabled: true,
  optIn: false,
};

/**
 * AP-005: @ts-expect-error directive
 *
 * Detects `@ts-expect-error` which is safer than @ts-ignore but still indicates
 * intentional type issues.
 */
const AP005_TS_EXPECT_ERROR: AntiPattern = {
  id: 'AP-005',
  name: '@ts-expect-error directive',
  category: 'type-safety',
  severity: 'info',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`@ts-expect-error`,
  },
  title: '@ts-expect-error used',
  explanation:
    '@ts-expect-error is safer than @ts-ignore as it fails when the error disappears. ' +
    'However, it still indicates intentional type system workarounds.',
  suggestion:
    'Consider if the underlying type issue can be fixed. ' +
    'If not, ensure the @ts-expect-error comment explains why.',
  enabled: true,
  optIn: true, // Often legitimate in tests
  allowlist: ['**/*.test.ts', '**/*.spec.ts', '**/__tests__/**'],
};

/**
 * AP-006: Empty catch block
 *
 * Detects catch blocks that swallow errors without handling them.
 */
const AP006_EMPTY_CATCH: AntiPattern = {
  id: 'AP-006',
  name: 'Empty catch block',
  category: 'error-handling',
  severity: 'warning',
  confidence: 'medium',
  detection: {
    type: 'regex',
    // Matches catch blocks with only whitespace/comments inside
    pattern: String.raw`catch\s*\([^)]*\)\s*\{\s*(?://[^\n]*\s*)?\}`,
  },
  title: 'Empty catch block swallows errors',
  explanation:
    'Empty catch blocks silently swallow errors, making debugging difficult. ' +
    'Errors should be logged, re-thrown, or explicitly handled.',
  suggestion:
    'At minimum, log the error for debugging. Consider if the error should be re-thrown ' +
    'or if specific recovery logic is needed.',
  enabled: true,
  optIn: false,
};

/**
 * AP-007: Console usage in production code
 *
 * Detects console.log, console.warn, etc. in production code.
 */
const AP007_CONSOLE_IN_PROD: AntiPattern = {
  id: 'AP-007',
  name: 'Console in production code',
  category: 'code-quality',
  severity: 'info',
  confidence: 'medium',
  detection: {
    type: 'regex',
    pattern: String.raw`console\.(log|warn|info|debug)\s*\(`,
  },
  title: 'Console statement in production code',
  explanation:
    'Console statements should not appear in production code. They can leak sensitive information, ' +
    'clutter the console, and indicate incomplete debugging.',
  suggestion:
    'Use a proper logging library with log levels, or remove the console statement. ' +
    'console.error is acceptable for actual error conditions.',
  enabled: true,
  optIn: true, // Noisy in development
  allowlist: ['**/*.test.ts', '**/*.spec.ts', '**/scripts/**', '**/cli/**'],
};

// =============================================================================
// Pattern Registry
// =============================================================================

/**
 * All built-in anti-patterns
 */
export const PATTERNS: readonly AntiPattern[] = [
  AP001_BROAD_ESLINT_DISABLE,
  AP002_RULE_SPECIFIC_ESLINT_DISABLE,
  AP003_ANY_TYPE,
  AP004_TS_IGNORE,
  AP005_TS_EXPECT_ERROR,
  AP006_EMPTY_CATCH,
  AP007_CONSOLE_IN_PROD,
  ...HTML_PATTERNS,
  ...CSS_PATTERNS,
] as const;

/**
 * Pattern categories for filtering
 */
export type PatternCategory = AntiPattern['category'];

// =============================================================================
// Lookup Functions
// =============================================================================

/**
 * Get a pattern by ID
 *
 * @param id - Pattern ID (e.g., 'AP-001')
 * @returns The pattern definition, or undefined if not found
 *
 * @example
 * ```ts
 * const pattern = getPattern('AP-001');
 * console.log(pattern?.name); // 'Broad eslint-disable'
 * ```
 */
export function getPattern(id: string): AntiPattern | undefined {
  return PATTERNS.find((p) => p.id === id);
}

/**
 * Get all patterns in a category
 *
 * @param category - The category to filter by
 * @returns Array of patterns in that category
 *
 * @example
 * ```ts
 * const escapeHatches = getPatternsByCategory('escape-hatch');
 * console.log(escapeHatches.map(p => p.id)); // ['AP-001', 'AP-002']
 * ```
 */
export function getPatternsByCategory(category: PatternCategory): AntiPattern[] {
  return PATTERNS.filter((p) => p.category === category);
}

/**
 * Get all enabled patterns (respects enabled flag, not optIn)
 *
 * @returns Array of enabled patterns
 */
export function getEnabledPatterns(): AntiPattern[] {
  return PATTERNS.filter((p) => p.enabled);
}

/**
 * Get all default patterns (enabled and not opt-in)
 *
 * @returns Array of patterns that are enabled by default
 */
export function getDefaultPatterns(): AntiPattern[] {
  return PATTERNS.filter((p) => p.enabled && !p.optIn);
}

/**
 * Get pattern IDs for all patterns
 *
 * @returns Array of pattern IDs
 */
export function getPatternIds(): string[] {
  return PATTERNS.map((p) => p.id);
}

/**
 * Check if a pattern ID is valid
 *
 * @param id - Pattern ID to check
 * @returns true if the ID exists in the catalogue
 */
export function isValidPatternId(id: string): boolean {
  return PATTERNS.some((p) => p.id === id);
}
