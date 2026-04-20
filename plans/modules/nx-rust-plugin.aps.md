<!-- APS Module: nx-rust-plugin -->
<!-- Status: Ready -->

# Nx Rust Plugin

Build an in-house Nx plugin, `@eddacraft/nx-rust`, that wraps `cargo` as
Nx executors and generators. Becomes the execution layer for Rust-side
orchestration in RUSTNX.

## Purpose

RUSTNX-004 and RUSTNX-005 call for per-crate `project.json` wrappers and
correct Nx inputs/outputs so the 9 Rust crates participate in `nx
affected` and the Azure remote cache. Today those work items are scoped
as hand-rolled — repeating the same cargo-wrapping logic across 9 crates
and the same input/output declarations across every target in `nx.json`.

Two off-the-shelf alternatives were evaluated and rejected:

- **`cargo-make`** — a Rust-only task runner. It does not orchestrate
  the 15 TS projects, does not integrate with `@nx/azure-cache`, and
  does not give us `nx affected`. Replacing Nx with it would leave TS
  unorchestrated.
- **`@monodon/rust`** — a community Nx plugin of the right shape, but
  the upstream repository has no LICENSE file (no grant of use), has
  not cut a release in over a year, and has many open issues.
  Commercial and technical risk both unacceptable.

Building the plugin in-house centralises the executor logic, matches
the existing `@eddacraft/anvil-generators` pattern at `tools/generators/`,
and puts us in control of Nx-version compatibility. The plugin stays
thin — `cargo` remains the build engine; the plugin only shapes input
hashing, output caching, and project-graph edges for Nx.

**Why this matters:**

1. **Remove duplicated wiring** — one executor definition applied across
   9 crates beats 9 hand-rolled `project.json` files.
2. **Correct affected detection** — a project-graph hook that parses
   `Cargo.toml` dependencies means `nx affected` sees Rust dep edges,
   not just filesystem proximity.
3. **Unblocks RUSTNX Tier 2** — RUSTNX-004 and RUSTNX-005 become thin
   consumers of this module rather than green-field scaffolding.
4. **Licence-clean** — in-house plugin is marked `PROPRIETARY`, matching
   `@eddacraft/anvil-generators`. No third-party licence uncertainty.

## In Scope

- New package at `tools/nx-rust/` named `@eddacraft/nx-rust`, licence
  `PROPRIETARY`, depending on `@nx/devkit ^22.6.5`
- MVP executors wrapping cargo: `build`, `test`, `check`, `clippy`, `fmt`
- One generator: `crate` (workspace-member crate with `--bin` flag),
  updates root `Cargo.toml` `[workspace.members]`
- Project-graph plugin that parses `crates/*/Cargo.toml` path-deps and
  workspace-deps into Nx graph edges
- Plugin registered in root `nx.json` alongside `@nx/js/typescript`,
  `@nx/eslint/plugin`, `@nx/vite/plugin`
- Pilot on `anvil-kernel-types` (smallest crate, no internal deps)
- Rollout `project.json` files to the remaining 8 crates
- Smoke test inside the plugin package that runs `check` against the
  pilot crate and asserts exit 0

## Out of Scope

- Changing the `cargo` execution model — cargo stays as the build
  engine; this plugin only wraps invocation
- `napi` executor — no Node-native Rust modules in Anvil today
- `bench` executor — Criterion runs via a direct cargo call; unchanged
- Cross-compile matrix — stays in `rust.yml` per RUSTNX constraints
- Publishing to crates.io (covered by ADR-017 + separate work)
- Replacing `@eddacraft/anvil-generators` — this plugin is a sibling,
  not a replacement
- Hand-porting any code from `@monodon/rust` — no licence, no grant;
  public API shape may be referenced, source code may not be copied

## Interfaces

### Depends On

- Nx 22.6.5 with `@nx/devkit` (already present)
- `@nx/azure-cache` (already configured) — this module produces
  executors whose outputs the cache can store
