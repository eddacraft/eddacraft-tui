# Tooling Packages

Build and development tooling configurations for the Anvil monorepo.

## Structure

```
tooling/
├── eslint-config/   # @anvil/eslint-config - Shared ESLint configurations
├── tsconfig/        # @anvil/tsconfig - Shared TypeScript configurations
└── release/         # (future) Release and versioning utilities
```

## @anvil/eslint-config

Shared ESLint configuration with base, TypeScript, and React presets.

### Usage

```javascript
// eslint.config.mjs
import { baseConfig, typescriptConfig, reactConfig } from '@anvil/eslint-config';
import typescriptEslint from 'typescript-eslint';

export default typescriptEslint.config(
  ...baseConfig,
  ...typescriptConfig,
  ...reactConfig, // optional, for React projects
);
```

### Exports

- `@anvil/eslint-config` - Default config (base + TypeScript)
- `@anvil/eslint-config/base` - Base JavaScript + Prettier rules
- `@anvil/eslint-config/typescript` - TypeScript-specific rules
- `@anvil/eslint-config/react` - React-specific rules

## @anvil/tsconfig

Shared TypeScript configurations for different project types.

### Usage

```json
{
  "extends": "@anvil/tsconfig/lib.json",
  "compilerOptions": {
    "outDir": "./dist",
    "rootDir": "./src"
  }
}
```

### Exports

- `@anvil/tsconfig/base.json` - Base configuration for all projects
- `@anvil/tsconfig/lib.json` - Library projects (with declarations)
- `@anvil/tsconfig/app.json` - Application projects (no declarations)
- `@anvil/tsconfig/node.json` - Node.js projects
- `@anvil/tsconfig/react.json` - React projects
