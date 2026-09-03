import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.neon.test.ts'],
    passWithNoTests: false,
    hookTimeout: 60_000,
    testTimeout: 60_000,
  },
});
