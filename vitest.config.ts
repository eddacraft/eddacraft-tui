import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'happy-dom',
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
      '@anvil/contracts': resolve(__dirname, './packages/anvil/contracts/src'),
      '@anvil/ports': resolve(__dirname, './packages/anvil/ports/src'),
      '@anvil/core': resolve(__dirname, './packages/anvil/core/src'),
      '@anvil/runtime': resolve(__dirname, './packages/anvil/runtime/src'),
      '@anvil/policy': resolve(__dirname, './packages/anvil/policy/src'),
      '@anvil/cli': resolve(__dirname, './apps/anvil-cli/src'),
      '@anvil/adapters': resolve(__dirname, './packages/adapters/src'),
      '@anvil/aps': resolve(__dirname, './packages/aps/src'),
      '@anvil/edda-stack': resolve(__dirname, './packages/edda-stack/src'),
      vscode: resolve(__dirname, './packages/vscode-extension/src/__mocks__/vscode.ts'),
    },
  },
});
