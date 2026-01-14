/**
 * React ESLint configuration for Anvil monorepo
 * @module @anvil/eslint-config/react
 */

import reactPlugin from 'eslint-plugin-react';

/**
 * React-specific ESLint rules
 */
export const reactConfig = [
  {
    files: ['**/*.jsx', '**/*.tsx'],
    ...reactPlugin.configs.flat.recommended,
    settings: {
      react: {
        version: 'detect',
      },
    },
  },
];

export default reactConfig;
