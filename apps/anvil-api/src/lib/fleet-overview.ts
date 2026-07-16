import type { NeonClient } from '../db/client.js';

export const FLEET_OVERVIEW_SCHEMA_VERSION = 'anvil.fleet-overview.v1';
export const FLEET_RAW_RETENTION_DAYS = 90;

export interface FleetBeaconRow {
  as_of: string;
  beacon_id: string | null;
  install_id: string | null;
  received_on: string | null;
  version: string | null;
  install_method: string | null;
  feature_id: string | null;
  feature_key: string | null;
  usage_count: number | string | null;
}

export interface FleetDistributionEntry {
  value: string;
  installs: number;
  share: number;
}

export interface FleetFeatureAdoptionEntry {
  featureKey: string;
  installs: number;
  share: number;
  usageCount: number;
}

export interface FleetRetentionPeriod {
  week: number;
  retained: number;
  share: number;
}

export interface FleetRetentionCohort {
  cohortStart: string;
  cohortSize: number;
  periods: FleetRetentionPeriod[];
}

export interface FleetOverview {
  schemaVersion: typeof FLEET_OVERVIEW_SCHEMA_VERSION;
  asOf: string;
  activeInstalls: {
    daily: number;
    weekly: number;
    monthly: number;
  };
  distributions: {
    versions: FleetDistributionEntry[];
    installMethods: FleetDistributionEntry[];
  };
  featureAdoption: FleetFeatureAdoptionEntry[];
  retentionCohorts: FleetRetentionCohort[];
  notes: {
    activityDefinition: 'beacon observed';
    rawRetentionDays: typeof FLEET_RAW_RETENTION_DAYS;
  };
}

interface FeatureObservation {
  id: string;
  key: string;
  usageCount: number;
}

interface Beacon {
  id: string;
  installId: string;
  receivedDay: number;
  version: string;
  installMethod: string;
  features: Map<string, FeatureObservation>;
}

const DAY_MILLISECONDS = 24 * 60 * 60 * 1000;

function parseDay(value: string): number {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) throw new Error(`invalid fleet date: ${value}`);
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const timestamp = Date.UTC(year, month - 1, day);
  const parsed = new Date(timestamp);
  if (
    parsed.getUTCFullYear() !== year ||
    parsed.getUTCMonth() !== month - 1 ||
    parsed.getUTCDate() !== day
  ) {
    throw new Error(`invalid fleet date: ${value}`);
  }
  return Math.floor(timestamp / DAY_MILLISECONDS);
}

function formatDay(day: number): string {
  return new Date(day * DAY_MILLISECONDS).toISOString().slice(0, 10);
}

function mondayOf(day: number): number {
  const weekday = new Date(day * DAY_MILLISECONDS).getUTCDay();
  return day - ((weekday + 6) % 7);
}

function compareText(left: string, right: string): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

function share(numerator: number, denominator: number): number {
  if (denominator === 0) return 0;
  return numerator / denominator;
}

function lastCompletedAgeWeek(
  members: Set<string>,
  firstSeen: Map<string, number>,
  asOfDay: number
): number {
  let latestFirstSeen = Number.NEGATIVE_INFINITY;
  for (const installId of members) {
    latestFirstSeen = Math.max(latestFirstSeen, firstSeen.get(installId)!);
  }
  return Math.floor((asOfDay - latestFirstSeen + 1) / 7) - 1;
}

function distribution(
  latest: Iterable<Beacon>,
  valueOf: (beacon: Beacon) => string,
  monthlyActive: number
): FleetDistributionEntry[] {
  const counts = new Map<string, number>();
  for (const beacon of latest) {
    const value = valueOf(beacon);
    counts.set(value, (counts.get(value) ?? 0) + 1);
  }
  return [...counts]
    .map(([value, installs]) => ({ value, installs, share: share(installs, monthlyActive) }))
    .sort((left, right) => right.installs - left.installs || compareText(left.value, right.value));
}

/**
 * Build the stable v1 fleet contract from retained, date-coarsened raw rows.
 *
 * The query supplies Postgres `current_date` as `as_of`; this function has no
 * wall-clock dependency, which keeps boundary behaviour deterministic in tests.
 */
