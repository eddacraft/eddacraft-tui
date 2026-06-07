# Branch Reconciliation Runbook

| Type    | Authority     | Owner   | Status | Freshness                                                                              |
| ------- | ------------- | ------- | ------ | -------------------------------------------------------------------------------------- |
| Runbook | Authoritative | OPMODEL | Live   | Last reviewed 2026-05-24 against the main-first cutover and `.github/workflows/ci.yml` |

| Upstream                            | Downstream                         |
| ----------------------------------- | ---------------------------------- |
| `.github/workflows/ci.yml`, ADR-001 | release council, on-call operators |

## Purpose

Recover from a long-lived `main`/`dev` divergence in a single reviewed
operation, then put a process rule in place to prevent recurrence. This is a
one-time recovery runbook, not a recurring operational task.

## When to use

- Right now (one-shot, before the `0.3.0` release goes out).
- Any future situation where `main` and `dev` have drifted enough that a trivial
  back-merge is no longer possible and the divergence affects shipped release
  behaviour.

## Context

This repository has two long-lived branches that diverged in process and in
content:

- `main` became the release integration line for the v0.3.x release train.
- `dev` continued as the active development line while release work was being
  promoted in chunks.

The result is not a normal "git got confused" situation. It is a process gap
that produced a state problem:

1. The release process did not close the loop.
2. `dev` continued to evolve structurally while `main` was frozen as the
   promotion target.
3. Both branches edited many of the same files in legitimate but different ways.

The next release is the first Rust-native `0.3.0` release. The goal is to
preserve work, recover to a single canonical line, and avoid repeating this
state.

## Problem Summary

### 1. Release-time fixes accumulated only on `main`

The v0.3.x release was assembled through multiple release branches cut from
`main`, with content composed from `dev`. Each release branch went through PR
review, council review, CI, and follow-up fixups. Those fixups landed on `main`,
but there was no runbook step to merge them back into `dev`.

That left `main` with a bounded but important tail of release-only work:

- CI hardening
- audit and security remediations
- TypeScript 6 compatibility fixes
- Rust review fixes
- release workflow fixes
- auth hardening and docs corrections

### 2. `dev` kept moving as the real product line

While `main` acted as the release target, `dev` continued to change the actual
shape of the repository:

- Node CLI archived to `archive/anvil-cli-node/`
- old packages removed or restructured
- `.claude/` and workflow setup cleaned up
- WELCOME and DOCSAUTH work continued
- manifests, lockfiles, plans, docs, and Rust sources kept evolving

This means `dev` reflects the current intended repository structure and `main`
does not.

### 3. Same-file divergence is real

Many files were changed independently on both branches. These are not fake
conflicts or whitespace-only conflicts. They include manifests, lockfiles,
workflow YAML, plans, docs, and Rust source.

Examples:

- `crates/anvil-cli/src/main.rs` `dev` added `ANVIL_DEV=1` bypass and related
  wiring. `main` changed `evaluate_auth` semantics.
- `pnpm-lock.yaml` `dev` moved package versions forward. `main` contains
  audit-driven dependency remediations.
- `Cargo.toml` and `Cargo.lock` both lines changed dependencies independently.

## Audit Findings

The branch audit produced these high-level facts:

- `origin/HEAD` points to `dev`.
- `main...dev` unique commit counts are `66` on `main` and `861` on `dev`.
- Symmetric three-dot diff (`git diff --stat origin/main...origin/dev`): `1107`
  files, `108051` insertions, `11575` deletions. This is the total amount of
  work done on either side since the merge base.
- Two-dot direct tree diff (`git diff --stat origin/main..origin/dev`): `733`
  files, `19138` insertions, `78537` deletions. This is the actual
  reconciliation surface — the file content that differs between current `main`
  and current `dev` right now. This is what a real merge would have to resolve.
- File-level breakdown of the two-dot diff: `47` files added on `dev` only,
  `491` files exist on `main` but not on `dev` (mostly archived/cleaned-up
  paths), `193` files modified on both branches with non-overlapping changes
  that require manual 3-way merging.

The 491 main-only files are concentrated in:

- `apps/anvil-cli/` (341): the old Node CLI; `dev` archived this to
  `archive/anvil-cli-node/` and `main` never received the cleanup
- `packages/platform/` (28): old package; removed on `dev`
- `crates/eddacraft-tui/` (27): old crate; removed on `dev`
- `plans/execution/` (22), old `plans/modules/` (15): plan reorg on `dev`
- `.claude/hooks/` (9), `.claude/agent-bus/` (9), `.claude/settings.json`,
  `.claude/mcp.json`: legacy Claude config dev removed
