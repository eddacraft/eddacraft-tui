# Test Integration Surface

| ID   | Owner      | Status   | Progress |
| ---- | ---------- | -------- | -------- |
| TINT | @eddacraft | Proposed | 0/15     |

**Last reviewed:** 2026-05-28 — work items given explicit `Status:` fields.
**Phases 1–2 (TINT-001..-011) fleshed to Ready**: the `apps/e2e/` harness and
its `cli/` + `helpers/cli-runner.ts` surfaces exist, the Rust binary is invoked
as a subprocess (ADR-030), and `crates/anvil-tui/src/snapshots/` already carries
`insta` snapshot infra for TINT-010. **Phase 3 (TINT-012..-015) re-scoped to
`Proposed` and flagged needs-design**: the original "build a mock daemon harness
before the daemon exists" premise is stale — the intercept daemon
(`anvil-intercept`, INTD archived 16/16, shipped in `v0.7.0-beta`) now exists
and already carries Rust-side lifecycle / shutdown / health / concurrency
coverage (`crates/anvil-intercept/tests/*`, `supervisor/lifecycle.rs`,
`wait_for_shutdown_signal`). Whether TINT should add a TS subprocess daemon
harness, narrow to an IPC contract test from TS, or defer to the Rust-side
coverage is an open design call. Module stays **Proposed** — not Ready — until
Phase 3 is re-scoped. Earlier (2026-04-26): TFIX, RCLI, KERN archived Complete;
the Ready Checklist prerequisite blockers are cleared.

## Purpose

Unit tests verify components in isolation; TCOV raises that coverage. But the
boundaries between subsystems — where TypeScript shells out to the Rust binary,
where the CLI drives the kernel watcher, where OPA results flow through the gate
pipeline — are structurally invisible to unit tests. These seams are where
production failures happen.

This module tests the integration surfaces: subprocess contracts, CLI-driven
workflows, TUI snapshot regression, watcher end-to-end behaviour, and daemon
lifecycle scaffolding. It prepares the test harness for the daemon before the
daemon itself is built.

## In Scope

- TS→Rust subprocess contract tests (stdout/stderr structure, exit codes, error
  propagation)
- CLI E2E expansion in `apps/e2e/` — `anvil watch`, `anvil gate` with real OPA,
  `anvil hooks`, `anvil export`
- TUI snapshot regression tests — render surfaces to a virtual terminal buffer,
  compare against golden snapshots
- Watcher integration tests — `anvil watch` as a subprocess with real filesystem
  events, verifying event output
- Daemon test harness scaffolding — start/stop lifecycle, health check, graceful
  shutdown, signal handling
- Full gate pipeline E2E — CLI invokes gate, gate invokes OPA, result reported
  to stdout

## Out of Scope

- Unit-level coverage (TCOV)
- CI infrastructure (TFIX)
- External service boundaries (TEXT)
- Daemon implementation (separate module)
- Web/API E2E — Playwright tests for website/docs-site already run in CI

## Interfaces

**Depends on:**

- TFIX (archived, Complete) — E2E harness enabled, OPA in CI
- RCLI (archived, Complete) — Rust CLI commands available for testing
- KERN (archived, Complete) — kernel watcher wired
- `apps/e2e/` — existing E2E harness and CLI runner

**Exposes:**

- Subprocess contract test patterns (reusable for new CLI commands)
- TUI golden snapshot test infrastructure
- Daemon test harness (reusable when daemon is implemented)

## Risks

| Risk                                            | Impact | Mitigation                                                |
| ----------------------------------------------- | ------ | --------------------------------------------------------- |
| Rust binary not built in E2E CI job             | high   | Add `cargo build` step before E2E; cache the binary       |
| Watcher tests are timing-sensitive              | medium | Use event-driven assertions with timeout, not sleep       |
| TUI snapshots break on terminal size changes    | low    | Fix virtual terminal to 80x24; document in test setup     |
| Daemon harness built before daemon exists       | low    | Scaffold with a mock long-running process; swap later     |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] TFIX Phase 1 complete (E2E harness, OPA in CI) — archived 2026-04
- [x] RCLI gate and watch commands functional — archived
- [x] KERN watcher event output stabilised — archived
- [ ] Phase 3 (TINT-012..-015) re-scoped against the now-shipped intercept
      daemon, or closed as superseded by the Rust-side daemon coverage
- [x] TINT-005 CI-build premise reconciled — closed as Superseded 2026-06-01 by
      the shipped graceful-skip `e2e-harness` job design

> **Readiness note:** Phases 1–2 (TINT-001..-004, -006..-011) are individually
> **Ready** and may be picked up now. The module-level status stays **Proposed**
> because Phase 3 still carries an unresolved scope decision (see its per-item
> notes); TINT-005 was settled 2026-06-01 (Superseded). Promote the module to
> Ready once Phase 3 is settled.

