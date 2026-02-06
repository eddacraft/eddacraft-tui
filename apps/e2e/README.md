# E2E Test Harness

Cross-surface end-to-end tests for the Anvil platform.

## Surfaces Covered

| Suite | Surface | What it tests |
| --- | --- | --- |
| `smoke/` | **All** | Import/load canary for every package |
| `cli/` | **CLI** | Real `anvil` binary via child\_process |
| `api/` | **API** | Hono routes via built-in request client |
| `core/` | **Core domain** | Plan lifecycle, drift detection across contracts → core → runtime |
| `contracts/` | **Contracts** | Zod schema compatibility, re-export consistency |
| `adapters/` | **Adapters** | Format detection and roundtrip fidelity |
| `mcp/` | **MCP Server** | Server creation, tool registration, config generation |

## Running Tests

```bash
# Run everything
pnpm test:e2e:harness

# Run only smoke tests (fast — run first)
pnpm test:e2e:smoke

# Run a specific surface
pnpm test:e2e:cli
pnpm test:e2e:api
pnpm test:e2e:core

# From within this package
pnpm --filter @eddacraft/anvil-e2e test
pnpm --filter @eddacraft/anvil-e2e test:smoke
```

## Prerequisites

The CLI E2E tests spawn the real binary, so you must build first:

```bash
pnpm build
```

## Architecture

```
apps/e2e/
├── src/
│   ├── helpers/
│   │   ├── cli-runner.ts       # Spawn CLI as child process
│   │   ├── api-client.ts       # Hono test client wrapper
│   │   ├── workspace.ts        # Isolated temp workspace creation
│   │   └── fixtures.ts         # Plan, change, and config factories
│   ├── cli/                    # CLI surface tests
│   ├── api/                    # API surface tests
│   ├── core/                   # Cross-package domain tests
│   ├── contracts/              # Schema compatibility tests
│   ├── adapters/               # Format adapter tests
│   ├── mcp/                    # MCP server tests
│   └── smoke/                  # All-surface smoke tests
└── vitest.config.ts
```

### Design Principles

- **Vitest-based** — consistent with the rest of the monorepo
- **Reuses existing utilities** — patterns from `test-workspace.ts`, `tuistory-utils.ts`
- **Process-based CLI testing** — spawns real binary via `child_process.execFile`
- **No live database** — API tests use Hono's `app.request()` with DB mocks
- **Isolated workspaces** — each test gets a temp directory, cleaned up automatically
- **Fast smoke suite** — detects broken imports/exports before deeper tests run

### Relationship to Other Test Suites

| Suite | Location | Runner | Purpose |
| --- | --- | --- | --- |
| Unit tests | Co-located `*.test.ts` | Root vitest.config | Package-level logic |
| TUI E2E | `apps/anvil-cli/src/__tests__/e2e/` | Tuistory / node-pty | Interactive terminal UI |
| Browser E2E | `playwright.config.ts` | Playwright | Website / UI (future) |
| **This harness** | `apps/e2e/` | Vitest (own config) | Cross-surface integration |
