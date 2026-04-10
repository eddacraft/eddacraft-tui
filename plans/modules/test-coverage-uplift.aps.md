# Test Coverage Uplift

| ID   | Owner      | Status |
| ---- | ---------- | ------ |
| TCOV | @eddacraft | Draft  |

## Purpose

Multiple packages and crates sit well below the 80% line coverage target:
Rust CLI at 49.9%, edda-stack at 42.8%, kindling-integration at 43.8%,
mcp-server at 43.5%, and eddacraft-tui at 59.6%. The OPA policy stacks (both
Rust and TypeScript) mock away the binary entirely — no test invokes a real
`opa eval`.

This module raises coverage to ≥80% for each targeted package/crate and
establishes real OPA binary tests before the compliance packs (CPACKS) module
begins. It focuses on unit and in-process integration tests only — cross-process
and external service boundaries belong to TINT and TEXT.

**Coverage target:** Raise line coverage to ≥80% for each package/crate
explicitly targeted in this module. Not a monorepo-wide gate — packages already
above 80% or not in scope retain their current baselines.

## In Scope

- Rust CLI (`anvil-cli` crate) command-level tests: hooks, admin, export,
  architecture, policy — the untested paths (49.9% → ≥80%)
- OPA/Rego real-binary tests for both the Rust and TS policy stacks
- OPA policy test runner for compliance pack fixture Rego files
- edda-stack contracts layer coverage (42.8% → ≥80%)
- kindling-integration emitter and service coverage (43.8% → ≥80%)
- mcp-server per-resource isolation and config generators (43.5% → ≥80%)
- eddacraft-tui render path coverage (59.6% → ≥80%)

## Out of Scope

- E2E/integration tests across process boundaries (TINT)
- External service contract tests (TEXT)
- CI infrastructure changes (TFIX)
- Coverage thresholds or enforcement gates (remains advisory per TFIX)
- Packages already ≥80% (aps 96.6%, core 83.4%, adapters 83.4%,
  platform-config 100%, platform-storage 90.5%)

## Interfaces

**Depends on:**

- TFIX — OPA binary in CI (required for Phase 2)
- `crates/anvil-cli/` — Rust CLI source
- `crates/anvil-policy/` — Rust OPA stack
- `crates/eddacraft-tui/` — shared TUI widgets
- `packages/anvil/policy/` — TS OPA stack
- `packages/edda-stack/` — Edda/Ember/Stack contracts and services
- `packages/kindling-integration/` — emitters and service layer
- `packages/mcp-server/` — MCP tools, resources, transports

**Exposes:**

- ≥80% line coverage for all targeted packages/crates
- Real OPA evaluation tests (usable as a pattern for CPACKS)
- Per-resource test isolation pattern for mcp-server

## Risks

| Risk                                              | Impact | Mitigation                                                   |
| ------------------------------------------------- | ------ | ------------------------------------------------------------ |
| OPA tests require binary — CI failures if missing | high   | TFIX-003/004 as hard dependency; skip-if-absent fallback     |
| edda-stack contracts are mostly types/schemas      | low    | Test via Zod parse round-trips, not logic coverage           |
| Rust CLI commands depend on filesystem state       | medium | Use `tempfile::TempDir` and `assert_cmd` test patterns       |
| 80% target may be unrealistic for some crates     | low    | Per-crate assessment in each task; adjust if justified        |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] Coverage targets set (≥80% per targeted package/crate)
- [ ] TFIX Phase 1 complete (OPA in CI)
- [ ] Phase 1 tasks validated against current untested surface

## Tasks

### Phase 1 — Rust CLI Commands

#### TCOV-001: test hooks command (install/uninstall/list)

- **Intent:** The `hooks` command handles git hook lifecycle but has zero test
  coverage for install, uninstall, and list operations.
- **Expected Outcome:** Tests cover install to a temp git repo, uninstall,
  list with and without hooks present, and error cases (not a git repo).
- **Files:**
  - `crates/anvil-cli/src/commands/hooks.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- hooks` passes; llvm-cov shows
  ≥80% for the hooks module.
- **Confidence:** high

#### TCOV-002: test admin command (approve workflow)

- **Intent:** The admin approval workflow is untested. Cover the approve and
  reject paths including validation and error handling.
- **Expected Outcome:** Tests exercise approve, reject, invalid token, and
  missing user scenarios.
