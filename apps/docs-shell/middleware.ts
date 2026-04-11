import { NextResponse, type NextRequest } from 'next/server';
import { verifyLicense } from './lib/jwt';
import { getCookie } from './lib/cookie';

const COOKIE_NAME = 'anvil-docs-session';

function redirectToLogin(request: NextRequest | Request, clearCookie: boolean): NextResponse {
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

export default async function middleware(request: NextRequest | Request): Promise<NextResponse> {
  const cookieHeader = request.headers.get('cookie');
  const token = getCookie(cookieHeader, COOKIE_NAME);

  if (!token) return redirectToLogin(request, false);

  const { valid } = await verifyLicense(token);
  if (!valid) return redirectToLogin(request, true);

  return NextResponse.next();
}

export const config = {
  matcher: ['/anvil/:path*'],
};
