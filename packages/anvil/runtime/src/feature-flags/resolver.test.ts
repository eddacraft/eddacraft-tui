import { describe, expect, it } from 'vitest';

import type { EvaluationContext, FeatureFlagDefinition } from '@eddacraft/anvil-contracts';

import { resolveFlag, evaluatePercentage } from './resolver.js';
import type { FlagOverrides } from './resolver.js';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

function booleanFlag(overrides: Partial<FeatureFlagDefinition> = {}): FeatureFlagDefinition {
  return {
    key: 'test.flag',
    owner: 'TEST',
    intent: 'Test flag',
    class: 'entitlement',
    valueType: 'boolean',
    variants: [
      { key: 'enabled', value: true },
      { key: 'disabled', value: false },
    ],
    defaultVariant: 'disabled',
    status: 'active',
    createdFor: 'FLAGS-003',
    ...overrides,
  } as FeatureFlagDefinition;
}

function devContext(overrides: Partial<EvaluationContext> = {}): EvaluationContext {
  return {
    targetingKey: 'session-abc',
    environment: { environment: 'development' },
    ...overrides,
  } as EvaluationContext;
}

function prodContext(overrides: Partial<EvaluationContext> = {}): EvaluationContext {
  return {
    targetingKey: 'session-abc',
    environment: { environment: 'production', channel: 'production' },
    audience: { accountTier: 'pro' },
    ...overrides,
  } as EvaluationContext;
}

// ---------------------------------------------------------------------------
// Resolution Precedence
// ---------------------------------------------------------------------------