- Cargo workspace at repo root with `resolver = "2"` (already present)
- `rust-toolchain.toml` at repo root (already present)

### Exposes

- `@eddacraft/nx-rust:build` / `:test` / `:check` / `:clippy` / `:fmt`
  executors, usable from any `crates/*/project.json`
- `@eddacraft/nx-rust:crate` generator, invokable via
  `pnpm exec nx g @eddacraft/nx-rust:crate <name>`
- Project-graph edges between Rust crates so `nx affected -t test`
  scopes correctly
- Stable input shape for cache keys: `{projectRoot}/**/*.rs`,
  `{projectRoot}/Cargo.toml`, `{workspaceRoot}/Cargo.lock`,
  `{workspaceRoot}/rust-toolchain.toml`

### Consumed By

- **RUSTNX-004** — switches from hand-rolled per-crate `project.json`
  to this plugin's executors
- **RUSTNX-005** — workspace-level `namedInputs` and `targetDefaults`
  only; per-target input/output declarations come from the plugin
- **RUSTNX-006** — root `pnpm test`/`lint`/`typecheck` fan out via
  `nx run-many` over crates registered by this plugin
- **RUSTNX-007** — `nx affected` for Rust works because of the
  project-graph plugin shipped here

## Constraints

- Zero regressions: every crate's `cargo check/test/clippy/fmt` must
  still pass when run directly, independent of Nx
- Output determinism: executor stdout must be suitable for Nx hash
  matching — no embedded timestamps, no PID leaks in logs
- Must work locally without Azure cache credentials (cache miss, not
  failure)
- Must work on CI without `@eddacraft/nx-rust` being published — the
  package stays intra-repo, linked via pnpm workspace protocol
- Plugin surface must be small enough that an Nx major-version upgrade
  can be audited and fixed in under a day
- UK English in all plan and README text
- Licence: `PROPRIETARY` — matches `@eddacraft/anvil-generators`

## Ready Checklist

- [x] RUSTNX module exists and identifies tier 2 as the target
- [x] `@eddacraft/anvil-generators` precedent at `tools/generators/`
      confirmed as the scaffold shape to mirror
- [x] Cargo workspace members list known (9 crates)
- [x] Nx 22.6.5 + `@nx/devkit` already in devDependencies
- [x] ADR-026 drafted to capture the tooling decision (see Decisions)
- [ ] Plugin spike on `anvil-kernel-types` run and cached

---

## Work Items

### NXRUST-001: Scaffold the `@eddacraft/nx-rust` package

- **Intent:** Stand up the plugin package shell with the same shape as
  `@eddacraft/anvil-generators`
- **Expected Outcome:** `tools/nx-rust/` exists with `package.json`
  (name `@eddacraft/nx-rust`, licence `PROPRIETARY`, dep
  `@nx/devkit ^22.6.5`), `tsconfig.json`, empty `executors.json` and
  `generators.json`, and a `build` script that compiles without error
- **Scope:** `tools/nx-rust/**`
- **Validation:** `pnpm --filter @eddacraft/nx-rust build` exits 0;
  `pnpm exec nx list` recognises the package (even with no executors
  yet)
- **Confidence:** high
- **Non-scope:** Executor or generator implementation — this task is
  package skeleton only

### NXRUST-002: Implement MVP executors

- **Intent:** Ship executors covering the five cargo commands RUSTNX
  needs: build, test, check, clippy, fmt
- **Expected Outcome:** Each executor accepts shared options
  (`release`, `features`, `target`, `toolchain`, passthrough `args`),
  shells out to the corresponding `cargo` subcommand scoped to a
  single workspace member (`-p <crate>`), and inherits stdio so cargo
  output is visible unchanged
- **Scope:** `tools/nx-rust/src/executors/**`,
  `tools/nx-rust/executors.json`
