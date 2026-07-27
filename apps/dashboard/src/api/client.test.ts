import { afterEach, describe, expect, it, vi } from 'vitest';

import { createDashboardApi } from '@/api/client';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('dashboard API transport failures', () => {
  it('propagates an ordinary development network TypeError instead of returning fixture data', async () => {
    const api = createDashboardApi({
      GET: vi.fn().mockRejectedValue(new TypeError('network offline')),
    });

    await expect(api.getProtectionOverview()).rejects.toThrow('network offline');
  });

  it('requests retained protection history through the generated endpoint', async () => {
    const GET = vi.fn().mockResolvedValue({ data: { points: [] } });
    const api = createDashboardApi({ GET });

    await expect(api.getProtectionHistory()).resolves.toEqual({ points: [] });
    expect(GET).toHaveBeenCalledWith('/api/v1/protection/history');
  });
});