- 5 obsolete workflows (`claude.yml`, `temper.yml`, `publish.yml`,
  `claude-address-pr-reviews.yml`, `import-state.yml`)
- Root-level junk (`.cursorrules`, `WARP.md`, `CLAUDE-README.md`, `.nxignore`,
  `.prettierrc`)

The 47 dev-only additions are concentrated in:

- `crates/anvil-tui/src/surfaces/onboarding/` and `tutorial/` (16): WELCOME
  module
- `crates/anvil-cli/src/services/` (2): WELCOME first-run detection
- `crates/anvil-checks/src/filter.rs` (1): WELCOME scan filter
- `apps/docs-site/api/auth/` and `middleware.ts` (4): DOCSAUTH OAuth
- `apps/anvil-api/src/routes/auth-github.ts` and a db migration (2): DOCSAUTH
  backend
- New plans, ADRs, vision docs, package READMEs

Top-level areas changed across the branch delta:

- `.claude/`: 63
- `.github/`: 15
- `Cargo.lock`: 1
- `Cargo.toml`: 1
- `apps/`: 53
- `archive/`: 373
- `crates/`: 220
- `docs/`: 90
- `infra/`: 8
- `package.json`: 1
- `packages/`: 115
- `plans/`: 108
- `pnpm-lock.yaml`: 1

Deleted-path counts also show meaningful structural cleanup, especially in
`packages/`, `.claude/`, and old root files. That is strong evidence that `dev`
is the structural truth of the repository and that a tree-level merge would
spend substantial effort reconciling obsolete topology.

## Alternatives Considered

Four approaches were evaluated before settling on the recommendation. They are
documented here so future-us understands why this one was chosen.

### A. Blind `main -> dev` merge

Run `git merge origin/main` on `dev`, resolve conflicts by hand in one pass.

Outcome of investigation: produced `129` conflicts across `156` files in a
single merge attempt, including `86` "added on both" (`AA`) files where both
branches independently created the same path with different content, `39`
"modified on both" (`UU`) files needing real 3-way merging, and `4` "deleted on
dev, modified on main" (`DU`) files. No `-X ours` or `-X theirs` flag resolves
this safely — both options silently lose real work. Rejected.

### B. Themed PRs from dev into current `main`

Open `~8` themed PRs (cleanup deletions, WELCOME, DOCSAUTH, plans, polish, sync
of evolved files) targeting `main` directly, each with its own council review.

Outcome of investigation: same conflict surface as Option A, just spread across
multiple PRs instead of one. The mechanical deletion PRs (cleanups) are easy,
but the "sync evolved files" PR still has to do 193-file 3-way merge work.
Worse, it operates on the live `main` so any in-flight problem risks production.
Rejected on risk grounds.

### C. Branch `main-next` from current `main`, PR dev's content into it

Same as B but landing into a parallel `main-next` branch instead of live `main`,
then atomically swap branch names at the end. Adds a safety net (rollback target
preserved) but the conflict surface and review burden are identical to B.
Rejected as offering no real improvement over B.

### D. Branch `main-next` from `dev`, cherry-pick `main`-only fixes onto it

Use `dev`'s tree as the base (it is structurally newer and cleaner), then
cherry-pick the `~48` `main`-only hardening fix commits onto a branch off `dev`.
When complete, that branch becomes the new `main`.

Outcome of investigation: structurally cleanest because the conflict surface
collapses from `~193` files to whatever the `~48` cherry-picks touch (typically
lockfiles, workflows, auth files, Rust CLI files). However, every piece of work
currently on `dev` (cleanups, WELCOME, DOCSAUTH, plans, post-release polish)
becomes part of the new `main` _without ever passing through a fresh
PR-to-`main` council review_. See "Council Review Gap" below for the rationale
for accepting this trade-off.

This is the chosen approach.

## Recommendation

Use `dev` as the canonical base, salvage the required `main`-only work onto it,
then reset branch roles.

This is preferable to a full `main`/`dev` reconciliation merge because:

1. `dev` contains the repository shape we want to keep.
2. `main` has a bounded release-line tail rather than the dominant line of
   product development.
3. Most important `main`-only work is hardening and review-fix work that can be
   ported onto the current `dev` code.
4. A full merge would force mechanical conflict resolution across archived,
   deleted, and moved paths that are no longer the desired structure.

