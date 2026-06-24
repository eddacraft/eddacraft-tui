import { timingSafeEqual } from 'node:crypto';

import { next } from '@vercel/functions';

// Carried over VERBATIM from apps/docs-public/middleware.ts. This is a Vercel
// *routing* middleware (framework-agnostic — runs at the edge, not an Astro
// integration), so the docs-shell upstream-secret contract survives the
// Docusaurus → Astro swap untouched. Only `vercel.json` framework/output
// settings differ between the two apps.
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

  return next();
}

export const config = {
  runtime: 'nodejs',
  matcher: ['/((?!favicon\\.ico).*)'],
};