## Work Items

### Phase 1 — TS↔Rust Boundary

### TINT-001: subprocess stdout/stderr contract tests

- **Intent:** Define and test the structured output contract between the TS
  layer and the Rust binary. When TS calls `anvil gate`, `anvil check`, etc.,
  the stdout format (JSON, human-readable) and stderr format (error messages,
  diagnostics) must be stable.
- **Expected Outcome:** Contract tests in `apps/e2e/` verify stdout structure,
  field presence, and JSON parsability for each command that TS consumes.
- **Files:**
  - `apps/e2e/src/cli/` (new contract test file)
  - `apps/e2e/src/helpers/cli-runner.ts`
- **Dependencies:** —
- **Validation:** Contract tests pass against the built Rust binary
  (`pnpm --filter @eddacraft/anvil-e2e test`); suite skips gracefully when the
  binary is absent via `cliBinaryAvailable()`.
- **Confidence:** high
- **Status:** Ready

### TINT-002: exit code contract tests

- **Intent:** Exit codes are the primary signal for CI integration and TS
  callers. Verify that each command returns correct codes for success, failure,
  and error conditions.
- **Expected Outcome:** Tests assert specific exit codes for: clean pass,
  warnings found, errors found, invalid args, missing config, OPA unavailable.
- **Files:**
  - `apps/e2e/src/cli/` (extend or new file)
- **Dependencies:** —
- **Validation:** Exit code assertions pass for all documented conditions
  (`pnpm --filter @eddacraft/anvil-e2e test`).
- **Confidence:** high
- **Status:** Ready

### TINT-003: error propagation across the binary boundary

- **Intent:** When the Rust binary fails (panic, OPA crash, filesystem error),
  the error must propagate correctly to the TS caller — no silent swallowing,
  no raw panic traces.
- **Expected Outcome:** Tests trigger error conditions (missing workspace,
  corrupt config, OPA timeout) and verify the TS-visible error message and
  exit code.
- **Files:**
  - `apps/e2e/src/cli/` (new error boundary test file)
- **Dependencies:** —
- **Validation:** All error conditions produce expected stderr and exit code
  (`pnpm --filter @eddacraft/anvil-e2e test`).
- **Confidence:** medium — some error conditions are hard to trigger reliably
- **Status:** Ready

### TINT-004: version and capability negotiation

- **Intent:** As the Rust binary evolves, the TS layer needs to detect which
  commands and flags are available. Test `--version` output parsing and
  capability detection.
- **Expected Outcome:** Tests verify version string format, feature flags in
  `--version --json` (if supported), and graceful handling of unknown commands.
- **Files:**
  - `apps/e2e/src/cli/`
- **Dependencies:** —
- **Validation:** Version parsing tests pass
  (`pnpm --filter @eddacraft/anvil-e2e test`).
- **Confidence:** high
- **Status:** Ready

### TINT-005: build Rust binary in E2E CI step

- **Intent:** The E2E harness needs the built Rust binary. Add a step to build
  it (or use a cached artifact) before E2E tests run.
- **Premise conflict (needs-decision):** The shipped `e2e-harness` job in
  `.github/workflows/ci.yml` deliberately does **not** build the Rust binary —
  the inline comment states "CLI suites are Rust-only and skip themselves when
  the anvil binary is absent (see apps/e2e helpers), so the Rust CLI is not
  required in this job." Building the binary in the TS E2E job contradicts that
  shipped design. Decide first: (a) keep the graceful-skip model and close this
  item as obsolete, or (b) add a separate CLI-E2E job that builds/caches the
  binary and runs only the CLI suites. This is a CI-design call, not a
  flesh-out.
- **Expected Outcome:** Decision taken 2026-06-01 — option (a): graceful-skip is
  ratified as the design; no CI-build step is added.
- **Files:**
  - `.github/workflows/ci.yml`
- **Dependencies:** TFIX (archived Complete — original `TFIX-002` reference is
  stale; the E2E harness + OPA-in-CI foundation it named is in place).
- **Validation:** A CLI-E2E job finds and runs the Rust binary, or the item is
  closed as superseded by the graceful-skip model.
- **Confidence:** low — premise conflicts with shipped CI design.
- **Status:** Superseded 2026-06-01 — closed in favour of the shipped
  graceful-skip e2e design (decision: option (a)). The `e2e-harness` job
  (`.github/workflows/ci.yml`) deliberately does not build the Rust binary, and
  the CLI suites skip themselves when it is absent
  (`apps/e2e/src/cli/commands.e2e.test.ts:15` — `cliBinaryAvailable() ? describe
  : describe.skip`, backed by `apps/e2e/src/helpers/cli-runner.ts`). Rust CLI
  behaviour is already covered by `cargo test` in `.github/workflows/rust.yml`,
  so building the
  binary in the TS E2E job would reverse a deliberate "Rust CLI is not required
  in this job" decision for no net coverage gain. Reopen only if a TS-driven
  CLI-E2E job (option (b)) is later justified.

