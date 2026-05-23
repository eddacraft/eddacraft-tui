# Monorepo Structure

This document describes a historical monorepo migration plan plus the archived
target shape it was aiming for. For the live repository layout, prefer the root
`README.md`, `apps/README.md`, and `plans/index.aps.md`.

## Historical Current Structure

```
anvil/
├── crates/                  # Rust workspace (Cargo.toml at root)
│   ├── anvil-cli/           # Rust CLI binary — primary entry point
│   ├── anvil-kernel/        # Watcher, parser, semantic graph, policy engine
│   ├── anvil-kernel-types/  # Shared type contracts (events, graph, trust)
│   ├── anvil-tui/           # Ratatui TUI surfaces (watch, gate, wizard, etc.)
│   ├── anvil-checks/        # Ported gate checks (secret, antipattern, command safety)
│   ├── anvil-policy/        # OPA policy evaluation engine
│   ├── anvil-architecture/  # Architecture enforcement (boundaries, drift)
│   ├── anvil-bench/         # Stress-test harness and benchmarks
│   └── spike/               # Phase 0 validation spikes (tree-sitter, notify, petgraph)
├── apps/                    # Deployable applications (TypeScript)
│   ├── anvil-cli/           # Legacy Node.js CLI at the time of this plan
│   ├── anvil-api/
│   ├── website/
│   ├── docs-site/
│   └── e2e/
├── packages/                # TypeScript domain packages
│   ├── anvil/
│   ├── adapters/
│   ├── aps/
│   ├── edda-stack/
│   ├── eslint-plugin-anvil/
│   ├── kindling-integration/
│   ├── mcp-server/
│   ├── platform/
│   ├── tooling/
│   ├── shared/
│   └── vscode-extension/
├── tools/                   # Generators and scripts
├── docs/                    # Internal documentation
└── plans/                   # APS planning specs
```

## Historical Target Structure (v1.1+ at the Time)

```
anvil/
├── crates/                      # Rust workspace
│   ├── anvil-cli/              # Rust CLI binary (primary)
│   ├── anvil-kernel/           # Core engine
│   ├── anvil-kernel-types/     # Shared type contracts
│   ├── anvil-tui/              # TUI surfaces (Ratatui)
│   ├── anvil-checks/           # Gate checks (Rust)
│   ├── anvil-policy/           # OPA policy engine
│   ├── anvil-architecture/     # Architecture enforcement
│   └── anvil-bench/            # Benchmarks
│
├── apps/                        # Deployable applications (TypeScript)
│   ├── anvil-api/              # REST API (Hono + Vercel)
│   ├── website/                # Marketing site + dashboard (Next.js)
│   ├── docs-site/              # Public documentation in the pre-cutover plan
│   └── e2e/                    # E2E test suites
│
├── packages/
│   ├── anvil/                  # Core domain (TypeScript)
│   │   ├── contracts/          # Schemas, events, types
│   │   ├── ports/              # Interfaces
│   │   ├── core/               # Pure domain logic
│   │   ├── runtime/            # Orchestration
│   │   ├── policy/             # OPA/Rego wrappers
│   │   └── sdk/                # Client SDK
│   │
│   ├── edda-stack/             # Memory/proposal system (single package)
│   │
│   ├── adapters/               # Per-integration adapters
│   ├── mcp-server/             # MCP tools, resources, prompts
│   ├── platform/               # Cross-cutting infrastructure in the old plan
│   └── tooling/                # Build configurations
│
├── tools/                      # Nx generators and scripts
├── docs/                       # Internal documentation
└── plans/                      # APS planning specs
```

## Migration Status

The table below is the most useful section of this document for current readers;
it reflects which elements from the old plan are now in place, legacy, or
archived.

| Component           | Current                      | Target                       | Status   |
| ------------------- | ---------------------------- | ---------------------------- | -------- |
| CLI (Rust)          | `crates/anvil-cli/`          | `crates/anvil-cli/`          | In place |
| Kernel              | `crates/anvil-kernel/`       | `crates/anvil-kernel/`       | In place |
| TUI                 | `crates/anvil-tui/`          | `crates/anvil-tui/`          | In place |
| Checks (Rust)       | `crates/anvil-checks/`       | `crates/anvil-checks/`       | In place |
| Policy (Rust)       | `crates/anvil-policy/`       | `crates/anvil-policy/`       | In place |
| Architecture (Rust) | `crates/anvil-architecture/` | `crates/anvil-architecture/` | In place |
| Core (TS)           | `packages/anvil/*`           | `packages/anvil/*`           | In place |
| Adapters            | `packages/adapters/`         | `packages/adapters/*`        | In place |
| API                 | `apps/anvil-api/`            | `apps/anvil-api/`            | In place |
| Admin CLI           | `apps/admin-cli/`            | `apps/admin-cli/`            | In place |
| Website             | `apps/website/`              | `apps/website/`              | In place |
| Docs (public)       | `apps/docs-public/`          | `apps/docs-public/`          | In place |
| Docs (shell)        | `apps/docs-shell/`           | `apps/docs-shell/`           | In place |
| Docs (internal)     | `apps/anvil-docs-private/`   | `apps/anvil-docs-private/`   | In place |
| Docs site (legacy)  | `apps/docs-site/`            | —                            | Legacy   |
| E2E                 | `apps/e2e/`                  | `apps/e2e/*`                 | In place |
| CLI (Node.js)       | `archive/anvil-cli-node/`    | —                            | Archived |
| Scripts             | `tools/scripts/`             | `tools/scripts/`             | In place |

## Migration Plan

The full migration is planned for v1.1. See:

- `docs/planning/monorepo-cleanup-impact-assessment.md` - Full impact analysis
- `plans/index.aps.md` - Release roadmap

## Workspace Configuration

The repo is a dual-workspace monorepo:

- **Cargo workspace** (`Cargo.toml`) manages Rust crates under `crates/`.
  Edition 2024, `unsafe_code = "forbid"`, clippy `all = "deny"`.
- **pnpm workspace** (`pnpm-workspace.yaml`) manages TypeScript packages under
  `apps/*`, `packages/*`, and `tools/*`.
