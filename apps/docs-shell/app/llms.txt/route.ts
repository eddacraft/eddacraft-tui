export const runtime = 'nodejs';

const BODY = `# EddaCraft Documentation
# Anvil is a commercial product in closed beta. Anvil documentation is private.
# Public sections: /kindling, /aps, /edda-stack, /blog

User-agent: *
Disallow: /anvil/
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