### Phase 2 — CLI E2E Expansion

### TINT-006: anvil gate E2E with real OPA

- **Intent:** Test the full `anvil gate` flow as a subprocess — loads config,
  runs checks (including OPA policy), and reports results.
- **Expected Outcome:** E2E test runs `anvil gate` in a fixture workspace with
  OPA policies, verifying pass/fail output and exit codes.
- **Files:**
  - `apps/e2e/src/cli/gate-with-opa.e2e.test.ts`
- **Dependencies:** TFIX (archived Complete — OPA-in-CI + harness foundation;
  original `TFIX-003`/`TFIX-004` references are stale ID forms).
- **Validation:** Test passes with OPA installed; skips gracefully without
  (`pnpm --filter @eddacraft/anvil-e2e test`).
- **Confidence:** medium
- **Status:** Ready

### TINT-007: anvil watch E2E with filesystem events

- **Intent:** Test `anvil watch` as a subprocess — start, detect file creation,
  report event, stop cleanly.
- **Expected Outcome:** E2E test starts `anvil watch` on a temp directory,
  creates a file, asserts the event appears in stdout, sends SIGINT, verifies
  clean exit.
- **Files:**
  - `apps/e2e/src/cli/watch.e2e.test.ts`
- **Dependencies:** KERN watcher wired in CLI (archived Complete).
- **Validation:** Watch test passes with event detection within a bounded
  event-driven timeout (no fixed sleep, per the timing-sensitivity risk).
- **Confidence:** medium — timing-sensitive
- **Status:** Ready

### TINT-008: anvil hooks E2E (install and verify)

- **Intent:** Test `anvil hooks install` in a real git repo, verify the hooks
  are created, run `anvil hooks list`, then `anvil hooks uninstall`.
- **Expected Outcome:** Full lifecycle test in a temp git repo.
- **Files:**
  - `apps/e2e/src/cli/hooks.e2e.test.ts`
- **Dependencies:** —
- **Validation:** Hook files exist after install, gone after uninstall
  (`pnpm --filter @eddacraft/anvil-e2e test`).
- **Confidence:** high
- **Status:** Ready

### TINT-009: anvil export E2E (all formats)

- **Intent:** Test `anvil export` as a subprocess for each format (llms-txt,
  mcp-resource, prompt-fragment) in a fixture workspace.
- **Expected Outcome:** Each format produces parseable output; llms-txt is
  valid markdown, mcp-resource is valid JSON.
- **Non-scope:** New export formats; the test asserts the currently shipped
  format set discovered from `anvil export --help` at test time.
- **Files:**
  - `apps/e2e/src/cli/export.e2e.test.ts`
- **Dependencies:** —
- **Validation:** Format validation passes for all output types
  (`pnpm --filter @eddacraft/anvil-e2e test`).
- **Confidence:** high
- **Status:** Ready

### TINT-010: TUI snapshot regression tests

- **Intent:** Render `anvil-tui` surfaces to a virtual terminal buffer (80x24)
  and compare against golden snapshots. Catches visual regressions without a
  real terminal.
- **Expected Outcome:** Each shipped surface has a golden snapshot; `cargo test`
  fails if output diverges. Extends the existing `crates/anvil-tui/src/snapshots/`
  `insta` infrastructure to cover the surfaces that lack a snapshot rather than
  introducing a new harness.
