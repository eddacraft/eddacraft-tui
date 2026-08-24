import { describe, expect, it } from 'vitest';
import { ProductCatalogueV1Schema } from '@eddacraft/anvil-contracts';
import * as packageRoot from '../src/index.js';
import {
  flagSurfaces,
  mustAlwaysBeOpenDeliverySurfaces,
  mustAlwaysBeOpenSurfaces,
  productCatalogue,
} from '../src/index.js';
import { normaliseProductCatalogueDocument } from '../src/manifest.js';
import { productCatalogueV1Migration } from '../src/compatibility/product-catalogue-v1-migration.js';
import v1FixtureJson from '../src/compatibility/product-catalogue-v1.json' with { type: 'json' };

function assertCatalogueMutationsAreRejected(): void {
  const catalogue = productCatalogue();
  // @ts-expect-error the authoritative collection is deeply readonly
  catalogue.productFeatures.push(catalogue.productFeatures[0]!);
  // @ts-expect-error nested catalogue arrays are deeply readonly
  catalogue.productFeatures[0]!.requires.push('other-feature');
}

describe('product catalogue compatibility reader', () => {
  it('does not expose structural document normalisation from the package root', () => {
    expect(packageRoot).not.toHaveProperty('loadProductCatalogue');
    expect(packageRoot).not.toHaveProperty('normaliseProductCatalogueDocument');
  });

  it('migrates the exact frozen 46-entry v1 catalogue with full rollback parity', () => {
    const v1 = ProductCatalogueV1Schema.parse(v1FixtureJson);
    const migrated = normaliseProductCatalogueDocument(v1FixtureJson);

    expect(Object.keys(productCatalogueV1Migration)).toEqual(
      v1.surfaces.map((surface) => surface.key)
    );
    expect(migrated.schemaVersion).toBe(2);
    expect(migrated.productFeatureGroups).toHaveLength(9);
    expect(migrated.productFeatures).toHaveLength(46);
    expect(migrated.deliverySurfaces).toHaveLength(46);
    expect(migrated.excludedDeliverySurfaces).toEqual([]);
    expect(migrated.deliverySurfaceMigrations).toEqual([]);

    for (const category of v1.categories) {
      const group = migrated.productFeatureGroups.find(
        (candidate) => candidate.key === category.id
      );
      expect(group, category.id).toEqual({
        key: category.id,
        name: category.name,
        defaultSurfacePosture: { access: category.defaultAccess },
        status: category.defaultStatus,
      });
    }

    for (const surface of v1.surfaces) {
      const feature = migrated.productFeatures.find((candidate) => candidate.key === surface.key);
      const delivery = migrated.deliverySurfaces.find(
        (candidate) => candidate.featureKey === surface.key
      );
      const category = v1.categories.find((candidate) => candidate.id === surface.category)!;

      expect(feature, surface.key).toEqual({
        key: surface.key,
        name: surface.name,
        groupKey: surface.category,
        owner: productCatalogueV1Migration[surface.key]?.owner,
        status: surface.status,
        requires: surface.requires,
        flagLinkage: {
          disposition: 'unflagged',
          reason: 'v1 compatibility projection; operational-flag linkage is canonical v2-only',
        },
        ...(surface.notes === undefined ? {} : { notes: surface.notes }),
      });
      expect(delivery, surface.key).toEqual({
        key: productCatalogueV1Migration[surface.key]?.deliveryKey,
        featureKey: surface.key,
        locator: productCatalogueV1Migration[surface.key]?.locator,
        posture: {
          access: surface.access ?? category.defaultAccess,
          ...(surface.audiences === undefined ? {} : { audiences: surface.audiences }),
          invocation: surface.invocation,
          mustAlwaysBeOpen: surface.mustAlwaysBeOpen,
        },
        status: surface.status,
      });
    }

    expect(flagSurfaces()).toEqual(v1);
  });

  it('strictly discriminates canonical v2 and unsupported payloads', () => {
    const canonical = productCatalogue();

    expect(normaliseProductCatalogueDocument(canonical)).toEqual(canonical);
    expect(() =>
      normaliseProductCatalogueDocument({
        ...canonical,
        categories: [],
      })
    ).toThrow();
    expect(() => normaliseProductCatalogueDocument({ schemaVersion: 3 })).toThrow(
      'unsupported product catalogue schemaVersion: 3'
    );
    expect(() => normaliseProductCatalogueDocument({})).toThrow(
      'unsupported product catalogue schemaVersion: undefined'
    );
  });
});

describe('read-only catalogue accessors', () => {
  it('deep-freezes v2 and keeps legacy feature keys separate from the v2 delivery floor', () => {
    const catalogue = productCatalogue();
    const nestedValues = [
      catalogue,
      catalogue.productFeatureGroups,
      catalogue.productFeatureGroups[0],
      catalogue.productFeatureGroups[0]?.defaultSurfacePosture,
      catalogue.productFeatures,
      catalogue.productFeatures[0],
      catalogue.productFeatures[0]?.requires,
      catalogue.deliverySurfaces,
      catalogue.deliverySurfaces[0],
      catalogue.deliverySurfaces[0]?.locator,
      catalogue.deliverySurfaces[0]?.posture,
      catalogue.excludedDeliverySurfaces,
      catalogue.deliverySurfaceMigrations,
    ];

    for (const value of nestedValues) {
      expect(Object.isFrozen(value)).toBe(true);
    }

    expect(assertCatalogueMutationsAreRejected).toBeTypeOf('function');

    expect(mustAlwaysBeOpenSurfaces()).toEqual(['admin.credential', 'auth']);
    expect(mustAlwaysBeOpenDeliverySurfaces()).toEqual([
      'cli.admin-credential',
      'cli.auth-login',
      'cli.login-alias',
      'cli.auth-refresh',
      'docs.auth-login',
      'docs.auth-callback',
      'api.auth-license-refresh',
      'api.auth-otp-request',
      'api.auth-otp-verify',
      'api.auth-session-refresh',
      'api.auth-github-callback',
      'api.auth-github-device-start',
      'api.auth-github-device-poll',
    ]);
    expect(Object.isFrozen(mustAlwaysBeOpenSurfaces())).toBe(true);
    expect(Object.isFrozen(mustAlwaysBeOpenDeliverySurfaces())).toBe(true);
  });
});
