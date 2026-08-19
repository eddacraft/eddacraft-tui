# anvil — Context Map

Fast orientation for humans and agents: **what we call things**, **where they
live**, and **where to go next**. This is a map, not a manual — it links to the
authoritative source for each topic instead of restating it.

- **How to behave** (planning, commits, scope, lifecycle) →
  [`AGENTS.md`](AGENTS.md)
- **Agent harness config** (hooks, skills, MCP) → [`CLAUDE.md`](CLAUDE.md)
- **Live work status** → [`plans/index.aps.md`](plans/index.aps.md)
- **Why the system is shaped this way** →
  [`docs/architecture/overview.md`](docs/architecture/overview.md)

If this file and a linked source disagree, the linked source wins — fix the
pointer here.

## What anvil is

A deterministic, zero-config governance tool for codebases: it watches saves and
runs checks/policies, warning on **new** violations rather than blocking. Core
principles — deterministic, warnings over blocks, new-edges-only — live in
[`docs/vision/anvil-scope-guard.md`](docs/vision/anvil-scope-guard.md) and the
ADRs. The shipped product is a **pure-Rust binary** (`anvil`); the TypeScript
surfaces are docs, API, and tooling, not the engine.

## Vocabulary

Terms you'll see everywhere, and what they actually mean:

| Term                                     | Meaning                                                                                                                                                         |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **APS**                                  | Anvil Plan Spec — the `.aps.md` planning format. All multi-step work uses it.                                                                                   |
| **Module / work item**                   | A unit of planned work (`PREFIX-NNN`) tracked in an APS module file.                                                                                            |
| **CIB**                                  | Continuous Improvement Backlog — the rolling small-improvement module.                                                                                          |
| **CI-log / pending queue**               | Session evidence log + `.git/anvil/ci-log-pending/` harvest path; see [`docs/guides/continuous-improvement-log.md`](docs/guides/continuous-improvement-log.md). |
| **Council**                              | Multi-persona code review (`/council quick\|mini\|full`), run before a PR.                                                                                      |
| **kernel / checks / policy / intercept** | The four engine layers — see _Where things live_.                                                                                                               |
| **gate / audit / check / drift**         | anvil CLI verbs; only `welcome`'s scan honours `.gitignore`, the rest use standard filters by design.                                                           |
| **kindling**                             | The published OSS integration glue (separate concern from the engine).                                                                                          |
| **edda-stack**                           | The memory/persistence stack (SQLite-backed).                                                                                                                   |
| **FLAGCAT**                              | The feature-flag catalogue (`flags/manifest.json` is the single definition).                                                                                    |
| **Worktrunk / `wt`**                     | The git-worktree workflow used for all branch work.                                                                                                             |
| **ADR**                                  | Architecture Decision Record, indexed in the decision log.                                                                                                      |

## Where things live

### The engine — `crates/` (Rust)

The product. Key crates by role:

| Role               | Crates                                                                                                                                              |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Core model & types | `anvil-kernel`, `anvil-kernel-types`                                                                                                                |
| Check pipeline     | `anvil-checks`, `anvil-rules`, `anvil-checks-napi` (NAPI bridge)                                                                                    |
| Policy             | `anvil-policy`, `anvil-policy-engine` (hybrid DC + OPA)                                                                                             |
| Save interception  | `anvil-intercept`, `anvil-intercept-rules`, `anvil-intercept-proto`, `anvil-intercept-win32`                                                        |
| CLI & runtime      | `anvil-cli`, `anvil-run`, `anvil-hook`, `anvil-config`                                                                                              |
| TUI                | `anvil-tui`, `eddacraft-tui` (the OSS rendering lib, consumed by path)                                                                              |
| Supporting         | `anvil-baseline`, `anvil-architecture`, `anvil-attribution`, `anvil-witness`, `anvil-l4`, `anvil-graph-cache`, `anvil-observability`, `anvil-bench` |

Rust crate layout and layering are documented in
[`docs/architecture/rust-architecture-overview.md`](docs/architecture/rust-architecture-overview.md).
Check, finding, gate, and surface concepts are documented in the
[`quality model`](docs/architecture/quality-model.md); as-built detail is in
`docs/architecture/*-as-built.md`.

### TypeScript surfaces — `packages/` and `apps/`

