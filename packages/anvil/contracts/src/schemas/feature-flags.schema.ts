import { z } from 'zod';

export const FEATURE_FLAG_SCHEMA_VERSION = 1;

// =============================================================================
// Flag Class
// =============================================================================

export const FlagClassSchema = z.enum(['rollout', 'entitlement', 'ops_kill_switch']);
export type FlagClass = z.infer<typeof FlagClassSchema>;

// =============================================================================
// Flag Status
// =============================================================================

export const FlagStatusSchema = z.enum(['draft', 'active', 'retiring', 'retired']);
export type FlagStatus = z.infer<typeof FlagStatusSchema>;

// =============================================================================
// Flag Value Type (OpenFeature-aligned)
// =============================================================================

export const FlagValueTypeSchema = z.enum(['boolean', 'string', 'number', 'object']);
export type FlagValueType = z.infer<typeof FlagValueTypeSchema>;

// =============================================================================
// Flag Variant
// =============================================================================

const FlagObjectPropertyValueSchema = z.union([z.boolean(), z.string(), z.number()]);
const FlagObjectValueSchema = z.record(z.string(), FlagObjectPropertyValueSchema);

export const FlagVariantSchema = z.object({
  key: z.string().min(1),
  value: z.union([z.boolean(), z.string(), z.number(), FlagObjectValueSchema]),
});
export type FlagVariant = z.infer<typeof FlagVariantSchema>;

// =============================================================================
// Environment Targeting
// =============================================================================

export const EnvironmentNameSchema = z.enum(['local', 'preview', 'dev', 'staging', 'prod']);
export type EnvironmentName = z.infer<typeof EnvironmentNameSchema>;

export const ChannelSchema = z.enum(['development', 'beta', 'production']);
export type Channel = z.infer<typeof ChannelSchema>;

export const EnvironmentContextSchema = z.object({
  environment: EnvironmentNameSchema,
  channel: ChannelSchema.optional(),
  deploymentRing: z.string().optional(),
});
export type EnvironmentContext = z.infer<typeof EnvironmentContextSchema>;

// =============================================================================
// Audience Targeting
// =============================================================================

export const AudienceContextSchema = z.object({
  accountTier: z.string().optional(),
  licencePlan: z.string().optional(),
  organisationId: z.string().optional(),
  userRole: z.string().optional(),
  cohort: z.string().optional(),
});
export type AudienceContext = z.infer<typeof AudienceContextSchema>;

// =============================================================================
// Evaluation Context (OpenFeature-aligned)
// =============================================================================

export const EvaluationContextSchema = z.object({
  targetingKey: z.string().min(1),
  environment: EnvironmentContextSchema,
  audience: AudienceContextSchema.optional(),
});
export type EvaluationContext = z.infer<typeof EvaluationContextSchema>;

// =============================================================================
// Targeting Operators
// =============================================================================

export const TargetingOperatorSchema = z.enum([
  'equals',
  'not_equals',
  'in_set',
  'not_in_set',
  'percentage',
  'segment',
]);
export type TargetingOperator = z.infer<typeof TargetingOperatorSchema>;

// =============================================================================
// Targeting Rule
// =============================================================================

const STRING_OPERATORS = ['equals', 'not_equals', 'segment'] as const;
const SET_OPERATORS = ['in_set', 'not_in_set'] as const;

export const TargetingConditionSchema = z
  .object({
    attribute: z.string().min(1),
    operator: TargetingOperatorSchema,
    value: z.union([z.string(), z.number(), z.array(z.string())]),
  })
  .check((ctx) => {
    const { operator, value } = ctx.value;
    if ((STRING_OPERATORS as readonly string[]).includes(operator)) {
      if (typeof value !== 'string') {
        ctx.issues.push({
          code: 'custom',
          input: value,
          message: `Operator "${operator}" requires a string value`,
          path: ['value'],
        });
      }
    } else if ((SET_OPERATORS as readonly string[]).includes(operator)) {
      if (!Array.isArray(value)) {
        ctx.issues.push({
          code: 'custom',
          input: value,
          message: `Operator "${operator}" requires an array value`,
          path: ['value'],
        });
      }
    } else if (operator === 'percentage') {
      if (typeof value !== 'number') {
        ctx.issues.push({
          code: 'custom',
          input: value,
          message: 'Operator "percentage" requires a numeric value',
          path: ['value'],
        });
      }
    }
  });
