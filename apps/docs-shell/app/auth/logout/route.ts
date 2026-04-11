import { NextResponse, type NextRequest } from 'next/server';

export const runtime = 'nodejs';

const COOKIE_NAME = 'anvil-docs-session';

export async function GET(request: NextRequest) {
  const url = new URL(request.url);
  const response = NextResponse.redirect(new URL('/', url.origin), 302);
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
