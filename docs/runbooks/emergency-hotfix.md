# Emergency Hotfix

| Type    | Authority     | Owner   | Status | Freshness                                                                                       |
| ------- | ------------- | ------- | ------ | ----------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | RELORCH | Live   | Last reviewed 2026-05-24 against v0.7.1-beta release dry-run and `scripts/release/preflight.sh` |

| Upstream                                                          | Downstream                                                  |
| ----------------------------------------------------------------- | ----------------------------------------------------------- |
| `scripts/release/preflight.sh`, `scripts/release/tag.sh`, ADR-049 | release council, on-call operators, post-deploy smoke check |

> **Owner:** Release council **Scope:** Out-of-band patch releases that cannot
> wait for the normal trunk-`main` release cadence. **Companion playbooks:**
> [`rollback-bad-main.md`](./rollback-bad-main.md),
> [`rollback-bad-candidate-artefact.md`](./rollback-bad-candidate-artefact.md),
> [`rollback-bad-published-release.md`](./rollback-bad-published-release.md).

## Purpose

Ship a small, focused fix on top of the latest released tag without dragging
unrelated in-flight work along. Preserve the operating-model invariants:
deterministic preflight, explicit operator approval per mutating step, release
records prove shipped state, APS reconciles from records.

## When to use

Trigger any of:

- Active production regression confirmed against the latest released version.
- Critical security fix that cannot wait for the next planned release.
- Time-bounded compliance, key-rotation, or installer-surface fix.
- Operator escalation that explicitly authorises an out-of-band release.

Do **not** use this playbook for:

- Normal feature work — use the standard release flow.
- A bad commit on the integration branch that has not been tagged — use
  [`rollback-bad-main.md`](./rollback-bad-main.md).
- A bad published release that the team is willing to supersede on the next
  planned cycle — use
  [`rollback-bad-published-release.md`](./rollback-bad-published-release.md) and
  schedule supersession into the next normal release.

## Required access

- Push access to `eddacraft/anvil-001` and the public mirror as required for
  release.
- `gh` authenticated against both repos.
- Permission to publish GitHub Releases and update Homebrew, Scoop, WinGet, and
  the install site (`https://install.eddacraft.ai`) as scope dictates.
- Operator approval — captured inline in the tracking issue — before the hotfix
  branch is opened.

## Scope guard

Before any code change, the operator must answer in writing on the tracking
issue:

- What is the smallest fix that resolves the trigger?
- Which files will it touch? Which packages and crates?
- What does it explicitly **not** include?
- Is a feature flag or kill switch a valid alternative? If yes, prefer that over
  an out-of-band release.

If the answer to "smallest fix" includes refactors, doc cleanups, or test
restructures, the change is not a hotfix; route it through the normal flow.

## Decision

Pick one of:

1. **Tagged patch release.** Default. Cut a hotfix branch from the latest
   released tag, fix, validate, merge to `main`, tag a patch (`+0.0.1`), publish
   via the release skill.
2. **Flag-driven mitigation.** Land the change behind a feature flag enabled
   only for affected users; defer the tag bump to the next normal release.
   Prefer this when a flag exists and gives equivalent risk reduction.
3. **Configuration-only mitigation.** Toggle an existing kill switch or
   environment variable; no code release required. Prefer this when it is
   sufficient on its own.

Record the choice and the reason in the open release tracking issue (or a new
issue labelled `release`) before any branch mutation.

## Commands

### Inspect current state

```bash
git fetch origin --tags
git tag --sort=-creatordate | head -5
gh release view --repo eddacraft/anvil-001 --json tagName,name,publishedAt
gh issue list --repo eddacraft/anvil-001 --label release --state open \
  --json number,title,url
```

Confirm the latest released tag and whether an open release tracking issue
already exists.

### Cut the hotfix branch from the released tag

```bash
git switch -c hotfix/<bad-tag>-<short-slug> <bad-tag>
```

Cut from the **released tag**, not from the integration branch HEAD. This keeps
the patch focused and avoids dragging unrelated work into the release.

### Implement the smallest fix

Apply the change scoped per the operator's written answers above. Do not include
refactors, formatting fixes, or unrelated dependency bumps.