export function buildFleetOverview(rows: FleetBeaconRow[]): FleetOverview {
  const asOf = rows[0]?.as_of;
  if (!asOf) throw new Error('fleet overview query did not return as_of');
  const asOfDay = parseDay(asOf);
  const rawBoundaryDay = asOfDay - (FLEET_RAW_RETENTION_DAYS - 1);

  const beacons = new Map<string, Beacon>();
  for (const row of rows) {
    if (
      !row.beacon_id ||
      !row.install_id ||
      !row.received_on ||
      !row.version ||
      !row.install_method
    ) {
      continue;
    }
    const receivedDay = parseDay(row.received_on);
    if (receivedDay < rawBoundaryDay || receivedDay > asOfDay) continue;

    let beacon = beacons.get(row.beacon_id);
    if (!beacon) {
      beacon = {
        id: row.beacon_id,
        installId: row.install_id,
        receivedDay,
        version: row.version,
        installMethod: row.install_method,
        features: new Map(),
      };
      beacons.set(row.beacon_id, beacon);
    }
    if (row.feature_id && row.feature_key && row.usage_count !== null) {
      const usageCount = Number(row.usage_count);
      if (!Number.isSafeInteger(usageCount)) {
        throw new Error(`invalid fleet usage count for feature ${row.feature_key}`);
      }
      beacon.features.set(row.feature_id, {
        id: row.feature_id,
        key: row.feature_key,
        usageCount,
      });
    }
  }

  const installsByWindow = {
    daily: new Set<string>(),
    weekly: new Set<string>(),
    monthly: new Set<string>(),
  };
  const monthlyLatest = new Map<string, Beacon>();
  const firstSeen = new Map<string, number>();
  const installBeacons = new Map<string, Beacon[]>();

  for (const beacon of beacons.values()) {
    const ageDays = asOfDay - beacon.receivedDay;
    if (ageDays === 0) installsByWindow.daily.add(beacon.installId);
    if (ageDays <= 6) installsByWindow.weekly.add(beacon.installId);
    if (ageDays <= 29) {
      installsByWindow.monthly.add(beacon.installId);
      const previous = monthlyLatest.get(beacon.installId);
      if (
        !previous ||
        beacon.receivedDay > previous.receivedDay ||
        (beacon.receivedDay === previous.receivedDay && compareText(beacon.id, previous.id) > 0)
      ) {
        monthlyLatest.set(beacon.installId, beacon);
      }
    }
    firstSeen.set(
      beacon.installId,
      Math.min(firstSeen.get(beacon.installId) ?? beacon.receivedDay, beacon.receivedDay)
    );
    const observed = installBeacons.get(beacon.installId) ?? [];
    observed.push(beacon);
    installBeacons.set(beacon.installId, observed);
  }

  const monthlyActive = installsByWindow.monthly.size;
  const adopters = new Map<string, Set<string>>();
  const usageCounts = new Map<string, number>();
  for (const beacon of beacons.values()) {
    if (asOfDay - beacon.receivedDay > 29) continue;
    for (const feature of beacon.features.values()) {
      if (feature.usageCount <= 0) continue;
      const featureAdopters = adopters.get(feature.key) ?? new Set<string>();
      featureAdopters.add(beacon.installId);
      adopters.set(feature.key, featureAdopters);
      const total = (usageCounts.get(feature.key) ?? 0) + feature.usageCount;
      if (!Number.isSafeInteger(total)) {
        throw new Error(`fleet usage count exceeds the safe integer range for ${feature.key}`);
      }
      usageCounts.set(feature.key, total);
    }
  }
  const featureAdoption = [...adopters]
    .map(([featureKey, featureInstalls]) => ({
      featureKey,
      installs: featureInstalls.size,
      share: share(featureInstalls.size, monthlyActive),
      usageCount: usageCounts.get(featureKey) ?? 0,
    }))
    .sort(
      (left, right) =>
        right.installs - left.installs || compareText(left.featureKey, right.featureKey)
    );

  const cohorts = new Map<number, Set<string>>();
  for (const [installId, firstSeenDay] of firstSeen) {
    const cohortStart = mondayOf(firstSeenDay);
    const members = cohorts.get(cohortStart) ?? new Set<string>();
    members.add(installId);
    cohorts.set(cohortStart, members);
  }
  const boundaryCohort = mondayOf(rawBoundaryDay);
  const retentionCohorts = [...cohorts]
    .filter(([cohortStart]) => {
      const members = cohorts.get(cohortStart)!;
      return (
        cohortStart !== boundaryCohort && lastCompletedAgeWeek(members, firstSeen, asOfDay) >= 0
      );
    })
    .sort(([left], [right]) => right - left)
    .slice(0, 8)
    .map(([cohortStart, members]) => {
      const completedAgeWeek = Math.min(8, lastCompletedAgeWeek(members, firstSeen, asOfDay));
      const periods = Array.from({ length: completedAgeWeek + 1 }, (_, week) => {
        let retained = 0;
        for (const installId of members) {
          const installFirstSeen = firstSeen.get(installId)!;
          const observed = (installBeacons.get(installId) ?? []).some(
            (beacon) => Math.floor((beacon.receivedDay - installFirstSeen) / 7) === week
          );
          if (observed) retained++;
        }
        return { week, retained, share: share(retained, members.size) };
      });
      return {
        cohortStart: formatDay(cohortStart),
        cohortSize: members.size,
        periods,
      };
    });

  return {
    schemaVersion: FLEET_OVERVIEW_SCHEMA_VERSION,
    asOf,
    activeInstalls: {
      daily: installsByWindow.daily.size,
      weekly: installsByWindow.weekly.size,
      monthly: monthlyActive,
    },
    distributions: {
      versions: distribution(monthlyLatest.values(), (beacon) => beacon.version, monthlyActive),
      installMethods: distribution(
        monthlyLatest.values(),
        (beacon) => beacon.installMethod,
        monthlyActive
      ),
    },
    featureAdoption,
    retentionCohorts,
    notes: {
      activityDefinition: 'beacon observed',
      rawRetentionDays: FLEET_RAW_RETENTION_DAYS,
    },
  };
}

/** Query the retained raw identity-bearing rows required by the v1 snapshot. */
export async function findFleetOverview(sql: NeonClient): Promise<FleetOverview> {
  const rows = await sql`
    SELECT
      current_date::text AS as_of,
      b.id::text AS beacon_id,
      b.install_id::text AS install_id,
      b.received_on::text AS received_on,
      b.version,
      b.install_method,
      f.id::text AS feature_id,
      f.feature_key,
      f.usage_count
    FROM (SELECT current_date AS as_of) AS clock
    LEFT JOIN telemetry_beacons AS b
      ON b.received_on BETWEEN
        clock.as_of - (${FLEET_RAW_RETENTION_DAYS}::int - 1)
        AND clock.as_of
    LEFT JOIN telemetry_beacon_features AS f ON f.beacon_id = b.id
    ORDER BY b.received_on ASC, b.id ASC, f.feature_key ASC, f.id ASC
  `;
  return buildFleetOverview(rows as unknown as FleetBeaconRow[]);
}