- **Files:**
  - `crates/anvil-cli/src/commands/admin.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- admin` passes.
- **Confidence:** medium — may need mock HTTP layer for API calls

#### TCOV-003: test export command (all format paths)

- **Intent:** The export command has multiple output formats (llms-txt,
  mcp-resource, prompt-fragment) but none are tested at the command level.
- **Expected Outcome:** Tests verify each format produces expected output
  structure from a fixture workspace.
- **Files:**
  - `crates/anvil-cli/src/commands/export.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- export` passes.
- **Confidence:** high

#### TCOV-004: test architecture command (validation paths)

- **Intent:** Architecture validation at the command level is untested —
  covers definition loading, rule evaluation, and violation reporting.
- **Expected Outcome:** Tests with valid/invalid architecture definitions,
  missing config, and violation output formatting.
- **Files:**
  - `crates/anvil-cli/src/commands/architecture.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- architecture` passes.
- **Confidence:** high

#### TCOV-005: test policy command (eval path)

- **Intent:** The `policy eval` subcommand's output parsing and formatting
  are untested at the command level.
- **Expected Outcome:** Tests cover eval with fixture policies, empty results,
  and error formatting.
- **Files:**
  - `crates/anvil-cli/src/commands/policy.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- policy` passes.
- **Confidence:** medium — eval path depends on OPA output structure

#### TCOV-006: test gate command (--plan and --no-cache flags)

- **Intent:** The `--plan` and `--no-cache` flags are scaffolded dead code.
  Either wire and test them, or remove them.
- **Expected Outcome:** Flags are wired to behaviour and tested, or removed
  with the dead code annotations.
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- gate` passes; no
  `#[allow(dead_code)]` remains on these flags.
- **Confidence:** medium

#### TCOV-007: test watch command (--file and --action args)

- **Intent:** The `--file` and `--action` args are marked dead code. Wire
  them to the watcher filter and test.
- **Expected Outcome:** `--file` filters watch events; `--action` selects
  handler. Both tested with fixture file events.
- **Files:**
  - `crates/anvil-cli/src/commands/watch.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- watch` passes; dead code
  annotations removed.
- **Confidence:** medium — requires watcher test infrastructure

#### TCOV-008: test auth device flow

- **Intent:** The device flow authentication touches network. Add tests with
  a mock HTTP server (wiremock or similar).
- **Expected Outcome:** Tests cover successful auth, timeout, invalid code,
  and polling behaviour.
- **Files:**
  - `crates/anvil-cli/src/auth/device_flow.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil -- device_flow` passes.
- **Confidence:** medium — mock HTTP adds test complexity

### Phase 2 — OPA Real-Binary Tests

#### TCOV-009: TS OPA executor real-binary integration test

- **Intent:** `opa-executor.ts` currently has tests that mock the binary.
  Add integration tests that invoke a real `opa eval` against fixture policies.
- **Expected Outcome:** Tests run `opa eval` with `change_scope.rego` and
  `security_baseline.rego` fixtures, asserting correct pass/fail results.
