import type { FeatureFlagDefinition, FeatureFlagManifest } from '@eddacraft/anvil-contracts';
import { FEATURE_FLAG_SCHEMA_VERSION } from '@eddacraft/anvil-contracts';

// =============================================================================
// Snapshot Shape
// =============================================================================

export interface FeatureFlagSnapshot {
  schemaVersion: number;
  snapshotVersion: number;
  issuedAt: string;
  flags: FeatureFlagDefinition[];
}

export interface SnapshotConfig {
  maxAgeSec: number;
}

// =============================================================================
// Snapshot Creation
// =============================================================================

let versionCounter = 0;

// Strip milliseconds for cross-runtime parity with Rust (second precision)
function toSecondPrecisionIso(date: Date): string {
  return date.toISOString().replace(/\.\d{3}Z$/, 'Z');
}

export function createSnapshot(manifest: FeatureFlagManifest): FeatureFlagSnapshot {
  versionCounter += 1;
  return {
    schemaVersion: manifest.schemaVersion,
    snapshotVersion: versionCounter,
    issuedAt: toSecondPrecisionIso(new Date()),
    flags: manifest.flags,
  };
}

// =============================================================================
// Snapshot Loading
// =============================================================================

export class SnapshotLoadError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SnapshotLoadError';
  }
}

export function loadSnapshot(json: string): FeatureFlagSnapshot {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    throw new SnapshotLoadError('Invalid JSON');
  }

  if (typeof parsed !== 'object' || parsed === null) {
    throw new SnapshotLoadError('Snapshot must be an object');
  }

  const obj = parsed as Record<string, unknown>;

  if (obj.schemaVersion !== FEATURE_FLAG_SCHEMA_VERSION) {
    throw new SnapshotLoadError(
      `Unsupported schema version: ${obj.schemaVersion} (expected ${FEATURE_FLAG_SCHEMA_VERSION})`
    );
  }

  if (
    typeof obj.snapshotVersion !== 'number' ||
    typeof obj.issuedAt !== 'string' ||
    !Array.isArray(obj.flags)
  ) {
    throw new SnapshotLoadError('Missing required snapshot fields');
  }

  // C-005: validate snapshotVersion is a positive integer
  if (!Number.isInteger(obj.snapshotVersion) || (obj.snapshotVersion as number) < 1) {
    throw new SnapshotLoadError('snapshotVersion must be a positive integer');
  }

  // C-006: validate issuedAt is a parseable timestamp
  if (isNaN(new Date(obj.issuedAt as string).getTime())) {
    throw new SnapshotLoadError('issuedAt is not a valid timestamp');
  }

  // C-004: validate flag array elements have required fields
  const flags = obj.flags as unknown[];
  for (let i = 0; i < flags.length; i++) {
    const flag = flags[i];
    if (typeof flag !== 'object' || flag === null) {
      throw new SnapshotLoadError(`flags[${i}] is not an object`);
    }
    const f = flag as Record<string, unknown>;
    if (
      typeof f.key !== 'string' ||
      typeof f.owner !== 'string' ||
      typeof f.defaultVariant !== 'string' ||
      typeof f.status !== 'string' ||
      typeof f.class !== 'string'
    ) {
      throw new SnapshotLoadError(`flags[${i}] is missing required fields`);
    }
  }

  return obj as unknown as FeatureFlagSnapshot;
}

// =============================================================================
// Freshness Check
// =============================================================================

// C-007: forward-skew tolerance — reject snapshots issued more than this
// many seconds in the future (clock skew protection)
const CLOCK_SKEW_TOLERANCE_SEC = 60;

export function isSnapshotFresh(snapshot: FeatureFlagSnapshot, config: SnapshotConfig): boolean {
  const issuedMs = new Date(snapshot.issuedAt).getTime();
  const nowMs = Date.now();
  const ageSec = (nowMs - issuedMs) / 1000;
  // Reject snapshots issued too far in the future
  if (ageSec < -CLOCK_SKEW_TOLERANCE_SEC) {
    return false;
  }
  return ageSec <= config.maxAgeSec;
}
