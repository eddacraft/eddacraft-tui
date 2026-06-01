import { describe, expect, it } from 'vitest';

import {
  FEATURE_FLAG_SCHEMA_VERSION,
  FlagClassSchema,
  FlagStatusSchema,
  FlagValueTypeSchema,
  FeatureFlagDefinitionSchema,
  FeatureFlagManifestSchema,
  EnvironmentNameSchema,
  ChannelSchema,
  EnvironmentContextSchema,
  AudienceContextSchema,
  EvaluationContextSchema,
  TargetingOperatorSchema,
  TargetingConditionSchema,
  TargetingRuleSchema,
  validateManifest,
  defaultVariantExists,
  failClosedClasses,
} from './feature-flags.schema.js';
import type { FeatureFlagDefinition } from './feature-flags.schema.js';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

function validFlag(overrides: Record<string, unknown> = {}) {
  return {
    key: 'cli.licence-gate',
    owner: 'BAUTH',
    intent: 'Gate CLI features behind licence validation',
    class: 'entitlement' as const,
    valueType: 'boolean' as const,
    variants: [
      { key: 'enabled', value: true },
      { key: 'disabled', value: false },
    ],
    defaultVariant: 'disabled',
    status: 'active' as const,
    createdFor: 'FLAGS-008',
    ...overrides,
  };
}

function validManifest(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: FEATURE_FLAG_SCHEMA_VERSION,
    flags: [validFlag()],
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Flag Class
// ---------------------------------------------------------------------------

describe('FlagClassSchema', () => {
  it.each(['rollout', 'entitlement', 'ops_kill_switch'])('accepts "%s"', (v) => {
    expect(FlagClassSchema.parse(v)).toBe(v);
  });

  it('rejects unknown class', () => {
    expect(FlagClassSchema.safeParse('experiment').success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Flag Status
// ---------------------------------------------------------------------------

describe('FlagStatusSchema', () => {
  it.each(['draft', 'active', 'retiring', 'retired'])('accepts "%s"', (v) => {
    expect(FlagStatusSchema.parse(v)).toBe(v);
  });

  it('rejects unknown status', () => {
    expect(FlagStatusSchema.safeParse('archived').success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Flag Value Type
// ---------------------------------------------------------------------------

describe('FlagValueTypeSchema', () => {
  it.each(['boolean', 'string', 'number', 'object'])('accepts "%s"', (v) => {
    expect(FlagValueTypeSchema.parse(v)).toBe(v);
  });

  it('rejects unknown value type', () => {
    expect(FlagValueTypeSchema.safeParse('array').success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Feature Flag Definition
// ---------------------------------------------------------------------------

describe('FeatureFlagDefinitionSchema', () => {
  it('accepts a valid flag definition', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(validFlag());
    expect(result.success).toBe(true);
  });

  it('accepts flag with optional fields', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        expiryOrReviewDate: '2026-07-01T00:00:00Z',
        description: 'Controls CLI licence gating',
      })
    );
    expect(result.success).toBe(true);
  });

  describe('key format', () => {
    it.each(['cli.licence-gate', 'docs_access', 'opa-rollout', 'simple'])(
      'accepts valid key "%s"',
      (key) => {
        const result = FeatureFlagDefinitionSchema.safeParse(validFlag({ key }));
        expect(result.success).toBe(true);
      }
    );

    it.each(['', 'CLI.gate', '123-start', 'has spaces', 'UPPER'])(
      'rejects invalid key "%s"',
      (key) => {
        const result = FeatureFlagDefinitionSchema.safeParse(validFlag({ key }));
        expect(result.success).toBe(false);
      }
    );
  });

  it('requires at least two variants', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({ variants: [{ key: 'only', value: true }] })
    );
    expect(result.success).toBe(false);
  });

  it('rejects missing required fields', () => {
    const { owner: _, ...noOwner } = validFlag();
    expect(FeatureFlagDefinitionSchema.safeParse(noOwner).success).toBe(false);

    const { intent: __, ...noIntent } = validFlag();
    expect(FeatureFlagDefinitionSchema.safeParse(noIntent).success).toBe(false);

    const { createdFor: ___, ...noCreatedFor } = validFlag();
    expect(FeatureFlagDefinitionSchema.safeParse(noCreatedFor).success).toBe(false);
  });

  it('rejects duplicate variant keys', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        variants: [
          { key: 'enabled', value: true },
          { key: 'enabled', value: false },
        ],
      })
    );
    expect(result.success).toBe(false);
  });

  it('rejects defaultVariant not referencing an existing variant', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({ defaultVariant: 'nonexistent' })
    );
    expect(result.success).toBe(false);
  });

  it('supports object variant values', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        valueType: 'object',
        defaultVariant: 'full',
        variants: [
          { key: 'full', value: { maxItems: 100, tier: 'pro' } },
          { key: 'limited', value: { maxItems: 10, tier: 'free' } },
        ],
      })
    );
    expect(result.success).toBe(true);
  });

  it('rejects rollout flags without expiryOrReviewDate', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(validFlag({ class: 'rollout' }));
    expect(result.success).toBe(false);
  });

  it('accepts rollout flags with expiryOrReviewDate', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({ class: 'rollout', expiryOrReviewDate: '2026-07-01T00:00:00Z' })
    );
    expect(result.success).toBe(true);
  });

  it('does not require expiryOrReviewDate for entitlement flags', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(validFlag({ class: 'entitlement' }));
    expect(result.success).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// valueType-variant alignment
// ---------------------------------------------------------------------------

describe('FeatureFlagDefinitionSchema valueType-variant alignment', () => {
  it('rejects boolean flag with string variant value', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        valueType: 'boolean',
        variants: [
          { key: 'enabled', value: 'yes' },
          { key: 'disabled', value: false },
        ],
      })
    );
    expect(result.success).toBe(false);
  });

  it('rejects string flag with boolean variant value', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        valueType: 'string',
        variants: [
          { key: 'on', value: true },
          { key: 'off', value: 'off' },
        ],
      })
    );
    expect(result.success).toBe(false);
  });

  it('rejects number flag with string variant value', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        valueType: 'number',
        variants: [
          { key: 'high', value: 'ten' },
          { key: 'low', value: 1 },
        ],
      })
    );
    expect(result.success).toBe(false);
  });

  it('rejects object flag with primitive variant value', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        valueType: 'object',
        defaultVariant: 'full',
        variants: [
          { key: 'full', value: 42 },
          { key: 'limited', value: { maxItems: 10 } },
        ],
      })
    );
    expect(result.success).toBe(false);
  });

  it('accepts boolean flag with boolean variant values', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(validFlag());
    expect(result.success).toBe(true);
  });

  it('accepts object flag with object variant values', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        valueType: 'object',
        defaultVariant: 'full',
        variants: [
          { key: 'full', value: { maxItems: 100, tier: 'pro' } },
          { key: 'limited', value: { maxItems: 10, tier: 'free' } },
        ],
      })
    );
    expect(result.success).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Feature Flag Manifest
