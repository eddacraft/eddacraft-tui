# Intent Ledger Governance (Anvil)

| ID    | Owner  | Status | Progress |
| ----- | ------ | ------ | -------- |
| ILGOV | @aneki | Draft  | 0/6      |

**Last reviewed:** 2026-07-17 (POLRESET topology flow-down; the module remains
Draft pending product timing and Rust-contract co-design with CONF).

> **Audit note (2026-04-26):** Status demoted Ready → Draft pending rescope.
> Retained — the thesis (intent-vs-effect provenance) was the **original
> Anvil use case**: all Anvil was going to be a system for proving the
> plan was followed. The current version is more powerful — Anvil uses
> the symbol/architecture graph (`crates/anvil-architecture`,
> `crates/anvil-kernel`) to **predict effect** of a change and compare
> against captured **intent**, rather than relying on after-the-fact
> diffing alone.
>
> Earlier audit pass framed the archived prerequisites
> (`edda-stack-integration`, `kindling-integration`,
> `lineage-authorship-confidence`) as evidence of staleness — that was
> a misread. Those *planning modules* are archived because their work
> items completed; the components are live code in
> `packages/kindling-integration/` and `packages/edda-stack/`. The
> archived LAC module retains the attribution overlay concept and is
> separately rescoped.
>
> **Reset disposition:**
> - Validation is now aligned with the owning Rust boundary:
>    `anvil-kernel-types` for the canonical record, the Rust CLI for
>    ingestion/correlation/explainability, and `anvil-policy-engine` for any
>    Rego-backed intent predicate. The deletion-slated `anvil-policy` support
>    crate is not an implementation home (ADR-098 AD-2).
> - Dependencies now name the live Rust crates and adapter packages rather
>   than completed planning modules.
>
> **Remaining rescope before Ready:**
> 1. Replace the illustrative TS-shaped `IntentLedgerRecord` below with a Rust
>    canonical in `crates/anvil-kernel-types`, co-designed with CONF-002 so the
>    two modules do not fork the contract.
> 2. Define graph-derived effect prediction
>    (e.g. "the symbol graph indicates this change touches scopes outside
>    the declared `scope_in`") as a first-class policy predicate.
>
> Tier C parking lot post-launch — not competing with RTAI for current
> release attention. Promote to Ready only after a product-timing decision and
> the CONF contract boundary are settled.

## Purpose

Define Anvil's canonical governance layer for intent provenance.

Kindling captures intent at the edge; Anvil must ingest, normalize, verify, and
apply policy to that intent so teams can prove alignment from:

**plan → intent event → code diff → gate decision → deploy outcome**.

## In Scope

- `IntentLedgerRecord` canonical schema for Anvil storage/query
- Ingestion adapter for Kindling JSONL export bundles
- Integrity verification (hash chain + sequence continuity + signature metadata)
- Intent-to-change correlation model (record ↔ commit ↔ PR ↔ warning/gate)
- Gate policy primitives using intent assertions (required constraints, scope fences)
- Explainability surfaces (`anvil intent explain`, gate evidence attachment)

## Out of Scope

- Raw intent capture UX (Kindling-owned)
- LLM interpretation of intent quality beyond deterministic rules
- Multi-org federation and long-term archival strategy

## Interfaces

**Depends on:**

- `crates/anvil-kernel-types` (canonical Rust record and reason-code contracts)
- `crates/anvil-architecture` and `crates/anvil-kernel` (graph-derived effect
  prediction)
- `packages/edda-stack/` and `packages/kindling-integration/` (live provenance
  and export adapters from the completed integration modules)
- CONF-002 (co-design constraint: extend one canonical intent record rather
  than fork it)

**Exposes:**

- `IntentLedgerRecord` schema + validation rules
- Ingestion command/API for intent bundles
- Policy predicates for intent-aware gate checks
- Evidence bundle section for intent lineage

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined

## Illustrative Record Shape (pre-rescope)

This TypeScript-era sketch preserves the product fields under discussion; it
is not the canonical implementation contract. ILGOV-001 must replace it with
the Rust type co-designed with CONF-002 before the module can become Ready.

```ts
interface IntentLedgerRecord {
  ledger_id: string;
  ingest_batch_id: string;
  source: {
    system: 'kindling';
    export_version: string;
    event_id: string;
    sequence: number;
  };
  context: {
    repo: string;
    branch?: string;
    commit?: string;
    pr?: string;
    session_id?: string;
  };
  objective: string;
  constraints: string[];
  success_criteria: string[];
  scope_in: string[];
  scope_out: string[];
  integrity: {
    event_hash: string;
    previous_hash?: string;
    verified: boolean;
    verification_reason_codes: string[];
  };
  links: {
    warning_ids: string[];
    gate_ids: string[];
    finding_ids: string[];
  };
  timestamps: {
    occurred_at: string;
    ingested_at: string;
  };
}
```

## Work Items

### ILGOV-001: Define `IntentLedgerRecord` + validation contract

- **Intent:** Establish one canonical shape for all downstream policy and audit usage.
- **Expected Outcome:** Shared schema package + migration/versioning guidance.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- intent_ledger`
- **Status:** Draft

### ILGOV-002: Build Kindling bundle ingestion adapter

- **Intent:** Convert exported Kindling events into normalized ledger records.
- **Expected Outcome:** Deterministic ingest pipeline with idempotency keys and replay support.
- **Validation:** `cargo test -p eddacraft-anvil -- intent_ingest`
- **Dependencies:** ILGOV-001
- **Status:** Draft

### ILGOV-003: Implement integrity verification pipeline

- **Intent:** Reject or quarantine tampered/incomplete intent streams.
- **Expected Outcome:** Hash-chain verification, sequence-gap detection, and reason-coded outcomes.
- **Validation:** `cargo test -p eddacraft-anvil -- intent_integrity`
- **Dependencies:** ILGOV-001, ILGOV-002
- **Status:** Draft

### ILGOV-004: Correlate intent to commits/PRs/gates

- **Intent:** Make intent lineage queryable from engineering artifacts.
- **Expected Outcome:** Bidirectional links between intent records and warnings/findings/gates.
- **Validation:** `cargo test -p eddacraft-anvil -- intent_correlation`
- **Dependencies:** ILGOV-002
- **Status:** Draft

### ILGOV-005: Add intent-aware policy predicates

- **Intent:** Enable deterministic guardrails against scope drift and missing constraints.
- **Expected Outcome:** Policy rules can assert required intent fields and detect out-of-scope changes.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- intent_policy`
- **Dependencies:** ILGOV-003, ILGOV-004
- **Status:** Draft

### ILGOV-006: Add explainability and evidence export surfaces

- **Intent:** Let reviewers/auditors see exactly why a gate passed/failed against intent.
- **Expected Outcome:** `anvil intent explain` + intent section in gate evidence bundles.
- **Validation:** `cargo test -p eddacraft-anvil -- intent_explain`
- **Dependencies:** ILGOV-003, ILGOV-004
- **Status:** Draft
