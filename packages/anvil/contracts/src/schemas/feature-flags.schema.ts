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

export const EnvironmentNameSchema = z.enum([
  'local',
  'development',
  'preview',
  'demo',
  'production',
]);
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
    // FLAGCAT-002 / gating model (ADR-048): the feature group this flag
    // belongs to (matches an id in groups.json) and an open-set tag list.
    // Optional on the base definition so un-migrated per-surface literals still
    // type-check (FLAGCAT-003/-005). The flags-catalogue loader requires it on
    // every *manifest* flag and validates groups.json membership at module
    // load; the full TS<->Rust<->JSON drift check is FLAGCAT-006.
    primaryGroup: z.string().min(1).optional(),
    tags: z.array(z.string().min(1)).optional(),
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
    // Variant values must match the declared valueType
    const { valueType } = ctx.value;
    variants.forEach((v: { key: string; value: unknown }, i: number) => {
      const { value } = v;
      let mismatch = false;
      if (valueType === 'boolean' && typeof value !== 'boolean') mismatch = true;
      if (valueType === 'string' && typeof value !== 'string') mismatch = true;
      if (valueType === 'number' && typeof value !== 'number') mismatch = true;
      if (
        valueType === 'object' &&
        (typeof value !== 'object' || value === null || Array.isArray(value))
      )
        mismatch = true;
      if (mismatch) {
        ctx.issues.push({
          code: 'custom',
          input: value,
          message: `Variant "${v.key}" value must be a ${valueType}, got ${typeof value}`,
          path: ['variants', i, 'value'],
        });
      }
    });
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
// Gating-model inventories (ADR-048 / 2026-05-19 gating-model spec)
// =============================================================================

// Inventory entries are never deleted once retired — their ids stay reserved
// forever (ADR-041 key-reservation rule, generalised).
export const InventoryEntryStatusSchema = z.enum(['active', 'retired']);
export type InventoryEntryStatus = z.infer<typeof InventoryEntryStatusSchema>;

// ── Audiences (flags/audiences.json) ──
export const AudienceAxisSchema = z.enum(['plan', 'role', 'staff', 'channel']);
export type AudienceAxis = z.infer<typeof AudienceAxisSchema>;

export const FlagAudienceSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  axis: AudienceAxisSchema,
  status: InventoryEntryStatusSchema,
});
export type FlagAudienceEntry = z.infer<typeof FlagAudienceSchema>;

export const FlagAudienceManifestSchema = z
  .object({
    schemaVersion: z.literal(FEATURE_FLAG_SCHEMA_VERSION),
    audiences: z.array(FlagAudienceSchema),
  })
  .check((ctx) => {
    const ids = ctx.value.audiences.map((a: { id: string }) => a.id);
    if (new Set(ids).size !== ids.length) {
      ctx.issues.push({
        code: 'custom',
        input: ctx.value.audiences,
        message: 'Audience ids must be unique within the inventory',
        path: ['audiences'],
      });
    }
  });
export type FlagAudienceManifest = z.infer<typeof FlagAudienceManifestSchema>;

// ── Environments (flags/environments.json) ──
export const FlagEnvironmentSchema = z.object({
  id: EnvironmentNameSchema,
  name: z.string().min(1),
  status: InventoryEntryStatusSchema,
});
export type FlagEnvironmentEntry = z.infer<typeof FlagEnvironmentSchema>;

export const FlagEnvironmentManifestSchema = z
  .object({
    schemaVersion: z.literal(FEATURE_FLAG_SCHEMA_VERSION),
    environments: z.array(FlagEnvironmentSchema),
  })
  .check((ctx) => {
    const ids = ctx.value.environments.map((e: { id: string }) => e.id);
    if (new Set(ids).size !== ids.length) {
      ctx.issues.push({
        code: 'custom',
        input: ctx.value.environments,
        message: 'Environment ids must be unique within the inventory',
        path: ['environments'],
      });
    }
  });
export type FlagEnvironmentManifest = z.infer<typeof FlagEnvironmentManifestSchema>;