- **`packages/`** — npm-scoped libraries (`@eddacraft/*`): adapters, APS tooling
  (`anvil-aps`), docs metadata (`anvil-docs-meta`), the memory stack
  (`edda-stack`), driver client, the `eslint-plugin-anvil` linter, and
  transactional email. Several entries (`anvil`, `libs`, `shared`, `tooling`)
  are grouping directories, not single packages.
- **`apps/`** — deployables and doc surfaces: `anvil-api` (Hono REST API),
  `admin-cli`, the Docusaurus/Next docs apps (`docs-public`, `docs-shell`,
  `docs-site`, `anvil-docs-private`), the marketing `website`, and the `e2e`
  Vitest harness. See [`apps/README.md`](apps/README.md).

### Planning & decisions — `plans/`

- [`plans/index.aps.md`](plans/index.aps.md) — **single source of truth** for
  module status. Read before any implementation work.
- `plans/modules/` — active modules; `plans/archive/modules/` — completed.
- [`plans/aps-rules.md`](plans/aps-rules.md) — APS format rules.
- [`plans/project-context.md`](plans/project-context.md) — anvil-specific
  workflow, release, and docs-governance context.
- [`plans/decisions/DECISION-LOG.md`](plans/decisions/DECISION-LOG.md) — ADR
  index; start here before proposing architecture changes.

### Documentation — `docs/`

`architecture/` (design + as-built), `guides/` (development practice), `vision/`
(north-star + scope guard), `runbooks/`, `public/`, `specs/`, `indexes/`
(generated). Authority model and the mandatory closeout checklist:
[`docs/guides/documentation-governance.md`](docs/guides/documentation-governance.md).

### Other roots

`flags/` (feature-flag manifest) · `patterns/` (compiled detection patterns) ·
`policies/` (Rego + fixtures) · `schemas/` · `scripts/` (release, APS, docs
tooling) · `.claude/` `.opencode/` `.codex/` (agent config) · `infra/` (Pulumi).

## Where to go next

| If you want to…                     | Go to                                                                                                          |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Start work / check status           | [`plans/index.aps.md`](plans/index.aps.md)                                                                     |
| Know the workflow & commit rules    | [`AGENTS.md`](AGENTS.md)                                                                                       |
| Locate system containers/components | [`docs/architecture/overview.md`](docs/architecture/overview.md)                                               |
| Understand Rust layout/layering     | [`docs/architecture/rust-architecture-overview.md`](docs/architecture/rust-architecture-overview.md)           |
| Understand check/gate concepts      | [`docs/architecture/quality-model.md`](docs/architecture/quality-model.md)                                     |
| Review trust and deployment         | [`docs/architecture/trust-and-deployment-boundaries.md`](docs/architecture/trust-and-deployment-boundaries.md) |
| Trace save-time validation          | [`docs/architecture/save-to-validation.md`](docs/architecture/save-to-validation.md)                           |
| Trace documentation delivery        | [`docs/architecture/docs-delivery.md`](docs/architecture/docs-delivery.md)                                     |
| Check scope before adding a feature | [`docs/vision/anvil-scope-guard.md`](docs/vision/anvil-scope-guard.md)                                         |
| Find a past decision                | [`plans/decisions/DECISION-LOG.md`](plans/decisions/DECISION-LOG.md)                                           |
| Run or change tests                 | [`AGENTS.md`](AGENTS.md) → _Test Infrastructure_                                                               |
| Touch a feature flag                | [`docs/guides/feature-flag-governance.md`](docs/guides/feature-flag-governance.md)                             |

## Local context (spokes)

This repo keeps **per-area detail in `AGENTS.md` files**, not in nested
`CONTEXT.md` files — root `CONTEXT.md` is the only context map. For local
conventions, read the nearest `AGENTS.md`. Current spokes:

- [`apps/docs-shell/AGENTS.md`](apps/docs-shell/AGENTS.md)
- [`apps/docs-site/AGENTS.md`](apps/docs-site/AGENTS.md)
- [`apps/website/AGENTS.md`](apps/website/AGENTS.md)
- [`packages/adapters/AGENTS.md`](packages/adapters/AGENTS.md)
- [`packages/aps/AGENTS.md`](packages/aps/AGENTS.md)
- [`packages/docs-meta/AGENTS.md`](packages/docs-meta/AGENTS.md)
