import { z } from 'zod';
import { TtlConfigSchema } from '../contracts/temporal.js';
import { EmberConfidenceSchema } from '../contracts/confidence.js';

const withObjectDefaults = <T extends z.ZodTypeAny>(schema: T) =>
  z.preprocess((value) => value ?? {}, schema);

export const EmberDecayConfigSchema = TtlConfigSchema.extend({
  default_ttl_days: z.number().int().min(7).max(90).default(30),
  min_ttl_days: z.number().int().positive().default(7),
  max_ttl_days: z.number().int().positive().default(90),
}).refine(
  (value) =>
    value.min_ttl_days <= value.default_ttl_days && value.default_ttl_days <= value.max_ttl_days,
  {
    message: 'default_ttl_days must be between min_ttl_days and max_ttl_days',
  }
);

export const EmberEvaluationConfigSchema = z.object({
  min_confidence: EmberConfidenceSchema.default(0.3),
  repetition_threshold: z.number().int().positive().default(3),
  escalation_window_hours: z.number().positive().default(24),
});

export const EmberLimitsConfigSchema = z.object({
  max_candidates: z.number().int().positive().default(1000),
  max_proposal_size_kb: z.number().int().positive().default(64),
});

export const EmberLayerConfigSchema = z.object({
  enabled: z.boolean().default(true),
  database: z.string().min(1).default('.anvil/ember.db'),
  decay: withObjectDefaults(EmberDecayConfigSchema),
  evaluation: withObjectDefaults(EmberEvaluationConfigSchema),
  limits: withObjectDefaults(EmberLimitsConfigSchema),
});

export const EmberConfigSchema = z.object({
  ember: withObjectDefaults(EmberLayerConfigSchema),
});

export type EmberDecayConfig = z.infer<typeof EmberDecayConfigSchema>;
export type EmberEvaluationConfig = z.infer<typeof EmberEvaluationConfigSchema>;
export type EmberLimitsConfig = z.infer<typeof EmberLimitsConfigSchema>;
export type EmberLayerConfig = z.infer<typeof EmberLayerConfigSchema>;
export type EmberConfig = z.infer<typeof EmberConfigSchema>;

export const DEFAULT_EMBER_CONFIG: EmberConfig = EmberConfigSchema.parse({});

export function parseEmberConfig(config: unknown): EmberConfig | null {
  const result = EmberConfigSchema.safeParse(config);
  return result.success ? result.data : null;
}
