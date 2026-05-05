import { timingSafeEqual } from 'node:crypto';

import { next } from '@vercel/functions';

const HEADER_NAME = 'x-docs-upstream-secret';

function safeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) {
    const buf = Buffer.from(a, 'utf-8');
    timingSafeEqual(buf, buf);
    return false;
  }
  return timingSafeEqual(Buffer.from(a, 'utf-8'), Buffer.from(b, 'utf-8'));
}

export default function middleware(request: Request): Response {
  const secret = process.env.DOCS_UPSTREAM_SECRET;
  if (!secret) {
    return new Response('Service misconfigured', { status: 500 });
  }

  const provided = request.headers.get(HEADER_NAME) ?? '';
  if (!safeEqual(provided, secret)) {
    return new Response('Unauthorized', { status: 401 });
  }

  // Vercel Routing Middleware on a non-Next framework (this app is
  // `docusaurus-2`): returning `undefined` is Next.js-style semantics
  // and on `framework: other` produces a 200 with an empty body. Use
  // `next()` from `@vercel/functions` to actually fall through to the
  // static build output. See
  // https://vercel.com/docs/routing-middleware/api#continuing-the-routing-middleware-chain
  return next();
}

export const config = {
  runtime: 'nodejs',
  matcher: ['/((?!favicon\\.ico).*)'],
};
