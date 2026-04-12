import { NextResponse, type NextRequest } from 'next/server';
import { decryptState } from '@/lib/state';
import { validateNext } from '@/lib/next-url';
import { exchangeGithubCode } from '@/lib/bauth';

export const runtime = 'nodejs';

const COOKIE_NAME = 'anvil-docs-session';
const COOKIE_MAX_AGE = 7 * 24 * 60 * 60;

function requireEnv(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`${name} is required`);
  return v;
}

function errorRedirect(origin: string, reason: string): NextResponse {
  const url = new URL('/auth/error', origin);
  url.searchParams.set('reason', reason);
  return NextResponse.redirect(url, 302);
}

function clearNonce(response: NextResponse): NextResponse {
  response.cookies.set({
    name: 'oauth-nonce',
    value: '',
    path: '/auth/callback',
    maxAge: 0,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  return response;
}

export async function GET(request: NextRequest) {
  const url = new URL(request.url);
  const code = url.searchParams.get('code');
  const stateParam = url.searchParams.get('state');
  const error = url.searchParams.get('error');

  if (error) {
    return clearNonce(
      errorRedirect(url.origin, error === 'access_denied' ? 'denied' : 'oauth_error')
    );
  }

  if (!code || !stateParam) {
    return errorRedirect(url.origin, 'missing_params');
  }

  const state = await decryptState(stateParam, requireEnv('DOCS_STATE_SECRET'));
  if (!state) {
    return errorRedirect(url.origin, 'invalid_state');
  }

  const cookieNonce = request.cookies.get('oauth-nonce')?.value;
  if (!cookieNonce || cookieNonce !== state.nonce) {
    return clearNonce(errorRedirect(url.origin, 'csrf_mismatch'));
  }

  const next = validateNext(state.next);

  const result = await exchangeGithubCode(code);
  if (result.status === 'pending') {
    return clearNonce(NextResponse.redirect(new URL('/auth/pending', url.origin), 302));
  }
  if (result.status === 'error') {
    return clearNonce(errorRedirect(url.origin, result.reason));
  }

  const response = NextResponse.redirect(new URL(next, url.origin), 302);
  response.cookies.set({
    name: COOKIE_NAME,
    value: result.license,
    path: '/',
    maxAge: COOKIE_MAX_AGE,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  return clearNonce(response);
}
