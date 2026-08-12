import { describe, expect, it } from 'vitest';
import { resolveFlag } from '@eddacraft/anvil-runtime/feature-flags';
import type { FeatureFlagDefinition } from '@eddacraft/anvil-contracts';
import {
  API_SCOPE_FLAGS,
  authenticatedApiEvaluationContext,
  defaultApiEvaluationContext,
  resolveApiScope,
} from '../lib/feature-flags.js';

describe('defaultApiEvaluationContext', () => {
  it('targets an anonymous key with no plan/account audience', () => {
    const context = defaultApiEvaluationContext();
    expect(context.targetingKey).toBe('api-anonymous');
    expect(context.audience).toBeUndefined();
  });
});

describe('authenticatedApiEvaluationContext (BACT-013)', () => {
  it('maps the account plan to its canonical plan-* audience id', () => {
    const context = authenticatedApiEvaluationContext({ targetingKey: 'user-1', plan: 'beta' });
    expect(context.targetingKey).toBe('user-1');
    expect(context.audience).toEqual({ accountTier: 'plan-beta' });
    expect(context.environment.environment).toBeDefined();
  });

  it('defaults to the `beta` plan (beta_users.plan column DEFAULT) when plan is omitted', () => {
    const context = authenticatedApiEvaluationContext({ targetingKey: 'user-2' });
    expect(context.audience).toEqual({ accountTier: 'plan-beta' });
  });

  it('defaults to `beta` when plan is explicitly null', () => {
    const context = authenticatedApiEvaluationContext({ targetingKey: 'user-3', plan: null });
    expect(context.audience).toEqual({ accountTier: 'plan-beta' });
  });

  it('passes an unrecognised plan value through unchanged (fail-closed: no invented plan-* id)', () => {
    const context = authenticatedApiEvaluationContext({ targetingKey: 'user-4', plan: 'zzz' });
    expect(context.audience).toEqual({ accountTier: 'zzz' });
  });
});

describe('resolveApiScope (BACT-013 — continues to respect api.scope.* behaviour)', () => {
  it('still resolves via the anonymous default context when none is passed (regression pin)', () => {
    const resolution = resolveApiScope('beta');
    expect(resolution?.allowed).toBe(true);
    expect(resolution?.details.reason).toBe('default');
  });

  it('resolves identically when an authenticated plan-axis context is passed — today’s api.scope.* flags carry no plan targeting', () => {
    const context = authenticatedApiEvaluationContext({ targetingKey: 'user-1', plan: 'beta' });
    const resolution = resolveApiScope('beta', context);
    expect(resolution?.allowed).toBe(true);
  });
});

describe('plan-axis audience governs targeting (BACT-013 wiring proof)', () => {
  // flags/manifest.json is read-only catalogue (no api.scope.* flag carries
  // plan targeting yet), so this proves the context/audience plumbing itself
  // — not a specific catalogue flag — using a local fixture shaped like the
  // real `docs.access` flag (flags/manifest.json), which is the one shipped
  // flag that does target `accountTier`.
  const planGatedFlag: FeatureFlagDefinition = {
    key: 'test.plan-gated',
    owner: 'BACT',
    intent: 'test fixture',
    class: 'entitlement',
    valueType: 'boolean',
    variants: [
      { key: 'enabled', value: true },
      { key: 'disabled', value: false },
    ],
    defaultVariant: 'disabled',
    status: 'active',
    targeting: [
      {
        conditions: [
          { attribute: 'accountTier', operator: 'in_set', value: ['plan-pro', 'plan-enterprise'] },
        ],
        variant: 'enabled',
      },
    ],
  };

  it('denies a beta-plan account (not in the targeted set)', () => {
    const context = authenticatedApiEvaluationContext({ targetingKey: 'user-1', plan: 'beta' });
    const details = resolveFlag(planGatedFlag, context);
    expect(details.variant).toBe('disabled');
    expect(details.reason).toBe('default');
  });

  it('allows a pro-plan account (targeted)', () => {
    const context = authenticatedApiEvaluationContext({ targetingKey: 'user-1', plan: 'pro' });
    const details = resolveFlag(planGatedFlag, context);
    expect(details.variant).toBe('enabled');
    expect(details.reason).toBe('targeting_match');
  });
});

describe('API_SCOPE_FLAGS re-export sanity', () => {
  it('still exposes the beta scope flag (unaffected by BACT-013)', () => {
    expect(API_SCOPE_FLAGS.beta.key).toBe('api.scope.beta');
  });
});
