<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Engine

| ID     | Owner | Priority | Status      |
| ------ | ----- | -------- | ----------- |
| POLENG | —     | high     | In Progress |

**Last reviewed:** 2026-05-25

> Module promoted to Ready 2026-05-13 once ADR-040 reached Accepted.
> POLENG-001 merged 2026-05-12 (PR #1485), so POLENG-002, POLENG-004, and
> POLENG-006 (which depend only on POLENG-001) are now executable; the
> remaining tasks unlock as their in-module dependencies close (the parser
> evaluates executability per-task, not per-module).

> ADR-040 picks `regorus` as the embedded Rust policy engine. This module
> owns the substrate: facade crate, input data document, builtins surface,
> determinism contract, coverage/trace, CLI entry point, and the bench
> parity gate. Tier-model work (OOB / YAML / Rego authoring) and product
> surface (debugger, NLP gen, watch mode) live in OPAE and successor
> modules; this module deliberately stays scoped to the runtime so OPAE
> can re-platform onto it without dragging the engine choice into every
> downstream Policy Governance task.

## Purpose

Establish the Rust embedded policy evaluation substrate every higher-level
Anvil policy tier depends on. POLENG provides the `crates/anvil-policy-engine`
facade over `regorus`, a stable `PolicyInput` data document, a curated and
audited builtins surface for Anvil-internal data sources, deterministic
evaluation guarantees, first-class coverage and trace, and a CLI entry point.
The bench harness validates ADR-040's parity assumption against Anvil's real
policy mix before downstream modules unblock.

The blocking chain matters: OPAE, ORGHIER, POLLC, COMPLY, POLFED, CPACKS,
ARCHCFG, AGOV, OPAG, IORISK, CPOL, and parts of RCLI2 currently sit on the
post-rust engine question. POLENG is the answer.

## In Scope

- `crates/anvil-policy-engine` facade crate wrapping `regorus` behind an
  Anvil-shaped public API; downstream crates depend on the facade, never on
  `regorus` directly
- `PolicyInput` v1 data document — repo state, plan files, decision log,
  diff, baseline cohort; versioned and snapshot-tested
- First-party builtins surface v1 — `anvil.repo_state()`, `anvil.plan(path)`,
  `anvil.decision(id)`, `anvil.is_new_edge(from, to)`,
  `anvil.baseline_contains(fingerprint)`
- Determinism contract enforcement — no clock, no network, no filesystem
  access except via audited builtins; declarative `DeterminismClass` on every
  registered builtin
- ADR-002 / ADR-003 post-processing applied uniformly to every eval result
  (severity classification, new-edge annotation, default `exit 0` on warnings,
  `--fail-on-warnings` override)
- Coverage and rule-firing trace as first-class `EvalResult` fields (basis
  for the OPAE debugger and POLFED federation reporting)
- CLI surface — `anvil policy eval <policy> [--input <path>] [--explain]
  [--why <finding-id>]` with JSON output and diagnostic envelope alignment
- Bench harness vs. Go OPA reference on Anvil's representative policy mix
  — **gate**: regorus at or above parity on every policy, otherwise ADR-040
  is revisited
- Public engine API surface documented as a stability contract

## Out of Scope

- ❌ Tier model — OOB / YAML / Rego authoring boundaries (separate ADR,
  succeeded by POLENG-Y / POLENG-O / POLENG-X follow-up modules)
- ❌ OOB rule catalogue v1 (POLENG-O successor module)
- ❌ YAML authoring frontend and grammar (POLENG-Y successor module)
- ❌ Custom user `.rego` discovery and loading (POLENG-X successor module)
- ❌ Bundle distribution, signing, fleet federation (OPAE-034..036, POLFED)
- ❌ Watch-mode UX, real-time evaluation on every keystroke (OPAE; constrained
  by ADR-031)
- ❌ Policy debugger TUI, NLP policy generation, impact simulator, PR
  auto-comments (OPAE product surface)
- ❌ Migration of any live Go OPA wiring (none exists in the active Rust tree
  per ADR-040 D-4; archived TS `OpaExecutor` stays archived)
- ❌ New cryptographic builtins as a general extension (first-party shims
  added only when a CPACKS pack hits a concrete gap)

## Interfaces

**Depends on:**

- **ADR-040** — engine choice (`regorus`); pinned minor version
- **`crates/anvil-kernel`** — repo state source feeding `PolicyInput`
- **`crates/anvil-policy`** — pack loader and manifest types
- **POLVAL** — pack manifest schema (input document must align)
- **ADR-031** — validation-latency rubric (bench harness cites the same SLOs)

**Exposes:**

- `anvil_policy_engine::Engine` — facade type (`new`, `eval`,
  `register_builtin`, `coverage`)
- `anvil_policy_engine::PolicyInput` — versioned input data document
- `anvil_policy_engine::Builtin` trait + `DeterminismClass` enum
- `anvil_policy_engine::{Coverage, Trace, EvalResult, Severity}`
- CLI surface — `anvil policy eval` and flags

**Coordinates with:**

- **OPAE** — re-platforms onto POLENG once Ready; OPAE's "we wrap OPA" Out
  of Scope clause resolves
- **CPACKS** — Rego packs run on this engine; CPACKS `NOTE(post-rust)`
  resolves once POLENG-001 lands
- **POLFED** — federation reads coverage/trace data exposed by POLENG-006
- **AIGUARD** — diagnostic envelope shared via POLENG-007 CLI surface
- **INTR / INTD** — daemon hot-path rules stay deterministic and Rust-native
  per ADR-031; POLENG does not run on the intercept hot path (see "Out of
  Scope" in INTR)

## Acceptance Criteria

- [ ] `crates/anvil-policy-engine` builds, tests pass on Linux/macOS/Windows
- [ ] `PolicyInput` schema v1 stable, snapshot-tested, documented at
      `docs/specs/policy-input-v1.md`
- [ ] At least five first-party builtins registered, each with declared
      `DeterminismClass` and unit tests
- [ ] Determinism repeatable-eval test runs a representative policy 100× over
      identical input and asserts byte-identical output
- [ ] `anvil policy eval` returns exit-coded result with JSON output and
      respects ADR-002 (`exit 0` on warnings, override via `--fail-on-warnings`)
- [ ] `--explain` renders coverage report; `--why <finding-id>` renders trace
- [ ] Bench harness shows regorus at or above Go OPA reference on every
      policy in the representative suite (parity gate; ADR-040 trigger)
- [ ] Public engine API surface documented; downstream crates verified to
      depend on the facade only (lint or audit)

## Risks & Mitigations

| Risk                                                              | Mitigation                                                                                              |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| regorus missing builtin discovered after CPACKS adoption          | First-party shim pattern via `Engine::register_builtin`; gap surfaced by POLENG-008 bench harness       |
| `PolicyInput` schema churn breaks CPACKS packs                    | Schema versioning + deprecation policy authored in POLENG-002; snapshot tests catch unintended changes  |
| Determinism leak through a careless builtin                       | `Builtin` trait requires explicit `DeterminismClass` declaration; CI lint blocks unmarked builtins      |
| Performance regression on the watch-mode hot path                 | POLENG-008 bench in CI; ADR-031 latency rubric pins the SLO                                             |
| Single-vendor risk on regorus                                     | Facade isolates engine; downstream depends on `anvil-policy-engine`, never on `regorus` (ADR-040 D-1)   |
| Bench parity gate fails on real policy mix                        | ADR-040 is revisited per its D-5; module stays Draft until parity established                           |

## Tasks

### POLENG-001: Engine facade crate skeleton

- **Status:** Merged 2026-05-12 via PR #1485
- **Intent:** Establish `crates/anvil-policy-engine` over `regorus` with the
  minimal facade types so downstream work can begin
- **Expected Outcome:** Crate added to the workspace; `Engine`,
  `EngineConfig`, `EvalResult`, `EngineError` defined; `Engine::new` +
  `Engine::eval` round-trip a "hello world" Rego policy over an empty
  `PolicyInput`
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine --lib`
- **Files:** `crates/anvil-policy-engine/Cargo.toml`,
  `crates/anvil-policy-engine/src/lib.rs`
- **Dependencies:** ADR-040 (Accepted 2026-05-13) — dependency satisfied
- **Confidence:** high

### POLENG-002: PolicyInput v1 schema

- **Status:** In Progress (2026-05-25)
- **Intent:** Pin the input data document policies receive — repo state,
  plan files, decisions, diff, baseline cohort — as a versioned, stable
  contract
- **Expected Outcome:** `PolicyInput` struct serialises to `regorus`'s input
  format; schema versioned (`v1`); snapshot-tested; spec doc lives at
  `docs/specs/policy-input-v1.md` with deprecation policy
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine --lib input_schema` plus schema-stability snapshot
- **Files:** `crates/anvil-policy-engine/src/input.rs`,
  `docs/specs/policy-input-v1.md`
- **Dependencies:** POLENG-001
- **Coordinates with:** CPACKS (consumes), POLVAL (manifest schema must
  align), `crates/anvil-kernel` (provides repo state)
- **Confidence:** medium
- **Risks:** schema churn — needs explicit deprecation policy from day one

### POLENG-003: First-party builtins surface v1

- **Status:** In Progress (2026-05-25)
- **Intent:** Expose Anvil's data sources as deterministic, audited Rego
  builtins so policy authors can query plan and repo state declaratively
- **Expected Outcome:** `anvil.repo_state()`, `anvil.plan(path)`,
  `anvil.decision(id)`, `anvil.is_new_edge(from, to)`,
  `anvil.baseline_contains(fingerprint)` registered; each builtin declares
  `DeterminismClass::Pure`; unit tests cover positive, negative, and
  malformed-input cases per builtin
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine --lib builtins`
- **Files:** `crates/anvil-policy-engine/src/builtins/`
- **Dependencies:** POLENG-001, POLENG-002
- **Confidence:** medium

### POLENG-004: Determinism contract enforcement

- **Status:** In Progress (2026-05-25)
- **Intent:** Guarantee that no policy can observe a clock, the network, or
  the filesystem except through audited builtins
- **Expected Outcome:** `Builtin` trait requires explicit `DeterminismClass`
  declaration; `Engine` config rejects non-pure builtins unless opted in;
  clippy/CI lint blocks new builtins without a declaration; repeatable-eval
  test runs a representative policy 100× over identical input and asserts
  byte-identical output
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine --lib determinism` plus the workspace lint
- **Files:** `crates/anvil-policy-engine/src/determinism.rs`,
  `crates/anvil-policy-engine/tests/determinism.rs`
- **Dependencies:** POLENG-001
- **Confidence:** medium
- **Note (2026-05-25):** The "block undeclared builtins" requirement is met by
  the type system, not a separate clippy lint: `Builtin::determinism` is a
  required trait method, so a builtin that omits its class does not compile —
  a strictly stronger guarantee than a lint that could be silenced. The impure
  opt-in lives on `EngineConfig::allow_impure_builtins`.
- **Risks:** hidden non-determinism in regorus internals — needs an upstream
  audit before this task closes

### POLENG-005: Result post-processing — warnings-over-blocks + new-edges-only

- **Status:** In Progress (2026-05-25)
- **Intent:** Apply ADR-002 and ADR-003 uniformly to every engine evaluation
  so downstream tiers inherit Anvil's defaults without re-implementing them
- **Expected Outcome:** `EvalResult` carries `Severity` (`Warning` / `Error`)
  and an `is_new_edge` annotation drawn from `anvil.is_new_edge`; default
  exit-code policy is `exit 0` on warnings; `--fail-on-warnings` flag at the
  CLI overrides; baselined findings annotated and excluded from new-edge set
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine --lib post_processing`
- **Files:** `crates/anvil-policy-engine/src/result.rs`
- **Dependencies:** POLENG-001, POLENG-003
- **Confidence:** high

### POLENG-006: Coverage and trace as first-class result fields

- **Status:** In Progress (2026-05-25)
- **Intent:** Expose line-level coverage and rule-firing trace on every
  evaluation so OPAE's debugger and POLFED federation reporting build on a
  shared substrate
- **Expected Outcome:** `EvalResult::coverage()` returns `Coverage` (covered
  / uncovered lines per source file); `EvalResult::trace()` returns the
  rule-firing order with input bindings; coverage renders via
  `anvil policy eval --explain`
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine --lib coverage`
- **Files:** `crates/anvil-policy-engine/src/coverage.rs`,
  `crates/anvil-policy-engine/src/trace.rs`
- **Dependencies:** POLENG-001
- **Confidence:** medium (regorus exposes coverage natively but needs shaping
  to the facade contract)
- **Note (2026-05-25):** Coverage is delivered in full (covered/uncovered lines
  per file, rendered by `--explain`). Trace is **partial by upstream
  constraint:** regorus 0.10.0 exposes no structured rule-firing-order trace
  through its public API — the internal `traces` buffer only gathers explicit
  `trace()` strings and has no `Engine` getter. `EvalResult::trace()` therefore
  captures the query variable bindings regorus *does* surface, and `Trace` is
  shaped so a richer rule-firing trace can populate it without an API break.
  Full trace is follow-up work gated on an upstream regorus capability (or a
  vendored evaluator hook); flagged for OPAE's debugger consumer.

### POLENG-007: CLI surface — `anvil policy eval`

- **Status:** In Progress (2026-05-25)
- **Intent:** Wire the engine into `crates/anvil-cli` so developers and CI
  can evaluate policies from the shell with structured output
- **Expected Outcome:** `anvil policy eval <policy> [--input <path>] [--explain] [--why <finding-id>] [--fail-on-warnings]` produces JSON result + exit code; output uses the shared AIGUARD diagnostic envelope; `--explain` renders coverage; `--why` renders trace for a specific finding
- **Validation:** `cargo test -p eddacraft-anvil --test policy_eval` (the CLI
  package is `eddacraft-anvil`, not `eddacraft-anvil-cli`)
- **Files:** `crates/anvil-cli/src/commands/policy/eval.rs` (added under the
  existing `policy` command group, which became `policy/mod.rs`)
- **Dependencies:** POLENG-001, POLENG-005, POLENG-006
- **Coordinates with:** AIGUARD diagnostic envelope
- **Confidence:** medium
- **Note (2026-05-25):** Implemented as a subcommand of the existing
  licence-gated `policy` group (so `eval` inherits the gate; tests use the
  suite-wide `ANVIL_DEV=1` bypass). No "AIGUARD diagnostic envelope" type
  exists in the CLI yet — output uses the established `crate::output::json`
  envelope and `EXIT_OK`/`EXIT_ERROR` codes; aligning on a shared AIGUARD
  envelope is deferred to when that surface lands. Added a `--query` flag
  (not in the original sketch) because the command needs to know which rule to
  evaluate; it defaults to `data`. `--why` enables trace but is limited by the
  POLENG-006 regorus trace constraint.

### POLENG-008: Bench harness — parity gate vs. Go OPA reference

- **Intent:** Validate ADR-040 D-1's parity assumption against Anvil's real
  policy mix before POLENG moves to Ready
- **Expected Outcome:** Bench suite covers Anvil's representative policies
  (CPACKS samples plus architecture boundary rules) on both regorus and the
  Go OPA reference; reports p50 / p95 / p99 per policy; **gate**: regorus at
  or above Go OPA reference on every measured policy; failure triggers an
  ADR-040 revisit
- **Validation:** `cargo bench -p eddacraft-anvil-policy-engine` plus
  `scripts/bench-vs-go-opa.sh` (CI sidecar job installs Go OPA reference)
- **Files:** `crates/anvil-policy-engine/benches/`,
  `scripts/bench-vs-go-opa.sh`
- **Dependencies:** POLENG-001, POLENG-003 (representative policies need
  real builtins)
- **Confidence:** low (real measurement — could surface gaps)
- **Risks:** Go OPA reference is not trivially installable in CI; sidecar
  job needed. Bench mix needs careful curation to be representative without
  becoming a maintenance burden.
