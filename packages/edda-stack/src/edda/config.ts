import { z } from 'zod';
import { EmberConfidenceSchema } from '../contracts/confidence.js';

const withObjectDefaults = <T extends z.ZodTypeAny>(schema: T) =>
  z.preprocess((value) => value ?? {}, schema);

export const EddaStorageConfigSchema = z.object({
  type: z.literal('git').default('git'),
  path: z.string().min(1).default('.anvil/edda/'),
  format: z.literal('yaml').default('yaml'),
});

export const EddaPromotionConfigSchema = z.object({
  require_reason: z.boolean().default(true),
  require_attribution: z.boolean().default(true),
  min_ember_confidence: EmberConfidenceSchema.default(0.5),
});

export const EddaLimitsConfigSchema = z.object({
  max_statement_length: z.number().int().positive().default(2000),
  max_context_length: z.number().int().positive().default(10000),
});

export const EddaLayerConfigSchema = z.object({
  enabled: z.boolean().default(true),
  storage: withObjectDefaults(EddaStorageConfigSchema),
  promotion: withObjectDefaults(EddaPromotionConfigSchema),
  limits: withObjectDefaults(EddaLimitsConfigSchema),
});

export const EddaConfigSchema = z.object({
  edda: withObjectDefaults(EddaLayerConfigSchema),
});

export type EddaStorageConfig = z.infer<typeof EddaStorageConfigSchema>;
export type EddaPromotionConfig = z.infer<typeof EddaPromotionConfigSchema>;
export type EddaLimitsConfig = z.infer<typeof EddaLimitsConfigSchema>;
export type EddaLayerConfig = z.infer<typeof EddaLayerConfigSchema>;
export type EddaConfig = z.infer<typeof EddaConfigSchema>;

export const DEFAULT_EDDA_CONFIG: EddaConfig = EddaConfigSchema.parse({});

export function parseEddaConfig(config: unknown): EddaConfig | null {
  const result = EddaConfigSchema.safeParse(config);
  return result.success ? result.data : null;
}
