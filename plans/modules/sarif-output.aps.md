<!--
APS Module: SARIF Output
========================
Additive `--format sarif` output mode for the finding-emitting commands
(`anvil check` / `anvil gate` / `anvil audit`). Promoted from CIB-014 after
the 2026-05-29 design pass resolved the flag-surface, module-home, and
shared-model gates. See plans/specs/2026-05-29-sarif-output-design.md.
-->

# SARIF Output

| ID      | Owner | Status   | Progress |
| ------- | ----- | -------- | -------- |
| SARIFOUT | —     | Complete    | 6/6      |

**Last reviewed:** 2026-05-31

> **Provenance:** Promoted from
> [CIB-014](continuous-improvement-backlog.aps.md) on 2026-05-29. Source
> brainstorm: [2026-05-24 Drako borrow assessment](../brainstorms/2026-05-24-drako-borrow-assessment.md)
> §4 Borrow A (Drako cited as parallel evolution, not dependency). Design pass
> resolving the three open gates:
> [2026-05-29 SARIF output design](../specs/2026-05-29-sarif-output-design.md).

## Designs

- [SARIF Output Design](../specs/2026-05-29-sarif-output-design.md) — resolves
  flag surface, module home, and shared-finding-model questions; defines the
  bounded 2.1.0 subset and the PR wave plan.

## Purpose

Make Anvil's deterministic findings consumable by GitHub Code Scanning and the
standard SARIF tool ecosystem (Sonar, DefectDojo, security dashboards) without
bespoke adapters. The findings already exist; this is a pure additive output
mode that introduces no new engine, scoring scalar, dashboard, or telemetry
sink.

**Problem:** the only machine-readable output today is each command's bespoke
JSON shape. SARIF consumers cannot ingest Anvil findings without writing an
adapter per command, which is adoption friction at exactly the point (CI / merge
gate) where Anvil's new-edges-only enforcement is supposed to bite.

## In Scope

- A global `--format` value-enum (`auto|tui|plain|json|sarif`) that becomes the
  canonical output selector; `--json` folded in as a backward-compatible alias.
- `OutputMode::Sarif` and a single format resolver with explicit precedence.
- A thin shared SARIF 2.1.0 emitter (`crates/anvil-cli/src/output/sarif.rs`)
  owning `runs[]` / `tool.driver` / `rules[]` / `results[]` / `locations[]` /
  `suppressions[]`, plus the bundled schema and a schema-validation harness.
- Per-command adapters mapping each existing result shape into the shared SARIF
  types: `anvil check`, `anvil audit`, `anvil gate`.
- Baseline / `@anvil-ignore`-suppressed `check` findings rendered under SARIF
  `suppressions[]` (§3.35).
- Deterministic, stable `partialFingerprints` so Code Scanning dedupes runs.

## Out of Scope

- Full SARIF 2.1.0 conformance (only the GitHub Code Scanning ingest subset:
  results / rules / locations / suppressions / partialFingerprints).
- Refactoring `anvil-checks` / `anvil-policy-engine` / `anvil-rules` onto a
  unified in-process finding model — SARIF itself is the shared target; each
  command maps independently via a small adapter.
- Framework-mapped compliance evidence (lives in COMPLY; SARIF is upstream of
  it, not a substitute).
- Any behaviour change to existing JSON / human output or to gate/threshold exit
  codes — SARIF emission is exit-code-neutral.
- Runtime / proxy enforcement (out per `docs/vision/anvil-scope-guard.md` and the
  2026-05-22 Proxilion decline).

## Interfaces

**Depends on:**

- `crates/anvil-cli/src/output/mod.rs` — `OutputMode` + resolver extended here.
- `crates/anvil-cli/src/main.rs` — `GlobalArgs` gains `--format`.
- Existing per-command result shapes: `check.rs` (`build_json_output` /
  `JsonWarning`), `gate.rs` (`GateResult` / `AiGateResultEnvelope`), `audit.rs`
  (`AuditOutput` / `IssueOutput`).
