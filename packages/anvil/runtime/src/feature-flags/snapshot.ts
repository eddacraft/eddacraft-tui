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

// C-016: timestamp-based version with sub-second counter for monotonicity
// within a single process. Survives restarts because the epoch-second base
// advances with wall-clock time.
let lastEpochSec = 0;
let subSecondCounter = 0;

// Strip milliseconds for cross-runtime parity with Rust (second precision)
function toSecondPrecisionIso(date: Date): string {
  return date.toISOString().replace(/\.\d{3}Z$/, 'Z');
}

export function createSnapshot(manifest: FeatureFlagManifest): FeatureFlagSnapshot {
  const epochSec = Math.floor(Date.now() / 1000);
  if (epochSec === lastEpochSec) {
    subSecondCounter += 1;
  } else {
    lastEpochSec = epochSec;
    subSecondCounter = 0;
  }
  // Encode as epochSec * 1000 + sub-second counter to keep a single integer
  const snapshotVersion = epochSec * 1000 + subSecondCounter;
  return {
    schemaVersion: manifest.schemaVersion,
    snapshotVersion,
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
  const KNOWN_VALUE_TYPES = ['boolean', 'string', 'number', 'object'] as const;
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
    // C-017: validate valueType is a known string
    if (!(KNOWN_VALUE_TYPES as readonly string[]).includes(f.valueType as string)) {
      throw new SnapshotLoadError(`flags[${i}] has invalid valueType: ${String(f.valueType)}`);
    }
    // C-017: validate variants array exists and is non-empty
    if (!Array.isArray(f.variants) || f.variants.length === 0) {
      throw new SnapshotLoadError(`flags[${i}] must have a non-empty variants array`);
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
