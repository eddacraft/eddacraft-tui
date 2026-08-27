# Tools

Development tools, generators, and infrastructure for the Anvil monorepo.

## Structure

```
tools/
├── generators/   # @eddacraft/anvil-generators - Nx generators for scaffolding
├── scripts/      # Build and utility scripts
└── test-utils/   # Shared test utilities and helpers
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

## Local agent runner (QoL)

Use the local Codex helper for unattended tasks with logs + completion wake
event:

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
| scripts    | Complete | Root `scripts/` |
| test-utils | Complete | New             |
