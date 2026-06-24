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
const BUILD_MARKER = '2026-05-04-explicit-content-length';

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
  // Node 22's undici fetch transparently decompresses the upstream
  // response body, so the upstream's `content-encoding` and
  // `content-length` headers no longer match the bytes we forward.
  // Leaving them in place produced an empty response from the edge
  // (browsers and Vercel's outer edge both saw a content-length /
  // body-bytes mismatch and dropped the body). Strip them and let
  // the runtime emit the correct length for the decoded payload.
  'content-encoding',
  'content-length',
  'transfer-encoding',
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

    // Buffer the body so we can both forward it and report its length via
    // a debug header. Both streaming `response.body` through into a new
    // Response *and* buffering via arrayBuffer were observed producing
    // empty 200 responses on `docs.eddacraft.ai` even after stripping
    // content-encoding/content-length, so we explicitly buffer to a
    // Uint8Array, attach a fresh content-length, and surface diagnostic
    // headers so the next regression can be traced live.
    const buf =
      response.status === 204 || response.status === 304
        ? null
        : new Uint8Array(await response.arrayBuffer());

    if (buf) {
      responseHeaders.set('content-length', String(buf.byteLength));
    }
    responseHeaders.set('x-docs-shell-build', BUILD_MARKER);
    responseHeaders.set('x-docs-shell-upstream-status', String(response.status));
    responseHeaders.set('x-docs-shell-upstream-bytes', String(buf?.byteLength ?? 0));

    return new Response(buf, {
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
    // SPIKE (docs-public-astro): Starlight's Pagefind search fetches its index
    // and WASM from `/pagefind/*`. Without this prefix the proxy 404s them and
    // search silently breaks behind the shell. Astro's hashed assets are kept
    // under `/assets/` (build.assets) so no `/_astro/` entry is needed.
    '/pagefind/:path*',
  ],
};
