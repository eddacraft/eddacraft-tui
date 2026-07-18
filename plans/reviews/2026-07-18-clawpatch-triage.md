# Clawpatch triage — 2026-07-18 (fix-first + verify)

**Prior scan:** `clawpatch map && clawpatch review` (run `20260718T130816-1a9b30`,
codex / `gpt-5.6-terra` high)
**Status command:** `clawpatch status`
**Findings input:** `plans/audits/2026-07-18-clawpatch-periodic-scan.json`
**Branch:** `test/clawpatch`
**Predecessor:** `plans/reviews/2026-07-02-clawpatch-triage.md`

## Why this run

Entry state after the 2026-07-18 scan: **920 findings / 270 open / 1 open high**.
This pass is a **fix-first triage** of the product-blocking high and a short
queue of high-confidence confirmed-bugs that were cheap to verify and land
alongside it. Remaining open items stay advisory (CIB / APS) unless they re-fire
as highs.

Open count **270 → 264**. Local clawpatch verdicts live in `.clawpatch/`
(gitignored); durable record is this doc plus the exported audit JSON.

## Verdicts recorded (this session)

| Finding | Verdict | Basis |
| ------- | ------- | ----- |
| Refresh-token race can mint a live session after family revocation (high) | **fixed** | Atomic `consumeAndRotateRefreshToken` CTE + `mintRotatedSession` on `/session/refresh` (CIB-141 rotation half) |
| OTP active-code limit is bypassable by concurrent requests (medium) | **fixed** | `insertOtpCodeIfUnderLimit` with `pg_advisory_xact_lock` + conditional insert |
| Write the generated OpenAPI contract atomically (medium) | **fixed** | `export_openapi` temp-file + rename |
| Button defaults to form submission inside a form (medium) | **fixed** | `type="button"` default on native `Button` path |
| Explicit branch fetch leaves `origin/main` stale (medium) | **fixed** | `wt-new.sh` fetches `refs/heads/main:refs/remotes/origin/main` |
| Detected generic token secrets can still be persisted after redaction (medium) | **wont-fix** | Retiring JS kindling surface; same #1826 / 2026-07-02 doctrine |

### 1. Refresh-token race (high) — fixed

Race: winner consumes a refresh token, loser revokes the family, winner then
inserts a replacement token after the revoke sweep — leaving a live post-theft
credential.

Fix: single data-modifying CTE that only consumes and inserts when the family
has no `revoked_at` rows. Route uses `mintRotatedSession` so a failed rotate
revokes the family and returns 401 without minting a licence.

Tests: `auth-session.test.ts`, `session.test.ts`, `queries.test.ts`.

### 2. OTP active-code cap race (medium) — fixed

`/auth/otp/request` previously count-then-inserted. Concurrent requests could
all observe a sub-cap count and overshoot `MAX_ACTIVE_CODES`.

Fix: `insertOtpCodeIfUnderLimit` holds a transaction-scoped advisory lock on the
user id and inserts only when the live active count is still below the cap.

### 3–5. OpenAPI atomic write, Button type default, wt-new fetch — fixed

Small, verified defects with local fixes and (where practical) unit coverage.
`export_openapi` no longer truncates the committed contract in place; dashboard
`Button` no longer submits surrounding forms by default; `wt-new.sh` updates the
remote-tracking ref that `--base origin/main` actually consumes.

### 6. Kindling redaction medium — wont-fix

Same retiring `packages/kindling-integration` surface as the 2026-07-02
disposition. Live path is Rust `KindlingDaemonSink`. Do not re-file under #1826
doctrine.

## Scan summary (post-session)

| Metric | Entry | After session |
| ------ | ----- | ------------- |
| Total findings | 920 | 920 |
| Open | 270 | **264** |
| Open highs | 1 | **0** |
| Fixed (lifetime) | 69 | **74** |

### Open finding mix (264)

- **Severity:** 0 high · 124 medium · 140 low
- **Triage:** 55 confirmed-bug · 93 test-gap · 81 risk · 33 contract-mismatch · 2 docs-gap

## Residual backlog

No open high-severity findings. Remaining open confirmed-bugs are medium/low and
should continue through CIB / APS rather than ad-hoc hotfix unless a later scan
promotes them.

Notable residual classes:

- **scripts/** release and dbcon hygiene (partial files, credential argv, APS
  assess worktree reads)
- **packages/anvil/policy** bundle-manager edge cases
- **crates/** medium contract/test-gap items (no open highs)
- **kindling / edda** retiring surfaces — prefer wont-fix or CIB, not new issues

## Validation evidence (this session)

```text
pnpm --dir apps/anvil-api exec vitest run \
  src/__tests__/auth-session.test.ts \
  src/__tests__/auth-otp.test.ts \
  src/__tests__/session.test.ts \
  src/__tests__/queries.test.ts
# 4 files, 53 tests passed

cargo test -p eddacraft-anvil-dashboard-server --bin export-openapi
# 1 test passed
```

## Docs Closeout

- Added `plans/reviews/2026-07-18-clawpatch-triage.md` (this file).
- Exported `plans/audits/2026-07-18-clawpatch-periodic-scan.json`.
- Noted CIB-141 rotation-half progress in
  `plans/modules/continuous-improvement-backlog.aps.md`.
