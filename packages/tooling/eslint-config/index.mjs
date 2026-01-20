/**
 * Shared ESLint configuration for Anvil monorepo
 * @module @eddacraft/anvil-eslint-config
 *
 * Usage:
 *   import { baseConfig, typescriptConfig, reactConfig } from '@eddacraft/anvil-eslint-config';
 *   import typescriptEslint from 'typescript-eslint';
 *
 *   export default typescriptEslint.config(
 *     ...baseConfig,
 *     ...typescriptConfig,
 *     ...reactConfig, // optional
 *   );
 */

export { baseConfig } from './base.mjs';
export { typescriptConfig } from './typescript.mjs';
export { reactConfig } from './react.mjs';

import typescriptEslint from 'typescript-eslint';
import { baseConfig } from './base.mjs';
import { typescriptConfig } from './typescript.mjs';

/**
 * Default configuration combining base and TypeScript rules
 * This is the recommended starting point for most packages
 */
export const defaultConfig = typescriptEslint.config(...baseConfig, ...typescriptConfig);

export default defaultConfig;
