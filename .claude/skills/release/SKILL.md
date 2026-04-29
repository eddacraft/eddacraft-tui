---
name: release
description: Agent-driven Anvil release. Owns version pick, branch strategy, tagging, workflow monitoring, artefact verification, changelog/docs review, comms, and cleanup. Reads live git/gh state each turn. Resumable.
---

# Release — Agent-Driven Anvil Release

You are the judgment half of the Anvil release process. The operator has
already run `./scripts/release.sh` for deterministic preflight (fmt,
clippy, lint, typecheck, tests). Your job: turn a clean preflight into a
shipped release by reading live repository and GitHub state each turn,
making version and branch-strategy calls with the operator, driving the
tag and workflow, and closing out verification, comms, and cleanup.

**Surface constants (use these verbatim):**

- Private repo: `EddaCraft/anvil-001`
- Public repo: `EddaCraft/anvil`
- Default base branch: `main`
- Dev branch: `dev`
- Issue label: `release`
- Install site: `https://install.eddacraft.ai`

## Resumability

Every step reads live state (`git`, `gh`) on entry. Nothing is persisted
to disk between invocations. If a Claude session dies mid-release, the
operator re-invokes `/release` and you pick up by inspecting:

- Open `release`-labelled issues on `EddaCraft/anvil-001`
- Existing tags that match `v*` and have no matching `gh release view`
- Running workflows triggered by recent tag pushes
- Current branch and working-tree state

At entry, after the preflight confirmation (below), search for an open
`release` issue. If one exists, read it, summarise what step looks
unfinished, and ask the operator whether to resume it or start a new
release.

## Entry — Confirm Preflight

Ask the operator:

> Did `./scripts/release.sh` pass cleanly? (y/n)

If **n** or uncertain: stop. Tell them:

> Run `./scripts/release.sh` from the repo root. Every step must report
> `PASS`. When the summary says "All preflight checks passed", come
> back and run `/release` again.

If **y**: proceed.

Then check for an in-flight release (see Resumability above). If none,
start a fresh release at Step 1.

## Step 1 — Assess

Read live state:

```bash
git fetch --all --tags --prune
git log --oneline dev..main                # what's on main not dev (usually 0)
git log --oneline main..dev                # what will ship
git tag --sort=-creatordate | head -5      # recent tags
head -40 CHANGELOG.md                      # current changelog head
git diff --stat main..dev                  # size of the release
```

From the last-tag-to-dev diff, classify what changed at a coarse level:

- Crates touched: `git diff --name-only main..dev -- 'crates/**' | cut -d/ -f2 | sort -u`
- Packages touched: `git diff --name-only main..dev -- 'packages/**' 'apps/**' | cut -d/ -f1-3 | sort -u`
- Docs touched: `git diff --name-only main..dev -- 'docs/**'`

Propose to the operator:

- **Version:** next SemVer from the most recent tag. Patch for fix-only
  diffs; minor for feature additions; major only if ADR-020 lockstep
  versioning demands it (rare). Respect the beta suffix convention —
  current beta cycle uses `x.y.z-beta`.
- **Release type:** `beta` if tag ends in `-beta`, otherwise `production`.
- **Branch strategy:**
  - `direct` — diff is small, risk-low, no hardening required. Promote
    `dev → main` via one PR.
  - `stabilisation` — diff is large or touches release-critical surfaces
    (anvil-cli, anvil-kernel, auth, IaC). Cut `release/vX.Y.Z` from `dev`,
    harden, promote that branch to `main`.

Present your assessment with the rationale for each pick. Wait for
operator confirmation or override.

## Step 2 — Open Tracking Issue

Ensure the `release` label exists (no-op if already):

```bash
gh label create release --repo EddaCraft/anvil-001 \
  --color 0e8a16 --description "Release tracking" 2>/dev/null || true
```

Create the tracking issue:

```bash
gh issue create --repo EddaCraft/anvil-001 --label release \
  --title "release/vX.Y.Z" \
  --body "<identity + assess summary + preflight confirmed>"
```

