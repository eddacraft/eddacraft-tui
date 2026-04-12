import { timingSafeEqual } from 'node:crypto';

const HEADER_NAME = 'x-docs-upstream-secret';

function safeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) {
    const buf = Buffer.from(a, 'utf-8');
    timingSafeEqual(buf, buf);
    return false;
  }
  return timingSafeEqual(Buffer.from(a, 'utf-8'), Buffer.from(b, 'utf-8'));
}

export default function middleware(request: Request): Response | undefined {
  const secret = process.env.DOCS_UPSTREAM_SECRET;
  if (!secret) {
    return new Response('Service misconfigured', { status: 500 });
  }

  const provided = request.headers.get(HEADER_NAME) ?? '';
  if (!safeEqual(provided, secret)) {
    return new Response('Unauthorized', { status: 401 });
  }

  return undefined;
}

export const config = {
  matcher: ['/((?!assets/|img/|favicon\\.ico).*)'],
};
