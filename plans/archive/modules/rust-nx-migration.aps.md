<!-- APS Module: rust-nx-migration -->
<!-- Status: Complete -->

# Rust Nx Migration

Bring the Rust workspace up to parity with the TypeScript Nx setup: CI
caching, affected-only builds, and workspace hygiene. Mirrors the completed
NXTASK module for Rust crates.

## Purpose

The Rust workspace has 9 crates (`anvil-kernel`, `anvil-cli`, `anvil-tui`,
`anvil-checks`, `anvil-policy`, `anvil-architecture`, `anvil-kernel-types`,
`anvil-bench`, `spike`) but CI treats it as one monolithic unit:

- `cargo check/test/clippy --workspace` runs every crate on every PR
- No `Swatinem/rust-cache`, no sccache — `target/` is rebuilt cold each run
- `cargo test` with no `cargo-nextest` — serial, slower, worse output
- `clippy` and `test` jobs chained via `needs: check` rather than running in
  parallel with a shared cache
- Crates are invisible to Nx — no `project.json` files, so the Azure remote
  cache covers only TypeScript
- No workspace-wide dependency hygiene tooling (`cargo-hakari`, `cargo-deny`)

**Why this matters:**

1. **Cold-cache CI** — typical Rust PR CI spends most of its time recompiling
   dependencies that never changed.
2. **Workspace-wide runs** — touching `anvil-cli` re-tests `anvil-kernel-types`
   even though nothing in its inputs changed.
3. **Inconsistency** — TS side runs affected-only with Azure remote cache; Rust
   side re-runs everything, every time.
4. **No shared feature resolution** — `cargo build -p X` and `-p Y` can rebuild
   shared dependencies with different feature sets, defeating caching.

## In Scope

### Tier 1 — CI speed, no architectural change

- Add `Swatinem/rust-cache@v2` to every Rust job
- Switch `cargo test` → `cargo-nextest`; keep coverage via `cargo-llvm-cov
  nextest`
- Decouple `clippy`/`test`/`format` jobs from `needs: check`; run in parallel
  behind the shared cache

### Tier 2 — Affected-only (the NXTASK analogue)

- Add per-crate `project.json` wrapping `cargo check/test/clippy/build`
  targets
- Wire Nx target defaults, inputs (crate sources + `Cargo.lock` + toolchain),
  and outputs (`target/<profile>/<crate>`) so the Azure remote cache applies
- Unify root scripts: `pnpm test` / `pnpm lint` cover TS + Rust via
  `nx run-many`
- Update CI to use `nx affected -t test lint typecheck build` for Rust crates

### Tier 3 — Workspace hygiene

- Adopt `cargo-hakari` workspace-hack to unify feature flags across crates
- Add `cargo-deny` CI job for licence/advisory/bans parity with `pnpm audit`

## Out of Scope

- Migrating the TypeScript side (already complete — see
  `plans/archive/modules/nx-task-migration.aps.md`)
- Replacing `cargo` with a different build system (Bazel, Buck) — Cargo stays
  as the execution engine; Nx only wraps it for caching and affected logic
- Changing crate boundaries or splitting/merging crates
- Changing the clippy lint configuration or `rustfmt.toml`
- Cross-compilation matrix changes (stays as-is on PRs targeting `main`)
- Nightly toolchain adoption or `cargo-udeps` (separate initiative)

## Interfaces

### Depends On

- Nx 22.x with `@nx/azure-cache` (already configured for TS side)
- `rust-toolchain.toml` pins the toolchain (already present)
- Cargo workspace manifest at repo root (already present)

### Exposes

- `cargo nextest run -p <crate>` — per-crate test entry point
- `nx run <crate>:test` / `nx run <crate>:lint` / `nx run <crate>:build` —
  Nx-wrapped cargo targets that participate in the Azure remote cache
- `pnpm test` / `pnpm lint` — unified across TS + Rust
- CI jobs that honour `nx affected` and skip unchanged crates

## Constraints

