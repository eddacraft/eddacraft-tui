import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    passWithNoTests: true,
  },
  bench: {
    globals: true,
    include: ['benchmarks/**/*.bench.ts'],
  },
});
