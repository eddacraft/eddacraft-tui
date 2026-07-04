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
> state-boundary and write-path gates. L3/L4 integration (EXCEPT-006) was
> authorised 2026-07-04; remaining CLI and capsule integration items stay
> Proposed pending next-work authorisation. Brainstorm:
> [`../brainstorms/git-native-governance/`](../brainstorms/git-native-governance/).

2026-06-12: items confirmed in the v0.8.0-beta tag (record:
plans/releases/v0.8.0-beta.md) advanced to Released/Shipped; enforcement
integration remains future work.

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
- **Expected Outcome:** Operators manage exceptions from the CLI; revocation preserves history. 2026-07-04: also ships `verify` (surfaces EXCEPT-005 verdicts plus summary counts, exits zero) and `migrate` (the ADR-073 explicit legacy→tracked promotion step); grant refuses unattributed records (including invisible-unicode attribution) and duplicate ids, warns on repo-wide scope / missing expiry; id-addressed verbs refuse ambiguous stores. Contract note: on a skipped write (read-only worktree), explicit-write commands warn **and exit non-zero** — the module's warn-not-block posture applies to gates and checks, not to a human-invoked write that did not persist.
- **Validation:** `cargo test -p eddacraft-anvil --bin anvil exception` and `cargo test -p eddacraft-anvil --test exception`
- **Dependencies:** EXCEPT-003, EXCEPT-007
- **Status:** Merged 2026-07-04 via PR #3153

### EXCEPT-005: Scope/expiry verification
- **Intent:** `anvil exception verify` validates scope globs, expiry at evaluation time, and revocation status.
- **Expected Outcome:** Expired/revoked exceptions do not apply; an unattributed v0-shape grant downgrades (`warn`/`degraded`), it is never silently honoured.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- exception_verify`
- **Dependencies:** EXCEPT-003
- **Status:** Merged 2026-06-08 via PR #2413

### EXCEPT-006: L3/L4 integration
- **Intent:** Apply only valid exceptions during gate evaluation; record exception use. The only rule-evaluation seam today is the L4 gate (`CommitAntipatternEngine`, serving the pre-push hook and `anvil l4-validate`); pre-commit (L3) writes witness lines without evaluating rules, so it inherits the engine-level seam when scanner integration lands (2026-07-04 council).
- **Expected Outcome:** Gates suppress only matching, valid findings — attributed grants suppress cleanly, unattributed grants downgrade to an annotated warn (ADR-073, never silently honoured), revoked/expired/out-of-scope grants leave findings standing; store-read failures fail safe (findings stand). Use is recorded via the gate's tracing channel (unattributed applications at `warn`, visible under the default filter); durable witness/capsule recording is EXCEPT-009.
- **Validation:** `cargo test -p eddacraft-anvil-l4 -- exceptions` and `cargo test -p eddacraft-anvil --bin anvil -- l4_engine` (gate-integration tests live in the CLI engine)
- **Dependencies:** EXCEPT-005, EXCEPT-007
- **Status:** Merged 2026-07-04 via PR #3140

### EXCEPT-007: Write-path hardening (pre-wiring contract)
- **Intent:** Close the council-identified (2026-06-08) write-path gaps before any caller is wired: (a) `load()` reports provenance (tracked/legacy/none) and `save()` refuses — or requires an explicit migrate acknowledgement — when the in-memory store originated from the legacy path, so the load→modify→save CRUD flow cannot silently promote local-only entries into git (ADR-073 demands an *explicit* cleanup step); (b) `flock` the load-modify-save cycle mirroring `anvil-witness::WitnessWriter`, closing the lost-write race and the `migrate()` exists→save TOCTOU; (c) read-only worktrees degrade to warn + no-op, never a propagated `Io` error from a gate (ADR-002); (d) refuse symlinked `anvil/exceptions/` path components (canonicalise + assert under the workspace root) — the same guard the capsule writer will need.
- **Expected Outcome:** The first CLI wiring inherits a safe store: no silent legacy promotion, no concurrent lost writes, no read-only blocking, no symlink escape.
- **Validation:** `cargo test -p eddacraft-anvil-policy exceptions` (incl. new provenance/lock/read-only/symlink tests)
- **Dependencies:** EXCEPT-001, EXCEPT-002
- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-08 via PR #2366

### EXCEPT-008: Operator write semantics + guidance
- **Intent:** Document the operator contract for the tracked store: `anvil/exceptions/store.json` is committed like `anvil/baseline.json`; writes happen only via explicit grant/revoke (EXCEPT-004), so checks never dirty a worktree; evaluate `.gitattributes` (`merge=union`) for concurrent-branch grant conflicts; an upgrade note covers the legacy→tracked migration step.
- **Expected Outcome:** A downstream operator knows to commit the store, why a worktree changed, and how to migrate.
- **Validation:** `pnpm docs:check` (docs/guides/policy-exceptions.md; aps/adr surface failures pre-exist on main)
- **Dependencies:** EXCEPT-004
- **Status:** Merged 2026-07-04 via PR #3156

### EXCEPT-009: Capsule inclusion
- **Intent:** Applied exceptions are collected into the capsule and re-verified during `anvil capsule verify` (scope/expiry/revocation). 2026-07-04: "applied" is approximated by "active at collect time" — the faithful relied-upon subset needs the gate's applied-exception record joined against a diagnostics source, and capsule create has no diagnostics source wired yet (`diagnostics.sarif` is an empty document for the same reason). The approximation is conservative for verification: a superset of anything the gate could have applied is collected and re-verified, so a since-revoked or expiring grant still degrades/blocks verify. Tightening to the true applied subset follows the create-side diagnostics wiring.
- **Expected Outcome:** A capsule names the exceptions a change relied on; an expired/revoked one degrades or blocks verification.
- **Validation:** `cargo test -p eddacraft-anvil-capsule -- exceptions`
- **Dependencies:** EXCEPT-005, GITGOV-009
- **Status:** Merged 2026-07-04 via PR #3155

### EXCEPT-010: Gate store trust model (2026-07-04 council intake)
- **Intent:** Decide gate store provenance — the L4 gate loads `anvil/exceptions/store.json` from the live worktree (same pattern as `anvil/policy.yml`), so an uncommitted local grant satisfies the local pre-push gate while CI/`l4-validate` sees only committed grants, and a range validation applies one store snapshot to every commit in the range; decide worktree- vs commit-tree-scoped loading and bind the CI semantics explicitly. 2026-07-04 scope reduction: the item's original (b) — make the OPA evaluator's `is_suppressed_at` consult `ExceptionVerdict` — was mooted by ADR-098 PR-C deleting the OPA-subprocess evaluator; `is_suppressed_at`/`filter_suppressed` have no production callers (dead-code disposition belongs to the OPAE rebuild). Original (c) — scope-breadth nudges — shipped via EXCEPT-004's grant-time warnings (repo-wide scope, missing expiry).
- **Expected Outcome:** A recorded provenance decision (ADR or decision-log entry) enforced by the gate loader, with CI semantics bound explicitly. 2026-07-04 owner decision: **tip-of-pushed-range tree loading (ADR-100)** — suppression authority must be committed; pre-push uses `local_sha`, `l4-validate` uses the range head, audit-chain uses the audited checkout's HEAD; tip-without-store/unreadable/oversized = no exceptions (fail-safe); legacy local store never influences gates.
- **Validation:** ADR-100 + `cargo test -p eddacraft-anvil --bin anvil -- l4_engine` (incl. `uncommitted_worktree_grant_does_not_apply`)
- **Dependencies:** EXCEPT-006
- **Status:** In Progress
