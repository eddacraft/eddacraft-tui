import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Factory implementations must exist at import time — index.ts fires the
// signing/verifying-key boot probes when the module loads.
vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn(async () => [{ '?column?': 1 }])),
}));

vi.mock('../lib/licence.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/licence.js')>();
  return {
    ...actual,
    verifySigningKey: vi.fn(async () => ({ ok: true }) as const),
    verifyVerifyingKey: vi.fn(async () => ({ ok: true }) as const),
  };
});

import app from '../index.js';
import { getClient } from '../db/client.js';
import { verifySigningKey, verifyVerifyingKey } from '../lib/licence.js';

const ORIGINAL_CLIENT_ID = process.env['GITHUB_CLI_CLIENT_ID'];
const ORIGINAL_CLIENT_SECRET = process.env['GITHUB_CLI_CLIENT_SECRET'];

beforeEach(() => {
  // Re-establish per test: the afterEach restore wipes the factory impls.
  vi.mocked(getClient).mockReturnValue(vi.fn(async () => [{ '?column?': 1 }]) as never);
  vi.mocked(verifySigningKey).mockResolvedValue({ ok: true });
  vi.mocked(verifyVerifyingKey).mockResolvedValue({ ok: true });
  process.env['GITHUB_CLI_CLIENT_ID'] = 'test-cli-client-id';
  process.env['GITHUB_CLI_CLIENT_SECRET'] = 'test-cli-client-secret';
});

afterEach(() => {
  vi.restoreAllMocks();
  if (ORIGINAL_CLIENT_ID === undefined) delete process.env['GITHUB_CLI_CLIENT_ID'];
  else process.env['GITHUB_CLI_CLIENT_ID'] = ORIGINAL_CLIENT_ID;
  if (ORIGINAL_CLIENT_SECRET === undefined) delete process.env['GITHUB_CLI_CLIENT_SECRET'];
  else process.env['GITHUB_CLI_CLIENT_SECRET'] = ORIGINAL_CLIENT_SECRET;
});

describe('GET /api/v1/health', () => {
  it('reports ok when db, keys, and CLI OAuth credentials are all present', async () => {
    const res = await app.request('/api/v1/health');
    expect(res.status).toBe(200);
    expect((await res.json()) as Record<string, unknown>).toEqual({
      status: 'ok',
      db: 'ok',
      signingKey: 'ok',
      verifyingKey: 'ok',
      githubCliCreds: 'ok',
    });
  });

  it('degrades to 503 when the CLI OAuth credentials are missing', async () => {
    // GHCLIAUTH-006: the device-flow login is the CLI default, so a deploy
    // without the Anvil CLI OAuth credentials is user-impacting and must
    // surface as degraded, not as an informational footnote.
    delete process.env['GITHUB_CLI_CLIENT_ID'];

    const res = await app.request('/api/v1/health');
    expect(res.status).toBe(503);
    expect((await res.json()) as Record<string, unknown>).toEqual({
      status: 'degraded',
      db: 'ok',
      signingKey: 'ok',
      verifyingKey: 'ok',
      githubCliCreds: 'unavailable',
    });
  });
});