describe('resolveFlag', () => {
  describe('precedence order', () => {
    const flag = booleanFlag({
      targeting: [
        {
          conditions: [{ attribute: 'environment', operator: 'equals', value: 'production' }],
          variant: 'enabled',
        },
      ],
    });

    it('resolves default when no overrides or targeting match', () => {
      const result = resolveFlag(flag, devContext());
      expect(result.variant).toBe('disabled');
      expect(result.reason).toBe('default');
      expect(result.value).toBe(false);
    });

    it('targeting match overrides default', () => {
      const result = resolveFlag(flag, prodContext());
      expect(result.variant).toBe('enabled');
      expect(result.reason).toBe('targeting_match');
    });

    it('local override overrides targeting', () => {
      const overrides: FlagOverrides = { local: { 'test.flag': 'disabled' } };
      const result = resolveFlag(flag, prodContext(), overrides);
      expect(result.variant).toBe('disabled');
      expect(result.reason).toBe('local_override');
    });

    it('emergency override overrides everything', () => {
      const overrides: FlagOverrides = {
        emergency: { 'test.flag': 'disabled' },
        local: { 'test.flag': 'enabled' },
      };
      const result = resolveFlag(flag, prodContext(), overrides);
      expect(result.variant).toBe('disabled');
      expect(result.reason).toBe('emergency_override');
    });
  });

  // -------------------------------------------------------------------------
  // Default resolution
  // -------------------------------------------------------------------------

  describe('default resolution', () => {
    it('resolves to defaultVariant', () => {
      const result = resolveFlag(booleanFlag(), devContext());
      expect(result.variant).toBe('disabled');
      expect(result.value).toBe(false);
      expect(result.flagKey).toBe('test.flag');
    });

    it('returns error when defaultVariant is missing', () => {
      const flag = booleanFlag({ defaultVariant: 'nonexistent' });
      const result = resolveFlag(flag, devContext());
      expect(result.reason).toBe('error');
      expect(result.errorCode).toBe('MISSING_DEFAULT_VARIANT');
    });
  });

  // -------------------------------------------------------------------------
  // Status handling
  // -------------------------------------------------------------------------

  describe('status handling', () => {
    it('retired flags resolve to default regardless of targeting', () => {
      const flag = booleanFlag({
        status: 'retired',
        targeting: [
          {
            conditions: [{ attribute: 'environment', operator: 'equals', value: 'production' }],
            variant: 'enabled',
          },
        ],
      });
      const result = resolveFlag(flag, prodContext());
      expect(result.variant).toBe('disabled');
      expect(result.reason).toBe('disabled');
    });

    it('draft flags resolve to default', () => {
      const flag = booleanFlag({ status: 'draft' });
      const result = resolveFlag(flag, devContext());
      expect(result.reason).toBe('disabled');
    });

    it('active flags evaluate normally', () => {
      const result = resolveFlag(booleanFlag({ status: 'active' }), devContext());
      expect(result.reason).toBe('default');
    });

    it('retiring flags still evaluate targeting', () => {
      const flag = booleanFlag({
        status: 'retiring',
        targeting: [
          {
            conditions: [{ attribute: 'environment', operator: 'equals', value: 'production' }],
            variant: 'enabled',
          },
        ],
      });
      const result = resolveFlag(flag, prodContext());
      expect(result.reason).toBe('targeting_match');
    });
  });

  // -------------------------------------------------------------------------
  // Targeting conditions
  // -------------------------------------------------------------------------

  describe('targeting conditions', () => {
    it('equals operator matches', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'environment', operator: 'equals', value: 'development' }],
            variant: 'enabled',
          },
        ],
      });
      expect(resolveFlag(flag, devContext()).variant).toBe('enabled');
    });

    it('not_equals operator matches', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'environment', operator: 'not_equals', value: 'production' }],
            variant: 'enabled',
          },
        ],
      });
      expect(resolveFlag(flag, devContext()).variant).toBe('enabled');
    });

    it('in_set operator matches', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [
              { attribute: 'accountTier', operator: 'in_set', value: ['pro', 'enterprise'] },
            ],
            variant: 'enabled',
          },
        ],
      });
      expect(resolveFlag(flag, prodContext()).variant).toBe('enabled');
    });

    it('not_in_set operator matches', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'accountTier', operator: 'not_in_set', value: ['free'] }],
            variant: 'enabled',
          },
        ],
      });
      expect(resolveFlag(flag, prodContext()).variant).toBe('enabled');
    });

    it('multiple conditions use AND semantics', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [
              { attribute: 'environment', operator: 'equals', value: 'production' },
              { attribute: 'accountTier', operator: 'equals', value: 'pro' },
            ],
            variant: 'enabled',
          },
        ],
      });
      // Both match
      expect(resolveFlag(flag, prodContext()).variant).toBe('enabled');
      // Environment doesn't match
      expect(resolveFlag(flag, devContext()).variant).toBe('disabled');
    });

    it('first matching rule wins', () => {
      const flag = booleanFlag({
        variants: [
          { key: 'enabled', value: true },
          { key: 'disabled', value: false },
          { key: 'limited', value: 'limited' },
        ],
        targeting: [
          {
            conditions: [{ attribute: 'environment', operator: 'equals', value: 'production' }],
            variant: 'limited',
          },
          {
            conditions: [{ attribute: 'accountTier', operator: 'equals', value: 'pro' }],
            variant: 'enabled',
          },
        ],
      });
      // First rule matches prod environment
      expect(resolveFlag(flag, prodContext()).variant).toBe('limited');
    });

    it('audience attributes resolve correctly', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'userRole', operator: 'equals', value: 'admin' }],
            variant: 'enabled',
          },
        ],
      });
      const ctx = prodContext({
        audience: { userRole: 'admin' },
      } as Partial<EvaluationContext>);
      expect(resolveFlag(flag, ctx).variant).toBe('enabled');
    });

    it('segment operator resolves to default variant with unimplemented_operator reason', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'cohort', operator: 'segment', value: 'beta-testers' }],
            variant: 'enabled',
          },
        ],
      });
      const result = resolveFlag(flag, prodContext());
      expect(result.variant).toBe('disabled');
      expect(result.reason).toBe('unimplemented_operator');
    });

    it('missing audience attribute does not match', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'cohort', operator: 'equals', value: 'beta' }],
            variant: 'enabled',
          },
        ],
      });
      expect(resolveFlag(flag, devContext()).variant).toBe('disabled');
    });
  });

  // -------------------------------------------------------------------------
  // Override handling
  // -------------------------------------------------------------------------

  describe('overrides', () => {
    it('invalid override on fail-closed class returns error', () => {
      // entitlement is fail-closed
      const overrides: FlagOverrides = { local: { 'test.flag': 'nonexistent' } };
      const result = resolveFlag(booleanFlag(), devContext(), overrides);
      expect(result.reason).toBe('error');
      expect(result.errorCode).toBe('INVALID_OVERRIDE_VARIANT');
    });

    it('invalid override on rollout class falls through with invalid_override_fallthrough reason', () => {
      const flag = booleanFlag({ class: 'rollout' });
      const overrides: FlagOverrides = { local: { 'test.flag': 'nonexistent' } };
      const result = resolveFlag(flag, devContext(), overrides);
      expect(result.reason).toBe('invalid_override_fallthrough');
      expect(result.variant).toBe('disabled');
    });

    it('invalid emergency override on rollout class falls through with invalid_override_fallthrough reason', () => {
      const flag = booleanFlag({ class: 'rollout' });
      const overrides: FlagOverrides = { emergency: { 'test.flag': 'nonexistent' } };
      const result = resolveFlag(flag, devContext(), overrides);
      expect(result.reason).toBe('invalid_override_fallthrough');
      expect(result.variant).toBe('disabled');
    });

    it('ignores override for different flag key', () => {
      const overrides: FlagOverrides = { emergency: { 'other.flag': 'enabled' } };
      const result = resolveFlag(booleanFlag(), devContext(), overrides);
      expect(result.reason).toBe('default');
    });
  });

  // -------------------------------------------------------------------------
  // Missing attribute handling (C-004, C-005)
  // -------------------------------------------------------------------------

  describe('missing attribute handling', () => {
    it('not_equals with missing attribute does not match', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'cohort', operator: 'not_equals', value: 'beta' }],
            variant: 'enabled',
          },
        ],
      });
      // No audience → cohort is undefined → should NOT match
      expect(resolveFlag(flag, devContext()).variant).toBe('disabled');
    });

    it('not_in_set with missing attribute does not match', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'cohort', operator: 'not_in_set', value: ['beta'] }],
            variant: 'enabled',
          },
        ],
      });
      expect(resolveFlag(flag, devContext()).variant).toBe('disabled');
    });

    it('in_set with missing attribute does not match', () => {
      const flag = booleanFlag({
        targeting: [
          {
            conditions: [{ attribute: 'cohort', operator: 'in_set', value: ['beta'] }],
            variant: 'enabled',
          },
        ],
      });
      expect(resolveFlag(flag, devContext()).variant).toBe('disabled');
    });
  });
});

// ---------------------------------------------------------------------------
// Percentage Rollout
// ---------------------------------------------------------------------------

describe('evaluatePercentage', () => {
  it('returns false for 0%', () => {
    expect(evaluatePercentage('any-key', 0)).toBe(false);
  });

  it('returns true for 100%', () => {
    expect(evaluatePercentage('any-key', 100)).toBe(true);
  });

  it('is deterministic for the same key', () => {
    const a = evaluatePercentage('stable-key', 50);
    const b = evaluatePercentage('stable-key', 50);
    expect(a).toBe(b);
  });

  it('different keys can produce different results', () => {
    const results = new Set<boolean>();
    for (let i = 0; i < 100; i++) {
      results.add(evaluatePercentage(`key-${i}`, 50));
    }
    // At 50%, we should see both true and false across 100 keys
    expect(results.size).toBe(2);
  });

  it('percentage rollout works with targeting', () => {
    const flag = booleanFlag({
      targeting: [
        {
          conditions: [{ attribute: 'rollout', operator: 'percentage', value: 100 }],
          variant: 'enabled',
        },
      ],
    });
    const result = resolveFlag(flag, devContext());
    expect(result.variant).toBe('enabled');
    expect(result.reason).toBe('targeting_match');
  });
});
