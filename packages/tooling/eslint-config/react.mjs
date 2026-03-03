/**
 * React ESLint configuration for Anvil monorepo
 * @module @eddacraft/anvil-eslint-config/react
 */

import eslintReact from '@eslint-react/eslint-plugin';

/**
 * React-specific ESLint rules
 */
export const reactConfig = [
  {
    files: ['**/*.jsx', '**/*.tsx'],
    ...eslintReact.configs['recommended-typescript'],
  },
];

export default reactConfig;
