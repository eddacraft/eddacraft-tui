# @eddacraft/anvil-tsconfig

Shared TypeScript configurations for the Anvil monorepo. Provides preset configs
for different package types.

## Status

Active

## Configurations

| Config | Description |
| --- | --- |
| `base.json` | Strict base settings shared by all packages |
| `lib.json` | Library packages (declaration emit, ESM output) |
| `app.json` | Application packages (apps/, no declaration emit) |
| `node.json` | Node.js targets (CLI tools, API servers) |
| `react.json` | React packages (JSX transform, DOM types) |

## Usage

```jsonc
// tsconfig.json
{
  "extends": "@eddacraft/anvil-tsconfig/lib.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src"]
}
```

## Consumers

- All TypeScript packages and apps in the monorepo
