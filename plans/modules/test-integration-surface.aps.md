# Test Integration Surface

| ID   | Owner      | Status |
| ---- | ---------- | ------ |
| TINT | @eddacraft | Draft  |

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

- TFIX — E2E harness enabled, OPA in CI
- RCLI — Rust CLI commands must exist to be tested
- KERN — kernel watcher must be wired for watch E2E
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
- [ ] TFIX Phase 1 complete (E2E harness, OPA in CI)
- [ ] RCLI gate and watch commands functional
- [ ] KERN watcher event output stabilised

## Tasks

### Phase 1 — TS↔Rust Boundary

#### TINT-001: subprocess stdout/stderr contract tests

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
- **Validation:** Contract tests pass against the built Rust binary.
- **Confidence:** high

#### TINT-002: exit code contract tests

- **Intent:** Exit codes are the primary signal for CI integration and TS
  callers. Verify that each command returns correct codes for success, failure,
  and error conditions.
- **Expected Outcome:** Tests assert specific exit codes for: clean pass,
  warnings found, errors found, invalid args, missing config, OPA unavailable.
- **Files:**
  - `apps/e2e/src/cli/` (extend or new file)
- **Dependencies:** —
- **Validation:** Exit code assertions pass for all documented conditions.
- **Confidence:** high

#### TINT-003: error propagation across the binary boundary

- **Intent:** When the Rust binary fails (panic, OPA crash, filesystem error),
  the error must propagate correctly to the TS caller — no silent swallowing,
  no raw panic traces.
- **Expected Outcome:** Tests trigger error conditions (missing workspace,
  corrupt config, OPA timeout) and verify the TS-visible error message and
  exit code.
- **Files:**
  - `apps/e2e/src/cli/` (new error boundary test file)
- **Dependencies:** —
- **Validation:** All error conditions produce expected stderr and exit code.
- **Confidence:** medium — some error conditions are hard to trigger reliably

#### TINT-004: version and capability negotiation

- **Intent:** As the Rust binary evolves, the TS layer needs to detect which
  commands and flags are available. Test `--version` output parsing and
  capability detection.
- **Expected Outcome:** Tests verify version string format, feature flags in
  `--version --json` (if supported), and graceful handling of unknown commands.
- **Files:**
  - `apps/e2e/src/cli/`
- **Dependencies:** —
- **Validation:** Version parsing tests pass.
- **Confidence:** high

#### TINT-005: build Rust binary in E2E CI step

- **Intent:** The E2E harness needs the built Rust binary. Add a step to build
  it (or use a cached artifact) before E2E tests run.
- **Expected Outcome:** `cargo build --release -p eddacraft-anvil` runs before E2E;
  binary path injected via env var.
- **Files:**
  - `.github/workflows/ci.yml`
- **Dependencies:** TFIX-002
- **Validation:** E2E CI job finds and runs the Rust binary.
- **Confidence:** high

### Phase 2 — CLI E2E Expansion

#### TINT-006: anvil gate E2E with real OPA

- **Intent:** Test the full `anvil gate` flow as a subprocess — loads config,
  runs checks (including OPA policy), and reports results.
- **Expected Outcome:** E2E test runs `anvil gate` in a fixture workspace with
  OPA policies, verifying pass/fail output and exit codes.
- **Files:**
  - `apps/e2e/src/cli/gate-with-opa.e2e.test.ts`
- **Dependencies:** TFIX-003, TFIX-004
- **Validation:** Test passes with OPA installed; skips gracefully without.
- **Confidence:** medium

#### TINT-007: anvil watch E2E with filesystem events

- **Intent:** Test `anvil watch` as a subprocess — start, detect file creation,
  report event, stop cleanly.
- **Expected Outcome:** E2E test starts `anvil watch` on a temp directory,
  creates a file, asserts the event appears in stdout, sends SIGINT, verifies
  clean exit.
- **Files:**
  - `apps/e2e/src/cli/watch.e2e.test.ts`
- **Dependencies:** KERN watcher wired in CLI
- **Validation:** Watch test passes with event detection within 5s timeout.
- **Confidence:** medium — timing-sensitive

#### TINT-008: anvil hooks E2E (install and verify)

- **Intent:** Test `anvil hooks install` in a real git repo, verify the hooks
  are created, run `anvil hooks list`, then `anvil hooks uninstall`.
