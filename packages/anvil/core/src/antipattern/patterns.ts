/**
 * Anti-pattern Catalogue
 *
 * Phase 2 of ANVFMT: the primary pattern catalogue is now the compiled
 * `.anvil` registry loaded at module initialisation (AP-001..AP-007, plus
 * new family rules — GS-001, RL-*, DD-*). Legacy HTML/CSS TS patterns
 * (AP-008..AP-013) remain here as static constants until ANVFMT-014/015
 * retires them.
 *
 * The public lookup API (`getPattern`, `getPatternsByCategory`,
 * `getDefaultPatterns`, etc.) is preserved — callers don't need to know
 * where a pattern was sourced from.
 *
 * @module antipattern/patterns
 */

import type { AntiPattern } from './types.js';
import { HTML_PATTERNS } from './patterns-html.js';
import { CSS_PATTERNS } from './patterns-css.js';
import { loadRegistryPatterns } from './registry-loader.js';

// =============================================================================
// Pattern Registry
// =============================================================================

/**
 * Build the full catalogue: compiled `.anvil` patterns first (in their
 * registry-sorted order), then the legacy HTML/CSS patterns.
 *
 * Order within the registry is deterministic (sorted by rule id) so the
 * resulting array is stable across runs and test snapshots.
 */
function buildPatterns(): readonly AntiPattern[] {
  const registryPatterns = loadRegistryPatterns();
  return [...registryPatterns, ...HTML_PATTERNS, ...CSS_PATTERNS];
}

/**
 * All built-in anti-patterns.
 *
 * This array is eagerly built at module load. Tests that need to swap the
 * registry (e.g., via `ANVIL_REGISTRY_PATH` or a fixture) should do so before
 * importing this module, or use `reloadPatterns()` to rebuild.
 */
 
export let PATTERNS: readonly AntiPattern[] = buildPatterns();

/**
 * Rebuild the `PATTERNS` array. Intended for tests that change the registry
 * source between cases (via `resetRegistryCache` + a new registry path).
 */
export function reloadPatterns(): readonly AntiPattern[] {
  PATTERNS = buildPatterns();
  return PATTERNS;
}

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
 * @param id - Pattern ID (e.g., 'AP-001', 'GS-001', 'RL-003')
 * @returns The pattern definition, or undefined if not found
 */
export function getPattern(id: string): AntiPattern | undefined {
  return PATTERNS.find((p) => p.id === id);
}

/**
 * Get all patterns in a category
 */
export function getPatternsByCategory(category: PatternCategory): AntiPattern[] {
  return PATTERNS.filter((p) => p.category === category);
}

/**
 * Get all enabled patterns (respects enabled flag, not optIn)
 */
export function getEnabledPatterns(): AntiPattern[] {
  return PATTERNS.filter((p) => p.enabled);
}

/**
 * Get all default patterns (enabled and not opt-in)
 */
export function getDefaultPatterns(): AntiPattern[] {
  return PATTERNS.filter((p) => p.enabled && !p.optIn);
}

/**
 * Get pattern IDs for all patterns
 */
export function getPatternIds(): string[] {
  return PATTERNS.map((p) => p.id);
}

/**
 * Check if a pattern ID is valid
 */
export function isValidPatternId(id: string): boolean {
  return PATTERNS.some((p) => p.id === id);
}

/**
 * Get patterns in a family (e.g., 'guardrail-suppression'). Returns [] for
 * legacy HTML/CSS patterns which have no family.
 */
export function getPatternsByFamily(family: string): AntiPattern[] {
  return PATTERNS.filter((p) => p.family === family);
}
