import { z } from 'zod';

export const FEATURE_FLAG_SCHEMA_VERSION = 1;
export const PRODUCT_CATALOGUE_SCHEMA_VERSION = 2;

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
    // FLAGCAT-013: product-feature keys this operational flag controls.
    // Optional on the base definition so un-migrated literals still type-check;
    // the flags-catalogue loader requires it on every manifest flag.
    controlsProductFeatures: z.array(z.string().regex(FLAG_KEY_PATTERN)).optional(),
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
    const controlledFeatures = ctx.value.controlsProductFeatures ?? [];
    if (new Set(controlledFeatures).size !== controlledFeatures.length) {
      ctx.issues.push({
        code: 'custom',
        input: controlledFeatures,
        message: 'controlsProductFeatures keys must be unique within a flag',
        path: ['controlsProductFeatures'],
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
export const SurfaceCategorySchema = z
  .object({
    id: z.string().min(1),
    name: z.string().min(1),
    defaultAccess: SurfaceAccessSchema,
    defaultStatus: InventoryEntryStatusSchema,
  })
  .strict();
export type SurfaceCategoryEntry = z.infer<typeof SurfaceCategorySchema>;

export const SurfaceSchema = z
  .object({
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
  })
  .strict();
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

export const ProductCatalogueV1Schema = z
  .object({
    schemaVersion: z.literal(FEATURE_FLAG_SCHEMA_VERSION),
    categories: z.array(SurfaceCategorySchema),
    surfaces: z.array(SurfaceSchema),
  })
  .strict()
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
export type ProductCatalogueV1 = z.infer<typeof ProductCatalogueV1Schema>;

/** @deprecated Use ProductCatalogueV1Schema during the v1 compatibility window. */
export const FlagSurfaceManifestSchema = ProductCatalogueV1Schema;

/** @deprecated Use ProductCatalogueV1 during the v1 compatibility window. */
export type FlagSurfaceManifest = ProductCatalogueV1;

// =============================================================================
// Product catalogue v2 (ADR-076 / FLAGCAT-011)
// =============================================================================

const PRODUCT_KEY_PATTERN = /^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$/;
const DELIVERY_SURFACE_KEY_PATTERN =
  /^(?:cli|mcp-tool|mcp-resource|api|daemon|dashboard|docs|hook|integration)\.[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$/;
const APS_MODULE_OWNER_PATTERN = /^[A-Z][A-Z0-9]*$/;

export const ProductCatalogueLifecycleSchema = z.enum(['active', 'retired']);
export type ProductCatalogueLifecycle = z.infer<typeof ProductCatalogueLifecycleSchema>;

export const ProductSurfacePostureDefaultsSchema = z
  .object({
    access: SurfaceAccessSchema,
  })
  .strict();
export type ProductSurfacePostureDefaults = z.infer<typeof ProductSurfacePostureDefaultsSchema>;

export const ProductFeatureGroupSchema = z
  .object({
    key: z.string().regex(PRODUCT_KEY_PATTERN),
    name: z.string().min(1),
    defaultSurfacePosture: ProductSurfacePostureDefaultsSchema,
    status: ProductCatalogueLifecycleSchema,
  })
  .strict();
export type ProductFeatureGroup = z.infer<typeof ProductFeatureGroupSchema>;

export const ProductFeatureFlagLinkageSchema = z.discriminatedUnion('disposition', [
  z
    .object({
      disposition: z.literal('linked'),
      flagKeys: z.array(z.string().regex(FLAG_KEY_PATTERN)).min(1),
    })
    .strict()
    .check((ctx) => {
      const { flagKeys } = ctx.value;
      if (new Set(flagKeys).size !== flagKeys.length) {
        ctx.issues.push({
          code: 'custom',
          input: flagKeys,
          message: 'Linked flag keys must be unique',
          path: ['flagKeys'],
        });
      }
    }),
  z
    .object({
      disposition: z.literal('unflagged'),
      reason: z.string().min(1),
    })
    .strict(),
]);
export type ProductFeatureFlagLinkage = z.infer<typeof ProductFeatureFlagLinkageSchema>;

export const CANONICAL_PLAN_AXIS_AUDIENCE_IDS = [
  'plan-free',
  'plan-beta',
  'plan-pro',
  'plan-enterprise',
] as const;
export type CanonicalPlanAxisAudienceId = (typeof CANONICAL_PLAN_AXIS_AUDIENCE_IDS)[number];

export const PlanAvailabilityDispositionSchema = z.enum(['available', 'unavailable', 'undecided']);
export type PlanAvailabilityDisposition = z.infer<typeof PlanAvailabilityDispositionSchema>;

export const UNDECIDED_PLAN_AVAILABILITY = {
  'plan-free': 'undecided',
  'plan-beta': 'undecided',
  'plan-pro': 'undecided',
  'plan-enterprise': 'undecided',
} as const satisfies Record<CanonicalPlanAxisAudienceId, 'undecided'>;

export const ProductFeaturePlanAvailabilitySchema = z
  .object({
    'plan-free': PlanAvailabilityDispositionSchema,
    'plan-beta': PlanAvailabilityDispositionSchema,
    'plan-pro': PlanAvailabilityDispositionSchema,
    'plan-enterprise': PlanAvailabilityDispositionSchema,
  })
  .strict();
export type ProductFeaturePlanAvailability = z.infer<typeof ProductFeaturePlanAvailabilitySchema>;

export const ProductFeatureSchema = z
  .object({
    key: z.string().regex(PRODUCT_KEY_PATTERN),
    name: z.string().min(1),
    groupKey: z.string().regex(PRODUCT_KEY_PATTERN),
    owner: z.string().regex(APS_MODULE_OWNER_PATTERN),
    status: ProductCatalogueLifecycleSchema,
    requires: z.array(z.string().regex(PRODUCT_KEY_PATTERN)),
    flagLinkage: ProductFeatureFlagLinkageSchema,
    planAvailability: ProductFeaturePlanAvailabilitySchema,
    notes: z.string().min(1).optional(),
  })
  .strict();
export type ProductFeature = z.infer<typeof ProductFeatureSchema>;

const CliDeliveryLocatorSchema = z
  .object({
    kind: z.literal('cli'),
    commandPath: z.array(z.string().min(1)),
  })
  .strict();
const McpToolDeliveryLocatorSchema = z
  .object({ kind: z.literal('mcp-tool'), name: z.string().min(1) })
  .strict();
const McpResourceDeliveryLocatorSchema = z
  .object({ kind: z.literal('mcp-resource'), uri: z.string().min(1) })
  .strict();
const ApiRouteDeliveryLocatorSchema = z
  .object({
    kind: z.literal('api-route'),
    method: z.string().min(1),
    path: z.string().min(1),
  })
  .strict();
const DaemonRpcDeliveryLocatorSchema = z
  .object({ kind: z.literal('daemon-rpc'), method: z.string().min(1) })
  .strict();
const DashboardRouteDeliveryLocatorSchema = z
  .object({ kind: z.literal('dashboard-route'), path: z.string().min(1) })
  .strict();
const DocsRouteDeliveryLocatorSchema = z
  .object({ kind: z.literal('docs-route'), pathPrefix: z.string().min(1) })
  .strict();
const HookDeliveryLocatorSchema = z
  .object({ kind: z.literal('hook'), hook: z.string().min(1) })
  .strict();
const IntegrationDeliveryLocatorSchema = z
  .object({
    kind: z.literal('integration'),
    integrationId: z.string().min(1),
    capability: z.string().min(1),
  })
  .strict();

export const DeliverySurfaceLocatorSchema = z.discriminatedUnion('kind', [
  CliDeliveryLocatorSchema,
  McpToolDeliveryLocatorSchema,
  McpResourceDeliveryLocatorSchema,
  ApiRouteDeliveryLocatorSchema,
  DaemonRpcDeliveryLocatorSchema,
  DashboardRouteDeliveryLocatorSchema,
  DocsRouteDeliveryLocatorSchema,
  HookDeliveryLocatorSchema,
  IntegrationDeliveryLocatorSchema,
]);
export type DeliverySurfaceLocator = z.infer<typeof DeliverySurfaceLocatorSchema>;

export const DeliverySurfacePostureSchema = z
  .object({
    access: SurfaceAccessSchema.optional(),
    audiences: z.array(z.string().min(1)).optional(),
    invocation: SurfaceInvocationSchema,
    mustAlwaysBeOpen: z.boolean(),
  })
  .strict();
export type DeliverySurfacePosture = z.infer<typeof DeliverySurfacePostureSchema>;

export const DeliverySurfaceSchema = z
  .object({
    key: z.string().regex(DELIVERY_SURFACE_KEY_PATTERN),
    featureKey: z.string().regex(PRODUCT_KEY_PATTERN),
    locator: DeliverySurfaceLocatorSchema,
    posture: DeliverySurfacePostureSchema,
    status: ProductCatalogueLifecycleSchema,
  })
  .strict();
export type DeliverySurface = z.infer<typeof DeliverySurfaceSchema>;

export const ExcludedDeliverySurfaceSchema = z
  .object({
    key: z.string().regex(DELIVERY_SURFACE_KEY_PATTERN),
    locator: DeliverySurfaceLocatorSchema,
    owner: z.string().regex(APS_MODULE_OWNER_PATTERN),
    classification: z.literal('internal-plumbing'),
    reason: z.string().min(1),
    reviewReference: z.string().min(1),
    status: ProductCatalogueLifecycleSchema,
  })
  .strict();
export type ExcludedDeliverySurface = z.infer<typeof ExcludedDeliverySurfaceSchema>;

export const DeliverySurfaceMigrationSchema = z
  .object({
    fromKeys: z.array(z.string().regex(DELIVERY_SURFACE_KEY_PATTERN)).min(1),
    toKeys: z.array(z.string().regex(DELIVERY_SURFACE_KEY_PATTERN)).min(1),
  })
  .strict();
export type DeliverySurfaceMigration = z.infer<typeof DeliverySurfaceMigrationSchema>;

export const ProductCatalogueManifestSchema = z
  .object({
    schemaVersion: z.literal(PRODUCT_CATALOGUE_SCHEMA_VERSION),
    productFeatureGroups: z.array(ProductFeatureGroupSchema),
    productFeatures: z.array(ProductFeatureSchema),
    deliverySurfaces: z.array(DeliverySurfaceSchema),
    excludedDeliverySurfaces: z.array(ExcludedDeliverySurfaceSchema),
    deliverySurfaceMigrations: z.array(DeliverySurfaceMigrationSchema),
  })
  .strict()
  .check((ctx) => {
    const {
      productFeatureGroups,
      productFeatures,
      deliverySurfaces,
      excludedDeliverySurfaces,
      deliverySurfaceMigrations,
    } = ctx.value;

    const groupKeys = productFeatureGroups.map((group) => group.key);
    const groupKeySet = new Set(groupKeys);
    if (groupKeySet.size !== groupKeys.length) {
      ctx.issues.push({
        code: 'custom',
        input: productFeatureGroups,
        message: 'Product feature group keys must be unique within the catalogue',
        path: ['productFeatureGroups'],
      });
    }

    const featureKeys = productFeatures.map((feature) => feature.key);
    const featureKeySet = new Set(featureKeys);
    if (featureKeySet.size !== featureKeys.length) {
      ctx.issues.push({
        code: 'custom',
        input: productFeatures,
        message: 'Product feature keys must be unique within the catalogue',
        path: ['productFeatures'],
      });
    }

    const deliveryKeys = [
      ...deliverySurfaces.map((surface) => surface.key),
      ...excludedDeliverySurfaces.map((surface) => surface.key),
    ];
    if (new Set(deliveryKeys).size !== deliveryKeys.length) {
      ctx.issues.push({
        code: 'custom',
        input: deliveryKeys,
        message: 'Delivery surface keys must be unique within the catalogue',
        path: ['deliverySurfaces'],
      });
    }

    const deliveryIdentities = new Map(
      [...deliverySurfaces, ...excludedDeliverySurfaces].map((surface) => [surface.key, surface])
    );
    const migratedFromKeys = new Set<string>();
    deliverySurfaceMigrations.forEach((migration, migrationIndex) => {
      const uniqueFromKeys = new Set(migration.fromKeys);
      if (uniqueFromKeys.size !== migration.fromKeys.length) {
        ctx.issues.push({
          code: 'custom',
          input: migration.fromKeys,
          message: 'Delivery surface migration source keys must be unique within the migration',
          path: ['deliverySurfaceMigrations', migrationIndex, 'fromKeys'],
        });
      }

      const uniqueToKeys = new Set(migration.toKeys);
      if (uniqueToKeys.size !== migration.toKeys.length) {
        ctx.issues.push({
          code: 'custom',
          input: migration.toKeys,
          message: 'Delivery surface migration target keys must be unique within the migration',
          path: ['deliverySurfaceMigrations', migrationIndex, 'toKeys'],
        });
      }

      for (const fromKey of uniqueFromKeys) {
        if (migratedFromKeys.has(fromKey)) {
          ctx.issues.push({
            code: 'custom',
            input: fromKey,
            message: `Delivery surface migration source "${fromKey}" belongs to more than one migration`,
            path: ['deliverySurfaceMigrations', migrationIndex, 'fromKeys'],
          });
        }
        migratedFromKeys.add(fromKey);

        const source = deliveryIdentities.get(fromKey);
        if (!source) {
          ctx.issues.push({
            code: 'custom',
            input: fromKey,
            message: `Delivery surface migration references unknown source "${fromKey}"`,
            path: ['deliverySurfaceMigrations', migrationIndex, 'fromKeys'],
          });
        } else if (source.status !== 'retired') {
          ctx.issues.push({
            code: 'custom',
            input: fromKey,
            message: `Delivery surface migration source "${fromKey}" must remain reserved with retired status`,
            path: ['deliverySurfaceMigrations', migrationIndex, 'fromKeys'],
          });
        }
      }

      for (const toKey of uniqueToKeys) {
        if (uniqueFromKeys.has(toKey)) {
          ctx.issues.push({
            code: 'custom',
            input: toKey,
            message: `Delivery surface migration cannot reuse source "${toKey}" as a target`,
            path: ['deliverySurfaceMigrations', migrationIndex, 'toKeys'],
          });
        }

        const target = deliveryIdentities.get(toKey);
        if (!target) {
          ctx.issues.push({
            code: 'custom',
            input: toKey,
            message: `Delivery surface migration references unknown target "${toKey}"`,
            path: ['deliverySurfaceMigrations', migrationIndex, 'toKeys'],
          });
        } else if (target.status !== 'active') {
          ctx.issues.push({
            code: 'custom',
            input: toKey,
            message: `Delivery surface migration target "${toKey}" must be active`,
            path: ['deliverySurfaceMigrations', migrationIndex, 'toKeys'],
          });
        }
      }
    });

    const requireMigrationHistory = (
      surface: DeliverySurface | ExcludedDeliverySurface,
      collection: 'deliverySurfaces' | 'excludedDeliverySurfaces',
      index: number
    ): void => {
      if (surface.status === 'retired' && !migratedFromKeys.has(surface.key)) {
        ctx.issues.push({
          code: 'custom',
          input: surface.key,
          message: `Retired delivery surface "${surface.key}" requires migration history`,
          path: [collection, index, 'status'],
        });
      }
    };

    deliverySurfaces.forEach((surface, index) => {
      requireMigrationHistory(surface, 'deliverySurfaces', index);
    });
    excludedDeliverySurfaces.forEach((surface, index) => {
      requireMigrationHistory(surface, 'excludedDeliverySurfaces', index);
    });

    productFeatures.forEach((feature, index) => {
      if (!groupKeySet.has(feature.groupKey)) {
        ctx.issues.push({
          code: 'custom',
          input: feature.groupKey,
          message: `Product feature "${feature.key}" references unknown group "${feature.groupKey}"`,
          path: ['productFeatures', index, 'groupKey'],
        });
      }
      const uniqueRequires = new Set(feature.requires);
      if (uniqueRequires.size !== feature.requires.length) {
        ctx.issues.push({
          code: 'custom',
          input: feature.requires,
          message: `Product feature "${feature.key}" has duplicate requires entries`,
          path: ['productFeatures', index, 'requires'],
        });
      }
      for (const requirement of uniqueRequires) {
        if (!featureKeySet.has(requirement)) {
          ctx.issues.push({
            code: 'custom',
            input: requirement,
            message: `Product feature "${feature.key}" requires unknown feature "${requirement}"`,
            path: ['productFeatures', index, 'requires'],
          });
        }
      }
    });

    if (featureKeySet.size === featureKeys.length) {
      const cycle = findRequiresCycle(productFeatures);
      if (cycle) {
        ctx.issues.push({
          code: 'custom',
          input: cycle,
          message: `Product feature requires graph must be acyclic; found cycle: ${cycle.join(' → ')}`,
          path: ['productFeatures'],
        });
      }
    }

    const expectedLocatorKindByHost: Readonly<Record<string, DeliverySurfaceLocator['kind']>> = {
      cli: 'cli',
      'mcp-tool': 'mcp-tool',
      'mcp-resource': 'mcp-resource',
      api: 'api-route',
      daemon: 'daemon-rpc',
      dashboard: 'dashboard-route',
      docs: 'docs-route',
      hook: 'hook',
      integration: 'integration',
    };

    const validateDeliveryIdentity = (
      surface: DeliverySurface | ExcludedDeliverySurface,
      collection: 'deliverySurfaces' | 'excludedDeliverySurfaces',
      index: number
    ): void => {
      const host = surface.key.slice(0, surface.key.indexOf('.'));
      const expectedKind = expectedLocatorKindByHost[host];
      if (surface.locator.kind !== expectedKind) {
        ctx.issues.push({
          code: 'custom',
          input: surface.locator.kind,
          message: `Delivery surface "${surface.key}" requires locator kind "${expectedKind}"`,
          path: [collection, index, 'locator', 'kind'],
        });
      }
    };

    deliverySurfaces.forEach((surface, index) => {
      validateDeliveryIdentity(surface, 'deliverySurfaces', index);
      if (!featureKeySet.has(surface.featureKey)) {
        ctx.issues.push({
          code: 'custom',
          input: surface.featureKey,
          message: `Delivery surface "${surface.key}" references unknown feature "${surface.featureKey}"`,
          path: ['deliverySurfaces', index, 'featureKey'],
        });
        return;
      }

      const feature = productFeatures.find((candidate) => candidate.key === surface.featureKey);
      const group = productFeatureGroups.find((candidate) => candidate.key === feature?.groupKey);
      const effectiveAccess = surface.posture.access ?? group?.defaultSurfacePosture.access;
      if (surface.posture.mustAlwaysBeOpen && effectiveAccess !== 'open') {
        ctx.issues.push({
          code: 'custom',
          input: effectiveAccess,
          message: `Delivery surface "${surface.key}" is mustAlwaysBeOpen but resolves to gated access "${effectiveAccess}"`,
          path: ['deliverySurfaces', index, 'posture', 'mustAlwaysBeOpen'],
        });
      }
      if (
        surface.posture.audiences &&
        surface.posture.audiences.length > 0 &&
        effectiveAccess !== 'staff' &&
        effectiveAccess !== 'admin-key'
      ) {
        ctx.issues.push({
          code: 'custom',
          input: surface.posture.audiences,
          message: `Delivery surface "${surface.key}" declares audiences but access "${effectiveAccess}" is not audience-gated`,
          path: ['deliverySurfaces', index, 'posture', 'audiences'],
        });
      }
      if (
        surface.posture.audiences &&
        new Set(surface.posture.audiences).size !== surface.posture.audiences.length
      ) {
        ctx.issues.push({
          code: 'custom',
          input: surface.posture.audiences,
          message: `Delivery surface "${surface.key}" has duplicate audience references`,
          path: ['deliverySurfaces', index, 'posture', 'audiences'],
        });
      }
    });

    excludedDeliverySurfaces.forEach((surface, index) => {
      validateDeliveryIdentity(surface, 'excludedDeliverySurfaces', index);
    });
  });
export type ProductCatalogueManifest = z.infer<typeof ProductCatalogueManifestSchema>;

export const ProductCatalogueV1MigrationEntrySchema = z
  .object({
    owner: z.string().regex(APS_MODULE_OWNER_PATTERN),
    deliveryKey: z.string().regex(DELIVERY_SURFACE_KEY_PATTERN),
    locator: DeliverySurfaceLocatorSchema,
  })
  .strict();
export type ProductCatalogueV1MigrationEntry = z.infer<
  typeof ProductCatalogueV1MigrationEntrySchema
>;
export type ProductCatalogueV1MigrationMap = Readonly<
  Record<string, ProductCatalogueV1MigrationEntry>
>;

/**
 * Pure, explicit v1-to-v2 migration.
 *
 * v1 does not contain product ownership or stable delivery identities. Callers
 * must curate both through `migrationByFeatureKey`; this function never derives
 * a locator from a display name or turns `catalogued: false` into an exclusion.
 */
export function normaliseProductCatalogueV1(
  input: z.input<typeof ProductCatalogueV1Schema>,
  migrationByFeatureKey: ProductCatalogueV1MigrationMap
): ProductCatalogueManifest {
  const v1 = ProductCatalogueV1Schema.parse(input);
  const featureKeys = new Set(v1.surfaces.map((surface) => surface.key));
  const migrationKeys = Object.keys(migrationByFeatureKey);

  const missingMigrationKeys = [...featureKeys].filter(
    (key) => !Object.prototype.hasOwnProperty.call(migrationByFeatureKey, key)
  );
  if (missingMigrationKeys.length > 0) {
    throw new Error(
      `Missing v1 product-catalogue migration entries: ${missingMigrationKeys.join(', ')}`
    );
  }

  const unexpectedMigrationKeys = migrationKeys.filter((key) => !featureKeys.has(key));
  if (unexpectedMigrationKeys.length > 0) {
    throw new Error(
      `Unexpected v1 product-catalogue migration entries: ${unexpectedMigrationKeys.join(', ')}`
    );
  }

  const migrations = new Map(
    migrationKeys.map((key) => [
      key,
      ProductCatalogueV1MigrationEntrySchema.parse(migrationByFeatureKey[key]),
    ])
  );
  const categoriesById = new Map(v1.categories.map((category) => [category.id, category]));

  return ProductCatalogueManifestSchema.parse({
    schemaVersion: PRODUCT_CATALOGUE_SCHEMA_VERSION,
    productFeatureGroups: v1.categories.map((category) => ({
      key: category.id,
      name: category.name,
      defaultSurfacePosture: {
        access: category.defaultAccess,
      },
      status: category.defaultStatus,
    })),
    productFeatures: v1.surfaces.map((surface) => {
      const migration = migrations.get(surface.key);
      if (!migration) {
        throw new Error(`Missing migration entry for v1 product feature "${surface.key}"`);
      }
      return {
        key: surface.key,
        name: surface.name,
        groupKey: surface.category,
        owner: migration.owner,
        status: surface.status,
        requires: surface.requires,
        flagLinkage: {
          disposition: 'unflagged',
          reason: 'v1 compatibility projection; operational-flag linkage is canonical v2-only',
        },
        planAvailability: { ...UNDECIDED_PLAN_AVAILABILITY },
        ...(surface.notes === undefined ? {} : { notes: surface.notes }),
      };
    }),
    deliverySurfaces: v1.surfaces.map((surface) => {
      const migration = migrations.get(surface.key);
      if (!migration) {
        throw new Error(`Missing migration entry for v1 product feature "${surface.key}"`);
      }
      const category = categoriesById.get(surface.category);
      if (!category) {
        throw new Error(
          `V1 product feature "${surface.key}" references unknown group "${surface.category}"`
        );
      }
      return {
        key: migration.deliveryKey,
        featureKey: surface.key,
        locator: migration.locator,
        posture: {
          access: surface.access ?? category.defaultAccess,
          ...(surface.audiences === undefined ? {} : { audiences: surface.audiences }),
          invocation: surface.invocation,
          mustAlwaysBeOpen: surface.mustAlwaysBeOpen,
        },
        status: surface.status,
      };
    }),
    excludedDeliverySurfaces: [],
    deliverySurfaceMigrations: [],
  });
}

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
