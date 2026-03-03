/**
 * React ESLint configuration for Anvil monorepo
 * @module @eddacraft/anvil-eslint-config/react
 */

import eslintReact from '@eslint-react/eslint-plugin';

const recommendedTsConfig = eslintReact.configs['recommended-typescript'] ?? {};

/**
 * React-specific ESLint rules
 */
export const reactConfig = [
  {
    files: ['**/*.jsx', '**/*.tsx'],
    plugins: recommendedTsConfig.plugins,
    rules: recommendedTsConfig.rules,
    languageOptions: recommendedTsConfig.languageOptions,
  },
];

export default reactConfig;
