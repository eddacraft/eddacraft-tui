import { describe, expect, it } from 'vitest';
import {
  ACCOUNT_ACTIVITY_ROLLUP_SCHEMA_VERSION,
  DEFAULT_ROLLUP_LOOKBACK_DAYS,
  ROLLUP_TOTAL_PLAN_KEY,
  completedUtcDays,
} from '../lib/account-activity-rollup.js';

describe('completedUtcDays (BACT-011)', () => {
  it('returns yesterday (UTC) as the single completed day for lookback=1', () => {
    const now = new Date('2026-08-13T00:30:00.000Z');
    expect(completedUtcDays(now, 1)).toEqual(['2026-08-12']);
  });

  it('never includes the current UTC day, even seconds before midnight', () => {
    const now = new Date('2026-08-13T23:59:59.000Z');
    expect(completedUtcDays(now, 1)).toEqual(['2026-08-12']);
  });

  it('rolls the date across a UTC boundary that a local offset would not', () => {
    // 00:05 UTC is still "yesterday" everywhere behind UTC, but the point
    // here is the function only ever reasons in UTC — no local timezone
    // conversion is applied.
    const now = new Date('2026-08-01T00:05:00.000Z');
    expect(completedUtcDays(now, 1)).toEqual(['2026-07-31']);
  });

  it('returns N completed days, most recent last, for lookback=N', () => {
    const now = new Date('2026-08-13T12:00:00.000Z');
    expect(completedUtcDays(now, 3)).toEqual(['2026-08-10', '2026-08-11', '2026-08-12']);
  });

  it('defaults the job lookback window to a small self-healing constant', () => {
    expect(DEFAULT_ROLLUP_LOOKBACK_DAYS).toBeGreaterThan(1);
    expect(DEFAULT_ROLLUP_LOOKBACK_DAYS).toBeLessThanOrEqual(14);
  });

  it('rejects a non-positive lookback', () => {
    const now = new Date('2026-08-13T12:00:00.000Z');
    expect(() => completedUtcDays(now, 0)).toThrow(/lookback/i);
    expect(() => completedUtcDays(now, -1)).toThrow(/lookback/i);
  });
});

describe('rollup constants', () => {
  it('exposes a schema version and a reserved all-plans total key', () => {
    expect(ACCOUNT_ACTIVITY_ROLLUP_SCHEMA_VERSION).toBe('anvil.account-activity-rollup.v1');
    // Reserved sentinel must never collide with a real (unprefixed) plan
    // name in the ACCOUNT_PLANS closed set (today only 'beta').
    expect(ROLLUP_TOTAL_PLAN_KEY).toBe('__all__');
  });
});
