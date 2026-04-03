<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Early Access Tests

| Scope  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| EATEST | —     | High     | Ready  |

## Purpose

Track all testing gaps identified during the v0.3.x release review councils.
Each item represents a missing test that would have caught (or would prevent
regression of) a real issue. Prioritised by risk — high-risk gaps are tests
that would have caught bugs found during council review.

## In Scope

- Wire-format pinning tests (serialisation contracts)
- Integration tests for subprocess-dependent code paths
- Edge case coverage for parsing, matching, and validation
- Regression tests for council findings that were fixed

## Out of Scope

- Performance benchmarks (covered by BENCH)
- End-to-end CLI tests (covered by TINT)
- TypeScript/Node.js test gaps (covered by TCOV)

## Interfaces

**Depends on:** RCLI (Tier 1), release slices 1–4 merged

**Exposes:** Higher confidence in shipped code, regression prevention

---

## Phase 1 — Kernel Types (slice 1)

### EATEST-001 — Wire-format pinning tests for EngineEvent

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** Pin the exact JSON wire representation for each EventPayload
  variant. Store expected JSON as inline string constants. Catches silent
  field renames or serde attribute changes that round-trip tests miss.
- **Files:** `crates/anvil-kernel-types/tests/type_invariants.rs`

### EATEST-002 — Deserialise from external JSON literals

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** Deserialise hand-written JSON (not serialiser-produced) for all
  four EventPayload variants and SymbolNode. Catches field naming regressions
  independently of the serialiser.
- **Files:** `crates/anvil-kernel-types/tests/type_invariants.rs`

### EATEST-003 — ErrorPayload with file: None round-trip

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Integration test constructing an Error event with `file: None`,
  serialising, deserialising, asserting `err.file.is_none()`.
- **Files:** `crates/anvil-kernel-types/tests/type_invariants.rs`

### EATEST-004 — Visibility invalid variant rejection

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Assert `serde_json::from_str::<Visibility>("\"Private\"").is_err()`
  — currently only SymbolKind and EdgeType have invalid-variant tests.
- **Files:** `crates/anvil-kernel-types/src/graph.rs`

---

## Phase 2 — Checks (slice 2)

### EATEST-005 — Integration test harness for checks crate

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** Create `crates/anvil-checks/tests/integration.rs` exercising
  `run_secret_check`, `run_antipattern_check`, and
  `run_command_safety_check` end-to-end with temp files. Verify score, passed
  flag, and findings match expected values for known-bad and known-good inputs.
- **Files:** `crates/anvil-checks/tests/integration.rs` (new)

### EATEST-006 — Git history scanning with real repository

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** Create a temp git repository, commit a file containing a known
  secret, assert `scan_git_history` returns a finding. Also test non-matching
  extensions are excluded.
- **Files:** `crates/anvil-checks/tests/secret_detection.rs`

### EATEST-007 — glob_to_regex unit tests

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Test glob_to_regex with: bare filename (`*.d.ts`), double-star
  prefix (`**/__mocks__/**`), nested double-star, special regex characters in
  directory names, Windows-style path separators.
- **Files:** `crates/anvil-checks/src/antipattern/scanner.rs`

### EATEST-008 — Credit card false-positive regression test

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Assert that known-safe 16-digit version strings (`1234-5678-
  9012-3456` in non-card context) do not match the credit card pattern.
  Documents the known limitation if the test is expected to fail.
- **Files:** `crates/anvil-checks/tests/secret_detection.rs`

### EATEST-009 — Sudo wrapper e2e check

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** `run_command_safety_check` with `sudo git reset --hard HEAD` —
  assert blocked with finding referencing `git-reset-hard`.
- **Files:** `crates/anvil-checks/tests/command_safety_validation.rs`

### EATEST-010 — Entropy duplicate-match assertion

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Test where a single line produces both quoted and assignment
  entropy matches. Assert deduplication behaviour is explicit.
- **Files:** `crates/anvil-checks/src/secret/entropy.rs`

### EATEST-011 — AntipatternCheckConfig severity_threshold=Info

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Assert `console.log` (AP-007, Info severity) causes `passed=false`
  when threshold is Info.
- **Files:** `crates/anvil-checks/tests/antipattern_scanning.rs`

---

## Phase 3 — Policy (slice 3)

