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

```typescript
import { parseDocument, parseIndex } from '@eddacraft/anvil-aps/parser';
import { validatePlanningDoc } from '@eddacraft/anvil-aps/validator';

// Leaf spec (module/task file)
const doc = await parseDocument(leafContent, 'plans/modules/my-feature.aps.md');

// Index file (the root plan)
const index = await parseIndex(indexContent, 'plans/index.aps.md');

// Validate any plan file by path
const result = await validatePlanningDoc('plans/modules/my-feature.aps.md');
```

See [`AGENTS.md`](./AGENTS.md) for the subpath-export map (parser, loader,
validator, state, templates, filter, types).

## Documentation

- [APS Planning Spec](https://github.com/eddacraft/anvil-plan-spec) — the
  canonical specification (Markdown shapes, status vocabulary, validation rule
  names)
- [AGENTS.md](./AGENTS.md) — package layout, subpath exports, validator rule
  list, and authoring conventions for changes inside this package
- [examples/](./examples) — runnable APS fixtures (`feature-auth.aps.md`,
  `refactor-error-handling.aps.md`, `system-ecommerce/`)
- [templates/](./templates) — the canonical templates emitted by
  `generateTemplate()` (`leaf-*.md`, `simple-*.md`, `index-*.md`)

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

Copyright (c) 2026 eddacraft, Inc. All rights reserved. See
[LICENSE](../../LICENSE) for details.