- **Files:**
  - `packages/anvil/policy/src/opa-executor.test.ts` (or new integration file)
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/`
- **Dependencies:** TFIX-003 (OPA in CI)
- **Validation:** Tests pass locally with OPA installed and in CI.
- **Confidence:** high

#### TCOV-010: Rust OPA executor real-binary integration test

- **Intent:** `crates/anvil-policy/src/opa.rs` tests assert OPA-not-found.
  Add tests that invoke real OPA.
- **Expected Outcome:** Tests run `opa eval` via the Rust executor against
  fixture policies with expected results.
- **Files:**
  - `crates/anvil-policy/src/opa.rs`
  - `crates/anvil-policy/tests/` (new integration test file)
- **Dependencies:** TFIX-004 (OPA in Rust CI)
- **Validation:** `cargo test -p eddacraft-anvil-policy` passes with OPA installed.
- **Confidence:** high

#### TCOV-011: run opa test against fixture Rego files

- **Intent:** The `.rego` test files (`change_scope_test.rego`,
  `security_baseline_test.rego`, `coverage_min_test.rego`) exist but are never
  run via `opa test` in CI.
- **Expected Outcome:** A CI step or test case runs `opa test` against all
  `*_test.rego` fixtures and reports results.
- **Files:**
  - `packages/anvil/runtime/src/gate/__fixtures__/policies/*_test.rego`
  - `.github/workflows/ci.yml` (or a test file that shells out to `opa test`)
- **Dependencies:** TFIX-003
- **Validation:** `opa test` passes against all fixture Rego files.
- **Confidence:** high

#### TCOV-012: gate pipeline integration with real OPA

- **Intent:** Test the full gate pipeline (gate-runner → policy check → OPA
  executor → result) with a real OPA binary, not mocked.
- **Expected Outcome:** Integration test exercises the gate runner with policy
  checks enabled, invoking real OPA, and asserting correct gate pass/fail.
- **Files:**
  - `packages/anvil/runtime/src/gate/integration.test.ts` (extend)
  - `packages/anvil/runtime/src/gate/checks/policy.check.ts`
- **Dependencies:** TFIX-003, TCOV-009
- **Validation:** Integration test passes with OPA installed.
- **Confidence:** medium — may surface latent issues in the policy check wiring

#### TCOV-013: OPA test pattern documentation

- **Intent:** Document the pattern for writing OPA policy tests so CPACKS
  contributors can follow it. Include fixture structure, naming conventions,
  and CI requirements.
- **Expected Outcome:** A guide in `docs/guides/` that a developer can follow
  to add a new policy pack with tests.
- **Files:**
  - `docs/guides/opa-policy-testing.md`
- **Dependencies:** TCOV-009, TCOV-010, TCOV-011
- **Validation:** Manual review — a developer can follow the guide to write
  and run a new policy test.
- **Confidence:** high

### Phase 3 — TypeScript Package Coverage

#### TCOV-014: edda-stack contracts layer tests

- **Intent:** The contracts layer (938-line `edda-extended.ts`, memory types,
  proposal types, identifiers, temporal, confidence) is exercised only
  incidentally via service tests. Add Zod parse round-trip tests.
- **Expected Outcome:** Each schema in the contracts layer has a dedicated test
  exercising valid parsing, invalid input rejection, and edge cases.
- **Files:**
  - `packages/edda-stack/src/contracts/` (all schema files)
- **Dependencies:** —
- **Validation:** `pnpm vitest run packages/edda-stack` shows contracts layer
  ≥80% line coverage.
- **Confidence:** high

#### TCOV-015: edda-stack port interface tests

- **Intent:** Port interfaces (`edda.port.ts`, `ember.port.ts`,
  `kindling.port.ts`) define contracts but have no validation tests.
- **Expected Outcome:** Tests verify that mock implementations satisfy the port
  interfaces and that the existing testing mocks are correct.
- **Files:**
  - `packages/edda-stack/src/contracts/ports/`
  - `packages/edda-stack/src/testing/mocks/`
- **Dependencies:** —
- **Validation:** Port tests pass; edda-stack coverage rises.
- **Confidence:** high

#### TCOV-016: edda-stack store interfaces and migration tests

- **Intent:** `store-interfaces.ts` and the migration module have partial
  coverage. Ensure store operations and migration paths are fully tested.
- **Expected Outcome:** Store interface operations tested via mock
  implementations; migration tested with fixture data from prior versions.
- **Files:**
  - `packages/edda-stack/src/edda/store-interfaces.ts`
  - `packages/edda-stack/src/edda/migration/`
- **Dependencies:** —
- **Validation:** ≥80% line coverage for store and migration modules.
- **Confidence:** medium

#### TCOV-017: kindling-integration emitter tests

- **Intent:** All 7 emitters (action, constraint, error, gate, human-input,
  plan, session) have zero test coverage. They are the primary output surface.
- **Expected Outcome:** Each emitter has a test file verifying event shape,
  required fields, and error handling.
- **Files:**
  - `packages/kindling-integration/src/emitters/*-emitter.ts`
- **Dependencies:** —
- **Validation:** `pnpm vitest run packages/kindling-integration` shows ≥80%.
- **Confidence:** high

#### TCOV-018: kindling-integration service and adapter tests

- **Intent:** `kindling-service.ts`, `adapter.ts`, `config.ts`, and supporting
  modules (`query-service.ts`, `retention.ts`, `status.ts`) are untested.
- **Expected Outcome:** Service lifecycle (init, query, shutdown), adapter
  wiring, config validation, query limits, and retention logic all tested.
- **Files:**
  - `packages/kindling-integration/src/kindling-service.ts`
  - `packages/kindling-integration/src/adapter.ts`
  - `packages/kindling-integration/src/config.ts`
  - `packages/kindling-integration/src/query-service.ts`
  - `packages/kindling-integration/src/retention.ts`
  - `packages/kindling-integration/src/status.ts`
- **Dependencies:** —
- **Validation:** ≥80% line coverage for the package.
- **Confidence:** medium

#### TCOV-019: mcp-server per-resource isolation tests

- **Intent:** 8 resources share one test file (`resources.test.ts`). Isolate
  each resource into its own test file for clarity and coverage precision.
- **Expected Outcome:** Each resource (`baseline`, `boundaries`, `config`,
  `constraints`, `drift`, `file-warnings`, `patterns`, `suppressions`) has a
  dedicated test file.
- **Files:**
  - `packages/mcp-server/src/resources/*.resource.ts`
- **Dependencies:** —
- **Validation:** Each resource test file passes independently; combined
  coverage ≥80%.
- **Confidence:** high

#### TCOV-020: mcp-server config generator tests

- **Intent:** Existing tests exercise `generateMcpConfig(...)` for supported
  targets, but the per-editor config generators (`claude-code.ts`, `cursor.ts`,
  `vscode.ts`, `windsurf.ts`) lack direct tests and edge-case coverage.
- **Expected Outcome:** Each generator is directly tested for correct output
  structure, path handling, and edge cases (missing editor, custom paths).
- **Files:**
  - `packages/mcp-server/src/config/*.ts`
- **Dependencies:** —
- **Validation:** Config module reaches ≥80% line coverage.
- **Confidence:** high

#### TCOV-021: mcp-server transport and entry point tests

- **Intent:** `streamable-http.ts` has partial coverage; `bin.ts` and
  `bin-http.ts` are untested entry points.
- **Expected Outcome:** Transport tested for connection lifecycle, error
  handling, and concurrent requests. Entry points tested via process spawn
  with `--help` / `--version`.
- **Files:**
  - `packages/mcp-server/src/transports/streamable-http.ts`
  - `packages/mcp-server/src/bin.ts`
  - `packages/mcp-server/src/bin-http.ts`
- **Dependencies:** —
- **Validation:** ≥80% line coverage for the mcp-server package.
- **Confidence:** medium — entry points may need process-level testing

### Phase 4 — Rust TUI Coverage

#### TCOV-022: eddacraft-tui widget interaction tests

- **Intent:** Widgets have unit tests for rendering but not for user
  interaction flows (key handling, state transitions, focus cycling).
- **Expected Outcome:** Each interactive widget (`text_input`, `select`,
  `confirm`, `log_panel`) has interaction tests covering key sequences.
- **Files:**
  - `crates/eddacraft-tui/src/widgets/`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-tui` passes; llvm-cov shows ≥80%.
- **Confidence:** high

#### TCOV-023: eddacraft-tui shell and theme coverage

- **Intent:** The `shell` module has snapshot tests but the theme module
  (`eddacraft` theme) only tests colour distinctness — not style application.
- **Expected Outcome:** Theme styles tested for correct foreground/background
  on each semantic token. Shell tested for responsive layout at different
  terminal sizes.
- **Files:**
  - `crates/eddacraft-tui/src/shell.rs`
  - `crates/eddacraft-tui/src/theme/`
- **Dependencies:** —
- **Validation:** Combined module coverage ≥80%.
- **Confidence:** high

#### TCOV-024: eddacraft-tui keyboard handler edge cases

- **Intent:** Keyboard handler tests cover basic navigation and quit but not
  modifier keys, rapid input, or unknown key handling.
- **Expected Outcome:** Tests for modifier combinations, unmapped keys, and
  rapid sequential input.
- **Files:**
  - `crates/eddacraft-tui/src/keyboard/handler.rs`
- **Dependencies:** —
- **Validation:** Handler module at ≥80% line coverage.
- **Confidence:** high

#### TCOV-025: eddacraft-tui surface trait compliance tests

- **Intent:** Verify all Surface trait implementations satisfy the full
  interface contract (render, handle_key, metadata, lifecycle).
- **Expected Outcome:** A parameterised test that runs the trait contract
  against every registered surface.
- **Files:**
  - `crates/eddacraft-tui/src/surface.rs`
- **Dependencies:** —
- **Validation:** All surfaces pass the compliance test.
- **Confidence:** high
