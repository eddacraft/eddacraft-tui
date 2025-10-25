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
      'core/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'cli/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'ui/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'packs/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'packages/adapters/src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
    ],
    exclude: ['**/node_modules/**', '**/dist/**', '**/.nx/**', '**/playwright-report/**'],
  },
  resolve: {
    alias: {
      '@anvil/core': resolve(__dirname, './core/src'),
      '@anvil/cli': resolve(__dirname, './cli/src'),
      '@anvil/ui': resolve(__dirname, './ui/src'),
      '@anvil/packs': resolve(__dirname, './packs/src'),
      '@anvil/adapters': resolve(__dirname, './packages/adapters/src'),
    },
  },
});
