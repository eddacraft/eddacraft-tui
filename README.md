# anvil

<p align="center">
  <img src="apps/website/public/images/anvil-brandmark-ember.svg" alt="anvil brandmark" width="120" />
</p>

> **AI agents make software probabilistic. anvil makes it deterministic.**

anvil enforces policy at generation time, not at review. It sits between
probabilistic AI agents and production code as a deterministic governance layer
that catches architectural drift, anti-patterns, security risks, and policy
violations **before they ever leave the developer's machine.**

**[→ Early access at eddacraft.ai](https://eddacraft.ai)** ·
[Docs](https://docs.eddacraft.ai/anvil/overview) ·
[GTM strategy](https://github.com/eddacraft/eddacraft-gtm) ·
[Brand & design](https://github.com/eddacraft/brand-and-design)

## Hero stats

```
10 µs      save-time check (incremental file update)
800 ns     full policy evaluation, all invariants
14.5 ms    cold graph build, 100-file codebase
0          perceptible delay
```

Measured 2026-04-03 against the Rust kernel via Criterion (100 samples, release
build). Governance overhead is effectively zero — anvil is in a different
category from SAST, not a faster scanner.

See [`crates/anvil-bench/`](./crates/anvil-bench/) for the harness and
[the GTM benchmark report](https://github.com/eddacraft/eddacraft-gtm/blob/main/competitive/anvil-benchmarks-2026-04-03.md)
for marketing-ready proof points.

## What anvil is

**Agentic engineering governance** — a category being defined right now. anvil
is not a SAST scanner, not a linter, not an observability product, not a
compliance dashboard. It is the governance layer that complements and constrains
AI coding tools (Cursor, Copilot, Codex) in real time, in the developer workflow
— _not_ in the PR queue.

For full positioning, ICP definition, competitive intelligence, and the GTM
primer, see
[`eddacraft/eddacraft-gtm`](https://github.com/eddacraft/eddacraft-gtm).

## Why now

The **EU AI Act becomes substantially enforceable on 2 August 2026** — under
four months out. High-risk obligations (Annex III), Article 50 transparency,
conformity assessments, technical documentation, CE marking, and EU database
registration all become required by that date for in-scope systems. Every
EU-exposed engineering team now has a hard deadline to prove their agents are
governed.

At the same time, adjacent funding tells the same story: Qodo closed a $70M
Series B and DAM Secure closed a $4M seed in the same week (April 2026);
Microsoft, GitHub, Cisco, Zapier, Nvidia, and JetBrains are all shipping
adjacent primitives in Q1–Q2 2026. The category is being built right now.

Full market context and the parallel-track ICP framing that follows from this
deadline live in
[`eddacraft-gtm/GTM-PRIMER.md`](https://github.com/eddacraft/eddacraft-gtm/blob/main/GTM-PRIMER.md).

## Related repos

| Repo                                                                          | Purpose                                                                       |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [`eddacraft/eddacraft-gtm`](https://github.com/eddacraft/eddacraft-gtm)       | GTM strategy, positioning, competitive radar, market signals, benchmark proof |
| [`eddacraft/brand-and-design`](https://github.com/eddacraft/brand-and-design) | Visual identity, design system, deck templates, brand assets                  |
| [`eddacraft/anvil-plan-spec`](https://github.com/eddacraft/anvil-plan-spec)   | The APS planning format used throughout this repo                             |

---

## For contributors

[![CI](https://github.com/eddacraft/anvil-001/actions/workflows/ci.yml/badge.svg)](https://github.com/eddacraft/anvil-001/actions/workflows/ci.yml)
[![NX](https://img.shields.io/badge/managed%20with-Nx-143055.svg?style=flat-square)](https://nx.dev/)
[![pnpm](https://img.shields.io/badge/maintained%20with-pnpm-cc00ff.svg?style=flat-square)](https://pnpm.io/)
[![TypeScript](https://img.shields.io/badge/TypeScript-6.0-blue.svg?style=flat-square)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-2024%20edition-DEA584.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Node.js](https://img.shields.io/badge/Node.js->=24-339933.svg?style=flat-square&logo=node.js&logoColor=white)](https://nodejs.org/)

eddacraft monorepo. Currently home to **anvil** — a deterministic development
automation platform that catches architecture drift and AI anti-patterns at file
save, before they reach code review.

Current trust/provenance direction includes line-level authorship attribution
planning (human/AI/mixed/unknown + model metadata + confidence), tracked in APS
module `LAC` and governed by ADR-014 (TypeScript vs Rust allocation tree).

Contributor workflow quick links:

- [Branching strategy](docs/guides/branching-strategy.md) — `main`/`dev` release
  and integration flow
- [Worktree policy](docs/guides/worktree-policy.md) — permanent vs disposable
  worktrees
- [Release runbook](docs/guides/release-runbook.md) — direct promotion vs
  `release/*` stabilisation
- [Contributing](CONTRIBUTING.md) — setup, commands, and submission checklist

## Vision

anvil ensures AI and humans cannot produce unsafe software.

AI generates code, infrastructure, and decisions at unprecedented speed. anvil
acts as a deterministic governance layer in the developer workflow, intercepting
and validating changes at the moment of creation.

It prevents:

- Anti-patterns
- Security risks
- Policy violations

Before they are ever executed. Only correct, compliant, and safe outcomes are
allowed to proceed.

## Repository Structure

This is an NX-managed pnpm workspace containing the following apps, packages,
and tooling.

### Root files

| File                       | Purpose                                                                    |
| -------------------------- | -------------------------------------------------------------------------- |
| `package.json`             | Workspace scripts, shared devDependencies, and package manager constraints |
| `pnpm-workspace.yaml`      | Workspace package discovery for apps, packages, tools, and infra           |
| `nx.json`                  | Nx task and workspace configuration                                        |
| `rust-toolchain.toml`      | Pinned Rust toolchain for workspace crates                                 |
| `dist-workspace.toml`      | `cargo-dist` release configuration for Rust binaries                       |
| `.anvilrc`                 | Repository-level anvil configuration                                       |
| `.node-version` / `.nvmrc` | Node version hints for CI and local tooling                                |

### Apps

| Directory         | Package                    | Description                                                                   | Deployment          |
| ----------------- | -------------------------- | ----------------------------------------------------------------------------- | ------------------- |
| `apps/anvil-cli`  | `@eddacraft/anvil-cli`     | CLI application (Commander.js, legacy — see `crates/anvil-cli/` for Rust CLI) | npm (`publish.yml`) |
| `apps/docs-site`  | `@eddacraft/docs-site`     | Docusaurus documentation site                                                 | Vercel              |
| `apps/website`    | `@eddacraft/anvil-website` | Marketing website (Next.js)                                                   | Vercel              |
| `apps/anvil-api`  | —                          | API service                                                                   | Vercel              |
| `apps/docs-shell` | `@eddacraft/docs-shell`    | Documentation shell (Next.js, auth-gated)                                     | Vercel              |
| `apps/e2e`        | —                          | End-to-end test suites (Playwright)                                           | —                   |

### Packages — anvil core

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
| `packages/json-render`          | `@eddacraft/json-render`                | JSON-driven dashboard renderer                    |
| `packages/transactional`        | `@eddacraft/transactional`              | Shared transactional email templates              |

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
| `crates/anvil-architecture` | Architecture rule evaluation                              |
| `crates/anvil-bench`        | Stress-test harness for capacity discovery                |
| `crates/anvil-checks`       | Gate checks ported to Rust (secret scan, anti-pattern)    |
| `crates/anvil-policy`       | OPA/policy evaluation engine                              |
| `crates/anvil-tui`          | Ratatui TUI surfaces (dashboard, wizard, gate explorer)   |
| `crates/spike`              | Validation spikes for tree-sitter, notify-rs, petgraph    |

### Infrastructure

| Directory | Package                  | Description                     |
| --------- | ------------------------ | ------------------------------- |
| `infra`   | `@eddacraft/anvil-infra` | Pulumi IaC (Vercel, DNS, cloud) |

### Tools

| Directory          | Description               |
| ------------------ | ------------------------- |
| `tools/scripts`    | Build and utility scripts |
| `tools/generators` | NX code generators        |
| `tools/codemods`   | Codemod transformations   |
| `tools/test-utils` | Shared test utilities     |

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
- **Rust toolchain** (for crates) — install via [rustup](https://rustup.rs/)
- **cargo-llvm-cov** (optional, for Rust coverage) —
  `cargo install cargo-llvm-cov`

### Setup

```bash
git clone https://github.com/eddacraft/anvil-001.git
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

> Last measured: 2026-04-13 · commit `2b613407`. `eddacraft-tui` is now an
> external git dependency and excluded from the Rust table. Run
> `pnpm test:coverage` for current numbers across both stacks.

Coverage reflects unit and integration tests only (v8 / llvm-cov providers). E2E
tests run separately via `apps/e2e/` and do not contribute to line coverage.

#### TypeScript

| Project                                 |     Lines |    Branch | Test Files | Types             |
| --------------------------------------- | --------: | --------: | ---------: | ----------------- |
| `contracts`                             |      100% |      100% |          1 | Unit              |
| `platform-config`                       |      100% |      100% |          2 | Unit              |
| `@eddacraft/anvil-aps`                  |     96.8% |     85.7% |          8 | Unit              |
| `platform-storage`                      |     95.0% |     87.5% |          1 | Unit              |
| `@eddacraft/anvil-mcp-server`           |     88.7% |     75.4% |         12 | Unit              |
| `@eddacraft/anvil-adapters`             |     87.0% |     76.4% |         13 | Unit              |
| `core`                                  |     83.6% |     73.8% |         37 | Unit              |
| `@eddacraft/anvil-edda-stack`           |     77.2% |     65.3% |         33 | Unit              |
| `policy`                                |     75.9% |     67.5% |          5 | Unit              |
| `runtime`                               |     71.2% |     63.1% |   27u + 2i | Unit, Integration |
| `anvil-vscode`                          |     62.5% |     43.1% |          7 | Unit              |
| `@eddacraft/anvil-api`                  |     62.0% |     54.6% |          6 | Unit              |
| `json-render`                           |     44.8% |     20.8% |          2 | Unit              |
| `@eddacraft/anvil-kindling-integration` |     19.9% |      6.4% |          1 | Unit              |
| `eslint-plugin-anvil`                   |    --[^1] |    --[^1] |          3 | Unit              |
| `infra`                                 |    --[^4] |    --[^4] |          3 | Unit              |
| `ports`                                 |   N/A[^2] |   N/A[^2] |          0 | --                |
| `platform-crypto`                       |    0%[^3] |    0%[^3] |          0 | --                |
| `anvil-website`                         |    --[^5] |    --[^5] |          0 | --                |
| `transactional`                         |    --[^5] |    --[^5] |          0 | --                |
| `docs-site`                             |    --[^5] |    --[^5] |          0 | --                |
| `anvil-generators`                      |    --[^5] |    --[^5] |          0 | --                |
| `docs-shell`                            |    --[^6] |    --[^6] |          6 | Unit              |
| **TS total**                            | **77.2%** | **67.0%** |    **164** |                   |

#### Rust

| Crate                |     Lines | Test Modules |
| -------------------- | --------: | -----------: |
| `anvil-kernel-types` |     99.4% |            5 |
| `anvil-kernel`       |     94.7% |           23 |
| `anvil-bench`        |     94.3% |            8 |
| `anvil-architecture` |     93.4% |            5 |
| `anvil-checks`       |     91.7% |           17 |
| `anvil-tui`          |     90.0% |           29 |
| `anvil-policy`       |     77.6% |            8 |
| `anvil-cli`          |     54.5% |           22 |
| `spike`              |      0.0% |            0 |
| **Rust total**       | **79.8%** |      **117** |

[^1]:
    `eslint-plugin` tests run via NX project-level config, not the root vitest
    config.

[^2]: `ports` contains pure interface definitions — no executable code to cover.

[^3]: `platform-crypto` has no tests yet.

[^4]: Coverage not yet measured for this project.

[^5]: No tests — not included in coverage totals.

[^6]: Coverage not yet measured for this project (new addition).

### Test type breakdown

| Type            | Files | Description                                            |
| --------------- | ----: | ------------------------------------------------------ |
| TS Unit         |   162 | Co-located `*.test.ts` — mocked deps, fast             |
| TS Integration  |     2 | `*-integration.test.ts` — multi-module, in-process     |
| TS E2E          |    10 | `*.e2e.test.ts` in `apps/e2e/` — cross-package testing |
| Rust Unit/Integ |   117 | `#[cfg(test)]` modules — inline and integration tests  |
| Rust Benchmarks |     — | Criterion micro-benchmarks (`cargo bench`, see below)  |

### Running coverage

```bash
# Full monorepo (TypeScript + Rust)
pnpm test:coverage

# TypeScript only (via Nx — runs all project-level vitest configs)
pnpm test:coverage:ts

# Rust only (cargo-llvm-cov → target/llvm-cov/html/)
pnpm test:coverage:rust

# Per TS project (via Nx)
pnpm nx test <project-name> --coverage

# Root vitest config only (excludes eslint-plugin-anvil — see ^1)
pnpm vitest run --coverage
```

Coverage output is written to the root `coverage/` directory (HTML, JSON, and
JSON summary), which is the path used by the built-in coverage gate check.

## Rust Kernel Benchmarks

The Rust kernel (`anvil-kernel`) includes Criterion micro-benchmarks for
regression detection. These validate the performance targets defined in the
[Kernel Spec](./docs/architecture/rust-kernel-spec.md).

### Performance Targets vs Measured

The kernel was designed against the targets in the
[Kernel Spec](./docs/architecture/rust-kernel-spec.md). The 2026-04-03 benchmark
run (rayon-parallel parser, release build, Criterion 100 samples):

| Metric                                    | Target      | Measured (2026-04-03)      | Status                            |
| ----------------------------------------- | ----------- | -------------------------- | --------------------------------- |
| Cold graph build, 100 files               | —           | **14.5 ms**                | Validated                         |
| Cold graph build, 1,000 files             | —           | **~565 ms** (extrapolated) | Validated                         |
| Cold graph build, 2,500 files             | —           | **~3.4 s** (extrapolated)  | Validated                         |
| Cold graph build, 100k LOC / ~2,000 files | < 3 seconds | Pending stress harness     | Pending                           |
| Incremental update (single file)          | < 100 ms    | **10.7 µs**                | Validated · ~10,000× under target |
| Policy evaluation (all invariants)        | —           | **799 ns**                 | Validated                         |
| Event emission (1,000 events)             | < 10 ms     | **408 µs**                 | Validated · ~25× under target     |
| Memory footprint (medium repo)            | < 500 MB    | Pending stress test        | Pending                           |
| File detection latency (p99)              | < 20 ms     | Validated (spike)          | Validated                         |
| tree-sitter parse (single file)           | < 1 ms      | Validated (spike + bench)  | Validated                         |

Full benchmark report and marketing-ready angles:
[`eddacraft-gtm/competitive/anvil-benchmarks-2026-04-03.md`](https://github.com/eddacraft/eddacraft-gtm/blob/main/competitive/anvil-benchmarks-2026-04-03.md)

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

> Windows aarch64 is not yet shipped via `cargo-dist` / GitHub Releases —
> axoupdater has no prebuilt binary for that target, so it is excluded from the
> `cargo-dist` matrix until upstream support lands. The target is still built in
> Rust CI (`.github/workflows/rust.yml`).

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

A reusable **Anvil Check** GitHub Action (that is the action's declared name) is
also provided at `.github/actions/anvil-check/` for running anvil analysis in
your own workflows.

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
