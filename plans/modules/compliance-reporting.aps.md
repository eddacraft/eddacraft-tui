<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Compliance Reporting

| ID  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| COMPLY | —     | medium   | Draft  |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: activation gate
restated, TS-era task paths rewritten to Rust crates per ADR-098 AD-2,
dependency block rewritten to shipped surfaces).

> **Reset posture (POLRESET-010 / ADR-098, 2026-07-04; restated 2026-07-11):**
> post-first-slice expansion — not a prerequisite for first policy value. Of
> the original prerequisites, pack validation (POLVAL, Done), the
> `anvil-baseline` starter pack (POLRESET-007, #3167), and report-only EVALCI
> (#3170) have all **shipped**. The one genuinely unspent prerequisite is
> **evidence semantics** — what counts as audit evidence, how it is bound to
> commits/refs, retention, and framing — which is defined **nowhere** today.
> Authoring that design (doc or ADR) is an explicit gate before this module
> can move to Ready; no compliance claims before it exists. Coordinated by
> [`POLRESET`](../archive/modules/policy-value-enforcement-reset.aps.md).

> **Policy-solution validation (2026-06-24):** COMPLY should consume
> regorus-backed policy outcomes (`anvil policy eval --json` v1, pack metadata,
> exceptions, and eval evidence), not Go OPA output directly.
>
> NOTE(post-rust, executed 2026-07-11): task scopes/validations previously
> referenced the retired TS tree (`packages/anvil/policy/src/`,
> `packages/anvil/runtime/src/`, `apps/anvil-cli/src/commands/`); they now
> target the Rust crates — registry/mapper/scoring/evidence/history in
> `crates/anvil-policy-engine/src/` (per ADR-098 AD-2 **not**
> `crates/anvil-policy`, which is deletion-slated once EXCEPT-012 completes),
> reporting/CLI in `crates/anvil-cli/src/commands/compliance.rs`. Dependency
> modules `opa-architecture-integration`, `drift-reporting`, and
> `suppressions` are archived (`policy-lifecycle` is **not** archived — it is
> a live Draft module and remains a real dependency); exception evidence is
> the EXCEPT store (ADR-100 committed authority — evidence must be read from
> committed/pushed trees, never worktree state; home moves to
> `anvil-exceptions` when EXCEPT-012 lands).

## Purpose

Produce audit-ready compliance evidence from policy evaluation results.
Organisations operating under regulatory frameworks (SOC 2, ISO 27001, GDPR,
PCI-DSS, HIPAA) need continuous proof that engineering controls are active and
effective. This module maps Anvil policy results to framework controls and
exports evidence in formats auditors accept.

## In Scope

- Compliance framework registry with control mappings
- Policy-to-control tagging so each policy links to one or more framework
  controls
- Evidence collection from the shipped primitives: `anvil policy eval --json`
  v1 output, gate runs, EXCEPT-store exceptions (ADR-100), EVALCI regression
  reports, GITGOV review capsules / witness chain, and drift snapshots
- Compliance posture score per framework
- Report generation in Markdown, JSON, and PDF-ready HTML
- Historical trend tracking for compliance posture over time
- CLI commands for compliance status and report generation
- Scheduled evidence snapshots for audit windows

## Out of Scope

- Certification or attestation issuance
- Direct integration with GRC platforms (export format covers this)
- Legal interpretation of framework requirements
- Real-time compliance alerting (use CI output)

## Interfaces

**Depends on:**

<!-- Rewritten 2026-07-11 to shipped surfaces; the previous block named three archived modules. -->
- `anvil policy eval --json` v1 (the frozen `anvil-policy-engine` facade
  output) — policy evaluation results; replaces the archived
  `opa-architecture-integration`
- `git-native-exceptions` — exception evidence for the audit trail via the
  EXCEPT store under ADR-100 committed authority (read from committed/pushed
  trees, never worktree state); replaces the archived `suppressions`; home
  moves to `anvil-exceptions` when EXCEPT-012 lands
- `eval-regression-ci-gate` — committed baseline + report-only regression
  reports as recurring evidence
- GITGOV review capsules / witness chain (ADR-037) — offline-verifiable
  evidence containers COMPLY should reuse, not reinvent
- drift snapshots — kernel-owned, successor of the archived `drift-reporting`
- `org-policy-hierarchy` — Org-level compliance baselines
- `policy-lifecycle` — Policy state for active coverage reporting (live Draft)
- **Evidence-semantics design gate** — undefined anywhere today; must be
  authored before promotion (see Reset posture)

**Exposes:**

- `ComplianceFrameworkRegistry` — Framework and control definitions
- `ComplianceMapper` — Policy-to-control mapping engine
- `EvidenceCollector` — Aggregates evidence from multiple sources
- `ComplianceReporter` — Report generation in multiple formats
- `anvil compliance status` — Show current posture per framework
- `anvil compliance report` — Generate audit-ready report
- `anvil compliance map` — Show policy-to-control mappings

## Acceptance Criteria

- [ ] At least SOC 2 and ISO 27001 control mappings ship as built-in frameworks
- [ ] Custom framework definitions are supported via YAML
- [ ] Each policy can be tagged to one or more framework controls
- [ ] Compliance posture score reflects percentage of controls with active policies
- [ ] Reports include evidence timestamps, policy versions, and evaluation results
- [ ] Historical posture is tracked per framework over time
- [ ] Reports generate in < 5 seconds for typical organisations
- [ ] Export formats include Markdown, JSON, and HTML

## Risks & Mitigations

| Risk                                      | Mitigation                                   |
| ----------------------------------------- | -------------------------------------------- |
| Mapping inaccuracy creates false assurance | Mark mappings as suggested; require review   |
| Framework updates invalidate mappings     | Version frameworks; detect stale mappings    |
| Report volume overwhelms auditors         | Executive summary with drill-down sections   |
| Privacy concerns in evidence data         | Aggregate metrics only; no individual code   |

## Work Items

### COMPLY-001: Compliance framework registry

- **Intent:** Define a registry for compliance frameworks and their controls
- **Expected Outcome:** Registry loads built-in and custom frameworks from YAML
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Policy evaluation
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- compliance_registry`
- **Confidence:** high

### COMPLY-002: SOC 2 and ISO 27001 framework definitions

- **Intent:** Ship built-in mappings for the two most common frameworks
- **Expected Outcome:** YAML definitions cover relevant engineering controls
- **Scope:** `crates/anvil-policy-engine/src/compliance/frameworks/`
- **Non-scope:** Legal interpretation
- **Dependencies:** COMPLY-001
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- framework_definitions`
- **Confidence:** medium

### COMPLY-003: Policy-to-control mapper

- **Intent:** Link policies to framework controls via tags
- **Expected Outcome:** Mapper resolves tags and reports coverage per control
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Evidence collection
- **Dependencies:** COMPLY-001
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- compliance_mapper`
- **Confidence:** high

### COMPLY-004: Evidence collector

- **Intent:** Aggregate evaluation results, exceptions, and drift data as evidence
- **Expected Outcome:** Collector pulls from gate runs, suppressions, and drift snapshots
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Report formatting
- **Dependencies:** COMPLY-003
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- evidence_collector`
- **Confidence:** medium

### COMPLY-005: Compliance posture scoring

- **Intent:** Calculate a compliance score per framework from coverage and results
- **Expected Outcome:** Score reflects percentage of controls with active, passing policies
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Trend storage
- **Dependencies:** COMPLY-003, COMPLY-004
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- posture_scoring`
- **Confidence:** high

### COMPLY-006: Report generator

- **Intent:** Produce audit-ready reports in multiple formats
- **Expected Outcome:** Reports include executive summary, control detail, and evidence
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** PDF rendering (HTML is PDF-ready)
- **Dependencies:** COMPLY-004, COMPLY-005
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- compliance_reporter`
- **Confidence:** medium

### COMPLY-007: Historical posture tracking

- **Intent:** Store compliance posture snapshots over time for trend analysis
- **Expected Outcome:** Posture history stored in `.anvil/compliance/` with timestamps
- **Scope:** `crates/anvil-policy-engine/src/`
- **Non-scope:** Dashboard visualisation
- **Dependencies:** COMPLY-005
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- posture_tracking`
- **Confidence:** high

### COMPLY-008: CLI compliance commands

- **Intent:** Expose compliance status, reporting, and mapping via the CLI
- **Expected Outcome:** `anvil compliance status`, `report`, and `map` commands work
- **Scope:** `crates/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** COMPLY-005, COMPLY-006
- **Validation:** `cargo test -p eddacraft-anvil -- compliance`
- **Confidence:** high
