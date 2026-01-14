/**
 * Base ESLint configuration for Anvil monorepo
 * @module @anvil/eslint-config/base
 */

import js from '@eslint/js';
import eslintPluginPrettierRecommended from 'eslint-plugin-prettier/recommended';
import globals from 'globals';

/**
 * Base configuration with JavaScript and Prettier rules
 */
export const baseConfig = [
  js.configs.recommended,
  eslintPluginPrettierRecommended,
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
