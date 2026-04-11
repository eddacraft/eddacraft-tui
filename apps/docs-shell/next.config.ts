import type { NextConfig } from 'next';

const ANVIL_DOCS_URL = process.env.ANVIL_DOCS_URL ?? 'https://anvil-docs-private.vercel.app';
const PUBLIC_DOCS_URL = process.env.PUBLIC_DOCS_URL ?? 'https://docs-public.vercel.app';

const config: NextConfig = {
  async rewrites() {
    return [
      {
        source: '/anvil/:path*',
        destination: `${ANVIL_DOCS_URL}/anvil/:path*`,
      },
      {
        source: '/kindling/:path*',
        destination: `${PUBLIC_DOCS_URL}/kindling/:path*`,
      },
      {
        source: '/aps/:path*',
        destination: `${PUBLIC_DOCS_URL}/aps/:path*`,
      },
      {
        source: '/edda-stack/:path*',
        destination: `${PUBLIC_DOCS_URL}/edda-stack/:path*`,
      },
      {
        source: '/blog/:path*',
        destination: `${PUBLIC_DOCS_URL}/blog/:path*`,
      },
    ];
  },
  async headers() {
    return [
      {
        source: '/anvil/:path*',
        headers: [{ key: 'X-Robots-Tag', value: 'noindex, nofollow' }],
      },
    ];
  },
};

export default config;