// ---------------------------------------------------------------------------

describe('FeatureFlagManifestSchema', () => {
  it('accepts a valid manifest', () => {
    const result = FeatureFlagManifestSchema.safeParse(validManifest());
    expect(result.success).toBe(true);
  });

  it('rejects wrong schema version', () => {
    const result = FeatureFlagManifestSchema.safeParse(validManifest({ schemaVersion: 99 }));
    expect(result.success).toBe(false);
  });

  it('accepts empty flags array', () => {
    const result = FeatureFlagManifestSchema.safeParse(validManifest({ flags: [] }));
    expect(result.success).toBe(true);
  });

  it('accepts multiple flags', () => {
    const result = FeatureFlagManifestSchema.safeParse(
      validManifest({
        flags: [
          validFlag(),
          validFlag({ key: 'docs.access', owner: 'DOCSAUTH', createdFor: 'FLAGS-008' }),
        ],
      })
    );
    expect(result.success).toBe(true);
  });

  it('rejects duplicate flag keys', () => {
    const result = FeatureFlagManifestSchema.safeParse(
      validManifest({
        flags: [validFlag(), validFlag()],
      })
    );
    expect(result.success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Utility Functions
// ---------------------------------------------------------------------------

describe('validateManifest', () => {
  it('returns success for valid data', () => {
    const result = validateManifest(validManifest());
    expect(result.success).toBe(true);
  });

  it('returns failure for invalid data', () => {
    const result = validateManifest({ schemaVersion: 99 });
    expect(result.success).toBe(false);
  });
});

describe('defaultVariantExists', () => {
  it('returns true when default variant is present', () => {
    expect(defaultVariantExists(validFlag() as FeatureFlagDefinition)).toBe(true);
  });

  it('returns false when default variant is missing', () => {
    expect(
      defaultVariantExists(validFlag({ defaultVariant: 'nonexistent' }) as FeatureFlagDefinition)
    ).toBe(false);
  });
});

describe('failClosedClasses', () => {
  it('includes ops_kill_switch and entitlement', () => {
    const classes = failClosedClasses();
    expect(classes).toContain('ops_kill_switch');
    expect(classes).toContain('entitlement');
  });

  it('does not include rollout', () => {
    expect(failClosedClasses()).not.toContain('rollout');
  });
});

// ---------------------------------------------------------------------------
// Environment Targeting (FLAGS-002)
// ---------------------------------------------------------------------------

describe('EnvironmentNameSchema', () => {
  it.each(['local', 'development', 'preview', 'demo', 'production'])('accepts "%s"', (v) => {
    expect(EnvironmentNameSchema.parse(v)).toBe(v);
  });

  it('rejects renamed/dropped environments', () => {
    for (const old of ['dev', 'prod', 'staging', 'test']) {
      expect(EnvironmentNameSchema.safeParse(old).success).toBe(false);
    }
  });
});

describe('ChannelSchema', () => {
  it.each(['development', 'beta', 'production'])('accepts "%s"', (v) => {
    expect(ChannelSchema.parse(v)).toBe(v);
  });

  it('rejects unknown channel', () => {
    expect(ChannelSchema.safeParse('nightly').success).toBe(false);
  });
});

describe('EnvironmentContextSchema', () => {
  it('accepts minimal context', () => {
    const result = EnvironmentContextSchema.safeParse({ environment: 'production' });
    expect(result.success).toBe(true);
  });

  it('accepts full context', () => {
    const result = EnvironmentContextSchema.safeParse({
      environment: 'demo',
      channel: 'beta',
      deploymentRing: 'canary',
    });
    expect(result.success).toBe(true);
  });

  it('rejects missing environment', () => {
    expect(EnvironmentContextSchema.safeParse({}).success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Audience Targeting (FLAGS-002)
// ---------------------------------------------------------------------------

describe('AudienceContextSchema', () => {
  it('accepts empty audience', () => {
    expect(AudienceContextSchema.safeParse({}).success).toBe(true);
  });

  it('accepts full audience', () => {
    const result = AudienceContextSchema.safeParse({
      accountTier: 'pro',
      licencePlan: 'team',
      organisationId: 'org-123',
      userRole: 'admin',
      cohort: 'early-adopter',
    });
    expect(result.success).toBe(true);
  });

  it('accepts partial audience', () => {
    const result = AudienceContextSchema.safeParse({
      accountTier: 'free',
    });
    expect(result.success).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Evaluation Context (FLAGS-002)
// ---------------------------------------------------------------------------

describe('EvaluationContextSchema', () => {
  it('accepts minimal context', () => {
    const result = EvaluationContextSchema.safeParse({
      targetingKey: 'session-abc',
      environment: { environment: 'development' },
    });
    expect(result.success).toBe(true);
  });

  it('accepts full context', () => {
    const result = EvaluationContextSchema.safeParse({
      targetingKey: 'session-xyz',
      environment: { environment: 'production', channel: 'production' },
      audience: { accountTier: 'enterprise', userRole: 'admin' },
    });
    expect(result.success).toBe(true);
  });

  it('rejects missing targetingKey', () => {
    const result = EvaluationContextSchema.safeParse({
      environment: { environment: 'development' },
    });
    expect(result.success).toBe(false);
  });

  it('rejects empty targetingKey', () => {
    const result = EvaluationContextSchema.safeParse({
      targetingKey: '',
      environment: { environment: 'development' },
    });
    expect(result.success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Targeting Operators (FLAGS-002)
// ---------------------------------------------------------------------------

describe('TargetingOperatorSchema', () => {
  it.each(['equals', 'not_equals', 'in_set', 'not_in_set', 'percentage', 'segment'])(
    'accepts "%s"',
    (v) => {
      expect(TargetingOperatorSchema.parse(v)).toBe(v);
    }
  );

  it('rejects unknown operator', () => {
    expect(TargetingOperatorSchema.safeParse('greater_than').success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Targeting Rules (FLAGS-002)
// ---------------------------------------------------------------------------

describe('TargetingConditionSchema', () => {
  it('accepts string value condition', () => {
    const result = TargetingConditionSchema.safeParse({
      attribute: 'environment',
      operator: 'equals',
      value: 'prod',
    });
    expect(result.success).toBe(true);
  });

  it('accepts numeric value condition (percentage)', () => {
    const result = TargetingConditionSchema.safeParse({
      attribute: 'rollout',
      operator: 'percentage',
      value: 25,
    });
    expect(result.success).toBe(true);
  });

  it('accepts set value condition', () => {
    const result = TargetingConditionSchema.safeParse({
      attribute: 'accountTier',
      operator: 'in_set',
      value: ['pro', 'enterprise'],
    });
    expect(result.success).toBe(true);
  });

  it('rejects array value for equals operator', () => {
    const result = TargetingConditionSchema.safeParse({
      attribute: 'environment',
      operator: 'equals',
      value: ['prod'],
    });
    expect(result.success).toBe(false);
  });

  it('rejects string value for in_set operator', () => {
    const result = TargetingConditionSchema.safeParse({
      attribute: 'accountTier',
      operator: 'in_set',
      value: 'pro',
    });
    expect(result.success).toBe(false);
  });

  it('rejects string value for percentage operator', () => {
    const result = TargetingConditionSchema.safeParse({
      attribute: 'rollout',
      operator: 'percentage',
      value: '25',
    });
    expect(result.success).toBe(false);
  });

  it('rejects empty attribute', () => {
    const result = TargetingConditionSchema.safeParse({
      attribute: '',
      operator: 'equals',
      value: 'prod',
    });
    expect(result.success).toBe(false);
  });
});

describe('TargetingRuleSchema', () => {
  it('accepts valid rule with single condition', () => {
    const result = TargetingRuleSchema.safeParse({
      conditions: [{ attribute: 'environment', operator: 'equals', value: 'prod' }],
      variant: 'enabled',
    });
    expect(result.success).toBe(true);
  });

  it('accepts rule with multiple conditions (AND semantics)', () => {
    const result = TargetingRuleSchema.safeParse({
      conditions: [
        { attribute: 'environment', operator: 'equals', value: 'prod' },
        { attribute: 'accountTier', operator: 'in_set', value: ['pro', 'enterprise'] },
      ],
      variant: 'enabled',
    });
    expect(result.success).toBe(true);
  });

  it('rejects rule with empty conditions', () => {
    const result = TargetingRuleSchema.safeParse({
      conditions: [],
      variant: 'enabled',
    });
    expect(result.success).toBe(false);
  });

  it('rejects rule with empty variant', () => {
    const result = TargetingRuleSchema.safeParse({
      conditions: [{ attribute: 'environment', operator: 'equals', value: 'prod' }],
      variant: '',
    });
    expect(result.success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Flag Definition with Targeting (FLAGS-002)
// ---------------------------------------------------------------------------

describe('FeatureFlagDefinitionSchema with targeting', () => {
  it('accepts flag with targeting rules', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        targeting: [
          {
            conditions: [{ attribute: 'environment', operator: 'equals', value: 'prod' }],
            variant: 'enabled',
          },
        ],
      })
    );
    expect(result.success).toBe(true);
  });

  it('accepts flag without targeting (optional)', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(validFlag());
    expect(result.success).toBe(true);
  });

  it('accepts flag with multiple targeting rules', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        targeting: [
          {
            conditions: [{ attribute: 'environment', operator: 'equals', value: 'prod' }],
            variant: 'enabled',
          },
          {
            conditions: [
              { attribute: 'accountTier', operator: 'in_set', value: ['pro'] },
              { attribute: 'channel', operator: 'equals', value: 'beta' },
            ],
            variant: 'enabled',
          },
        ],
      })
    );
    expect(result.success).toBe(true);
  });

  it('accepts percentage rollout targeting', () => {
    const result = FeatureFlagDefinitionSchema.safeParse(
      validFlag({
        targeting: [
          {
            conditions: [{ attribute: 'rollout', operator: 'percentage', value: 25 }],
            variant: 'enabled',
          },
        ],
      })
    );
    expect(result.success).toBe(true);
  });
});
