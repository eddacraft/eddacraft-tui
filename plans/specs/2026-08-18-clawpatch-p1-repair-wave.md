# Clawpatch P1 repair wave

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for CLAWFIX design | [CLAWFIX](../modules/clawpatch-p1-repair-wave.aps.md) | Accepted | 2026-08-18 — operator promoted the selected repair wave |

| Upstream | Downstream |
| -------- | ---------- |
| Clawpatch findings selected during the 2026-08-18 complete-store triage; current source and focused baseline tests at `67fb0fdd` | `plans/modules/clawpatch-p1-repair-wave.aps.md`; the nine named source/test surfaces |

**Execution authority** is CLAWFIX-001..006. This specification fixes the
repair invariants; it does not promote the remaining finding tail, edit the
shared CIB, authorise merge, or make a release claim.

## Problem

Nine confirmed findings form six repair clusters. Each is individually small,
but together they cross documentation gates, hosted administration, OAuth,
migration safety, L4 policy, and capsule filesystem containment. Implementing
them without one explicit design would invite inconsistent boundary choices:
counting instead of identifying baseline occurrences, check-then-act database
writes, or another path check that happens after the unsafe operation.

## Decisions

1. **Tag baselines are a consumable multiset.** The existing baseline file
   shape remains compatible, but each stored message suppresses at most one
   matching finding. A second identical occurrence in the same file remains an
   error.
2. **Retired survivors are identified, not counted.** A survivor baseline
   stores SHA-256 fingerprints over its path and adjacent trimmed line context.
   Moving the phrase to an unrelated line changes the fingerprint. Any tracked
   file that cannot be read is a tooling failure (exit 2), reported after the
   scan rather than treated as a clean corpus.
3. **Containment precedes expansion.** `check-asbuilt-paths` resolves every
   normalised reference lexically against the repository root and rejects an
   escaping relative or absolute path before invoking globby.
4. **Approval is one database claim.** A data-modifying CTE conditionally
   updates `waitlist` with `approved_at IS NULL` and derives the user,
   grant, and successful-approval audit writes from that claimed row. Zero
   returned rows means no approval grant, success audit, audience transition,
   or invite. A rejected no-scope request remains a distinct auditable operator
   attempt and occurs before the claim. Email/audience calls happen only after
   the claim statement succeeds.
5. **OAuth exchange and revocation share the upstream bound.** Both GitHub
   fetches receive `AbortSignal.timeout(8_000)`; the existing generic
   authentication-failure response remains the public contract.
6. **Cutoffs use full object IDs.** L4 accepts the two Git object formats the
   repository supports: 40 hexadecimal characters for SHA-1 and 64 for
   SHA-256. Abbreviations are rejected at parse and pin boundaries rather than
   resolved implicitly in a pure policy library.
7. **Capsules publish completed directories.** On Unix, the writer requires an
   owner-controlled parent, captures the created staging inode, verifies the
   opened directory's identity, owner, mode, and emptiness, then creates every
   file relative to its pinned descriptor. On Windows, the writer creates the
   staging directory atomically with a protected owner-only DACL beneath a
   pinned no-reparse parent, verifies the returned handle's owner and protected
   DACL, and creates every file relative to that handle. Both paths write the
   manifest last and
   rename the held completed directory into place without following a symlink
   or junction. Known staging content is removed on error; a staging directory
   moved by a hostile same-identity process may remain empty at its unknown new
   name, but receives no evidence.
8. **Dry-run performs only read queries.** It probes tracking-table existence
   without creating it; absence means an empty applied set. The normal apply
   path still creates the table before reading or writing migration records.

## Verification strategy

Every cluster starts with a regression that fails on the clean base for the
finding's stated reason, then receives the minimum implementation to turn it
green. Focused suites run after each slice. The wave finishes with
`pnpm validate:changed`, the aggregate docs gate, full anvil-api tests, both
Rust crate suites, and independent review.

## Rollback and compatibility

All changes are code and test changes; there is no data migration. The
documentation baseline file shape remains compatible for tags. Retired-claim
baseline entries move from counts to fingerprints, and the live list currently
has no survivors, so no existing allowance is invalidated. The approval CTE
uses PostgreSQL data-modifying CTEs already supported by the hosted database.
Rollback is a normal code revert, except that restoring the prior approval path
would knowingly restore duplicate-grant risk.
