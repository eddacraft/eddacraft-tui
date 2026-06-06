# Git-Native Exceptions

| ID | Owner | Status |
|----|-------|--------|
| EXCEPT | @josh | In Progress |

**Last reviewed:** 2026-06-06

> **Operator-authorised (2026-06-06).** The storage-path migration slice
> (EXCEPT-001/002) is authorised for immediate execution: it is the ADR-073
> reconciliation and fixes a live governance gap. `ExceptionStore`
> (`crates/anvil-policy/src/exceptions.rs`) currently persists to
> `.anvil/exceptions.json` (gitignored, local-only) and has **no callers** —
> so exceptions neither travel with the repo nor are wired into evaluation yet.
> Remaining items (schema enrichment, CLI, L3/L4 + capsule integration) stay
> Proposed pending review. Brainstorm:
> [`../brainstorms/git-native-governance/`](../brainstorms/git-native-governance/).

## Purpose

Make intentional policy deviations **scoped, expiring, attributed, reviewable,
revocable, and included in governance evidence** — and, first, **durable**: move
exception storage out of the gitignored `.anvil/` tree into tracked `anvil/`
([ADR-073](../decisions/073-durable-vs-local-anvil-state.md)) so exceptions
travel with the repository and are visible in PR review. This is the file-based
sibling of the in-source `@anvil-ignore` suppression syntax (ADR-004).

## In Scope

- Tracked storage under `anvil/exceptions/` with legacy read-fallback + a
  one-time, non-destructive migration.
- Enriched `anvil.exception.v1` schema (owner/attribution, revocation audit).
- `anvil exception grant|revoke|list|show|verify` CLI.
- L3/L4 evaluation integration; witness-envelope + capsule inclusion.

## Out of Scope

Enforcement of unrelated policy classes; the inline `@anvil-ignore` path
(ADR-004, already in-tree); any auto-deletion of legacy data.

## Interfaces

- `crates/anvil-policy/src/exceptions.rs` (`ExceptionStore`, `PolicyException`).
- Future CLI: `crates/anvil-cli/src/commands/exception.rs`.
- Consumed by GITGOV capsule verification (GITGOV-009).

## Work Items

### EXCEPT-001: Tracked storage path
- **Intent:** Persist exceptions under tracked `anvil/exceptions/` instead of gitignored `.anvil/exceptions.json`, so they travel with the repo (ADR-073).
- **Expected Outcome:** `ExceptionStore::save` writes `anvil/exceptions/store.json`; `load` prefers the tracked path.
- **Validation:** `cargo test -p eddacraft-anvil-policy exceptions`
- **Status:** In Progress

### EXCEPT-002: Legacy read-fallback + migration
- **Intent:** Read the legacy `.anvil/exceptions.json` when the tracked store is absent; provide an idempotent, non-destructive `migrate` that copies legacy → tracked and leaves the legacy file in place.
- **Expected Outcome:** Existing local exceptions keep working; `migrate` moves them into the tracked tree without data loss.
- **Validation:** `cargo test -p eddacraft-anvil-policy exceptions`
- **Dependencies:** EXCEPT-001
- **Status:** In Progress

### EXCEPT-003: Enriched `anvil.exception.v1` schema
- **Intent:** Add owner/`created_by` attribution, stable exception id, finding hash, and a revoked (soft-delete) audit trail; keep backward-compatible deserialisation of the v0 shape.
- **Expected Outcome:** Schema supports grant/revoke without erasure.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- exception_schema`
- **Dependencies:** EXCEPT-001
- **Status:** Proposed

### EXCEPT-004: Grant/revoke/list/show CLI
- **Intent:** `anvil exception grant|revoke|list|show` writing tracked records.
- **Expected Outcome:** Operators manage exceptions from the CLI; revocation preserves history.
- **Validation:** `cargo test -p eddacraft-anvil-cli exception`
- **Dependencies:** EXCEPT-003
- **Status:** Proposed

### EXCEPT-005: Scope/expiry verification
- **Intent:** `anvil exception verify` validates scope globs, expiry at evaluation time, and revocation status.
- **Expected Outcome:** Expired/revoked exceptions do not apply.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- exception_verify`
- **Dependencies:** EXCEPT-003
- **Status:** Proposed

### EXCEPT-006: L3/L4 integration
- **Intent:** Apply only valid exceptions during pre-commit/pre-push evaluation; record exception use.
- **Expected Outcome:** Gates suppress only matching, valid findings; use is recorded.
- **Validation:** `cargo test -p eddacraft-anvil-l4 -- exceptions`
- **Dependencies:** EXCEPT-005
- **Status:** Proposed

### EXCEPT-009: Capsule inclusion
- **Intent:** Applied exceptions are collected into the capsule and re-verified during `anvil capsule verify` (scope/expiry/revocation).
- **Expected Outcome:** A capsule names the exceptions a change relied on; an expired/revoked one degrades or blocks verification.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- exceptions`
- **Dependencies:** EXCEPT-005, GITGOV-009
- **Status:** Proposed
