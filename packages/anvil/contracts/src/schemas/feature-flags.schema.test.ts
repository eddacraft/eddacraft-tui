import { describe, expect, it } from 'vitest';

import {
  FEATURE_FLAG_SCHEMA_VERSION,
  PRODUCT_CATALOGUE_SCHEMA_VERSION,
  ProductCatalogueManifestSchema,
  ProductCatalogueV1Schema,
  DeliverySurfaceLocatorSchema,
  FlagSurfaceManifestSchema,
  normaliseProductCatalogueV1,
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

function validProductCatalogue(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: PRODUCT_CATALOGUE_SCHEMA_VERSION,
    productFeatureGroups: [
      {
        key: 'governance',
        name: 'Governance engine',
        defaultSurfacePosture: { access: 'licence' },
        status: 'active',
      },
    ],
    productFeatures: [
      {
        key: 'check',
        name: 'Project checks',
        groupKey: 'governance',
        owner: 'CLI',
        status: 'active',
        requires: [],
        flagLinkage: {
          disposition: 'unflagged',
          reason: 'Fixture has no operational flag',
        },
      },
    ],
    deliverySurfaces: [
      {
        key: 'cli.check',
        featureKey: 'check',
        locator: { kind: 'cli', commandPath: ['check'] },
        posture: {
          invocation: 'user',
          mustAlwaysBeOpen: false,
        },
        status: 'active',
      },
    ],
    excludedDeliverySurfaces: [],
    deliverySurfaceMigrations: [],
    ...overrides,
  };
}

