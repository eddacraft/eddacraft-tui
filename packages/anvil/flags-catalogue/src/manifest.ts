import {
  FeatureFlagManifestSchema,
  FlagAudienceManifestSchema,
  FlagEnvironmentManifestSchema,
  FlagGroupManifestSchema,
  FlagSurfaceManifestSchema,
  type FeatureFlagManifest,
  type FlagAudienceManifest,
  type FlagEnvironmentManifest,
  type FlagGroupManifest,
  type FlagSurfaceManifest,
} from '@eddacraft/anvil-contracts';

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
const SURFACES: FlagSurfaceManifest = parseOrThrow(
  'flags/surfaces.json',
  FlagSurfaceManifestSchema,
  surfacesJson
);

// Cross-inventory integrity, enforced fail-loud at module load. The base
// FeatureFlagDefinitionSchema keeps `primaryGroup` optional so un-migrated
// per-surface literals still type-check (FLAGCAT-003/-005); the *manifest*,
// however, requires every flag to carry a `primaryGroup` that resolves to a
// real group. (FLAGCAT-006 adds the full TS<->Rust<->JSON drift check in CI;
// this is the load-time guarantee the catalogue itself depends on.)
function assertCrossInventoryIntegrity(): void {
  const groupIds = new Set(GROUPS.groups.map((g) => g.id));
  const audienceIds = new Set(AUDIENCES.audiences.map((a) => a.id));

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

  // ADR-076: every gating audience a surface names must exist in the audience
  // inventory. (Structural surface checks — keys/categories/requires/acyclicity
  // /mustAlwaysBeOpen — are enforced by FlagSurfaceManifestSchema itself.)
  for (const surface of SURFACES.surfaces) {
    for (const audience of surface.audiences ?? []) {
      if (!audienceIds.has(audience)) {
        throw new Error(
          `[anvil-flags-catalogue] surface "${surface.key}" references unknown audience "${audience}"`
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

/**
 * The validated surface registry (ADR-076). Declared inventory + static checks
 * only; runtime cascade-off and auth-list derivation are deferred.
 */
export function flagSurfaces(): FlagSurfaceManifest {
  return SURFACES;
}

/**
 * Recovery-critical surfaces that a registry edit can never gate — the
 * `MUST_ALWAYS_BE_OPEN` floor (ADR-076 §6). Derived from the registry so the
 * floor and the data cannot drift.
 */
export function mustAlwaysBeOpenSurfaces(): readonly string[] {
  return SURFACES.surfaces.filter((s) => s.mustAlwaysBeOpen).map((s) => s.key);
}
