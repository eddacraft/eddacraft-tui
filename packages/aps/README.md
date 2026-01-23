# @eddacraft/anvil-aps

Anvil Planning Spec (APS) library for parsing, loading, validating, and managing
planning documents.

## Overview

The `@eddacraft/anvil-aps` package provides a complete toolkit for working with
Anvil Planning Spec documents:

- **Parser**: Parse Markdown-based planning documents using remark AST
- **Loader**: Load and resolve planning document graphs with dependency tracking
- **Validator**: Validate planning documents against APS rules
- **State**: Manage task state and locking for concurrent execution

## Installation

```bash
pnpm add @eddacraft/anvil-aps
```

## Usage

Coming soon - see `docs/` directory for detailed documentation.

## Documentation

- [APS Planning Spec](./docs/APS-Planning-Spec-v0.1.md) - Full specification
- [APS Conventions](./docs/APS-Conventions.md) - Markdown conventions and
  patterns
- [Non-Goals](./docs/APS-NonGoals.md) - What APS explicitly does not do
- [Anvil Integration](./docs/APS-Anvil-Integration.md) - How APS integrates with
  Anvil CLI

## Development

```bash
# Build the package
nx build aps

# Run tests
nx test aps

# Run tests in watch mode
nx test aps --watch

# Type check
pnpm typecheck

# Lint
nx lint aps
```

## Licence

Copyright (c) 2026 EddaCraft. All rights reserved. See [LICENSE](../../LICENSE)
for details.
