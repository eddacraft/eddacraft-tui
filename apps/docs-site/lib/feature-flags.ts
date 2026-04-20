/**
 * FLAGS-008 exemplar: `/anvil` docs access via the shared flag model.
 *
 * Inlines the `docs.access` flag and runs a minimal evaluation compatible
 * with `resolveFlag` from @eddacraft/anvil-runtime. Kept inline because
 * Vercel edge functions cannot pull the workspace runtime package; once a
 * docs-side snapshot distribution exists this helper will delegate to the
 * shared resolver.
 *
 * This evaluator follows the shared model exercised in
 * `packages/anvil/runtime/src/feature-flags/exemplars.test.ts` with one
 * intentional compatibility difference: the exemplar defaults
 * `docs.access` to `disabled` (Entitlement class fails closed), while this
 * inline evaluator keeps the missing-tier case as `enabled` so sessions
 * whose JWTs were minted before the `tier` claim landed are not regressed.
 * The full cutover to a fail-closed default is scoped to FLAGM-004 and
 * will land alongside the resolver-backed replacement of this stub.
 *
 * Behaviour:
 *   - `accountTier` ∈ {beta, pro, enterprise}      → enabled (targeting match)
 *   - any other tier value                         → disabled (default)
 *   - missing claim (backwards compat with old tokens) → enabled (default)
 */

export const DOCS_ACCESS_FLAG_KEY = 'docs.access';

export type DocsAccessVariant = 'enabled' | 'disabled';

export interface DocsAccessResolution {
  flagKey: typeof DOCS_ACCESS_FLAG_KEY;
  variant: DocsAccessVariant;
  reason: 'targeting_match' | 'default';
  accountTier: string | null;
}

const ALLOWED_TIERS: ReadonlyArray<string> = ['beta', 'pro', 'enterprise'];

export interface DocsEvaluationInput {
  accountTier?: string | null;
}

export function evaluateDocsAccess(input: DocsEvaluationInput): DocsAccessResolution {
  const tier = input.accountTier ?? null;

  if (tier && ALLOWED_TIERS.indexOf(tier) !== -1) {
    return {
      flagKey: DOCS_ACCESS_FLAG_KEY,
      variant: 'enabled',
      reason: 'targeting_match',
      accountTier: tier,
    };
  }

  // Backwards compatibility: tokens minted before the tier claim landed may
  // omit the field entirely. Those sessions fall through to the default
  // variant so the flag introduction doesn't regress existing users.
  if (!tier) {
    return {
      flagKey: DOCS_ACCESS_FLAG_KEY,
      variant: 'enabled',
      reason: 'default',
      accountTier: null,
    };
  }

  return {
    flagKey: DOCS_ACCESS_FLAG_KEY,
    variant: 'disabled',
    reason: 'default',
    accountTier: tier,
  };
}
