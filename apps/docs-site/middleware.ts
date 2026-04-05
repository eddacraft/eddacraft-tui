/**
 * Vercel Routing Middleware — gates /anvil/* behind JWT authentication.
 *
 * Verifies the `anvil-docs-session` cookie contains a valid ES256 JWT
 * signed by the BAUTH API. Unauthenticated requests are redirected to
 * /auth/login. The matcher config restricts this to /anvil/* paths only.
 *
 * Key rotation: deploying a new LICENSE_PUBLIC_KEY requires a
 * redeployment to flush the cached key in edge isolates.
 */

import { jwtVerify, importSPKI } from 'jose';

const COOKIE_NAME = 'anvil-docs-session';

let cachedKey: CryptoKey | null = null;

async function getPublicKey(): Promise<CryptoKey> {
  if (cachedKey) return cachedKey;

  const pem = process.env.LICENSE_PUBLIC_KEY;
  if (!pem) {
    throw new Error('LICENSE_PUBLIC_KEY environment variable is required');
  }

  cachedKey = await importSPKI(pem, 'ES256');
  return cachedKey;
}

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

export default async function middleware(request: Request): Promise<Response | undefined> {
  const url = new URL(request.url);

  const cookieHeader = request.headers.get('cookie');
  const token = getCookie(cookieHeader, COOKIE_NAME);

  if (!token) {
    const loginUrl = new URL('/auth/login', url.origin);
    loginUrl.searchParams.set('next', url.pathname);
    return Response.redirect(loginUrl.toString(), 302);
  }

  try {
    const publicKey = await getPublicKey();
    await jwtVerify(token, publicKey, { algorithms: ['ES256'] });
    // Valid JWT — return undefined to let Vercel serve the static content
    return undefined;
  } catch {
    // Invalid or expired JWT — clear cookie and redirect to login
    const loginUrl = new URL('/auth/login', url.origin);
    loginUrl.searchParams.set('next', url.pathname);
    return new Response(null, {
      status: 302,
      headers: {
        Location: loginUrl.toString(),
        'Set-Cookie': `${COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax`,
      },
    });
  }
}

export const config = {
  matcher: ['/anvil/:path*'],
};
