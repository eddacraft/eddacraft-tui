import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { generateKeyPair, exportPKCS8 } from 'jose';
import { Hono } from 'hono';
import { authGithub } from '../routes/auth-github.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', () => ({
  linkOrCreateGitHubUser: vi.fn(),
  insertAuditLog: vi.fn().mockResolvedValue({
    id: 'audit-1',
    action: '',
    actor: '',
    metadata: {},
    created_at: new Date().toISOString(),
  }),
  insertRefreshToken: vi.fn().mockResolvedValue(undefined),
  findActiveScopesForUser: vi.fn().mockResolvedValue(['beta']),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    hashToken: vi.fn(),
  };
});

import {
  linkOrCreateGitHubUser,
  insertAuditLog,
  insertRefreshToken,
  findActiveScopesForUser,
} from '../db/queries.js';
import { hashToken } from '../lib/token.js';

const app = new Hono();
app.route('/auth/github', authGithub);

const ORIGINAL_CLIENT_ID = process.env['GITHUB_CLIENT_ID'];
const ORIGINAL_CLIENT_SECRET = process.env['GITHUB_CLIENT_SECRET'];
let originalSigningKey: string | undefined;

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  const { privateKey } = await generateKeyPair('ES256', { extractable: true });
  process.env['LICENSE_SIGNING_KEY'] = await exportPKCS8(privateKey);
});

afterAll(() => {
  if (originalSigningKey === undefined) delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
});

beforeEach(() => {
  vi.resetAllMocks();
  // Restore factory defaults after reset (resetAllMocks wipes implementations,
  // not just call history — so every mock needs its default re-stated here to
  // keep each test hermetic).
  vi.mocked(hashToken).mockReturnValue('mocked-refresh-hash');
  // Default: a brand-new pending user (overridden per lifecycle test). Tests
  // that fail before resolving identity assert this was never called.
  vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
    user: {
      id: 'user-1',
      email: 'octo@example.com',
      name: 'octocat',
      status: 'pending',
      notes: null,
      github_id: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    },
    isNewPending: true,
    didFirstLink: false,
  });
  vi.mocked(insertAuditLog).mockResolvedValue({
    id: 'audit-1',
    action: '',
    actor: '',
    metadata: {},
    auth_method: 'shared',
    created_at: new Date().toISOString(),
  });
  vi.mocked(insertRefreshToken).mockResolvedValue({
    id: 'refresh-1',
    user_id: 'user-1',
    token_hash: 'mocked-refresh-hash',
    family_id: 'family-1',
    expires_at: new Date(Date.now() + 90 * 24 * 60 * 60 * 1000).toISOString(),
    revoked_at: null,
    consumed_at: null,
    created_at: new Date().toISOString(),
  });
  // Default to the conservative `['beta']` fallback so existing tests
  // don't have to know about the new scope-lookup call.
  vi.mocked(findActiveScopesForUser).mockResolvedValue(['beta']);
  process.env['GITHUB_CLIENT_ID'] = 'test-client-id';
  process.env['GITHUB_CLIENT_SECRET'] = 'test-client-secret';
  vi.spyOn(globalThis, 'fetch').mockImplementation(() => {
    throw new Error('fetch called without a per-test mock — set one via mockFetchSequence');
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  if (ORIGINAL_CLIENT_ID === undefined) delete process.env['GITHUB_CLIENT_ID'];
  else process.env['GITHUB_CLIENT_ID'] = ORIGINAL_CLIENT_ID;
  if (ORIGINAL_CLIENT_SECRET === undefined) delete process.env['GITHUB_CLIENT_SECRET'];
  else process.env['GITHUB_CLIENT_SECRET'] = ORIGINAL_CLIENT_SECRET;
});

type FetchResponseSpec =
  | { ok: true; status?: number; json: unknown }
  | { ok: false; status: number; json?: unknown };

