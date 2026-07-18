/**
 * GET /auth/login — Redirects to GitHub OAuth authorize URL.
 *
 * Generates an encrypted state parameter containing the `next` URL
 * and a CSRF nonce, then redirects the browser to GitHub.
 */

import { randomBytes, createCipheriv, createHash } from 'node:crypto';

const GITHUB_AUTHORIZE_URL = 'https://github.com/login/oauth/authorize';
const SCOPES = 'read:user user:email';

function getClientId(): string {
  const id = process.env.GITHUB_CLIENT_ID;
  if (!id) throw new Error('GITHUB_CLIENT_ID is required');
  return id;
}

function getStateSecret(): Buffer {
  const secret = process.env.STATE_SECRET;
  if (!secret) throw new Error('STATE_SECRET is required');
  return createHash('sha256').update(secret).digest();
}

function encryptState(payload: Record<string, string>): string {
  const key = getStateSecret();
  const iv = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', key, iv);
  const plaintext = JSON.stringify(payload);
  const encrypted = Buffer.concat([cipher.update(plaintext, 'utf8'), cipher.final()]);
  const tag = cipher.getAuthTag();
  // iv (12) + tag (16) + ciphertext
  return Buffer.concat([iv, tag, encrypted]).toString('base64url');
}

function validateNext(next: string | null): string {
  if (!next) return '/anvil/overview';
  // Normalise to resolve .. segments, then check prefix
  const resolved = new URL(next, 'https://placeholder').pathname;
  if (!resolved.startsWith('/anvil')) return '/anvil/overview';
  return resolved;
}

export default async function handler(request: Request): Promise<Response> {
  const url = new URL(request.url);
  const next = validateNext(url.searchParams.get('next'));

  const nonce = randomBytes(16).toString('hex');
  const state = encryptState({ next, nonce });

  const callbackUrl = new URL('/auth/callback', url.origin).toString();

  const authorizeUrl = new URL(GITHUB_AUTHORIZE_URL);
  authorizeUrl.searchParams.set('client_id', getClientId());
  authorizeUrl.searchParams.set('redirect_uri', callbackUrl);
  authorizeUrl.searchParams.set('scope', SCOPES);
  authorizeUrl.searchParams.set('state', state);

  // Show a brief interstitial for users without JS redirect
  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta http-equiv="refresh" content="0;url=${authorizeUrl.toString()}">
  <title>Sign in — eddacraft Docs</title>
  <style>
    body { font-family: system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; background: #0a0a0a; color: #e5e5e5; }
    .card { text-align: center; max-width: 400px; padding: 2rem; }
    a { display: inline-block; margin-top: 1rem; padding: 0.75rem 1.5rem; background: #2563eb; color: white; text-decoration: none; border-radius: 0.5rem; font-weight: 500; }
    a:hover { background: #1d4ed8; }
  </style>
</head>
<body>
  <div class="card">
    <h1>anvil docs</h1>
    <p>Sign in with GitHub to access anvil documentation.</p>
    <a href="${authorizeUrl.toString()}">Sign in with GitHub</a>
  </div>
</body>
</html>`;

  // Persist nonce in a short-lived HttpOnly cookie for CSRF validation in callback
  const nonceCookie = `oauth-nonce=${nonce}; Path=/auth/callback; Max-Age=600; HttpOnly; Secure; SameSite=Lax`;

  return new Response(html, {
    status: 200,
    headers: [
      ['Content-Type', 'text/html; charset=utf-8'],
      ['Set-Cookie', nonceCookie],
    ],
  });
}