The objective is not to preserve every `main` commit. The objective is to
preserve every required behaviour and fix that exists only on `main`.

## Council Review Gap (Accepted Trade-off)

Under this approach, `dev` content that becomes the new `main` (cleanup work,
WELCOME, DOCSAUTH, plans, post-release polish) does **not** receive a fresh
council review on a "PR to `main`". Only the reconciliation PR carrying the
`main`-only hardening fixes goes through council review during cutover.

This is a known gap. We accept it for the following reasons:

1. **Each `dev` item already had council review on its own PR into `dev`.**
   WELCOME, DOCSAUTH, and the post-release polish landed via standard PR review.
   The "promotion to `main`" review step would be re-reviewing already-reviewed
   work.
2. **Re-running PR-to-`main` council review on every existing `dev` item
   requires reproducing the conflict surface we are trying to avoid.** It is the
   same trade-off Options B and C made, with the same cost.
3. **The reconciliation PR itself carries the cross-cutting risk** (auth,
   workflows, manifests, release packaging, Rust CLI behaviour). That PR gets
   full council review during cutover and is where production-affecting
   regressions would actually surface.
4. **The back-merge process fix (see "Follow-up Process Fix")** is what
   permanently closes this gap. From the next release onwards, every `main`
   change gets back-merged into `dev` immediately, so the
   "items-staying-put-skip-council-on-promotion" problem never recurs.

If a stakeholder objects to this trade-off, the alternative is to fall back to
Option B or C and accept the additional weeks of merge work. The decision should
be made before Phase 1 of this runbook starts.

## Strategy

1. Freeze both branches briefly.
2. Create a reconciliation branch from `dev`.
3. Classify `main`-only commits into:
   - `pick`
   - `manual-port`
   - `skip`
4. Apply required fixes onto `dev`.
5. Regenerate lockfiles from final manifests.
6. Validate the reconciled line.
7. Open and merge a PR from the reconciliation branch into `dev`.
8. Archive old `main`.
9. Promote the reconciled `dev` line to new `main`.
10. Create a fresh `dev` from the new `main`.

## Branch Policy After Recovery

After cutover, the branch model should be:

- `main`: canonical release branch
- `dev`: fresh development branch cut from `main`

The process change that prevents recurrence is simple:

Any fix that lands on `main` during release stabilisation must be merged or
ported back to `dev` before the release is considered complete.

## Commit Disposition

The `main`-only commit set falls into three classes.

### `pick`

These can usually be cherry-picked or replayed with little or no modification.

- `140a3195` `fix(test): add missing mock secret and JWT claims in tests`
- `45e38850` `fix(dist): jq quoting, mktemp portability, prettier formatting`
- `eb7fc881`
  `fix(ci): gate parse_kb_value test for linux-only, allow unused_mut`
- `55e440c5` `fix: add ignoreDeprecations to CJS packages for TS 6 compat`
- `fbcde70b` `fix: add ignoreDeprecations for TS 6.0 baseUrl deprecation`
- `e1c38d5c` `fix(workspace): add packages/libs/* to pnpm workspace`

These still need review after application, but they are the best candidates for
direct cherry-pick.

### `manual-port`

These are required, but direct cherry-pick is risky because they overlap with
current `dev` structure or with lockfiles/manifests/workflows.

- `c8774a40` `fix(deps): resolve all 37 audit vulnerabilities`
- `b2e9f6e9`
  `fix(auth): harden token pepper, add JWT claims, cleanup expired refresh tokens`
- `9a37a9f6` `fix(auth): address review feedback on auth flows`
- `f5aad7d6`
  `fix(ci): SHA-pin all actions, replace curl-pipe-shell, fix script injection`
- `2af495d2` `fix(ci): address review feedback on workflows and cross-compile`
- `b7b2cde6`
  `fix(ci): resolve cross-platform test failures on macOS and Windows`
- `059aaa6d` `ci: update CI/CD workflows for v0.3.x`
- `06d03d08` `fix: upgrade remaining tools to TypeScript ~6.0.2`
- `73b5a6e1` `fix: remove baseUrl, use relative paths in tsconfig.base`
- `e2a4e1c5`
  `feat(dist): distribution pipeline — release workflow, cargo-dist, installers`

Rust review-fix commits that should be mined by file and behaviour, not applied
blindly:

- `0623146c`
  `fix(cli): terminal teardown guard, doctor probe safety, device flow newline`
- `f5766723`
  `fix(cli): address review feedback on error handling and consistency`
