/**
 * GET /auth/logout — Clears the session cookie and redirects to /.
 */

const COOKIE_NAME = 'anvil-docs-session';

export default async function handler(request: Request): Promise<Response> {
  const url = new URL(request.url);
  const redirectUrl = new URL('/', url.origin);

  return new Response(null, {
    status: 302,
    headers: {
      Location: redirectUrl.toString(),
      'Set-Cookie': `${COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax`,
    },
  });
}
