import type { EvaluationContext, FeatureFlagDefinition } from '@eddacraft/anvil-contracts';
import {
  resolveFlag,
  type FlagOverrides,
  type ResolutionDetails,
} from '@eddacraft/anvil-runtime/feature-flags';

// =============================================================================
// api.scope.* — per-scope entitlement flags (FLAGM-005)
// =============================================================================
//
// The single source of truth for valid API scope names. The flag manifest
// below is derived from this tuple so Zod's enum validator, the admin route
// resolver, and the flag manifest all agree by construction.
//
// Adding a scope is a three-step change: extend this tuple, add the matching
// FeatureFlagDefinition to API_SCOPE_FLAGS, and bump any telemetry filters.
// Removing one follows the usual FLAGS retirement flow.

export const API_SCOPE_FLAG_PREFIX = 'api.scope.' as const;

export const API_SCOPE_NAMES = ['beta', 'preview', 'internal'] as const;
export type ApiScopeName = (typeof API_SCOPE_NAMES)[number];

export const ALLOWED_API_SCOPES: readonly ApiScopeName[] = API_SCOPE_NAMES;

function makeScopeFlag(name: ApiScopeName, intent: string): FeatureFlagDefinition {
  return {
    key: `${API_SCOPE_FLAG_PREFIX}${name}`,
    owner: 'BAUTH',
    intent,
    class: 'entitlement',
    valueType: 'boolean',
    variants: [
      { key: 'enabled', value: true },
      { key: 'disabled', value: false },
    ],
    defaultVariant: 'enabled',
    status: 'active',
    createdFor: 'FLAGM-005',
  } satisfies FeatureFlagDefinition;
}

export const API_SCOPE_FLAGS: Readonly<Record<ApiScopeName, FeatureFlagDefinition>> = Object.freeze(
  {
    beta: makeScopeFlag('beta', 'Allow the beta scope on admin-issued access tokens'),
    preview: makeScopeFlag('preview', 'Allow the preview scope on admin-issued access tokens'),
    internal: makeScopeFlag('internal', 'Allow the internal scope on admin-issued access tokens'),
  }
);

// Module-load invariant: the manifest is derived from API_SCOPE_NAMES, so the
// key suffixes must match the tuple exactly. A mismatch is a coding error
// that would silently let Zod accept a scope with no backing flag; surface it
// at boot.
for (const name of API_SCOPE_NAMES) {
  const flag = API_SCOPE_FLAGS[name];
  const expectedKey = `${API_SCOPE_FLAG_PREFIX}${name}`;
  if (flag.key !== expectedKey) {
    throw new Error(
      `api.scope flag manifest mismatch: expected key "${expectedKey}" but flag exposes "${flag.key}"`
    );
  }
}

export function isApiScopeName(value: string): value is ApiScopeName {
  return (API_SCOPE_NAMES as readonly string[]).includes(value);
}

export function apiScopeFlagFor(scope: string): FeatureFlagDefinition | undefined {
  if (!isApiScopeName(scope)) return undefined;
  return API_SCOPE_FLAGS[scope];
}

// Default evaluation context for flag resolution in unauthenticated or
// targeting-less paths. Callers with a principal should build a richer
// context and pass it through `resolveApiScope`.
export function defaultApiEvaluationContext(): EvaluationContext {
  return {
    targetingKey: 'api-anonymous',
    environment: { environment: 'prod' },
  };
}

export interface ApiScopeResolution {
  allowed: boolean;
  details: ResolutionDetails;
}

export function resolveApiScope(
  scope: string,
  context: EvaluationContext = defaultApiEvaluationContext(),
  overrides?: FlagOverrides
): ApiScopeResolution | undefined {
  const flag = apiScopeFlagFor(scope);
  if (!flag) return undefined;
  const details = resolveFlag(flag, context, overrides);
  return { allowed: details.variant === 'enabled', details };
}

export function isScopeAllowed(
  scope: string,
  context?: EvaluationContext,
  overrides?: FlagOverrides
): boolean {
  return resolveApiScope(scope, context, overrides)?.allowed ?? false;
}
