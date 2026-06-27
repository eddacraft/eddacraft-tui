# Full Council Review — `fix/cib` wave (PR #2964)

**Date:** 2026-06-27  
**Tier:** full  
**Target:** `origin/main...HEAD` on branch `fix/cib`  
**Scope:** 21 commits, 78 files (~5.7k LOC)  
**PR:** https://github.com/eddacraft/anvil-001/pull/2964

## Roles

| Role | Lens |
| --- | --- |
| general | Correctness, maintainability, test coverage |
| adversarial | Edge cases, failure paths, abuse cases |
| operations | CI, release, deployment, observability, recovery |
| security | Secrets, auth, policy, injection, trust boundaries |
| pragmatic | Proportionality, scope, ship-readiness |

## Local verification evidence

```bash
bash scripts/aps/_test/index-counts.test.sh          # pass
bash scripts/ci/release-sign-artefacts-workflow.test.sh  # pass (not in CI bundle)
bash scripts/dev/wt-cleanup-sweep.test.sh          # pass (not in CI bundle)
node scripts/aps/index-counts.mjs --check          # pass (76/108 reconciled)
```

## Unified verdict

**AMEND** — ship after addressing CRITICAL/MAJOR blockers below. Pragmatic lens:
**SHIP WITH FOLLOW-UPS** once blockers are fixed or explicitly waived with
documented operator procedure.

---

## CRITICAL

### CRITICAL operations: Prerelease CLI tags no longer auto-sign (regression vs `main`)

CIB-044 tightened the `release-sign-artefacts` gate to `!prerelease &&
startsWith(tag, 'v')`. On `main`, prerelease `v0.*` betas still sign via
`!prerelease || startsWith(tag, 'v0.')`. Beta CLI releases may ship unsigned
unless operators manually `workflow_dispatch`.

- `.github/workflows/release-sign-artefacts.yml:30-32` (compare `origin/main:30-32`)
- **Fix:** restore `v0.*` prerelease carve-out **or** document + automate
  `workflow_dispatch` for every beta tag in release runbooks.

### CRITICAL adversarial: `anvil uninstall` bypasses ADR-060 project write gate

`drift migrate`, `drift snapshot`, `baseline`, `hooks`, and `init` call
`ensure_project_write_allowed` under gated `ANVIL_HOME`. `uninstall` removes
`.anvil/`, `.anvilrc`, and git hooks with **no** gate check — a side-by-side
candidate session can wipe production project state while other mutations are
blocked.

- `crates/anvil-cli/src/commands/uninstall.rs:171-196` (no gate)
- Contrast: `crates/anvil-cli/src/commands/drift.rs:529`, `792`
- **Fix:** gate project-scoped removals; add regression test mirroring
  `welcome`/`drift` gated-root behaviour.

### CRITICAL adversarial: `workflow_dispatch` signing binds wrong commit SHA

Signing stamps `commit=${GITHUB_SHA}` from the workflow ref, not the release
tag's target commit. Manual dispatch can sign installers from tag `TAG` while
attributing provenance to default-branch HEAD.

- `.github/workflows/release-sign-artefacts.yml:115-119`
- **Fix:** resolve tag → `target_commitish` before signing, or checkout that
  SHA explicitly.

---

## MAJOR

### MAJOR general + adversarial: Drift migrate write failure not reported as partial

Read/skip paths set `MigrateReport.partial`, but `migrate_one` write failure
aborts via `?` after earlier baselines may already be migrated — mixed workspace
without structured partial summary (CIB-088 intent gap).

- `crates/anvil-cli/src/commands/drift.rs:432`

### MAJOR general + security + adversarial: CIB-080 benign-context breadth risks false negatives

`has_validator_fixture_context` matches loose substrings (`valid`, `url`,
`parse`, `jwt`, …) in a ±2-line window; `is_benign_context` ORs with broad
path heuristics (`examples/`, etc.). Production code near ordinary tokens can
suppress high-entropy secret hits.

- `crates/anvil-checks/src/secret/context.rs:29-55`
- `crates/anvil-checks/src/secret/entropy.rs:163-188`
- **Fix:** tighten to test/fixture paths + exact vectors; add production-path
  regression tests.

### MAJOR operations: Release-sign and wt-cleanup fixture tests not in CI bundle

`release-sign-artefacts-workflow.test.sh` and `wt-cleanup-sweep.test.sh` pass
locally but are not wired into `Run CI script fixtures`
(`.github/workflows/ci.yml:234-250`). Gate regressions can merge silently.

### MAJOR operations + pragmatic: 21-commit monolith — bisect/rollback surface

Wave bundles APS policy, CLI help, checks/AST, drift, uninstall, TUI, CI, and
ops scripts. One revert is all-or-nothing. Acceptable as a deliberate CIB drain
only with green CI and post-merge verification plan.

