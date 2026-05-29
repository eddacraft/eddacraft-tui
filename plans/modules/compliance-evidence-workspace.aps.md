# Compliance Evidence Workspace

| ID   | Owner  | Status |
| ---- | ------ | ------ |
| CEWS | @aneki | Draft  |

**Last reviewed:** 2026-04-26

> **Status correction 2026-04-26:** Demoted Ready → Draft per Council C
> finding. CEWS depends on COMPLY's evidence collector
> (COMPLY-004 specifically). COMPLY is itself Draft; CEWS being Ready
> while its upstream is Draft was backwards. Promote back to Ready when
> COMPLY-001..004 land, or rescope CEWS to drop the compliance-framework
> coupling.

> NOTE(post-rust): Validation targets updated to Rust workspace.
> Dependency `policy-lifecycle` is archived; the upstream
> `compliance-reporting` (COMPLY) module remains Draft and still
> references retired TS paths — re-validate the dependency once COMPLY
> is rewritten against the Rust crates.

## Purpose

Create an Anvil-native workspace for compliance evidence mapping that links controls, policy outcomes, and audit artifacts in a clear, operational model.

## In Scope

- Control-to-evidence mapping model
- Evidence status and ownership workflow
- Exportable trust/compliance summaries
- Evidence linkage from policy/eval outcomes

## Out of Scope

- External compliance platform embedding
- Full GRC workflow replacement

## Interfaces

**Depends on:**
- `compliance-reporting`
- `policy-lifecycle`
- `eval-harness-integration`

**Exposes:**
- `ControlEvidenceMap`
- `EvidenceRecord`
- `ComplianceWorkspaceReport`

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined

## Work Items

### CEWS-001: Define control-evidence data model
- **Intent:** Establish canonical entities for controls, evidence, ownership, and status.
- **Expected Outcome:** Data model supports traceability from policy to artifact.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- control_evidence_model`

### CEWS-002: Build evidence ingestion and linking
- **Intent:** Link policy and eval outcomes to evidence records.
- **Expected Outcome:** Evidence records auto-link to relevant controls and runs.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- evidence_linking`
- **Dependencies:** CEWS-001

### CEWS-003: Add evidence workspace views/contracts
- **Intent:** Provide operational views for status, gaps, and ownership.
- **Expected Outcome:** Workspace outputs support compliance review workflows.
- **Validation:** `cargo test -p eddacraft-anvil -- evidence_workspace`
- **Dependencies:** CEWS-002

### CEWS-004: Add export packs
- **Intent:** Generate audit-friendly exports from workspace state.
- **Expected Outcome:** Exports include control status, evidence links, and timestamps.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- compliance_export`
- **Dependencies:** CEWS-003

## Execution

Steps: [../execution/CEWS.steps.md](../execution/CEWS.steps.md)
