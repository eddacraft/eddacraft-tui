import js from '@eslint/js';
import eslintPluginPrettierRecommended from 'eslint-plugin-prettier/recommended';
import reactPlugin from 'eslint-plugin-react';
import nxPlugin from '@nx/eslint-plugin';
import globals from 'globals';
import typescriptEslint from 'typescript-eslint';
let anvilPlugin;
try {
  anvilPlugin = (await import('eslint-plugin-anvil')).default;
} catch {
  // eslint-plugin-anvil not built yet (fresh clone / CI) — skip anvil rules
  anvilPlugin = null;
}
import unicornPlugin from 'eslint-plugin-unicorn';
// import jsonPlugin from '@eslint/json'; // TODO: Re-enable once compatible with ESLint 9

export default typescriptEslint.config(
  js.configs.recommended,
  ...typescriptEslint.configs.recommended,
  eslintPluginPrettierRecommended,
  {
    ignores: [
      '**/dist/',
      '**/node_modules/',
      '**/.nx/',
      '**/coverage/',
      '**/playwright-report/',
      '**/.next/',
      '**/.vercel/',
      '**/.docusaurus/',
      '**/build/',
      '.claude/',
      'eslint.config.mts',
      '**/*.md',
      '**/next-env.d.ts',
      'vitest.config.js',
      'playwright.config.js',
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
  // Nx dependency checks for package.json files
  {
    files: ['**/package.json'],
    ignores: ['node_modules/**', 'apps/e2e/package.json'],
    plugins: {
      '@nx': nxPlugin,
    },
    languageOptions: {
      parser: (await import('jsonc-eslint-parser')).default,
    },
    rules: {
      '@nx/dependency-checks': [
        'error',
        {
          ignoredFiles: [
            '{projectRoot}/eslint.config.{js,cjs,mjs,ts,cts,mts}',
            '{projectRoot}/vitest.config.{js,ts,mjs,mts}',
            '{projectRoot}/esbuild.config.{js,mjs,ts,mts}',
            '{projectRoot}/**/*.test.ts',
            '{projectRoot}/**/*.spec.ts',
            '{projectRoot}/**/__tests__/**',
            '{projectRoot}/**/__mocks__/**',
          ],
          ignoredDependencies: [
            'vscode',
            '@types/vscode',
            'vitest',
            'react-dom',
            '@eddacraft/anvil-core',
            '@eddacraft/anvil-runtime',
            '@eddacraft/anvil-kindling-integration',
            '@eddacraft/anvil-adapters',
            '@eddacraft/anvil-aps',
          ],
        },
      ],
    },
  },
  // React files
  {
    files: ['**/*.jsx', '**/*.tsx'],
    ...reactPlugin.configs.flat.recommended,
    settings: {
      react: {
        version: 'detect',
      },
    },
    rules: {
      ...reactPlugin.configs.flat.recommended.rules,
      // Not needed with React 17+ automatic JSX runtime (Next.js, Vite, etc.)
      'react/react-in-jsx-scope': 'off',
    },
  },
  {
    files: ['**/*.{ts,tsx,mts,cts}'],
    plugins: {
      unicorn: unicornPlugin,
    },
    languageOptions: {
      parserOptions: {
        project: false,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/explicit-module-boundary-types': 'off',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      // Prefer node: protocol for Node.js built-in imports (e.g., 'node:fs' instead of 'fs')
      'unicorn/prefer-node-protocol': 'error',
    },
  },
  ...(anvilPlugin
    ? [
        {
          files: ['**/*.test.ts', '**/*.spec.ts', '**/vitest.config.ts', '**/jest.config.ts'],
          plugins: {
            anvil: anvilPlugin,
          },
          languageOptions: {
            parserOptions: {
              project: false,
              tsconfigRootDir: import.meta.dirname,
            },
          },
          rules: {
            '@typescript-eslint/no-explicit-any': 'off',
            'no-console': 'off',
            // Anvil test quality rules
            'anvil/no-any-in-tests': 'error',
            'anvil/require-mock-cleanup': 'error',
            'anvil/require-cwd-restoration': 'error',
          },
        },
      ]
    : [
        {
          files: ['**/*.test.ts', '**/*.spec.ts', '**/vitest.config.ts', '**/jest.config.ts'],
          rules: {
            '@typescript-eslint/no-explicit-any': 'off',
            'no-console': 'off',
          },
        },
      ]),
  {
    files: ['apps/anvil-cli/**/*.ts', 'cli/**/*.ts'],
    rules: {
      'no-console': 'off',
    },
  },
  {
    files: ['**/scripts/**/*.ts', 'tools/**/*.ts'],
    rules: {
      'no-console': 'off',
    },
  }
);
