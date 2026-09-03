import { describe, expect, it, vi } from 'vitest';
import { waitForOtpClaimContention } from './support/neon-otp-contention.js';

function observerWithCounts(...waitingCounts: number[]) {
  return {
    query: vi.fn(async () => ({
      rows: [{ waiting_claimants: waitingCounts.shift() ?? 0 }],
    })),
  };
}

describe('Neon OTP contention observer', () => {
  it('returns only after more than the attempt cap are visibly waiting on database locks', async () => {
    const observer = observerWithCounts(0, 2, 4);

    await expect(
      waitForOtpClaimContention(observer, '2026-08-31T06:00:00.000Z', {
        minimumWaitingClaimants: 4,
        maximumObservations: 5,
      })
    ).resolves.toEqual({ observations: 3, waitingClaimants: 4 });

    expect(observer.query).toHaveBeenCalledTimes(3);
    expect(observer.query.mock.calls[0]?.[0]).toContain("wait_event_type = 'Lock'");
    expect(observer.query.mock.calls[0]?.[0]).toContain('UPDATE otp_codes');
  });

  it('fails closed when database contention is never observed', async () => {
    const observer = observerWithCounts(0, 1, 2);

    await expect(
      waitForOtpClaimContention(observer, '2026-08-31T06:00:00.000Z', {
        minimumWaitingClaimants: 4,
        maximumObservations: 3,
      })
    ).rejects.toThrow('observed 2 waiting OTP claimants; required 4');

    expect(observer.query).toHaveBeenCalledTimes(3);
  });
});
