<!--
APS Module: Branch Reconciliation
==================================
Recover from divergent `main` and `dev` histories by porting release-critical
fixes from `main` onto `dev`, then cutting over so `dev` becomes the new
canonical lineage.

Reference runbook: docs/runbooks/branch-reconciliation.md

Scopes: BRECON (main)
-->

# Branch Reconciliation

| ID     | Owner | Status      |
| ------ | ----- | ----------- |
| BRECON | aneki | In Progress |

## Purpose

Reconcile divergent `main` and `dev` histories on the `anvil-001` monorepo.
`dev` carries the current structural base (889 commits ahead of `main`) but is
missing CI hardening, auth hardening, Rust CLI review fixes, and release
pipeline changes that landed directly on `main` (66 commits). Port those fixes
onto `dev`, validate as a single integrated branch, land via a normal PR, then
cut `main` over to the reconciled lineage and archive the old `main`.

**Why:** `main` and `dev` diverged over the `0.3.0` Rust rewrite. Several
release-critical and security fixes were merged to `main` without being
back-merged to `dev`. A blind `main -> dev` merge would resurrect deleted
archived paths and regress the newer `dev` topology. The runbook's Option D
(dev-as-base with surgical ports) is the recovery path.

## In Scope

- Soft freeze on `main` and `dev` during reconciliation
- Safety anchor branches (`backup/dev-before-reconcile`,
  `backup/main-before-reconcile`, `reconcile/dev-with-main-fixes`)
- Porting CI/workflow, auth, Rust CLI, TypeScript, release, docs, and lockfile
  changes from `main` onto the reconciliation branch
- Full validation pass on the reconciled branch
- Integrated PR into `dev`
- Branch cutover (`main` becomes the reconciled lineage, old `main` archived)

## Out of Scope

- Ruleset tightening (deferred — soft-freeze approach relies on social
  coordination, not GitHub rulesets)