- Upstream SARIF 2.1.0 JSON Schema (bundled into the repo; no schema fork).

**Coordinates with:**

- COMPLY (compliance-reporting) — SARIF is upstream of framework mapping.
- CIB-008 / CIB-009 (both Merged) — dispatcher consistency landed first, so
  SARIF reflects the corrected finding set, not the old bug.

**Exposes:**

- `--format sarif` on `anvil check` / `anvil gate` / `anvil audit`.
- A reusable SARIF emitter for any future machine-output format work.

## Estimated Scope

Six single-purpose work items across four waves; see the design doc's PR wave
table. No engine-crate refactor.

## Candidate ADRs

Both ADRs are filed and **Accepted 2026-05-29**: (1) the `--format` value-enum
as canonical output selector —
**[ADR-056](../decisions/056-format-flag-output-selector.md)** (amended to
per-command scope during SARIFOUT-001); (2) shared SARIF emitter + per-command
adapters with no unified finding model —
**[ADR-058](../decisions/058-sarif-shared-emitter-no-finding-model.md)** (filed
with SARIFOUT-002).

## Work Items

> Status: Complete (all six work items Merged and Released/Shipped in
> v0.7.3-beta, tag `8bfd48c4d` on 2026-05-31). The three
> design decisions (flag surface, module home, shared model) were ratified by the
> operator on 2026-05-29, promoting this module out of Proposed. SARIFOUT-001
> Merged via PR #2099; SARIFOUT-002 via PR #2105; SARIFOUT-003 via PR #2107;
> SARIFOUT-004 via PR #2112; SARIFOUT-005 via PR #2115; SARIFOUT-006 via PR
> #2116. Flag surface landed per-command (not global) — see ADR-056's Amendment.

### SARIFOUT-001: `--format` value-enum and `OutputMode::Sarif` resolver

- **Status:** Merged 2026-05-29 via PR #2099
- **Intent:** A single canonical output selector supports SARIF without breaking
  the existing `--json` / `--no-tui` contract.
- **Expected Outcome:** A `--format <auto|tui|plain|json|sarif>` flag on the
  three finding-emitting commands (`check`/`gate`/`audit`) resolves through one
  precedence-ordered resolver (`OutputMode::resolve_format` /
  `from_command_format`); `--json` behaves exactly as `--format json`; `--format`
  on a non-finding command is a clap `unexpected argument` error (the flag does
  not exist there); SARIF is never auto-selected by TTY detection. Until the
  adapters land, `--format sarif` reports a pending state. **Per-command, not
  global** — see ADR-056's Amendment (collision with `export`/`validate`).
- **Validation:** `cargo test -p eddacraft-anvil --bins output::` (the crate
  package name is `eddacraft-anvil`; the dir is `anvil-cli`) — resolver
  precedence, `--json`/`--format json` parity, and never-auto-select-SARIF unit
  tests; `tests/format_flag.rs` integration tests for the reject + alias +
  sarif-path wiring. Existing `output/mod.rs` tests stay green.
- **Files:** `crates/anvil-cli/src/output/mod.rs`,
  `crates/anvil-cli/src/commands/{check,gate,audit}.rs`,
  `crates/anvil-cli/tests/format_flag.rs`.
- **Dependencies:** —
- **Confidence:** high

### SARIFOUT-002: Shared SARIF 2.1.0 emitter and schema-validation harness

- **Status:** Merged 2026-05-29 via PR #2105
- **Intent:** A bounded, reusable SARIF document emitter exists with a
  deterministic schema-validation gate, independent of any command wiring.
- **Expected Outcome:** `crates/anvil-cli/src/output/sarif.rs` produces a SARIF
  2.1.0 document covering the pinned subset (`runs`/`tool.driver`/`rules`/
  `results`/`locations`/`suppressions`/`partialFingerprints`); the upstream 2.1.0
  JSON Schema is bundled in-repo (verbatim, no fork); a test validates a
  hand-built document against the schema; `partialFingerprints` are deterministic
  (stable SHA-256-derived digest). No command is wired yet (dead-code-allowed
  until the SARIFOUT-003 adapter consumes it). Pattern recorded in ADR-058.
