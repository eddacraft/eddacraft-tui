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
    // Retry flaky E2E tests once (TFIX-009). Real failures still fail on 2nd run.
    retry: 1,
    reporters: ['default', 'json'],
    // outputFile is resolved relative to test.root (`apps/e2e/src`), not the
    // config dir. Three `../` climbs land at the repo root so the JSON report
    // lives alongside the TypeScript coverage summary under /coverage/.
    outputFile: {
      json: '../../../coverage/e2e-results.json',
    },
  },
  resolve: {
    alias: {
      '@helpers': resolve(__dirname, 'src/helpers'),
      // Map workspace packages to source for Vitest resolution
      '@eddacraft/anvil-contracts': resolve(__dirname, '../../packages/anvil/contracts/src'),
      '@eddacraft/anvil-core': resolve(__dirname, '../../packages/anvil/core/src'),
      '@eddacraft/anvil-runtime': resolve(__dirname, '../../packages/anvil/runtime/src'),
      '@eddacraft/anvil-policy': resolve(__dirname, '../../packages/anvil/policy/src'),
      '@eddacraft/anvil-aps': resolve(__dirname, '../../packages/aps/src'),
      '@eddacraft/anvil-adapters': resolve(__dirname, '../../packages/adapters/src'),
      '@eddacraft/anvil-edda-stack': resolve(__dirname, '../../packages/edda-stack/src'),
      '@eddacraft/anvil-api': resolve(__dirname, '../anvil-api/src'),
    },
  },
});
