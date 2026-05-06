/**
 * Reliability-budget unit tests.
 *
 * Pinned by DRVR-007 §2.3a + §2.6 + DRVR-001 brief:
 *   - Quarantine MUST key on `originating_driver_id`, not `driverName`.
 *   - Quarantine survives reconnect (covered indirectly: budget is
 *     in-process and not reset by the client constructor — see
 *     `reliability-survives-reconnect.test.ts` once the integration
 *     test lands).
 *   - Sliding window with cooldown.
 */

import { describe, expect, it } from 'vitest';

import { ReliabilityBudget } from './budget.js';

describe('ReliabilityBudget', () => {
  it('does nothing for undefined / empty identity (pre-handshake)', () => {
    const budget = new ReliabilityBudget({ failureThreshold: 1 });
    // Pre-handshake failures are not counted — the daemon already
    // handles them via SO_PEERCRED, double-budgeting would self-DoS.
    expect(budget.recordFailure(undefined)).toBe(false);
    expect(budget.recordFailure('')).toBe(false);
    expect(budget.isQuarantined(undefined)).toBe(false);
    expect(budget.snapshot()).toEqual([]);
  });

  it('quarantines after threshold failures within window', () => {
    let nowMs = 1_000_000;
    const budget = new ReliabilityBudget({
      failureThreshold: 3,
      windowMs: 60_000,
      cooldownMs: 5 * 60_000,
      now: () => nowMs,
    });
    const id = 'driver-id-A';
    expect(budget.recordFailure(id)).toBe(false);
    expect(budget.recordFailure(id)).toBe(false);
    expect(budget.recordFailure(id)).toBe(true); // crosses threshold
    expect(budget.isQuarantined(id)).toBe(true);
    // Cooldown not yet elapsed
    nowMs += 60_000;
    expect(budget.isQuarantined(id)).toBe(true);
    // After cooldown
    nowMs += 5 * 60_000;
    expect(budget.isQuarantined(id)).toBe(false);
  });

  it('does not count failures outside the window', () => {
    let nowMs = 1_000_000;
    const budget = new ReliabilityBudget({
      failureThreshold: 2,
      windowMs: 1_000,
      cooldownMs: 1_000,
      now: () => nowMs,
    });
    const id = 'driver-A';
    budget.recordFailure(id);
    nowMs += 5_000; // outside window
    expect(budget.recordFailure(id)).toBe(false);
    expect(budget.isQuarantined(id)).toBe(false);
  });

  it('keys quarantine per-identity (different drivers do not interfere)', () => {
    const nowMs = 1_000_000;
    const budget = new ReliabilityBudget({
      failureThreshold: 2,
      windowMs: 60_000,
      now: () => nowMs,
    });
    expect(budget.recordFailure('driver-A')).toBe(false);
    expect(budget.recordFailure('driver-A')).toBe(true);
    expect(budget.isQuarantined('driver-A')).toBe(true);
    expect(budget.isQuarantined('driver-B')).toBe(false);
  });

  it('recordSuccess clears failures and ends quarantine', () => {
    const nowMs = 1_000_000;
    const budget = new ReliabilityBudget({
      failureThreshold: 2,
      windowMs: 60_000,
      now: () => nowMs,
    });
    const id = 'driver-A';
    budget.recordFailure(id);
    budget.recordFailure(id);
    expect(budget.isQuarantined(id)).toBe(true);
    budget.recordSuccess(id);
    expect(budget.isQuarantined(id)).toBe(false);
    expect(budget.snapshot()[0]?.failures).toEqual([]);
  });

  it('quarantine survives transport reconnect (no implicit reset)', () => {
    // The "survives reconnect" property is a function of who holds
    // the budget. The contract: a single ReliabilityBudget instance
    // keeps state across however many transport drops the client
    // experiences. We exercise that by simulating successive failure
    // bursts separated by "reconnect" (no-op on the budget).
    const nowMs = 1_000_000;
    const budget = new ReliabilityBudget({
      failureThreshold: 3,
      windowMs: 60_000,
      cooldownMs: 60_000,
      now: () => nowMs,
    });
    const id = 'driver-A';
    budget.recordFailure(id);
    budget.recordFailure(id);
    // Simulated reconnect: no API call against the budget.
    expect(budget.isQuarantined(id)).toBe(false);
    expect(budget.recordFailure(id)).toBe(true);
    expect(budget.isQuarantined(id)).toBe(true);
  });

  it('snapshot returns clones, not live references', () => {
    const budget = new ReliabilityBudget({ failureThreshold: 5 });
    budget.recordFailure('driver-A');
    const snap = budget.snapshot();
    snap[0]!.failures.push(1);
    expect(budget.snapshot()[0]?.failures.length).toBe(1);
  });

  it('rejects bad construction options', () => {
    expect(() => new ReliabilityBudget({ failureThreshold: 0 })).toThrow(RangeError);
    expect(() => new ReliabilityBudget({ windowMs: 0 })).toThrow(RangeError);
    expect(() => new ReliabilityBudget({ cooldownMs: -1 })).toThrow(RangeError);
  });
});
