/**
 * Status utility tests (TCOV-018)
 *
 * Covers: getKindlingStatus (all branches), formatKindlingStatus (all
 * output variants), formatBytes edge cases, and safeCall error recovery.
 */

import { describe, it, expect } from 'vitest';
import {
  getKindlingStatus,
  formatKindlingStatus,
  type KindlingStatusStore,
  type KindlingStatusConfig,
} from './status.js';

// =============================================================================
// Test helpers
// =============================================================================

function makeStore(
  overrides: Partial<{
    count: number;
    sizeBytes: number;
    lastTs: string | undefined;
    countError: boolean;
    sizeError: boolean;
    tsError: boolean;
  }> = {}
): KindlingStatusStore {
  return {
    countObservations: async () => {
      if (overrides.countError) throw new Error('count failed');
      return overrides.count ?? 0;
    },
    getDatabaseSizeBytes: async () => {
      if (overrides.sizeError) throw new Error('size failed');
      return overrides.sizeBytes ?? 0;
    },
    getLastObservationTimestamp: async () => {
      if (overrides.tsError) throw new Error('ts failed');
      return overrides.lastTs;
    },
  };
}

const enabledCfg: KindlingStatusConfig = { enabled: true, retention: { days: 90 } };
const disabledCfg: KindlingStatusConfig = { enabled: false };

// =============================================================================
// getKindlingStatus
// =============================================================================

describe('getKindlingStatus — disabled', () => {
  it('returns enabled: false when config says disabled', async () => {
    const status = await getKindlingStatus(disabledCfg);
    expect(status.enabled).toBe(false);
  });

  it('returns no extra fields when disabled', async () => {
    const status = await getKindlingStatus(disabledCfg);
    expect(status.observationCount).toBeUndefined();
    expect(status.dbSizeBytes).toBeUndefined();
    expect(status.retentionDays).toBeUndefined();
    expect(status.lastObservationAt).toBeUndefined();
  });

  it('ignores store even when provided while disabled', async () => {
    const store = makeStore({ count: 99 });
    const status = await getKindlingStatus(disabledCfg, store);
    expect(status.enabled).toBe(false);
    expect(status.observationCount).toBeUndefined();
  });
});

describe('getKindlingStatus — enabled, no store', () => {
  it('returns enabled: true', async () => {
    const status = await getKindlingStatus(enabledCfg);
    expect(status.enabled).toBe(true);
  });

  it('returns retentionDays from config', async () => {
    const status = await getKindlingStatus(enabledCfg);
    expect(status.retentionDays).toBe(90);
  });

  it('returns undefined retentionDays when retention not set', async () => {
    const status = await getKindlingStatus({ enabled: true });
    expect(status.retentionDays).toBeUndefined();
  });

  it('has no observationCount or dbSizeBytes', async () => {
    const status = await getKindlingStatus(enabledCfg);
    expect(status.observationCount).toBeUndefined();
    expect(status.dbSizeBytes).toBeUndefined();
  });
});

describe('getKindlingStatus — enabled with store', () => {
  it('returns observationCount from store', async () => {
    const store = makeStore({ count: 42 });
    const status = await getKindlingStatus(enabledCfg, store);
    expect(status.observationCount).toBe(42);
  });

  it('returns dbSizeBytes from store', async () => {
    const store = makeStore({ sizeBytes: 81920 });
    const status = await getKindlingStatus(enabledCfg, store);
    expect(status.dbSizeBytes).toBe(81920);
  });

  it('returns lastObservationAt from store', async () => {
    const ts = '2026-02-15T10:00:00.000Z';
    const store = makeStore({ lastTs: ts });
    const status = await getKindlingStatus(enabledCfg, store);
    expect(status.lastObservationAt).toBe(ts);
  });

  it('returns undefined lastObservationAt when store returns none', async () => {
    const store = makeStore({ lastTs: undefined });
    const status = await getKindlingStatus(enabledCfg, store);
    expect(status.lastObservationAt).toBeUndefined();
  });

  it('includes retentionDays from config', async () => {
    const store = makeStore({ count: 5 });
    const status = await getKindlingStatus({ enabled: true, retention: { days: 30 } }, store);
    expect(status.retentionDays).toBe(30);
  });
});

