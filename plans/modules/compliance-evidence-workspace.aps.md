# Compliance Evidence Workspace

| ID   | Owner  | Status |
| ---- | ------ | ------ |
| CEWS | @aneki | Draft  |

**Last reviewed:** 2026-07-17 (POLRESET topology flow-down: dependencies and
future implementation homes reconciled with ADR-098 and current Rust truth).

> **Status correction 2026-04-26:** Demoted Ready → Draft per Council C
> finding. CEWS depends on COMPLY's evidence collector
> (COMPLY-004 specifically). COMPLY is itself Draft; CEWS being Ready
> while its upstream is Draft was backwards. Promote back to Ready when
> COMPLY-001..004 land, or rescope CEWS to drop the compliance-framework
> coupling.

> **Reset posture (POLRESET-010 / ADR-098, reviewed 2026-07-17):** CEWS is
> post-first-slice enterprise expansion, not a prerequisite for policy value.
> `policy-lifecycle` (POLLC) is a live Draft module, not archived, and
> `compliance-reporting` (COMPLY) has been retargeted to the Rust/regorus
> product path. CEWS remains Draft until COMPLY-001..004 and the required
> POLLC lifecycle contracts land, or until a separate design deliberately
> removes the compliance-framework coupling. New work must use
> `anvil-policy-engine`, `anvil-kernel-types`, and the Rust CLI; the
> deletion-slated `anvil-policy` support crate is not an implementation home.

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
- `compliance-reporting` (COMPLY-001..004, Draft)
- `policy-lifecycle` (POLLC, Draft)
- `eval-harness-integration` (EVAL, Complete)

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
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- evidence_linking`
- **Dependencies:** CEWS-001

### CEWS-003: Add evidence workspace views/contracts
- **Intent:** Provide operational views for status, gaps, and ownership.
- **Expected Outcome:** Workspace outputs support compliance review workflows.
- **Validation:** `cargo test -p eddacraft-anvil -- evidence_workspace`
- **Dependencies:** CEWS-002

### CEWS-004: Add export packs
- **Intent:** Generate audit-friendly exports from workspace state.
- **Expected Outcome:** Exports include control status, evidence links, and timestamps.
- **Validation:** `cargo test -p eddacraft-anvil -- compliance_export`
- **Dependencies:** CEWS-003

## Execution

Action plan: [../execution/CEWS.actions.md](../execution/CEWS.actions.md)
