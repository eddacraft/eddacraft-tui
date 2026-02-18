# Tools

Development tools, generators, codemods, and infrastructure for the Anvil
monorepo.

## Structure

```
tools/
├── generators/   # @eddacraft/anvil-generators - Nx generators for scaffolding
├── codemods/     # @eddacraft/anvil-codemods - Code transformation utilities
├── scripts/      # Build and utility scripts (migrate from scripts/)
└── docker/       # Docker configurations and compose files
```

## @eddacraft/anvil-generators

Nx generators for creating new packages in the monorepo.

### Usage

```bash
# Create a new package in any directory
pnpm generate:package <name>

# Create a new anvil core package (contracts, ports, core, runtime, policy, sdk)
pnpm generate:anvil-package <name>
```

### Available Generators

- `@eddacraft/anvil-generators:package` - Create new package in any directory
- `@eddacraft/anvil-generators:anvil-package` - Create new @eddacraft/anvil-\*
  package with proper dependencies

## @eddacraft/anvil-codemods

Code transformation utilities for the monorepo migration.

### Usage

```bash
# Preview import path changes (dry run)
pnpm codemod:imports:dry

# Apply import path changes
pnpm codemod:imports
```

### Available Codemods

- `imports` - Rewrite @eddacraft/anvil-core imports to new package structure
  - `@eddacraft/anvil-core/schema` -> `@eddacraft/anvil-contracts`
  - `@eddacraft/anvil-core/types` -> `@eddacraft/anvil-contracts`
  - `@eddacraft/anvil-core/gate/policy` -> `@eddacraft/anvil-policy`
  - `@eddacraft/anvil-core/cache` -> `@eddacraft/anvil-runtime`
  - etc.

## Local agent runner (QoL)

Use the local Codex helper for unattended tasks with logs + completion wake event:

```bash
pnpm agent:run "<task prompt>"
# or
bash tools/local-agent-run.sh "<task prompt>"
```

Logs are written to `plans/agent-runs/`.

## Migration Status

| Directory  | Status   | Source          |
| ---------- | -------- | --------------- |
| generators | Complete | New (MONO-001)  |
| codemods   | Complete | New (MONO-002)  |
| scripts    | Pending  | Root `scripts/` |
| docker     | Pending  | New             |
