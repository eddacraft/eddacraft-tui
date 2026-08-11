# Test Coverage Uplift

| ID   | Owner      | Status      | Progress |
| ---- | ---------- | ----------- | -------- |
| TCOV | @eddacraft | In Progress | 26/26    |

## Progress (as of 2026-06-26)

- **TCOV-026 filed 2026-06-26.** Benchmark contract drift: `pnpm bench`,
  `bench.yml`, and `benchmarks/history/` schema diverged after GCALL/GV2-025
  gates landed. New work item aligns the routine suite with CI and extends the
  committed history schema (`secret_scan_parallel`, `walk_discovery`,
  `hot_read`, `call_lift`, resource budgets). Status **In Progress**.
- **TCOV-026 follow-up Merged 2026-08-11 via #3730.** The first full-suite
  comparison after the rule catalogue expanded showed that antipattern timings
  were being compared across different rule workloads and a TypeScript-heavy
  corpus. The merged follow-up adds language-labelled Rust-scanner cases, a
  balanced corpus v2, and automatic catalogue fingerprints in committed
  history.

## Progress (as of 2026-06-20)

- **Ready surface delivered + MCP items reconciled 2026-06-20.** All five Ready
  items merged: TCOV-015 (#2819), TCOV-016 (#2820), TCOV-017 (#2817), TCOV-018
  (#2818), TCOV-025 (#2816). The three mcp-server items (TCOV-019..-021) were
  resolved as **Complete (covered by the Rust RMCPF port, #2809)** — the MCP
  capability returned in Rust already comprehensively tested, so the archived TS
  targets are retired rather than re-tested (owner decision). Done count 14 →
  22/25.
- **Phase 4 re-scoped 2026-06-20.** TCOV-022..-024 were unblocked: the earlier
  "scope drift" call inspected the wrong crate. The targets live in
  `crates/eddacraft-tui/` (the primitives crate that kept the widgets/theme/
  keyboard after the `anvil-tui` app-layer split), not `crates/anvil-tui/`. The
  three items are repointed there against measured current coverage and promoted
  to **Ready** (no count change — Ready is not terminal). The module now has no
  Blocked items: 22/25 done + TCOV-022/023/024 Ready.
- **TCOV-024 implemented 2026-06-20 (#2828).** Keymap edge-case tests landed in
  `eddacraft-tui/src/keyboard/handler.rs` (every `map()` arm now covered). Done
  count 22 → 23/25; TCOV-022/023 remain Ready.
- **TCOV-023 implemented 2026-06-20 (#2829).** Shell footer truncation/watermark-
  priority/degenerate-size tests + theme `fg`-contract tests (incl. a second
  `MinimalTheme` implementor) in `eddacraft-tui/src/{shell,theme}`. Done count
  23 → 24/25; only TCOV-022 (interactive widgets) remains Ready.
- **TCOV-022 implemented 2026-06-20 (#2831) — module fully delivered.**
  `confirm` + `log_panel` state-transition tests (the thin widgets; the
  already-covered `select`/`data_table`/`tree` were not padded). Done count
  24 → **25/25 — every work item is now done.** The module status stays
  `In Progress` until the Complete flip + archive cascade is done as its own
  release-tag-verified PR (per the APS archive process); no Blocked or Ready
  items remain.

## Progress (as of 2026-05-28)

- **Flesh-out pass 2026-05-28.** Phase 3 in-workspace items (TCOV-015..-018,
  edda-stack + kindling-integration) promoted to Ready. Phase 3 mcp-server
  items (TCOV-019..-021) re-classified `Blocked — needs decision` because
  their target was archived under ADR-033 and excluded from the workspace.
  Phase 4: TCOV-025 (surface trait compliance) promoted to Ready against the
  live `eddacraft-tui`-re-export reality; TCOV-022..-024 stay `Blocked —
  scope drift` pending a scope-refresh design call (no `theme/`/`keyboard/`
  dirs; two-widget set). Done count unchanged at 14/25.

## Progress (as of 2026-04-21)

- **Phase 1 — Rust CLI Commands:** Complete (8/8). All `anvil-cli` command and
  `auth/device_flow.rs` test files now carry their own `#[cfg(test)]` modules
  (hooks 26 tests, admin 12, export 37, architecture 15, policy 19, gate
  regression for removed `--no-cache`, watch 29, device_flow 34).
- **Phase 2 — OPA Real-Binary:** Complete (5/5). TCOV-009 (TS),
  TCOV-010 (Rust), TCOV-011 (`opa test` against fixtures), TCOV-012
  (gate-runner → policy.check → real OPA pipeline), and TCOV-013
  (`docs/guides/opa-policy-testing.md`) all landed; fixtures are anchored
  at `policies/fixtures/`. TCOV-010 surfaced and fixed a latent stdio-pipe
  bug in `crates/anvil-policy/src/opa.rs::evaluate`.
- **Phase 3 — TypeScript Packages:** Partial (1/8). `edda-stack` contracts have
  8 dedicated test files (TCOV-014). `kindling-integration` still has only
  `malicious-ai.test.ts`; `mcp-server` resources remain in a single
  `resources.test.ts`; `bin.ts`/`bin-http.ts` untested; ports/store have no
  dedicated tests.
- **Phase 4 — Rust TUI:** Scope drift. The crate was renamed
  `eddacraft-tui` → `anvil-tui` and the directory layout no longer matches
  the original work items (no `theme/` or `keyboard/` subdir; widgets are
  `results_dashboard.rs` and `quick_wins_panel.rs`, not the
  text_input/select/confirm/log_panel set the items name). Phase 4 needs a
  scope refresh against the current crate before any work begins.

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
- `archive/anvil-mcp-server/` — MCP tools, resources, transports

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
- [x] TFIX Phase 1 complete (OPA in CI) — Rust side done (TFIX-004/005/011);
      TS side (TFIX-003) addressed by the `ci.yml` OPA install step pinned to
      the current `DEFAULT_OPA_VERSION`
- [x] Phase 1 tasks validated against current untested surface (all 8
      commands now have in-file unit tests)

## Work Items

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
- **Status:** Complete — `crates/anvil-cli/src/commands/hooks.rs` ships a
  `#[cfg(test)]` module with 26 tests (install/uninstall/list, error paths).

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
- **Status:** Complete — `commands/admin.rs` carries 12 in-file tests covering
  approve/reject/validation paths.

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
- **Status:** Complete — landed in 481962f1 with council follow-up b632b1e2;
  `commands/export.rs` ships 37 tests covering each format.

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
- **Status:** Complete — `commands/architecture.rs` carries 15 in-file tests
  spanning definition load, evaluation, and violation reporting.

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
- **Status:** Complete — `commands/policy.rs` carries 19 in-file tests covering
  fixture eval, empty results, and error formatting.

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
- **Status:** Complete — `--no-cache` removed in bd9a01c3 with a regression
  guard test (`no_cache_flag_removed`). `--plan` resolved during the same pass.

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
- **Status:** Complete — landed in 3ebc4033; `commands/watch.rs` ships 29
  in-file tests for the pure logic of `--file`/`--action` filtering.

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
- **Status:** Complete — landed in 7695175e with council follow-up 82fb0192;
  `auth/device_flow.rs` ships 34 in-file tests covering success, timeout,
  invalid code, and polling behaviour.

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
- **Status:** Complete — landed in d1067241; new
  `packages/anvil/policy/src/opa-real.integration.test.ts` runs 6 evaluate
  cases against `policies/fixtures/` and skips gracefully when `opa` is
  absent.

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
- **Status:** Complete — landed in 9d993071; new
  `crates/anvil-policy/tests/opa_real_binary.rs` runs 7 cases (6 evaluate +
  1 `opa test`) and surfaced a latent stdio-pipe bug in
  `OpaExecutor::evaluate()` which was fixed in the same commit. (Suite later
  deleted 2026-07-04 under ADR-098 AD-1 PR-A — OPA-to-regorus replacement;
  the record above stands as history.)

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
- **Status:** Complete — covered in d1067241 (TS, via `OPAExecutor.runTests`
  call in `opa-real.integration.test.ts`) and 9d993071 (Rust, via
  `opa_test_fixture_rego_files_all_pass`); fixtures live at
  `policies/fixtures/` not under `packages/anvil/runtime/.../__fixtures__/`.

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
- **Status:** Complete — landed in this branch via
  `packages/anvil/runtime/src/gate/policy.integration.test.ts`. Three cases
  drive `GateRunner.runGate` with `policy_dir=.anvil/policies` populated from
  `policies/fixtures/`: large plan fails with `change_scope` violations,
  small plan with `security-review` tag passes, and the loaded policy set
  is asserted to be `change_scope`/`coverage_min`/`security_baseline`.

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
- **Status:** Complete — landed in this branch as
  `docs/guides/opa-policy-testing.md`. Documents the fixture layout
  (`policies/fixtures/`), the `<name>.rego` / `<name>_test.rego`
  convention, the pinned OPA version (`DEFAULT_OPA_VERSION` in
  `opa-binary-manager.ts`), how to run the direct OPA fixture suite plus the TS
  and Rust real-binary integration suites (historical gate pipeline archived
  with the TypeScript scanner), the policy input schema reference, and a
  troubleshooting matrix for the common failures
  surfaced while building Phase 2.

### Phase 3 — TypeScript Package Coverage

> **mcp-server items resolved 2026-06-20 (covered by RMCPF).** TCOV-019, -020,
> and -021 targeted `anvil-archive/anvil-mcp-server/`, archived under
> **ADR-033 (2026-04-29)** and now living in the sibling
> `eddacraft/anvil-archive` repository outside this pnpm workspace. They sat
> `Blocked — needs decision` until the MCP capability
> returned **in Rust** via the RMCPF port (#2809): `crates/anvil-cli/src/mcp/`
> now provides the `anvil://` resources and MCP tools, with the editor-config
> install path in `util.rs`. That port arrived already covered — ~304 MCP tests
> plus 21 install-path tests — so the original coverage intent is met without
> re-testing the dead TS surface. **Owner decision 2026-06-20: reconcile all
> three as Complete (covered-by-RMCPF); the archived TS targets stay retired.**
> The edda-stack and kindling-integration items (TCOV-014..-018) shipped
> separately (TCOV-014 Done earlier; TCOV-015..-018 Merged 2026-06-20).

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
- **Status:** Complete — eight dedicated test files now cover the contracts
  layer (`contracts.test.ts`, `events.test.ts`, `evolution.test.ts`,
  `memory-types.test.ts`, `observation-mappings.test.ts`,
  `proposal-types.test.ts`, `provenance.test.ts`, `type-mappings.test.ts`).

#### TCOV-015: edda-stack port interface tests

- **Intent:** Port interfaces (`edda.port.ts`, `ember.port.ts`,
  `kindling.port.ts`) define contracts but have no validation tests.
- **Expected Outcome:** Tests verify that mock implementations satisfy the port
  interfaces and that the existing testing mocks are correct.
- **Files:**
  - `packages/edda-stack/src/contracts/ports/` (`edda.port.ts`,
    `ember.port.ts`, `kindling.port.ts`, `index.ts`)
  - `packages/edda-stack/src/testing/mocks/`
- **Dependencies:** —
- **Validation:** `pnpm vitest run packages/edda-stack` passes; the ports
  layer shows ≥80% line coverage.
- **Confidence:** high
- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2819 — port contract bound to a real
  conforming mock (compile-time + runtime), `testing/mocks` layer 99.6%.

#### TCOV-016: edda-stack store interfaces and migration tests

- **Intent:** `store-interfaces.ts` and the migration module have partial
  coverage. Ensure store operations and migration paths are fully tested.
- **Expected Outcome:** Store interface operations tested via mock
  implementations; migration tested with fixture data from prior versions.
- **Files:**
  - `packages/edda-stack/src/edda/store-interfaces.ts`
  - `packages/edda-stack/src/edda/migration/` (`index.ts`)
- **Dependencies:** —
- **Validation:** `pnpm vitest run packages/edda-stack` shows ≥80% line
  coverage for the store-interfaces and migration modules.
- **Confidence:** medium
- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2820 — concrete in-memory store/version
  tracker drives the type-only contracts; migration branch coverage 100%.

#### TCOV-017: kindling-integration emitter tests

- **Intent:** All 7 emitters (action, constraint, error, gate, human-input,
  plan, session) have zero test coverage. They are the primary output surface.
- **Expected Outcome:** Each emitter has a test file verifying event shape,
  required fields, and error handling.
- **Files:**
  - `packages/kindling-integration/src/emitters/*-emitter.ts` (action,
    constraint, error, gate, human-input, plan, session — all present today)
- **Dependencies:** —
- **Validation:** `pnpm vitest run packages/kindling-integration` shows ≥80%
  line coverage; each emitter has a dedicated test file.
- **Confidence:** high
- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2817 — dedicated test file per emitter;
  `emitters/` layer 0% → 100%.

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
- **Validation:** `pnpm vitest run packages/kindling-integration` shows ≥80%
  line coverage for the package.
- **Confidence:** medium
- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2818 — service/adapter/config/query/
  retention/status all ≥94% (most 100%); secure `mkdtempSync` config fixtures.

#### TCOV-019: mcp-server per-resource isolation tests

- **Intent:** 8 resources share one test file (`resources.test.ts`). Isolate
  each resource into its own test file for clarity and coverage precision.
- **Expected Outcome:** Each resource (`baseline`, `boundaries`, `config`,
  `constraints`, `drift`, `file-warnings`, `patterns`, `suppressions`) has a
  dedicated test file.
- **Files:**
  - `archive/anvil-mcp-server/src/resources/*.resource.ts`
- **Dependencies:** —
- **Validation:** Each resource test file passes independently; combined
  coverage ≥80%.
- **Confidence:** high
- **Status:** Complete 2026-06-20 — superseded by the Rust MCP server (RMCPF,
  #2809). The archived TS `*.resource.ts` target stays retired under ADR-033;
  the equivalent coverage now lives in `crates/anvil-cli/src/mcp/resources/`
  (`anvil.rs` + `tests.rs`, 14 tests) covering each `anvil://` resource
  (baseline, boundaries, patterns, suppressions, config, constraints, drift) as
  part of the ~304-test MCP suite (`cargo test -p eddacraft-anvil --bins mcp`).
  `anvil://file/{path}/warnings` was deliberately retired in the port. Owner
  decision 2026-06-20: reconcile as covered-by-RMCPF rather than re-test the
  dead TS surface.

#### TCOV-020: mcp-server config generator tests

- **Intent:** Existing tests exercise `generateMcpConfig(...)` for supported
  targets, but the per-editor config generators (`claude-code.ts`, `cursor.ts`,
  `vscode.ts`, `windsurf.ts`) lack direct tests and edge-case coverage.
- **Expected Outcome:** Each generator is directly tested for correct output
  structure, path handling, and edge cases (missing editor, custom paths).
- **Files:**
  - `archive/anvil-mcp-server/src/config/*.ts`
- **Dependencies:** —
- **Validation:** Config module reaches ≥80% line coverage.
- **Confidence:** high
- **Status:** Complete 2026-06-20 — superseded by the Rust MCP server (RMCPF,
  #2809). The TS per-editor generators stay retired under ADR-033; the live
  Rust analog is the MCP-config install path (`crates/anvil-cli/src/util.rs`,
  writing `~/.cursor/mcp.json` / `~/.claude.json`), covered by 21 tests
  including the symlink-safety threat model (LAUNCH-009.5). Owner decision
  2026-06-20: reconcile as covered-by-RMCPF.

#### TCOV-021: mcp-server transport and entry point tests

- **Intent:** `streamable-http.ts` has partial coverage; `bin.ts` and
  `bin-http.ts` are untested entry points.
- **Expected Outcome:** Transport tested for connection lifecycle, error
  handling, and concurrent requests. Entry points tested via process spawn
  with `--help` / `--version`.
- **Files:**
  - `archive/anvil-mcp-server/src/transports/streamable-http.ts`
  - `archive/anvil-mcp-server/src/bin.ts`
  - `archive/anvil-mcp-server/src/bin-http.ts`
- **Dependencies:** —
- **Validation:** ≥80% line coverage for the mcp-server package.
- **Confidence:** medium — entry points may need process-level testing
- **Status:** Complete 2026-06-20 — superseded by the Rust MCP server (RMCPF,
  #2809). The archived TS transport/entry points stay retired under ADR-033;
  the Rust MCP server runs in-process under `anvil-cli` (no separate
  `streamable-http`/`bin` surface), and the tools/enforcement/validation paths
  carry ~304 MCP tests. Owner decision 2026-06-20: reconcile as
  covered-by-RMCPF.

### Phase 4 — Rust TUI Coverage

> **Phase 4 re-scoped 2026-06-20 (crate correction).** The earlier "scope
> drift" call looked at the wrong crate. The `anvil-tui` split moved the **app
> surfaces** to `crates/anvil-tui/` but kept the **primitives** in
> `crates/eddacraft-tui/` (cargo name `eddacraft-tui`; version per its
> `Cargo.toml`) — which still holds the exact targets TCOV-022..-024 named:
>
> - `crates/eddacraft-tui/src/widgets/` has 23 widgets including the original
>   `text_input.rs` / `select.rs` / `confirm.rs` / `log_panel.rs` set — so
>   TCOV-022's interactive-widget target is concrete after all.
> - `crates/eddacraft-tui/src/theme/` **exists** (`mod.rs`, `traits.rs`,
>   `eddacraft.rs`) — TCOV-023's theme premise holds.
> - `crates/eddacraft-tui/src/keyboard/handler.rs` **exists**
>   (`Keymap::map(KeyEvent) -> Action`) — TCOV-024's target file is real.
>
> The three items are therefore **repointed to `crates/eddacraft-tui/`** and
> scoped against measured current coverage (the gap is interaction-flow,
> shell-layout-by-size, theme-trait, and keymap-edge-case tests), then promoted
> to **Ready**. `eddacraft-tui` is OSS-cleared (ADR-018) and already a module
> dependency, so this stays in original scope. **TCOV-025 (surface trait
> compliance) covered the `anvil-tui` app-surface side and is Merged via #2816.**

#### TCOV-022: eddacraft-tui interactive-widget state/key tests

- **Intent:** `crates/eddacraft-tui/src/widgets/` (23 widgets) is mostly
  render-tested, but **stateful interaction flows** (key sequence → state
  transition, selection/focus/scroll) are thin on the genuinely interactive
  widgets. Cover those flows so a regression in key-driven state is caught.
- **Expected Outcome:** Interaction tests (key/event → asserted state change)
  for the interactive widgets that lack them — `select.rs` (move/wrap/select),
  `confirm.rs` (yes/no toggle + default), `data_table.rs` (row nav/paging),
  `tree.rs` (expand/collapse/navigate), `log_panel.rs` (scroll/follow).
  `editor.rs` (51 tests) and `text_input.rs` (9) are already adequate — only
  add untested edit edge cases there, do not pad.
- **Files:**
  - `crates/eddacraft-tui/src/widgets/{select,confirm,data_table,tree,log_panel}.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-tui -- widgets` passes; llvm-cov ≥80%
  line for each targeted widget; each interactive widget has ≥1 state-transition
  test (not render-only).
- **Confidence:** high — files exist; pattern established by `editor.rs`/`tree.rs`.
- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2831 — added state-transition tests for the
  thin widgets (`confirm` default/confirm/reset; `log_panel` filter/match-nav/
  jump/scroll-bounds). `select`/`data_table`/`tree` already carried their
  interaction tests, so they were not padded. `cargo test -p eddacraft-tui` 287
  passed.

#### TCOV-023: eddacraft-tui shell layout + theme tests

- **Intent:** `crates/eddacraft-tui/src/shell.rs::render_shell` does responsive
  header/content/footer layout with width-driven help-text truncation and
  watermark fitting (8 tests). The theme layer (`theme/mod.rs`, `theme/traits.rs`)
  has **0 tests** (only `theme/eddacraft.rs`, 4). Cover shell layout across
  terminal sizes and the `Theme` trait's colour/style accessors.
- **Expected Outcome:** Shell tested at varying `Rect` widths/heights — help-text
  truncation boundary (fits vs elided), watermark gap math, and degenerate small
  sizes (1×1, zero-width) without panic. `EddaCraftTheme` asserted against the
  `Theme` trait contract (every accessor returns a usable style; default theme
  internally consistent).
- **Files:**
  - `crates/eddacraft-tui/src/shell.rs`
  - `crates/eddacraft-tui/src/theme/{mod,traits,eddacraft}.rs`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-tui -- shell theme` passes; llvm-cov
  ≥80% line for `shell.rs` and the `theme/` module; small/zero-size renders
  proven panic-free.
- **Confidence:** medium — shell truncation math is the main subject; theme
  traits may be largely accessor-only (assert contract, don't pad).
- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2829 — shell footer truncation/watermark-
  priority/degenerate-size tests + theme `fg`-contract tests (incl. a second
  `MinimalTheme` implementor exercising the trait's default methods).
  `cargo test -p eddacraft-tui -- shell theme` 22 passed.

#### TCOV-024: eddacraft-tui keyboard mapper edge cases

- **Intent:** `crates/eddacraft-tui/src/keyboard/handler.rs::Keymap::map(KeyEvent)
  -> Action` is the central key→action mapper (6 tests). Cover its mapping edge
  cases. **Re-scope note:** the original "rapid sequential input" premise is
  dropped — `map` is a *pure, stateless* function, so rapid input is just
  repeated calls with no state to corrupt; the real edge cases are modifiers and
  unmapped keys.
- **Expected Outcome:** Tests for modifier handling (`Ctrl+C` → `Quit`; other
  `Ctrl`/`Alt`/`Shift` combos → their intended action or `None`), every mapped
  `KeyCode` → expected `Action`, and unmapped keys → `Action::None`. Locks the
  keymap contract that `Surface::handle_key` consumers depend on.
- **Files:**
  - `crates/eddacraft-tui/src/keyboard/handler.rs`
  - `crates/eddacraft-tui/src/keyboard/mod.rs` (if it carries logic)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-tui -- keyboard` passes; llvm-cov ≥80%
  line for `handler.rs`; every `KeyCode` arm and the modifier branch covered.
- **Confidence:** high — small, pure, fully testable surface.
- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2828 — 6 edge-case tests added to
  `keyboard/handler.rs` (editing keys, `Char`→`Character` fallthrough incl.
  case-sensitivity, unmapped→`None`, Ctrl-only-maps-`Ctrl+C`, Alt/Shift
  pass-through). Every `map()` arm now exercised; `cargo test -p eddacraft-tui
  -- keyboard` 12 passed. Dropped the stale "rapid input" premise (map is pure).

#### TCOV-025: anvil-tui surface trait compliance tests — Merged

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-20 via #2816 — single parameterised runtime
  compliance test over all 22 registered surfaces (non-empty name/help,
  render/handle_key/reset without error) plus a registry-count guard. Was the
  one Phase 4 item whose premise survived the rename.
  The `Surface` trait is defined in `crates/eddacraft-tui/src/surface.rs:13`
  (`pub trait Surface<T: Theme = EddaCraftTheme>`) and re-exported via the
  3-line shim `crates/anvil-tui/src/surface.rs`; `crates/anvil-tui/src/surfaces/`
  holds the live `impl Surface` set (audit, browser, dashboard, doctor, gate,
  init, notifications, onboarding, plan_dashboard, status, tutorial, update_hint,
  watch, welcome, wizard).
- **Intent:** Verify every registered `anvil-tui` surface satisfies the
  `Surface` trait contract via a single parameterised compliance test, so a
  new surface cannot ship a partial implementation.
- **Expected Outcome:** A compliance test runs the trait contract (the methods
  the trait actually requires — confirm against
  `crates/eddacraft-tui/src/surface.rs`) against every surface in
  `crates/anvil-tui/src/surfaces/`, and fails if a surface is added without
  satisfying it.
- **Files:**
  - `crates/anvil-tui/src/surface.rs` (re-export shim)
  - `crates/anvil-tui/src/surfaces/`
  - `crates/eddacraft-tui/src/surface.rs` (trait definition — read-only
    reference for the contract under test)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-tui surface` passes; the
  compliance test enumerates the registered surfaces.
- **Confidence:** medium — the trait lives in `eddacraft-tui`, a separately
  released crate (currently `eddacraft-tui-v0.2.3`); the compliance test
  asserts against the re-exported trait, so an upstream trait change is a real
  (if low-frequency) source of churn.

### Phase 5 — Benchmark contract alignment

> Filed 2026-06-26 after the quiet-box `benchmarks/history/2026-06-26.json`
> capture exposed drift: `pnpm bench`, `.github/workflows/bench.yml`, and the
> history schema no longer describe the same surfaces (e.g. `secret_scan_parallel`
> ran locally but not in CI; `hot_read` / `call_lift` gated in
> `resource-budget.yml` but absent from `run.sh` and history). Closes the
> measurement side of V050F-008 for dev-box baselines; CI-class baselines remain
> a follow-up.

#### TCOV-026: Align routine bench suite, CI, and history schema

- **Intent:** One contract for pre-release benchmark capture: the surfaces in
  `scripts/bench/run.sh`, the Criterion job in `.github/workflows/bench.yml`,
  and `benchmarks/history/` schema + normaliser must match.
- **Expected Outcome:**
  - `bench.yml` runs `secret_scan_parallel` and uploads its log.
  - `run.sh` runs `walk_discovery`, `hot_read`, and `call_lift` (the same
    latency gates CI enforces via `resource-budget.yml`).
  - `benchmarks/README.md` schema documents resource budgets, gate benches, and
    `walk_discovery`.
  - `scripts/bench/to-history.py` normalises a `benchmark-results/manual-*`
    artifact dir into `benchmarks/history/<date>.json`.
  - The checks benchmark names the Rust scanner explicitly and measures matched
    plus clean TypeScript, Rust, and Python source inputs.
  - The parallel scanner benchmark uses a versioned corpus whose source share is
    split evenly across TypeScript, Rust, and Python.
  - History records fingerprint the performance-relevant regex catalogue and
    record default per-language rule counts; changed fingerprints establish a
    new baseline instead of reporting a scanner-only regression.
- **Files:**
  - `scripts/bench/run.sh`
  - `scripts/bench/to-history.py`
  - `.github/workflows/bench.yml`
  - `crates/anvil-checks/benches/checks.rs`
  - `crates/anvil-bench/benches/antipattern_scan.rs`
  - `crates/anvil-bench/README.md`
  - `benchmarks/README.md`
  - `benchmarks/history/<date>.json`
  - `docs/testing/benchmark-results.md`
- **Dependencies:** —
- **Validation:** `pnpm bench -- --skip-resource-budget` completes on a quiet
  box; `python3 scripts/bench/to-history.py --help` documents usage; workflow
  YAML includes `secret_scan_parallel`; `cargo check -p eddacraft-anvil-graph-cache
  --benches` is in the bench compile-check job.
  Follow-up: `python3 scripts/bench/to_history_test.py`;
  `cargo bench -p eddacraft-anvil-checks --bench checks -- --test
  antipattern_rust_scanner`; `cargo bench -p anvil-bench --bench
  antipattern_scan -- --test`.
- **Confidence:** high
- **Status:** Merged 2026-08-11 via #3730 — required GitHub checks passed,
  review threads were resolved, and rebase result `03d260dc2` is on `main`.
