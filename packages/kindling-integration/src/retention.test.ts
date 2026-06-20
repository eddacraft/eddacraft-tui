/**
 * Retention management tests (TCOV-018)
 *
 * Covers: isRetentionCapable type guard, pruneOldObservations,
 * getStorageStats, and PruneResult shapes.
 */

import { describe, it, expect } from 'vitest';
import {
  isRetentionCapable,
  pruneOldObservations,
  getStorageStats,
  type IRetentionCapableStore,
  type StorageStats,
} from './retention.js';
import { KindlingConfigSchema } from './config.js';

// =============================================================================
// Test helpers
// =============================================================================

const enabledConfig = KindlingConfigSchema.parse({ enabled: true, retention: { days: 30 } });
const disabledConfig = KindlingConfigSchema.parse({ enabled: false });

/** A minimal store that supports retention operations */
function makeRetentionStore(
  opts: { deletedCount?: number; stats?: StorageStats } = {}
): IRetentionCapableStore {
  return {
    deleteObservationsOlderThan: async (_olderThan: string) => opts.deletedCount ?? 0,
    getStats: async () => opts.stats ?? { observation_count: 42, estimated_size_bytes: 81920 },
  };
}

// =============================================================================
// isRetentionCapable
// =============================================================================

describe('isRetentionCapable', () => {
  it('returns true for a properly implemented retention store', () => {
    expect(isRetentionCapable(makeRetentionStore())).toBe(true);
  });

  it('returns false for null', () => {
    expect(isRetentionCapable(null)).toBe(false);
  });

  it('returns false for a plain string', () => {
    expect(isRetentionCapable('not a store')).toBe(false);
  });

  it('returns false for an empty object', () => {
    expect(isRetentionCapable({})).toBe(false);
  });

  it('returns false when only deleteObservationsOlderThan is present', () => {
    expect(isRetentionCapable({ deleteObservationsOlderThan: async () => 0 })).toBe(false);
  });

  it('returns false when only getStats is present', () => {
    expect(isRetentionCapable({ getStats: async () => ({}) })).toBe(false);
  });

  it('returns false when methods are non-functions', () => {
    expect(
      isRetentionCapable({
        deleteObservationsOlderThan: 'not-a-function',
        getStats: 42,
      })
    ).toBe(false);
  });

  it('returns false for undefined', () => {
    expect(isRetentionCapable(undefined)).toBe(false);
  });

  it('returns false for a number', () => {
    expect(isRetentionCapable(42)).toBe(false);
  });
});

// =============================================================================
// pruneOldObservations
// =============================================================================

describe('pruneOldObservations', () => {
  it('skips pruning when Kindling is disabled', async () => {
    const store = makeRetentionStore({ deletedCount: 99 });
    const result = await pruneOldObservations(store, disabledConfig);
    expect(result.pruned).toBe(false);
    expect(result.deleted_count).toBe(0);
    expect(result.skip_reason).toMatch(/disabled/i);
  });

  it('skips pruning when store does not support retention', async () => {
    const result = await pruneOldObservations({}, enabledConfig);
    expect(result.pruned).toBe(false);
    expect(result.deleted_count).toBe(0);
    expect(result.skip_reason).toMatch(/retention/i);
  });

  it('prunes and returns deleted count', async () => {
    const store = makeRetentionStore({ deletedCount: 17 });
    const result = await pruneOldObservations(store, enabledConfig);
    expect(result.pruned).toBe(true);
    expect(result.deleted_count).toBe(17);
    expect(result.skip_reason).toBeUndefined();
  });

  it('returns zero deleted count when nothing is old enough', async () => {
    const store = makeRetentionStore({ deletedCount: 0 });
    const result = await pruneOldObservations(store, enabledConfig);
    expect(result.pruned).toBe(true);
    expect(result.deleted_count).toBe(0);
  });

  it('cutoff_date is an ISO8601 string in the past', async () => {
    const store = makeRetentionStore({ deletedCount: 5 });
    const before = new Date();
    const result = await pruneOldObservations(store, enabledConfig);
    const cutoff = new Date(result.cutoff_date);
    expect(cutoff.getTime()).toBeLessThan(before.getTime());
  });

  it('cutoff_date reflects the configured retention.days', async () => {
    const config = KindlingConfigSchema.parse({ enabled: true, retention: { days: 7 } });
    const store = makeRetentionStore({ deletedCount: 0 });
    // Bracket the call: the cutoff must fall within the 7-days-ago window
    // measured immediately before and after the prune. This is independent of
    // how long the async call takes, so it won't flake on slow CI.
    const before = new Date();
    before.setDate(before.getDate() - 7);
    const result = await pruneOldObservations(store, config);
    const after = new Date();
    after.setDate(after.getDate() - 7);
    const cutoff = new Date(result.cutoff_date).getTime();
    expect(cutoff).toBeGreaterThanOrEqual(before.getTime());
    expect(cutoff).toBeLessThanOrEqual(after.getTime());
  });

  it('returns empty cutoff_date string when disabled', async () => {
    const result = await pruneOldObservations({}, disabledConfig);
    expect(result.cutoff_date).toBe('');
  });

  it('passes the cutoff ISO string to the store method', async () => {
    const calls: string[] = [];
    const store: IRetentionCapableStore = {
      deleteObservationsOlderThan: async (olderThan) => {
        calls.push(olderThan);
        return 0;
      },
      getStats: async () => ({ observation_count: 0, estimated_size_bytes: 0 }),
    };
    await pruneOldObservations(store, enabledConfig);
    expect(calls).toHaveLength(1);
    // Should be a valid ISO date
    expect(() => new Date(calls[0]).toISOString()).not.toThrow();
  });
});

// =============================================================================
// getStorageStats
// =============================================================================

describe('getStorageStats', () => {
  it('returns zero stats when store does not support retention', async () => {
    const result = await getStorageStats({});
    expect(result.observation_count).toBe(0);
    expect(result.estimated_size_bytes).toBe(0);
  });

  it('returns zero stats for null', async () => {
    const result = await getStorageStats(null);
    expect(result.observation_count).toBe(0);
    expect(result.estimated_size_bytes).toBe(0);
  });

  it('delegates to store.getStats() for a retention-capable store', async () => {
    const store = makeRetentionStore({
      stats: { observation_count: 100, estimated_size_bytes: 204800 },
    });
    const result = await getStorageStats(store);
    expect(result.observation_count).toBe(100);
    expect(result.estimated_size_bytes).toBe(204800);
  });

  it('returns actual stats from the store', async () => {
    const store = makeRetentionStore({
      stats: { observation_count: 1, estimated_size_bytes: 512 },
    });
    const result = await getStorageStats(store);
    expect(result.observation_count).toBe(1);
    expect(result.estimated_size_bytes).toBe(512);
  });
});
