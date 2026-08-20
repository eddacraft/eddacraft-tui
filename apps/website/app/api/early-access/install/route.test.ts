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

function installRequest() {
  return new Request('https://eddacraft.ai/api/early-access/install', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ accessKey: 'EDICT-test-key' }),
  });
}

describe('POST /api/early-access/install', () => {
  it('bounds verification and maps a timeout to service unavailable', async () => {
    const timeoutSignal = AbortSignal.abort(new DOMException('timed out', 'TimeoutError'));
    const timeout = vi.spyOn(AbortSignal, 'timeout').mockReturnValue(timeoutSignal);
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockRejectedValue(new DOMException('timed out', 'TimeoutError'));
    globalThis.fetch = fetchMock;
    process.env.NEXT_PUBLIC_API_URL = 'https://api.example.test/';

    const response = await POST(installRequest());

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

  it('maps a timeout while reading a successful response body to service unavailable', async () => {
    const timeoutController = new AbortController();
    vi.spyOn(AbortSignal, 'timeout').mockReturnValue(timeoutController.signal);
    globalThis.fetch = vi.fn<typeof fetch>().mockImplementation(async (_input, init) => {
      const signal = init?.signal;
      const json = () =>
        new Promise<never>((_resolve, reject) => {
          const rejectWithAbort = () =>
            reject(signal?.reason ?? new DOMException('timed out', 'TimeoutError'));
          if (signal?.aborted) {
            rejectWithAbort();
            return;
          }
          signal?.addEventListener('abort', rejectWithAbort, { once: true });
          setTimeout(
            () => timeoutController.abort(new DOMException('timed out', 'TimeoutError')),
            0
          );
        });

      return { ok: true, status: 200, json } as Response;
    });

    const response = await POST(installRequest());

    expect(response.status).toBe(503);
    await expect(response.json()).resolves.toEqual({ error: 'access_service_unavailable' });
  });
});
