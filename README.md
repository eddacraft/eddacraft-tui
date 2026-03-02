# EddaCraft

[![CI](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml/badge.svg)](https://github.com/EddaCraft/anvil-001/actions/workflows/ci.yml)
[![NX](https://img.shields.io/badge/managed%20with-Nx-143055.svg?style=flat-square)](https://nx.dev/)
[![pnpm](https://img.shields.io/badge/maintained%20with-pnpm-cc00ff.svg?style=flat-square)](https://pnpm.io/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.9-blue.svg?style=flat-square)](https://www.typescriptlang.org/)
[![Node.js](https://img.shields.io/badge/Node.js->=20.0.0-339933.svg?style=flat-square&logo=node.js&logoColor=white)](https://nodejs.org/)

EddaCraft monorepo. Currently home to **Anvil** — a deterministic development
automation platform that catches architecture drift and AI anti-patterns at file
save, before they reach code review.

## Repository Structure

This is an NX-managed pnpm workspace containing the following apps, packages,
and tooling.

### Apps

| Directory        | Package                      | Description                              | Deployment          |
| ---------------- | ---------------------------- | ---------------------------------------- | ------------------- |
| `apps/anvil-cli` | `@eddacraft/anvil-cli`       | CLI application (Commander.js + Ink TUI) | npm (`publish.yml`) |
| `apps/docs-site` | `@eddacraft/anvil-docs-site` | Docusaurus documentation site            | Vercel              |
| `apps/website`   | `@eddacraft/anvil-website`   | Marketing website (Next.js)              | Vercel              |
| `apps/anvil-api` | —                            | API service                              | —                   |
| `apps/anvil-ui`  | —                            | Web UI                                   | —                   |
| `apps/e2e`       | —                            | End-to-end test suites (Playwright)      | —                   |

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

| Directory                       | Package                                 | Description                               |
| ------------------------------- | --------------------------------------- | ----------------------------------------- |
| `packages/adapters`             | `@eddacraft/anvil-adapters`             | Format converters (SpecKit, BMAD)         |
| `packages/aps`                  | `@eddacraft/anvil-aps`                  | APS document parser                       |
| `packages/eslint-plugin-anvil`  | `eslint-plugin-anvil`                   | ESLint rules for test quality enforcement |
| `packages/vscode-extension`     | `anvil-vscode`                          | VS Code integration                       |
| `packages/kindling-integration` | `@eddacraft/anvil-kindling-integration` | Kindling memory integration contracts     |
| `packages/edda-stack`           | `@eddacraft/anvil-edda-stack`           | Kindling · Ember · Edda memory stack      |
| `packages/shared`               | —                                       | Shared utilities                          |

### Packages — Tooling

| Directory                        | Description                      |
| -------------------------------- | -------------------------------- |
| `packages/tooling/tsconfig`      | Shared TypeScript configurations |
| `packages/tooling/eslint-config` | Shared ESLint configurations     |

### Tools

| Directory          | Description               |
| ------------------ | ------------------------- |
| `tools/scripts`    | Build and utility scripts |
| `tools/generators` | NX code generators        |
| `tools/codemods`   | Codemod transformations   |

## Getting Started

### Prerequisites

- **Node.js** >= 20.0.0
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

# Link Anvil CLI globally
pnpm link:cli
```

NX is used under the hood — you can also use `npx nx` commands directly for
targeted builds, affected-only runs, and task graph visualisation.

## Test Coverage

> Last measured: 2026-02-26 · commit `8663d83` · 176 unit/integration test files
> (184 total incl. E2E) · 4,182 tests passing

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
| `@eddacraft/anvil-mcp-server`           |     43.5% |     36.6% |                          4[^1] | Unit                   |
| `anvil-vscode`                          |     62.5% |     43.9% |                              7 | Unit                   |
| `eslint-plugin-anvil`                   |    --[^2] |    --[^2] |                              3 | Unit                   |
| `contracts`                             |      100% |      100% |                              1 | Unit                   |
| `ports`                                 |   N/A[^3] |   N/A[^3] |                              0 | --                     |
| `core`                                  |     83.4% |     73.3% |                             35 | Unit                   |
| `runtime`                               |     60.3% |     53.0% |               24 unit, 2 integ | Unit, Integration      |
| `policy`                                |     76.4% |     67.2% |                              5 | Unit                   |
| `platform-config`                       |      100% |      100% |                              2 | Unit                   |
| `platform-storage`                      |     90.5% |     79.2% |                              1 | Unit                   |
| `platform-crypto`                       |    0%[^4] |    0%[^4] |                              0 | --                     |
| **Monorepo total**                      | **64.0%** | **53.8%** |                        **176** |                        |

[^1]:
    `mcp-server` runs 4 of 12 test files — remaining tool tests have
    pre-existing failures from mock paths that need updating.

[^2]:
    `eslint-plugin` tests run via NX project-level config, not the root vitest
    config.

[^3]: `ports` contains pure interface definitions — no executable code to cover.

[^4]: `platform-crypto` has no tests yet.

### Test type breakdown

| Type        | Files | Description                                            |
| ----------- | ----: | ------------------------------------------------------ |
| Unit        |   171 | Co-located `*.test.ts` — mocked deps, fast             |
| Integration |     5 | `*-integration.test.ts` — multi-module, in-process     |
| CLI E2E     |     5 | `*.e2e.test.ts` — `execFile`/`spawn`-based CLI testing |
| TUI E2E     |     3 | `*.tuistory.e2e.test.ts` — Ink pseudo-terminal testing |

### Running coverage

```bash
# Per project (via Nx)
pnpm nx test <project-name> --coverage

# Full monorepo (via Nx — runs all project-level vitest configs)
pnpm nx run-many -t test --coverage

# Root vitest config only (excludes eslint-plugin-anvil — see ^2)
pnpm vitest run --coverage
```

Coverage output is written to the root `coverage/` directory (HTML, JSON, and
JSON summary), which is the path used by the built-in coverage gate check.

## Deployment

| App         | Platform | Trigger                                               |
| ----------- | -------- | ----------------------------------------------------- |
| `anvil-cli` | npm      | Git tag (`v*`) via `publish.yml` GitHub Action        |
| `docs-site` | Vercel   | Push to `main` (automatic via Vercel Git integration) |
| `website`   | Vercel   | Push to `main` (automatic via Vercel Git integration) |
| `anvil-api` | —        | Not yet deployed                                      |
| `anvil-ui`  | —        | Not yet deployed                                      |

## CI/CD

The repository has several GitHub Actions workflows:

- **ci.yml** — Lint, typecheck, test, and build on every push and PR. Runs
  against Node.js 20.x and 22.x with smart change detection (docs-only changes
  skip code tests).
- **publish.yml** — Publishes `@eddacraft/anvil-cli` to npm on version tags
  (`v*`) by default (beta tags are forced CLI-only). Workspace package
  publishing is manual/explicit only. Validates tag/package version alignment,
  runs the full test suite, and creates a GitHub release.
- **claude.yml** — Claude Code integration for AI-assisted issue triage and PR
  review.
- **claude-code-review.yml** — Automated code review on pull requests via Claude
  Code.

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

| Document                                                             | Description                 |
| -------------------------------------------------------------------- | --------------------------- |
| [Quick Start](./docs/public/anvil/quickstart.md)                     | Get running in 5 minutes    |
| [CLI Reference](./apps/anvil-cli/README.md)                          | Complete command reference  |
| [First Project](./docs/public/anvil/first-project.md)                | Real-world setup example    |
| [Troubleshooting](./docs/public/anvil/operations/troubleshooting.md) | Common issues and solutions |
| [Configuration](./docs/public/anvil/operations/config.md)            | Configuration options       |
| [Architecture](./docs/ARCHITECTURE.md)                               | System design               |
| [Release Runbook](./docs/guides/release-runbook.md)                  | Safe CLI release checklist  |
| [Plans](./plans/index.aps.md)                                        | Detailed roadmap            |
