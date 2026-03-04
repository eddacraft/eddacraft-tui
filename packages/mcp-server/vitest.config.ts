import { defineConfig } from 'vitest/config';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const monorepoRoot = resolve(__dirname, '../..');

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: ['node_modules', 'dist', 'src/**/*.d.ts', 'src/index.ts', 'src/**/index.ts'],
    },
  },
  resolve: {
    alias: {
      '@eddacraft/anvil-contracts': resolve(monorepoRoot, 'packages/anvil/contracts/src'),
      '@eddacraft/anvil-ports': resolve(monorepoRoot, 'packages/anvil/ports/src'),
      '@eddacraft/anvil-core': resolve(monorepoRoot, 'packages/anvil/core/src'),
      '@eddacraft/anvil-runtime': resolve(monorepoRoot, 'packages/anvil/runtime/src'),
      '@eddacraft/anvil-policy': resolve(monorepoRoot, 'packages/anvil/policy/src'),
    },
  },
});