// ── Primary groups (flags/groups.json) — defaults carriers per ADR-048 ──
export const FlagGroupSchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  defaultClass: FlagClassSchema,
  defaultAudiences: z.array(z.string().min(1)),
  defaultStatus: FlagStatusSchema,
});
export type FlagGroupEntry = z.infer<typeof FlagGroupSchema>;

export const FlagGroupManifestSchema = z
  .object({
    schemaVersion: z.literal(FEATURE_FLAG_SCHEMA_VERSION),
    groups: z.array(FlagGroupSchema),
  })
  .check((ctx) => {
    const ids = ctx.value.groups.map((g: { id: string }) => g.id);
    if (new Set(ids).size !== ids.length) {
      ctx.issues.push({
        code: 'custom',
        input: ctx.value.groups,
        message: 'Group ids must be unique within the inventory',
        path: ['groups'],
      });
    }
  });
export type FlagGroupManifest = z.infer<typeof FlagGroupManifestSchema>;

// =============================================================================
// Surface registry (ADR-076) — flags/surfaces.json
// =============================================================================
//
// The surface/feature is the catalogue's primary noun. This registry
// back-captures the CLI surface inventory. It is declared data + static checks
// only — runtime cascade-off and auth-list derivation are deferred (ADR-076).

// Current access posture of a surface (descriptive back-capture of today's
// reality, from the 2026-06-08 audit). The entitlement+audience encoding the
// resolver consumes is the later derivation slice; here `access` records the
// observed posture. `open` = ungated.
export const SurfaceAccessSchema = z.enum(['open', 'licence', 'admin-key', 'staff']);
export type SurfaceAccess = z.infer<typeof SurfaceAccessSchema>;

// How a surface is invoked. `system` surfaces (e.g. git hooks) MUST NOT be
// refused by a kill-switch — a non-zero exit breaks the calling tool, not a UX
// message (ADR-076 §3 categorical exception).
export const SurfaceInvocationSchema = z.enum(['user', 'system']);
export type SurfaceInvocation = z.infer<typeof SurfaceInvocationSchema>;

// Category = capability grouping within ADR-048's `cli` surface (ADR-076 §2),
// a defaults carrier for access posture.
export const SurfaceCategorySchema = z.object({
  id: z.string().min(1),
  name: z.string().min(1),
  defaultAccess: SurfaceAccessSchema,
  defaultStatus: InventoryEntryStatusSchema,
});
export type SurfaceCategoryEntry = z.infer<typeof SurfaceCategorySchema>;

export const SurfaceSchema = z.object({
  key: z.string().regex(FLAG_KEY_PATTERN),
  name: z.string().min(1),
  category: z.string().min(1),
  // Override the category's defaultAccess. `open` here overrides a gated
  // category default to ungated (it is an explicit value, not absence).
  access: SurfaceAccessSchema.optional(),
  // The gating audience(s) when access is `staff`/`admin-key`; validated
  // against flags/audiences.json by the loader. The schema rejects audiences
  // unless effectiveAccess is `staff`/`admin-key` (so `open`/`licence` carry
  // none). Descriptive today — runtime enforcement is the deferred resolver
  // slice.
  audiences: z.array(z.string().min(1)).optional(),
  invocation: SurfaceInvocationSchema.default('user'),
  // Inventoried but not policy-managed (foundational plumbing) when false.
  catalogued: z.boolean().default(true),
  // Categorically immune to refusal/kill-switch — recovery-critical surfaces
  // (auth, admin.credential). Enforced open by the schema.
  mustAlwaysBeOpen: z.boolean().default(false),
  // Declared hard dependencies (other surface keys). Drives the static
  // blast-radius/acyclicity check; runtime cascade-off is deferred.
  requires: z.array(z.string().min(1)).default([]),
  status: InventoryEntryStatusSchema.default('active'),
  notes: z.string().optional(),
});
export type SurfaceEntry = z.infer<typeof SurfaceSchema>;