- **Files:**
  - `crates/anvil-tui/src/snapshots/` (existing `insta` snapshot dir — extend)
  - `crates/anvil-tui/src/surfaces/**` (surface modules needing snapshot tests)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-tui` passes; `cargo insta
  review` shows clean diffs on intentional changes. Fix the virtual terminal to
  80x24 per the snapshot-stability risk.
- **Confidence:** high — `insta` snapshot infra already exists in `anvil-tui`;
  this is gap-filling, not greenfield.
- **Status:** Ready

### TINT-011: full gate pipeline E2E (CLI → gate → OPA → report)

- **Intent:** End-to-end test of the entire gate pipeline invoked via the CLI
  binary: config loading, check discovery, OPA policy evaluation, result
  aggregation, and human-readable + JSON output.
- **Expected Outcome:** A fixture workspace with known violations produces
  expected output structure and exit code.
- **Files:**
  - `apps/e2e/src/cli/gate-pipeline.e2e.test.ts`
- **Dependencies:** TINT-006 (gate-with-OPA fixture + skip-without-OPA pattern).
- **Validation:** Pipeline test passes with all checks enabled including OPA
  (`pnpm --filter @eddacraft/anvil-e2e test`).
- **Confidence:** medium
- **Status:** Ready

### Phase 3 — Daemon Prep

> **Needs-design (2026-05-28):** Phase 3 was written when the daemon did not yet
> exist, so it scaffolds a *mock* daemon harness in `apps/e2e/`. The intercept
> daemon now exists (`crates/anvil-intercept`, INTD archived 16/16, shipped in
> `v0.7.0-beta`) and already carries Rust-side lifecycle, signal/shutdown,
> health, and concurrency coverage (`crates/anvil-intercept/tests/*`,
> `supervisor/lifecycle.rs`, `wait_for_shutdown_signal`, owner-only IPC + DACL
> tests). Before promoting any Phase 3 item to Ready, decide the scope: (a)
> close Phase 3 as superseded by the Rust-side daemon coverage; (b) re-scope to a
> TS-side IPC contract test that drives the real `anvil intercept` daemon as a
> subprocess (no mock); or (c) keep a mock harness for a different, still-future
> long-running surface. All four items below stay **Proposed** until this call
> is made; do not write mock-harness code against the stale premise.

### TINT-012: daemon test harness scaffold

- **Intent:** Build the test harness for a long-running daemon process before
  the daemon itself exists. Use a mock server that listens on a socket and
  responds to health checks.
- **Re-scope (needs-design):** See the Phase 3 note. The "before the daemon
  exists" premise is stale; the real daemon and its Rust-side coverage now
  exist. Resolve the Phase 3 scope decision before authoring.
- **Expected Outcome:** Per the Phase 3 decision — either superseded, or a TS
  subprocess harness that drives the real `anvil intercept` daemon.
- **Files:**
  - `apps/e2e/src/helpers/daemon-harness.ts` (new)
  - `apps/e2e/src/daemon/lifecycle.e2e.test.ts` (new)
- **Dependencies:** Phase 3 scope decision.
- **Validation:** Daemon lifecycle test passes against the chosen target.
- **Confidence:** low — premise stale, scope undecided.
- **Status:** Proposed

### TINT-013: daemon graceful shutdown test

- **Intent:** Verify the daemon shuts down cleanly on SIGTERM and SIGINT —
  in-flight operations complete, resources are released, exit code is 0.
- **Re-scope (needs-design):** The real daemon already has Rust-side
  signal/shutdown coverage (`wait_for_shutdown_signal`, 250 ms shutdown drain).
  Confirm whether a TS-side shutdown E2E adds signal beyond that before
  authoring (Phase 3 decision).
- **Expected Outcome:** Tests assert clean shutdown of the chosen target within
  a timeout.
- **Files:**
  - `apps/e2e/src/daemon/shutdown.e2e.test.ts`
- **Dependencies:** TINT-012, Phase 3 scope decision.
- **Validation:** Shutdown tests pass on Linux and macOS.
- **Confidence:** low — premise stale, scope undecided.
- **Status:** Proposed

### TINT-014: daemon health check pattern

- **Intent:** Define and test the health check contract — HTTP endpoint or
  Unix socket, response format, timeout handling.
- **Re-scope (needs-design):** The real daemon exposes an owner-only IPC
  endpoint and a `health` notification on its event stream (per INTD). Align the
  health-check target with that shipped surface rather than inventing a mock
  contract (Phase 3 decision).
- **Expected Outcome:** Health check test verifies the chosen daemon target
  responds correctly and times out gracefully when the process is dead.
- **Files:**
  - `apps/e2e/src/daemon/health.e2e.test.ts`
- **Dependencies:** TINT-012, Phase 3 scope decision.
- **Validation:** Health check passes when running, fails fast when stopped.
- **Confidence:** low — premise stale, scope undecided.
- **Status:** Proposed

### TINT-015: daemon concurrent request handling

- **Intent:** The daemon will serve multiple concurrent requests (file events,
  gate checks). Test that the harness can handle concurrent connections.
- **Re-scope (needs-design):** The real daemon already handles concurrent
  sessions with per-connection `JoinSet` and stable-order flush (INTD); confirm
  the gap a TS-side concurrency E2E fills before authoring (Phase 3 decision).
- **Expected Outcome:** Tests send concurrent requests to the chosen daemon
  target; all receive correct responses without deadlock or corruption.
- **Files:**
  - `apps/e2e/src/daemon/concurrency.e2e.test.ts`
- **Dependencies:** TINT-012, Phase 3 scope decision.
- **Validation:** All concurrent requests return correct responses.
- **Confidence:** low — premise stale, scope undecided.
- **Status:** Proposed
