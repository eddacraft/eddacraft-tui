/**
 * FLAGS-008 / FLAGM-004 / FLAGCAT-003: `/anvil` docs access via the shared
 * flag catalogue.
 *
 * The `docs.access` definition now lives in `flags/manifest.json` and is
 * imported from `@eddacraft/anvil-flags-catalogue` (FLAGCAT-003) — no flag
 * literal lives in this module anymore. What remains is the docs-site
 * evaluation glue: building the context (incl. tier reconciliation) and
 * environment detection. The consumer path stays edge-safe (no `fs`/`path`).
 *
 * Audience reconciliation (FLAGCAT-003): the catalogue's `docs.access`
 * targeting uses canonical `plan-*` audience ids, while the JWT `tier` claim
 * is a bare tier name (`beta`/`pro`/`enterprise`). `evaluateDocsAccess` maps
 * the bare tier to its canonical audience id before resolving, so the resolved
 * decision is byte-identical to the pre-catalogue (bare-targeting) behaviour.
 *
 * Fail-closed direction: a missing or unknown tier resolves to the flag's
 * `defaultVariant: 'disabled'`.
 */

import type { EvaluationContext } from '@eddacraft/anvil-contracts';
import { resolveFlag } from '@eddacraft/anvil-runtime/feature-flags';
import {
  DOCS_ACCESS_FLAG,
  DOCS_ACCESS_FLAG_KEY,
  canonicalAccountTier,
} from '@eddacraft/anvil-flags-catalogue';

// Re-export the catalogue definition so existing importers migrate with a
// path change, not a rename. The flag literal is gone from this module.
export { DOCS_ACCESS_FLAG, DOCS_ACCESS_FLAG_KEY };

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

export function evaluateDocsAccess(input: DocsEvaluationInput): DocsAccessResolution {
  const tier = input.accountTier ?? null;

  const context: EvaluationContext = {
    targetingKey: input.sessionSubject ?? 'anon-visitor',
    environment: { environment: currentEnvironment() },
    ...(tier !== null ? { audience: { accountTier: canonicalAccountTier(tier) } } : {}),
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
  // The manifest enum uses native NODE_ENV/VERCEL_ENV names (FLAGCAT-002
  // rename); 'test' aliases to 'development'.
  if (raw === 'test') return 'development';
  return (KNOWN_ENVIRONMENTS as readonly string[]).includes(raw)
    ? (raw as FlagEnvironment)
    : 'development';
}