export type TargetingCondition = z.infer<typeof TargetingConditionSchema>;

export const TargetingRuleSchema = z.object({
  conditions: z.array(TargetingConditionSchema).min(1),
  variant: z.string().min(1),
});
export type TargetingRule = z.infer<typeof TargetingRuleSchema>;

// =============================================================================
// Feature Flag Definition
// =============================================================================

const FLAG_KEY_PATTERN = /^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$/;

export const FeatureFlagDefinitionSchema = z
  .object({
    key: z.string().regex(FLAG_KEY_PATTERN),
    owner: z.string().min(1),
    intent: z.string().min(1),
    class: FlagClassSchema,
    valueType: FlagValueTypeSchema,
    variants: z.array(FlagVariantSchema).min(2),
    defaultVariant: z.string().min(1),
    status: FlagStatusSchema,
    createdFor: z.string().min(1),
    expiryOrReviewDate: z.iso.datetime().optional(),
    description: z.string().optional(),
    targeting: z.array(TargetingRuleSchema).optional(),
  })
  .check((ctx) => {
    const { class: flagClass, variants, defaultVariant, expiryOrReviewDate } = ctx.value;
    // C-011: rollout flags must define an expiry or review date
    if (flagClass === 'rollout' && expiryOrReviewDate === undefined) {
      ctx.issues.push({
        code: 'custom',
        input: expiryOrReviewDate,
        message: 'Rollout flags must define expiryOrReviewDate',
        path: ['expiryOrReviewDate'],
      });
    }
    // C-010: defaultVariant must reference an existing variant key
    if (!variants.some((v: { key: string }) => v.key === defaultVariant)) {
      ctx.issues.push({
        code: 'custom',
        input: defaultVariant,
        message: `defaultVariant "${defaultVariant}" must reference an existing variant key`,
        path: ['defaultVariant'],
      });
    }
    // C-008: variant keys must be unique
    const keys = variants.map((v: { key: string }) => v.key);
    if (new Set(keys).size !== keys.length) {
      ctx.issues.push({
        code: 'custom',
        input: variants,
        message: 'Variant keys must be unique within a flag',
        path: ['variants'],
      });
    }
  });

export type FeatureFlagDefinition = z.infer<typeof FeatureFlagDefinitionSchema>;

// =============================================================================
// Feature Flag Manifest
// =============================================================================

export const FeatureFlagManifestSchema = z
  .object({
    schemaVersion: z.literal(FEATURE_FLAG_SCHEMA_VERSION),
    flags: z.array(FeatureFlagDefinitionSchema),
  })
  .check((ctx) => {
    // C-008: flag keys must be unique across the manifest
    const keys = ctx.value.flags.map((f: { key: string }) => f.key);
    if (new Set(keys).size !== keys.length) {
      ctx.issues.push({
        code: 'custom',
        input: ctx.value.flags,
        message: 'Flag keys must be unique within the manifest',
        path: ['flags'],
      });
    }
  });

export type FeatureFlagManifest = z.infer<typeof FeatureFlagManifestSchema>;

// =============================================================================
// Validation
// =============================================================================

export interface ManifestValidationResult {
  success: boolean;
  data?: FeatureFlagManifest;
  errors?: Array<{ path: string; message: string }>;
}

export function validateManifest(data: unknown): ManifestValidationResult {
  const result = FeatureFlagManifestSchema.safeParse(data);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return {
    success: false,
    errors: result.error.issues.map((i) => ({
      path: i.path.join('.'),
      message: i.message,
    })),
  };
}

export function defaultVariantExists(flag: FeatureFlagDefinition): boolean {
  return flag.variants.some((v) => v.key === flag.defaultVariant);
}

export function failClosedClasses(): readonly FlagClass[] {
  return ['ops_kill_switch', 'entitlement'] as const;
}