- `21148747`
  `fix(cli): align ignore dirs, use workspace_root in audit/status, fix check metadata`
- `caa2687c` `fix(cli): address review feedback and cargo check warnings`
- `319cd9a7` `fix(cli): revert IPv6 host_str change, fix clippy similar_names`
- `2c67c70c` `fix(cli): resolve clippy warnings in audit and status commands`
- `abe0943b` `fix(cli): make evaluate_auth pure, fix credential path docs`

### `skip`

These should not be applied directly because `dev` already supersedes them or
they are primarily release-branch bookkeeping/style churn.

- broad Rust feature-foundation commits already present on `dev`
- root/archive/process reshaping commits such as `7dc6f578` and `4bf0c620`
- style-only formatting commits
- release bookkeeping/docs state updates that `dev` later rewrote more
  accurately

## Working Rules During Reconciliation

1. `dev` owns repository structure.
2. Preserve behaviours from `main`, not necessarily the exact commits.
3. Never reintroduce files or directories that `dev` intentionally archived or
   removed unless they are still required.
4. Do not text-merge lockfiles.
5. Merge manifests first, regenerate lockfiles second.
6. For same-file conflicts in hot paths, port intent manually.

## High-Risk Files

The following should be treated as deliberate merge surfaces:

- `Cargo.toml`
- `Cargo.lock`
- `pnpm-lock.yaml`
- `package.json`
- app-level `package.json` files
- `.github/workflows/*.yml`
- `.github/actions/anvil-check/action.yml`
- `crates/anvil-cli/src/main.rs`
- `crates/anvil-cli/src/commands/*`
- `plans/**`
- `docs/public/**`
- release and install docs

## Day-of Runbook

### Phase 0: Freeze

1. Pause merges to `main` and `dev`.
2. Announce that only reconciliation work lands during the window.
3. Ask other contributors not to rename or force-push branches during the
   cutover.

### Phase 1: Create safety anchors

```bash
git fetch origin --prune
git checkout dev
git pull --ff-only origin dev
git branch backup/dev-before-reconcile
git checkout main
git pull --ff-only origin main
git branch backup/main-before-reconcile
git checkout dev
git checkout -b reconcile/dev-with-main-fixes
```

Optional remote safety anchors:

```bash
git push origin backup/dev-before-reconcile backup/main-before-reconcile
git push -u origin reconcile/dev-with-main-fixes
```

### Phase 2: Prepare the working checklist

Track work in these buckets:

1. CI and workflow hardening
2. auth and API hardening
3. Rust CLI review fixes
4. TypeScript and workspace compatibility
5. release and distribution pipeline
6. dependency manifests and lockfiles
7. release-critical docs
8. full validation
9. branch cutover

### Phase 3: Apply CI and security fixes first

Port or cherry-pick these in this order:

1. `f5aad7d6`
2. `2af495d2`
3. `eb7fc881`
4. `b7b2cde6`
5. `059aaa6d`

Focus files:

- `.github/workflows/*.yml`
- `.github/actions/anvil-check/action.yml`

Preferred approach:

```bash
git cherry-pick -n <sha>
git diff --staged
git reset --mixed
```

If the staged diff is clean and still appropriate for `dev`, keep it. If not,
port the logic manually.

### Phase 4: Apply auth hardening

Preserve the behaviour from:

1. `b2e9f6e9`
2. `9a37a9f6`
3. `140a3195`

Focus files:

- `apps/anvil-api/src/routes/auth*.ts`
- `apps/anvil-api/src/routes/admin.ts`
- `apps/anvil-api/src/lib/*`
- relevant auth tests

### Phase 5: Port Rust CLI review fixes

Use current `dev` files as the base and port missing logic from these commits:

1. `0623146c`
2. `f5766723`
3. `21148747`
4. `caa2687c`
5. `319cd9a7`
6. `2c67c70c`
7. `abe0943b`

Primary focus files:

- `crates/anvil-cli/src/main.rs`
- `crates/anvil-cli/src/commands/*`
- `crates/anvil-cli/src/auth/*`

Example rule for same-file divergence:

- keep `dev`'s `ANVIL_DEV=1` behaviour and current structure
- also preserve any still-relevant `main` auth evaluation and review-fix logic

### Phase 6: TypeScript and workspace compatibility

Review and port only still-relevant changes from:

1. `55e440c5`
2. `06d03d08`
3. `73b5a6e1`
4. `fbcde70b`
5. `e1c38d5c`

Focus files:

