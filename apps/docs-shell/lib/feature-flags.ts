import { DOCS_ACCESS_FLAG, canonicalAccountTier } from '@eddacraft/anvil-flags-catalogue';
import { resolveFlag } from '@eddacraft/anvil-runtime/feature-flags';

type FlagEnvironment = 'local' | 'development' | 'preview' | 'demo' | 'production';

const DOCS_SHELL_TARGETING_KEY = 'docs-shell';
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

  if (raw === 'test') return 'development';
  return (KNOWN_ENVIRONMENTS as readonly string[]).includes(raw)
    ? (raw as FlagEnvironment)
    : 'development';
}

export function evaluateDocsAccess(plan: string): boolean {
  const details = resolveFlag(DOCS_ACCESS_FLAG, {
    targetingKey: DOCS_SHELL_TARGETING_KEY,
    environment: { environment: currentEnvironment() },
    audience: { accountTier: canonicalAccountTier(plan) },
  });

  return details.variant === 'enabled' && details.value === true;
}
