export const runtime = 'nodejs';

const BODY = `User-agent: *
Disallow: /anvil/
Disallow: /auth/

Sitemap: https://docs.eddacraft.ai/sitemap.xml
`;

export async function GET() {
  return new Response(BODY, {
    status: 200,
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
}