- **Dependencies:** NXRUST-001
- **Validation:** From a minimal fixture `project.json` that uses each
  executor, `pnpm exec nx run <fixture>:<target>` produces the same
  exit code and comparable stdout as the equivalent raw `cargo`
  command
- **Confidence:** high
- **Risks:** Executor argument composition drift (features/profiles
  not passed through correctly) — mitigated by a shared
  `buildCargoArgs(options)` helper with unit coverage

### NXRUST-003: Implement the `crate` generator

- **Intent:** Let contributors scaffold a new workspace-member crate
  with `project.json` pre-wired to this plugin's executors
- **Expected Outcome:** `pnpm exec nx g @eddacraft/nx-rust:crate <name>`
  creates `crates/<name>/{Cargo.toml,src/lib.rs,project.json}`,
  updates the root `Cargo.toml` `[workspace.members]` list, and
  honours a `--bin` flag to produce `src/main.rs` instead
- **Scope:** `tools/nx-rust/src/generators/crate/**`,
  `tools/nx-rust/generators.json`, templated files under
  `tools/nx-rust/src/generators/crate/files/`
- **Dependencies:** NXRUST-002
- **Validation:** Generating a throwaway crate, running `cargo check`
  on it, and running `pnpm exec nx run <generated>:test` all succeed;
  reverting the generation cleans up with no leftovers in `Cargo.toml`
- **Confidence:** medium
- **Non-scope:** Library variants beyond the single `crate` generator;
  `napi` scaffolding

### NXRUST-004: Implement the project-graph plugin

- **Intent:** Give `nx affected` correct visibility into Rust-to-Rust
  dependencies by parsing Cargo manifests into Nx graph edges
- **Expected Outcome:** A `processProjectGraph` (or Nx 22 equivalent)
  hook reads every `crates/*/Cargo.toml`, resolves workspace and
  path-based dependencies, and emits edges; `pnpm exec nx graph`
  renders the expected `anvil-cli → anvil-kernel → anvil-kernel-types`
  chain
- **Scope:** `tools/nx-rust/src/graph/**`, `tools/nx-rust/src/index.ts`
- **Dependencies:** NXRUST-002
- **Validation:** `pnpm exec nx graph --focus=anvil-kernel` shows its
  declared dependents and dependencies; after touching only
  `crates/anvil-cli/src/**`,
  `pnpm exec nx affected -t test --base=HEAD~1` does not run
  `anvil-kernel-types` targets
- **Confidence:** medium
- **Risks:** Nx 22 project-graph plugin API drift vs older docs —
  mitigated by copying the `@nx/js` plugin's shape from
  `node_modules/@nx/js`

### NXRUST-005: Pilot the plugin on `anvil-kernel-types`

- **Intent:** Validate the full plugin end-to-end against the smallest
  real crate before rolling out
- **Expected Outcome:** `crates/anvil-kernel-types/project.json` uses
  the plugin's executors; `pnpm exec nx run anvil-kernel-types:test`
  matches `cargo test -p anvil-kernel-types`; a second local run
  reports a cache hit; with Azure creds, a fresh clone hits the remote
  cache
- **Scope:** `crates/anvil-kernel-types/project.json`, possibly small
  `nx.json` `namedInputs`/`targetDefaults` tweaks
- **Dependencies:** NXRUST-004
- **Validation:** See Expected Outcome; all three checks must pass
- **Confidence:** medium
- **Risks:** `target/` caching is notoriously fragile (per RUSTNX-005
  risk notes). Pilot may reveal we can cache test/clippy exit codes
  but not raw build outputs — if so, narrow the `outputs` declaration
  rather than abandon the cache

### NXRUST-006: Roll plugin out to the remaining 8 crates

- **Intent:** Bring every Rust crate under the plugin so `nx affected`
  covers the full workspace
