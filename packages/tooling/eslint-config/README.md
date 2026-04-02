# @eddacraft/anvil-eslint-config

Shared ESLint flat configuration for the Anvil monorepo. Provides composable
config layers for base rules, TypeScript, and React.

## Status

Active

## Usage

```js
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
  ...reactConfig // optional, for React packages
);
```

Or use the default (base + TypeScript):

```js
import defaultConfig from '@eddacraft/anvil-eslint-config';

export default defaultConfig;
```

## API Surface

| Export                                      | Description                        |
| ------------------------------------------- | ---------------------------------- |
| `@eddacraft/anvil-eslint-config`            | Default config (base + TypeScript) |
| `@eddacraft/anvil-eslint-config/base`       | Base linting rules                 |
| `@eddacraft/anvil-eslint-config/typescript` | TypeScript-specific rules          |
| `@eddacraft/anvil-eslint-config/react`      | React-specific rules               |

## Consumers

- All packages and apps in the monorepo
