import type { FeatureFlagDefinition } from '@eddacraft/anvil-contracts';
import { featureFlagManifest } from './manifest.js';

// Flag key constants — preserved as named exports so existing `*_KEY` imports
// migrate with a path change, not a rename (FLAGCAT-003/-005).
export const CLI_LICENCE_GATE_KEY = 'cli.licence-gate';
export const DOCS_ACCESS_FLAG_KEY = 'docs.access';
export const API_SCOPE_FLAG_PREFIX = 'api.scope.' as const;

export const API_SCOPE_NAMES = ['beta', 'preview', 'internal'] as const;
export type ApiScopeName = (typeof API_SCOPE_NAMES)[number];

/** Scopes granted by default on admin-issued access tokens. */
export const DEFAULT_APPROVAL_SCOPES: readonly ApiScopeName[] = ['beta'];

const FLAGS_BY_KEY: ReadonlyMap<string, FeatureFlagDefinition> = new Map(
  featureFlagManifest().flags.map((flag) => [flag.key, flag])
);

/** Look up a flag definition by key, throwing if it is not in the manifest. */
export function flagByKey(key: string): FeatureFlagDefinition {
  const flag = FLAGS_BY_KEY.get(key);
  if (flag === undefined) {
    throw new Error(`[anvil-flags-catalogue] no flag with key "${key}" in the manifest`);
  }
  return flag;
}

/** Look up a flag definition by key, returning undefined if absent. */
export function tryFlagByKey(key: string): FeatureFlagDefinition | undefined {
  return FLAGS_BY_KEY.get(key);
}

function apiScopeKey(name: ApiScopeName): string {
  return `${API_SCOPE_FLAG_PREFIX}${name}`;
}

// Typed accessors for every shipped flag. Shapes are byte-compatible with the
// per-surface modules these will replace (FLAGCAT-003/-005).
export const CLI_LICENCE_GATE: FeatureFlagDefinition = flagByKey(CLI_LICENCE_GATE_KEY);
export const DOCS_ACCESS_FLAG: FeatureFlagDefinition = flagByKey(DOCS_ACCESS_FLAG_KEY);

export const API_SCOPE_FLAGS: Readonly<Record<ApiScopeName, FeatureFlagDefinition>> = Object.freeze(
  API_SCOPE_NAMES.reduce(
    (acc, name) => {
      acc[name] = flagByKey(apiScopeKey(name));
      return acc;
    },
    {} as Record<ApiScopeName, FeatureFlagDefinition>
  )
);

/** True when the given string is a known API scope name. */
export function isApiScopeName(value: string): value is ApiScopeName {
  return (API_SCOPE_NAMES as readonly string[]).includes(value);
}
