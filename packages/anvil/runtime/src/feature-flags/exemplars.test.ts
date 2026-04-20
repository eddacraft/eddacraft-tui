import { describe, expect, it } from 'vitest';

import type {
  FeatureFlagDefinition,
  FeatureFlagManifest,
  EvaluationContext,
} from '@eddacraft/anvil-contracts';
import { FEATURE_FLAG_SCHEMA_VERSION, FeatureFlagManifestSchema } from '@eddacraft/anvil-contracts';

import { resolveFlag } from './resolver.js';
import { createSnapshot, loadSnapshot } from './snapshot.js';
import type { FlagOverrides } from './resolver.js';

// ---------------------------------------------------------------------------
// Exemplar: CLI licence-gated actions
// ---------------------------------------------------------------------------

const cliLicenceFlag: FeatureFlagDefinition = {
  key: 'cli.licence-gate',
  owner: 'BAUTH',
  intent: 'Gate CLI features behind licence validation',
  class: 'entitlement',
  valueType: 'boolean',
  variants: [
    { key: 'enabled', value: true },
    { key: 'disabled', value: false },
  ],
  defaultVariant: 'disabled',
  status: 'active',
  createdFor: 'FLAGS-008',
  description: 'Controls access to licence-gated CLI commands',
} as FeatureFlagDefinition;

// ---------------------------------------------------------------------------
// Exemplar: Docs access gating
// ---------------------------------------------------------------------------

const docsAccessFlag: FeatureFlagDefinition = {
  key: 'docs.access',
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
        { attribute: 'accountTier', operator: 'in_set', value: ['beta', 'pro', 'enterprise'] },
      ],
      variant: 'enabled',
    },
  ],
} as FeatureFlagDefinition;

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

const exemplarManifest: FeatureFlagManifest = {
  schemaVersion: FEATURE_FLAG_SCHEMA_VERSION,
  flags: [cliLicenceFlag, docsAccessFlag],
} as FeatureFlagManifest;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('exemplar manifest', () => {
  it('validates against the manifest schema', () => {
    const result = FeatureFlagManifestSchema.safeParse(exemplarManifest);
    expect(result.success).toBe(true);
  });

  it('round-trips through snapshot creation and loading', () => {
    const snapshot = createSnapshot(exemplarManifest);
    const json = JSON.stringify(snapshot);
    const loaded = loadSnapshot(json);
    expect(loaded.flags).toHaveLength(2);
    expect(loaded.flags[0].key).toBe('cli.licence-gate');
    expect(loaded.flags[1].key).toBe('docs.access');
  });
});

describe('CLI licence gate resolution', () => {
  const freeContext: EvaluationContext = {
    targetingKey: 'session-free',
    environment: { environment: 'prod' },
    audience: { licencePlan: 'free' },
  } as EvaluationContext;

  const proContext: EvaluationContext = {
    targetingKey: 'session-pro',
    environment: { environment: 'prod' },
    audience: { licencePlan: 'pro', accountTier: 'pro' },
  } as EvaluationContext;

  it('defaults to disabled (no targeting rules on the flag)', () => {
    const result = resolveFlag(cliLicenceFlag, freeContext);
    expect(result.variant).toBe('disabled');
    expect(result.value).toBe(false);
  });

  it('can be enabled via local override for operators', () => {
    const overrides: FlagOverrides = { local: { 'cli.licence-gate': 'enabled' } };
    const result = resolveFlag(cliLicenceFlag, freeContext, overrides);
    expect(result.variant).toBe('enabled');
    expect(result.reason).toBe('local_override');
  });

  it('can be disabled via emergency kill switch', () => {
    const overrides: FlagOverrides = {
      emergency: { 'cli.licence-gate': 'disabled' },
      local: { 'cli.licence-gate': 'enabled' },
    };
    const result = resolveFlag(cliLicenceFlag, proContext, overrides);
    expect(result.variant).toBe('disabled');
    expect(result.reason).toBe('emergency_override');
  });

  it('fails closed on invalid override (entitlement class)', () => {
    const overrides: FlagOverrides = { local: { 'cli.licence-gate': 'bogus' } };
    const result = resolveFlag(cliLicenceFlag, proContext, overrides);
    expect(result.reason).toBe('error');
    expect(result.errorCode).toBe('INVALID_OVERRIDE_VARIANT');
  });
});

