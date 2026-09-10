# JOURNEY-014 — Reliability closure and regression evidence gate

**Date:** 2026-09-10 (AWST)
**Pinned source:** `9684aa560b4742cf03aaccd4d8f3674827e412bc` (`origin/main` at evidence collection)
**Platform:** Linux x86_64 (deus)
**Claim:** #4570
**Conductor command:** `node scripts/journey/verify.mjs --no-build` (JREL-012 published gate)
**APS lint:** `node scripts/aps/active-lint.mjs` — 140 files checked, all clean
**Identity:** [2026-09-10-journey-014-identity.json](./2026-09-10-journey-014-identity.json)

## Purpose

Establish that the complete setup and daily-use spine works before changing the journey contract (JOURNEY-015 / JSIMP). This record links every JREL item to its merged implementation and records the non-skipping journey verification against the pinned build.

## JREL closure matrix

| Item | Status | Implementation / reconcile | Notes |
| ---- | ------ | -------------------------- | ----- |
| JREL-001 | Merged | PR #4497 (claim #4406) | Lossless MCP upgrade / session continuity |
| JREL-002 | Merged | PR #4416 (reconcile #4427 / #4417) | Client-attributed live session evidence |
| JREL-003 | Merged | PR #4428 (claim #4423) | Save-time driver recovery |
| JREL-004 | Merged | PR #4428 (claim #4424) | One daemon identity through start/recycle |
| JREL-005 | Merged | PR #4509 (claim #4502) at `0c86cb164` | Typed readiness / recovery outcomes |
| JREL-006 | Merged | PR #4512 (claim #4511) at `870bdba79` | Learning across projects / interrupted setup |
| JREL-007 | Merged | PR #4527 (claim #4521) at `e0565cfde` | Guided setup project routing |
| JREL-008 | Merged | PR #4533 (claim #4530) at `76493bf7d` | Welcome terminal restoration |
| JREL-009 | Merged | PR #4556 (claim #4549) at `be59ea474` | Preserve explicit MCP launch choices |
| JREL-010 | Merged | PR #4562 (claim #4553) at `a2734e822` | Admitted workspace identity in MCP |
| JREL-011 | Merged | PR #4566 (claim #4552) at `76294a9e9` | Lifecycle budget / integration exchange |
| JREL-012 | Merged | PR #4523 (claim #4516) at `c3a8763c5` | Fail-closed journey verify gate |
| JREL-013 | Merged | PR #4568 (claim #4554); reconcile #4569 | Scoped pre-write protection evidence |

Module status on the pinned SHA: **JREL Done 13/13**.

## Owner reconciliation (CIB-405 / #4231)

- Issue #4231 (`[Clawpatch] Bound daemon and Git-history reads by one wall-clock budget`) is **CLOSED**; not an open P0/P1 on the reliability spine.
- CIB-405 remains **Proposed** under its existing owner (connection-reuse migration). JREL boundaries already record that CIB-405 owns that residual and JREL-011 does not duplicate it. No unresolved P0/P1 from that surface is labelled complete by this gate.

## Journey verification

Executed at `2026-09-10T00:50:53.438Z` (08:50 AWST) against the pinned SHA on Linux.

| Field | Value |
| ----- | ----- |
| Binary | `target/debug/anvil` (`anvil 0.9.7-beta`) |
| Binary sha256 | `10b38f787b7425c91eaa08534de88a463fe9284aa3d88c4bc1fde692f84c14dc` |
| Platform | linux / x64 |

| Scenario | Result |
| -------- | ------ |
| daemon-identity | pass |
| save-time-driver-recovery | pass |
| mcp-protection-claim | pass |
| mcp-stdio | pass |
| e2e-cli | pass |
| e2e-smoke | pass |
| upgrade-previous-public | not-supplied (`ANVIL_PREVIOUS_PUBLIC_BIN` unset; optional unless `--require-upgrade`) |

First verify attempt failed when the host shell exported `ANVIL_NO_SAVE_TIME_DRIVER=1`, which the e2e harness inherited and disabled driver supervision. That is host pollution, not a product defect. This PR hermetically clears the opt-out in the save-time smoke isolated env and in `cli-runner` unless a test opts in. Re-verify after that change is green.

## Limitations

- Evidence collected on **Linux only** in this gate. macOS/Windows matrix and interactive legs remain for JOURNEY-015 (pinned-main rehearsal); any platform waiver there must be explicit.
- Previous-public-build upgrade leg not required for JOURNEY-014 (optional catalog entry); JOURNEY-015 owns upgrade-from-previous-public when rehearsing a release candidate.
- No publication or version claim is authorised by this gate.

## Decision

**JOURNEY-014 passes.** JOURNEY-015 is unlocked. JSIMP remains blocked until JOURNEY-015 passes.
