# Rollback Bad Candidate Artefact

| Type    | Authority     | Owner   | Status | Freshness                                                                                                          |
| ------- | ------------- | ------- | ------ | ------------------------------------------------------------------------------------------------------------------ |
| Runbook | Authoritative | RELORCH | Live   | Last reviewed 2026-05-24 against the release-readiness workflow spec and `.github/workflows/release-readiness.yml` |

| Upstream                                           | Downstream                                                    |
| -------------------------------------------------- | ------------------------------------------------------------- |
| `.github/workflows/release-readiness.yml`, ADR-049 | release council, on-call operators, rollback-bad-main runbook |

> **Owner:** Release council **Scope:** Candidate-stage release artefacts
> produced by the release-readiness workflow defined in
> [`2026-05-10-release-readiness-workflow.md`](../../plans/specs/2026-05-10-release-readiness-workflow.md).
> Candidate records are non-publishing — they prove a SHA can be released, not
> that it has been. **Companion playbooks:**
> [`rollback-bad-main.md`](./rollback-bad-main.md),
> [`rollback-bad-published-release.md`](./rollback-bad-published-release.md),
> [`emergency-hotfix.md`](./emergency-hotfix.md).

## Purpose

Discard a bad candidate cleanly so the next promotion attempt starts from a
known state. A candidate is bad when it is built from the wrong SHA, the
artefacts it produced are wrong or missing, the readiness workflow lied about
required checks, or the operator decided not to ship that SHA.

## When to use

Trigger any of:

- The release-readiness workflow reports `success` but artefacts are missing,
  malformed, or fail a follow-up check.
- The candidate SHA is no longer wanted for any reason — regression, scope
  change, business decision, or operator preference — before a tag is pushed.
- A required check passed at readiness time but is now known to have masked a
  failure (e.g. flake, false positive, coverage gap).
- The operator decided to abandon the candidate in favour of a different SHA or
  a different release scope.
- An automated supersession rule fires (e.g. a newer candidate exists for the
  same target version and the older one should be discarded).

Do **not** use this playbook once a tag has been pushed for the candidate SHA —
switch to
[`rollback-bad-published-release.md`](./rollback-bad-published-release.md).

## Required access

- Read access to the readiness workflow runs on `eddacraft/anvil-001`.
- Write access to the open release tracking issue (`label:release`) and to the
  candidate release-record location (compatibility mode: tracking-issue comment
  block; target mode: the record location chosen by RELORCH-001).
- Operator approval before discarding a candidate associated with comms or
  partner expectations.

## Decision

Pick one of:

1. **Discard.** Default. Candidate will not ship — defect, scope change, or
   operator decision. Abandon it and let the next promotion build a fresh
   candidate from a known-good SHA.
2. **Rebuild.** The candidate inputs are correct but the artefacts are wrong
   (build flake, missing checksum, partial upload). Re-run the readiness
   workflow against the same SHA.
3. **Supersede.** A newer candidate already exists or is about to be built for
   the same target version. Mark the older candidate `superseded` and link to
   the successor.

Record the choice and the reason in the open release tracking issue before
making changes to artefacts or records.

## Commands

### Inspect candidate state

```bash
gh run list --repo eddacraft/anvil-001 \
  --workflow release-readiness.yml --limit 10
gh run view --repo eddacraft/anvil-001 <run-id> --log-failed
gh release list --repo eddacraft/anvil-001 --exclude-drafts=false | head -20
```

If candidate artefacts were uploaded as a draft release or workflow artefact,
locate the run ID, the source SHA, and the artefact URLs.

### Option 1 — discard

Delete the artefacts and record the discard in the candidate record:

```bash
# Draft release path:
gh release delete <candidate-tag-or-name> \
  --repo eddacraft/anvil-001 --cleanup-tag --yes

# Workflow artefact path — list, then delete each artefact by ID:
gh api repos/eddacraft/anvil-001/actions/runs/<run-id>/artifacts \
  --jq '.artifacts[] | "\(.id)\t\(.name)"'
gh api -X DELETE repos/eddacraft/anvil-001/actions/artifacts/<artifact-id>
# Optional once all artefacts are deleted: also delete the run record itself.
gh run delete --repo eddacraft/anvil-001 <run-id>
```

`gh run delete` removes the run from the Actions UI but leaves uploaded
artefacts downloadable for the configured retention window — the artefact API
calls above are required for true cleanup.

Stop and ask the operator before deletion if the candidate has been shared
externally or referenced in comms.

### Option 2 — rebuild

