/**
 * GET /auth/callback — GitHub OAuth callback handler.
 *
 * Decrypts the state parameter, validates the CSRF nonce, exchanges
 * the OAuth code via the BAUTH API, sets the session cookie, and
 * redirects to the original page.
 */

import { createDecipheriv, createHash } from 'node:crypto';

const COOKIE_NAME = 'anvil-docs-session';
const NONCE_COOKIE = 'oauth-nonce';
const COOKIE_MAX_AGE = 7 * 24 * 60 * 60; // 7 days in seconds

function getCookie(cookieHeader: string | null, name: string): string | undefined {
  if (!cookieHeader) return undefined;
  const match = cookieHeader.match(new RegExp(`(?:^|;\\s*)${name}=([^;]*)`));
  if (!match) return undefined;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return undefined;
  }
}

function getStateSecret(): Buffer {
  const secret = process.env.STATE_SECRET;
  if (!secret) throw new Error('STATE_SECRET is required');
  return createHash('sha256').update(secret).digest();
}

function getApiUrl(): string {
  return process.env.BAUTH_API_URL ?? 'https://api.eddacraft.ai';
}

function decryptState(encrypted: string): { next: string; nonce: string } | null {
  try {
    const key = getStateSecret();
    const buf = Buffer.from(encrypted, 'base64url');
    if (buf.length < 28) return null; // iv(12) + tag(16) minimum

    const iv = buf.subarray(0, 12);
    const tag = buf.subarray(12, 28);
    const ciphertext = buf.subarray(28);

    const decipher = createDecipheriv('aes-256-gcm', key, iv);
    decipher.setAuthTag(tag);
    const plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]).toString(
      'utf8'
    );

    const parsed = JSON.parse(plaintext);
    if (typeof parsed.next !== 'string' || typeof parsed.nonce !== 'string') return null;
    return parsed as { next: string; nonce: string };
  } catch {
    return null;
  }
}

function validateNext(next: string): string {
  // Normalise to resolve .. segments, then check prefix
  const resolved = new URL(next, 'https://placeholder').pathname;
  if (!resolved.startsWith('/anvil')) return '/anvil/overview';
  return resolved;
}

function makeCookie(token: string): string {
  return `${COOKIE_NAME}=${token}; Path=/; Max-Age=${COOKIE_MAX_AGE}; HttpOnly; Secure; SameSite=Lax`;
}

function errorRedirect(origin: string, error: string): Response {
  const url = new URL('/', origin);
  url.searchParams.set('error', error);
  return Response.redirect(url.toString(), 302);
}

function pendingPage(): Response {
  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Access Pending — eddacraft Docs</title>
  <style>
    body { font-family: system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; background: #0a0a0a; color: #e5e5e5; }
    .card { text-align: center; max-width: 480px; padding: 2rem; }
    a { color: #60a5fa; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Access Pending</h1>
    <p>Your GitHub account has been registered, but access to anvil documentation requires approval.</p>
    <p>You'll receive an email once your access has been approved.</p>
    <p><a href="/">Return to docs home</a></p>
  </div>
</body>
</html>`;
  return new Response(html, {
    status: 403,
    headers: { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' },
  });
}

export default async function handler(request: Request): Promise<Response> {
  const url = new URL(request.url);
  const code = url.searchParams.get('code');
  const stateParam = url.searchParams.get('state');
  const error = url.searchParams.get('error');

  // GitHub OAuth denied/cancelled
  if (error) {
    return errorRedirect(url.origin, error === 'access_denied' ? 'denied' : 'oauth_error');
  }

  if (!code || !stateParam) {
    return errorRedirect(url.origin, 'missing_params');
  }

  // Decrypt and validate state (CSRF protection)
  const state = decryptState(stateParam);
  if (!state) {
    return errorRedirect(url.origin, 'invalid_state');
  }

  // Verify CSRF nonce matches the cookie set by /auth/login
  const cookieHeader = request.headers.get('cookie');
  const cookieNonce = getCookie(cookieHeader, NONCE_COOKIE);
  if (!cookieNonce || cookieNonce !== state.nonce) {
    return errorRedirect(url.origin, 'csrf_mismatch');
  }

  const next = validateNext(state.next);

  // Exchange code via BAUTH API
  const apiUrl = getApiUrl();
  let apiRes: Response;
  try {
    apiRes = await fetch(`${apiUrl}/api/v1/auth/github/callback`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });
  } catch {
    return errorRedirect(url.origin, 'api_error');
  }

  // Handle BAUTH API error responses
  if (apiRes.status === 403) {
    // User exists but not active (pending approval)
    return pendingPage();
  }

  if (!apiRes.ok) {
    return errorRedirect(url.origin, 'auth_failed');
  }

  let body: unknown;
  try {
    body = await apiRes.json();
  } catch {
    return errorRedirect(url.origin, 'invalid_response');
  }

  const parsed = body as Record<string, unknown>;
  if (!parsed || typeof parsed.license !== 'string') {
    return errorRedirect(url.origin, 'invalid_response');
  }

  // Set session cookie, clear nonce cookie, and redirect to original page
  const redirectUrl = new URL(next, url.origin);
  const clearNonce = `${NONCE_COOKIE}=; Path=/auth/callback; Max-Age=0; HttpOnly; Secure; SameSite=Lax`;
  return new Response(null, {
    status: 302,
    headers: [
      ['Location', redirectUrl.toString()],
      ['Set-Cookie', makeCookie(parsed.license)],
      ['Set-Cookie', clearNonce],
    ],
  });
}