- `package.json`
- `pnpm-workspace.yaml`
- `tsconfig.base.json`
- app/package manifests
- workspace tooling config

Rules:

1. prefer the current `dev` package structure
2. preserve still-needed compatibility flags
3. do not regress newer `dev` topology to older `main` assumptions

### Phase 7: Release and distribution pipeline

Manually port release-critical behaviour from:

1. `e2a4e1c5`
2. `45e38850`

Focus files:

- `.github/workflows/release.yml`
- distribution config and installer scripts

Because `0.3.0` is the first Rust-native distribution release, these changes are
release-critical and should be reviewed carefully against the current `dev`
state.

### Phase 8: Dependency resolution and lockfiles

Merge manifest intent first, then regenerate lockfiles.

Do not hand-merge:

- `pnpm-lock.yaml`
- `Cargo.lock`

Commands:

```bash
pnpm install
cargo generate-lockfile
```

If the Cargo workspace requires resolution through build metadata:

```bash
cargo check
```

Then inspect the regenerated lockfiles before committing them.

### Phase 9: Release-critical docs

Review only correctness-bearing docs from `main`, especially:

- Rust release docs
- install and upgrade docs
- auth docs
- public quickstart and product docs related to `0.3.0`

Likely sources to mine:

1. `f754401c`
2. `9feb0489`
3. `48adec2b`

Skip pure formatting churn unless it carries a semantic fix.

### Phase 10: Full validation

Run a final validation pass on the reconciliation branch.

Baseline commands:

```bash
pnpm install
pnpm run lint
pnpm run typecheck
pnpm run test -- --run
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If the repository has a canonical CI entrypoint or release preflight command,
run that as well.

### Phase 11: Pre-cutover verification

Before branch cutover, verify these questions explicitly:

1. Did we preserve auth hardening from `main`?
2. Did we preserve CI hardening and action pinning?
3. Did we preserve release and distribution changes needed for Rust `0.3.0`?
4. Did we preserve missing Rust CLI review fixes?
5. Were lockfiles regenerated from final manifests?
6. Did we avoid resurrecting obsolete archived or deleted paths?

If the answer to all six is yes, the branch is ready to become the new canonical
line.

### Phase 12: PR to `dev`

Before changing branch roles, land the reconciliation work through a normal PR
into `dev`.

Recommended PR shape:

- base: `dev`
- compare: `reconcile/dev-with-main-fixes`
- title: `fix(branching): reconcile main-only release fixes onto dev`

PR summary should state:

1. `dev` remains the structural base
2. required `main`-only hardening and release fixes were ported
3. lockfiles were regenerated from final manifests
4. this PR is the final pre-cutover reconciliation before `main` is archived

Do not open separate PRs per subsystem unless the branch is too unstable to
validate as a whole. The main risk here is cross-cutting interaction across
workflows, manifests, release packaging, and Rust CLI behaviour, so one
integrated PR is preferred.

After the PR merges, validate `dev` again from the merge commit before any
branch-role cutover.

### Phase 13: Branch cutover

Keep old `main` as an archive and promote the reconciled line.

Suggested sequence:

1. Ensure the reconciliation PR is merged into `dev`.
2. Create `main-archive` from the current remote `main`.
3. Repoint `main` to the reconciled `dev` commit.
4. Recreate or reset `dev` from the new `main` if a clean post-cutover `dev`
   branch is desired.
5. Restore branch protection and default-branch settings.

The important invariant is:

- old `main` remains reachable as `main-archive`
- new `main` points at the reconciled `dev` lineage
- new `dev` starts from that reconciled state

## What Not To Do

1. Do not attempt a blind `main -> dev` merge and resolve conflicts by hand in
   one pass.
2. Do not rebase `dev` onto `main`.
3. Do not text-merge lockfiles.
4. Do not restore deleted or archived paths only because `main` still had them.
5. Do not optimise for commit preservation over behavioural preservation.

## Success Criteria

The recovery is complete when:

1. all required `main`-only hardening and release fixes exist on the reconciled
   line
2. `dev`'s structural cleanup remains intact
3. the repository validates cleanly
4. the release line for `0.3.0` is based on the reconciled branch
5. the post-release runbook explicitly requires `main -> dev` closure for any
   future release-line fixes

## Follow-up Process Fix

Add this rule to the release runbook:

> Any review, CI, audit, or release-time fix that lands on `main` during
> stabilisation must be merged or ported back to `dev` before the release is
> declared complete.

This is the missing loop that caused the divergence in the first place.