- **Validation:** `cargo test -p eddacraft-anvil --bins sarif::` — schema
  validation against the bundled schema, golden pinned-subset snapshot, and a
  fingerprint determinism test.
- **Files:** `crates/anvil-cli/src/output/sarif.rs`,
  `crates/anvil-cli/src/output/sarif-schema-2.1.0.json` (vendored schema),
  `crates/anvil-cli/src/output/mod.rs`, `crates/anvil-cli/Cargo.toml`
  (`jsonschema` dev-dep), `.prettierignore`.
- **Dependencies:** —
- **Confidence:** high

### SARIFOUT-003: `anvil check` SARIF adapter with `suppressions[]`

- **Status:** Merged 2026-05-29 via PR #2107
- **Intent:** `anvil check --format sarif` emits schema-valid SARIF including
  `@anvil-ignore`-suppressed findings under `suppressions[]`.
- **Expected Outcome:** both `check` paths (source scan + non-source artifact)
  build SARIF directly from the upstream typed findings — antipattern `Warning`s
  and secret `SecretFinding`s — so suppression is read from `Warning.suppressed`
  **before** the `JsonWarning` projection drops it. Each finding → one
  `results[]` entry (`ruleId`/`level`/`message`/`locations[]` with line +
  column); suppressed warnings carry an in-source `suppressions[]` entry (§3.35)
  with the suppression reason as `justification`; `partialFingerprints` reuse
  `Warning.fingerprint` when present, else a stable digest. The result set
  matches the JSON finding set (suppressed warnings are in both). SARIF stays
  exit-code-neutral and is kept off stdout-printed human notices.
- **Validation:** `cargo test -p eddacraft-anvil` — `check.rs` unit test builds
  an accumulator with a normal + suppressed warning + a secret and validates the
  document against the bundled schema (asserting `suppressions[]`, deduped
  `rules[]`, secret ruleId/path); `tests/format_flag.rs` runs `check --format
  sarif` end to end and parses the SARIF envelope.
- **Files:** `crates/anvil-cli/src/commands/check.rs`,
  `crates/anvil-cli/src/output/sarif.rs` (consumer), `crates/anvil-cli/tests/format_flag.rs`.
- **Dependencies:** SARIFOUT-001, SARIFOUT-002
- **Confidence:** high

### SARIFOUT-004: `anvil audit` SARIF adapter

- **Status:** Merged 2026-05-29 via PR #2112
- **Intent:** `anvil audit --format sarif` emits schema-valid SARIF for audit
  issues.
- **Expected Outcome:** `build_audit_sarif` maps each `AuditData` issue to one
  SARIF `result` (`category` → `ruleId`, `IssueSeverity` →
  `level` [Critical/High→error, Medium→warning, Low/Info→note], `file`/`line` →
  `region`, deterministic `partialFingerprints`); rules deduped by category; no
  `suppressions[]` (audit has no suppression model). The result set matches the
  JSON output's `issues[]`. Empty cwd → empty-but-valid document.
- **Validation:** `cargo test -p eddacraft-anvil` — `audit.rs` unit test
  validates the document against the bundled schema (severity→level, dedup,
  no-suppressions); `tests/format_flag.rs` runs `audit --format sarif` end to
  end.
- **Files:** `crates/anvil-cli/src/commands/audit.rs`,
  `crates/anvil-cli/src/output/sarif.rs` (consumer), `crates/anvil-cli/tests/format_flag.rs`.
- **Dependencies:** SARIFOUT-001, SARIFOUT-002
- **Confidence:** high

### SARIFOUT-005: `anvil gate` SARIF adapter (per-check results)

- **Status:** Merged 2026-05-29 via PR #2115
- **Intent:** `anvil gate --format sarif` emits schema-valid SARIF for gate
  check results, handling the config-gap case coherently.