```bash
pnpm validate:full
pnpm lint:md
pnpm format:check
```

If `pnpm aps:drift` (OPMODEL-010) reports new drift outside the hotfix scope,
fix the drift in a separate PR after the hotfix ships; do not bundle.

### Open the PR

Hotfix PRs target `main` directly.

```bash
gh pr create --repo eddacraft/anvil-001 --base main \
  --title "fix: <one-line reason>" \
  --body "$(cat <<'EOF'
## Reason

<What broke and why this is the smallest fix>

## Scope

- Touches: <file/package list>
- Does not touch: <explicit out-of-scope list>

## Validation

- `pnpm validate:full`
- `pnpm lint:md`
- `pnpm format:check`

## Release plan

Tag `<successor-version>` after merge per
[`docs/runbooks/emergency-hotfix.md`](docs/runbooks/emergency-hotfix.md).
EOF
)"
```

Council review per repo policy is required even on hotfixes; use `/council`
quick if a fast turnaround is needed.

### Merge, tag, publish

After merge, follow the [release runbook](./release-runbook.md#2-run-preflight).
Stop at preflight. Ask the operator for exact mutating commands per step (tag,
publish, monitor, verify, downstream surfaces). Record every command and its
result on the release tracking issue.

There is no back-merge step. `main` is the single integration target.

## Success criteria

- The hotfix tag verifies per the release runbook's
  [verification step](./release-runbook.md#8-verify-the-release).
- Smoke checks ([`post-deploy-smoke-check.md`](./post-deploy-smoke-check.md))
  pass against the hotfix release.
- The trigger that justified the hotfix is resolved (regression closed, security
  advisory addressed, compliance window met).
- The release tracking issue records: trigger, scope guard answers, decision
  (tagged patch / flag / config), commands run, surfaces updated, and operator
  approver.
- A `published` release record exists for the successor tag per the
  [release-record schema](../../plans/specs/2026-05-10-release-record-schema.md).

## Release-record updates

- Publish a `published` release record for the successor tag per the normal
  schema. Source SHA must be the `main` HEAD reached by the new tag.
- If the hotfix is correcting a previously published release, mark the prior
  release record `superseded` with `supersededBy` set to the hotfix release per
  [`rollback-bad-published-release.md`](./rollback-bad-published-release.md).
  The supersession edit on the prior record happens in the same change as
  publishing the successor.
- If the hotfix shipped behind a feature flag without a tag bump, no release
  record is created; record the flag rollout decision on the open release
  tracking issue and link the flag's manifest entry per
  [`feature-flag-governance.md`](../guides/feature-flag-governance.md).

## APS / issue closeout

- Create or reuse an APS work item for the hotfix in the appropriate module
  (security fixes typically belong in the owning surface module). The item
  carries `changeType: fix`, `releaseIntent: candidate`, and any release-note
  metadata per
  [`project-context.md`](../../plans/project-context.md#release-metadata-extensions).
- Mark the item `In Progress` before opening the hotfix PR; mark it `Merged` on
  PR merge; mark it `Released/Shipped` only when the hotfix release record is
  `published` and reconciliation per the
  [APS reconciliation rules](../../plans/specs/2026-05-10-release-record-schema.md#aps-reconciliation-rules)
  is satisfied.
- If the hotfix supersedes a prior `Released/Shipped` item, follow the guidance
  in [`rollback-bad-published-release.md`](./rollback-bad-published-release.md):
  preserve the original item's status if the supersession only fixed a
  regression that left the original change intact, or revert the original to
  `Merged` if the supersession also rolled back the original change.

Close the release tracking issue (or the dedicated incident issue) when:

- the hotfix release is verified,
- APS statuses are reconciled,
- and the operator records closure inline.

## Notes

- Hotfix branches cut from the released tag and merge to `main`. There is no
  back-merge step — `main` is the single integration target.
- Until RELORCH-011 lands the target release command surface, mutating release
  commands remain operator-owned per the
  [release runbook](./release-runbook.md#2-run-preflight). The workflow stops at
  preflight in this mode and asks for exact mutating commands; it must not
  improvise an emergency tag push. Treat that emergency-recovery boundary as
  load-bearing.
