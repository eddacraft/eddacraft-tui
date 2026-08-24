import {
  FeatureFlagManifestSchema,
  FlagAudienceManifestSchema,
  FlagEnvironmentManifestSchema,
  FlagGroupManifestSchema,
  ProductCatalogueManifestSchema,
  ProductCatalogueV1Schema,
  normaliseProductCatalogueV1,
  type FeatureFlagManifest,
  type FlagAudienceManifest,
  type FlagEnvironmentManifest,
  type FlagGroupManifest,
  type ProductCatalogueManifest,
  type ProductCatalogueV1,
} from '@eddacraft/anvil-contracts';
import { productCatalogueV1Migration } from './compatibility/product-catalogue-v1-migration.js';
import productCatalogueV1Json from './compatibility/product-catalogue-v1.json' with { type: 'json' };

export type DeepReadonly<T> =
  T extends ReadonlyArray<infer Item>
    ? ReadonlyArray<DeepReadonly<Item>>
    : T extends object
      ? { readonly [Key in keyof T]: DeepReadonly<T[Key]> }
      : T;

export type ReadonlyProductCatalogue = DeepReadonly<ProductCatalogueManifest>;
export type ReadonlyProductCatalogueV1 = DeepReadonly<ProductCatalogueV1>;

// Canonical sources of truth at the repo root (OpenFeature-adjacent layout).
// These are imported as JSON modules so the catalogue stays edge-bundle safe:
// no `fs`/`path`/`process` on the consumer path — bundlers inline the data.
import manifestJson from '../../../../flags/manifest.json' with { type: 'json' };
import groupsJson from '../../../../flags/groups.json' with { type: 'json' };
import audiencesJson from '../../../../flags/audiences.json' with { type: 'json' };
import environmentsJson from '../../../../flags/environments.json' with { type: 'json' };
import surfacesJson from '../../../../flags/surfaces.json' with { type: 'json' };

/**
 * Parse-or-throw at module load. A malformed manifest fails loudly here, at
 * import time, rather than at the first resolver call.
 */
function parseOrThrow<T>(
  label: string,
  schema: { safeParse: (data: unknown) => { success: boolean; data?: T; error?: unknown } },
  data: unknown
): T {
  const result = schema.safeParse(data);
  if (!result.success) {
    throw new Error(
      `[anvil-flags-catalogue] ${label} failed schema validation: ${JSON.stringify(result.error)}`
    );
  }
  return result.data as T;
}

const MANIFEST: FeatureFlagManifest = parseOrThrow(
  'flags/manifest.json',
  FeatureFlagManifestSchema,
  manifestJson
);
const GROUPS: FlagGroupManifest = parseOrThrow(
  'flags/groups.json',
  FlagGroupManifestSchema,
  groupsJson
);
const AUDIENCES: FlagAudienceManifest = parseOrThrow(
  'flags/audiences.json',
  FlagAudienceManifestSchema,
  audiencesJson
);
const ENVIRONMENTS: FlagEnvironmentManifest = parseOrThrow(
  'flags/environments.json',
  FlagEnvironmentManifestSchema,
  environmentsJson
);
/**
 * Internal structural normalisation for supported catalogue documents.
 *
 * This validates document-local v1/v2 shape and references only. It does not
 * apply repository-backed owner or audience checks and must not be exposed as
 * an authoritative package-root loader.
 */
export function normaliseProductCatalogueDocument(input: unknown): ReadonlyProductCatalogue {
  const schemaVersion =
    input !== null && typeof input === 'object' && 'schemaVersion' in input
      ? (input as { schemaVersion?: unknown }).schemaVersion
      : undefined;

  if (schemaVersion === 1) {
    return deepFreeze(
      normaliseProductCatalogueV1(
        ProductCatalogueV1Schema.parse(input),
        productCatalogueV1Migration
      )
    );
  }
  if (schemaVersion === 2) {
    return deepFreeze(ProductCatalogueManifestSchema.parse(input));
  }
  throw new Error(
    `[anvil-flags-catalogue] unsupported product catalogue schemaVersion: ${String(schemaVersion)}`
  );
}

const PRODUCT_CATALOGUE = normaliseProductCatalogueDocument(surfacesJson);

