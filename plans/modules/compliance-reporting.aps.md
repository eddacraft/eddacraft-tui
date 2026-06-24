<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Compliance Reporting

| ID  | Owner | Priority | Status |
| ------ | ----- | -------- | ------ |
| COMPLY | —     | medium   | Draft  |

**Last reviewed:** 2026-04-26

> **Policy-solution validation (2026-06-24):** COMPLY should consume
> regorus-backed policy outcomes (`anvil policy eval --json` v1, pack metadata,
> exceptions, and eval evidence), not Go OPA output directly. The module remains
> Draft because the work items still carry TS-era paths and `nx test` commands;
> rewrite them to `crates/anvil-policy`, `crates/anvil-kernel-types`, and
> `crates/anvil-cli` before promotion.
>
> NOTE(post-rust): Task scopes/files reference the retired TS tree
> (`packages/anvil/policy/src/`, `packages/anvil/runtime/src/`,
> `apps/anvil-cli/src/commands/`). When this module moves to Ready, retarget
> to Rust crates: registry/mapper/scoring/reporter live in
> `crates/anvil-policy/src/`; evidence aggregation and posture history live
> in `crates/anvil-policy/src/` (or `crates/anvil-kernel/src/policy/` for
> drift-derived inputs); CLI commands land in
> `crates/anvil-cli/src/commands/compliance.rs`. Dependency modules
> `opa-architecture-integration`, `drift-reporting`, `suppressions`, and
> `policy-lifecycle` are all archived — capability is now in
> `crates/anvil-policy` (drift, exceptions) and the kernel.

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
- Evidence collection from gate runs, exceptions, and drift snapshots
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

- `opa-architecture-integration` — Policy evaluation results
- `org-policy-hierarchy` — Org-level compliance baselines
- `policy-lifecycle` — Policy state for active coverage reporting
- `drift-reporting` — Drift data as compliance evidence
- `suppressions` — Exception data for audit trail

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
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Policy evaluation
- **Validation:** `nx test policy --testNamePattern="compliance-registry"`
- **Confidence:** high

### COMPLY-002: SOC 2 and ISO 27001 framework definitions

- **Intent:** Ship built-in mappings for the two most common frameworks
- **Expected Outcome:** YAML definitions cover relevant engineering controls
- **Scope:** `packages/anvil/policy/src/frameworks/`
- **Non-scope:** Legal interpretation
- **Dependencies:** COMPLY-001
- **Validation:** `nx test policy --testNamePattern="framework-definitions"`
- **Confidence:** medium

### COMPLY-003: Policy-to-control mapper

- **Intent:** Link policies to framework controls via tags
- **Expected Outcome:** Mapper resolves tags and reports coverage per control
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Evidence collection
- **Dependencies:** COMPLY-001
- **Validation:** `nx test policy --testNamePattern="compliance-mapper"`
- **Confidence:** high

### COMPLY-004: Evidence collector

- **Intent:** Aggregate evaluation results, exceptions, and drift data as evidence
- **Expected Outcome:** Collector pulls from gate runs, suppressions, and drift snapshots
- **Scope:** `packages/anvil/runtime/src/`
- **Non-scope:** Report formatting
- **Dependencies:** COMPLY-003
- **Validation:** `nx test runtime --testNamePattern="evidence-collector"`
- **Confidence:** medium

### COMPLY-005: Compliance posture scoring

- **Intent:** Calculate a compliance score per framework from coverage and results
- **Expected Outcome:** Score reflects percentage of controls with active, passing policies
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** Trend storage
- **Dependencies:** COMPLY-003, COMPLY-004
- **Validation:** `nx test policy --testNamePattern="posture-scoring"`
- **Confidence:** high

### COMPLY-006: Report generator

- **Intent:** Produce audit-ready reports in multiple formats
- **Expected Outcome:** Reports include executive summary, control detail, and evidence
- **Scope:** `packages/anvil/policy/src/`
- **Non-scope:** PDF rendering (HTML is PDF-ready)
- **Dependencies:** COMPLY-004, COMPLY-005
- **Validation:** `nx test policy --testNamePattern="compliance-reporter"`
- **Confidence:** medium

### COMPLY-007: Historical posture tracking

- **Intent:** Store compliance posture snapshots over time for trend analysis
- **Expected Outcome:** Posture history stored in `.anvil/compliance/` with timestamps
- **Scope:** `packages/anvil/runtime/src/`
- **Non-scope:** Dashboard visualisation
- **Dependencies:** COMPLY-005
- **Validation:** `nx test runtime --testNamePattern="posture-tracking"`
- **Confidence:** high

### COMPLY-008: CLI compliance commands

- **Intent:** Expose compliance status, reporting, and mapping via the CLI
- **Expected Outcome:** `anvil compliance status`, `report`, and `map` commands work
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Dependencies:** COMPLY-005, COMPLY-006
- **Validation:** `nx test cli --testNamePattern="compliance"`
- **Confidence:** high
