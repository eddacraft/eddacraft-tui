import { NextResponse, type NextRequest } from 'next/server';
import { encryptState } from '@/lib/state';
import { validateNext } from '@/lib/next-url';

export const runtime = 'nodejs';

const GITHUB_AUTHORIZE_URL = 'https://github.com/login/oauth/authorize';
const SCOPES = 'read:user user:email';

function requireEnv(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`${name} is required`);
  return v;
}

function randomNonce(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

export async function GET(request: NextRequest) {
  const url = new URL(request.url);
  const next = validateNext(url.searchParams.get('next'));
  const nonce = randomNonce();

  const state = await encryptState({ next, nonce }, requireEnv('DOCS_STATE_SECRET'));

  const callbackUrl = new URL('/auth/callback', url.origin).toString();
  const authorizeUrl = new URL(GITHUB_AUTHORIZE_URL);
  authorizeUrl.searchParams.set('client_id', requireEnv('GITHUB_CLIENT_ID'));
  authorizeUrl.searchParams.set('redirect_uri', callbackUrl);
  authorizeUrl.searchParams.set('scope', SCOPES);
  authorizeUrl.searchParams.set('state', state);

  const response = NextResponse.redirect(authorizeUrl.toString(), 302);
  response.cookies.set({
    name: 'oauth-nonce',
    value: nonce,
    path: '/auth/callback',
    maxAge: 600,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  return response;
}