/** Detect a cycle in the `requires` graph; returns the first cycle found. */
function findRequiresCycle(
  surfaces: ReadonlyArray<{ key: string; requires?: string[] }>
): string[] | null {
  const edges = new Map<string, string[]>();
  for (const s of surfaces) edges.set(s.key, s.requires ?? []);
  const WHITE = 0,
    GREY = 1,
    BLACK = 2;
  const colour = new Map<string, number>();
  for (const k of edges.keys()) colour.set(k, WHITE);
  const stack: string[] = [];

  function dfs(node: string): string[] | null {
    colour.set(node, GREY);
    stack.push(node);
    for (const next of edges.get(node) ?? []) {
      if (!edges.has(next)) continue; // missing target reported separately
      const c = colour.get(next);
      if (c === GREY) return [...stack.slice(stack.indexOf(next)), next];
      if (c === WHITE) {
        const found = dfs(next);
        if (found) return found;
      }
    }
    stack.pop();
    colour.set(node, BLACK);
    return null;
  }

  for (const k of edges.keys()) {
    if (colour.get(k) === WHITE) {
      const cycle = dfs(k);
      if (cycle) return cycle;
    }
  }
  return null;
}

export const FlagSurfaceManifestSchema = z
  .object({
    schemaVersion: z.literal(FEATURE_FLAG_SCHEMA_VERSION),
    categories: z.array(SurfaceCategorySchema),
    surfaces: z.array(SurfaceSchema),
  })
  .check((ctx) => {
    const { categories, surfaces } = ctx.value;

    const categoryIds = new Set(categories.map((c) => c.id));
    if (categoryIds.size !== categories.length) {
      ctx.issues.push({
        code: 'custom',
        input: categories,
        message: 'Category ids must be unique within the registry',
        path: ['categories'],
      });
    }

    const surfaceKeys = surfaces.map((s) => s.key);
    const surfaceKeySet = new Set(surfaceKeys);
    if (surfaceKeySet.size !== surfaceKeys.length) {
      ctx.issues.push({
        code: 'custom',
        input: surfaces,
        message: 'Surface keys must be unique within the registry',
        path: ['surfaces'],
      });
    }

    surfaces.forEach((s, i) => {
      if (!categoryIds.has(s.category)) {
        ctx.issues.push({
          code: 'custom',
          input: s.category,
          message: `Surface "${s.key}" references unknown category "${s.category}"`,
          path: ['surfaces', i, 'category'],
        });
      }
      for (const dep of s.requires ?? []) {
        if (!surfaceKeySet.has(dep)) {
          ctx.issues.push({
            code: 'custom',
            input: dep,
            message: `Surface "${s.key}" requires unknown surface "${dep}"`,
            path: ['surfaces', i, 'requires'],
          });
        }
      }
      const cat = categories.find((c) => c.id === s.category);
      const effectiveAccess = s.access ?? cat?.defaultAccess ?? 'open';
      // Recovery-critical surfaces must be open — you cannot pin-open a gate.
      if (s.mustAlwaysBeOpen && effectiveAccess !== 'open') {
        ctx.issues.push({
          code: 'custom',
          input: effectiveAccess,
          message: `Surface "${s.key}" is mustAlwaysBeOpen but resolves to gated access "${effectiveAccess}"`,
          path: ['surfaces', i, 'mustAlwaysBeOpen'],
        });
      }
      // Audiences only make sense for an audience-gated posture.
      if (
        s.audiences &&
        s.audiences.length > 0 &&
        !['staff', 'admin-key'].includes(effectiveAccess)
      ) {
        ctx.issues.push({
          code: 'custom',
          input: s.audiences,
          message: `Surface "${s.key}" declares audiences but access "${effectiveAccess}" is not audience-gated`,
          path: ['surfaces', i, 'audiences'],
        });
      }
    });

    // Cycle detection is only meaningful on a key-unique graph — duplicate
    // keys collapse edges in the adjacency map and could mask a cycle. Skip it
    // when duplicates are already flagged above.
    if (surfaceKeySet.size === surfaceKeys.length) {
      const cycle = findRequiresCycle(surfaces);
      if (cycle) {
        ctx.issues.push({
          code: 'custom',
          input: cycle,
          message: `requires graph must be acyclic; found cycle: ${cycle.join(' → ')}`,
          path: ['surfaces'],
        });
      }
    }
  });
export type FlagSurfaceManifest = z.infer<typeof FlagSurfaceManifestSchema>;

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
