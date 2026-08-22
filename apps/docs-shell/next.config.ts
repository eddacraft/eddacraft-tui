import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { NextConfig } from 'next';

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

const config: NextConfig = {
  transpilePackages: [
    '@eddacraft/anvil-contracts',
    '@eddacraft/anvil-flags-catalogue',
    '@eddacraft/anvil-runtime',
  ],
  turbopack: {
    root: workspaceRoot,
    resolveAlias: {
      '@eddacraft/anvil-contracts': '../../packages/anvil/contracts/dist/index.js',
      '@eddacraft/anvil-flags-catalogue': '../../packages/anvil/flags-catalogue/dist/index.js',
      '@eddacraft/anvil-runtime/feature-flags':
        '../../packages/anvil/runtime/dist/feature-flags/index.js',
    },
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