- **Expected Outcome:** Full lifecycle test in a temp git repo.
- **Files:**
  - `apps/e2e/src/cli/hooks.e2e.test.ts`
- **Dependencies:** —
- **Validation:** Hook files exist after install, gone after uninstall.
- **Confidence:** high

#### TINT-009: anvil export E2E (all formats)

- **Intent:** Test `anvil export` as a subprocess for each format (llms-txt,
  mcp-resource, prompt-fragment) in a fixture workspace.
- **Expected Outcome:** Each format produces parseable output; llms-txt is
  valid markdown, mcp-resource is valid JSON.
- **Files:**
  - `apps/e2e/src/cli/export.e2e.test.ts`
- **Dependencies:** —
- **Validation:** Format validation passes for all output types.
- **Confidence:** high

#### TINT-010: TUI snapshot regression tests

- **Intent:** Render `anvil-tui` surfaces to a virtual terminal buffer (80x24)
  and compare against golden snapshots. Catches visual regressions without a
  real terminal.
- **Expected Outcome:** Each surface (welcome, doctor, status, gate, watch,
  wizard) has a golden snapshot; `cargo test` fails if output diverges.
- **Files:**
  - `crates/anvil-tui/src/snapshots/` (golden snapshot files)
  - `crates/anvil-tui/src/**` (TUI modules with snapshot-based tests)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-tui -- snapshot` passes; `cargo insta
  review` shows clean diffs on intentional changes.
- **Confidence:** high

#### TINT-011: full gate pipeline E2E (CLI → gate → OPA → report)

- **Intent:** End-to-end test of the entire gate pipeline invoked via the CLI
  binary: config loading, check discovery, OPA policy evaluation, result
  aggregation, and human-readable + JSON output.
- **Expected Outcome:** A fixture workspace with known violations produces
  expected output structure and exit code.
- **Files:**
  - `apps/e2e/src/cli/gate-pipeline.e2e.test.ts`
- **Dependencies:** TINT-006
- **Validation:** Pipeline test passes with all checks enabled including OPA.
- **Confidence:** medium

### Phase 3 — Daemon Prep

#### TINT-012: daemon test harness scaffold

- **Intent:** Build the test harness for a long-running daemon process before
  the daemon itself exists. Use a mock server that listens on a socket and
  responds to health checks.
- **Expected Outcome:** Harness can start a process, wait for health check,
  send commands, and verify shutdown. Reusable for the real daemon.
- **Files:**
  - `apps/e2e/src/helpers/daemon-harness.ts` (new)
  - `apps/e2e/src/daemon/lifecycle.e2e.test.ts` (new, uses mock)
- **Dependencies:** —
- **Validation:** Mock daemon lifecycle test passes.
- **Confidence:** high

#### TINT-013: daemon graceful shutdown test

- **Intent:** Verify the daemon shuts down cleanly on SIGTERM and SIGINT —
  in-flight operations complete, resources are released, exit code is 0.
- **Expected Outcome:** Tests send signals to the mock daemon and assert
  clean shutdown within a timeout.
- **Files:**
  - `apps/e2e/src/daemon/shutdown.e2e.test.ts`
- **Dependencies:** TINT-012
- **Validation:** Shutdown tests pass on Linux and macOS.
- **Confidence:** high

#### TINT-014: daemon health check pattern

- **Intent:** Define and test the health check contract — HTTP endpoint or
  Unix socket, response format, timeout handling.
- **Expected Outcome:** Health check test verifies the mock daemon responds
  correctly and times out gracefully when the process is dead.
- **Files:**
  - `apps/e2e/src/daemon/health.e2e.test.ts`
- **Dependencies:** TINT-012
- **Validation:** Health check passes when running, fails fast when stopped.
- **Confidence:** high

#### TINT-015: daemon concurrent request handling

- **Intent:** The daemon will serve multiple concurrent requests (file events,
  gate checks). Test that the harness can handle concurrent connections.
- **Expected Outcome:** Tests send 10 concurrent requests to the mock daemon,
  all receive correct responses without deadlock or corruption.
- **Files:**
  - `apps/e2e/src/daemon/concurrency.e2e.test.ts`
- **Dependencies:** TINT-012
- **Validation:** All concurrent requests return correct responses.
- **Confidence:** medium
