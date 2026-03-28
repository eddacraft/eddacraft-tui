/**
 * Inline stack config schema and helpers.
 *
 * These were previously imported from @eddacraft/anvil-edda-stack but are
 * inlined here to avoid pulling that package into the CLI's publish tree.
 */

import { z } from 'zod';

const StackLayerConfigSchema = z.object({ enabled: z.boolean().default(true) }).passthrough();

const StackValidationConfigSchema = z
  .object({
    check_provenance_integrity: z.boolean().default(true),
    check_schema_compatibility: z.boolean().default(true),
  })
  .passthrough();

export const StackConfigSchema = z
  .object({
    kindling: StackLayerConfigSchema.optional(),
    ember: StackLayerConfigSchema.optional(),
    edda: StackLayerConfigSchema.optional(),
    validation: StackValidationConfigSchema.optional(),
  })
  .passthrough();

export type StackConfig = z.infer<typeof StackConfigSchema>;

export function isLayerEnabled(
  config: StackConfig | undefined,
  layer: 'kindling' | 'ember' | 'edda'
): boolean {
  if (!config) return false;
  return config[layer]?.enabled === true;
}

export function getEnabledLayerCount(config: StackConfig | undefined): number {
  if (!config) return 0;
  let count = 0;
  if (config.kindling?.enabled) count++;
  if (config.ember?.enabled) count++;
  if (config.edda?.enabled) count++;
  return count;
}

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
