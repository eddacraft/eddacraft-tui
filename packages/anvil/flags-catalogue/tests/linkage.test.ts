import { describe, expect, it } from 'vitest';

import { featureFlagManifest, productCatalogue } from '../src/index.js';

describe('FLAGCAT-013 operational flag linkage', () => {
  const flags = featureFlagManifest().flags;
  const features = productCatalogue().productFeatures;

  it('every manifest flag declares controlsProductFeatures', () => {
    for (const flag of flags) {
      expect(flag.controlsProductFeatures, flag.key).toBeDefined();
    }
  });

  it('every product feature declares a reviewed linkage disposition', () => {
    for (const feature of features) {
      expect(['linked', 'unflagged']).toContain(feature.flagLinkage.disposition);
      if (feature.flagLinkage.disposition === 'unflagged') {
        expect(feature.flagLinkage.reason.length).toBeGreaterThan(0);
      } else {
        expect(feature.flagLinkage.flagKeys.length).toBeGreaterThan(0);
      }
    }
  });

  it('linked features and controlling flags agree in both directions', () => {
    const flagsByFeature = new Map<string, string[]>();
    for (const flag of flags) {
      for (const featureKey of flag.controlsProductFeatures ?? []) {
        flagsByFeature.set(featureKey, [...(flagsByFeature.get(featureKey) ?? []), flag.key]);
      }
    }

    for (const feature of features) {
      const actual = [...(flagsByFeature.get(feature.key) ?? [])].sort();
      if (feature.flagLinkage.disposition === 'linked') {
        expect(actual, feature.key).toEqual([...feature.flagLinkage.flagKeys].sort());
      } else {
        expect(actual, feature.key).toEqual([]);
      }
    }
  });
});