- Zero regressions: all currently-passing `cargo check/test/clippy/fmt` runs
  must continue to pass
- Coverage output (`coverage-rust.json`, `coverage-rust-summary.txt`) must
  remain compatible with the existing CI summary step
- Cross-compile matrix must continue to run on pushes to `main` and PRs
  targeting `main`
- The OPA test step (`opa test` + `regal lint`) must keep running — it lives
  in the Rust workflow today but isn't a cargo target
- `unsafe_code = "forbid"` and workspace `clippy::pedantic` lint levels stay
  authoritative
- Must work locally without Azure cache credentials (cache miss, not failure)

## Ready Checklist

- [x] NXTASK complete — TypeScript side uses `nx run-many` for
      `build`/`lint`/`test`/`typecheck`
- [x] 9 Rust crates identified in `Cargo.toml` workspace members
- [x] Existing `rust.yml` workflow reviewed (check → test/clippy/format
      chain, no rust-cache, no nextest)
- [x] Azure remote cache configured via `@nx/azure-cache` for TS side
- [x] Cargo workspace uses resolver v2 (required for per-crate feature
      unification)

---

## Work Items

### RUSTNX-001: Add Swatinem/rust-cache to Rust CI jobs [Complete]

- **Intent:** Cache `~/.cargo/registry`, `~/.cargo/git`, and `target/`
  between Rust CI runs so recompilation cost amortises over PRs
- **Expected Outcome:** `check`, `test`, `clippy`, and `format` jobs in
  `rust.yml` restore a shared cache keyed on `Cargo.lock` + rustc version;
  cache-hit PR runs complete noticeably faster than cold runs
- **Scope:** `.github/workflows/rust.yml`
- **Validation:** CI green on a test PR; second run on the same PR shows
  "cache hit" in the rust-cache step logs
- **Confidence:** high
- **Non-scope:** Adding sccache, changing `CARGO_INCREMENTAL`, or touching
  the cross-compile matrix
- **Resolution:** Added `Swatinem/rust-cache@v2.9.1` (SHA-pinned) to
  `check`, `test`, `clippy`, and `format` jobs with `shared-key: rust-ci`
  so all four jobs pull from a single cache keyed on `Cargo.lock` + rustc
  version. `format` uses `save-if: 'false'` because rustfmt doesn't
  populate `target/` and an empty save would overwrite useful state.

### RUSTNX-002: Adopt cargo-nextest for workspace test runs [Complete]

- **Intent:** Replace `cargo test` with `cargo-nextest` for faster,
  parallel, better-reported test execution
- **Expected Outcome:** CI `test` job uses `cargo nextest run --workspace`
  (or equivalent); coverage runs via `cargo llvm-cov nextest`; root
  `pnpm test:coverage:rust` script is updated; per-crate test counts match
  pre-migration results
- **Scope:** `.github/workflows/rust.yml`, root `package.json`
  (`test:coverage:rust` script)
- **Dependencies:** RUSTNX-001
- **Validation:** `cargo nextest run --workspace` passes locally and in CI;
  coverage artifact is produced
- **Confidence:** high
- **Risks:** A small number of tests may rely on `cargo test`-specific
  behaviour (e.g. ignored doctest handling). `cargo test --doc` must still
  run for doctests since nextest does not cover them
- **Resolution:** CI `test` job installs `cargo-nextest` 0.9.133 via
  `taiki-e/install-action@v2.75.18` (both SHA-pinned; version tracked in
  `CARGO_NEXTEST_VERSION` env so bumps are deterministic). Coverage runs
  through `cargo llvm-cov nextest --workspace --json`, and
  `cargo test --doc --workspace` runs as a separate `if: always()` step so
  doctest failures surface even if the coverage step fails. Root
  `pnpm test:coverage:rust` chains
  `cargo llvm-cov nextest --workspace --html && cargo test --doc --workspace`
  to keep local and CI test surfaces aligned. Note: doctests execute but are
  not instrumented for coverage on the pinned stable toolchain; this matches
  pre-migration behaviour, since `cargo llvm-cov --doc` requires nightly and
  nightly is explicitly out of scope for this module.