- **Expected Outcome:** `build_gate_sarif` maps `GateResult.checks[]` to SARIF
  `results[]`, one per **failed** (`error`) or **config-gap** (`requires_config`
  → `note`) check; passed, fully-configured checks are omitted (not findings).
  Check `name` → `ruleId`. **Impl decisions (the medium-confidence flag):**
  (1) config-gap checks are `note`-level results — they surface without
  inflating the failure set (chosen over a suppression, since a config-gap is a
  "couldn't run" state, not an accepted finding); (2) results are **repo-level
  with no `locations[]`** (gate findings are per-check aggregates, not
  per-location) — the emitter now omits empty `locations`. SARIF emission does
  not alter gate exit codes (still `Ok(overall)` → `EXIT_GATE_FAIL` on failure).
  Known limitation: GitHub Code Scanning may not surface location-less results;
  noted for the SARIFOUT-006 docs.
- **Validation:** `cargo test -p eddacraft-anvil` — `gate.rs` unit test builds a
  `GateResult` (failed + config-gap + passed checks) and validates against the
  bundled schema (failed→error, config-gap→note, passed omitted, no locations);
  `tests/format_flag.rs` runs `gate --format sarif` end to end.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/output/sarif.rs` (locations now skip-when-empty),
  `crates/anvil-cli/tests/format_flag.rs`.
- **Dependencies:** SARIFOUT-001, SARIFOUT-002
- **Confidence:** medium — gate findings are per-check aggregates; the
  granularity + config-gap decisions are recorded above.

### SARIFOUT-006: Docs, CHANGELOG, and manual Code Scanning upload smoke check

- **Status:** Merged 2026-05-29 via PR #2116
- **Intent:** The SARIF surface is documented and validated end-to-end against a
  real Code Scanning consumer out-of-band.
- **Expected Outcome:** the GitHub integration guide gains a **Code Scanning
  (SARIF)** section describing `--format sarif` on the three commands, the
  pinned 2.1.0 subset, the per-command result/location/suppression mapping, and
  the gate location-less-result limitation; the CHANGELOG `Unreleased` section
  records the additive mode; a runbook
  (`docs/runbooks/sarif-code-scanning-upload.md`) documents the manual upload to
  a Code Scanning sandbox with an empty **Verification record** for an operator
  to fill in (manual, out-of-band — **not** a CI test, since it is a
  non-deterministic external dependency, so it ships un-run by design).
- **Validation:** `pnpm docs:check && pnpm format:check` (+ `docs:index:check`
  for the new runbook in the generated indexes); the manual upload check is
  recorded in the runbook's Verification record when an operator performs it.
- **Files:** `docs/public/anvil/integrations/github.md`,
  `docs/runbooks/sarif-code-scanning-upload.md`, `CHANGELOG.md`,
  `docs/indexes/*` (regenerated).
- **Dependencies:** SARIFOUT-003, SARIFOUT-004, SARIFOUT-005
- **Confidence:** medium

## Waves

| Wave | Work items | Gate |
| ---- | ---------- | ---- |
| 1 | SARIFOUT-001, SARIFOUT-002 | resolver + schema harness land; no command wired |
| 2 | SARIFOUT-003, SARIFOUT-004 | check + audit emit schema-valid SARIF |
| 3 | SARIFOUT-005 | gate emits schema-valid SARIF |
| 4 | SARIFOUT-006 | docs + manual upload smoke check recorded |

## Risks

- **Gate granularity (SARIFOUT-005):** gate findings are per-check aggregates,
  not per-location warnings. The SARIF result granularity is an open impl
  decision; the medium-confidence flag and PR-recorded decision are the
  mitigation.
- **Schema drift:** the bundled SARIF 2.1.0 schema is pinned. Treat any upstream
  schema bump as a deliberate, separate change; the in-repo schema-validation
  test is the guard.
- **Scope creep toward full conformance:** the pinned subset is the boundary;
  `codeFlows`/`taxonomies`/`fixes`/multi-run are explicitly out. The Out of Scope
  section is the guard.
