import type { Spec } from '@json-render/core';

import { catalog } from './catalog-registry.js';

export interface ValidationResult {
  valid: boolean;
  errors: string[];
}

/**
 * Validate a JSON spec against the Anvil catalog.
 *
 * Uses the catalog's built-in Zod validation to check structure,
 * component types, and prop shapes.
 */
// @json-render's schema declares `visible: s.any()` without marking it optional,
// so Zod 4.4+ rejects elements that omit the key. Inject `visible: null` on
// elements that don't have it so omitted-by-design conditions still validate.
function normalizeSpec(spec: unknown): unknown {
  if (!spec || typeof spec !== 'object') return spec;
  const root = spec as Record<string, unknown>;
  const elements = root.elements;
  if (!elements || typeof elements !== 'object') return spec;
  const normalizedElements: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(elements as Record<string, unknown>)) {
    if (value && typeof value === 'object' && !('visible' in (value as object))) {
      normalizedElements[key] = { ...(value as Record<string, unknown>), visible: null };
    } else {
      normalizedElements[key] = value;
    }
  }
  return { ...root, elements: normalizedElements };
}

export function validateSpec(spec: unknown): ValidationResult {
  const result = catalog.validate(normalizeSpec(spec));

  if (result.success) {
    return { valid: true, errors: [] };
  }

  const errors = result.error
    ? result.error.issues.map((issue) => `${issue.path.join('.') || '(root)'}: ${issue.message}`)
    : ['Invalid spec'];

  return { valid: false, errors };
}

/**
 * Return all component names registered in the catalog.
 */
export function getComponentNames(): string[] {
  return catalog.componentNames;
}

// Re-export the Spec type for consumers
export type { Spec };
