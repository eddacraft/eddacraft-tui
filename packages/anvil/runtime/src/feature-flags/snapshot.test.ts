import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

import type { FeatureFlagDefinition, FeatureFlagManifest } from '@eddacraft/anvil-contracts';
import { FEATURE_FLAG_SCHEMA_VERSION } from '@eddacraft/anvil-contracts';

import { createSnapshot, loadSnapshot, isSnapshotFresh, SnapshotLoadError } from './snapshot.js';
import type { FeatureFlagSnapshot, SnapshotConfig } from './snapshot.js';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

function validManifest(): FeatureFlagManifest {
  return {
    schemaVersion: FEATURE_FLAG_SCHEMA_VERSION,
    flags: [
      {
        key: 'cli.licence-gate',
        owner: 'BAUTH',
        intent: 'Gate CLI features behind licence validation',
        class: 'entitlement',
        valueType: 'boolean',
        variants: [
          { key: 'enabled', value: true },
          { key: 'disabled', value: false },
        ],
        defaultVariant: 'disabled',
        status: 'active',
        createdFor: 'FLAGS-008',
      },
    ],
  } as FeatureFlagManifest;
}

function defaultConfig(): SnapshotConfig {
  return {
    maxAgeSec: 300,
  };
}

// ---------------------------------------------------------------------------
// createSnapshot
// ---------------------------------------------------------------------------

