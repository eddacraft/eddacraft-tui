# Line-Level Authorship and Confidence

| ID | Owner | Status |
|----|-------|--------|
| LAC | @aneki | Ready |

## Purpose

Provide deterministic, auditable attribution for code changes at file/line granularity so Anvil can answer who authored each line (human, AI, mixed, unknown), what model was involved when known, and how confident that attribution is.

## In Scope

- Canonical line-level attribution schema
- Multi-source attribution collection (git + session/tool metadata)
- Reconciliation and confidence scoring with reason codes
- Query surfaces for line blame and PR-level attribution summaries
- Exportable evidence bundles for audit and verification
- Language allocation guardrails (TS vs Rust decision tree reference)

## Out of Scope

- Full historical backfill for all prior repository history
- Real-time IDE heatmaps in V1
- Non-git ecosystems without adapters

## Interfaces

**Depends on:**
- `rust-kernel` — optional performance acceleration path
- `opa-agent-orchestration` — policy-linked use of attribution outcomes
- `compliance-evidence-workspace` — evidence mapping and reporting paths

**Exposes:**
- `LineAttributionRecord`
- `AuthorshipConfidenceResult`
- `authorship blame` and `authorship summary` CLI outputs
- Attribution evidence export bundles

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined

## Tasks

### LAC-001: Define canonical line attribution schema
- **Intent:** Standardize the attribution contract used across collection, storage, and query.
- **Expected Outcome:** A single validated schema exists for actor type, model fields, evidence refs, and confidence.
- **Validation:** `pnpm nx test contracts --testNamePattern="attribution schema"`

### LAC-002: Implement attribution collectors
- **Intent:** Capture attribution signals from git metadata and AI/session/tool sources.
- **Expected Outcome:** Collector outputs provide mergeable attribution evidence with source provenance.
- **Validation:** `pnpm nx test anvil-cli --testNamePattern="provenance collector"`
- **Dependencies:** LAC-001

### LAC-003: Implement reconciliation + confidence engine
- **Intent:** Resolve competing attribution signals into deterministic line-level outcomes with confidence and reasons.
- **Expected Outcome:** Each attributed line/range has actor classification, optional model identity, confidence score/band, and reason codes.
- **Validation:** `pnpm nx test anvil-cli --testNamePattern="authorship confidence"`
- **Dependencies:** LAC-001, LAC-002

### LAC-004: Persist and query line-level attribution
- **Intent:** Make attribution results retrievable by file/line, commit, and PR scope.
- **Expected Outcome:** Query APIs return low-latency line-level attribution and summary distributions.
- **Validation:** `pnpm nx test anvil-cli --testNamePattern="authorship store"`
- **Dependencies:** LAC-003

### LAC-005: Expose authorship CLI surfaces
- **Intent:** Provide practical user-facing commands for trust and review workflows.
- **Expected Outcome:** `authorship blame` and `authorship summary` commands produce stable, explainable output.
- **Validation:** `pnpm nx test anvil-cli --testNamePattern="authorship command"`
- **Dependencies:** LAC-004

### LAC-006: Export and sign attribution evidence bundles
- **Intent:** Support compliance and external verification with portable evidence output.
- **Expected Outcome:** Export artifacts include attribution results, confidence rationale, and signing status.
- **Validation:** `pnpm nx test anvil-cli --testNamePattern="authorship export"`
- **Dependencies:** LAC-004

## Decision Reference

Language allocation (TypeScript vs Rust kernel) is governed by:
`plans/decisions/014-language-allocation-tree-ts-vs-rust.md`

## Execution

Steps: [../execution/LAC.steps.md](../execution/LAC.steps.md)
