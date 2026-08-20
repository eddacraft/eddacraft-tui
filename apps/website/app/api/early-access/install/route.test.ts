import { afterEach, describe, expect, it, vi } from 'vitest';

import { POST } from './route';

const originalFetch = globalThis.fetch;
const originalApiUrl = process.env.NEXT_PUBLIC_API_URL;

afterEach(() => {
  globalThis.fetch = originalFetch;
  if (originalApiUrl === undefined) {
    delete process.env.NEXT_PUBLIC_API_URL;
  } else {
    process.env.NEXT_PUBLIC_API_URL = originalApiUrl;
  }
  vi.restoreAllMocks();
});

describe('POST /api/early-access/install', () => {
  it('bounds verification and maps a timeout to service unavailable', async () => {
    const timeoutSignal = AbortSignal.abort(new DOMException('timed out', 'TimeoutError'));
    const timeout = vi.spyOn(AbortSignal, 'timeout').mockReturnValue(timeoutSignal);
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockRejectedValue(new DOMException('timed out', 'TimeoutError'));
    globalThis.fetch = fetchMock;
    process.env.NEXT_PUBLIC_API_URL = 'https://api.example.test/';

    const response = await POST(
      new Request('https://eddacraft.ai/api/early-access/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ accessKey: 'EDICT-test-key' }),
      })
    );

    expect(timeout).toHaveBeenCalledWith(8_000);
    expect(fetchMock).toHaveBeenCalledWith(
      'https://api.example.test/api/v1/auth/verify',
      expect.objectContaining({
        method: 'POST',
        signal: timeoutSignal,
      })
    );
    expect(response.status).toBe(503);
    await expect(response.json()).resolves.toEqual({ error: 'access_service_unavailable' });
  });
});
