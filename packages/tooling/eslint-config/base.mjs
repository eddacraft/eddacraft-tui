/**
 * Base ESLint configuration for Anvil monorepo
 * @module @eddacraft/anvil-eslint-config/base
 */

import js from '@eslint/js';
import globals from 'globals';

/**
 * Base configuration with JavaScript rules
 */
export const baseConfig = [
  js.configs.recommended,
  {
    ignores: [
      '**/dist/',
      '**/node_modules/',
      '**/.nx/',
      '**/coverage/',
      '**/playwright-report/',
      '**/*.md',
    ],
  },
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
        ...globals.es2022,
      },
    },
  },
];

export default baseConfig;
