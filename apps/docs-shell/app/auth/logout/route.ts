import { NextResponse, type NextRequest } from 'next/server';

export const runtime = 'nodejs';

const COOKIE_NAME = 'anvil-docs-session';

export async function GET() {
  return new NextResponse('Method Not Allowed', {
    status: 405,
    headers: { Allow: 'POST', 'Cache-Control': 'no-store' },
  });
}

export async function POST(request: NextRequest) {
  const url = new URL(request.url);
  const origin = request.headers.get('origin');
  const site = request.headers.get('sec-fetch-site');
  // Require a browser's exact serialised origin. Null/missing origins and
  // sibling subdomains are denied; forwarded host headers are not authority.
  if (origin !== url.origin || (site !== null && site !== 'same-origin')) {
    return new NextResponse('Forbidden', { status: 403, headers: { 'Cache-Control': 'no-store' } });
  }

  const response = NextResponse.redirect(new URL('/', url.origin), 303);
  response.headers.set('Cache-Control', 'no-store');
  response.cookies.set({
    name: COOKIE_NAME,
    value: '',
    path: '/',
    maxAge: 0,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  return response;
}
