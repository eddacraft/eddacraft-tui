# eslint-plugin-anvil

ESLint plugin enforcing Anvil test quality conventions. Catches common mistakes
in test files such as untyped mocks, missing mock cleanup, and unreset working
directories.

## Status

Active -- shippable (npm)

## Installation

```bash
npm install --save-dev eslint-plugin-anvil
```

Requires `eslint >= 8.0.0` as a peer dependency.

## Rules

| Rule                            | Description                                                     | Default |
| ------------------------------- | --------------------------------------------------------------- | ------- |
| `anvil/no-any-in-tests`         | Disallow `any` type assertions in test files                    | warn    |
| `anvil/require-mock-cleanup`    | Require `vi.restoreAllMocks()` or equivalent cleanup            | warn    |
| `anvil/require-cwd-restoration` | Require `process.chdir` restoration after tests that change cwd | warn    |

## Usage

Use the recommended config to enable all rules at `warn` level:

```js
// eslint.config.mjs
import anvil from 'eslint-plugin-anvil';

export default [
  {
    plugins: { anvil },
    rules: anvil.configs.recommended.rules,
  },
];
```

## API Surface

- **Plugin object** with `meta`, `rules`, and `configs.recommended`

## Consumers

- All packages in the Anvil monorepo (via `@eddacraft/anvil-eslint-config`)

## Development

```bash
pnpm --filter eslint-plugin-anvil build
pnpm --filter eslint-plugin-anvil test
```