Body content:

- Identity: version, tag, release type, branch strategy
- Preflight: "confirmed by operator on <date>"
- Assess summary: crates/packages touched, notable changes, diff size
- A `## Log` section you will append to as the release progresses

Capture the issue number — use it in every later `gh issue comment` call.

## Step 3 — Version Bump on `dev`

Ensure clean working tree, then update version strings on `dev`:

- `Cargo.toml` (workspace `[workspace.package]` version)
- `package.json` (root)
- Bundled workspace `package.json` files — the list matches the bundled
  set used by the publish pipeline:
  - `apps/anvil-api`
  - `packages/adapters`
  - `packages/anvil/contracts`
  - `packages/anvil/core`
  - `packages/anvil/policy`
  - `packages/anvil/ports`
  - `packages/anvil/runtime`
  - `packages/aps`
  - `archive/anvil-mcp-server`
  - `packages/edda-stack`
  - `packages/kindling-integration`
  - `packages/shared/storage`
  - `packages/libs/render`
- `CHANGELOG.md` — add a new section (Keep a Changelog format). Draft it
  from the `main..dev` commit range; present the draft to the operator
  and apply their edits before committing.
- `docs/public/anvil/beta-testing-guide.md` — `**Current version:**` line
- `docs/public/anvil/releases/upgrade-notes.md` — `## Current Version:` heading

Commit on `dev`:

```
chore(release): prepare vX.Y.Z
```

Push `dev`. Confirm `origin` fetch and push URLs both match
`EddaCraft/anvil-001` before pushing — refuse if not.

Append a log line to the tracking issue: "Version bump committed:
`<sha>`".

## Step 4 — Promote to `main`

### Direct strategy

Open the promotion PR:

```bash
gh pr create --repo EddaCraft/anvil-001 --base main --head dev \
  --title "release: vX.Y.Z" \
  --body "Promote dev to main for release vX.Y.Z. Tracking: #<issue>"
```

Ask the operator to review and merge the PR on GitHub. Poll until
merged (max 2 min, 5 s interval):

```bash
gh pr view <pr> --repo EddaCraft/anvil-001 --json state -q .state
```

### Stabilisation strategy

```bash
git switch dev && git pull --ff-only origin dev
git switch -c release/vX.Y.Z
git push -u origin release/vX.Y.Z
gh pr create --repo EddaCraft/anvil-001 --base main \
  --head release/vX.Y.Z \
  --title "release: vX.Y.Z" \
  --body "Promote release/vX.Y.Z to main. Tracking: #<issue>"
```

Operator applies stabilisation fixes to `release/vX.Y.Z` as needed.
Merge when stable. Poll as above.

### Post-merge verification

After merge:

```bash
git switch main && git pull --ff-only origin main
```

Verify `Cargo.toml` on `main` reports the release version. If not, the
release prep commit did not land — abort, ask operator to investigate.

Verify local `main` HEAD matches the PR's merge commit SHA
(`gh pr view <pr> --json mergeCommit -q .mergeCommit.oid`). Mismatch
means another commit landed between merge and pull — abort.

## Step 5 — Tag

From `main` at the verified HEAD:

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

Append to tracking issue: strategy, dev SHA, main SHA, tag SHA, tag
pushed.

## Step 6 — Monitor Workflow

```bash
gh run list --repo EddaCraft/anvil-001 --limit 5 \
  --workflow release.yml \
  --json databaseId,headBranch,event,displayTitle,status
```

Find the run whose `displayTitle` contains `vX.Y.Z`. Capture its run ID.

Watch progress:

```bash
gh run watch <run-id> --repo EddaCraft/anvil-001
```

If the operator prefers not to block, poll every 60 s instead:

```bash
gh run view <run-id> --repo EddaCraft/anvil-001 --json status,conclusion
```

Expected jobs (cargo-dist workflow):

- `plan` succeeds
- `build-local-artifacts` succeeds (6 target matrix)
- `build-global-artifacts` succeeds
- `host` updates GitHub Releases on both `EddaCraft/anvil-001` and
  `EddaCraft/anvil`
