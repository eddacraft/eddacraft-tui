# Tooling Packages

> **Status:** Placeholder for v1.1+

Build and development tooling configurations.

## Structure

```
tooling/
├── eslint-config/   # Shared ESLint configurations
├── tsconfig/        # Shared TypeScript configurations
└── release/         # Release and versioning utilities
```

## Migration Plan

| New Package   | Source                           |
| ------------- | -------------------------------- |
| eslint-config | Root `eslint.config.js` patterns |
| tsconfig      | Root `tsconfig.*.json` files     |
| release       | New (changeset integration)      |

## Usage

These packages provide consistent configurations across all workspaces.