- **Expected Outcome:** All 9 `crates/*/project.json` files exist and
  use the plugin's executors; `pnpm exec nx show projects` lists every
  crate alongside the TS projects; `pnpm exec nx run-many -t test`
  passes across the whole workspace
- **Scope:** `crates/*/project.json` for the 8 remaining crates
  (`anvil-kernel`, `anvil-tui`, `anvil-checks`, `anvil-bench`,
  `anvil-cli`, `anvil-policy`, `anvil-architecture`, `spike`)
- **Dependencies:** NXRUST-005
- **Validation:** `pnpm exec nx show projects --type=app,lib` includes
  every Rust crate; `pnpm exec nx run-many -t test` completes green
- **Confidence:** high
- **Non-scope:** Changing crate features, profiles, or the cross-
  compile matrix

### NXRUST-007: Smoke test for the plugin in CI

- **Intent:** Catch regressions in the plugin on every PR without
  depending on full RUSTNX wiring
- **Expected Outcome:** CI job (extending `rust.yml` or a new workflow)
  runs `pnpm --filter @eddacraft/nx-rust build` and then
  `pnpm exec nx run anvil-kernel-types:check`; failure blocks merge
- **Scope:** `.github/workflows/rust.yml` (or new job file)
- **Dependencies:** NXRUST-005
- **Validation:** A PR that deliberately breaks the plugin (e.g. a
  malformed schema) fails CI with a clear error
- **Confidence:** high

### NXRUST-008: Update RUSTNX-004 and RUSTNX-005 to consume the plugin

- **Intent:** Fold this module back into the RUSTNX plan so the two
  documents stay aligned
- **Expected Outcome:** `plans/modules/rust-nx-migration.aps.md`
  RUSTNX-004 intent/scope references the plugin; RUSTNX-005 trimmed to
  the workspace-level `Cargo.lock`/toolchain `namedInputs` and
  `targetDefaults` the plugin doesn't own; "Depends on" list updated
  to include this module; Ready checklist updated
- **Scope:** `plans/modules/rust-nx-migration.aps.md`
- **Dependencies:** NXRUST-006
- **Validation:** Diff review — RUSTNX-004 no longer specifies
  hand-rolled wiring; RUSTNX-005 scope reduced but not removed
- **Confidence:** high

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
| ---- | ------ | ---------- | ---------- |
| Nx 22 project-graph plugin API differs from our reference | medium | medium | Clone the shape from `@nx/js` via `node_modules`; write against the actual installed API, not external docs |
| `target/` caching produces stale artefacts | high | medium | Prefer narrow `outputs` declarations (exit-code + report) over caching raw build output; see RUSTNX-005 parallel risk |
| Plugin falls behind on Nx major upgrade | medium | medium | Small API surface, smoke test in CI, explicit "check plugin still builds" item in Nx upgrade runbook |
| Executor option drift across the five cargo commands | low | medium | Shared `buildCargoArgs` helper with unit tests |
| Licence question re-raised about referencing monodon's API shape | low | low | API inspection is not copying; we write from cargo docs + `@nx/devkit` docs. No source code from monodon enters the tree |

## Decisions

- **ADR-026 (Proposed)** —
  `plans/decisions/026-in-house-nx-rust-plugin.md`. Captures: reject
  cargo-make as non-substitute for Nx; reject `@monodon/rust` due to
  missing licence, stale upstream, and open-issue backlog; adopt
  in-house plugin mirroring `@eddacraft/anvil-generators`.
- Supersedes the "hand-rolled per-crate project.json" implementation
  path originally documented in RUSTNX-004.

## Open Questions

- [ ] Should the plugin ship a `bench` executor in a follow-up, or
      should Criterion stay on direct `cargo bench` forever?
- [ ] Do we cache `fmt --check` (cheap + deterministic) or skip caching
      entirely for that target?
- [ ] Is there value in a second generator for a library-only crate
      preset (no binary scaffolding), or is `crate` with `--bin` enough
      for the foreseeable future?