Confirm the source SHA and re-run readiness against that exact SHA (not just
`main` HEAD, which may have moved). The workflow's `workflow_dispatch` requires
five inputs per
[`2026-05-10-release-readiness-workflow.md`](../../plans/specs/2026-05-10-release-readiness-workflow.md#inputs)
— reuse the original run's inputs unless the operator explicitly changes one:

```bash
gh workflow run release-readiness.yml \
  --repo eddacraft/anvil-001 \
  --ref <known-good-sha> \
  --field sourceSha=<known-good-sha> \
  --field mode=<readiness|candidate-artifacts> \
  --field channel=<beta|stable> \
  --field expectedReachableFrom=<main|migration-dev> \
  --field baseBoundary=<previous-tag-or-sha>
gh run watch --repo eddacraft/anvil-001 <new-run-id>
```

All five `--field` values are required; omitting any of them fails dispatch with
`invalid-input`. The `--ref` and `--field sourceSha` values must match — the
workflow re-checks the SHA and fails if the ref does not resolve to the input
SHA. Recover the original inputs from the prior run's summary block, the release
tracking issue, or `gh run view <prior-run-id> --json …`.

Once the new run reports `success`, treat the prior candidate as superseded (see
Option 3).

### Option 3 — supersede

In the candidate record (compatibility mode: tracking-issue comment; target
mode: the candidate record file/URL):

- Set `lifecycleState` to `superseded`.
- Set `supersededBy` to the replacement candidate's `version`, `tag`, and
  `recordUrl` (use the workflow run URL when no formal record file exists).
- Leave the original artefact metadata in place — the schema preserves
  historical evidence.

Reference: candidate vs published vs superseded states are defined in
[`2026-05-10-release-record-schema.md`](../../plans/specs/2026-05-10-release-record-schema.md#lifecycle-states).

## Success criteria

- The bad candidate is unambiguously marked discarded, rebuilt, or superseded.
- No draft release, workflow artefact, or release-record entry presents the bad
  candidate as eligible for promotion.
- If a rebuild ran, the new readiness run reports `success` and its artefacts
  are accessible.
- The release tracking issue records: bad candidate run ID and SHA, decision,
  successor SHA (if any), and operator approver.
- `pnpm aps:drift` reports no new finding compared to the last clean run before
  the incident. If no pre-incident baseline was captured, record the current
  findings in the tracking issue and defer resolution to a follow-up.

## Release-record updates

The
[record schema](../../plans/specs/2026-05-10-release-record-schema.md#lifecycle-states)
defines `discarded` as the terminal state for a candidate that must never be
promoted. The matching `candidate-discard` policy decision carries the operator
rationale and approval metadata.

- **Discard:** set `lifecycleState: discarded`. Append a `policyDecisions` entry
  describing the discard so reconciliation tools can refuse to consume the
  candidate, including on older compatibility records:

  ```json
  {
    "decision": "candidate-discard",
    "value": "discarded",
    "reason": "<one-line operator reason>",
    "appliedAt": "<ISO-8601 timestamp>",
    "approver": "<operator handle>"
  }
  ```

  Do not delete the record entry — the audit trail must survive.

- **Rebuild:** leave the original candidate record in place and add a successor
  candidate record for the new run; cross-link them.

- **Supersede:** set `lifecycleState: superseded` and populate `supersededBy`
  per the schema. The successor record's lifecycle stays `candidate` until a tag
  promotes it to `published`.

A bad candidate must **never** be transitioned to `published` to "save" it.
Build a new candidate from the corrected SHA instead.

## APS / issue closeout

Candidate-stage rollbacks do not change APS shipped-state, because no APS item
should have been marked `Released/Shipped` from a candidate record. Per the
[record schema](../../plans/specs/2026-05-10-release-record-schema.md#lifecycle-states),
candidates are explicitly ineligible to mark APS items shipped.

For each affected work item:

- If the item was reading `Merged` and was expected to be in the discarded
  candidate, leave the status as `Merged`. Add an inline note that the planned
  candidate was discarded and which successor candidate (if any) the item is
  expected to ride.
- If the item was prematurely marked `Released/Shipped` from a candidate (a
  process bug), revert it to `Merged` and record the correction inline.

Update the release tracking issue with:

- bad candidate run ID, source SHA, and artefact references
- decision (discard / rebuild / supersede) and the operator approver
- successor candidate run ID and SHA, if any
- impacted APS item IDs and any status corrections

Close any incident issue raised for the candidate failure once the successor
candidate or the abandonment decision is recorded.

## Mode notes

- **Compatibility mode (today).** The release-readiness workflow described in
  [OPMODEL-005](../../plans/specs/2026-05-10-release-readiness-workflow.md) is
  not yet wired by RELORCH/CICD; candidate state is held in the release tracking
  issue. Until it is wired, follow the playbook decisions but record the
  rollback in the tracking issue rather than a candidate record file.
- **Target mode.** Candidate records become first-class artefacts; supersession
  is a record-level state change rather than an issue-comment edit. The decision
  tree in this playbook is unchanged.
- **Release skill interaction.** The release skill must not promote a candidate
  whose record has `lifecycleState: discarded` or carries a compatibility
  `policyDecisions` entry with `decision: "candidate-discard"`, or whose
  `lifecycleState` is `superseded`. If the skill encounters such a record during
  resume, stop and ask the operator before any further mutation.
