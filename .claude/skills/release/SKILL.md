---
name: release
description: Post-release verification, doc review, comms, and cleanup. Reads .release/manifest.json written by scripts/release.sh. Refuses to start without a valid manifest.
---

# Release — Post-Script Verification & Cleanup

You are the second half of the Anvil release process. The release script
(`scripts/release.sh`) has already run preflight, branching, and tagging.
It wrote `.release/manifest.json` as your gate contract. Follow these steps
in order.

## Gate — Read the Manifest

Read `.release/manifest.json`. If it does not exist:

> **Stop.** No valid release manifest found. Run `scripts/release.sh` first.

If the `timestamp` is older than 24 hours:

> **Stop.** Manifest is stale (written more than 24h ago). Re-run
> `scripts/release.sh` for a fresh release cycle.

Parse and display the manifest summary:

- Version, tag, release type
- Branch strategy
- Issue number and link
- Workflow run ID
- Preflight results (any skips?)
- Diff summary (changed crates and packages)

## Step 1 — Validate Manifest Against Live State

Verify these match reality:

```bash
git tag -l <tag>                          # tag exists
gh issue view <issueNumber> --repo EddaCraft/anvil-001  # issue exists
gh run view <workflowRunId> --repo EddaCraft/anvil-001   # run exists
```

If any fail, warn and ask whether to proceed or abort.

## Step 2 — Monitor Workflow

Check workflow status:

```bash
gh run view <workflowRunId> --repo EddaCraft/anvil-001 --json status,conclusion
```

If still running, inform the operator and ask whether to wait or continue
with other steps and come back.

Expected jobs to verify:
- `plan` — succeeded
- `build-local-artifacts` — succeeded (6 targets)
- `build-global-artifacts` — succeeded
- `host` — created GitHub Release on `EddaCraft/anvil`
- `announce` — posted release notes

Update the GitHub Issue (section 3) with results.

## Step 3 — Verify Artefacts

Check the public release has all expected artefacts:

```bash
gh release view <tag> --repo EddaCraft/anvil --json assets --jq '.assets[].name'
```

Expected artefacts (8):
- `eddacraft-anvil-aarch64-apple-darwin.tar.xz`
- `eddacraft-anvil-x86_64-apple-darwin.tar.xz`
- `eddacraft-anvil-aarch64-unknown-linux-gnu.tar.xz`
- `eddacraft-anvil-x86_64-unknown-linux-gnu.tar.xz`
- `eddacraft-anvil-x86_64-pc-windows-msvc.zip`
- `eddacraft-anvil-aarch64-pc-windows-msvc.zip`
- `eddacraft-anvil-installer.sh`
- `eddacraft-anvil-installer.ps1`

Report any missing artefacts. Check that the release is not stuck in
prerelease if the release type is `production`.

Update the GitHub Issue (section 4) with results.

## Step 4 — Changelog Review

Read `CHANGELOG.md` and cross-reference against `diffSummary` from the
manifest. Assess:

- Does the changelog mention all significant changes visible in the diff?
- Is the format consistent with Keep a Changelog?
- Are there any changes in the diff that seem notable but missing from the
  changelog?

Present findings to the operator. This is a judgment call — the operator
decides whether to update the changelog.

Update the GitHub Issue (section 5) with results.

## Step 5 — Documentation Triage

Read `docs/guides/release-doc-checklist.md`. Cross-reference the
`diffSummary.changedPaths` from the manifest against the checklist items.

For each checklist section, determine whether any items are relevant based
on what changed. Present only the applicable items — skip sections where
nothing changed.

Walk through the applicable items with the operator. For each:
- Check the referenced file for accuracy against the new release
- Flag anything that needs updating
- Mark as reviewed

Update the GitHub Issue (section 5) with results.

## Step 6 — Communications

Draft a release comms message using the template from the runbook (section
8 of `docs/guides/release-runbook.md`):

```
Anvil CLI <tag> is live.
Install: curl --proto '=https' --tlsv1.2 -LsSf https://github.com/EddaCraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
Login: anvil auth login
```

Include any notable changes or known workarounds from the changelog.

Present the draft to the operator for approval before sending.

Update the GitHub Issue (section 6).

## Step 7 — Post-Release Cleanup

### Back-merge to dev

If the branch strategy was `direct`:
- Check if main has commits not on dev (the release commit + tag)
- If so, create a PR to merge main back to dev:

```bash
gh pr create --repo EddaCraft/anvil-001 --base dev --head main \
  --title "chore: sync release <tag> back to dev" \
  --body "Sync version bump and changelog from release <tag>"
```

If the branch strategy was `stabilisation`:
- Create a PR to merge the release branch back to dev:

```bash
gh pr create --repo EddaCraft/anvil-001 --base dev --head <releaseBranch> \
  --title "chore: merge release <tag> back to dev" \
  --body "Sync release hardening, version bump, and changelog from <tag>"
```

- After the back-merge PR is merged, delete the release branch:

```bash
git push origin --delete <releaseBranch>
```

### Public repo release state

If the release type is `production`, verify the public release is not
marked as prerelease:

```bash
gh release view <tag> --repo EddaCraft/anvil --json isPrerelease --jq '.isPrerelease'
```

If it is, flag this — it may need manual promotion.

### Install site health

```bash
curl -fsSL -o /dev/null -w "%{http_code}" https://install.eddacraft.ai
```

Verify it returns 200.

Update the GitHub Issue (section 7) with all cleanup results.

## Step 8 — Close the Issue

Once all steps are verified, close the tracking issue:

```bash
gh issue close <issueNumber> --repo EddaCraft/anvil-001 --comment "Release <tag> verified and cleanup complete."
```

## Important Notes

- Always update the GitHub Issue as you go — it is the permanent record
- If any step fails, do not silently continue. Present the failure to the
  operator and ask how to proceed
- The incident playbook in the runbook (section 6) covers recovery steps
  for common failures — reference it if something goes wrong
- The manifest is ephemeral — once this skill completes, it can be deleted