describe('getKindlingStatus — safeCall fallbacks', () => {
  it('falls back to 0 observationCount when store.countObservations throws', async () => {
    const store = makeStore({ countError: true });
    const status = await getKindlingStatus(enabledCfg, store);
    expect(status.observationCount).toBe(0);
  });

  it('falls back to 0 dbSizeBytes when store.getDatabaseSizeBytes throws', async () => {
    const store = makeStore({ sizeError: true });
    const status = await getKindlingStatus(enabledCfg, store);
    expect(status.dbSizeBytes).toBe(0);
  });

  it('falls back to undefined lastObservationAt when store.getLastObservationTimestamp throws', async () => {
    const store = makeStore({ tsError: true });
    const status = await getKindlingStatus(enabledCfg, store);
    expect(status.lastObservationAt).toBeUndefined();
  });

  it('never throws even when all store methods throw', async () => {
    const store = makeStore({ countError: true, sizeError: true, tsError: true });
    await expect(getKindlingStatus(enabledCfg, store)).resolves.toBeDefined();
  });
});

// =============================================================================
// formatKindlingStatus
// =============================================================================

describe('formatKindlingStatus', () => {
  it('returns single "disabled" line when disabled', () => {
    const output = formatKindlingStatus({ enabled: false });
    expect(output).toBe('Kindling: disabled');
  });

  it('shows "enabled" when status.enabled is true', () => {
    const output = formatKindlingStatus({ enabled: true });
    expect(output).toContain('Kindling: enabled');
  });

  it('includes observation count when present', () => {
    const output = formatKindlingStatus({ enabled: true, observationCount: 42 });
    expect(output).toContain('Observations: 42');
  });

  it('includes database size when present', () => {
    const output = formatKindlingStatus({ enabled: true, dbSizeBytes: 1024 });
    expect(output).toContain('Database size:');
    expect(output).toContain('KB');
  });

  it('includes retention days when present', () => {
    const output = formatKindlingStatus({ enabled: true, retentionDays: 90 });
    expect(output).toContain('Retention: 90 days');
  });

  it('includes lastObservationAt when present', () => {
    const ts = '2026-02-15T10:00:00.000Z';
    const output = formatKindlingStatus({ enabled: true, lastObservationAt: ts });
    expect(output).toContain(`Last observation: ${ts}`);
  });

  it('shows "none" for last observation when count is 0 and no timestamp', () => {
    const output = formatKindlingStatus({ enabled: true, observationCount: 0 });
    expect(output).toContain('Last observation: none');
  });

  it('does not show "none" when observationCount > 0 but no timestamp', () => {
    const output = formatKindlingStatus({ enabled: true, observationCount: 5 });
    expect(output).not.toContain('Last observation: none');
    expect(output).not.toContain('Last observation:');
  });

  it('omits observation count line when undefined', () => {
    const output = formatKindlingStatus({ enabled: true });
    expect(output).not.toContain('Observations:');
  });

  it('omits db size line when undefined', () => {
    const output = formatKindlingStatus({ enabled: true });
    expect(output).not.toContain('Database size:');
  });

  it('omits retention line when undefined', () => {
    const output = formatKindlingStatus({ enabled: true });
    expect(output).not.toContain('Retention:');
  });

  it('formats 0 bytes as "0 B"', () => {
    const output = formatKindlingStatus({ enabled: true, dbSizeBytes: 0 });
    expect(output).toContain('Database size: 0 B');
  });

  it('formats bytes in KB range', () => {
    // 2048 bytes = 2.0 KB (value 2 < 10, so toFixed(1))
    const output = formatKindlingStatus({ enabled: true, dbSizeBytes: 2048 });
    expect(output).toContain('2.0 KB');
  });

  it('formats bytes in MB range', () => {
    // 1MB = 1.0 MB (value 1 < 10, so toFixed(1))
    const output = formatKindlingStatus({ enabled: true, dbSizeBytes: 1024 * 1024 });
    expect(output).toContain('1.0 MB');
  });

  it('formats bytes in GB range', () => {
    // 1GB = 1.0 GB (value 1 < 10, so toFixed(1))
    const output = formatKindlingStatus({ enabled: true, dbSizeBytes: 1024 * 1024 * 1024 });
    expect(output).toContain('1.0 GB');
  });

  it('formats fractional bytes with decimal (< 10 in unit)', () => {
    // 1536 bytes = 1.5 KB — value 1.5 < 10, toFixed(1)
    const output = formatKindlingStatus({ enabled: true, dbSizeBytes: 1536 });
    expect(output).toContain('1.5 KB');
  });

  it('formats large bytes with rounding (>= 10 in unit)', () => {
    // 10240 bytes = 10 KB — value 10 >= 10, Math.round
    const output = formatKindlingStatus({ enabled: true, dbSizeBytes: 10 * 1024 });
    expect(output).toContain('10 KB');
  });

  it('does not include disabled section fields when enabled is true', () => {
    const output = formatKindlingStatus({ enabled: true });
    // disabled path returns early; enabled path continues
    expect(output.split('\n')[0]).toBe('Kindling: enabled');
  });
});