### RUSTNX-003: Parallelise Rust CI jobs behind shared cache [Complete]

- **Intent:** Let `clippy`, `test`, and `format` run concurrently rather than
  serialising behind `needs: check`
- **Expected Outcome:** `needs: check` removed from `clippy` and `test`;
  `format` already independent; total wall-clock time on cache-warm CI
  measurably lower; no regressions in failure signal
- **Scope:** `.github/workflows/rust.yml`
- **Dependencies:** RUSTNX-001
- **Validation:** CI green; job timing in GitHub Actions shows `clippy` and
  `test` starting in parallel
- **Confidence:** high
- **Non-scope:** Merging jobs into a single matrix — keep them separate for
  clear failure attribution
- **Resolution:** Removed `needs: check` from `test` and `clippy` jobs so
  all four jobs (`check`, `test`, `clippy`, `format`) start in parallel off
  the shared rust-cache restore. `cross-compile` retains `needs: check`
  since it only runs on PRs targeting `main` and is by far the heaviest
  matrix — gating it behind check avoids wasting 6 matrix runners when a
  basic compile error would fail them all.

### RUSTNX-004: Bring Rust crates under Nx via `@eddacraft/nx-rust` [Complete]

- **Intent:** Make every Rust crate visible to Nx with executors wrapping
  `cargo` commands as Nx targets
- **Expected Outcome:** Each of the 9 crates registers as an Nx project
  with `build`, `test`, `check`, `clippy`, `fmt`/`fmt-check`, and
  `nx-release-publish` targets; `nx show projects` lists every crate
  alongside TS projects
- **Scope:** `tools/nx-rust/` (plugin) and `nx.json` (plugin registration)
- **Dependencies:** RUSTNX-003, NXRUST
- **Validation:** `pnpm exec nx show projects` lists all 9 crates by
  Cargo package name; `pnpm exec nx show project eddacraft-anvil-kernel-types --json`
  shows plugin-provided targets
- **Confidence:** high
- **Resolution:** Delivered via the NXRUST module (see
  `plans/modules/nx-rust-plugin.aps.md`) rather than hand-rolled
  `project.json` files. The plugin's `cargo metadata` graph driver
  auto-registers every workspace member with pre-wired executors and
  dependency edges, so `nx affected` is correct across Rust-to-Rust
  edges. ADR-026 captures the tooling decision.

### RUSTNX-005: Workspace-level cache inputs for Rust [Complete]

- **Intent:** Ensure Nx cache keys for Rust targets invalidate when the
  workspace toolchain or resolved dependency set changes
- **Expected Outcome:** `nx.json` `namedInputs.sharedGlobals` includes
  `Cargo.lock` and `rust-toolchain.toml` so every Rust target's hash
  picks up toolchain bumps and dependency updates; per-target inputs
  and outputs (e.g. narrow `outputs` for lint targets, full `target/`
  for build/test) are owned by `@eddacraft/nx-rust`
- **Scope:** `nx.json` `namedInputs.sharedGlobals`
- **Dependencies:** RUSTNX-004
- **Validation:** `nx show project <crate> --json` reports `cache: true`
  on build/test; `nx reset && nx run <crate>:check` then repeat reports
  "cache hit" on the second run; a `Cargo.lock` edit invalidates the
  cache on the next run
- **Confidence:** high
- **Risks:** Caching cargo build artifacts is notoriously fragile —
  `target/` has embedded absolute paths. Plugin mitigates by caching
  exit-code-only for check/clippy/fmt-check and scoping build/test
  outputs to `{options.target-dir}` + `{workspaceRoot}/target`
- **Resolution:** Delivered in branch `feat/nxrust`. Added
  `{workspaceRoot}/Cargo.lock` and `{workspaceRoot}/rust-toolchain.toml`
  to `nx.json` `namedInputs.sharedGlobals`. Per-target input/output
  declarations come from `@eddacraft/nx-rust` — see
  `tools/nx-rust/src/utils/target-configs.ts`. The Azure remote cache
  picks these up via the existing `@nx/azure-cache` plugin with no
  Rust-specific configuration.

