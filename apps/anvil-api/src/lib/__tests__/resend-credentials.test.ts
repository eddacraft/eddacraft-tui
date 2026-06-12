import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { verifyResendKey, _resetResendKeyCacheForTests } from '../resend-credentials.js';

const ORIGINAL_KEY = process.env['RESEND_API_KEY'];

function mockFetchResponse(status: number, body: unknown): ReturnType<typeof vi.spyOn> {
  return vi.spyOn(globalThis, 'fetch').mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response);
}

beforeEach(() => {
  _resetResendKeyCacheForTests();
  process.env['RESEND_API_KEY'] = 're_test_key';
});

afterEach(() => {
  vi.restoreAllMocks();
  _resetResendKeyCacheForTests();
  if (ORIGINAL_KEY === undefined) delete process.env['RESEND_API_KEY'];
  else process.env['RESEND_API_KEY'] = ORIGINAL_KEY;
});

describe('verifyResendKey (CIB-067)', () => {
  it('reports ok for an accepted key', async () => {
    mockFetchResponse(200, { data: [] });
    expect(await verifyResendKey()).toBe('ok');
  });

  it('reports ok for a sending-only (restricted) key', async () => {
    // Sending-only keys are rejected by read endpoints with a distinct
    // error name — the key is alive and can send, which is all that
    // production email needs.
    mockFetchResponse(401, { statusCode: 401, name: 'restricted_api_key' });
    expect(await verifyResendKey()).toBe('ok');
  });

  it('reports invalid for a dead key', async () => {
    mockFetchResponse(401, { statusCode: 401, name: 'validation_error' });
    expect(await verifyResendKey()).toBe('invalid');
  });

  it('reports unverifiable on Resend-side failure', async () => {
    mockFetchResponse(500, {});
    expect(await verifyResendKey()).toBe('unverifiable');
  });

  it('reports unverifiable on network failure', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('ECONNRESET'));
    expect(await verifyResendKey()).toBe('unverifiable');
  });

  it('reports unconfigured without touching the network when the env is missing', async () => {
    delete process.env['RESEND_API_KEY'];
    const spy = vi.spyOn(globalThis, 'fetch');
    expect(await verifyResendKey()).toBe('unconfigured');
    expect(spy).not.toHaveBeenCalled();
  });

  it('caches the probe result so health polling does not hammer Resend', async () => {
    const spy = mockFetchResponse(200, { data: [] });
    await verifyResendKey();
    await verifyResendKey();
    await verifyResendKey();
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('does not cache the unconfigured state (mid-flight provisioning shows up)', async () => {
    delete process.env['RESEND_API_KEY'];
    expect(await verifyResendKey()).toBe('unconfigured');
    process.env['RESEND_API_KEY'] = 're_test_key';
    mockFetchResponse(200, { data: [] });
    expect(await verifyResendKey()).toBe('ok');
  });
});
