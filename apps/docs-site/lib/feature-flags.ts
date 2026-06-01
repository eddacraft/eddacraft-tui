/**
 * FLAGS-008 / FLAGM-004: `/anvil` docs access via the shared flag model.
 *
 * Post-FLAGM-004: the middleware evaluates `docs.access` by calling
 * `resolveFlag` from `@eddacraft/anvil-runtime/feature-flags` directly.
 * The pre-FLAGM-004 inline evaluator is gone — the `/feature-flags`
 * subpath has no Node-only imports and bundles cleanly for Vercel edge.
 *
 * Fail-closed direction: missing `tier` claim now resolves to the flag's
 * `defaultVariant: 'disabled'`. The pre-FLAGM-004 backwards-compat
 * carve-out that treated missing-tier as `enabled` is deleted because
 * all sessions were reissued with a `tier` claim before the cutover.
 */

import type { EvaluationContext, FeatureFlagDefinition } from '@eddacraft/anvil-contracts';
import { resolveFlag } from '@eddacraft/anvil-runtime/feature-flags';

export const DOCS_ACCESS_FLAG_KEY = 'docs.access';

export type DocsAccessVariant = 'enabled' | 'disabled';

export interface DocsAccessResolution {
  flagKey: typeof DOCS_ACCESS_FLAG_KEY;
  variant: DocsAccessVariant;
  reason: 'targeting_match' | 'default';
  accountTier: string | null;
}

export interface DocsEvaluationInput {
  accountTier?: string | null;
  sessionSubject?: string | null;
}

// Mirrors `packages/anvil/runtime/src/feature-flags/exemplars.test.ts`.
// Kept here rather than imported from a shared package because the docs
// site is the only caller; promotion to a shared flag catalogue is
// tracked under FLAGM-006.
export const DOCS_ACCESS_FLAG: FeatureFlagDefinition = {
  key: DOCS_ACCESS_FLAG_KEY,
  owner: 'DOCSAUTH',
  intent: 'Gate /anvil docs access for authenticated beta users',
  class: 'entitlement',
  valueType: 'boolean',
  variants: [
    { key: 'enabled', value: true },
    { key: 'disabled', value: false },
  ],
  defaultVariant: 'disabled',
  status: 'active',
  createdFor: 'FLAGS-008',
  targeting: [
    {
      conditions: [
        {
          attribute: 'accountTier',
          operator: 'in_set',
          value: ['beta', 'pro', 'enterprise'],
        },
      ],
      variant: 'enabled',
    },
  ],
} as FeatureFlagDefinition;

export function evaluateDocsAccess(input: DocsEvaluationInput): DocsAccessResolution {
  const tier = input.accountTier ?? null;

  const context: EvaluationContext = {
    targetingKey: input.sessionSubject ?? 'anon-visitor',
    environment: { environment: currentEnvironment() },
    ...(tier !== null ? { audience: { accountTier: tier } } : {}),
  };

  const details = resolveFlag(DOCS_ACCESS_FLAG, context);
  const variant: DocsAccessVariant = details.variant === 'enabled' ? 'enabled' : 'disabled';
  const reason: 'targeting_match' | 'default' =
    details.reason === 'targeting_match' ? 'targeting_match' : 'default';

  return {
    flagKey: DOCS_ACCESS_FLAG_KEY,
    variant,
    reason,
    accountTier: tier,
  };
}

type FlagEnvironment = 'local' | 'development' | 'preview' | 'demo' | 'production';
const KNOWN_ENVIRONMENTS: readonly FlagEnvironment[] = [
  'local',
  'development',
  'preview',
  'demo',
  'production',
];

function currentEnvironment(): FlagEnvironment {
  const raw =
    (typeof process !== 'undefined' && process.env
      ? (process.env.VERCEL_ENV ?? process.env.NODE_ENV)
      : undefined) ?? 'development';
  // The manifest enum now uses native NODE_ENV/VERCEL_ENV names directly
  // (FLAGCAT-002 rename); 'test' aliases to 'development'.
  if (raw === 'test') return 'development';
  return (KNOWN_ENVIRONMENTS as readonly string[]).includes(raw)
    ? (raw as FlagEnvironment)
    : 'development';
}