### RUSTNX-006: Unify root scripts across TS and Rust [Complete]

- **Intent:** Make `pnpm test`, `pnpm lint`, `pnpm typecheck` cover both
  TypeScript and Rust projects via `nx run-many`
- **Expected Outcome:** Root `package.json` `test`/`lint`/`typecheck`
  scripts include Rust projects; `test:coverage:rust` becomes a convenience
  alias; a contributor running `pnpm test` gets full TS + Rust coverage
- **Scope:** root `package.json`
- **Dependencies:** RUSTNX-005
- **Validation:** `pnpm test` runs at least one target per Rust crate and
  exits 0
- **Confidence:** high
- **Resolution:** Root `package.json` now routes Rust through the same top-level
  contributor commands as TypeScript. `pnpm test` already covered the Rust Nx
  `test` targets; `pnpm run lint:check` now adds Rust `clippy` + `fmt-check`,
  and `pnpm run typecheck` now adds Rust `check` across the dynamically
  discovered Rust crate set. The
  remaining validation blocker was unrelated TS path resolution in
  `apps/anvil-api` and `apps/docs-site`; both tsconfigs now see the workspace
  package paths, so the root scripts run end-to-end.

### RUSTNX-007: Switch Rust CI to nx affected [Complete]

- **Intent:** Run only affected Rust crates on PRs, matching the TS-side CI
  behaviour
- **Expected Outcome:** `rust.yml` runs only the Rust crates affected by a
  PR by diffing against the PR's base ref (`origin/$GITHUB_BASE_REF`) and
  invoking `nx run-many` for `check`, `test`, `clippy`, and `fmt-check` on
  the resolved crate list. Pushes to `main`/`dev` run the full workspace via
  the existing `cargo --workspace` paths. The cross-compile matrix is
  unchanged.
- **Scope:** `.github/workflows/rust.yml`, possibly `.github/workflows/ci.yml`
- **Dependencies:** RUSTNX-006
- **Validation:** A PR that only touches `crates/anvil-cli/src/**` skips
  `anvil-kernel`/`anvil-tui`/etc. lint+test targets; a PR touching
  `Cargo.lock` runs everything
- **Confidence:** medium
- **Risks:** `nx affected` granularity depends on correct input declarations
  from RUSTNX-005. A missed input means affected detection silently skips
  work that should have run
- **Resolution:** `rust.yml` now uses `./.github/actions/setup-workspace`
  on pull requests, discovers the Rust Nx projects via
  `nx show projects --affected --withTarget=check` with `--base=origin/<PR base ref>`,
  and runs `nx run-many` for `check`, `test`, `clippy`, and `fmt-check` against
  the resolved affected list. Pushes keep the full-workspace
  cargo path, doctests still run on pull requests, coverage stays on the full
  test run, and the cross-compile matrix remains unchanged. `nx.json`
  `sharedGlobals` now include the Rust workflow, setup action, package-manager
  metadata, the root workspace manifest, and vendored `tools/nx-rust/**` so Rust CI/tooling changes
  correctly fan out to all Rust projects.

### RUSTNX-008: Adopt cargo-hakari workspace-hack [Complete]

- **Intent:** Unify feature flags across crates so per-crate builds share
  dependency compilations instead of rebuilding with different feature sets
- **Expected Outcome:** A `workspace-hack` crate exists, is listed as a
  dev-dependency in every workspace member, and `cargo hakari generate --diff`
  reports no drift; per-crate builds observe noticeably fewer dependency
  recompilations
- **Scope:** `Cargo.toml` (workspace), `crates/workspace-hack/` (new),
  per-crate `Cargo.toml` files
- **Dependencies:** RUSTNX-007
- **Validation:** `cargo hakari verify` exits 0; `cargo hakari generate
  --diff` reports no changes; CI adds a `cargo hakari verify` step
