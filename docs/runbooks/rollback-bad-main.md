# Rollback Bad `main`

| Type    | Authority     | Owner   | Status | Freshness                                                                                     |
| ------- | ------------- | ------- | ------ | --------------------------------------------------------------------------------------------- |
| Runbook | Authoritative | RELORCH | Live   | Last reviewed 2026-05-24 against v0.7.1-beta release dry-run and `scripts/release/prepare.sh` |

| Upstream                              | Downstream                         |
| ------------------------------------- | ---------------------------------- |
| `scripts/release/prepare.sh`, ADR-049 | release council, on-call operators |

> **Owner:** Release council **Scope:** `main`-first model — `main` is the
> single integration target. **Companion playbooks:**
> [`rollback-bad-candidate-artefact.md`](./rollback-bad-candidate-artefact.md),
> [`rollback-bad-published-release.md`](./rollback-bad-published-release.md),
> [`emergency-hotfix.md`](./emergency-hotfix.md).

## Purpose

Recover when trunk `main` is broken — before a release tag has been pushed for
the broken state. Stop further work, choose between revert and fix-forward, and
reset APS state so no work item carries spurious shipped-state evidence.

## When to use

Trigger any of the following:

- A merged PR breaks `pnpm validate:full`, `pnpm lint:md`, `pnpm format:check`,
  or required CI checks on the target branch.
- A merged PR introduces a regression visible in smoke checks
  ([`post-deploy-smoke-check.md`](./post-deploy-smoke-check.md)) before any
  release tag has been pushed against the broken commit.
- `pnpm aps:drift` (OPMODEL-010) reports new drift caused by a recent promotion
  that cannot be reconciled by an APS update alone.
- The release skill stops with `preflight` failure caused by recent
  target-branch history rather than the candidate change.

Do **not** use this playbook when a release has already been tagged or published
against the bad commit — switch to
[`rollback-bad-published-release.md`](./rollback-bad-published-release.md).

## Required access

- Push access to `eddacraft/anvil-001` for the target branch (compat: `main`).
- `gh` authenticated against `eddacraft/anvil-001`.
- Write access to the open release tracking issue (`label:release`) if one is
  open.
- Operator approval to freeze promotion is required before any branch mutation.

## Decision

Pick one of:

1. **Revert.** Default. The bad commit (or a small contiguous group) has a
   straightforward revert and the team has not yet built on top of it.
2. **Fix-forward.** Use when a revert is harder than the fix, or when a revert
   would itself break a dependent change that must stay.
3. **Reset target branch to a known-good SHA.** Requires force push and explicit
   operator approval. Use only when the broken history is small, recent, and
   nobody has built on top of it.

Record the choice and the reason in the open release tracking issue (or a new
issue labelled `release` if none is open) before mutating the branch.

## Freeze rule

Before any mutation:

```bash
gh pr list --repo eddacraft/anvil-001 --base main --state open \
  --json number,title,headRefName,statusCheckRollup
```

Notify open-PR owners that promotion to the target branch is paused. Do not
merge any further PR into the target branch until the recovery commit is in.

## Commands

### Inspect the breakage

```bash
git fetch origin
git log --oneline origin/main -n 20
gh run list --repo eddacraft/anvil-001 --branch main --limit 10
gh run view --repo eddacraft/anvil-001 <run-id> --log-failed
```

Identify the commit (or commit range) that introduced the breakage.

### Option 1 — revert

```bash
git switch main
git pull --ff-only origin main
git switch -c revert/<short-slug>
git revert --no-edit <bad-sha>           # or: git revert --no-edit <range>
```

Open the PR:

```bash
gh pr create --repo eddacraft/anvil-001 --base main \
  --title "revert: <one-line reason>" \
  --body "$(cat <<'EOF'
## Reason

Reverts <bad-sha>. <Why the original change was bad>.

## Validation

- `pnpm validate:full`
- Target branch CI back to green
EOF
)"
```