- Dependabot schedule changes (weekly cadence won't fire mid-freeze)
- Process overhaul to prevent recurrence (tracked as a follow-up, see Risks)
- Any feature work unrelated to reconciliation

## Interfaces

**Depends on:**

- `docs/runbooks/branch-reconciliation.md` — source of truth for phase detail
- Zero open PRs on `dev` and `main` before Phase 1
- No in-flight Dependabot PRs (weekly cadence confirmed)

**Exposes:**

- A reconciled `dev` that preserves both the newer structural base and the
  release-critical fixes that only live on `main`
- An archived `main-archive` branch for historical reference

## Constraints

- **No blind merges.** Phase ports must be surgical (`cherry-pick -n` then
  inspect before commit, or manual logic porting)
- **No hand-merged lockfiles.** `pnpm-lock.yaml` and `Cargo.lock` must be
  regenerated from final manifests
- **No resurrecting archived paths.** Current `dev` structure wins by default
- **No force-push to `main` or `dev`** during the freeze window
- **One integrated PR** into `dev` — cross-cutting interactions (workflows,
  manifests, release packaging, Rust CLI) need validation as a whole

## Ready Checklist

Change status to **Ready** when:

- [x] Runbook exists (`docs/runbooks/branch-reconciliation.md`)
- [x] Zero open PRs on both branches
- [x] No in-flight Dependabot work
- [x] Approach agreed (Option D — dev-as-base with surgical ports)
- [x] Soft freeze approach agreed (no ruleset tightening)

---

## Phase 0 — Freeze

### BRECON-001: Announce freeze and create marker

- **Status:** Complete
- **Intent:** Signal to humans and agents that `main` and `dev` are frozen for
  reconciliation so no merges, rebases, or force-pushes happen during the
  window
- **Expected Outcome:** `RECONCILIATION-IN-PROGRESS.md` marker file at repo
  root, AGENTS.md has a freeze notice block, BRECON module registered in
  `plans/index.aps.md`
- **Validation:** Marker file exists; `grep -n "freeze" AGENTS.md` returns the
  notice; `plans/index.aps.md` lists BRECON under a recovery section
- **Files:** `RECONCILIATION-IN-PROGRESS.md`, `AGENTS.md`,
  `plans/index.aps.md`, `plans/modules/branch-reconciliation.aps.md`,
  `.claude/rules/aps-project.md`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

## Phase 1 — Safety Anchors

### BRECON-002: Create safety anchor branches

- **Status:** Complete
- **Intent:** Create local and remote backup branches so reconciliation work
  is reversible
- **Expected Outcome:** `backup/dev-before-reconcile`,
  `backup/main-before-reconcile`, and `reconcile/dev-with-main-fixes` exist
  locally and on origin
- **Validation:** `git branch -a | grep -E 'backup/|reconcile/'` lists all
  three; `git ls-remote origin 'backup/*' 'reconcile/*'` confirms remote
- **Result:** `backup/dev-before-reconcile` @ `e32e78d0`,
  `backup/main-before-reconcile` @ `4bf0c620`,
  `reconcile/dev-with-main-fixes` @ `e32e78d0`. All three pushed to origin.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** BRECON-001

---

## Phase 2 — Working Checklist

### BRECON-003: Prepare reconciliation working checklist

- **Status:** Complete
- **Intent:** Translate the nine runbook buckets into a trackable todo list
  scoped to the reconciliation branch
- **Expected Outcome:** Internal checklist exists covering CI/workflow, auth,
  Rust CLI, TypeScript, release, lockfiles, docs, validation, cutover
- **Validation:** Checklist covers all nine runbook buckets
- **Result:** Session task list tracks BRECON-004 through BRECON-014,
  one-to-one with runbook Phases 3–13.
- **Confidence:** high
- **Priority:** High
- **Dependencies:** BRECON-002

---

## Phase 3 — CI & Security

### BRECON-004: Port CI and security fixes from `main`

- **Status:** Complete
- **Intent:** Apply CI hardening and action pinning from commits
  `f5aad7d6`, `2af495d2`, `eb7fc881`, `b7b2cde6`, `059aaa6d` onto the
  reconciliation branch
- **Expected Outcome:** Relevant workflow hardening is present on
  `reconcile/dev-with-main-fixes` without regressing newer `dev` workflow
  topology
- **Validation:** `pnpm run lint` still green; workflow YAML parses;
  `gh workflow list` shows the same workflows
- **Files:** `.github/workflows/*.yml`,
  `.github/actions/anvil-check/action.yml`
- **Result:** Four commits landed on `reconcile/dev-with-main-fixes`:
  `4933245a` (SHA-pin actions/labeler@v6 from f5aad7d6),
  `4f1b8fcd` (review-feedback tweaks from 2af495d2 minus rust.yml hunk,
  which would regress TFIX-003/004 OPA CI work), `38001285` (gate
  `parse_kb_value` on linux + allow unused_mut), `a4f9f137` (cross-platform
  test fixes from b7b2cde6). `059aaa6d` verified fully absorbed by dev's
  parallel CI evolution — 8 of 10 files identical to main, 2 strictly
  ahead via Dependabot bumps; no port needed. `eb7fc881` content already
  present on dev.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** BRECON-002

---

## Phase 4 — Auth Hardening

### BRECON-005: Port auth hardening from `main`

- **Status:** Complete
- **Intent:** Preserve the auth behaviour landed on `main` via
  `b2e9f6e9`, `9a37a9f6`, `140a3195`
- **Expected Outcome:** Auth routes and admin endpoints match intended
  hardening; `dev`'s `ANVIL_DEV=1` behaviour is kept
- **Validation:** `pnpm vitest run apps/anvil-api/src/routes/auth` passes;
  admin and waitlist flows still reachable
- **Files:** `apps/anvil-api/src/routes/auth*.ts`,
  `apps/anvil-api/src/routes/admin.ts`, `apps/anvil-api/src/lib/*`
- **Result:** Ported as `9e0951d6` on `reconcile/dev-with-main-fixes`.
  Nine files touched: JWT `iss`/`aud` claims added in `licence.ts`;
  `TOKEN_PEPPER` fail-fast at module load in `token.ts`;
  refresh-token cleanup (7-day revoked retention) in `cron.ts`;
  atomic OTP consume via `UPDATE ... RETURNING` in `auth-otp.ts`;
  `waitUntil` wrapper for Resend audience update in `waitlist.ts`;
  direct `contacts.remove({id: email})` in `audience.ts`;
  `TOKEN_PEPPER` env var restored in `infra/src/vercel.ts`;
  test assertions added in `licence.test.ts` and `vercel.test.ts`.
  All anvil-api (63/63) and infra (3/3) tests pass.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** BRECON-002

---

## Phase 5 — Rust CLI Review Fixes

### BRECON-006: Port Rust CLI review fixes from `main`

- **Status:** Complete
- **Intent:** Apply review fixes from `0623146c`, `f5766723`, `21148747`,
  `caa2687c`, `319cd9a7`, `2c67c70c`, `abe0943b` using current `dev` files as
  the base
- **Expected Outcome:** Rust CLI preserves still-relevant `main` auth
  evaluation and review-fix logic while keeping `dev`'s newer structure
- **Validation:** `cargo build --workspace`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace` all green
- **Files:** `crates/anvil-cli/src/main.rs`,
  `crates/anvil-cli/src/commands/*`, `crates/anvil-cli/src/auth/*`
- **Result:** No port needed. All seven review-fix commits already
  absorbed by dev's parallel Rust CLI evolution: pure `evaluate_auth` +
  `check_auth` wrapper, `ANVIL_DEV=1` bypass, `parse_kb_value` +
  `unused_mut` gate, cross-platform test fixes. Verified with
  `cargo build --workspace` (clean), `cargo clippy --workspace
  --all-targets -- -D warnings` (clean), `cargo test --workspace`
  (506/506 pass).
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** BRECON-002

---

## Phase 6 — TypeScript & Workspace Compatibility

### BRECON-007: Port TypeScript compatibility fixes from `main`

- **Status:** Complete
- **Intent:** Review and port only still-relevant changes from `55e440c5`,
  `06d03d08`, `73b5a6e1`, `fbcde70b`, `e1c38d5c` without regressing newer
  `dev` package structure
- **Expected Outcome:** `pnpm run typecheck` green on the reconciliation
  branch; no lost compatibility flags
- **Validation:** `pnpm run typecheck`; `pnpm exec nx sync --yes` reports no
  drift
- **Files:** `package.json`, `pnpm-workspace.yaml`, `tsconfig.base.json`, app
  and package manifests, workspace tooling config
- **Result:** No port needed. All five TS compatibility commits already
  absorbed. Dev's `nodenext` module resolution sidesteps main's TS 6
  `ignoreDeprecations` workarounds, and dev carries newer dep versions
  across the board. `pnpm run typecheck` clean on reconcile branch.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** BRECON-002

---

## Phase 7 — Release & Distribution

### BRECON-008: Port release pipeline changes from `main`

- **Status:** Complete
- **Intent:** Manually port release-critical behaviour from `e2a4e1c5` and
  `45e38850` because `0.3.0` is the first Rust-native distribution release
- **Expected Outcome:** Release workflow still cuts the Rust binary
  correctly; installer scripts reflect current distribution layout
- **Validation:** `gh workflow view release.yml`; manual review against
  current `dev` state; no regressions in DIST module expectations
- **Files:** `.github/workflows/release.yml`, installer scripts, distribution
  config
- **Result:** Ported as `5838deb8` on `reconcile/dev-with-main-fixes`.
  Two real regressions fixed: `.github/workflows/release.yml` jq nested
  single-quoting at lines 92 and 270 (inside GITHUB_OUTPUT echo steps);
  `install.sh` portable `mktemp` fallback for BSD/macOS using
  `-t anvil-installer` then `${TMPDIR:-/tmp}/anvil-installer.XXXXXX`
  with trap cleanup. `e2a4e1c5`'s remaining content was already absorbed.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** BRECON-002

---

## Phase 8 — Lockfiles

### BRECON-009: Regenerate lockfiles from final manifests

- **Status:** Complete
- **Intent:** Reconcile dependency state by regenerating rather than
  hand-merging lockfiles
- **Expected Outcome:** `pnpm-lock.yaml` and `Cargo.lock` regenerated after
  manifest ports; no hand-merged markers; workspace installs clean
- **Validation:** `pnpm install` completes; `cargo check` completes;
  lockfiles contain no conflict markers
- **Files:** `pnpm-lock.yaml`, `Cargo.lock`
- **Result:** No regeneration needed — manifests were not touched by
  ports 005–008. `pnpm install --frozen-lockfile` clean;
  `cargo check --workspace` clean; no conflict markers in either
  lockfile. Lockfile freshness verified directly against final state.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** BRECON-004, BRECON-005, BRECON-006, BRECON-007,
  BRECON-008

---

## Phase 9 — Release-Critical Docs

### BRECON-010: Port release-critical docs from `main`

- **Status:** Complete
- **Intent:** Mine `f754401c`, `9feb0489`, `48adec2b` for Rust release docs,
  install/upgrade docs, auth docs, and `0.3.0` quickstart/product docs
- **Expected Outcome:** Release-critical docs are correct on the
  reconciliation branch without pulling pure formatting churn
- **Validation:** Rendered Docusaurus build succeeds
  (`pnpm --filter @eddacraft/docs-site run build`); broken-link check passes
- **Files:** `docs/public/anvil/releases/*`, `docs/public/anvil/quickstart.md`,
  install/upgrade/auth docs
- **Result:** Ported as `d99f93ea` on `reconcile/dev-with-main-fixes`.
  Five files updated after verifying each claim against the Rust
  v0.3.x code:
  `mcp.md` (Node.js 18+/npm 7+ note),
  `rust-rewrite.md` (exit codes 3/4 are actively emitted by
  `EXIT_AUTH_REQUIRED`/`EXIT_CONFIG_ERROR`, not reserved; Step 4
  Authenticate restored),
  `troubleshooting.md` (`.anvil/suppressions.json` schema matches
  `SuppressionStore` in `export.rs`),
  `tutorials/ci.md` (pre-commit hook uses `anvil check --changed
  --staged` because `anvil gate` scans unstaged+staged; exit codes
  3/4 corrected),
  `tutorials/architecture.md` (restored `schema_version: '0.1.0'` /
  `template` / layers-as-mapping / `patterns` / `depends_on` format
  that the Rust `ArchDefinition` parser accepts).
  Three files deliberately skipped (dev correct or ahead):
  `beta-testing-guide.md` (drift + architecture subcommands exist),
  `beta/quickstart.md` (policy explain/list + watch --source exist),
  `release-runbook.md` (dev has strictly newer Option A/B flow).
  `pnpm --filter @eddacraft/docs-site run build` clean.
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** BRECON-004, BRECON-005, BRECON-008

---

## Phase 10 — Full Validation

### BRECON-011: Run full validation pass

- **Status:** Complete
- **Intent:** Run the reconciliation-branch validation matrix from the
  runbook so the branch is known-good before PR
- **Expected Outcome:** Lint, typecheck, unit tests, cargo fmt, clippy, and
  cargo test all pass on `reconcile/dev-with-main-fixes`
- **Validation:** `pnpm run lint && pnpm run typecheck && pnpm run test
  -- --run && cargo fmt --all --check && cargo clippy --workspace
  --all-targets -- -D warnings && cargo test --workspace`
- **Result:** All six gates green on `reconcile/dev-with-main-fixes` at
  `cd6a6067`. `pnpm run lint` clean (0 errors, 4 pre-existing website
  warnings). `pnpm run typecheck` clean across 21 projects.
  `pnpm test --run` 3,497 TS tests + 85 MCP + 159 source tests all
  pass. `cargo fmt --all --check` clean. `cargo clippy --workspace
  --all-targets -- -D warnings` clean. `cargo test --workspace`
  1,573/1,573 pass.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** BRECON-004 through BRECON-010

---

## Phase 11 — Pre-Cutover Verification

### BRECON-012: Answer pre-cutover checklist

- **Status:** Complete
- **Intent:** Explicitly verify the six runbook questions before changing
  branch roles
- **Expected Outcome:** Documented yes/no for: auth hardening preserved,
  CI hardening preserved, release changes preserved, Rust CLI fixes
  preserved, lockfiles regenerated from final manifests, no archived paths
  resurrected
- **Validation:** All six answers are "yes" and recorded in the
  reconciliation PR description
- **Result:** All six questions answered yes.
  1. **Auth hardening preserved:** yes. BRECON-005 commit `5e63a3da`
     ports JWT iss/aud claims, TOKEN_PEPPER fail-fast, refresh-token
     cleanup, atomic OTP consume, waitUntil for Resend audience
     update, direct `contacts.remove({id: email})`, plus
     `TOKEN_PEPPER` env var in `infra/src/vercel.ts`.
  2. **CI hardening preserved:** yes. BRECON-004 commits `deb5d97e`
     (SHA-pinned `actions/labeler@v6`), `0b4be31b` (review-feedback
     tweaks), `61073098` (gate `parse_kb_value`, allow `unused_mut`),
     `b6c76e76` (cross-platform test fixes). `f5aad7d6`/`2af495d2`
     content ported; `059aaa6d`/`eb7fc881` verified already absorbed
     by dev.
  3. **Release pipeline changes preserved:** yes. BRECON-008 commit
     `2a6903ab` ports `.github/workflows/release.yml` jq nested
     quoting fix at lines 92 and 270 and `install.sh` portable
     `mktemp` fallback for BSD/macOS.
  4. **Rust CLI review fixes preserved:** yes. BRECON-006 verified
     all seven review-fix commits fully absorbed by dev's parallel
     Rust CLI evolution. `cargo clippy --workspace --all-targets
     -- -D warnings` and `cargo test --workspace` (1,573/1,573)
     clean on reconcile.
  5. **Lockfiles regenerated from final manifests:** yes. BRECON-009
     verified no manifest drift from ports — `git diff dev..
     reconcile/dev-with-main-fixes -- pnpm-lock.yaml Cargo.lock`
     returns nothing. `pnpm install --frozen-lockfile` clean;
     `cargo check --workspace` clean.
  6. **No archived paths resurrected:** yes. `git diff dev..
     reconcile/dev-with-main-fixes --name-status | grep "^A" | wc -l`
     returns 0 — reconcile introduces only modifications, never
     additions, so none of `main`'s deleted archived directories
     came back.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** BRECON-011

---

## Phase 12 — PR into `dev`

### BRECON-013: Land reconciliation PR into `dev`

- **Status:** Complete
- **Intent:** Open one integrated PR so cross-cutting interactions validate
  as a whole rather than in fragments
- **Expected Outcome:** PR titled `fix(branching): reconcile main-only
  release fixes onto dev` merges into `dev`; `dev` is revalidated from the
  merge commit
- **Validation:** PR checks green; merged; post-merge `pnpm run
  lint/typecheck/test` green on `dev`
- **Files:** PR description references runbook, summarises ported commits,
  notes lockfile regeneration
- **Result:** PR #823 (`fix(branching): reconcile main-only release fixes
  onto dev`) opened with 7 reconcile commits. All CI checks green
  (lint, typecheck, unit tests, build, clippy, format, check, Pulumi
  preview, SAST, secret scan, dependency audit, license compliance,
  CodeQL, security summary, Vercel previews). Merged at
  2026-04-09T06:40:44Z via admin merge. Post-merge `pnpm run
  lint/typecheck/test` all green on `dev` HEAD.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** BRECON-012

---

## Phase 13 — Cutover

### BRECON-014: Cut `main` over to reconciled lineage

- **Intent:** Promote the reconciled line and archive the old `main` so the
  history story is coherent going forward
- **Expected Outcome:** `main-archive` exists at the old `main` tip; `main`
  points at the reconciled `dev` commit; branch protection and
  default-branch settings restored; freeze lifted
- **Validation:** `git log origin/main ^origin/main-archive` is empty;
  `gh api repos/EddaCraft/anvil-001` confirms default branch; freeze marker
  removed; AGENTS.md freeze notice removed
- **Files:** `RECONCILIATION-IN-PROGRESS.md` (delete), `AGENTS.md`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** BRECON-013

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Unseen `main`-only behaviour missed during porting | Medium | High | Drive from the commit SHAs listed in the runbook; run full validation; pre-cutover checklist |
| Lockfile drift regenerates unexpected versions | Medium | Medium | Regenerate after all manifests are final; inspect the diff before committing |
| Rust CLI regression during review-fix ports | Medium | High | Use `dev` files as base, port surgically, run `cargo test --workspace` after each phase |
| Freeze broken by concurrent merge | Low | High | Marker file + AGENTS.md notice + direct coordination with the only active agent |
| Dependabot fires during window | Low | Medium | Weekly schedule confirmed; no action needed unless window extends across a Monday |
| Cutover breaks external integrations referencing `main` | Low | Medium | `main-archive` preserves old SHA; communicate cutover before flipping default branch |
| Process recurrence (same divergence happens again) | High | Medium | Follow-up: branch policy + required back-merge workflow (tracked separately post-cutover) |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Freeze | 1 | Complete |
| 1 — Safety Anchors | 1 | Complete |
| 2 — Working Checklist | 1 | Complete |
| 3 — CI & Security | 1 | Complete |
| 4 — Auth Hardening | 1 | Complete |
| 5 — Rust CLI Review Fixes | 1 | Complete |
| 6 — TypeScript Compatibility | 1 | Complete |
| 7 — Release & Distribution | 1 | Complete |
| 8 — Lockfiles | 1 | Complete |
| 9 — Release-Critical Docs | 1 | Complete |
| 10 — Full Validation | 1 | Complete |
| 11 — Pre-Cutover Verification | 1 | Complete |
| 12 — PR into `dev` | 1 | Complete |
| 13 — Cutover | 1 | Ready |
| **Total** | **14** | **13/14 done** |