// Cross-inventory integrity, enforced fail-loud at module load. The base
// FeatureFlagDefinitionSchema keeps `primaryGroup` optional so un-migrated
// per-surface literals still type-check (FLAGCAT-003/-005); the *manifest*,
// however, requires every flag to carry a `primaryGroup` that resolves to a
// real group. (FLAGCAT-006 adds the full TS<->Rust<->JSON drift check in CI;
// this is the load-time guarantee the catalogue itself depends on.)
function assertCrossInventoryIntegrity(): void {
  const groupIds = new Set(GROUPS.groups.map((g) => g.id));
  const audienceIds = new Set(AUDIENCES.audiences.map((a) => a.id));

  const featureByKey = new Map(
    PRODUCT_CATALOGUE.productFeatures.map((feature) => [feature.key, feature])
  );
  const flagsByFeature = new Map<string, string[]>();

  for (const flag of MANIFEST.flags) {
    if (flag.primaryGroup === undefined) {
      throw new Error(
        `[anvil-flags-catalogue] flag "${flag.key}" is missing required primaryGroup`
      );
    }
    if (!groupIds.has(flag.primaryGroup)) {
      throw new Error(
        `[anvil-flags-catalogue] flag "${flag.key}" references unknown primaryGroup "${flag.primaryGroup}"`
      );
    }
    if (flag.controlsProductFeatures === undefined) {
      throw new Error(
        `[anvil-flags-catalogue] flag "${flag.key}" is missing required controlsProductFeatures`
      );
    }
    for (const featureKey of flag.controlsProductFeatures) {
      if (!featureByKey.has(featureKey)) {
        throw new Error(
          `[anvil-flags-catalogue] flag "${flag.key}" controls unknown product feature "${featureKey}"`
        );
      }
      const owners = flagsByFeature.get(featureKey) ?? [];
      owners.push(flag.key);
      flagsByFeature.set(featureKey, owners);
    }
  }

  for (const group of GROUPS.groups) {
    for (const audience of group.defaultAudiences) {
      if (!audienceIds.has(audience)) {
        throw new Error(
          `[anvil-flags-catalogue] group "${group.id}" references unknown defaultAudience "${audience}"`
        );
      }
    }
  }

  for (const feature of PRODUCT_CATALOGUE.productFeatures) {
    const linkedFlagKeys = flagsByFeature.get(feature.key) ?? [];
    if (feature.flagLinkage.disposition === 'linked') {
      const declared = [...feature.flagLinkage.flagKeys].sort();
      const actual = [...linkedFlagKeys].sort();
      if (declared.join('\0') !== actual.join('\0')) {
        throw new Error(
          `[anvil-flags-catalogue] product feature "${feature.key}" linked flags [${declared.join(', ')}] must match operational flags that control it [${actual.join(', ')}]`
        );
      }
    } else if (linkedFlagKeys.length > 0) {
      throw new Error(
        `[anvil-flags-catalogue] product feature "${feature.key}" is unflagged but controlled by ${linkedFlagKeys.join(', ')}`
      );
    }
  }

  // ADR-076: every gating audience a delivery surface names must exist in the audience
  // inventory. (Structural surface checks — keys/categories/requires/acyclicity
  // /mustAlwaysBeOpen — are enforced by ProductCatalogueManifestSchema itself.)
  for (const surface of PRODUCT_CATALOGUE.deliverySurfaces) {
    for (const audience of surface.posture.audiences ?? []) {
      if (!audienceIds.has(audience)) {
        throw new Error(
          `[anvil-flags-catalogue] delivery surface "${surface.key}" references unknown audience "${audience}"`
        );
      }
    }
  }
}

assertCrossInventoryIntegrity();

/** The validated feature-flag manifest. Validated once at module load. */
export function featureFlagManifest(): FeatureFlagManifest {
  return MANIFEST;
}

/** The validated primary-group inventory. */
export function flagGroups(): FlagGroupManifest {
  return GROUPS;
}

/** The validated audience inventory. */
export function flagAudiences(): FlagAudienceManifest {
  return AUDIENCES;
}

/** The validated environment inventory. */
export function flagEnvironments(): FlagEnvironmentManifest {
  return ENVIRONMENTS;
}

/** The authoritative validated product catalogue v2. */
export function productCatalogue(): ReadonlyProductCatalogue {
  return PRODUCT_CATALOGUE;
}

function deepFreeze<T>(value: T): DeepReadonly<T> {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const nested of Object.values(value as Record<string, unknown>)) {
      deepFreeze(nested);
    }
    Object.freeze(value);
  }
  return value as DeepReadonly<T>;
}

const LEGACY_SURFACES = deepFreeze(ProductCatalogueV1Schema.parse(productCatalogueV1Json));
const MUST_ALWAYS_BE_OPEN_FEATURE_KEYS = deepFreeze(
  LEGACY_SURFACES.surfaces
    .filter((surface) => surface.mustAlwaysBeOpen)
    .map((surface) => surface.key)
);
const MUST_ALWAYS_BE_OPEN_DELIVERY_KEYS = deepFreeze(
  PRODUCT_CATALOGUE.deliverySurfaces
    .filter((surface) => surface.posture.mustAlwaysBeOpen)
    .map((surface) => surface.key)
);

/**
 * @deprecated Compatibility-only v1 projection of the frozen 46-feature CLI
 * subset. It is incomplete and must not drive completeness or enforcement.
 */
export function flagSurfaces(): ReadonlyProductCatalogueV1 {
  return LEGACY_SURFACES;
}

/** @deprecated Recovery-critical legacy v1 feature keys. */
export function mustAlwaysBeOpenSurfaces(): readonly string[] {
  return MUST_ALWAYS_BE_OPEN_FEATURE_KEYS;
}

/**
 * Recovery-critical v2 delivery identities that a registry edit can never
 * gate — the `MUST_ALWAYS_BE_OPEN` floor (ADR-076 §6).
 */
export function mustAlwaysBeOpenDeliverySurfaces(): readonly string[] {
  return MUST_ALWAYS_BE_OPEN_DELIVERY_KEYS;
}
