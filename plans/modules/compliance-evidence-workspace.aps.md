# Compliance Evidence Workspace

| ID | Owner | Status |
|----|-------|--------|
| CEWS | @aneki | Ready |

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

## Tasks

### CEWS-001: Define control-evidence data model
- **Intent:** Establish canonical entities for controls, evidence, ownership, and status.
- **Expected Outcome:** Data model supports traceability from policy to artifact.
- **Validation:** `pnpm nx test contracts --testNamePattern="control evidence model"`

### CEWS-002: Build evidence ingestion and linking
- **Intent:** Link policy and eval outcomes to evidence records.
- **Expected Outcome:** Evidence records auto-link to relevant controls and runs.
- **Validation:** `pnpm nx test core --testNamePattern="evidence linking"`
- **Dependencies:** CEWS-001

### CEWS-003: Add evidence workspace views/contracts
- **Intent:** Provide operational views for status, gaps, and ownership.
- **Expected Outcome:** Workspace outputs support compliance review workflows.
- **Validation:** `pnpm nx test cli --testNamePattern="evidence workspace"`
- **Dependencies:** CEWS-002

### CEWS-004: Add export packs
- **Intent:** Generate audit-friendly exports from workspace state.
- **Expected Outcome:** Exports include control status, evidence links, and timestamps.
- **Validation:** `pnpm nx test core --testNamePattern="compliance export"`
- **Dependencies:** CEWS-003

## Execution

Steps: [../execution/CEWS.steps.md](../execution/CEWS.steps.md)