### MAJOR adversarial: Drift scan cap drops baselines without migrating them

Over 1000 snapshots, mtime heap eviction excludes files; migrate may exit with
`scan_limit_exceeded` after partial work — operators may believe full migration
ran.

- `crates/anvil-cli/src/commands/drift.rs:398-400`, `1174-1234`

### MAJOR pragmatic: CIB-090 cross-references wrong follow-up ID (CIB-105 → CIB-108)

After merge renumber, CIB-090 Done text still points Windows reparse parity to
**CIB-105** (insights nudge). Actual follow-up is **CIB-108**.

- `plans/modules/continuous-improvement-backlog.aps.md:2458`, `2470`
- `plans/reviews/2026-06-26-cib-090-sidecar-hardening.md`

### MAJOR adversarial: Windows global uninstall removes pid file without stopping daemon

`stop_daemon` may delete pid file while process remains — user believes daemon
stopped.

- `crates/anvil-cli/src/commands/uninstall.rs:443-453`

---

## MINOR

| Role | Finding | Location |
| --- | --- | --- |
| operations | Advisory `aps:index:check` drift easy to miss in Docs Lint logs (no `::warning::`) | `scripts/aps/index-counts.mjs:181-184` |
| operations | ADR-053 says structural failures may block; script always exits 0 in `--check` | `053-advisory-aps-index-counts.md:93-94` |
| general | CLIC-010 COMMON FLAGS block is globally hardcoded — may mislead nested commands | `help_layout.rs:39-40` |
| general | `aps:index:check` only runs when docs paths trigger Docs Lint | `ci.yml:146-150` |
| general | CIB-040 actions plan cites missing `tests/help_layout.rs` | `plans/execution/CIB-040.actions.md` |
| security | `usage.salt` / FP list reads lack O_NOFOLLOW parity with append path | `usage.rs:125-154`, `1030-1037` |
| security | `workflow_dispatch` has no tag-shape guard (operator error surface) | `release-sign-artefacts.yml:30-32` |
| security | `WT_BIN` env hijack in wt-cleanup `--apply` (dev-only) | `wt-cleanup-sweep.py:225-254` |
| adversarial | `report-fp --include-snippet` writes plaintext secrets to local NDJSON | `report_fp.rs:115-121` |
| adversarial | Drift listing under scan cap is mtime-gameable | `drift.rs:1174-1230` |
| adversarial | Corrupt NDJSON line fails entire `list_false_positive_reports` | `usage.rs:1040-1053` |

---

## NIT

- Merge commit on feature branch adds review noise (`aa66e78b4`).
- CIB-047 receipt lacked adversarial content; ops + tests are binding evidence.
- CIB-044 post-merge proof deferred to next `eddacraft-tui-v*` tag (expected).
- RS-007/008 AST rules are deliberately incomplete (documented opt-in scope).

---

## Positive outcomes (converged)

- **CIB-025 / ADR-053:** Advisory count model implemented; agent docs aligned;
  CIB-107 split for full-row generation.
- **CIB-090:** Unix sidecar O_NOFOLLOW + dirfd discipline with tests.
- **CIB-101:** `ANVIL_HOME` uninstall scoped to install-root `user/` with symlink
  refusal tests.
- **CIB-028:** Conservative worktree sweep (merge proof, dry-run default, no
  `--force`).
- **CIB-040:** CLIC-010 runtime help + lint; internal IDs stripped from
  user-visible text.
- **CIB-047:** Watch TUI daemon-fallback notice with reconnect clear.
- **CIB-044 (intent):** Library `eddacraft-tui-v*` tags skip no-op signer.

---

## Required before merge

1. ~~Fix prerelease CLI signing regression~~ — **fixed** (`startsWith(v)` gate restores beta signing; library tags still skip).
2. ~~Add `ensure_project_write_allowed` to project-scoped `uninstall` paths + test~~ — **fixed** (`anvil_home.rs` regression test).
3. ~~Fix `workflow_dispatch` commit binding in release signing~~ — **fixed** (`targetCommitish` step).
4. ~~Repoint CIB-090 references from CIB-105 → CIB-108~~ — **fixed**.

Resolved in commit following this review (2026-06-27).

## Recommended follow-ups (non-blocking)

1. Wire `release-sign-artefacts-workflow.test.sh` + `wt-cleanup-sweep.test.sh`
   into CI script fixtures.
2. Tighten CIB-080 benign-context suppressions + production regression tests.
3. Drift migrate write-failure → partial reporting.
4. Post-merge: `pnpm aps:index:check`, CLIC-010 help spot-check, library-tag
   signing skip proof, `wt-cleanup-sweep --dry-run` on real fleet.

---

**Status:** Converged  
**Tier:** full  
**Target:** `origin/main...HEAD` (`fix/cib`)