Merge after CI is green and one council reviewer (or `/council` quick) signs
off. The freeze lifts when CI on the target branch is green again.

### Option 2 — fix-forward

```bash
git switch main
git pull --ff-only origin main
git switch -c fix/<short-slug>
# implement the fix
pnpm validate:full
```

Open the PR with `fix:` prefix and link the bad SHA in the body:

```text
Fixes regression introduced in <bad-sha>.
```

Merge after CI is green and council review per repo policy.

### Option 3 — reset target branch (requires explicit operator approval)

```bash
git switch main
git fetch origin
git log --oneline origin/main -n 20
git reset --hard <known-good-sha>
git push --force-with-lease origin main
```

Stop and ask the operator before running `--force-with-lease`. Before pushing,
confirm in the release tracking issue that no other operator is executing Option
3 simultaneously — `--force-with-lease` only protects against ref drift the
local checkout has observed, not against another operator who fetched and reset
to the same SHA in parallel. Record the prior HEAD SHA and the just-fetched
origin SHA in the tracking issue before pushing so anyone with checkouts can
re-anchor and so a second operator entering after the push has a clear "already
done" signal.

## Success criteria

- Target branch CI is green on the recovery commit.
- `pnpm validate:full` passes locally on the target branch HEAD.
- `pnpm aps:drift` reports no new finding compared to the last clean run before
  the incident. If no pre-incident baseline was captured, record the current
  findings in the tracking issue and defer resolution to a follow-up.
- Open-PR owners have been notified the promotion freeze has lifted.
- The release tracking issue (or the new incident issue) records: bad SHA,
  recovery option, recovery SHA, time freeze started and lifted, who approved.

## Release-record updates

If no release record exists for the bad SHA — the normal case for this playbook
— no release record changes are required.

If a candidate release record was created against the bad SHA (e.g. an
in-progress candidate from
[OPMODEL-005](../../plans/specs/2026-05-10-release-readiness-workflow.md)):

- Append a `policyDecisions` entry to the candidate record with
  `decision: "candidate-discard"` per
  [`rollback-bad-candidate-artefact.md`](./rollback-bad-candidate-artefact.md#release-record-updates);
  do not promote it.
- Cross-reference the recovery commit SHA in the candidate record's notes.
- Follow
  [`rollback-bad-candidate-artefact.md`](./rollback-bad-candidate-artefact.md)
  for the candidate-side procedure.

If the bad SHA was already tagged and published, this is the wrong playbook — go
to [`rollback-bad-published-release.md`](./rollback-bad-published-release.md).

## APS / issue closeout

For each work item that was marked `Merged` against the bad commit and is
affected by the revert or reset:

- If the revert or reset removed the change, return the item's status from
  `Merged` to `In Progress` (or to `Ready` if the team is reassessing scope).
  Record the rollback SHA inline so the next attempt can cite it.
- If the change was kept via fix-forward, leave the item's status as `Merged`
  and record the fix-forward SHA in the item's body.
- Do **not** mark any affected item `Released/Shipped` unless and until a
  `published` release record exists per the
  [release-record schema](../../plans/specs/2026-05-10-release-record-schema.md).

Update the open release tracking issue or the incident issue with:

- bad SHA, recovery option, recovery SHA
- impacted APS item IDs and their new statuses
- whether a candidate record was discarded
- promotion-freeze start and end timestamps
- operator approver for any force push

Close the incident issue once CI is green, APS statuses reflect the recovery,
and the freeze is lifted.

## Notes

- Reverts and fix-forwards land via PR against trunk `main`. Force pushes to
  `main` require explicit operator approval and stay the exception, not the
  default — everyone is working off `main` directly, so prefer revert or
  fix-forward unless the operator explicitly authorises the reset.
- **Release skill interaction.** The release skill must stop at preflight when
  `main` is broken; do not run mutating release commands until this playbook has
  restored a green `main`.
