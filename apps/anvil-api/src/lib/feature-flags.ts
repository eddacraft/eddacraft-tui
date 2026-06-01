import type { EvaluationContext, FeatureFlagDefinition } from '@eddacraft/anvil-contracts';
import {
  resolveFlag,
  type FlagOverrides,
  type ResolutionDetails,
} from '@eddacraft/anvil-runtime/feature-flags';
import {
  API_SCOPE_FLAGS,
  API_SCOPE_FLAG_PREFIX,
  API_SCOPE_NAMES,
  DEFAULT_APPROVAL_SCOPES,
  isApiScopeName,
  type ApiScopeName,
} from '@eddacraft/anvil-flags-catalogue';

// =============================================================================
// api.scope.* — per-scope entitlement flags
// =============================================================================
//
// FLAGCAT-003: the api.scope.* definitions (and the scope-name tuple, prefix,
// default-approval set) now live in `flags/manifest.json` and are sourced from
// `@eddacraft/anvil-flags-catalogue` — no flag literal lives in this module.
// What remains is the api-side evaluation glue. Re-exported so existing
// importers migrate with a path change, not a rename.

export {
  API_SCOPE_FLAGS,
  API_SCOPE_FLAG_PREFIX,
  API_SCOPE_NAMES,
  DEFAULT_APPROVAL_SCOPES,
  isApiScopeName,
  type ApiScopeName,
};

export function apiScopeFlagFor(scope: string): FeatureFlagDefinition | undefined {
  if (!isApiScopeName(scope)) return undefined;
  return API_SCOPE_FLAGS[scope];
}

// Derive the evaluation environment from process.env at call time so
// non-prod deployments don't silently match production targeting rules.
// Falls back to 'development' to keep the fail-safe direction if both vars
// are missing.
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
  // returns 'production' / 'development' / 'test'. The manifest enum uses
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
