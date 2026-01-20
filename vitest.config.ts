import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'happy-dom',
    passWithNoTests: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
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
      // New monorepo structure
      'packages/anvil/*/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'apps/anvil-cli/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'packages/adapters/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'packages/vscode-extension/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'packages/aps/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'packages/edda-stack/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
    ],
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/.nx/**',
      '**/playwright-report/**',
      // Docs archive excluded from test discovery
      'docs/archive/**',
    ],
  },
  resolve: {
    alias: {
      // New package structure
      '@eddacraft/anvil-contracts': resolve(__dirname, './packages/anvil/contracts/src'),
      '@eddacraft/anvil-ports': resolve(__dirname, './packages/anvil/ports/src'),
      '@eddacraft/anvil-core': resolve(__dirname, './packages/anvil/core/src'),
      '@eddacraft/anvil-runtime': resolve(__dirname, './packages/anvil/runtime/src'),
      '@eddacraft/anvil-policy': resolve(__dirname, './packages/anvil/policy/src'),
      '@eddacraft/anvil-cli': resolve(__dirname, './apps/anvil-cli/src'),
      '@eddacraft/anvil-adapters': resolve(__dirname, './packages/adapters/src'),
      '@eddacraft/anvil-aps': resolve(__dirname, './packages/aps/src'),
      '@eddacraft/anvil-edda-stack': resolve(__dirname, './packages/edda-stack/src'),
      vscode: resolve(__dirname, './packages/vscode-extension/src/__mocks__/vscode.ts'),
    },
  },
});
