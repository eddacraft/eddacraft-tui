import { defineConfig } from 'vitest/config';
import { resolve } from 'node:path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    passWithNoTests: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'json-summary', 'html'],
      exclude: [
        'node_modules/',
        'dist/',
        '*.config.ts',
        '*.config.js',
        '*.config.mjs',
        '**/*.d.ts',
        '**/*.test.ts',
        '**/*.spec.ts',
        '**/index.ts',
      ],
    },
    include: [
      // New monorepo structure: include tests from all packages by default
      'packages/**/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'apps/anvil-api/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
    ],
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/.nx/**',
      '**/playwright-report/**',
      // Docs archive excluded from test discovery
      'docs/archive/**',
      // E2E tests run in dedicated CI jobs, not the unit test step
      '**/*.e2e.test.*',
      '**/*.e2e.spec.*',
    ],
  },
  resolve: {
    alias: {
      // New package structure
      '@eddacraft/anvil-driver-client': resolve(__dirname, './packages/anvil-driver-client/src'),
      '@eddacraft/anvil-contracts': resolve(__dirname, './packages/anvil/contracts/src'),
      '@eddacraft/anvil-ports': resolve(__dirname, './packages/anvil/ports/src'),
      '@eddacraft/anvil-core': resolve(__dirname, './packages/anvil/core/src'),
      '@eddacraft/anvil-runtime': resolve(__dirname, './packages/anvil/runtime/src'),
      '@eddacraft/anvil-observability': resolve(__dirname, './packages/anvil/observability/src'),
      '@eddacraft/anvil-policy': resolve(__dirname, './packages/anvil/policy/src'),
      '@eddacraft/anvil-adapters': resolve(__dirname, './packages/adapters/src'),
      '@eddacraft/anvil-aps': resolve(__dirname, './packages/aps/src'),
      // @eddacraft/anvil-mcp-server archived under ADR-033 (2026-04-29)
      // → archive/anvil-mcp-server/. Intentionally not aliased here;
      // pnpm-workspace.yaml's '!archive/**' glob already excludes it
      // from package resolution.
      '@eddacraft/anvil-edda-stack': resolve(__dirname, './packages/edda-stack/src'),
      '@eddacraft/anvil-kindling-integration': resolve(
        __dirname,
        './packages/kindling-integration/src'
      ),
      '@eddacraft/shared-storage': resolve(__dirname, './packages/shared/storage/src'),
      '@eddacraft/render': resolve(__dirname, './packages/libs/render/src'),
      // vscode-extension archived under ADR-033 (2026-04-29)
      // → archive/anvil-vscode-extension/. Intentionally not aliased here.
    },
  },
});