### EATEST-012 — OPA evaluate happy path integration test

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** Write a minimal Rego policy to a temp dir, call
  `Evaluator::evaluate()` with matching input, assert violations returned.
  Skip with `#[cfg(not(feature = "integration"))]` when OPA unavailable. This
  would have caught the double-wait() bug.
- **Files:** `crates/anvil-policy/tests/` (new)

### EATEST-013 — OPA timeout enforcement test

- **Status:** Ready
- **Priority:** High
- **Confidence:** Medium
- **Intent:** Pass a policy that hangs (or mock a slow process), verify
  `OpaError::Timeout` is returned within the configured timeout, and that the
  child process is killed.
- **Files:** `crates/anvil-policy/tests/` (new)

### EATEST-014 — Exception filtering with active suppressions

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Mock OPA result, add matching exceptions, assert violation count
  is reduced and `suppressed_count` is correct.
- **Files:** `crates/anvil-policy/src/exceptions.rs`

### EATEST-015 — Empty OPA stdout handling

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Test `extract_violations()` with empty or minimal JSON
  (`{"result": []}`) — verify empty violations vec, no panic.
- **Files:** `crates/anvil-policy/src/opa.rs`

### EATEST-016 — Bundle symlink escape rejection

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Create a symlink within a bundle directory pointing outside it,
  reference in manifest, assert `load_bundle()` returns Validation error.
- **Files:** `crates/anvil-policy/src/bundle.rs`

### EATEST-017 — Missing package declaration warning test

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Write a `.rego` file with no package line, call
  `load_policies_with_warnings()`, assert warning is returned. Completed
  during council fix round.
- **Files:** `crates/anvil-policy/src/loader.rs`

### EATEST-018 — Profile case-insensitive parsing

- **Status:** Ready
- **Priority:** Low
- **Confidence:** High
- **Intent:** Verify `"STANDARD".parse::<Profile>()` succeeds (FromStr already
  lowercases, but no test covers mixed-case input).
- **Files:** `crates/anvil-policy/src/profiles.rs`

---

## Phase 4 — Architecture (slice 4)

### EATEST-019 — Legacy line=0 backward-compat in is_existing_violation

- **Status:** Ready
- **Priority:** High
- **Confidence:** High
- **Intent:** Create a baseline with violations whose IDs were generated with
  `line=0`, assert `is_existing_violation` returns true for a violation with
  same from/to but `line != 0`. Only regression protection for the backward-
  compat fallback.
- **Files:** `crates/anvil-architecture/src/baseline.rs`

### EATEST-020 — YAML size limit boundary test

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Write a file of exactly `MAX_YAML_SIZE` bytes — assert parses.
  Write `MAX_YAML_SIZE+1` bytes — assert `InvalidYaml` error. Verify no
  off-by-one.
- **Files:** `crates/anvil-architecture/src/yaml_parser.rs`

### EATEST-021 — Multi-layer file match determinism

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Define two layers whose globs match the same file. Assert which
  layer is assigned and that it is deterministic (alphabetical first). Pins
  the BTreeMap ordering behaviour.
- **Files:** `crates/anvil-architecture/src/validator.rs`

### EATEST-022 — merge_violations duplicate ID collision

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Call `merge_violations` with two violations sharing the same ID
  but different `from_file`. Assert result contains exactly one and it is from
  `new_violations` (new-wins). Pins the precedence semantics.
- **Files:** `crates/anvil-architecture/src/baseline.rs`

### EATEST-023 — Schema version forward-compat test

- **Status:** Ready
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Pass definition with schema_version `0.1.1` (future minor bump),
  assert validation succeeds. Pass `1.0.0`, assert it fails. Pins the semver
  tolerance behaviour.
- **Files:** `crates/anvil-architecture/src/definition.rs`

### EATEST-024 — ESM extension swap resolution

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Provide ImportEdge where `to_file` ends in `.js` but the actual
  file in assignments is `.ts`. Assert boundary check resolves the layer
  correctly.
- **Files:** `crates/anvil-architecture/src/validator.rs`

### EATEST-025 — Monorepo cross-app boundary test

- **Status:** Ready
- **Priority:** Low
- **Confidence:** Medium
- **Intent:** Using monorepo template, validate an edge from `apps/a/index.ts`
  to `apps/b/index.ts`. Assert whether a violation is produced (documents the
  lack of cross-app enforcement).
- **Files:** `crates/anvil-architecture/src/validator.rs`
