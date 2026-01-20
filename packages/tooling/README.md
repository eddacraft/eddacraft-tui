# Tooling Packages

Build and development tooling configurations for the Anvil monorepo.

## Structure

```
tooling/
├── eslint-config/   # @eddacraft/anvil-eslint-config - Shared ESLint configurations
├── tsconfig/        # @eddacraft/anvil-tsconfig - Shared TypeScript configurations
└── release/         # (future) Release and versioning utilities
```

## @eddacraft/anvil-eslint-config

Shared ESLint configuration with base, TypeScript, and React presets.

### Usage

```javascript
// eslint.config.mjs
import {
  baseConfig,
  typescriptConfig,
  reactConfig,
} from '@eddacraft/anvil-eslint-config';
import typescriptEslint from 'typescript-eslint';

export default typescriptEslint.config(
  ...baseConfig,
  ...typescriptConfig,
  ...reactConfig // optional, for React projects
);
```

### Exports

- `@eddacraft/anvil-eslint-config` - Default config (base + TypeScript)
- `@eddacraft/anvil-eslint-config/base` - Base JavaScript + Prettier rules
- `@eddacraft/anvil-eslint-config/typescript` - TypeScript-specific rules
- `@eddacraft/anvil-eslint-config/react` - React-specific rules

## @eddacraft/anvil-tsconfig

Shared TypeScript configurations for different project types.

### Usage

```json
{
  "extends": "@eddacraft/anvil-tsconfig/lib.json",
  "compilerOptions": {
    "outDir": "./dist",
    "rootDir": "./src"
  }
}
```

### Exports

- `@eddacraft/anvil-tsconfig/base.json` - Base configuration for all projects
- `@eddacraft/anvil-tsconfig/lib.json` - Library projects (with declarations)
- `@eddacraft/anvil-tsconfig/app.json` - Application projects (no declarations)
- `@eddacraft/anvil-tsconfig/node.json` - Node.js projects
- `@eddacraft/anvil-tsconfig/react.json` - React projects
