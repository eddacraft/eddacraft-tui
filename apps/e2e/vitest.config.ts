import { defineConfig } from 'vitest/config';
import { resolve } from 'node:path';

export default defineConfig({
  test: {
    globals: true,
    root: resolve(__dirname, 'src'),
    include: ['**/*.e2e.test.ts'],
    testTimeout: 30_000,
    hookTimeout: 15_000,
    pool: 'forks',
    poolOptions: {
      forks: {
        // E2E tests spawn subprocesses; isolate them properly
        singleFork: false,
      },
    },
    reporters: ['default', 'json'],
    outputFile: {
      json: '../../coverage/e2e-results.json',
    },
  },
  resolve: {
    alias: {
      '@helpers': resolve(__dirname, 'src/helpers'),
      // Map workspace packages to source for Vitest resolution
      '@eddacraft/anvil-contracts': resolve(__dirname, '../../packages/anvil/contracts/src'),
      '@eddacraft/anvil-core': resolve(__dirname, '../../packages/anvil/core/src'),
      '@eddacraft/anvil-runtime': resolve(__dirname, '../../packages/anvil/runtime/src'),
      '@eddacraft/anvil-aps': resolve(__dirname, '../../packages/aps/src'),
      '@eddacraft/anvil-adapters': resolve(__dirname, '../../packages/adapters/src'),
      '@eddacraft/anvil-edda-stack': resolve(__dirname, '../../packages/edda-stack/src'),
      '@eddacraft/anvil-mcp-server': resolve(__dirname, '../../packages/mcp-server/src'),
      '@eddacraft/anvil-api': resolve(__dirname, '../anvil-api/src'),
    },
  },
});
