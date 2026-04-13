import { NextResponse, type NextRequest } from 'next/server';
import { verifyLicense } from './lib/jwt';
import { getCookie } from './lib/cookie';

const COOKIE_NAME = 'anvil-docs-session';

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`${name} environment variable is required`);
  return value;
}

const ANVIL_DOCS_URL = requireEnv('ANVIL_DOCS_URL');
const PUBLIC_DOCS_URL = requireEnv('PUBLIC_DOCS_URL');
const UPSTREAM_SECRET = requireEnv('DOCS_UPSTREAM_SECRET');

const FORWARDED_REQUEST_HEADERS = [
  'accept',
  'accept-encoding',
  'accept-language',
  'if-none-match',
  'if-modified-since',
  'cache-control',
  'user-agent',
  'range',
];

const STRIP_RESPONSE_HEADERS = [
  'set-cookie',
  'server',
  'x-vercel-id',
  'x-vercel-cache',
  'x-middleware-rewrite',
  'x-middleware-next',
  'x-docs-upstream-secret',
  'via',
];

function redirectToLogin(request: NextRequest, clearCookie: boolean): NextResponse {
  const url = new URL(request.url);
  const loginUrl = new URL('/auth/login', url.origin);
  loginUrl.searchParams.set('next', url.pathname);
  const response = NextResponse.redirect(loginUrl, 302);
  if (clearCookie) {
    response.cookies.set({
      name: COOKIE_NAME,
      value: '',
      path: '/',
      maxAge: 0,
      httpOnly: true,
      secure: true,
      sameSite: 'lax',
    });
  }
  return response;
}

async function proxyToUpstream(request: NextRequest, upstream: string): Promise<Response> {
  const url = new URL(request.url);
  const destination = new URL(url.pathname + url.search, upstream);

  const headers = new Headers();
  for (const name of FORWARDED_REQUEST_HEADERS) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }
  headers.set('x-docs-upstream-secret', UPSTREAM_SECRET);

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const response = await fetch(destination, {
      method: request.method,
      headers,
      body: ['GET', 'HEAD'].includes(request.method) ? undefined : request.body,
      redirect: 'manual',
      signal: controller.signal,
    });

    clearTimeout(timeout);

    const responseHeaders = new Headers(response.headers);
    for (const h of STRIP_RESPONSE_HEADERS) {
      responseHeaders.delete(h);
    }

    if (response.status >= 300 && response.status < 400) {
      const location = responseHeaders.get('location');
      if (location) {
        try {
          const locUrl = new URL(location);
          if (
            locUrl.origin === new URL(ANVIL_DOCS_URL).origin ||
            locUrl.origin === new URL(PUBLIC_DOCS_URL).origin
          ) {
            if (locUrl.pathname.startsWith('/auth/')) {
              return new Response('Forbidden', { status: 403 });
            } else {
              locUrl.hostname = url.hostname;
              locUrl.port = url.port;
              locUrl.protocol = url.protocol;
              responseHeaders.set('location', locUrl.toString());
            }
          }
        } catch {
          // relative URL or parse error — pass through
        }
      }
    }

    const body = response.status === 204 || response.status === 304 ? null : response.body;

    return new Response(body, {
      status: response.status,
      statusText: response.statusText,
      headers: responseHeaders,
    });
  } catch (err) {
    clearTimeout(timeout);
    const message =
      err instanceof Error && err.name === 'AbortError'
        ? 'Upstream timeout'
        : 'Upstream unavailable';
    return new Response(message, { status: 503 });
  }
}

export default async function proxy(request: NextRequest): Promise<Response> {
  const { pathname } = new URL(request.url);

  if (pathname.startsWith('/anvil/') || pathname === '/anvil') {
    const token = getCookie(request.headers.get('cookie'), COOKIE_NAME);
    if (!token) return redirectToLogin(request, false);

    const { valid } = await verifyLicense(token);
    if (!valid) return redirectToLogin(request, true);

    return proxyToUpstream(request, ANVIL_DOCS_URL);
  }

  return proxyToUpstream(request, PUBLIC_DOCS_URL);
}

export const config = {
  matcher: [
    '/anvil',
    '/anvil/:path*',
    '/kindling',
    '/kindling/:path*',
    '/aps',
    '/aps/:path*',
    '/edda-stack',
    '/edda-stack/:path*',
    '/blog',
    '/blog/:path*',
    '/assets/:path*',
    '/img/:path*',
  ],
};
