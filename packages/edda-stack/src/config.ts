/**
 * Stack Configuration Schema (STACK-012)
 *
 * Zod schemas for validating stack configuration within .anvilrc.
 * These provide strict runtime validation of stack settings.
 *
 * @module @eddacraft/anvil-edda-stack/config
 */

import { z } from 'zod';

// =============================================================================
// Layer Configuration Schemas
// =============================================================================

/**
 * Configuration for a single stack layer
 */
export const StackLayerConfigSchema = z
  .object({
    /** Whether this layer is enabled */
    enabled: z.boolean().default(true),
  })
  .passthrough(); // Allow layer-specific extensions

export type StackLayerConfig = z.infer<typeof StackLayerConfigSchema>;

// =============================================================================
// Validation Configuration Schema
// =============================================================================

/**
 * Validation settings for stack integrity checks
 */
export const StackValidationConfigSchema = z
  .object({
    /** Check that provenance links resolve correctly across layers */
    check_provenance_integrity: z.boolean().default(true),
    /** Check that schemas are compatible between layers */
    check_schema_compatibility: z.boolean().default(true),
  })
  .passthrough();

export type StackValidationConfig = z.infer<typeof StackValidationConfigSchema>;

// =============================================================================
// Stack Configuration Schema
// =============================================================================

/**
 * Stack-wide configuration for the Edda Stack
 */
export const StackConfigSchema = z
  .object({
    /** Kindling layer configuration (observation) */
    kindling: StackLayerConfigSchema.optional(),
    /** Ember layer configuration (candidate memories) */
    ember: StackLayerConfigSchema.optional(),
    /** Edda layer configuration (canonical memories) */
    edda: StackLayerConfigSchema.optional(),
    /** Validation settings for stack integrity checks */
    validation: StackValidationConfigSchema.optional(),
  })
  .passthrough();

export type StackConfig = z.infer<typeof StackConfigSchema>;

// =============================================================================
// Default Configuration
// =============================================================================

/**
 * Default stack configuration with all layers disabled
 */
export const DEFAULT_STACK_CONFIG: StackConfig = {
  kindling: { enabled: false },
  ember: { enabled: false },
  edda: { enabled: false },
  validation: {
    check_provenance_integrity: true,
    check_schema_compatibility: true,
  },
};

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Parse and validate stack configuration
 *
 * @param config - Raw config object from .anvilrc
 * @returns Validated stack config or null if invalid
 */
export function parseStackConfig(config: unknown): StackConfig | null {
  const result = StackConfigSchema.safeParse(config);
  return result.success ? result.data : null;
}

/**
 * Check if a specific layer is enabled in the config
 *
 * @param config - Stack config
 * @param layer - Layer name to check
 * @returns True if layer is explicitly enabled
 */
export function isLayerEnabled(
  config: StackConfig | undefined,
  layer: 'kindling' | 'ember' | 'edda'
): boolean {
  if (!config) return false;
  const layerConfig = config[layer];
  return layerConfig?.enabled === true;
}

/**
 * Get count of enabled layers
 *
 * @param config - Stack config
 * @returns Number of enabled layers (0-3)
 */
export function getEnabledLayerCount(config: StackConfig | undefined): number {
  if (!config) return 0;

  let count = 0;
  if (config.kindling?.enabled) count++;
  if (config.ember?.enabled) count++;
  if (config.edda?.enabled) count++;
  return count;
}

/**
 * Get list of enabled layer names
 *
 * @param config - Stack config
 * @returns Array of enabled layer names
 */
export function getEnabledLayers(
  config: StackConfig | undefined
): Array<'kindling' | 'ember' | 'edda'> {
  if (!config) return [];

  const layers: Array<'kindling' | 'ember' | 'edda'> = [];
  if (config.kindling?.enabled) layers.push('kindling');
  if (config.ember?.enabled) layers.push('ember');
  if (config.edda?.enabled) layers.push('edda');
  return layers;
}
