# Rollback Bad Published Release

| Type    | Authority     | Owner   | Status | Freshness                                                                                |
| ------- | ------------- | ------- | ------ | ---------------------------------------------------------------------------------------- |
| Runbook | Authoritative | RELORCH | Live   | Last reviewed 2026-05-24 against release-record schema and `scripts/release/closeout.sh` |

| Upstream                                                                                          | Downstream                                                      |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `scripts/release/closeout.sh`, `scripts/release/tag.sh`, `.github/workflows/release.yml`, ADR-049 | release council, on-call operators, installer surface operators |

> **Owner:** Release council **Scope:** Released versions — the tag is pushed,
> GitHub Releases are published on `eddacraft/anvil-001` (private) and/or
> `eddacraft/anvil` (public), and at least one downstream surface (Homebrew,
> Scoop, WinGet, install site) may have already updated. **Companion
> playbooks:** [`rollback-bad-main.md`](./rollback-bad-main.md),
> [`rollback-bad-candidate-artefact.md`](./rollback-bad-candidate-artefact.md),
> [`emergency-hotfix.md`](./emergency-hotfix.md).

## Purpose

Recover when a published release is bad: ship a corrected successor and mark the
bad release `superseded` per the
[release-record schema](../../plans/specs/2026-05-10-release-record-schema.md#lifecycle-states),
or — only when the operator explicitly authorises it — yank the release. Update
release records, downstream surfaces, and APS shipped-state evidence
consistently.

Tags are immutable. This playbook never rewrites or reuses a released tag; it
either supersedes the release with a new tag or yanks the release as an
explicit, logged exception.

## When to use

Trigger any of:

- Post-deploy smoke check
  ([`post-deploy-smoke-check.md`](./post-deploy-smoke-check.md)) fails against
  the published version.
- A released artefact is missing, corrupted, or has a wrong checksum.
- A regression is reported against the released version that warrants a patch
  release before the next normal cycle.
- A security fix needs to ship outside the normal release cadence (also see
  [`emergency-hotfix.md`](./emergency-hotfix.md)).
- An installer surface (Homebrew, Scoop, WinGet, install site) is serving the
  bad release and the team has agreed to roll it back or supersede it.

Do **not** use this playbook when the bad commit is on the integration branch
but no tag has been pushed against it — switch to
[`rollback-bad-main.md`](./rollback-bad-main.md).

## Required access

- Push access to `eddacraft/anvil-001` (and `eddacraft/anvil` for public
  mirroring of release artefacts where applicable).
- `gh` authenticated against both repos.
- Permission to publish or yank GitHub Releases on both repos.
- Access to publish updates on Homebrew, Scoop, WinGet, and the install site
  (`https://install.eddacraft.ai`).
- Operator approval — captured inline in the tracking issue — before any tag
  push, release publish, release yank, or installer-surface mutation.

## Decision

Pick one of:

1. **Supersede with a patch release.** Default. Ship a corrected version with a
   new tag, mark the bad release `superseded` and link the successor. Preferred
   because tags and assets are immutable and downstream surfaces resolve
   "latest" via release ordering.
2. **Yank.** Use only when the bad release must not remain reachable as "latest"
   or as an installable version, and a successor is not ready to ship. Requires
   explicit operator approval per surface and per repo.
3. **Comms-only correction.** Use when the artefact itself is fine but the
   release notes, advisory, or attribution is wrong. Tag stays. Update the
   release notes and the release record's `releaseNote` text in place; do not
   touch artefacts or checksums.

Record the choice and the reason in the open release tracking issue (or a new
issue labelled `release` if none is open) before any mutation. Operator approval
must be quoted inline.

## Commands

### Inspect published state

```bash
gh release view <tag> --repo eddacraft/anvil-001
gh release view <tag> --repo eddacraft/anvil    # public mirror, if applicable
gh api repos/eddacraft/anvil-001/releases/tags/<tag> --jq '.assets[].name'
curl -sSI https://install.eddacraft.ai/ | head -5
```

Confirm the source SHA, asset names, checksums, and which downstream surfaces
have updated.

### Option 1 — supersede with a patch release

Cut the patch from the released tag, not from `main` HEAD, unless `main` has
moved in a way the operator explicitly approves shipping.

```bash
git fetch origin --tags
git switch -c hotfix/<bad-tag>-<short-slug> <bad-tag>
# implement the fix
pnpm validate:full
```

Open the PR, merge after CI is green and council review per repo policy. Then
follow the release workflow in compatibility mode. Stop at
[preflight](./release-runbook.md#2-run-preflight); ask the operator for exact
mutating commands. Tag, build, publish, and verify per the release runbook.

When the new release is verified:

- Mark the bad release record `lifecycleState: superseded`.
- Set `supersededBy` to the new release `version`, `tag`, and `recordUrl` per
  the
  [record schema](../../plans/specs/2026-05-10-release-record-schema.md#lifecycle-states).
- Update the bad release's GitHub Release body with a single line at the top
  pointing to the successor.
- Update Homebrew, Scoop, and WinGet to the new version per their respective
  publication paths.
- Update the install site and `https://install.eddacraft.ai` to serve the new
  version as latest.

Do **not** delete the bad release or its assets; supersession preserves the
historical record.

### Option 2 — yank (requires explicit operator approval per surface)

Pause the bad release on each surface so installers stop pulling it. Setting
`--draft=true` removes the release from `/releases/latest` and prevents
installer resolution; `--prerelease` is a separate signal and should not be
toggled as part of a yank:

```bash
gh release edit <bad-tag> --repo eddacraft/anvil-001 --draft=true
gh release edit <bad-tag> --repo eddacraft/anvil --draft=true
```

Stop and ask the operator before each step. For installers:

- Homebrew: revert the formula PR or publish a corrective bottle that points at
  the prior good version per Homebrew's documented yank process.
- Scoop: revert the manifest PR.
- WinGet: submit a manifest update per the WinGet validation pipeline.
- Install site: revert the published manifest to the prior good version.

Tags are not deleted: a yanked tag stays in git history. Record the yank
decision, time, and operator approver in the tracking issue.

### Option 3 — comms-only correction

Tag and assets stay. Edit release notes and the release-record `releaseNote`
text in place:

```bash
gh release edit <tag> --repo eddacraft/anvil-001 --notes-file <fixed-notes.md>
gh release edit <tag> --repo eddacraft/anvil --notes-file <fixed-notes.md>
```

Add an inline note in the release record explaining what the original notes got
wrong and when the correction was applied.

## Success criteria

- The bad release is no longer presented as the recommended or latest version on
  any surface where the team has authority (private repo, public mirror,
  Homebrew, Scoop, WinGet, install site).
- For supersession: the new release passes the release runbook's
  [verification step](./release-runbook.md#8-verify-the-release), and smoke
  checks ([`post-deploy-smoke-check.md`](./post-deploy-smoke-check.md)) pass
  against the new release.
- The bad release record is `superseded` with `supersededBy` populated, or
  marked yanked with an inline rationale.
- The release tracking issue captures every mutating command, surface change,
  and operator approver.
- APS items previously cited as `Released/Shipped` from the bad release are
  re-evaluated per the rules below.

## Release-record updates

Per the [record schema](../../plans/specs/2026-05-10-release-record-schema.md):

- **Supersede:** set `lifecycleState: superseded` on the bad record. Set
  `supersededBy.version`, `supersededBy.tag`, and `supersededBy.recordUrl` to
  the successor release. The successor release publishes its own `published`
  record per normal release flow.
- **Yank:** set `lifecycleState: yanked` and append a `policyDecisions` entry
  that reconciliation tools must treat as a hard block on APS shipped-state
  evidence, including on older compatibility records:

  ```json
  {
    "decision": "release-yank",
    "value": "yanked",
    "reason": "<one-line operator reason>",
    "appliedAt": "<ISO-8601 timestamp>",
    "approver": "<operator handle>",
    "surfaces": [
      "github-private",
      "github-public",
      "homebrew",
      "scoop",
      "winget",
      "install-site"
    ]
  }
  ```

- **Comms-only correction:** update `releaseNote.text` and add an inline note in
  the record body explaining the correction. Artefacts, checksums, source SHA,
  and `lifecycleState` stay unchanged. A bad release record must never be edited
  to remove evidence of the bad release; supersession and yank both preserve
  historical state.

## APS / issue closeout

For each APS work item that was marked `Released/Shipped` against the bad
release:

- **Supersede:** if the successor release fixes the regression and the work item
  itself is still correct, the item stays `Released/Shipped` and an inline note
  records that it shipped first in `<bad-tag>` and was preserved through the
  supersession in `<successor-tag>`.
- **Supersede with rollback of the item:** if the supersession reverted the work
  item, return the item from `Released/Shipped` to `Merged` (or to `In Progress`
  if the team will re-attempt). Add an inline note citing the bad and successor
  release records.
- **Yank:** for each affected item, return its status from `Released/Shipped` to
  `Merged` and add an inline note citing the yanked release record and the
  rationale. The item becomes eligible for `Released/Shipped` again only when a
  successor `published` release record exists per the
  [APS reconciliation rules](../../plans/specs/2026-05-10-release-record-schema.md#aps-reconciliation-rules).
- **Comms-only correction:** APS shipped-state is unchanged; only the
  release-note text is corrected.

Update the release tracking issue (or open a new incident issue labelled
`release`) with:

- bad release version, tag, and record URL
- decision (supersede / yank / comms-only) and the operator approver
- successor release version, tag, and record URL (if any)
- per-surface mutations applied (private repo, public mirror, Homebrew, Scoop,
  WinGet, install site) and timestamps
- impacted APS item IDs and their new statuses
- any external comms sent or held

Close the incident issue when the successor release is verified, all authorised
surfaces reflect the decision, APS statuses are reconciled, and operator
approval for closure is recorded inline.

## Mode notes

- **Compatibility mode (today).** Mutating release commands are operator-owned
  per the [release runbook](./release-runbook.md#2-run-preflight): the workflow
  stops after preflight, the operator supplies exact commands per step, and the
  tracking issue is the durable log. Treat that boundary as load-bearing — do
  not delegate the supersession or yank autonomously.
- **Target mode.** When `scripts/release/*.sh` and the release record location
  are wired (post RELORCH-011), supersession runs as a normal release through
  `prepare.sh` / `tag.sh` / `verify.sh` / `closeout.sh`. The yank path stays
  operator-owned because it is a destructive surface mutation.
- **Branching mode.** In compatibility mode, hotfix branches are cut from the
  bad tag and merged to `main` via PR; the back-merge to `dev` is part of the
  hotfix, not deferred. In target mode (post OPMODEL-012) the hotfix branch is
  cut from the bad tag and merged to trunk `main` directly.