describe('ProductCatalogueManifestSchema', () => {
  it('rejects missing or malformed flag linkage', () => {
    const missingLinkage = validProductCatalogue();
    delete missingLinkage.productFeatures[0]!.flagLinkage;
    expect(ProductCatalogueManifestSchema.safeParse(missingLinkage).success).toBe(false);

    const emptyLinkedKeys = validProductCatalogue();
    emptyLinkedKeys.productFeatures[0]!.flagLinkage = {
      disposition: 'linked',
      flagKeys: [],
    };
    expect(ProductCatalogueManifestSchema.safeParse(emptyLinkedKeys).success).toBe(false);

    const duplicateLinkedKeys = validProductCatalogue();
    duplicateLinkedKeys.productFeatures[0]!.flagLinkage = {
      disposition: 'linked',
      flagKeys: ['cli.licence-gate', 'cli.licence-gate'],
    };
    expect(ProductCatalogueManifestSchema.safeParse(duplicateLinkedKeys).success).toBe(false);

    const emptyUnflaggedReason = validProductCatalogue();
    emptyUnflaggedReason.productFeatures[0]!.flagLinkage = {
      disposition: 'unflagged',
      reason: '',
    };
    expect(ProductCatalogueManifestSchema.safeParse(emptyUnflaggedReason).success).toBe(false);
  });

  it('rejects duplicate controlsProductFeatures keys', () => {
    expect(
      FeatureFlagDefinitionSchema.safeParse(
        validFlag({
          controlsProductFeatures: ['check', 'check'],
        })
      ).success
    ).toBe(false);
  });

  it('uses its own strict v2 schema independently of operational flags', () => {
    expect(PRODUCT_CATALOGUE_SCHEMA_VERSION).toBe(2);
    expect(FEATURE_FLAG_SCHEMA_VERSION).toBe(1);
    expect(ProductCatalogueManifestSchema.safeParse(validProductCatalogue()).success).toBe(true);
    expect(
      ProductCatalogueManifestSchema.safeParse({
        ...validProductCatalogue(),
        schemaVersion: FEATURE_FLAG_SCHEMA_VERSION,
      }).success
    ).toBe(false);
  });

  it('rejects invalid references, duplicate identities, posture violations, and cycles', () => {
    const invalidCatalogues: unknown[] = [];

    const unknownGroup = validProductCatalogue();
    unknownGroup.productFeatures[0]!.groupKey = 'missing';
    invalidCatalogues.push(unknownGroup);

    const duplicateFeature = validProductCatalogue();
    duplicateFeature.productFeatures.push({ ...duplicateFeature.productFeatures[0]! });
    invalidCatalogues.push(duplicateFeature);

    const unknownFeature = validProductCatalogue();
    unknownFeature.deliverySurfaces[0]!.featureKey = 'missing';
    invalidCatalogues.push(unknownFeature);

    const duplicateDeliveryIdentity = validProductCatalogue();
    duplicateDeliveryIdentity.excludedDeliverySurfaces.push({
      key: 'cli.check',
      locator: { kind: 'cli', commandPath: ['internal-check'] },
      owner: 'CLI',
      classification: 'internal-plumbing',
      reason: 'Internal command dispatcher',
      reviewReference: 'FLAGCAT-011',
      status: 'active',
    });
    invalidCatalogues.push(duplicateDeliveryIdentity);

    const mismatchedDeliveryHost = validProductCatalogue();
    mismatchedDeliveryHost.deliverySurfaces[0]!.locator = {
      kind: 'api-route',
      method: 'GET',
      path: '/check',
    };
    invalidCatalogues.push(mismatchedDeliveryHost);

    const missingRequirement = validProductCatalogue();
    missingRequirement.productFeatures[0]!.requires = ['missing'];
    invalidCatalogues.push(missingRequirement);

    const cycle = validProductCatalogue();
    cycle.productFeatures.push({
      key: 'gate',
      name: 'Project gate',
      groupKey: 'governance',
      owner: 'CLI',
      status: 'active',
      requires: ['check'],
    });
    cycle.productFeatures[0]!.requires = ['gate'];
    invalidCatalogues.push(cycle);

    const closedRecoveryFloor = validProductCatalogue();
    closedRecoveryFloor.deliverySurfaces[0]!.posture.mustAlwaysBeOpen = true;
    invalidCatalogues.push(closedRecoveryFloor);

    const audienceOnOpenSurface = validProductCatalogue();
    audienceOnOpenSurface.productFeatureGroups[0]!.defaultSurfacePosture.access = 'open';
    audienceOnOpenSurface.deliverySurfaces[0]!.posture.audiences = ['staff'];
    invalidCatalogues.push(audienceOnOpenSurface);

    for (const catalogue of invalidCatalogues) {
      expect(ProductCatalogueManifestSchema.safeParse(catalogue).success).toBe(false);
    }
  });

  it('accepts every approved strict locator, including the bare CLI path', () => {
    const locators = [
      { kind: 'cli', commandPath: [] },
      { kind: 'mcp-tool', name: 'anvil_status' },
      { kind: 'mcp-resource', uri: 'graph://stats' },
      { kind: 'api-route', method: 'GET', path: '/v1/status' },
      { kind: 'daemon-rpc', method: 'status' },
      { kind: 'dashboard-route', path: '/architecture' },
      { kind: 'docs-route', pathPrefix: '/docs' },
      { kind: 'hook', hook: 'pre-commit' },
      {
        kind: 'integration',
        integrationId: 'github',
        capability: 'checks',
      },
    ];

    for (const locator of locators) {
      expect(DeliverySurfaceLocatorSchema.safeParse(locator).success).toBe(true);
    }
    expect(DeliverySurfaceLocatorSchema.safeParse({ kind: 'cli', commandPath: [''] }).success).toBe(
      false
    );
    expect(
      DeliverySurfaceLocatorSchema.safeParse({ kind: 'cli', commandPath: [], extra: true }).success
    ).toBe(false);
  });

  it('records split and merge history while reserving every retired delivery identity', () => {
    const catalogue = validProductCatalogue({
      deliverySurfaces: [
        {
          key: 'cli.legacy-check',
          featureKey: 'check',
          locator: { kind: 'cli', commandPath: ['legacy-check'] },
          posture: { invocation: 'user', mustAlwaysBeOpen: false },
          status: 'retired',
        },
        {
          key: 'cli.legacy-lint',
          featureKey: 'check',
          locator: { kind: 'cli', commandPath: ['legacy-lint'] },
          posture: { invocation: 'user', mustAlwaysBeOpen: false },
          status: 'retired',
        },
        {
          key: 'cli.legacy-gate',
          featureKey: 'check',
          locator: { kind: 'cli', commandPath: ['legacy-gate'] },
          posture: { invocation: 'user', mustAlwaysBeOpen: false },
          status: 'retired',
        },
        {
          key: 'cli.check',
          featureKey: 'check',
          locator: { kind: 'cli', commandPath: ['check'] },
          posture: { invocation: 'user', mustAlwaysBeOpen: false },
          status: 'active',
        },
        {
          key: 'cli.check-report',
          featureKey: 'check',
          locator: { kind: 'cli', commandPath: ['check', 'report'] },
          posture: { invocation: 'user', mustAlwaysBeOpen: false },
          status: 'active',
        },
        {
          key: 'cli.validate',
          featureKey: 'check',
          locator: { kind: 'cli', commandPath: ['validate'] },
          posture: { invocation: 'user', mustAlwaysBeOpen: false },
          status: 'active',
        },
      ],
      deliverySurfaceMigrations: [
        {
          fromKeys: ['cli.legacy-check'],
          toKeys: ['cli.check', 'cli.check-report'],
        },
        {
          fromKeys: ['cli.legacy-lint', 'cli.legacy-gate'],
          toKeys: ['cli.validate'],
        },
      ],
    });

    expect(ProductCatalogueManifestSchema.safeParse(catalogue).success).toBe(true);

    const missingHistory = validProductCatalogue() as Record<string, unknown>;
    delete missingHistory.deliverySurfaceMigrations;
    expect(ProductCatalogueManifestSchema.safeParse(missingHistory).success).toBe(false);

    const missingSource = structuredClone(catalogue);
    missingSource.deliverySurfaceMigrations[0]!.fromKeys = ['cli.missing'];
    expect(ProductCatalogueManifestSchema.safeParse(missingSource).success).toBe(false);

    const missingTarget = structuredClone(catalogue);
    missingTarget.deliverySurfaceMigrations[0]!.toKeys = ['cli.missing'];
    expect(ProductCatalogueManifestSchema.safeParse(missingTarget).success).toBe(false);

    const reusedRetiredKey = structuredClone(catalogue);
    reusedRetiredKey.deliverySurfaces[0]!.status = 'active';
    expect(ProductCatalogueManifestSchema.safeParse(reusedRetiredKey).success).toBe(false);

    const retiredTarget = structuredClone(catalogue);
    retiredTarget.deliverySurfaces[3]!.status = 'retired';
    expect(ProductCatalogueManifestSchema.safeParse(retiredTarget).success).toBe(false);

    const duplicateHistory = structuredClone(catalogue);
    duplicateHistory.deliverySurfaceMigrations[1]!.fromKeys.push('cli.legacy-check');
    expect(ProductCatalogueManifestSchema.safeParse(duplicateHistory).success).toBe(false);

    const duplicateSource = structuredClone(catalogue);
    duplicateSource.deliverySurfaceMigrations[0]!.fromKeys.push('cli.legacy-check');
    expect(ProductCatalogueManifestSchema.safeParse(duplicateSource).success).toBe(false);

    const duplicateTarget = structuredClone(catalogue);
    duplicateTarget.deliverySurfaceMigrations[0]!.toKeys.push('cli.check');
    expect(ProductCatalogueManifestSchema.safeParse(duplicateTarget).success).toBe(false);
  });

  it('rejects a retired delivery identity without migration history', () => {
    const catalogue = validProductCatalogue();
    catalogue.deliverySurfaces[0]!.status = 'retired';

    expect(ProductCatalogueManifestSchema.safeParse(catalogue).success).toBe(false);
  });

  it('rejects a retired excluded identity without migration history', () => {
    const catalogue = validProductCatalogue({
      excludedDeliverySurfaces: [
        {
          key: 'cli.internal-check',
          locator: { kind: 'cli', commandPath: ['internal-check'] },
          owner: 'CLI',
          classification: 'internal-plumbing',
          reason: 'Retired internal dispatcher',
          reviewReference: 'FLAGCAT-011',
          status: 'retired',
        },
      ],
    });

    expect(ProductCatalogueManifestSchema.safeParse(catalogue).success).toBe(false);
  });
});

