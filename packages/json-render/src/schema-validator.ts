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
export function validateSpec(spec: unknown): ValidationResult {
  const result = catalog.validate(spec);

  if (result.success) {
    return { valid: true, errors: [] };
  }

  const errors = result.error
    ? result.error.issues.map((issue) => `${issue.path.join('.')}: ${issue.message}`)
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
