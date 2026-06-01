import {
  FeatureFlagManifestSchema,
  FlagAudienceManifestSchema,
  FlagEnvironmentManifestSchema,
  FlagGroupManifestSchema,
  type FeatureFlagManifest,
  type FlagAudienceManifest,
  type FlagEnvironmentManifest,
  type FlagGroupManifest,
} from '@eddacraft/anvil-contracts';

// Canonical sources of truth at the repo root (OpenFeature-adjacent layout).
// These are imported as JSON modules so the catalogue stays edge-bundle safe:
// no `fs`/`path`/`process` on the consumer path — bundlers inline the data.
import manifestJson from '../../../../flags/manifest.json' with { type: 'json' };
import groupsJson from '../../../../flags/groups.json' with { type: 'json' };
import audiencesJson from '../../../../flags/audiences.json' with { type: 'json' };
import environmentsJson from '../../../../flags/environments.json' with { type: 'json' };

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
