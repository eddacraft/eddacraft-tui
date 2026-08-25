import { describe, expect, it } from 'vitest';

import {
  CANONICAL_PLAN_AXIS_AUDIENCE_IDS,
  type CanonicalPlanAxisAudienceId,
  type PlanAvailabilityDisposition,
} from '@eddacraft/anvil-contracts';
import { featureFlagManifest, flagAudiences, productCatalogue } from '../src/index.js';

const PLAN_DISPOSITIONS: PlanAvailabilityDisposition[] = ['available', 'unavailable', 'undecided'];

function planTargets(flag: {
  targeting?: Array<{
    variant?: string;
    conditions: Array<{ attribute: string; operator?: string; value: unknown }>;
  }>;
}): Set<CanonicalPlanAxisAudienceId> {
  const found = new Set<CanonicalPlanAxisAudienceId>();
  for (const rule of flag.targeting ?? []) {
    if (rule.variant !== undefined && rule.variant !== 'enabled') continue;
    for (const condition of rule.conditions) {
      if (condition.attribute !== 'accountTier' || condition.operator !== 'in_set') continue;
      const values = Array.isArray(condition.value) ? condition.value : [condition.value];
      for (const value of values) {
        if (
          typeof value === 'string' &&
          (CANONICAL_PLAN_AXIS_AUDIENCE_IDS as readonly string[]).includes(value)
        ) {
          found.add(value as CanonicalPlanAxisAudienceId);
        }
      }
    }
  }
  return found;
}

describe('FLAGCAT-015 plan-audience availability', () => {
  const features = productCatalogue().productFeatures;
  const flags = featureFlagManifest().flags;
  const flagsByKey = new Map(flags.map((flag) => [flag.key, flag]));
  const planAudienceIds = flagAudiences()
    .audiences.filter((audience) => audience.axis === 'plan' && audience.status === 'active')
    .map((audience) => audience.id)
    .sort();

  it('uses the live plan-axis audience ids as the approved vocabulary', () => {
    expect(planAudienceIds).toEqual([...CANONICAL_PLAN_AXIS_AUDIENCE_IDS].sort());
  });

  it('records a reviewed disposition for every product feature and plan id', () => {
    for (const feature of features) {
      expect(Object.keys(feature.planAvailability).sort(), feature.key).toEqual(
        [...CANONICAL_PLAN_AXIS_AUDIENCE_IDS].sort()
      );
      for (const planId of CANONICAL_PLAN_AXIS_AUDIENCE_IDS) {
        expect(PLAN_DISPOSITIONS, `${feature.key}:${planId}`).toContain(
          feature.planAvailability[planId]
        );
      }
    }
  });

  it('maps entitlement plan targeting to available and leaves other plan ids unavailable', () => {
    for (const feature of features) {
      if (feature.flagLinkage.disposition !== 'linked') {
        expect(
          CANONICAL_PLAN_AXIS_AUDIENCE_IDS.every(
            (planId) => feature.planAvailability[planId] === 'undecided'
          ),
          feature.key
        ).toBe(true);
        continue;
      }

      const evidence = new Set<CanonicalPlanAxisAudienceId>();
      let hasPlanEntitlement = false;
      for (const flagKey of feature.flagLinkage.flagKeys) {
        const flag = flagsByKey.get(flagKey);
        expect(flag, `${feature.key}:${flagKey}`).toBeDefined();
        if (flag?.class !== 'entitlement') continue;
        const targets = planTargets(flag);
        if (targets.size > 0) {
          hasPlanEntitlement = true;
          for (const planId of targets) evidence.add(planId);
        }
      }

      if (!hasPlanEntitlement) {
        expect(
          CANONICAL_PLAN_AXIS_AUDIENCE_IDS.every(
            (planId) => feature.planAvailability[planId] === 'undecided'
          ),
          feature.key
        ).toBe(true);
        continue;
      }

      for (const planId of CANONICAL_PLAN_AXIS_AUDIENCE_IDS) {
        expect(feature.planAvailability[planId], `${feature.key}:${planId}`).toBe(
          evidence.has(planId) ? 'available' : 'unavailable'
        );
      }
    }
  });
});
