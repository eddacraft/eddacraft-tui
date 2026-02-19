/**
 * esbuild bundling config for anvil-cli
 *
 * Bundles all workspace packages and pure-JS npm deps into a single
 * dist/index.js so the published npm package is self-contained.
 */

import esbuild from 'esbuild';
import { readFileSync } from 'node:fs';

const pkg = JSON.parse(readFileSync('./package.json', 'utf-8'));

await esbuild.build({
  entryPoints: ['src/index.ts'],
  bundle: true,
  platform: 'node',
  target: 'node20',
  format: 'esm',
  outfile: 'dist/index.js',
  banner: {
    js: [
      '#!/usr/bin/env node',
      "import { createRequire as __createRequire } from 'module';",
      'const require = __createRequire(import.meta.url);',
      "import { fileURLToPath as __fileURLToPath } from 'url';",
      "import { dirname as __dirnameFn } from 'path';",
      'const __filename = __fileURLToPath(import.meta.url);',
      'const __dirname = __dirnameFn(__filename);',
    ].join('\n'),
  },
  define: {
    __CLI_VERSION__: JSON.stringify(pkg.version),
  },
  external: [
    // Kindling has native deps (better-sqlite3) — keep external.
    // The meta-package @eddacraft/kindling provides kindling-core,
    // kindling-store-sqlite, and kindling-provider-local as transitive deps.
    '@eddacraft/kindling*',
  ],
  alias: {
    // Ink optional dev dep — stub out to avoid missing-module error at runtime
    'react-devtools-core': './src/stubs/react-devtools-core.ts',
  },
  jsx: 'automatic',
  sourcemap: true,
  logLevel: 'info',
});