function jsonResponse(spec: FetchResponseSpec): Response {
  const status = spec.status ?? (spec.ok ? 200 : 400);
  const body = 'json' in spec && spec.json !== undefined ? JSON.stringify(spec.json) : '{}';
  return new Response(body, {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

/**
 * Drive the global fetch mock with a URL-keyed response map. Any request
 * not covered throws, which surfaces missing mock setup loudly.
 */
function mockFetch(responses: Record<string, FetchResponseSpec | FetchResponseSpec[]>) {
  const queues = new Map<string, FetchResponseSpec[]>();
  for (const [url, spec] of Object.entries(responses)) {
    queues.set(url, Array.isArray(spec) ? [...spec] : [spec]);
  }
  const keysByLongest = [...queues.keys()].sort((a, b) => b.length - a.length);
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (input: RequestInfo | URL) => {
    const url = typeof input === 'string' ? input : input.toString();
    const matchedKey = keysByLongest.find((k) => url.startsWith(k));
    if (!matchedKey) throw new Error(`no mock for fetch(${url})`);
    const queue = queues.get(matchedKey);
    if (!queue?.length) throw new Error(`no more mocked responses for ${matchedKey}`);
    const spec = queue.shift();
    return jsonResponse(spec!);
  });
}

function callback(body: unknown) {
  return app.request('/auth/github/callback', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

describe('POST /auth/github/callback', () => {
  describe('input validation', () => {
    it('rejects missing code via Zod with 400', async () => {
      const res = await callback({});
      expect(res.status).toBe(400);
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });

    it('rejects empty code strings via Zod with 400', async () => {
      const res = await callback({ code: '' });
      expect(res.status).toBe(400);
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });

    it('rejects codes longer than 256 chars via Zod with 400', async () => {
      const res = await callback({ code: 'x'.repeat(257) });
      expect(res.status).toBe(400);
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });
  });

  describe('GitHub API failures', () => {
    it('returns 401 when the token exchange HTTP call fails', async () => {
      mockFetch({
        'https://github.com/login/oauth/access_token': { ok: false, status: 502 },
      });

      const res = await callback({ code: 'gh-code' });
      expect(res.status).toBe(401);
      expect(await res.json()).toEqual({ error: 'GitHub authentication failed' });
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });

    it('returns 401 when GitHub returns an OAuth error body', async () => {
      mockFetch({
        'https://github.com/login/oauth/access_token': {
          ok: true,
          json: { error: 'bad_verification_code', error_description: 'bad code' },
        },
      });

      const res = await callback({ code: 'gh-code' });
      expect(res.status).toBe(401);
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });

    it('returns 401 when the user profile fetch fails', async () => {
      mockFetch({
        'https://github.com/login/oauth/access_token': {
          ok: true,
          json: { access_token: 'gh-token', token_type: 'bearer' },
        },
        'https://api.github.com/user': { ok: false, status: 500 },
        'https://api.github.com/user/emails': { ok: true, json: [] },
      });

      const res = await callback({ code: 'gh-code' });
      expect(res.status).toBe(401);
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });

    it('returns 401 when the account has no verified primary email', async () => {
      mockFetch({
        'https://github.com/login/oauth/access_token': {
          ok: true,
          json: { access_token: 'gh-token', token_type: 'bearer' },
        },
        'https://api.github.com/user': {
          ok: true,
          json: { id: 1, login: 'octocat', name: 'Octocat', avatar_url: null },
        },
        'https://api.github.com/user/emails': {
          ok: true,
          json: [{ email: 'octo@example.com', primary: false, verified: true }],
        },
      });

      const res = await callback({ code: 'gh-code' });
      expect(res.status).toBe(401);
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });

    it('returns 401 when GITHUB_CLIENT_ID/SECRET are unset', async () => {
      delete process.env['GITHUB_CLIENT_ID'];
      delete process.env['GITHUB_CLIENT_SECRET'];

      const res = await callback({ code: 'gh-code' });
      expect(res.status).toBe(401);
      expect(vi.mocked(linkOrCreateGitHubUser)).not.toHaveBeenCalled();
    });
  });

  describe('user lifecycle', () => {
    function mockHappyGitHub(overrides: { email?: string; id?: number; login?: string } = {}) {
      const id = overrides.id ?? 42;
      const login = overrides.login ?? 'octocat';
      const email = overrides.email ?? 'octo@example.com';
      mockFetch({
        'https://github.com/login/oauth/access_token': {
          ok: true,
          json: { access_token: 'gh-token', token_type: 'bearer' },
        },
        'https://api.github.com/user': {
          ok: true,
          json: { id, login, name: 'Octocat', avatar_url: null },
        },
        'https://api.github.com/user/emails': {
          ok: true,
          json: [{ email, primary: true, verified: true }],
        },
        // revokeGitHubToken fires-and-forgets to DELETE /applications/:id/token
        // after a successful profile fetch. We absorb it here so the background
        // fetch doesn't throw against the strict default-throw mock. Asserting
        // on it is deliberately skipped: the call is not awaited, so it races
        // the test's afterEach and would be flaky.
        'https://api.github.com/applications/': { ok: true, json: {} },
      });
      return { id, login, email };
    }

    it('creates a pending user on first-time signup and returns 403 account_pending', async () => {
      mockHappyGitHub();
      vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
        user: {
          id: 'user-1',
          email: 'octo@example.com',
          name: 'octocat',
          status: 'pending',
          notes: null,
          github_id: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        isNewPending: true,
        didFirstLink: false,
      });

      const res = await callback({ code: 'gh-code' });

      expect(res.status).toBe(403);
      expect(await res.json()).toEqual({ error: 'Account pending approval' });
      expect(vi.mocked(linkOrCreateGitHubUser)).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({ id: 42, login: 'octocat' })
      );
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_signup',
        'octo@example.com',
        expect.objectContaining({ githubId: 42, githubLogin: 'octocat' })
      );
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_blocked',
        'octo@example.com',
        expect.objectContaining({ githubId: 42, status: 'pending' })
      );
      expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
    });

    it('returns 403 without issuing tokens for an existing non-active user', async () => {
      mockHappyGitHub();
      vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
        user: {
          id: 'user-2',
          email: 'octo@example.com',
          name: 'octocat',
          status: 'suspended',
          notes: null,
          github_id: 42,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        isNewPending: false,
        didFirstLink: false,
      });

      const res = await callback({ code: 'gh-code' });

      expect(res.status).toBe(403);
      expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_blocked',
        'octo@example.com',
        expect.objectContaining({ status: 'suspended' })
      );
      expect(vi.mocked(insertAuditLog)).not.toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_signup',
        expect.anything(),
        expect.anything()
      );
    });

    it('issues a licence JWT and refresh token for an active user', async () => {
      mockHappyGitHub({ email: 'active@example.com' });
      vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
        user: {
          id: 'user-3',
          email: 'active@example.com',
          name: 'octocat',
          status: 'active',
          notes: null,
          github_id: 42,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        isNewPending: false,
        didFirstLink: false,
      });

      const res = await callback({ code: 'gh-code' });

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(typeof body.license).toBe('string');
      expect(body.license.split('.').length).toBe(3);
      expect(typeof body.refreshToken).toBe('string');
      expect(body.refreshToken.length).toBeGreaterThanOrEqual(32);
      expect(body.expiresAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);

      expect(vi.mocked(insertRefreshToken)).toHaveBeenCalledWith(
        expect.anything(),
        'user-3',
        'mocked-refresh-hash',
        expect.stringMatching(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i),
        expect.any(Date)
      );
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_login',
        'active@example.com',
        expect.objectContaining({ githubId: 42 })
      );
    });

    it('audits github_oauth_link when first-linking an active invite', async () => {
      mockHappyGitHub({ email: 'invited@example.com' });
      vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
        user: {
          id: 'user-4',
          email: 'invited@example.com',
          name: 'octocat',
          status: 'active',
          notes: null,
          github_id: 42,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        isNewPending: false,
        didFirstLink: true,
      });

      const res = await callback({ code: 'gh-code' });

      expect(res.status).toBe(200);
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_link',
        'invited@example.com',
        expect.objectContaining({ githubId: 42, githubLogin: 'octocat' })
      );
    });
  });
});
