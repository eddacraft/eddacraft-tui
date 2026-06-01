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

// Day-1 default scope set for device-code approval. Keep in sync with the
// /admin/approve handler's hardcoded choice; FLAGM-006 moves this into the
// flag manifest as a targeted default.
export const DEFAULT_APPROVAL_SCOPES: readonly ApiScopeName[] = ['beta'];

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
    // Day-1 parity with the pre-FLAGM-005 ALLOWED_SCOPES constant: every
    // listed scope is accepted. The spec's fail-closed entitlement contract
    // is satisfied via *override* semantics — operators disable a scope by
    // flipping the flag to 'disabled', which takes precedence over this
    // default. FLAGM-006 reviews whether to invert once the full evaluation
    // context (principal, plan) is plumbed through.
    defaultVariant: 'enabled',
    status: 'active',
    createdFor: 'FLAGM-005',
  } satisfies FeatureFlagDefinition;
}

export const API_SCOPE_FLAGS: Readonly<Record<ApiScopeName, FeatureFlagDefinition>> = {
  beta: makeScopeFlag('beta', 'Allow the beta scope on admin-issued access tokens'),
  preview: makeScopeFlag('preview', 'Allow the preview scope on admin-issued access tokens'),
  internal: makeScopeFlag('internal', 'Allow the internal scope on admin-issued access tokens'),
};

// Manifest-vs-tuple agreement is enforced at compile time by typing
// API_SCOPE_FLAGS as Record<ApiScopeName, …> and in the unit test suite.
// No runtime boot-time invariant is needed — a mismatch would fail the
// tsc build and the "keeps API_SCOPE_NAMES in sync with the manifest" test
// long before a Vercel edge worker touched this module.

export function isApiScopeName(value: string): value is ApiScopeName {
  return (API_SCOPE_NAMES as readonly string[]).includes(value);
}

export function apiScopeFlagFor(scope: string): FeatureFlagDefinition | undefined {
  if (!isApiScopeName(scope)) return undefined;
  return API_SCOPE_FLAGS[scope];
}

// Derive the evaluation environment from process.env at call time so
// non-prod deployments don't silently match production targeting rules
// once FLAGM-006 plumbs real targeting. Falls back to 'development' to keep
// the fail-safe direction if both vars are missing.
type FlagEnvironment = 'local' | 'development' | 'preview' | 'demo' | 'production';
const KNOWN_ENVIRONMENTS: readonly FlagEnvironment[] = [
  'local',
  'development',
  'preview',
  'demo',
  'production',
];

function currentEnvironment(): FlagEnvironment {
  const raw = process.env.VERCEL_ENV ?? process.env.NODE_ENV ?? 'development';
  // VERCEL_ENV returns 'production' / 'preview' / 'development'; NODE_ENV
  // returns 'production' / 'development' / 'test'. The manifest enum now uses
  // these native names directly (FLAGCAT-002 rename); 'test' aliases to
  // 'development' (a transient runtime state, not a deployment target).
  if (raw === 'test') return 'development';
  return (KNOWN_ENVIRONMENTS as readonly string[]).includes(raw)
    ? (raw as FlagEnvironment)
    : 'development';
}

// Default evaluation context for flag resolution in unauthenticated or
// targeting-less paths. Callers with a principal should build a richer
// context and pass it through `resolveApiScope`.
export function defaultApiEvaluationContext(): EvaluationContext {
  return {
    targetingKey: 'api-anonymous',
    environment: { environment: currentEnvironment() },
  };
}

export interface ApiScopeResolution {
  allowed: boolean;
  details: ResolutionDetails<boolean>;
}

export function resolveApiScope(
  scope: string,
  context: EvaluationContext = defaultApiEvaluationContext(),
  overrides?: FlagOverrides
): ApiScopeResolution | undefined {
  const flag = apiScopeFlagFor(scope);
  if (!flag) return undefined;
  const details = resolveFlag(flag, context, overrides) as ResolutionDetails<boolean>;
  return { allowed: details.variant === 'enabled', details };
}

export function isScopeAllowed(
  scope: string,
  context?: EvaluationContext,
  overrides?: FlagOverrides
): boolean {
  return resolveApiScope(scope, context, overrides)?.allowed ?? false;
}