describe('ProductCatalogueV1Schema', () => {
  const legacyCatalogue = {
    schemaVersion: 1,
    categories: [
      {
        id: 'foundational',
        name: 'Foundational plumbing',
        defaultAccess: 'open',
        defaultStatus: 'active',
      },
    ],
    surfaces: [
      {
        key: 'config',
        name: 'anvil config',
        category: 'foundational',
        catalogued: false,
      },
    ],
  };

  it('is the frozen strict v1 parser and legacy schema alias', () => {
    expect(ProductCatalogueV1Schema.parse(legacyCatalogue)).toEqual(
      FlagSurfaceManifestSchema.parse(legacyCatalogue)
    );
    expect(
      ProductCatalogueV1Schema.safeParse({ ...legacyCatalogue, unexpected: true }).success
    ).toBe(false);
    expect(
      ProductCatalogueV1Schema.safeParse({
        ...legacyCatalogue,
        surfaces: [{ ...legacyCatalogue.surfaces[0], unexpected: true }],
      }).success
    ).toBe(false);
  });
});

describe('normaliseProductCatalogueV1', () => {
  const legacyCatalogue = {
    schemaVersion: 1 as const,
    categories: [
      {
        id: 'governance',
        name: 'Governance engine',
        defaultAccess: 'licence' as const,
        defaultStatus: 'active' as const,
      },
      {
        id: 'foundational',
        name: 'Foundational plumbing',
        defaultAccess: 'open' as const,
        defaultStatus: 'active' as const,
      },
    ],
    surfaces: [
      {
        key: 'check',
        name: 'anvil check',
        category: 'governance',
        access: 'staff' as const,
        audiences: ['staff-internal-developer'],
        invocation: 'system' as const,
        mustAlwaysBeOpen: false,
        requires: ['config'],
        notes: 'Legacy note',
      },
      {
        key: 'config',
        name: 'anvil config',
        category: 'foundational',
        catalogued: false,
        mustAlwaysBeOpen: true,
      },
    ],
  };

  const migrationByFeatureKey = {
    check: {
      owner: 'CLI',
      deliveryKey: 'cli.project-check',
      locator: { kind: 'cli' as const, commandPath: ['renamed', 'check'] },
    },
    config: {
      owner: 'CLI',
      deliveryKey: 'cli.configuration',
      locator: { kind: 'cli' as const, commandPath: ['config'] },
    },
  };

  it('preserves v1 semantics using only explicit curated delivery identities', () => {
    const normalised = normaliseProductCatalogueV1(legacyCatalogue, migrationByFeatureKey);

    expect(normalised.schemaVersion).toBe(PRODUCT_CATALOGUE_SCHEMA_VERSION);
    expect(normalised.productFeatureGroups.map((group) => group.key)).toEqual([
      'governance',
      'foundational',
    ]);
    expect(normalised.productFeatures).toEqual([
      {
        key: 'check',
        name: 'anvil check',
        groupKey: 'governance',
        owner: 'CLI',
        status: 'active',
        requires: ['config'],
        flagLinkage: {
          disposition: 'unflagged',
          reason: 'v1 compatibility projection; operational-flag linkage is canonical v2-only',
        },
        notes: 'Legacy note',
      },
      {
        key: 'config',
        name: 'anvil config',
        groupKey: 'foundational',
        owner: 'CLI',
        status: 'active',
        requires: [],
        flagLinkage: {
          disposition: 'unflagged',
          reason: 'v1 compatibility projection; operational-flag linkage is canonical v2-only',
        },
      },
    ]);
    expect(normalised.deliverySurfaces[0]).toEqual({
      key: 'cli.project-check',
      featureKey: 'check',
      locator: { kind: 'cli', commandPath: ['renamed', 'check'] },
      posture: {
        access: 'staff',
        audiences: ['staff-internal-developer'],
        invocation: 'system',
        mustAlwaysBeOpen: false,
      },
      status: 'active',
    });
    expect(normalised.excludedDeliverySurfaces).toEqual([]);
    expect(normalised.productFeatures.some((feature) => feature.key === 'config')).toBe(true);
  });

  it('fails closed for a retired v1 delivery identity without replacement history', () => {
    const retiredCatalogue = {
      ...legacyCatalogue,
      surfaces: [
        {
          ...legacyCatalogue.surfaces[0]!,
          status: 'retired' as const,
        },
        legacyCatalogue.surfaces[1]!,
      ],
    };

    expect(() => normaliseProductCatalogueV1(retiredCatalogue, migrationByFeatureKey)).toThrow(
      'requires migration history'
    );
  });

  it('fails closed when migration curation is missing, extra, blank, or duplicated', () => {
    expect(() => normaliseProductCatalogueV1(legacyCatalogue, {})).toThrow();
    expect(() =>
      normaliseProductCatalogueV1(legacyCatalogue, {
        ...migrationByFeatureKey,
        unexpected: migrationByFeatureKey.check,
      })
    ).toThrow();
    expect(() =>
      normaliseProductCatalogueV1(legacyCatalogue, {
        ...migrationByFeatureKey,
        check: { ...migrationByFeatureKey.check, owner: '' },
      })
    ).toThrow();
    expect(() =>
      normaliseProductCatalogueV1(legacyCatalogue, {
        ...migrationByFeatureKey,
        config: { ...migrationByFeatureKey.config, deliveryKey: 'cli.project-check' },
      })
    ).toThrow();
  });
});

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
