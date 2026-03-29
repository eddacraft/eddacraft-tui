# EddaCraft

[![CI](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml/badge.svg)](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml)
[![NX](https://img.shields.io/badge/managed%20with-Nx-143055.svg?style=flat-square)](https://nx.dev/)
[![pnpm](https://img.shields.io/badge/maintained%20with-pnpm-cc00ff.svg?style=flat-square)](https://pnpm.io/)
[![TypeScript](https://img.shields.io/badge/TypeScript-6.0-blue.svg?style=flat-square)](https://www.typescriptlang.org/)
[![Node.js](https://img.shields.io/badge/Node.js->=24-339933.svg?style=flat-square&logo=node.js&logoColor=white)](https://nodejs.org/)

EddaCraft monorepo. Currently home to **Anvil** — a deterministic development
automation platform that catches architecture drift and AI anti-patterns at file
save, before they reach code review.

Current trust/provenance direction includes line-level authorship attribution
planning (human/AI/mixed/unknown + model metadata + confidence), tracked in APS
module `LAC` and governed by ADR-014 (TypeScript vs Rust allocation tree).

## Vision

Anvil ensures that AI and humans cannot produce unsafe software.

AI can generate code, infrastructure, and decisions at unprecedented speed.
Anvil acts as a real-time control layer, intercepting and validating changes as
they are created.

It prevents:

- Anti-patterns
- Security risks
- Policy violations

Before they are ever executed.

Only correct, compliant, and safe outcomes are allowed to proceed.

## Repository Structure

This is an NX-managed pnpm workspace containing the following apps, packages,
and tooling.

### Apps

| Directory        | Package                    | Description                              | Deployment          |
| ---------------- | -------------------------- | ---------------------------------------- | ------------------- |
| `apps/anvil-cli` | `@eddacraft/anvil-cli`     | CLI application (Commander.js, legacy — see `crates/anvil-cli/` for Rust CLI) | npm (`publish.yml`) |
| `apps/docs-site` | `@eddacraft/docs-site`     | Docusaurus documentation site            | Vercel              |
| `apps/website`   | `@eddacraft/anvil-website` | Marketing website (Next.js)              | Vercel              |
| `apps/anvil-api` | —                          | API service                              | Vercel              |
| `apps/e2e`       | —                          | End-to-end test suites (Playwright)      | —                   |

### Packages — Anvil Core

| Directory                  | Package                      | Description                                               |
| -------------------------- | ---------------------------- | --------------------------------------------------------- |
| `packages/anvil/contracts` | `@eddacraft/anvil-contracts` | Schemas, types, and events with zero dependencies         |
| `packages/anvil/ports`     | `@eddacraft/anvil-ports`     | Interface definitions depending only on contracts         |
| `packages/anvil/core`      | `@eddacraft/anvil-core`      | Pure domain logic depending on ports and contracts        |
| `packages/anvil/runtime`   | `@eddacraft/anvil-runtime`   | Orchestration and I/O depending on core, ports, contracts |
| `packages/anvil/policy`    | `@eddacraft/anvil-policy`    | OPA/Rego wrappers depending on contracts                  |

### Packages — Platform

| Directory                   | Package                             | Description                                  |
| --------------------------- | ----------------------------------- | -------------------------------------------- |
| `packages/platform/config`  | `@eddacraft/anvil-platform-config`  | Configuration loading and validation         |
| `packages/platform/storage` | `@eddacraft/anvil-platform-storage` | File system and persistence abstractions     |
| `packages/platform/crypto`  | `@eddacraft/anvil-platform-crypto`  | Hashing, signing, and verification utilities |

### Packages — Ecosystem

| Directory                       | Package                                 | Description                                       |
| ------------------------------- | --------------------------------------- | ------------------------------------------------- |
| `packages/adapters`             | `@eddacraft/anvil-adapters`             | Format converters (SpecKit, BMAD)                 |
| `packages/aps`                  | `@eddacraft/anvil-aps`                  | APS document parser                               |
| `packages/eslint-plugin-anvil`  | `eslint-plugin-anvil`                   | ESLint rules for test quality enforcement         |
| `packages/vscode-extension`     | `anvil-vscode`                          | VS Code integration                               |
| `packages/kindling-integration` | `@eddacraft/anvil-kindling-integration` | Kindling memory integration contracts             |
| `packages/edda-stack`           | `@eddacraft/anvil-edda-stack`           | Observation, proposal, and memory lifecycle stack |
| `packages/mcp-server`           | `@eddacraft/anvil-mcp-server`           | MCP tools, resources, and prompts                 |

### Packages — Tooling

| Directory                        | Description                      |
| -------------------------------- | -------------------------------- |
| `packages/tooling/tsconfig`      | Shared TypeScript configurations |
| `packages/tooling/eslint-config` | Shared ESLint configurations     |

### Crates (Rust)

| Directory                   | Description                                               |
| --------------------------- | --------------------------------------------------------- |
| `crates/anvil-cli`          | Native CLI binary (cross-platform: macOS, Linux, Windows) |
| `crates/anvil-kernel`       | Rust kernel — watcher, parser, semantic graph, policy     |
| `crates/anvil-kernel-types` | Shared types for the Rust kernel (events, graph, trust)   |
| `crates/anvil-tui`          | Ratatui TUI surfaces (dashboard, wizard, gate explorer)   |
| `crates/anvil-checks`       | Gate checks ported to Rust (secret scan, anti-pattern)    |
| `crates/eddacraft-tui`      | Shared Ratatui component library                          |
| `crates/spike`              | Validation spikes for tree-sitter, notify-rs, petgraph    |

### Tools

| Directory          | Description               |
| ------------------ | ------------------------- |
| `tools/scripts`    | Build and utility scripts |
| `tools/generators` | NX code generators        |
| `tools/codemods`   | Codemod transformations   |

### Plans

| Directory         | Description                     |
| ----------------- | ------------------------------- |
| `plans/modules`   | APS module specs and work items |
| `plans/decisions` | Architecture decision records   |
| `plans/execution` | Step-level execution evidence   |

## Getting Started

### Prerequisites

- **Node.js** >= 24
- **pnpm** >= 10.20.0

### Setup

```bash
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001
pnpm install
pnpm build
```

### Common Commands

```bash
# Build all packages
pnpm build

# Run tests
pnpm test

# Type checking
pnpm typecheck

# Linting
pnpm lint

```

NX is used under the hood — you can also use `npx nx` commands directly for
targeted builds, affected-only runs, and task graph visualisation.

## Test Coverage

> Last measured: 2026-02-26 · commit `8663d83` — percentages below may be stale;
> run `pnpm nx run-many -t test --coverage` for current numbers.

Coverage reflects unit and integration tests only (v8 provider). E2E tests (CLI
E2E, TUI E2E) run separately and do not contribute to line coverage.

| Project                                 |     Lines |    Branch |                     Test Files | Types                  |
| --------------------------------------- | --------: | --------: | -----------------------------: | ---------------------- |
| `@eddacraft/anvil-cli`                  |     52.5% |     45.3% | 63 unit, 3 integ, 5 e2e, 3 tui | Unit, Integration, E2E |
| `@eddacraft/anvil-api`                  |     77.9% |     58.1% |                              3 | Unit                   |
| `@eddacraft/anvil-aps`                  |     96.6% |     85.0% |                              8 | Unit                   |
| `@eddacraft/anvil-adapters`             |     83.4% |     70.9% |                             12 | Unit                   |
| `@eddacraft/anvil-edda-stack`           |     42.8% |     25.8% |                              5 | Unit                   |
| `@eddacraft/anvil-kindling-integration` |     43.8% |     23.4% |                              1 | Unit                   |
| `@eddacraft/anvil-mcp-server`           |     43.5% |     36.6% |                             12 | Unit                   |
| `anvil-vscode`                          |     62.5% |     43.9% |                              7 | Unit                   |
| `eslint-plugin-anvil`                   |    --[^1] |    --[^1] |                              3 | Unit                   |
| `contracts`                             |      100% |      100% |                              1 | Unit                   |
| `ports`                                 |   N/A[^2] |   N/A[^2] |                              0 | --                     |
| `core`                                  |     83.4% |     73.3% |                             35 | Unit                   |
| `runtime`                               |     60.3% |     53.0% |               24 unit, 2 integ | Unit, Integration      |
| `policy`                                |     76.4% |     67.2% |                              5 | Unit                   |
| `platform-config`                       |      100% |      100% |                              2 | Unit                   |
| `platform-storage`                      |     90.5% |     79.2% |                              1 | Unit                   |
| `platform-crypto`                       |    0%[^3] |    0%[^3] |                              0 | --                     |
| **Monorepo total**                      | **64.0%** | **53.8%** |                        **176** |                        |

[^1]:
    `eslint-plugin` tests run via NX project-level config, not the root vitest
    config.

[^2]: `ports` contains pure interface definitions — no executable code to cover.

[^3]: `platform-crypto` has no tests yet.

### Test type breakdown

| Type        | Files | Description                                            |
| ----------- | ----: | ------------------------------------------------------ |
| Unit        |   171 | Co-located `*.test.ts` — mocked deps, fast             |
| Integration |     5 | `*-integration.test.ts` — multi-module, in-process     |
| CLI E2E     |     5 | `*.e2e.test.ts` — `execFile`/`spawn`-based CLI testing |
| TUI E2E     |     — | Migrated to Ratatui snapshot tests (`crates/anvil-tui/`) |

### Running coverage

```bash
# Per project (via Nx)
pnpm nx test <project-name> --coverage

# Full monorepo (via Nx — runs all project-level vitest configs)
pnpm nx run-many -t test --coverage

# Root vitest config only (excludes eslint-plugin-anvil — see ^1)
pnpm vitest run --coverage
```

Coverage output is written to the root `coverage/` directory (HTML, JSON, and
JSON summary), which is the path used by the built-in coverage gate check.

## Rust Kernel Benchmarks

The Rust kernel (`anvil-kernel`) includes Criterion micro-benchmarks for
regression detection. These validate the performance targets defined in the
[Kernel Spec](./docs/architecture/rust-kernel-spec.md).

### Performance Targets

| Metric                                    | Target      | Status                          |
| ----------------------------------------- | ----------- | ------------------------------- |
| Cold graph build (100k LOC / ~2000 files) | < 3 seconds | Pending validation at scale     |
| Incremental update (single file)          | < 100ms     | Validated (micro-bench)         |
| Event emission overhead                   | < 10ms      | Validated (micro-bench)         |
| Memory footprint (medium repo)            | < 500MB     | Pending stress test             |
| File detection latency (p99)              | < 20ms      | Validated (spike)               |
| tree-sitter parse (single file)           | < 1ms       | Validated (spike + micro-bench) |

### Benchmark Groups

| Group                       | What it measures                                   | Scale                                |
| --------------------------- | -------------------------------------------------- | ------------------------------------ |
| `cold_graph_build`          | Full scan → parse → graph build                    | 10, 50, 100, 500, 1k, 5k files       |
| `incremental_update`        | Reparse + graph delta for single file change       | 1 file                               |
| `incremental_update_varied` | Parse + graph update for files of varying size     | 10, 100, 500, 1000 LOC               |
| `policy_evaluation`         | All H1 invariants evaluated on one delta           | 1 delta, 4 invariants                |
| `policy_scaling`            | Policy evaluation with varied invariant/delta size | 4–50 invariants × 1–50 symbol deltas |
| `event_emission`            | 1000 progress events through mpsc channel          | 1000 events                          |
| `graph_query`               | `symbols_in_file` and `outgoing_edges` lookups     | 1k, 5k, 10k node graphs              |
| `debouncer_throughput`      | Record + tick cycle under burst and backpressure   | 100, 500, 1000 pending changes       |

### Running Benchmarks

```bash
# Run all Criterion micro-benchmarks
cargo bench --bench kernel

# Run a specific group
cargo bench --bench kernel -- cold_graph_build
cargo bench --bench kernel -- incremental_update
cargo bench --bench kernel -- incremental_update_varied
cargo bench --bench kernel -- policy_evaluation
cargo bench --bench kernel -- policy_scaling
cargo bench --bench kernel -- event_emission
cargo bench --bench kernel -- graph_query
cargo bench --bench kernel -- debouncer_throughput
```

Criterion produces HTML reports in `target/criterion/` — open
`target/criterion/report/index.html` for detailed charts and comparison against
previous runs.

### Planned Extensions

The [Kernel Benchmarking Spec](./docs/architecture/kernel-benchmarking-spec.md)
defines a stress-test harness (`anvil-bench`) for capacity discovery — watcher
saturation, graph memory ceiling, incremental throughput under sustained load,
and cold start scaling. See the
[BENCH module](./plans/modules/kernel-benchmarking.aps.md) for Phase 2 and Phase
3 work items.

## Deployment

| App                  | Platform        | Trigger                                               |
| -------------------- | --------------- | ----------------------------------------------------- |
| `anvil` (Rust)       | GitHub Releases | Git tag (`v*`) via `release.yml` (cargo-dist)         |
| `anvil-cli` (legacy) | npm             | Git tag (`v*`) via `publish.yml` GitHub Action        |
| `docs-site`          | Vercel          | Push to `main` (automatic via Vercel Git integration) |
| `website`            | Vercel          | Push to `main` (automatic via Vercel Git integration) |
| `anvil-api`          | Vercel          | Push to `main` (automatic via Vercel Git integration) |

### Native Binary Targets

The Rust CLI is built for the following platforms via cargo-dist:

| Platform | Architecture            | Binary      |
| -------- | ----------------------- | ----------- |
| macOS    | x86_64                  | `anvil`     |
| macOS    | aarch64 (Apple Silicon) | `anvil`     |
| Linux    | x86_64                  | `anvil`     |
| Linux    | aarch64                 | `anvil`     |
| Windows  | x86_64                  | `anvil.exe` |
| Windows  | aarch64                 | `anvil.exe` |

## CI/CD

The repository has several GitHub Actions workflows:

- **ci.yml** — Lint, typecheck, test, and build on every push and PR. Runs
  against Node.js 20.x and 22.x with smart change detection (docs-only changes
  skip code tests).
- **publish.yml** — Publishes `@eddacraft/anvil-cli` to npm on version tags
  (`v*`) by default (beta tags are forced CLI-only). Workspace package
  publishing is manual/explicit only. Validates tag/package version alignment,
  runs the full test suite, and creates a GitHub release.
- **security.yml** — SAST (Semgrep), dependency audit, secret scan, and licence
  compliance on every PR.
- **labeler.yml** — Automatic PR labelling based on changed paths.
- **infra.yml** — Infrastructure provisioning and validation.
- **bench.yml** — Rust kernel benchmark runs.
- **codeql.yml** — GitHub CodeQL static analysis.
- **rust.yml** — Rust CI (clippy, test, format).

A reusable **Anvil Check** GitHub Action is also provided at
`.github/actions/anvil-check/` for running Anvil analysis in your own workflows.

## Code Conventions

- **UK English** — organise, colour, behaviour
- **ESM with .js extensions** — `import { foo } from './bar.js'`
- **Zod-first schemas** — Define with Zod, export inferred types
- **Tests co-located** — `file.ts` + `file.test.ts`

## Contributing

1. Fork and clone
2. Create feature branch: `git checkout -b feature/my-feature`
3. Make changes, run `pnpm test && pnpm typecheck && pnpm lint`
4. Open PR

See [AGENTS.md](./AGENTS.md) for AI-assisted development instructions.

## Documentation

| Document                                                                | Description                                 |
| ----------------------------------------------------------------------- | ------------------------------------------- |
| [Quick Start](./docs/public/anvil/quickstart.md)                        | Get running in 5 minutes                    |
| [CLI Reference](./apps/anvil-cli/README.md)                             | Complete command reference                  |
| [First Project](./docs/public/anvil/first-project.md)                   | Real-world setup example                    |
| [Troubleshooting](./docs/public/anvil/operations/troubleshooting.md)    | Common issues and solutions                 |
| [Configuration](./docs/public/anvil/operations/config.md)               | Configuration options                       |
| [Architecture](./docs/architecture/overview.md)                         | System design                               |
| [Release Runbook](./docs/guides/release-runbook.md)                     | Safe CLI release checklist                  |
| [Plans](./plans/index.aps.md)                                           | Detailed roadmap                            |
| [LAC Module](./plans/modules/lineage-authorship-confidence.aps.md)      | Line-level authorship + confidence planning |
| [ADR-014](./plans/decisions/014-language-allocation-tree-ts-vs-rust.md) | TS vs Rust language allocation policy       |