describe('createSnapshot', () => {
  it('creates a snapshot from a manifest', () => {
    const manifest = validManifest();
    const snapshot = createSnapshot(manifest);

    expect(snapshot.schemaVersion).toBe(FEATURE_FLAG_SCHEMA_VERSION);
    expect(snapshot.snapshotVersion).toBeGreaterThan(0);
    expect(snapshot.issuedAt).toBeDefined();
    expect(snapshot.flags).toEqual(manifest.flags);
  });

  it('generates monotonically increasing snapshot versions', () => {
    const manifest = validManifest();
    const a = createSnapshot(manifest);
    const b = createSnapshot(manifest);
    expect(b.snapshotVersion).toBeGreaterThan(a.snapshotVersion);
  });

  it('sets issuedAt to current time (second precision)', () => {
    const beforeSec = Math.floor(Date.now() / 1000);
    const snapshot = createSnapshot(validManifest());
    const afterSec = Math.floor(Date.now() / 1000);
    const tsSec = Math.floor(new Date(snapshot.issuedAt).getTime() / 1000);
    expect(tsSec).toBeGreaterThanOrEqual(beforeSec);
    expect(tsSec).toBeLessThanOrEqual(afterSec);
    // Verify no milliseconds in the ISO string
    expect(snapshot.issuedAt).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
  });

  it('includes all flags from the manifest', () => {
    const manifest = validManifest();
    manifest.flags.push({
      key: 'docs.access',
      owner: 'DOCSAUTH',
      intent: 'Gate docs access',
      class: 'entitlement',
      valueType: 'boolean',
      variants: [
        { key: 'enabled', value: true },
        { key: 'disabled', value: false },
      ],
      defaultVariant: 'disabled',
      status: 'active',
      createdFor: 'FLAGS-008',
    } as FeatureFlagDefinition);
    const snapshot = createSnapshot(manifest);
    expect(snapshot.flags).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// loadSnapshot
// ---------------------------------------------------------------------------

describe('loadSnapshot', () => {
  it('parses valid snapshot JSON', () => {
    const snapshot = createSnapshot(validManifest());
    const json = JSON.stringify(snapshot);
    const loaded = loadSnapshot(json);
    expect(loaded.schemaVersion).toBe(FEATURE_FLAG_SCHEMA_VERSION);
    expect(loaded.flags).toEqual(snapshot.flags);
  });

  it('throws SnapshotLoadError for invalid JSON', () => {
    expect(() => loadSnapshot('not json')).toThrow(SnapshotLoadError);
  });

  it('throws SnapshotLoadError for wrong schema version', () => {
    const snapshot = createSnapshot(validManifest());
    const obj = { ...snapshot, schemaVersion: 99 };
    expect(() => loadSnapshot(JSON.stringify(obj))).toThrow(SnapshotLoadError);
  });

  it('throws SnapshotLoadError for missing fields', () => {
    expect(() => loadSnapshot(JSON.stringify({ schemaVersion: 1 }))).toThrow(SnapshotLoadError);
  });

  // C-005: snapshotVersion validation
  it('throws for snapshotVersion=0', () => {
    const snapshot = createSnapshot(validManifest());
    const obj = JSON.parse(JSON.stringify(snapshot));
    obj.snapshotVersion = 0;
    expect(() => loadSnapshot(JSON.stringify(obj))).toThrow('positive integer');
  });

  it('throws for negative snapshotVersion', () => {
    const snapshot = createSnapshot(validManifest());
    const obj = JSON.parse(JSON.stringify(snapshot));
    obj.snapshotVersion = -5;
    expect(() => loadSnapshot(JSON.stringify(obj))).toThrow('positive integer');
  });

  it('throws for fractional snapshotVersion', () => {
    const snapshot = createSnapshot(validManifest());
    const obj = JSON.parse(JSON.stringify(snapshot));
    obj.snapshotVersion = 1.5;
    expect(() => loadSnapshot(JSON.stringify(obj))).toThrow('positive integer');
  });

  // C-006: issuedAt validation
  it('throws for invalid issuedAt string', () => {
    const snapshot = createSnapshot(validManifest());
    const obj = JSON.parse(JSON.stringify(snapshot));
    obj.issuedAt = 'not-a-date';
    expect(() => loadSnapshot(JSON.stringify(obj))).toThrow('valid timestamp');
  });

  // C-004: flag definition validation
  it('throws for null flag entry', () => {
    const snapshot = createSnapshot(validManifest());
    const obj = JSON.parse(JSON.stringify(snapshot));
    obj.flags = [null];
    expect(() => loadSnapshot(JSON.stringify(obj))).toThrow('not an object');
  });

  it('throws for flag missing required fields', () => {
    const snapshot = createSnapshot(validManifest());
    const obj = JSON.parse(JSON.stringify(snapshot));
    obj.flags = [{ key: 'test' }];
    expect(() => loadSnapshot(JSON.stringify(obj))).toThrow('missing required fields');
  });
});

// ---------------------------------------------------------------------------
// isSnapshotFresh
// ---------------------------------------------------------------------------

describe('isSnapshotFresh', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns true for a just-created snapshot', () => {
    const snapshot = createSnapshot(validManifest());
    expect(isSnapshotFresh(snapshot, defaultConfig())).toBe(true);
  });

  it('returns false when snapshot exceeds maxAgeSec', () => {
    vi.setSystemTime(new Date('2026-04-12T00:00:00Z'));
    const snapshot = createSnapshot(validManifest());
    // Advance 301 seconds past the 300s max age
    vi.setSystemTime(new Date('2026-04-12T00:05:01Z'));
    expect(isSnapshotFresh(snapshot, defaultConfig())).toBe(false);
  });

  it('returns true at exactly maxAgeSec boundary', () => {
    vi.setSystemTime(new Date('2026-04-12T00:00:00Z'));
    const snapshot = createSnapshot(validManifest());
    // Advance exactly 300 seconds
    vi.setSystemTime(new Date('2026-04-12T00:05:00Z'));
    expect(isSnapshotFresh(snapshot, defaultConfig())).toBe(true);
  });

  it('respects custom maxAgeSec', () => {
    vi.setSystemTime(new Date('2026-04-12T00:00:00Z'));
    const snapshot = createSnapshot(validManifest());
    vi.setSystemTime(new Date('2026-04-12T00:00:11Z'));
    expect(isSnapshotFresh(snapshot, { maxAgeSec: 10 })).toBe(false);
    expect(isSnapshotFresh(snapshot, { maxAgeSec: 60 })).toBe(true);
  });

  // C-007: forward-skew tolerance
  it('treats snapshot within 60s clock skew as fresh', () => {
    vi.setSystemTime(new Date('2026-04-12T00:00:00Z'));
    // issuedAt is 30 seconds in the future (within tolerance)
    const snapshot: FeatureFlagSnapshot = {
      schemaVersion: FEATURE_FLAG_SCHEMA_VERSION,
      snapshotVersion: 1,
      issuedAt: '2026-04-12T00:00:30Z',
      flags: [],
    };
    expect(isSnapshotFresh(snapshot, defaultConfig())).toBe(true);
  });

  it('rejects snapshot issued far in the future', () => {
    vi.setSystemTime(new Date('2026-04-12T00:00:00Z'));
    // issuedAt is 10 minutes in the future (beyond 60s tolerance)
    const snapshot: FeatureFlagSnapshot = {
      schemaVersion: FEATURE_FLAG_SCHEMA_VERSION,
      snapshotVersion: 1,
      issuedAt: '2026-04-12T00:10:00Z',
      flags: [],
    };
    expect(isSnapshotFresh(snapshot, defaultConfig())).toBe(false);
  });
});
