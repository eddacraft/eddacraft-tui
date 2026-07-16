import { describe, expect, it } from 'vitest';
import { buildFleetOverview, type FleetBeaconRow } from '../lib/fleet-overview.js';

const AS_OF = '2026-07-16';

interface SeedBeacon {
  id: string;
  installId: string;
  receivedOn: string;
  version: string;
  installMethod: string;
  features?: Array<[string, number]>;
}

function rows(beacons: SeedBeacon[]): FleetBeaconRow[] {
  return beacons.flatMap((beacon) => {
    const base = {
      as_of: AS_OF,
      beacon_id: beacon.id,
      install_id: beacon.installId,
      received_on: beacon.receivedOn,
      version: beacon.version,
      install_method: beacon.installMethod,
    };
    if (!beacon.features?.length) {
      return [{ ...base, feature_id: null, feature_key: null, usage_count: null }];
    }
    return beacon.features.map(([featureKey, usageCount], index) => ({
      ...base,
      feature_id: `${beacon.id}-feature-${index}`,
      feature_key: featureKey,
      usage_count: usageCount,
    }));
  });
}

describe('fleet overview aggregation', () => {
  it('computes bounded active, latest-distribution, positive-adoption, and cohort metrics', () => {
    const overview = buildFleetOverview(
      rows([
        {
          id: 'a-01',
          installId: 'install-a',
          receivedOn: '2026-05-18',
          version: '0.8.0',
          installMethod: 'cargo_install',
        },
        {
          id: 'a-02',
          installId: 'install-a',
          receivedOn: '2026-05-25',
          version: '0.8.0',
          installMethod: 'cargo_install',
        },
        {
          id: 'a-03',
          installId: 'install-a',
          receivedOn: '2026-07-16',
          version: '1.1.0',
          installMethod: 'homebrew',
          features: [
            ['alpha', 2],
            ['zero-is-not-adoption', 0],
          ],
        },
        {
          id: 'b-01',
          installId: 'install-b',
          receivedOn: '2026-06-01',
          version: '0.9.0',
          installMethod: 'scoop',
        },
        {
          id: 'b-02',
          installId: 'install-b',
          receivedOn: '2026-07-10',
          version: '1.0.0',
          installMethod: 'winget',
          features: [['beta', 4]],
        },
        {
          id: 'b-03',
          installId: 'install-b',
          receivedOn: '2026-07-10',
          version: '1.2.0',
          installMethod: 'scoop',
          features: [['alpha', 3]],
        },
        {
          id: 'c-01',
          installId: 'install-c',
          receivedOn: '2026-06-17',
          version: '1.0.0',
          installMethod: 'homebrew',
          features: [['beta', 0]],
        },
        {
          id: 'd-01',
          installId: 'install-d',
          receivedOn: '2026-06-16',
          version: '0.7.0',
          installMethod: 'unknown',
          features: [['outside-monthly-window', 99]],
        },
        {
          id: 'boundary-01',
          installId: 'install-boundary',
          receivedOn: '2026-04-18',
          version: '0.6.0',
          installMethod: 'dev_build',
        },
      ])
    );

    expect(overview).toMatchObject({
      schemaVersion: 'anvil.fleet-overview.v1',
      asOf: AS_OF,
      activeInstalls: { daily: 1, weekly: 2, monthly: 3 },
      distributions: {
        versions: [
          { value: '1.0.0', installs: 1, share: 1 / 3 },
          { value: '1.1.0', installs: 1, share: 1 / 3 },
          { value: '1.2.0', installs: 1, share: 1 / 3 },
        ],
        installMethods: [
          { value: 'homebrew', installs: 2, share: 2 / 3 },
          { value: 'scoop', installs: 1, share: 1 / 3 },
        ],
      },
      featureAdoption: [
        { featureKey: 'alpha', installs: 2, share: 2 / 3, usageCount: 5 },
        { featureKey: 'beta', installs: 1, share: 1 / 3, usageCount: 4 },
      ],
      notes: {
        activityDefinition: 'beacon observed',
        rawRetentionDays: 90,
      },
    });

    expect(overview.retentionCohorts).not.toContainEqual(
      expect.objectContaining({ cohortStart: '2026-04-13' })
    );
    expect(overview.retentionCohorts).toContainEqual({
      cohortStart: '2026-05-18',
      cohortSize: 1,
      periods: [
        { week: 0, retained: 1, share: 1 },
        { week: 1, retained: 1, share: 1 },
        { week: 2, retained: 0, share: 0 },
        { week: 3, retained: 0, share: 0 },
        { week: 4, retained: 0, share: 0 },
        { week: 5, retained: 0, share: 0 },
        { week: 6, retained: 0, share: 0 },
        { week: 7, retained: 0, share: 0 },
      ],
    });
  });

  it('returns zeroes and empty collections without non-finite shares', () => {
    const overview = buildFleetOverview([
      {
        as_of: AS_OF,
        beacon_id: null,
        install_id: null,
        received_on: null,
        version: null,
        install_method: null,
        feature_id: null,
        feature_key: null,
        usage_count: null,
      },
    ]);

    expect(overview).toEqual({
      schemaVersion: 'anvil.fleet-overview.v1',
      asOf: AS_OF,
      activeInstalls: { daily: 0, weekly: 0, monthly: 0 },
      distributions: { versions: [], installMethods: [] },
      featureAdoption: [],
      retentionCohorts: [],
      notes: { activityDefinition: 'beacon observed', rawRetentionDays: 90 },
    });
    expect(JSON.stringify(overview)).not.toMatch(/NaN|Infinity/);
  });

  it('omits a cohort until every install has a completed age week', () => {
    const overview = buildFleetOverview(
      rows([
        {
          id: 'partial-01',
          installId: 'install-partial',
          receivedOn: '2026-07-12',
          version: '1.0.0',
          installMethod: 'homebrew',
        },
      ])
    );

    expect(overview.retentionCohorts).toEqual([]);
  });
});
