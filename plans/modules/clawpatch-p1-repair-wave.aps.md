<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Clawpatch P1 Repair Wave

| ID      | Owner | Priority | Status      | Progress |
| ------- | ----- | -------- | ----------- | -------- |
| CLAWFIX | —     | P1       | In Progress | 0/6      |

**Last reviewed:** 2026-08-19 — CLAWFIX-001..006 implementation and wave-level
validation are complete in the dedicated Worktrunk workspace. Required
changed-file validation also exposed and repaired inherited TUI formatting and
E2E fixture drift. Review evidence is recorded below; PR #4010 is open pending
integration.

**Pull request:** [#4010](https://github.com/eddacraft/anvil-001/pull/4010)
against `main`; work remains `In Progress` pending integration.

## Purpose

Close the six highest-priority correctness and containment clusters selected
from the 2026-08-18 Clawpatch triage without absorbing the remaining actionable
tail or advisory test gaps.

## Governing design

[2026-08-18 Clawpatch P1 repair-wave design](../specs/2026-08-18-clawpatch-p1-repair-wave.md).

## In scope

- Documentation-gate baseline identity, unreadable-file failure, and glob
  containment.
- Atomic waitlist approval claiming and bounded GitHub OAuth exchange.
- Canonical L4 cutoff object IDs.
- Capsule output-directory race containment.
- Read-only migration preview.

## Out of scope

- The remaining Clawpatch actionable tail and grouped advisory test gaps.
- Shared-CIB bookkeeping.
- Product or API redesign beyond the named invariants.
- PR merge, release, or administrator-policy override.

## Work Items

### CLAWFIX-001: Preserve documentation-gate integrity

- **Status:** In Progress
- **Intent:** A baseline suppresses only the occurrence it records, an
  unreadable tracked file is never reported as clean, and an escaping glob is
  rejected before filesystem expansion.
- **Expected Outcome:** tag baselines are consumed one-for-one; retired-claim
  survivors use stable context fingerprints and unreadable files exit 2; all
  as-built references pass lexical root containment before globby runs.
- **Files:** `scripts/docs/check-tags.mjs`,
  `scripts/docs/check-retired-claims.mjs`,
  `scripts/docs/retired-claims.mjs`,
  `scripts/docs/check-asbuilt-paths.mjs`,
  `scripts/docs/docs-check.test.sh`
- **Validation:** `bash scripts/docs/docs-check.test.sh`;
  `pnpm docs:check`
- **Finding IDs:** `fnd_sig-feat-library-27f69289a1-3a46_9439dcfe20`,
  `fnd_sig-feat-library-27f69289a1-5f67_2e640dd574`,
  `fnd_sig-feat-library-27f69289a1-6835_0c21e75ea5`,
  `fnd_sig-feat-library-f261e42bd1-f562_fb6681a493`
- **Risk:** standard

### CLAWFIX-002: Atomically claim admin approvals

- **Status:** In Progress
- **Intent:** Overlapping batch requests cannot create a successful approval
  grant, success audit, audience transition, or invite for the same waitlist
  entry twice. Rejected no-scope requests remain distinct auditable operator
  attempts.
- **Expected Outcome:** one conditional database statement claims the pending
  row and creates the durable grant/audit effects only for the winner; external
  invite side effects run only after a successful claim.
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/__tests__/admin.test.ts`
- **Validation:** `pnpm --dir apps/anvil-api exec vitest run src/__tests__/admin.test.ts`
- **Finding ID:** `fnd_sig-feat-library-9ec8761ef6-7947_dba982bdeb`
- **Risk:** high

### CLAWFIX-003: Bound GitHub OAuth token exchange

- **Status:** In Progress
- **Intent:** A stalled upstream token exchange cannot hold the callback open
  indefinitely.
- **Expected Outcome:** the exchange fetch receives the same 8-second timeout
  signal as revocation; timeout and network failures retain the generic 401
  contract.
- **Files:** `apps/anvil-api/src/routes/auth-github.ts`,
  `apps/anvil-api/src/__tests__/auth-github.test.ts`
- **Validation:** `pnpm --dir apps/anvil-api exec vitest run src/__tests__/auth-github.test.ts`
- **Finding ID:** `fnd_sig-feat-library-2c10e75d6b-3214_c72afca831`
- **Risk:** standard

### CLAWFIX-004: Enforce canonical cutoff object IDs

- **Status:** In Progress
- **Intent:** Every accepted cutoff value can compare directly with the full
  object IDs produced by Git.
- **Expected Outcome:** policy parsing and pinning accept only canonical
  40-character SHA-1 or 64-character SHA-256 hexadecimal object IDs; focused
  tests pair policy cutoffs with full ancestry values.
- **Files:** `crates/anvil-l4/src/policy.rs`,
  `crates/anvil-l4/src/resolve.rs`,
  `crates/anvil-cli/src/commands/baseline.rs`,
  `crates/anvil-cli/src/commands/hook.rs`
- **Validation:** `cargo test -p eddacraft-anvil-l4`;
  `cargo test -p eddacraft-anvil --no-fail-fast`
- **Finding ID:** `fnd_sig-feat-library-a54a4d09dc-422d_d4edbfd895`
- **Risk:** standard

### CLAWFIX-005: Publish capsules without an output-directory race

- **Status:** In Progress
- **Intent:** Replacing the named output directory after validation cannot
  redirect capsule writes.
- **Expected Outcome:** all capsule files are written relative to a pinned,
  private sibling staging directory (Unix mode 0700 or a Windows protected
  owner-only DACL) and the completed directory is renamed into place only while
  its identity still matches; errors clean up known
  staging content and never write through a swapped staging or destination
  symlink.
- **Files:** `crates/anvil-capsule/src/format.rs`,
  `crates/anvil-capsule/Cargo.toml`,
  `crates/anvil-intercept-win32/src/lib.rs`,
  `crates/anvil-intercept-win32/src/path_nofollow.rs`,
  `crates/workspace-hack/Cargo.toml`, `Cargo.lock`
- **Validation:** `cargo test -p eddacraft-anvil-capsule format::tests`
  and `cargo clippy -p eddacraft-anvil-capsule --lib --target
  x86_64-pc-windows-gnu -- -D warnings` and `cargo clippy -p
  eddacraft-anvil-intercept-win32 --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings`
- **Finding ID:** `fnd_sig-feat-library-af58dd546d-d826_543cbcf557`
- **Risk:** high

### CLAWFIX-006: Keep migration dry-run read-only

- **Status:** In Progress
- **Intent:** Previewing migrations against a fresh database does not create
  the tracking table or execute any other DDL/DML.
- **Expected Outcome:** dry-run checks whether the tracking table exists using
  a read-only query, treats absence as no applied migrations, and never calls
  the table-creation path.
- **Files:** `apps/anvil-api/src/db/migrate.ts`,
  `apps/anvil-api/src/__tests__/migrate.test.ts`
- **Validation:** `pnpm --dir apps/anvil-api exec vitest run src/__tests__/migrate.test.ts`
- **Finding ID:** `fnd_sig-feat-service-3fe477570c-6526_37c162a902`
- **Risk:** high

## Wave validation

- `pnpm validate:changed`
- `pnpm docs:check`
- `pnpm --dir apps/anvil-api test -- --run`
- `cargo test -p eddacraft-anvil-l4 -p eddacraft-anvil-capsule`
- `cargo test -p eddacraft-anvil --no-fail-fast`

## Landing validation repairs

- `crates/anvil-tui/src/surfaces/audit/mod.rs`: the rustfmt-only correction
  required when workspace metadata made the TUI an affected project landed on
  `main` as `9e4c13a62` and was absorbed by the final rebase.
- `apps/e2e/src/cli/activation.e2e.test.ts`: align activation fixtures with
  canonical `.anvil.yaml`, mutually exclusive MCP selection, and the compact
  single-state contract.
- `apps/e2e/src/smoke/smoke.e2e.test.ts`: request full MCP validation detail
  before asserting full-envelope fields.
- Focused E2E validation: 25/25 passed.
- `pnpm validate:changed`: passed after the affected validation repairs. One
  tracing-capture test transiently missed its INFO event on the preceding run;
  the exact test passed 5/5 in isolation before the clean full-gate rerun.

## Review evidence

- Frozen reviewed implementation diff (before landing reconciliation):
  `f5d1eb3c2ef9b7807db8c79321a658e0d8594a29dd2b45b8425d6d59aa9f0c5f`.
- Council session `council-8ce13643`: converged; three findings fixed, none
  open.
- Independent verification: PASS for CLAWFIX-001..006.
- Fresh validation: docs contract passed including cases I-O; docs gate 11/11;
  API 757/757; L4 plus capsule 239/239; Windows-target strict Clippy clean.
- Non-blocking coverage gaps: Windows containment tests were cross-compiled
  rather than run natively; approval concurrency was verified through mocked
  database tests plus direct review of the PostgreSQL conditional CTE.
