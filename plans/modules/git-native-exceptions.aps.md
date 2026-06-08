# Git-Native Exceptions

| ID | Owner | Status |
|----|-------|--------|
| EXCEPT | @josh | In Progress |

**Last reviewed:** 2026-06-08

> **Operator-authorised (2026-06-06).** The storage-path migration slice
> (EXCEPT-001/002, now Done) was authorised for immediate execution as the
> ADR-073 reconciliation. `ExceptionStore`
> (`crates/anvil-policy/src/exceptions.rs`) previously persisted to
> `.anvil/exceptions.json` (gitignored, local-only); it now writes the tracked
> `anvil/exceptions/store.json` with a legacy read-fallback and a one-time,
> non-destructive migration, so exceptions travel with the repo. The store
> still has **no callers**: exceptions remain **unenforced** (a hand-written
> `anvil/exceptions/store.json` does nothing) until EXCEPT-006 wires
> evaluation, and the first write surface is gated on the EXCEPT-007
> hardening contract.
> EXCEPT-003 is Done after ADR-073 and EXCEPT-007 cleared the required
> state-boundary and write-path gates. Remaining CLI, L3/L4, and capsule
> integration items stay Proposed pending next-work authorisation. Brainstorm:
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
- Write-path hardening (provenance, locking, read-only worktrees, symlink
  guard) before any CLI write surface.
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
- **Validation:** `cargo test -p eddacraft-anvil-policy exceptions` (19 passed, incl. `save_writes_tracked_path_not_legacy`)
- **Status:** Done

### EXCEPT-002: Legacy read-fallback + migration
- **Intent:** Read the legacy `.anvil/exceptions.json` when the tracked store is absent; provide an idempotent, non-destructive `migrate` that copies legacy → tracked and leaves the legacy file in place.
- **Expected Outcome:** Existing local exceptions keep working; `migrate` moves them into the tracked tree without data loss.
- **Validation:** `cargo test -p eddacraft-anvil-policy exceptions` (incl. `load_falls_back_to_legacy_when_tracked_absent`, `migrate_copies_legacy_to_tracked_non_destructive`, `migrate_is_idempotent_and_noop_without_legacy`)
- **Dependencies:** EXCEPT-001
- **Status:** Done

### EXCEPT-003: Enriched `anvil.exception.v1` schema
- **Intent:** Add owner/`created_by` attribution, stable exception id, finding hash, and a revoked (soft-delete) audit trail; keep backward-compatible deserialisation of the v0 shape. Decide the on-disk layout explicitly — v0 shipped a flat `store.json`, while the brainstorm (`architecture.md` §2.3, `solution.md` §5.6) sketches per-exception files under `active/`/`revoked/` — so the layout choice is deliberate, not inherited.
- **Expected Outcome:** Schema supports grant/revoke without erasure.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- exception_schema` (8 passed, 2026-06-08)
- **Dependencies:** EXCEPT-001
- **Status:** Done

### EXCEPT-004: Grant/revoke/list/show CLI
- **Intent:** `anvil exception grant|revoke|list|show` writing tracked records via `ExceptionStore::update` (the EXCEPT-007 locked primitive). Writes are **explicit-only** (human-invoked grant/revoke); no evaluation or check command writes the store implicitly, so checks never dirty a worktree. On `WriteOutcome::SkippedReadOnly`, warn and log the underlying I/O error at verbose level — the outcome deliberately conflates read-only checkouts with permission misconfig, so the diagnostic must be reachable (2026-06-08 council).
- **Expected Outcome:** Operators manage exceptions from the CLI; revocation preserves history.
- **Validation:** `cargo test -p eddacraft-anvil-cli exception`
- **Dependencies:** EXCEPT-003, EXCEPT-007
- **Status:** Proposed

### EXCEPT-005: Scope/expiry verification
- **Intent:** `anvil exception verify` validates scope globs, expiry at evaluation time, and revocation status.
- **Expected Outcome:** Expired/revoked exceptions do not apply; an unattributed v0-shape grant downgrades (`warn`/`degraded`), it is never silently honoured.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- exception_verify`
- **Dependencies:** EXCEPT-003
- **Status:** Proposed

### EXCEPT-006: L3/L4 integration
- **Intent:** Apply only valid exceptions during pre-commit/pre-push evaluation; record exception use.
- **Expected Outcome:** Gates suppress only matching, valid findings; use is recorded.
- **Validation:** `cargo test -p eddacraft-anvil-l4 -- exceptions`
- **Dependencies:** EXCEPT-005, EXCEPT-007
- **Status:** Proposed

### EXCEPT-007: Write-path hardening (pre-wiring contract)
- **Intent:** Close the council-identified (2026-06-08) write-path gaps before any caller is wired: (a) `load()` reports provenance (tracked/legacy/none) and `save()` refuses — or requires an explicit migrate acknowledgement — when the in-memory store originated from the legacy path, so the load→modify→save CRUD flow cannot silently promote local-only entries into git (ADR-073 demands an *explicit* cleanup step); (b) `flock` the load-modify-save cycle mirroring `anvil-witness::WitnessWriter`, closing the lost-write race and the `migrate()` exists→save TOCTOU; (c) read-only worktrees degrade to warn + no-op, never a propagated `Io` error from a gate (ADR-002); (d) refuse symlinked `anvil/exceptions/` path components (canonicalise + assert under the workspace root) — the same guard the capsule writer will need.
- **Expected Outcome:** The first CLI wiring inherits a safe store: no silent legacy promotion, no concurrent lost writes, no read-only blocking, no symlink escape.
- **Validation:** `cargo test -p eddacraft-anvil-policy exceptions` (incl. new provenance/lock/read-only/symlink tests)
- **Dependencies:** EXCEPT-001, EXCEPT-002
- **Status:** Merged 2026-06-08 via PR #2366

### EXCEPT-008: Operator write semantics + guidance
- **Intent:** Document the operator contract for the tracked store: `anvil/exceptions/store.json` is committed like `anvil/baseline.json`; writes happen only via explicit grant/revoke (EXCEPT-004), so checks never dirty a worktree; evaluate `.gitattributes` (`merge=union`) for concurrent-branch grant conflicts; an upgrade note covers the legacy→tracked migration step.
- **Expected Outcome:** A downstream operator knows to commit the store, why a worktree changed, and how to migrate.
- **Validation:** `pnpm docs:check`
- **Dependencies:** EXCEPT-004
- **Status:** Proposed

### EXCEPT-009: Capsule inclusion
- **Intent:** Applied exceptions are collected into the capsule and re-verified during `anvil capsule verify` (scope/expiry/revocation).
- **Expected Outcome:** A capsule names the exceptions a change relied on; an expired/revoked one degrades or blocks verification.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- exceptions`
- **Dependencies:** EXCEPT-005, GITGOV-009
- **Status:** Proposed
