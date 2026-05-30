# Clawpatch pre-tag findings (v0.7.0-beta cut)

<!-- Executable only if tasks exist and status is Ready or In Progress. -->

| ID    | Owner  | Status      | Progress |
| ----- | ------ | ----------- | -------- |
| CLAWP | @aneki | In Progress | 14/64    |

**Last reviewed:** 2026-05-21 (thirteen findings now Merged: CLAWP-001 PR #1732 `6c106a4d`, CLAWP-008 PR #1765 `7c1fcce4`, CLAWP-011 PR #1791 `be927818`, CLAWP-012 PR #1772 `8eae1cfe`, CLAWP-013 PR #1788 `af78867f`, CLAWP-014 PR #1786 `ab00ee9a`, CLAWP-015 PR #1783 `0e63b52e`, CLAWP-021 PR #1764 `8d2d8da7`, CLAWP-022 PR #1770 `265f45d9`, CLAWP-028 PR #1763, CLAWP-029 PR #1789 `5fc13990`, CLAWP-030 commit `9253d9f3` in PR #1732, CLAWP-019 PR #2065 (post-review burn-down 2026-05-29). CLAWP-008 clears the only fix-before-tag obligation still outstanding from the release-council pass-2 verdict map. Per the release runbook §2 loop rule, the `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json` artefact is now invalidated by the post-merge bits — a fresh `claw-sweep` on the new `main` SHA is required before §2 council can run. Original filing: 2026-05-19 at v0.7.0-beta cut pre-flight; 64 open findings tracked here, 5 fixed findings recorded in the report file but not tracked as tasks since they need no action.)

## Purpose

Single tracking artefact for every clawpatch finding raised against the
`v0.7.0-beta` candidate. Each task captures the clawpatch finding ID so
the artefact is grep-searchable from the report JSON. Where an existing
`[Clawpatch]` GitHub issue already covers a finding by title, the task
cross-references the issue number — those issues remain authoritative
for triage and fix conversation; the APS task is the lifecycle handle.

## Convention

Tasks are sorted by (severity, triage) — `high > medium > low`, then
`confirmed-bug > contract-mismatch > docs-gap > risk > test-gap`. Each
task ID is `CLAWP-NNN`; the clawpatch finding ID lives in the body for
grep linkage. Status lifecycle follows `plans/aps-rules.md`.

## Severity breakdown at filing

- High: 1
- Medium: 25
- Low: 38

## Triage breakdown at filing

- confirmed-bug: 2
- contract-mismatch: 4
- docs-gap: 2
- risk: 10
- test-gap: 46

## Council verdict map (release council pass 1 + pass 2, 2026-05-20)

Canonical artefact: `plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`.
Every CLAWP-NNN now carries an explicit verdict satisfying the runbook §2 contract.

**Fix-before-tag (3):**
- CLAWP-001 — Merged via PR #1732 (closed -001, -029, -030 together at `6c106a4d`)
- CLAWP-008 — Merged via PR #1765 (merged at `7c1fcce4`; registry-load panic now caught inside `scan_artifact_json`; release-napi contract restored, fix-before-tag obligation cleared)
- CLAWP-028 — Merged via PR #1763 (off-by-one fixed in env-gated repro; production `welcome.rs` verified correct and unchanged)

**Ship (9, no action):**
- CLAWP-002, -003 — pass 1 (kernel-maintainer)
- CLAWP-016, -018, -020, -026, -036 — pass 2 verdicts; rationale per finding body
- CLAWP-029 — initial framing fix bundled in PR #1732 (commits `47d8b0b0`, `f444d52d`, `6c106a4d`); caller-side validation guidance refined in PR #1789. CLAWP-030 — closed by PR #1732 (commit `9253d9f3`).

**Defer with individual GH issue (24):**
- CLAWP-004 → #1736, CLAWP-005 → #1737 (pass 1)
- CLAWP-006 → #1646, CLAWP-007 → #1648, CLAWP-009 → #1743, CLAWP-010 → #1645
- CLAWP-011 → #1744, CLAWP-012 → #1745, CLAWP-013 → #1746, CLAWP-014 → #1747, CLAWP-015 → #1748
- CLAWP-017 → #1749, CLAWP-019 → #1750, CLAWP-021 → #1751, CLAWP-022 → #1752, CLAWP-023 → #1753
- CLAWP-024 → #1756, CLAWP-025 → #1754, CLAWP-027 → #1755
- CLAWP-031 → #1642, CLAWP-032 → #1644, CLAWP-033 → #1742, CLAWP-037 → #1643, CLAWP-038 → #1651

**Defer in batch tracker (#1740) — 28 low-severity test-hygiene items:**
- CLAWP-034, -035, -039, -040, -041, -042, -043, -044, -045, -046, -047, -048, -049, -050, -051, -052, -053, -054, -055, -056, -057, -058, -059, -060, -061, -062, -063, -064

## Findings

### CLAWP-001: --check --insecure-skip-verify tests are not exercising the intended update verification behaviour

- **Clawpatch finding:** `fnd_sig-feat-test-suite-73ba6156c4-e_b30c969c73`
- **Feature:** `feat_test-suite_73ba6156c4` — Rust integration test eddacraft-anvil/update_resolution_chain
- **Severity / Triage / Category:** high / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged
- **PR:** #1732 (rebase-merged 2026-05-20 at `6c106a4d`)
- **Resolution:** The two vacuous integration tests
  (`update_insecure_skip_verify_flag_is_accepted` and
  `update_skipping_verification_on_dev_build_logs_unconditional_warning`,
  both ran against `--check` which short-circuits before
  `verify_pending_install`) have been deleted from
  `crates/anvil-cli/tests/update_resolution_chain.rs`. The warning
  blocks inside `verify_pending_install` are extracted into
  `write_skip_verify_warning` / `write_dev_key_warning` helpers
  (with `.expect("stderr write must succeed for ADR-045 warning")`
  preserving `eprintln!`'s panic-on-failure contract) and
  unit-tested in `crates/anvil-cli/src/commands/update.rs` against a
  `Vec<u8>` sink. A clap-parser wrapper (`UpdateArgsParser` with
  `GlobalArgs` flattened) covers the hidden flag plus combinations
  with `--json`, `--force`, and `--version`. Counter bumped
  0/64 → 1/64 in this commit.
- **Recommendation:** Replace these with a deterministic parser-level test for the hidden flag and a direct test of the update verification preflight or a mocked library-fallback path that asserts stderr contains the loud `WARNING` text when verification is skipped. Avoid using the real `update --check` network/update probe as a parser surrogate.
- **Evidence:** Deleted: `crates/anvil-cli/tests/update_resolution_chain.rs` (the two named tests). Added: `crates/anvil-cli/src/commands/update.rs` `write_skip_verify_warning` / `write_dev_key_warning` helpers + 8 unit tests under `mod tests` (2 warning-emission + 6 parser).
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-002: Eval silently drops all but the first Rego query expression/result

- **Clawpatch finding:** `fnd_sig-feat-library-cf04c15e28-21c6_b62c4d5a74`
- **Feature:** `feat_library_cf04c15e28` — Rust library eddacraft-anvil-policy-engine
- **Severity / Triage / Category:** medium / contract-mismatch / api-contract
- **Confidence:** medium
- **Status:** Ship (release-council verdict 2026-05-20, kernel-maintainer; see `plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`)
- **Verdict rationale:** `Engine::eval` has zero external callers (workspace grep returns only in-module test invocations at `crates/anvil-policy-engine/src/lib.rs:136, :163, :169`). All current callers pass single-expression queries. Module doc at `:6` explicitly scopes this as a skeleton; multi-result shape is deferred to POLENG-002..006. The `.first().first()` collapse is consistent with the current contract.
- **Recommendation:** Either narrow the API contract by validating/rejecting multi-result, multi-expression, or binding queries, or change EvalResult to expose the full query result shape needed by downstream callers.
- **Evidence:** `crates/anvil-policy-engine/src/lib.rs:81` (`Engine::eval`), `crates/anvil-policy-engine/src/lib.rs:94` (`Engine::eval`), `crates/anvil-policy-engine/src/lib.rs:120` (`tests`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-003: QueryStatus serializes to an undocumented command name instead of the documented JSON-RPC method

- **Clawpatch finding:** `fnd_sig-feat-library-ea87528a72-bad4_0db6eef5bf`
- **Feature:** `feat_library_ea87528a72` — Rust library eddacraft-anvil-intercept-proto
- **Severity / Triage / Category:** medium / contract-mismatch / api-contract
- **Confidence:** high
- **Status:** Ship (release-council verdict 2026-05-20, kernel-maintainer; see `plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`)
- **Verdict rationale:** The `QueryStatus` variant is reachable on the NDJSON command-envelope path but the daemon returns a deliberate redirect error at `crates/anvil-intercept/src/ipc.rs:3018` ("query_status is a JSON-RPC-only method; use the query_status JSON-RPC frame"). All live traffic uses the documented JSON-RPC `query-status` method (`ipc.rs:2016-2034`, `cli/commands/intercept.rs:511`). The asymmetry is documented at `anvil-intercept-proto/src/lib.rs:117-118` — intentional, not silent.
- **Recommendation:** Do not expose QueryStatus through the serializable command-envelope enum unless the daemon accepts that wire shape. Either remove/split the variant from the serde wire enum, add a custom serde implementation that rejects command-envelope QueryStatus, or add an explicit supported route and tests for the `query-status` command form.
- **Evidence:** `crates/anvil-intercept-proto/src/lib.rs:62` (`IpcCommand`), `crates/anvil-intercept-proto/src/lib.rs:115` (`IpcCommand::QueryStatus`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-004: Named-pipe EOF is surfaced as an OS error despite the documented Ok(0) contract

- **Clawpatch finding:** `fnd_sig-feat-library-f75344b3ff-d56e_ccce569060`
- **GH issue:** [#1736](https://github.com/eddacraft/anvil-001/issues/1736)
- **Feature:** `feat_library_f75344b3ff` — Rust library eddacraft-anvil-intercept-win32
- **Severity / Triage / Category:** medium / contract-mismatch / api-contract
- **Confidence:** high
- **Status:** Deferred (release-council verdict 2026-05-20, kernel-maintainer; tracked in #1736)
- **Verdict rationale:** Bug confirmed at `crates/anvil-intercept-win32/src/lib.rs:235` (unconditional `Err(last_os_error())` on `ReadFile` returning 0). Windows-only; the only current caller reads a single frame and closes, never observing post-payload EOF. ~15 line fix, post-tag.
- **Recommendation:** Map the expected pipe EOF condition, at least ERROR_BROKEN_PIPE and any other documented named-pipe EOF status used by this handle mode, to Ok(0). Keep other ReadFile failures as io::Error::last_os_error().
- **Evidence:** `crates/anvil-intercept-win32/src/lib.rs:218` (`OwnerOnlyPipeClient::read`), `crates/anvil-intercept-win32/src/lib.rs:667` (`tests::connect_owner_only_pipe_client_round_trips_against_local_server`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-005: Shared contract API is trapped inside an integration test target

- **Clawpatch finding:** `fnd_sig-feat-test-suite-e6f2772c8e-a_8635488109`
- **GH issue:** [#1737](https://github.com/eddacraft/anvil-001/issues/1737)
- **Feature:** `feat_test-suite_e6f2772c8e` — Rust integration test eddacraft-anvil-intercept/midedit_contract
- **Severity / Triage / Category:** medium / contract-mismatch / api-contract
- **Confidence:** medium
- **Status:** Deferred (release-council verdict 2026-05-20, kernel-maintainer; tracked in #1737)
- **Verdict rationale:** Bug structurally correct but `anvil-rmcp` (RTAI-006) does not exist yet, so no consumer crate is currently broken. Fix must precede the first RMCP / TS driver integration test. ~400-line move, post-tag.
- **Recommendation:** Move the reusable request builders, assertions, fixture list, and helper services into a library-visible test-support module or small dev-support crate, then have `tests/midedit_contract.rs` import and exercise that API.
- **Evidence:** `crates/anvil-intercept/tests/midedit_contract.rs:25`, `crates/anvil-intercept/tests/midedit_contract.rs:88` (`FIXTURE_NAMES`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-006: Node type definitions exceed the supported runtime contract

- **Clawpatch finding:** `fnd_sig-feat-config-7528cb5b98-06a31_f279789bca`
- **GH issue:** [#1646](https://github.com/eddacraft/anvil-001/issues/1646)
- **Feature:** `feat_config_7528cb5b98` — Project config package.json
- **Severity / Triage / Category:** medium / risk / build-release
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Either align @types/node to the minimum supported Node major, or raise the engines.node minimum if Node 25 APIs are intentionally allowed.
- **Evidence:** `package.json:5` (`engines.node`), `package.json:93` (`devDependencies.@types/node`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-007: rule panic isolation is not valid in release builds with panic=abort

- **Clawpatch finding:** `fnd_sig-feat-library-367c2a4c02-33fe_7493568327`
- **GH issue:** [#1648](https://github.com/eddacraft/anvil-001/issues/1648)
- **Feature:** `feat_library_367c2a4c02` — Rust library eddacraft-anvil-intercept
- **Severity / Triage / Category:** medium / risk / build-release
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Either build the daemon/rule-execution path with panic="unwind" if catch_unwind is the intended boundary, or move rule execution behind a process boundary so aborting rules cannot terminate the daemon. Add the documented multi-process release fixture before treating the contract as covered.
- **Evidence:** `crates/anvil-intercept/tests/midedit_contract.rs:347` (`assert_rule_panic_response`), `crates/anvil-intercept/tests/midedit_contract.rs:648` (`panicking_rule_service`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-008: scan_artifact_json does not catch registry-load panics

- **Clawpatch finding:** `fnd_sig-feat-library-5f9e6b4709-fde2_0d203aecad`
- **GH issue:** [#1650](https://github.com/eddacraft/anvil-001/issues/1650)
- **Feature:** `feat_library_5f9e6b4709` — Rust library eddacraft-anvil-checks-napi
- **Severity / Triage / Category:** medium / risk / bug
- **Confidence:** medium
- **Status:** Merged via PR #1765 (merged at `7c1fcce4`; `scan_artifact_json`'s registry load now flows through the same `catch_unwind` → `panic_to_error` panic-boundary pattern already used by `get_default_patterns_json` / `get_pattern_json`; release-council pass-2 fix-before-tag obligation cleared 2026-05-20)
- **Recommendation:** Move the registry load into the same catch_unwind block as scan_artifact_rust, or wrap the whole scan_artifact_json implementation after JSON argument validation so registry panics are converted through panic_to_error.
- **Evidence:** `crates/anvil-checks-napi/src/lib.rs:170` (`scan_artifact_json`), `crates/anvil-checks-napi/src/lib.rs:183` (`scan_artifact_json`), `crates/anvil-checks-napi/src/lib.rs:254` (`get_default_patterns_json`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-009: Root lint command mutates Markdown files during validation

- **Clawpatch finding:** `fnd_sig-feat-release-4862937c51-dcb1_18d9dd2f14`
- **Feature:** `feat_release_4862937c51` — Package script lint
- **Severity / Triage / Category:** medium / risk / build-release
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Make scripts.lint non-mutating by removing --fix or delegating to pnpm run lint:check. Keep auto-fixing behind lint:md:fix or a dedicated lint:fix script.
- **Evidence:** `package.json:29` (`scripts.lint`), `package.json:30` (`scripts.lint:check`), `package.json:37` (`scripts.lint:md/lint:md:fix`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-010: Root Vitest discovery can miss package __tests__ directories outside src

- **Clawpatch finding:** `fnd_sig-feat-config-57f59ddd1e-1ee10_75c9a6fa4c`
- **GH issue:** [#1645](https://github.com/eddacraft/anvil-001/issues/1645)
- **Feature:** `feat_config_57f59ddd1e` — Project config vitest.config.ts
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Add an include pattern for package __tests__ directories that are not necessarily under src, or align the documented convention and config so there is a single discoverable test location. Consider disabling passWithNoTests at the root config if CI expects the root suite to find tests.
- **Evidence:** `vitest.config.ts:19`, `vitest.config.ts:6`
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-011: AI profile integration test does not verify the process exit status

- **Clawpatch finding:** `fnd_sig-feat-test-suite-07da3568bc-e_54227fecf1`
- **GH issue:** [#1744](https://github.com/eddacraft/anvil-001/issues/1744)
- **Feature:** `feat_test-suite_07da3568bc` — Rust integration test eddacraft-anvil/ai_guardrail_profile
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged via PR #1791 (merged 2026-05-21 at `be927818`)
- **Branch:** `fix/1744-clawp-011-ai-exit`
- **Resolution:** Added two assertions to
  `ai_profile_emits_diagnostic_envelope_in_json_mode`: (1)
  `!output.status.success()` because the curated AI guardrail run
  under strict-config MUST block on an empty workspace; (2) the
  envelope's `exit_code` value equals `output.status.code()` via a
  checked `i32::try_from` (clippy refuses the bare cast). Without
  these, a regression that silently turned the block into a pass — or
  drifted `exit_code` away from the process status — would have left
  every other envelope-shape assertion still green. Pure test
  addition; no production change. Counter bumped 11/64 → 12/64 in
  this commit.
- **Recommendation:** Assert the expected blocking exit behaviour explicitly, for example by checking `!output.status.success()` and comparing `parsed["exit_code"]` to `output.status.code()` when available.
- **Evidence:** `crates/anvil-cli/tests/ai_guardrail_profile.rs:28` (`ai_profile_emits_diagnostic_envelope_in_json_mode`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-012: Optional runner env file is omitted from the gitignore hygiene check when absent

- **Clawpatch finding:** `fnd_sig-feat-test-suite-3cc630c104-2_e0eaa945f3`
- **GH issue:** [#1745](https://github.com/eddacraft/anvil-001/issues/1745)
- **Feature:** `feat_test-suite_3cc630c104` — Rust integration test eddacraft-anvil-checks/surfenv_anvil_baseline
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged via PR #1772 (merged at `8eae1cfe`)
- **Branch:** `fix/1745-clawp-012-env-hygiene`
- **Resolution:** Split `ANVIL_ENV_FILES` into
  `ANVIL_COMMITTED_ENV_FILES` (templates that must exist) and
  `ANVIL_OPTIONAL_ENV_FILES` (gitignored, may be absent). The
  SURFENV-002 hygiene test now includes absent optional paths with
  empty content so the gitignore pattern is exercised regardless of
  checkout shape; the hygiene check itself is path-only, so empty
  content is safe (no in-file suppression directive can exist when
  the file doesn't). Added a dedicated
  `anvil_committed_env_templates_are_present` trip-wire so a regression
  that deletes a tracked template fails loudly instead of silently
  skipping the SURFENV-001 / -003 / -004 scans for it. Pure test
  changes. Advances to **Merged** on this PR's merge; counter bump in
  the same step.
- **Recommendation:** Keep absent optional env paths in the hygiene input with empty content, or add a separate path-only assertion that the configured gitignore patterns cover every gitignored path in `ANVIL_ENV_FILES`. Required committed templates should still fail if absent.
- **Evidence:** `crates/anvil-checks/tests/surfenv_anvil_baseline.rs:44` (`ANVIL_ENV_FILES`), `crates/anvil-checks/tests/surfenv_anvil_baseline.rs:111` (`surfenv_002_gitignore_hygiene_is_clean_on_anvil`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-013: Synthetic external smoke does not exercise positive SURFENV-001 detection

- **Clawpatch finding:** `fnd_sig-feat-test-suite-3cc630c104-6_d3e315b8d1`
- **GH issue:** [#1746](https://github.com/eddacraft/anvil-001/issues/1746)
- **Feature:** `feat_test-suite_3cc630c104` — Rust integration test eddacraft-anvil-checks/surfenv_anvil_baseline
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged via PR #1788 (merged at `af78867f`)
- **Branch:** `fix/1746-clawp-013-surfenv-smoke`
- **Resolution:** Added an `AWS_ACCESS_KEY_ID=AKIAQRSTUVWXYZ012345`
  line to the synthetic `.env.local` in
  `external_validation_smoke_against_synthetic_repo` and a positive
  SURFENV-001 assertion via `scan_env_file(... config_no_entropy())`.
  Value chosen to match the deterministic `AKIA[0-9A-Z]{16}` pattern
  in `crates/anvil-checks/src/secret/patterns.rs` while avoiding the
  case-insensitive default allowlist (`example` / `test` / `dummy` /
  `sample` / `placeholder` / `lorem ipsum`) — the well-known AWS docs
  example `AKIAIOSFODNN7EXAMPLE` is allowlisted via the `example`
  substring. Pure test addition. Advances to **Merged** on this PR's
  merge; counter bump in the same step.
- **Recommendation:** Add an adversarial synthetic env value that deterministically triggers SURFENV-001 with entropy disabled, then assert at least one unsuppressed SURFENV-001 finding from `scan_env_file`.
- **Evidence:** `crates/anvil-checks/tests/surfenv_anvil_baseline.rs:87` (`surfenv_001_secret_scan_is_clean_on_anvil`), `crates/anvil-checks/tests/surfenv_anvil_baseline.rs:185` (`external_validation_smoke_against_synthetic_repo`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-014: Runtime exclude test does not exercise a violation-producing change

- **Clawpatch finding:** `fnd_sig-feat-test-suite-436ea7fad0-0_75b1e494bf`
- **GH issue:** [#1747](https://github.com/eddacraft/anvil-001/issues/1747)
- **Feature:** `feat_test-suite_436ea7fad0` — Rust integration test eddacraft-anvil-kernel/watch_pattern_filter
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged via PR #1786 (merged at `ab00ee9a`)
- **Branch:** `fix/1747-clawp-014-runtime-exclude`
- **Resolution:** Replaced the prior no-import body
  (`export const added = 1;`) with a bare external import of
  `unseen-external-pkg` — the `NewDependencyIntroduction` invariant
  flags exactly this shape (`TrustLevel::External` target not in
  `previously_imported`), so the runtime add is now a
  violation-producing change. Added a paired control test
  `unfiltered_runtime_change_does_emit_violation` that runs the same
  write with empty `exclude_patterns` and asserts the violation DOES
  fire — without it, the excluded test could still pass for the wrong
  reason. Both tests share `RUNTIME_VENDOR_ADD_SOURCE` so the controls
  cannot drift. Pure test changes. Advances to **Merged** on this PR's
  merge; counter bump in the same step.
- **Recommendation:** Make the runtime write use a fixture that would emit a violation when unfiltered, or add a paired control in the same test/suite showing the same runtime change emits a violation without `exclude_patterns`. Then assert that only the excluded configuration suppresses it.
- **Evidence:** `crates/anvil-kernel/tests/watch_pattern_filter.rs:180` (`excluded_runtime_change_does_not_emit_violation`), `crates/anvil-kernel/tests/watch_pattern_filter.rs:187` (`excluded_runtime_change_does_not_emit_violation`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-015: Bare-exclude JSON stream test can pass without any stdout event

- **Clawpatch finding:** `fnd_sig-feat-test-suite-5361f7c11c-5_df9503bd96`
- **GH issue:** [#1748](https://github.com/eddacraft/anvil-001/issues/1748)
- **Feature:** `feat_test-suite_5361f7c11c` — Rust integration test eddacraft-anvil/watch_json_output
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged via PR #1783 (merged at `0e63b52e`)
- **Branch:** `fix/1748-clawp-015-bare-exclude-json`
- **Resolution:** Added a snapshot-envelope assertion after `parse_envelopes`
  in `watch_json_stdout_carries_only_ndjson_when_bare_exclude_warning_present`
  — mirrors the pattern in the sibling
  `watch_json_emits_initial_progress_and_snapshot` test. Without it, a
  `collect_until` timeout produced an empty `lines` and the for-loop
  + stderr-routing assertions all passed vacuously. The contract now
  proves an event actually arrived before the routing claims are
  evaluated. Pure test addition; no production change. Advances to
  **Merged** on this PR's merge; counter bump in the same step.
- **Recommendation:** After parsing, assert that the captured envelopes contain `WatchEventType::Snapshot`, or assert that `lines` includes the matching snapshot before checking stderr routing.
- **Evidence:** `crates/anvil-cli/tests/watch_json_output.rs:227` (`watch_json_stdout_carries_only_ndjson_when_bare_exclude_warning_present`), `crates/anvil-cli/tests/watch_json_output.rs:264` (`watch_json_stdout_carries_only_ndjson_when_bare_exclude_warning_present`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-016: Idempotence guard can pass even when verify fails or mutates HOME

- **Clawpatch finding:** `fnd_sig-feat-test-suite-5e2d518d88-f_7c1216fc4f`
- **Feature:** `feat_test-suite_5e2d518d88` — Rust integration test eddacraft-anvil/status_verify
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Assert both invocations succeed and still render the expected state, then snapshot the isolated home directory as well as the workdir before and after the verify runs.
- **Evidence:** `crates/anvil-cli/tests/status_verify.rs:183` (`status_verify_is_idempotent_and_does_not_mutate_workdir`), `crates/anvil-cli/tests/status_verify.rs:190` (`status_verify_is_idempotent_and_does_not_mutate_workdir`), `crates/anvil-cli/tests/status_verify.rs:213` (`status_verify_is_idempotent_and_does_not_mutate_workdir`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-017: Connection-liveness claim is not actually asserted

- **Clawpatch finding:** `fnd_sig-feat-test-suite-6e164ac8f1-c_fa5fda839c`
- **Feature:** `feat_test-suite_6e164ac8f1` — Rust integration test eddacraft-anvil-intercept/jsonrpc_conformance
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** After the no-response timeout, recover the client, write a normal request such as session.list with an id on the same stream, read one line, and assert the expected JSON-RPC result before shutting down.
- **Evidence:** `crates/anvil-intercept/tests/jsonrpc_conformance.rs:419` (`oversized_scan_buffer_notification_is_dropped_silently`), `crates/anvil-intercept/tests/jsonrpc_conformance.rs:446` (`oversized_scan_buffer_notification_is_dropped_silently`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-018: Claimed resolution-chain coverage is absent from the integration entrypoint

- **Clawpatch finding:** `fnd_sig-feat-test-suite-73ba6156c4-1_6b7c656b49`
- **Feature:** `feat_test-suite_73ba6156c4` — Rust integration test eddacraft-anvil/update_resolution_chain
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add deterministic tests for the strategy order itself. If full integration is impractical because `current_exe()` is hard to spoof, extract a small strategy resolver that accepts the executable path and sidecar lookup inputs, then test package-manager precedence, adjacent sidecar precedence over PATH sidecar, and library fallback when neither is...
- **Evidence:** `crates/anvil-cli/tests/update_resolution_chain.rs:1`, `crates/anvil-cli/tests/update_resolution_chain.rs:24`, `crates/anvil-cli/tests/update_resolution_chain.rs:122` (`signature_fixture_public_key_matches_dev_constant`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-019: Audit hard-codes current SURFENV rules, so new rules can bypass it

- **Clawpatch finding:** `fnd_sig-feat-test-suite-7d1e850c95-0_0c13ed7da6`
- **GH issue:** [#1750](https://github.com/eddacraft/anvil-001/issues/1750)
- **Feature:** `feat_test-suite_7d1e850c95` — Rust integration test eddacraft-anvil-checks/surfenv_suppression_audit
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged via PR #2065
- **Branch:** `fix/1750-clawp-019-surfenv-audit-registry`
- **Resolution:** Added a `SURFENV_RULES` registry constant to
  `crates/anvil-checks/src/surface/env/mod.rs` and rewrote
  `rule_ids_follow_surfenv_nnn_shape` to iterate it, so a newly
  registered rule is shape-checked automatically. Added an
  exhaustiveness trip-wire
  `every_registered_rule_has_a_suppression_case` that asserts every
  registered rule appears in the hand-written `AUDITED` set — adding a
  rule without a suppression case now fails the audit loudly. Proven
  non-vacuous by injecting a bogus `SURFENV-999`: the shape check still
  passed while the trip-wire failed, which is exactly the silent-bypass
  gap CLAWP-019 named. Pure test-hardening plus one public constant; no
  runtime behaviour change. Advances to **Merged** on this PR's merge;
  counter bumped 12/64 → 13/64 in the same step.
- **Recommendation:** Drive this audit from a single SURFENV rule registry, or add an explicit coverage table that fails when the registered SURFENV rule count exceeds the audited cases. Include suppression mode metadata so every line-level and header-level rule gets a positive suppression case and at least one wrong-rule negative case.
- **Evidence:** `crates/anvil-checks/tests/surfenv_suppression_audit.rs:6`, `crates/anvil-checks/tests/surfenv_suppression_audit.rs:32` (`rule_ids_follow_surfenv_nnn_shape`), `crates/anvil-checks/tests/surfenv_suppression_audit.rs:93` (`cross_rule_directives_do_not_leak`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-020: Claude Code missing-type test can pass for the wrong reason

- **Clawpatch finding:** `fnd_sig-feat-test-suite-b0175af8f6-7_95e15fd8fa`
- **Feature:** `feat_test-suite_b0175af8f6` — Rust integration test eddacraft-anvil/mcp_config
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Make the fixture command match the default expected command, or pass --command /tmp/fake/anvil during verification, so the only failing condition is the missing type field.
- **Evidence:** `crates/anvil-cli/tests/mcp_config.rs:636` (`mcp_install_verify_claude_code_requires_stdio_type`), `crates/anvil-cli/tests/mcp_config.rs:651` (`mcp_install_verify_claude_code_requires_stdio_type`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-021: Non-object config refusal tests do not verify the original file survives

- **Clawpatch finding:** `fnd_sig-feat-test-suite-b0175af8f6-8_9fe99e50f9`
- **GH issue:** [#1751](https://github.com/eddacraft/anvil-001/issues/1751) (auto-closed via `Closes #1751` trailer on PR #1764 merge)
- **Feature:** `feat_test-suite_b0175af8f6` — Rust integration test eddacraft-anvil/mcp_config
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged
- **Branch:** `fix/1751-clawp-021-config-survives` → PR #1764 (merged at `8d2d8da7`)
- **Resolution:** Captured the seeded config bytes before invoking
  `anvil mcp install --client cursor` against a non-object
  `mcpServers` container and against a non-object config root, then
  re-read after the refusal and asserted byte-identical. The refusal
  contract is now load-bearing rather than coincidental. Counter
  bumped 2/64 → 3/64 in this commit.
- **Recommendation:** Read the config file after the failed install and assert it remains byte-identical to the seeded non-object JSON.
- **Evidence:** `crates/anvil-cli/tests/mcp_config.rs:663` (`mcp_install_refuses_non_object_mcp_servers_container`), `crates/anvil-cli/tests/mcp_config.rs:685` (`mcp_install_refuses_non_object_config_root`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-022: start --json test accepts forbidden and wrong states

- **Clawpatch finding:** `fnd_sig-feat-test-suite-c4fa96f467-1_7bf4640dfa`
- **GH issue:** [#1752](https://github.com/eddacraft/anvil-001/issues/1752)
- **Feature:** `feat_test-suite_c4fa96f467` — Rust integration test eddacraft-anvil/start
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Merged via PR #1770 (merged at `265f45d9`)
- **Branch:** `fix/1752-clawp-022-start-json-state`
- **Resolution:** Replace the "any of six known literals" assertion in
  `start_json_emits_state_literal_in_status_verify_shape` with
  `assert_eq!(state, "needs_action")` — the concrete read-only fresh
  outcome, since `--json` implies `--verify` per
  `crates/anvil-cli/src/commands/start.rs:130`
  (`read_only = args.verify || global.json`). Add an explicit triple
  rejection of `protecting`, `watching`, and `ready_restart_required`
  for the same path, and pin the read-only contract structurally by
  asserting `.anvilrc`, `~/.cursor/mcp.json`, and `~/.claude.json` are
  not created. Advances to **Merged** on this PR's merge; counter
  bump in the same step.
- **Recommendation:** Strengthen `start_json_emits_state_literal_in_status_verify_shape` to assert the concrete expected read-only fresh result, at minimum `state == "needs_action"` or the intended fresh JSON state, and explicitly reject `protecting`. Also assert the complete status-verify JSON key set if the contract is that the shape matches `anvil status --verify`...
- **Evidence:** `crates/anvil-cli/tests/start.rs:17`, `crates/anvil-cli/tests/start.rs:38`, `crates/anvil-cli/tests/start.rs:230` (`start_json_emits_state_literal_in_status_verify_shape`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-023: Dual-run harness never executes the TypeScript engine

- **Clawpatch finding:** `fnd_sig-feat-test-suite-d2dd9b86fc-9_722c2c10a6`
- **Feature:** `feat_test-suite_d2dd9b86fc` — Rust integration test eddacraft-anvil-kernel/dual_run
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Wire run_dual to execute the real TS engine, or rename/scope the test as a scaffold until parity execution exists. Once integrated, remove the placeholder assertion and assert on known matching and intentionally divergent fixtures.
- **Evidence:** `crates/anvil-kernel/tests/dual_run.rs:30` (`run_dual`), `crates/anvil-kernel/tests/dual_run.rs:157` (`ts_placeholder_always_empty`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-024: Workspace-root containment is only integration-tested for read-only tools

- **Clawpatch finding:** `fnd_sig-feat-test-suite-dd97196f08-e_7d298c66c1`
- **Feature:** `feat_test-suite_dd97196f08` — Rust integration test eddacraft-anvil/mcp_serve_stdio
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add stdio integration tests for anvil_suppress and anvil_fix where the server is started in one tempdir, workspaceRoot points at a sibling tempdir, and the response is an error while the sibling file remains unchanged. Apply the same containment pattern to any other workspaceRoot-accepting or file-mutating MCP tool as it is added.
- **Evidence:** `crates/anvil-cli/tests/mcp_serve_stdio.rs:86` (`mcp_serve_stdio_tools_call_status_rejects_workspace_outside_server_root`), `crates/anvil-cli/tests/mcp_serve_stdio.rs:709` (`mcp_serve_stdio_tools_call_check_rejects_workspace_outside_server_root`), `crates/anvil-cli/tests/mcp_serve_stdio.rs:1041` (`mcp_serve_stdio_tools_call_suppress_inserts_comment_in_workspace_file`), _+1 more in the report_
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-025: Status air-gap coverage can pass despite attempted network work

- **Clawpatch finding:** `fnd_sig-feat-test-suite-e20f33eb79-7_827e764e5b`
- **Feature:** `feat_test-suite_e20f33eb79` — Rust integration test eddacraft-anvil/air_gapped
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Tighten the status assertions to prove the local probe path was used: parse the JSON output and assert the expected local diagnostic shape, assert absence of network/auth/update error markers, or extend the harness to detect socket/connect attempts and fail on any such attempt.
- **Evidence:** `crates/anvil-cli/tests/air_gapped.rs:5`, `crates/anvil-cli/tests/air_gapped.rs:131` (`anvil_status_verify_json_exits_cleanly_with_no_network`), `crates/anvil-cli/tests/air_gapped.rs:171` (`anvil_status_verify_json_skips_auth_refresh_with_expired_credentials`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-026: Baseline suppression tests miss same-name symbol collisions

- **Clawpatch finding:** `fnd_sig-feat-test-suite-ef1734fc53-9_b335b32a65`
- **Feature:** `feat_test-suite_ef1734fc53` — Rust integration test eddacraft-anvil-kernel/architecture_parity
- **Severity / Triage / Category:** medium / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add collision fixtures with two distinct symbol IDs and different files but the same symbol name: mark the original as previously public or privileged, add the second symbol in the delta, and assert the expansion violation is still emitted for the new file/symbol identity. If the implementation fails, change baseline keys to include file and/or...
- **Evidence:** `crates/anvil-kernel/tests/architecture_parity.rs:612` (`previously_public_symbol_suppressed`), `crates/anvil-kernel/tests/architecture_parity.rs:649` (`previously_privileged_symbol_suppressed`), `crates/anvil-kernel/tests/architecture_parity.rs:670` (`baseline_suppresses_known_but_flags_new`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-027: Fixture update mode is racy under the default Rust test harness

- **Clawpatch finding:** `fnd_sig-feat-test-suite-67ce748437-b_352b05bc8e`
- **Feature:** `feat_test-suite_67ce748437` — Rust integration test eddacraft-anvil/status_render
- **Severity / Triage / Category:** low / confirmed-bug / concurrency
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Make update mode single-owner and deterministic: either combine generation and verification into one test, skip round-trip verification while updating, use a serial test guard, or document and enforce `-- --test-threads=1` for fixture regeneration. Prefer atomic write-to-temp-then-rename if tests can ever read while updating.
- **Evidence:** `crates/anvil-cli/tests/status_render.rs:47` (`assert_or_update_fixture`), `crates/anvil-cli/tests/status_render.rs:146` (`fixture_directory_layout_is_pinned`), `crates/anvil-cli/tests/status_render.rs:187` (`every_fixture_round_trips_through_protection_claim`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-028: Capped run over-reports scanned file count

- **Clawpatch finding:** `fnd_sig-feat-test-suite-6f256e5de5-4_3a7ce63d82`
- **Feature:** `feat_test-suite_6f256e5de5` — Rust integration test eddacraft-anvil-checks/discovery_repro
- **Severity / Triage / Category:** low / confirmed-bug / bug
- **Confidence:** high
- **Status:** Merged
- **PR:** #1763 (rebase-merged 2026-05-20 at `51db7780`)
- **Resolution:** Moved the cap check ahead of the file read in `crates/anvil-checks/tests/discovery_repro.rs`, switched to `>=`, and extracted the predicate as `cap_reached(files_scanned, cap)` with a non-gated boundary unit test (`cap_reached_honours_boundary`) so the regression is now caught by `cargo test` without needing `ANVIL_DISCOVERY_REPRO=1`. The release-council's secondary inference that production `welcome.rs` shared the off-by-one was refuted on inspection: `crates/anvil-cli/src/commands/welcome.rs:611-626` pushes-then-checks `candidates.len() >= SCAN_MAX_FILES`, so `files_scanned = candidates.len() - panics - read_failures` cannot exceed 500 and needed no change. Counter bumped 1/64 → 2/64 in this commit.
- **Recommendation:** Move the cap check before incrementing files_scanned, or change the condition to break before counting/scanning once files_scanned has already reached 500.
- **Evidence:** `crates/anvil-checks/tests/discovery_repro.rs` (`cap_reached` predicate + boundary test, cap-check moved ahead of metadata/read).
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-029: Public crate docs still describe shipped validation as deferred

- **Clawpatch finding:** `fnd_sig-feat-library-105b94af10-dfb2_90c70e8495`
- **GH issue:** [#1647](https://github.com/eddacraft/anvil-001/issues/1647)
- **Feature:** `feat_library_105b94af10` — Rust library eddacraft-anvil-config
- **Severity / Triage / Category:** low / docs-gap / docs-gap
- **Confidence:** high
- **Status:** Merged via PR #1789 (merged at `5fc13990`)
- **Branch:** `fix/clawp-029-config-docs` → PR #1789
- **Resolution:** PR #1732's docs bundle (commits `47d8b0b0`
  "note hard-pinned validation is shipped", `f444d52d` "correct
  hard-pinned validation framing", `6c106a4d` copilot follow-up)
  reframed the scope/out-of-scope section so hard-pinned validation
  no longer reads as deferred. PR #1789 refines the remaining
  caller-side gap named in the recommendation: it makes the
  `parse_str`/`parse_file` body docs explicitly direct operator-config
  callers to invoke `validate_hard_pinned_classes` after parsing,
  matching the contract the integration tests already exercise.
  Advances to **Merged** on this PR's merge; counter bump in the same
  step.
- **Recommendation:** Update the `lib.rs` scope/out-of-scope section to reflect that hard-pinned validation is now part of the crate, and explicitly state that `parse_str`/`parse_file` parse only while callers that load operator config should call `validate_hard_pinned_classes` after parsing.
- **Evidence:** `crates/anvil-config/src/lib.rs:21`, `crates/anvil-config/src/lib.rs:46`, `crates/anvil-config/tests/hard_pinned_integration.rs:1`
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-030: Top-level rustdoc incorrectly says the 80-writer stress test is ignored

- **Clawpatch finding:** `fnd_sig-feat-library-549f398eb4-24d9_9940f5b408`
- **GH issue:** [#1649](https://github.com/eddacraft/anvil-001/issues/1649) (auto-closed by commit `9253d9f3`)
- **Feature:** `feat_library_549f398eb4` — Rust library eddacraft-anvil-witness
- **Severity / Triage / Category:** low / docs-gap / docs-gap
- **Confidence:** high
- **Status:** Merged
- **Resolution:** Commit `9253d9f3` "docs(anvil-witness): drop
  80-writer deferred follow-up" removed the stale deferred-entry text
  from `crates/anvil-witness/src/lib.rs` (lines 43-47 of the previous
  rustdoc block). The fix landed in PR #1732's docs bundle on
  2026-05-20 alongside the CLAWP-001 / CLAWP-029 framing fixes;
  MLP2-015 had promoted the 80-writer stress test out of `#[ignore]`
  in wave 1I and the lib.rs follow-up text was the last surface still
  reading as deferred. APS status was left at Draft at the time —
  caught in the PR #1789 review-feedback pass as a sibling housekeeping
  item to CLAWP-029. Counter bumped 10/64 → 11/64 in this commit.
- **Recommendation:** Update the `crates/anvil-witness/src/lib.rs` deferred follow-up text to match the current test state, or remove the deferred follow-up entirely if MLP2-015 completed it.
- **Evidence:** Pre-fix `crates/anvil-witness/src/lib.rs:46-50` (the
  five-line "80-writer stress test" deferred-follow-up bullet, body
  beginning "Concurrency safety is exercised at 16 writers in
  `tests/concurrency.rs`…"), removed by commit `9253d9f3` — see
  `git show 9253d9f3 -- crates/anvil-witness/src/lib.rs` for the
  exact deletion. `crates/anvil-witness/tests/concurrency.rs:16`
  remains the concurrency-suite header that MLP2-015's 10/10
  flake-budget review references.
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-031: Shell integration tests source the wrapper path without shell quoting

- **Clawpatch finding:** `fnd_sig-feat-cli-command-171a82ab94-_271f413775`
- **GH issue:** [#1642](https://github.com/eddacraft/anvil-001/issues/1642)
- **Feature:** `feat_cli-command_171a82ab94` — Rust command eddacraft-anvil-run
- **Severity / Triage / Category:** low / risk / build-release
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add a small shell-quoting helper for paths embedded in `bash -c` scripts, or avoid string interpolation by passing the script path as an argument/environment variable and sourcing `"$ANVIL_RUN_SCRIPT"`. Apply it consistently at all three call sites.
- **Evidence:** `crates/anvil-run/tests/shell_integration.rs:86` (`dispatcher_invokes_stub_with_tool_and_trailing_args`), `crates/anvil-run/tests/shell_integration.rs:115` (`anvil_run_disable_bypasses_the_launcher`), `crates/anvil-run/tests/shell_integration.rs:160` (`missing_binary_falls_through_to_the_underlying_command`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-032: Root tsconfig lists files that TypeScript does not actually check

- **Clawpatch finding:** `fnd_sig-feat-config-0c1c23856a-88473_3306548ebe`
- **GH issue:** [#1644](https://github.com/eddacraft/anvil-001/issues/1644)
- **Feature:** `feat_config_0c1c23856a` — Project config tsconfig.json
- **Severity / Triage / Category:** low / risk / build-release
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Either remove the inert entries and keep this as a narrow TypeScript-only config check, or add a dedicated config-check tsconfig that enables `allowJs`/`checkJs` for eslint.config.mjs and uses the current E2E path, likely `apps/e2e/**/*` if root-level E2E files are still intended.
- **Evidence:** `tsconfig.json:4` (`include`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-033: Watcher integration test uses a shorter readiness window despite known slow watcher registration

- **Clawpatch finding:** `fnd_sig-feat-test-suite-103d059679-6_bb22c34222`
- **Feature:** `feat_test-suite_103d059679` — Rust integration test eddacraft-anvil-kernel/watcher_integration
- **Severity / Triage / Category:** low / risk / build-release
- **Confidence:** medium
- **Status:** Merged 2026-05-30 via PR #2136 (closes #1742) — aligned the `filters_out_non_parseable_files` warm-up sleep to 250 ms and widened its `recv_timeout` to 10 s, matching the conservative budget already used by `detects_parseable_file_creation`.
- **Recommendation:** Use a shared watcher-test helper with the same conservative readiness strategy for both tests, or create a sentinel parseable file and wait until it is observed before starting the actual assertion scenario. At minimum, align the warm-up and recv_timeout values with the more conservative test.
- **Evidence:** `crates/anvil-kernel/tests/watcher_integration.rs:20` (`detects_parseable_file_creation`), `crates/anvil-kernel/tests/watcher_integration.rs:46` (`filters_out_non_parseable_files`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-034: Busy test is hard-coded to two in-flight scan requests despite using a configurable limit

- **Clawpatch finding:** `fnd_sig-feat-test-suite-6e164ac8f1-5_316d8e4688`
- **Feature:** `feat_test-suite_6e164ac8f1` — Rust integration test eddacraft-anvil-intercept/jsonrpc_conformance
- **Severity / Triage / Category:** low / risk / maintainability
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Spawn MAX_CONCURRENT_SCAN_BUFFERS client tasks in a loop, store their handles, wait until all are blocking, then assert the busy response and release/join all handles generically.
- **Evidence:** `crates/anvil-intercept/tests/jsonrpc_conformance.rs:790` (`scan_buffer_busy_returns_structured_server_error`), `crates/anvil-intercept/tests/jsonrpc_conformance.rs:821` (`scan_buffer_busy_returns_structured_server_error`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-035: Air-gapped harness calls have no per-test timeout

- **Clawpatch finding:** `fnd_sig-feat-test-suite-e20f33eb79-2_88c86cdb52`
- **Feature:** `feat_test-suite_e20f33eb79` — Rust integration test eddacraft-anvil/air_gapped
- **Severity / Triage / Category:** low / risk / build-release
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Run the harness through a bounded child process helper with a short timeout; on timeout, kill the child and fail the test with captured stdout/stderr and argv.
- **Evidence:** `crates/anvil-cli/tests/air_gapped.rs:73` (`run_air_gapped`), `crates/anvil-cli/tests/air_gapped.rs:101` (`run_air_gapped_without_dev`), `crates/anvil-cli/tests/air_gapped.rs:136` (`anvil_status_verify_json_exits_cleanly_with_no_network`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-036: Blocking-rule tests can leave parked workers when an assertion fails

- **Clawpatch finding:** `fnd_sig-feat-test-suite-e6f2772c8e-8_a18632c8db`
- **Feature:** `feat_test-suite_e6f2772c8e` — Rust integration test eddacraft-anvil-intercept/midedit_contract
- **Severity / Triage / Category:** low / risk / concurrency
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Wrap the blocking sections in a guard that releases the barrier on drop, or structure the tests so cleanup runs even when assertions panic. For async tests, a small RAII guard plus explicit disarm after normal release is enough.
- **Evidence:** `crates/anvil-intercept/tests/midedit_contract.rs:946` (`rust_consumer_surfaces_transport_timeout`), `crates/anvil-intercept/tests/midedit_contract.rs:983` (`rust_consumer_surfaces_server_busy`), `crates/anvil-intercept/tests/midedit_contract.rs:1101` (`rust_consumer_busy_response_satisfies_envelope_invariant`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-037: Standalone daemon command is not covered by process-level tests

- **Clawpatch finding:** `fnd_sig-feat-cli-command-43c5f1e5c2-_0ddd283bd8`
- **GH issue:** [#1643](https://github.com/eddacraft/anvil-001/issues/1643)
- **Feature:** `feat_cli-command_43c5f1e5c2` — Rust command eddacraft-anvil-intercept
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add a small process-level integration test for the binary contract, at minimum covering help/usage and invalid invocation. If feasible on Unix, also spawn `anvil-intercept start` under an isolated temporary HOME/state directory, send SIGTERM, and assert it exits successfully.
- **Evidence:** `crates/anvil-intercept/src/main.rs:21` (`Cli`), `crates/anvil-intercept/src/main.rs:38` (`main`), `crates/anvil-intercept/tests/jsonrpc_conformance.rs:74` (`with_dispatcher_and_scan_buffer`), _+1 more in the report_
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-038: Pool cap behaviour is not covered by the library test

- **Clawpatch finding:** `fnd_sig-feat-library-8a1266b4d7-3821_ea242dc15a`
- **GH issue:** [#1651](https://github.com/eddacraft/anvil-001/issues/1651)
- **Feature:** `feat_library_8a1266b4d7` — Rust library eddacraft-anvil-rayon-init
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add a regression test that validates the cap in an isolated process before any other rayon consumer can initialise the global pool. If direct global-pool testing is too fragile for unit tests, factor the cap calculation into a small pure helper and test that helper, then add one integration or subprocess smoke test for `init_global` applying it...
- **Evidence:** `crates/anvil-rayon-init/src/lib.rs:61` (`init_global`), `crates/anvil-rayon-init/src/lib.rs:93` (`tests::init_global_is_idempotent`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-039: SURFENV-003 fixture test does not assert two expected findings by key or indicator

- **Clawpatch finding:** `fnd_sig-feat-test-suite-07b2e91acd-f_c80db87108`
- **Feature:** `feat_test-suite_07b2e91acd` — Rust integration test eddacraft-anvil-checks/surfenv_prod_value
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Assert the exact expected finding set, for example by comparing keys and indicators for DATABASE_URL/prod host, FEATURE_FLAGS_ENV/production value, SECRET_PROD/key suffix, and LEGACY_HOST/suppressed.
- **Evidence:** `crates/anvil-checks/tests/surfenv_prod_value.rs:23` (`prod_in_local_fixture_yields_expected_findings`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-040: Only the first diagnostic is validated as a diagnostic envelope payload

- **Clawpatch finding:** `fnd_sig-feat-test-suite-07da3568bc-2_29981e65a9`
- **Feature:** `feat_test-suite_07da3568bc` — Rust integration test eddacraft-anvil/ai_guardrail_profile
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Iterate over every diagnostic and assert the required payload fields and category type for each one, then collect categories with a hard failure on missing or non-string values.
- **Evidence:** `crates/anvil-cli/tests/ai_guardrail_profile.rs:55` (`ai_profile_emits_diagnostic_envelope_in_json_mode`), `crates/anvil-cli/tests/ai_guardrail_profile.rs:75` (`ai_profile_emits_diagnostic_envelope_in_json_mode`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-041: Explicit-any test would pass duplicate or spurious AP-003 detections

- **Clawpatch finding:** `fnd_sig-feat-test-suite-10e78e1ec5-b_f1265e8ed2`
- **Feature:** `feat_test-suite_10e78e1ec5` — Rust integration test eddacraft-anvil-checks/antipattern_scanning
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Assert the exact number of AP-003 warnings for this fixture, and ideally assert distinct source lines or spans for the three expected occurrences.
- **Evidence:** `crates/anvil-checks/tests/antipattern_scanning.rs:80` (`detects_explicit_any_in_realistic_service`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-042: Empty-catch detection lacks a non-empty catch negative test

- **Clawpatch finding:** `fnd_sig-feat-test-suite-10e78e1ec5-d_65ca2eb29e`
- **Feature:** `feat_test-suite_10e78e1ec5` — Rust integration test eddacraft-anvil-checks/antipattern_scanning
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add a negative integration test with one or more non-empty catch blocks and assert that AP-006 is absent for those cases.
- **Evidence:** `crates/anvil-checks/tests/antipattern_scanning.rs:93` (`detects_empty_catch_block`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-043: Test still inherits explicit Git repository environment

- **Clawpatch finding:** `fnd_sig-feat-test-suite-31703286ca-8_babb865ee0`
- **Feature:** `feat_test-suite_31703286ca` — Rust integration test eddacraft-anvil/doctor_missing_git
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Remove explicit Git environment from both Command invocations, at least GIT_DIR and GIT_WORK_TREE, and consider using env_clear with a small allowlist if the CLI does not need the broader parent environment.
- **Evidence:** `crates/anvil-cli/tests/doctor_missing_git.rs:20` (`doctor_in_dir_without_git_repo_exits_zero_with_guidance`), `crates/anvil-cli/tests/doctor_missing_git.rs:64` (`doctor_json_in_dir_without_git_repo_reports_warn_with_remediation`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-044: Template drift test can pass vacuously when no drift findings are emitted

- **Clawpatch finding:** `fnd_sig-feat-test-suite-3cc630c104-f_d314f6c2d3`
- **Feature:** `feat_test-suite_3cc630c104` — Rust integration test eddacraft-anvil-checks/surfenv_anvil_baseline
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Assert that each checked template with keys produces at least one unsuppressed finding, or assert the exact expected missing keys for the current templates.
- **Evidence:** `crates/anvil-checks/tests/surfenv_anvil_baseline.rs:168` (`surfenv_004_drift_check_template_only_pairwise_run_is_missing_from_concrete_only`), `crates/anvil-checks/tests/surfenv_anvil_baseline.rs:178` (`surfenv_004_drift_check_template_only_pairwise_run_is_missing_from_concrete_only`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-045: Timeout regression is only tested on the successful handshake path

- **Clawpatch finding:** `fnd_sig-feat-test-suite-42e9aad964-e_15997a51e9`
- **Feature:** `feat_test-suite_42e9aad964` — Rust integration test eddacraft-anvil/spawn_probe
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add a negative timeout integration test using an installed MCP entry whose command hangs or never emits a JSON-RPC initialize response, then assert `status --verify` returns within the expected budget plus tight CI slack and does not promote the client.
- **Evidence:** `crates/anvil-cli/tests/spawn_probe.rs:114` (`handshake_against_real_anvil_promotes_restart_required_client`), `crates/anvil-cli/tests/spawn_probe.rs:142` (`handshake_against_real_anvil_promotes_restart_required_client`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-046: Test counts clean JSON lines but does not verify every writer's record survived

- **Clawpatch finding:** `fnd_sig-feat-test-suite-43eb68bbe4-2_8c325cb9d1`
- **Feature:** `feat_test-suite_43eb68bbe4` — Rust integration test eddacraft-anvil-witness/concurrency
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Collect parsed WitnessLine values and assert that the observed seq or commit_sha set exactly matches the expected values for all thread IDs.
- **Evidence:** `crates/anvil-witness/tests/concurrency.rs:26` (`line_for_thread`), `crates/anvil-witness/tests/concurrency.rs:73` (`run_concurrent`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-047: Concurrency test does not force simultaneous append contention

- **Clawpatch finding:** `fnd_sig-feat-test-suite-43eb68bbe4-9_38d05a9184`
- **Feature:** `feat_test-suite_43eb68bbe4` — Rust integration test eddacraft-anvil-witness/concurrency
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add an Arc<Barrier> shared by all worker threads and the main thread, have workers wait before calling append, then release them together after all handles are spawned.
- **Evidence:** `crates/anvil-witness/tests/concurrency.rs:56` (`run_concurrent`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-048: Invalid fenced JSON examples in public docs are silently ignored

- **Clawpatch finding:** `fnd_sig-feat-test-suite-5361f7c11c-a_34a54d762d`
- **Feature:** `feat_test-suite_5361f7c11c` — Rust integration test eddacraft-anvil/watch_json_output
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Return a `Result` from `extract_json_blocks` or panic with the block content/location when a fenced `json` block fails to parse.
- **Evidence:** `crates/anvil-cli/tests/watch_json_output.rs:415` (`extract_json_blocks`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-049: Windows build excludes every positive violation assertion

- **Clawpatch finding:** `fnd_sig-feat-test-suite-5cabaeb54d-6_0669bf06b0`
- **Feature:** `feat_test-suite_5cabaeb54d` — Rust integration test eddacraft-anvil-policy/opa_real_binary
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Add at least one Windows-compatible positive real-binary assertion, or gate only the path-sensitive cases and keep a platform-neutral positive case such as coverage_min below threshold active on Windows once the executor/path handling is fixed.
- **Evidence:** `crates/anvil-policy/tests/opa_real_binary.rs:85`, `crates/anvil-policy/tests/opa_real_binary.rs:91` (`change_scope_flags_oversized_plans`), `crates/anvil-policy/tests/opa_real_binary.rs:156` (`security_baseline_flags_sensitive_paths_without_review_tag`), _+1 more in the report_
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-050: coverage_min_passes_at_threshold does not test the threshold boundary

- **Clawpatch finding:** `fnd_sig-feat-test-suite-5cabaeb54d-7_e7acfb492c`
- **Feature:** `feat_test-suite_5cabaeb54d` — Rust integration test eddacraft-anvil-policy/opa_real_binary
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Change this test to use 80, or add a separate exact-threshold test while keeping the high-coverage case if desired.
- **Evidence:** `crates/anvil-policy/tests/opa_real_binary.rs:238` (`coverage_min_passes_at_threshold`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-051: Auth-failure test depends on host networking state

- **Clawpatch finding:** `fnd_sig-feat-test-suite-619533fb13-4_0fb8b14272`
- **Feature:** `feat_test-suite_619533fb13` — Rust integration test eddacraft-anvil/init_post_analysis
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Avoid a magic external port. Bind a local TcpListener to port 0 for a deterministic test double, or use an explicitly invalid/unroutable endpoint only if the CLI contract maps that exact failure mode to authentication_required across platforms.
- **Evidence:** `crates/anvil-cli/tests/init_post_analysis.rs:178` (`json_verbose_edict_auth_failure_emits_only_json_error`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-052: Any ANVIL_UPDATE_FIXTURES value disables fixture assertions

- **Clawpatch finding:** `fnd_sig-feat-test-suite-67ce748437-d_ce9ff94a77`
- **Feature:** `feat_test-suite_67ce748437` — Rust integration test eddacraft-anvil/status_render
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Require the environment variable to equal `1` exactly, and panic or ignore any other value so accidental configuration cannot turn golden assertions into fixture rewrites.
- **Evidence:** `crates/anvil-cli/tests/status_render.rs:42` (`assert_or_update_fixture`), `crates/anvil-cli/tests/status_render.rs:47` (`assert_or_update_fixture`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-053: Suppressed fixture path is not checked for finding kind

- **Clawpatch finding:** `fnd_sig-feat-test-suite-771f54e1ca-3_eecea46a11`
- **Feature:** `feat_test-suite_771f54e1ca` — Rust integration test eddacraft-anvil-checks/surfenv_gitignore_hygiene
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Add assertions for the suppressed finding's kind and suggested_pattern, mirroring the .env.local checks.
- **Evidence:** `crates/anvil-checks/tests/surfenv_gitignore_hygiene.rs:40` (`fixture_repo_yields_one_unignored_finding_and_one_suppressed`), `crates/anvil-checks/tests/surfenv_gitignore_hygiene.rs:44` (`fixture_repo_yields_one_unignored_finding_and_one_suppressed`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-054: Nested wrapper provenance can regress without failing this suite

- **Clawpatch finding:** `fnd_sig-feat-test-suite-79bc3daaff-2_aa3678a985`
- **Feature:** `feat_test-suite_79bc3daaff` — Rust integration test eddacraft-anvil-checks/command_safety_validation
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Assert the full expected nested wrapper chain for the parsed rm command, preferably in order, e.g. ["sudo", "bash"].
- **Evidence:** `crates/anvil-checks/tests/command_safety_validation.rs:168` (`unwraps_nested_sudo_bash`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-055: git clean -f expectation is internally inconsistent

- **Clawpatch finding:** `fnd_sig-feat-test-suite-79bc3daaff-b_e9b87cdd08`
- **Feature:** `feat_test-suite_79bc3daaff` — Rust integration test eddacraft-anvil-checks/command_safety_validation
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Pick one intended behaviour for git clean -f. If the intended default is Warn, make warns_on_git_clean_f assert CommandAction::Warn exactly; if Block is acceptable, loosen or parameterise the score test so it does not encode a contradictory contract.
- **Evidence:** `crates/anvil-checks/tests/command_safety_validation.rs:219` (`warns_on_git_clean_f`), `crates/anvil-checks/tests/command_safety_validation.rs:496` (`check_reports_correct_score_for_mixed_findings`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-056: Current-version contract is documented but not asserted

- **Clawpatch finding:** `fnd_sig-feat-test-suite-9c19d7b3f9-1_0c8ca0cd57`
- **Feature:** `feat_test-suite_9c19d7b3f9` — Rust integration test eddacraft-anvil/version_offline
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Compare `parsed["current_version"]` to the expected crate/package version available to the integration test, rather than only checking that it is non-empty.
- **Evidence:** `crates/anvil-cli/tests/version_offline.rs:67` (`version_offline_json_keys_are_stable`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-057: Auth-gate regression test still inherits auth credentials from the parent environment

- **Clawpatch finding:** `fnd_sig-feat-test-suite-9c19d7b3f9-d_2072549bd5`
- **Feature:** `feat_test-suite_9c19d7b3f9` — Rust integration test eddacraft-anvil/version_offline
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Explicitly remove the known Anvil auth-related environment variables for this command, or use `env_clear()` and then add only the minimal variables needed for the binary to execute in the test environment.
- **Evidence:** `crates/anvil-cli/tests/version_offline.rs:76` (`version_offline_does_not_require_auth`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-058: Human output test does not bind coverage tiers to the matching language

- **Clawpatch finding:** `fnd_sig-feat-test-suite-ac76327313-1_bbcaddc2d7`
- **Feature:** `feat_test-suite_ac76327313` — Rust integration test eddacraft-anvil/status_verify_languages
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Assert the association between each language and its expected tier, for example by matching full output rows/lines or by isolating the languages block and checking that the TypeScript row contains supported and the Python row contains unsupported.
- **Evidence:** `crates/anvil-cli/tests/status_verify_languages.rs:163` (`human_render_shows_per_language_breakdown`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-059: Unclassified-file test allows overcounting to pass

- **Clawpatch finding:** `fnd_sig-feat-test-suite-ac76327313-d_13f16e7ef0`
- **Feature:** `feat_test-suite_ac76327313` — Rust integration test eddacraft-anvil/status_verify_languages
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Make the expected count precise for this fixture, or add a second assertion that classified language files do not affect unclassified_files_seen. If .anvilrc is intentionally counted or ignored, encode that decision explicitly in the expected value.
- **Evidence:** `crates/anvil-cli/tests/status_verify_languages.rs:188` (`unclassified_files_surface_in_json_output`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-060: Vendored-directory regression test does not exercise the documented .git exclusion

- **Clawpatch finding:** `fnd_sig-feat-test-suite-ac76327313-d_a4301216f1`
- **Feature:** `feat_test-suite_ac76327313` — Rust integration test eddacraft-anvil/status_verify_languages
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Add a source-like file under .git, such as .git/hooks/pre-commit.py or .git/objects/example.rs, and assert that it does not produce a Python or Rust language entry or increase files_seen.
- **Evidence:** `crates/anvil-cli/tests/status_verify_languages.rs:206` (`vendored_dirs_are_excluded_from_language_count`), `crates/anvil-cli/tests/status_verify_languages.rs:230` (`vendored_dirs_are_excluded_from_language_count`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-061: Graph edge round-trip test permits a dangling target endpoint

- **Clawpatch finding:** `fnd_sig-feat-test-suite-c0c1a8f559-5_7f6cade3f4`
- **Feature:** `feat_test-suite_c0c1a8f559` — Rust integration test eddacraft-anvil-kernel-types/type_invariants
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Create both source and target nodes, round-trip them with the edge, and assert that both edge_back.from and edge_back.to are present in the deserialised node id set.
- **Evidence:** `crates/anvil-kernel-types/tests/type_invariants.rs:153` (`graph_types_round_trip_together`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-062: Event type and payload pairing invariant is not exercised

- **Clawpatch finding:** `fnd_sig-feat-test-suite-c0c1a8f559-9_50c60f8400`
- **Feature:** `feat_test-suite_c0c1a8f559` — Rust integration test eddacraft-anvil-kernel-types/type_invariants
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** medium
- **Status:** Draft
- **Recommendation:** Add a helper/assertion that maps each EventType to the expected EventPayload variant and use it in the event tests, including coverage for all four event kinds. If mismatched events should be rejected, add a negative deserialisation or validation test around that contract.
- **Evidence:** `crates/anvil-kernel-types/tests/type_invariants.rs:39` (`engine_event_binds_engine_id_to_payload`), `crates/anvil-kernel-types/tests/type_invariants.rs:117` (`full_event_with_nested_types_round_trips`), `crates/anvil-kernel-types/tests/type_invariants.rs:207` (`both_engines_produce_valid_events`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-063: Lockfile hash test disables the entropy path it is meant to protect

- **Clawpatch finding:** `fnd_sig-feat-test-suite-ded28997b4-4_6828102b71`
- **Feature:** `feat_test-suite_ded28997b4` — Rust integration test eddacraft-anvil-checks/secret_detection
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Run this case with entropy enabled, preferably with a threshold low enough to exercise the hash-like strings, and assert that no findings are returned for lockfile integrity and resolved hash lines.
- **Evidence:** `crates/anvil-checks/tests/secret_detection.rs:151` (`does_not_flag_hex_hashes_in_lockfile`), `crates/anvil-checks/tests/secret_detection.rs:143` (`does_not_flag_hex_hashes_in_lockfile`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

### CLAWP-064: Busy response assertion does not enforce JSON-RPC id echo

- **Clawpatch finding:** `fnd_sig-feat-test-suite-e6f2772c8e-5_8998d76c93`
- **Feature:** `feat_test-suite_e6f2772c8e` — Rust integration test eddacraft-anvil-intercept/midedit_contract
- **Severity / Triage / Category:** low / test-gap / test-gap
- **Confidence:** high
- **Status:** Draft
- **Recommendation:** Change the public busy assertion to take an expected id, or provide an `assert_busy_response_for(response, idx_or_id)` helper that validates `response["id"]` alongside the existing busy error shape.
- **Evidence:** `crates/anvil-intercept/tests/midedit_contract.rs:460` (`assert_busy_response`), `crates/anvil-intercept/tests/midedit_contract.rs:993` (`rust_consumer_surfaces_server_busy`)
- **Source:** Clawpatch pre-tag sweep 2026-05-19 (full finding body in `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`).

---

## Fixed findings (recorded, no action needed)

5 findings in the report have status `fixed` and are not tracked here. They remain in the canonical JSON report under `plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json` for historical reference.