- `announce` posts release notes

Log workflow run URL and final status to the tracking issue.

## Step 7 — Verify Artefacts

Expected assets on each repo's release (8 files):

- `eddacraft-anvil-aarch64-apple-darwin.tar.xz`
- `eddacraft-anvil-x86_64-apple-darwin.tar.xz`
- `eddacraft-anvil-aarch64-unknown-linux-gnu.tar.xz`
- `eddacraft-anvil-x86_64-unknown-linux-gnu.tar.xz`
- `eddacraft-anvil-x86_64-pc-windows-msvc.zip`
- `eddacraft-anvil-aarch64-pc-windows-msvc.zip`
- `eddacraft-anvil-installer.sh`
- `eddacraft-anvil-installer.ps1`

Check both:

```bash
gh release view vX.Y.Z --repo EddaCraft/anvil-001 --json assets \
  --jq '.assets[].name' | sort
gh release view vX.Y.Z --repo EddaCraft/anvil       --json assets \
  --jq '.assets[].name' | sort
```

Report any missing assets. If release type is `production`, confirm the
public release is not marked as prerelease:

```bash
gh release view vX.Y.Z --repo EddaCraft/anvil \
  --json isPrerelease --jq '.isPrerelease'
```

Append verification to the tracking issue.

## Step 8 — Changelog & Docs Review

### Changelog

Re-read `CHANGELOG.md` against the live diff:

```bash
git log --oneline <previous-tag>..main   # commits shipped in this release
```

Check that every user-visible change in the commit range is represented
in the new changelog section. Surface gaps to the operator; offer to
draft additions.

### Docs triage

Read `docs/guides/release-doc-checklist.md`. For each section of the
checklist, decide whether any items are relevant based on what changed.
Present only applicable items. Walk through them with the operator;
flag anything that needs updating. Mark reviewed.

Append changelog + docs outcomes to the tracking issue.

## Step 9 — Comms

Draft a release message using the runbook template
(`docs/guides/release-runbook.md` section 8):

```
Anvil CLI vX.Y.Z is live.
Install: curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/EddaCraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh \
  | sh
Login: anvil auth login
```

Include notable changes and known workarounds from the changelog.
Present for operator approval before sending.

Append comms draft + send-outcome to the tracking issue.

## Step 10 — Cleanup

### Back-merge

**Direct strategy:** if `main` has commits not on `dev` (the release prep
commit + tag commit typically do), open a back-merge PR:

```bash
gh pr create --repo EddaCraft/anvil-001 --base dev --head main \
  --title "chore: sync release vX.Y.Z back to dev" \
  --body "Sync version bump and changelog from release vX.Y.Z."
```

**Stabilisation strategy:** back-merge the release branch:

```bash
gh pr create --repo EddaCraft/anvil-001 --base dev --head release/vX.Y.Z \
  --title "chore: merge release vX.Y.Z back to dev" \
  --body "Sync release hardening, version bump, and changelog from vX.Y.Z."
```

After the back-merge PR merges, delete the release branch:

```bash
git push origin --delete release/vX.Y.Z
```

### Public release state

If release type is `production` and the public release is still marked
as prerelease, flag this — it needs manual promotion on the public repo
until cargo-dist gets a patch.

### Install site

```bash
curl -fsSL -o /dev/null -w "%{http_code}" https://install.eddacraft.ai
```

Expect 200. Flag any other status.

Append cleanup results to the tracking issue.

## Step 11 — Close the Issue

```bash
gh issue close <issue> --repo EddaCraft/anvil-001 \
  --comment "Release vX.Y.Z verified and cleanup complete."
```

## Notes

- The tracking issue is the durable record — update it at every step.
- If a step fails, stop. Surface the failure to the operator with
  options: retry, skip with reason logged, or abort.
- The incident playbook (`docs/guides/release-runbook.md` section 6)
  covers recovery for common failures. Reference it rather than
  improvising.
- Do not create local files to track state. Live reads only — that is
  what makes this resumable.
