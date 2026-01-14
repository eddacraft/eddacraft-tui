# Tools

Development tools, generators, codemods, and infrastructure for the Anvil monorepo.

## Structure

```
tools/
├── generators/   # @anvil/generators - Nx generators for scaffolding
├── codemods/     # @anvil/codemods - Code transformation utilities
├── scripts/      # Build and utility scripts (migrate from scripts/)
└── docker/       # Docker configurations and compose files
```

## @anvil/generators

Nx generators for creating new packages in the monorepo.

### Usage

```bash
# Create a new package in any directory
pnpm generate:package <name>

# Create a new anvil core package (contracts, ports, core, runtime, policy, sdk)
pnpm generate:anvil-package <name>
```

### Available Generators

- `@anvil/generators:package` - Create new package in any directory
- `@anvil/generators:anvil-package` - Create new @anvil/* package with proper dependencies

## @anvil/codemods

Code transformation utilities for the monorepo migration.

### Usage

```bash
# Preview import path changes (dry run)
pnpm codemod:imports:dry

# Apply import path changes
pnpm codemod:imports
```

### Available Codemods

- `imports` - Rewrite @anvil/core imports to new package structure
  - `@anvil/core/schema` -> `@anvil/contracts`
  - `@anvil/core/types` -> `@anvil/contracts`
  - `@anvil/core/gate/policy` -> `@anvil/policy`
  - `@anvil/core/cache` -> `@anvil/runtime`
  - etc.

## Migration Status

| Directory  | Status   | Source          |
| ---------- | -------- | --------------- |
| generators | Complete | New (MONO-001)  |
| codemods   | Complete | New (MONO-002)  |
| scripts    | Pending  | Root `scripts/` |
| docker     | Pending  | New             |