describe('docs access resolution', () => {
  const unauthContext: EvaluationContext = {
    targetingKey: 'anon-visitor',
    environment: { environment: 'prod' },
  } as EvaluationContext;

  const betaContext: EvaluationContext = {
    targetingKey: 'beta-user-1',
    environment: { environment: 'prod' },
    audience: { accountTier: 'beta' },
  } as EvaluationContext;

  const proContext: EvaluationContext = {
    targetingKey: 'pro-user-1',
    environment: { environment: 'prod' },
    audience: { accountTier: 'pro' },
  } as EvaluationContext;

  const freeContext: EvaluationContext = {
    targetingKey: 'free-user-1',
    environment: { environment: 'prod' },
    audience: { accountTier: 'free' },
  } as EvaluationContext;

  it('denies access to unauthenticated visitors', () => {
    const result = resolveFlag(docsAccessFlag, unauthContext);
    expect(result.variant).toBe('disabled');
  });

  it('grants access to beta users', () => {
    const result = resolveFlag(docsAccessFlag, betaContext);
    expect(result.variant).toBe('enabled');
    expect(result.reason).toBe('targeting_match');
  });

  it('grants access to pro users', () => {
    const result = resolveFlag(docsAccessFlag, proContext);
    expect(result.variant).toBe('enabled');
  });

  it('denies access to free tier users', () => {
    const result = resolveFlag(docsAccessFlag, freeContext);
    expect(result.variant).toBe('disabled');
  });

  it('can be disabled via emergency override', () => {
    const overrides: FlagOverrides = { emergency: { 'docs.access': 'disabled' } };
    const result = resolveFlag(docsAccessFlag, betaContext, overrides);
    expect(result.variant).toBe('disabled');
    expect(result.reason).toBe('emergency_override');
  });
});

// -----------------------------------------------------------------------------
// FLAGM-004 parity — the shared resolver replaces the inline evaluator that
// lived in apps/docs-site/lib/feature-flags.ts before cutover. Two cases must
// agree (enabled/disabled) and one case is an intentional divergence: the
// pre-cutover inline evaluator granted access to missing-tier sessions as a
// backwards-compat carve-out; the resolver denies them because the flag is
// fail-closed. All sessions were reissued with a `tier` claim before the flip
// so no real user hits the divergence.
// -----------------------------------------------------------------------------

type LegacyVariant = 'enabled' | 'disabled';

function legacyInlineEval(accountTier: string | null): LegacyVariant {
  const ALLOWED = ['beta', 'pro', 'enterprise'];
  if (accountTier && ALLOWED.indexOf(accountTier) !== -1) return 'enabled';
  if (!accountTier) return 'enabled'; // pre-FLAGM-004 carve-out
  return 'disabled';
}

function sharedVariant(accountTier: string | null): LegacyVariant {
  const context: EvaluationContext = {
    targetingKey: 'parity-subject',
    environment: { environment: 'prod' },
    ...(accountTier !== null ? { audience: { accountTier } } : {}),
  } as EvaluationContext;
  return resolveFlag(docsAccessFlag, context).variant === 'enabled' ? 'enabled' : 'disabled';
}

describe('docs access parity (FLAGM-004)', () => {
  it('enabled: tier=pro agrees on both paths', () => {
    expect(legacyInlineEval('pro')).toBe('enabled');
    expect(sharedVariant('pro')).toBe('enabled');
  });

  it('disabled: tier=free agrees on both paths', () => {
    expect(legacyInlineEval('free')).toBe('disabled');
    expect(sharedVariant('free')).toBe('disabled');
  });

  it('default: missing tier diverges — legacy allowed, shared denies (fail-closed flip)', () => {
    expect(legacyInlineEval(null)).toBe('enabled');
    expect(sharedVariant(null)).toBe('disabled');
  });

  it('sweep over every known tier agrees on both paths', () => {
    for (const tier of ['beta', 'pro', 'enterprise', 'free', 'student', 'trial']) {
      expect(sharedVariant(tier)).toBe(legacyInlineEval(tier));
    }
  });
});
