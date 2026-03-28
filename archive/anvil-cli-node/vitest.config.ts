import { defineConfig } from 'vitest/config';
import { resolve } from 'node:path';

export default defineConfig({
  test: {
    root: resolve(__dirname),
    globals: true,
    environment: 'node',
    passWithNoTests: true,
    include: ['src/**/*.{test,spec}.{js,ts,tsx}'],
    exclude: ['**/node_modules/**', '**/dist/**', '**/*.e2e.test.*', '**/*.e2e.spec.*'],
  },
  resolve: {
    alias: {
      '@eddacraft/anvil-contracts': resolve(__dirname, '../../packages/anvil/contracts/src'),
      '@eddacraft/anvil-ports': resolve(__dirname, '../../packages/anvil/ports/src'),
      '@eddacraft/anvil-core': resolve(__dirname, '../../packages/anvil/core/src'),
      '@eddacraft/anvil-runtime': resolve(__dirname, '../../packages/anvil/runtime/src'),
      '@eddacraft/anvil-policy': resolve(__dirname, '../../packages/anvil/policy/src'),
      '@eddacraft/anvil-adapters': resolve(__dirname, '../../packages/adapters/src'),
      '@eddacraft/anvil-aps': resolve(__dirname, '../../packages/aps/src'),
      '@eddacraft/anvil-kindling-integration': resolve(
        __dirname,
        '../../packages/kindling-integration/src'
      ),
      vscode: resolve(__dirname, '../../packages/vscode-extension/src/__mocks__/vscode.ts'),
    },
  },
});