- **Confidence:** medium
- **Risks:** Hakari can interact awkwardly with `publish = true` crates;
  none of Anvil's crates publish to crates.io today, so this should be
  safe, but worth confirming before the module ships any
- **Resolution:** Workspace-hack crate scaffolded at `crates/workspace-hack/`
  (private, `publish = false`, `LicenseRef-Proprietary`). Every workspace
  member depends on it via `workspace-hack.workspace = true` using the
  workspace-dotted line style. `.config/hakari.toml` pins the target triples
  to match the cross-compile matrix. All other internal crates now also
  carry `publish = false` so cargo-deny's `allow-wildcard-paths` covers the
  intra-workspace path deps. CI enforces freshness via a new `hakari` job
  that runs `cargo hakari verify` and `cargo hakari generate --diff`.

### RUSTNX-009: Add cargo-deny CI gate [Complete]

- **Intent:** Match TypeScript `pnpm audit` with Rust-side licence,
  advisory, and dependency-ban checks
- **Expected Outcome:** `deny.toml` at repo root defines licence allow-list,
  advisory policy, and crate ban list; CI runs `cargo deny check` on every
  Rust PR; failures block merge
- **Scope:** `deny.toml` (new), `.github/workflows/rust.yml` (or new
  dedicated workflow)
- **Dependencies:** RUSTNX-001
- **Validation:** `cargo deny check` runs green locally and in CI; a
  deliberate policy violation (e.g. adding a GPL-licensed dep in a fixture)
  is rejected
- **Confidence:** high
- **Non-scope:** Yanked-crate detection (already covered by
  `cargo audit`-style tooling elsewhere), SBOM generation, signature
  verification
- **Resolution:** `deny.toml` enforces advisories (v2), licences (v2
  allowlist: MIT, Apache-2.0 + LLVM, BSD-2/3, ISC, Unicode-DFS-2016,
  Unicode-3.0, CC0-1.0, Zlib, MPL-2.0, LicenseRef-Proprietary), bans
  (`multiple-versions = warn`, `wildcards = deny`, `allow-wildcard-paths =
  true`, `workspace-hack` skipped), and sources (crates.io only). CI adds a
  `deny` job. `about.toml` + `about.hbs` generate
  `crates/anvil-cli/resources/third-party-notices.md`, committed to git and
  enforced fresh by the CI `about-diff` job. The `anvil licenses`
  subcommand prints the embedded attribution at runtime, honouring
  `NO_COLOR` and `--no-tui` for colour suppression.

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
| ---- | ------ | ---------- | ---------- |
| `target/` caching produces stale artifacts | high | medium | Prefer caching compiled dependencies only; let `target/<workspace>/` rebuild per job |
| Nx input declarations miss a file → affected skips real work | high | medium | Broad defaults (`{projectRoot}/**/*.rs`, `Cargo.lock`, toolchain file); audit with a "touch everything" probe PR |
| Nextest behavioural drift vs cargo test | medium | low | Keep `cargo test --doc` for doctests; run both in parallel during migration week |
| Hakari churn in small PRs | low | medium | Add `cargo hakari generate` to a commit hook or scripted task |
| Cross-compile matrix breaks under new cache keys | medium | low | Scope rust-cache keys to include target triple |

## Decisions

No new ADRs required — this module implements infrastructure that extends
existing decisions (D-007 Pulumi/monorepo tooling, D-011a Rust core engine).
If Hakari adoption (RUSTNX-008) proves contentious, spin out a lightweight
ADR before implementing.

## Open Questions

- [ ] Merge `rust.yml` into `ci.yml` during RUSTNX-007, or keep separate?
      Separate is simpler; merged enables true TS+Rust affected runs in one
      workflow
- [ ] Is sccache worth adding on top of rust-cache + Nx caching, or does it
      become redundant once RUSTNX-005 lands?
- [ ] Should `anvil-bench` participate in `nx affected -t test`, or stay
      opt-in only (`nx run anvil-bench:bench`)?
