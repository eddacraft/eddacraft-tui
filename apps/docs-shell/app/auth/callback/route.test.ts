import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextRequest } from 'next/server';

vi.mock('@/lib/state', () => ({
  decryptState: vi.fn(),
}));

vi.mock('@/lib/bauth', () => ({
  exchangeGithubCode: vi.fn(),
}));

import { decryptState } from '@/lib/state';
import { exchangeGithubCode } from '@/lib/bauth';
import { GET } from './route';

const SECRET = 'docs-state-secret-for-tests';
const NONCE = 'nonce-1';

function callbackRequest(query: string, cookie?: string): NextRequest {
  return new NextRequest(`https://docs.eddacraft.ai/auth/callback${query}`, {
    headers: cookie ? { cookie } : undefined,
  });
}

describe('GET /auth/callback', () => {
  beforeEach(() => {
    process.env.DOCS_STATE_SECRET = SECRET;
    vi.mocked(decryptState).mockReset();
    vi.mocked(exchangeGithubCode).mockReset();
  });

  afterEach(() => {
    delete process.env.DOCS_STATE_SECRET;
    vi.restoreAllMocks();
  });

  it('redirects provider denial to the denied error page and clears the nonce', async () => {
    const response = await GET(callbackRequest('?error=access_denied'));
    expect(response.status).toBe(302);
    expect(response.headers.get('location')).toBe(
      'https://docs.eddacraft.ai/auth/error?reason=denied'
    );
    expect(response.cookies.get('oauth-nonce')?.value).toBe('');
    expect(decryptState).not.toHaveBeenCalled();
  });

  it('redirects other provider errors to oauth_error', async () => {
    const response = await GET(callbackRequest('?error=server_error'));
    expect(response.headers.get('location')).toBe(
      'https://docs.eddacraft.ai/auth/error?reason=oauth_error'
    );
  });

  it('rejects a missing code or state', async () => {
    const response = await GET(callbackRequest('?code=gh-code'));
    expect(response.status).toBe(302);
    expect(response.headers.get('location')).toBe(
      'https://docs.eddacraft.ai/auth/error?reason=missing_params'
    );
    expect(decryptState).not.toHaveBeenCalled();
  });

  it('rejects invalid state', async () => {
    vi.mocked(decryptState).mockResolvedValue(null);
    const response = await GET(callbackRequest('?code=gh-code&state=bad'));
    expect(response.headers.get('location')).toBe(
      'https://docs.eddacraft.ai/auth/error?reason=invalid_state'
    );
    expect(exchangeGithubCode).not.toHaveBeenCalled();
  });

  it('rejects a nonce mismatch and clears the nonce cookie', async () => {
    vi.mocked(decryptState).mockResolvedValue({
      next: '/anvil/overview',
      nonce: NONCE,
      iat: 1,
    });
    const response = await GET(callbackRequest('?code=gh-code&state=enc', 'oauth-nonce=other'));
    expect(response.headers.get('location')).toBe(
      'https://docs.eddacraft.ai/auth/error?reason=csrf_mismatch'
    );
    expect(response.cookies.get('oauth-nonce')?.value).toBe('');
    expect(exchangeGithubCode).not.toHaveBeenCalled();
  });

  it('sends pending BAUTH outcomes to /auth/pending and clears the nonce', async () => {
    vi.mocked(decryptState).mockResolvedValue({
      next: '/anvil/overview',
      nonce: NONCE,
      iat: 1,
    });
    vi.mocked(exchangeGithubCode).mockResolvedValue({ status: 'pending' });
    const response = await GET(callbackRequest('?code=gh-code&state=enc', `oauth-nonce=${NONCE}`));
    expect(response.headers.get('location')).toBe('https://docs.eddacraft.ai/auth/pending');
    expect(response.cookies.get('oauth-nonce')?.value).toBe('');
  });

  it('maps BAUTH errors onto the error page', async () => {
    vi.mocked(decryptState).mockResolvedValue({
      next: '/anvil/overview',
      nonce: NONCE,
      iat: 1,
    });
    vi.mocked(exchangeGithubCode).mockResolvedValue({
      status: 'error',
      reason: 'auth_failed',
    });
    const response = await GET(callbackRequest('?code=gh-code&state=enc', `oauth-nonce=${NONCE}`));
    expect(response.headers.get('location')).toBe(
      'https://docs.eddacraft.ai/auth/error?reason=auth_failed'
    );
  });

  it('falls back to the default next path when state.next is not an /anvil destination', async () => {
    vi.mocked(decryptState).mockResolvedValue({
      next: 'https://evil.example',
      nonce: NONCE,
      iat: 1,
    });
    vi.mocked(exchangeGithubCode).mockResolvedValue({
      status: 'ok',
      license: 'jwt.here',
    });
    const response = await GET(callbackRequest('?code=gh-code&state=enc', `oauth-nonce=${NONCE}`));
    expect(response.headers.get('location')).toBe('https://docs.eddacraft.ai/anvil/overview');
  });

  it('sets the session cookie and clears the nonce on success', async () => {
    vi.mocked(decryptState).mockResolvedValue({
      next: '/anvil/overview',
      nonce: NONCE,
      iat: 1,
    });
    vi.mocked(exchangeGithubCode).mockResolvedValue({
      status: 'ok',
      license: 'jwt.here',
    });
    const response = await GET(callbackRequest('?code=gh-code&state=enc', `oauth-nonce=${NONCE}`));
    expect(response.status).toBe(302);
    expect(response.headers.get('location')).toBe('https://docs.eddacraft.ai/anvil/overview');
    const session = response.cookies.get('anvil-docs-session');
    expect(session?.value).toBe('jwt.here');
    expect(session?.path).toBe('/');
    expect(session?.httpOnly).toBe(true);
    expect(session?.secure).toBe(true);
    expect(session?.sameSite).toBe('lax');
    expect(session?.maxAge).toBe(7 * 24 * 60 * 60);
    expect(response.cookies.get('oauth-nonce')?.value).toBe('');
  });
});
