/**
 * Anti-pattern Catalogue
 *
 * The catalogue is the compiled `.anvil` registry loaded at module
 * initialisation. Legacy HTML/CSS TypeScript patterns (AP-008..AP-013) were
 * retired under ANVFMT-014/015 — HTML/CSS are out of scope for the `.anvil`
 * format (see D-002); dedicated linters (HTMLHint, Stylelint) cover them.
 *
 * The public lookup API (`getPattern`, `getPatternsByCategory`,
 * `getDefaultPatterns`, etc.) is preserved — callers don't need to know
 * where a pattern was sourced from.
 *
 * @module antipattern/patterns
 */

import type { AntiPattern } from './types.js';
import { loadRegistryPatterns } from './registry-loader.js';

// =============================================================================
// Pattern Registry
// =============================================================================

/**
 * Build the catalogue from the compiled registry. Order is deterministic
 * (sorted by rule id at compile time) so the resulting array is stable
 * across runs and test snapshots.
 */
function buildPatterns(): readonly AntiPattern[] {
  return loadRegistryPatterns();
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
 * Get patterns in a family (e.g., 'guardrail-suppression').
 */
export function getPatternsByFamily(family: string): AntiPattern[] {
  return PATTERNS.filter((p) => p.family === family);
}
