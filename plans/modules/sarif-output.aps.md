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
| SARIFOUT | —     | Proposed | 0/6      |

**Last reviewed:** 2026-05-29

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

Two decisions warrant ADRs (see the design doc): (1) the `--format` value-enum
as canonical output selector, (2) shared SARIF emitter + per-command adapters
with no unified finding model. File **Proposed** alongside SARIFOUT-001/-002
after design sign-off; run `pnpm adr:check` for the next number.

## Work Items

> Status: Proposed. This module stays Proposed until the three design decisions
> (flag surface, module home, shared model) are signed off. Work items are
> drafted to Ready quality but are not execution-authorised until promotion.

### SARIFOUT-001: `--format` value-enum and `OutputMode::Sarif` resolver

- **Status:** Proposed
- **Intent:** A single canonical output selector supports SARIF without breaking
  the existing `--json` / `--no-tui` contract.
- **Expected Outcome:** A global `--format <auto|tui|plain|json|sarif>` flag
  resolves through one precedence-ordered resolver; `--json` behaves exactly as
  `--format json`; `--format sarif` is a clap-level error on commands that do not
  emit findings; SARIF is never auto-selected by TTY detection.
- **Validation:** `cargo test -p anvil-cli output::` — resolver precedence
  tests, `--json`/`--format json` parity test, and a reject test for `sarif` on a
  non-finding command. Existing `output/mod.rs` tests stay green.
- **Files:** `crates/anvil-cli/src/output/mod.rs`,
  `crates/anvil-cli/src/main.rs`.
- **Dependencies:** —
- **Confidence:** high

### SARIFOUT-002: Shared SARIF 2.1.0 emitter and schema-validation harness

- **Status:** Proposed
- **Intent:** A bounded, reusable SARIF document emitter exists with a
  deterministic schema-validation gate, independent of any command wiring.
- **Expected Outcome:** `crates/anvil-cli/src/output/sarif.rs` produces a SARIF
  2.1.0 document covering the pinned subset (`runs`/`tool.driver`/`rules`/
  `results`/`locations`/`suppressions`/`partialFingerprints`); the upstream 2.1.0
  JSON Schema is bundled in-repo; a test validates a hand-built document against
  the schema. No command is wired yet.
- **Validation:** `cargo test -p anvil-cli` — schema-validation test on a
  fixture document; the pinned-subset shape is golden-snapshotted.
- **Files:** `crates/anvil-cli/src/output/sarif.rs`, bundled schema asset under
  `crates/anvil-cli/` (path decided at impl), `crates/anvil-cli/src/output/mod.rs`.
- **Dependencies:** —
- **Confidence:** high

### SARIFOUT-003: `anvil check` SARIF adapter with `suppressions[]`

- **Status:** Proposed
- **Intent:** `anvil check --format sarif` emits schema-valid SARIF including
  baseline / `@anvil-ignore`-suppressed findings under `suppressions[]`.
- **Expected Outcome:** `check` warnings map to SARIF `results[]` with
  `ruleId` / `level` / `message` / `locations[]`; suppressed warnings render as
  `results[].suppressions[]` (§3.35) so reviewers see what was accepted at
  baseline time; the result set matches the JSON output's finding set. Note: the
  serialized `JsonWarning` shape drops suppression state, so the adapter reads
  `Warning.suppressed` from the **upstream `Warning`** path before the JSON
  projection (`antipattern_warning_to_json`) — or, if cleaner at impl time,
  carries suppression onto `JsonWarning` first; the PR records which.
- **Validation:** `cargo test -p anvil-cli` — golden + schema-validation fixture
  including at least one suppressed finding.
- **Files:** `crates/anvil-cli/src/commands/check.rs`,
  `crates/anvil-cli/src/output/sarif.rs`.
- **Dependencies:** SARIFOUT-001, SARIFOUT-002
- **Confidence:** high

### SARIFOUT-004: `anvil audit` SARIF adapter

- **Status:** Proposed
- **Intent:** `anvil audit --format sarif` emits schema-valid SARIF for audit
  issues.
- **Expected Outcome:** `AuditOutput.issues[]` map to SARIF `results[]` with
  `category` → `ruleId`, severity → `level`, and `file`/`line` →
  `locations[].physicalLocation.region`; the result set matches the JSON
  output's issue set.
- **Validation:** `cargo test -p anvil-cli` — golden + schema-validation fixture.
- **Files:** `crates/anvil-cli/src/commands/audit.rs`,
  `crates/anvil-cli/src/output/sarif.rs`.
- **Dependencies:** SARIFOUT-001, SARIFOUT-002
- **Confidence:** high

### SARIFOUT-005: `anvil gate` SARIF adapter (per-check results)

- **Status:** Proposed
- **Intent:** `anvil gate --format sarif` emits schema-valid SARIF for gate
  check results, handling the config-gap case coherently.
- **Expected Outcome:** `GateResult.checks[]` map to SARIF `results[]` (one per
  failed / needs-config check) with check `name` → `ruleId`; `requires_config`
  config-gap checks are represented without inflating the failure set
  (suppression or notification, decided at impl); SARIF emission does not alter
  gate exit codes.
- **Validation:** `cargo test -p anvil-cli` — golden + schema-validation fixture
  covering a failing check and a config-gap check.
- **Files:** `crates/anvil-cli/src/commands/gate.rs`,
  `crates/anvil-cli/src/output/sarif.rs`.
- **Dependencies:** SARIFOUT-001, SARIFOUT-002
- **Confidence:** medium — gate findings are per-check aggregates, so the
  SARIF result granularity needs an impl-time decision recorded in the PR.

### SARIFOUT-006: Docs, CHANGELOG, and manual Code Scanning upload smoke check

- **Status:** Proposed
- **Intent:** The SARIF surface is documented and validated end-to-end against a
  real Code Scanning consumer out-of-band.
- **Expected Outcome:** User docs describe `--format sarif` on the three
  commands and the pinned subset; CHANGELOG records the additive mode; a runbook
  records a manual upload of emitted SARIF to a GitHub Code Scanning sandbox repo
  confirming findings render (manual, out-of-band — **not** a CI test, because it
  is a non-deterministic external dependency).
- **Validation:** `pnpm docs:check && pnpm format:check`; manual upload check
  recorded in the runbook with date + sandbox repo reference.
- **Files:** `docs/` (path at impl), `CHANGELOG.md`, a runbook under
  `docs/runbooks/`.
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
