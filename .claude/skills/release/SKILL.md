---
name: release
description: Agent-driven Anvil release. Thin operator wrapper around deterministic release commands. Owns judgement gates, approvals, monitoring, and failure decisions; executable release mechanics live in scripts/release/*.
---

# Release — Agent-Orchestrated Anvil Release

You are the operator interface for an Anvil release. Separate current
compatibility execution from the target command-driven model. Deterministic
release work must be performed by checked-in commands when those commands exist;
do not describe target commands as executable before RELORCH implements them.

Your responsibilities:

- Read live repository and GitHub state before each decision.
- Ask the operator for judgement calls: version, release type, branch strategy,
  comms approval, and failure recovery choices.
- Run the deterministic release commands below.
- Stop when a required command is missing or fails.
- Record outcomes in the release tracking issue.

You must not manually edit release files, manually push tags, manually create
release PRs, or manually publish downstream package-manager manifests unless the
operator explicitly asks for emergency recovery.

## Constants

- Private repo: `EddaCraft/anvil-001`
- Public repo: `EddaCraft/anvil`
- Default base branch: `main`
- Retired dev branch: `dev` (tag `dev-retired-2026-05-11`; deletion scheduled
  on or after 2026-07-10 — see issue #1419)
- Issue label: `release`
- Install site: `https://install.eddacraft.ai`

## Mode Selection

At entry, verify the target command model is executable: all required
`scripts/release/*.sh` commands listed below are present and invokable with
`bash`. If any command is missing or fails its own preconditions, stop and report
the failure. Do not substitute manual release choreography.

## Required Commands

Target command mode depends on deterministic helper commands under
`scripts/release/`. At entry, verify they exist and are executable or invokable
with `bash`:

```bash
bash scripts/release/assess.sh --help
bash scripts/release/preflight.sh --help
bash scripts/release/prepare.sh --help
bash scripts/release/promote.sh --help
bash scripts/release/tag.sh --help
bash scripts/release/monitor.sh --help
bash scripts/release/verify.sh --help
bash scripts/release/closeout.sh --help
```

If any command is missing, stop and report the missing command. Do not fall back
to manually performing that command's responsibility.

Expected command ownership:

- `assess.sh`: fetches refs/tags, evaluates the selected `main` SHA against
  the previous release tag, reports candidate version, release type, touched
  areas, risk, and recommended strategy.
- `preflight.sh`: runs deterministic local gates and verifies tool/version pins.
- `prepare.sh`: updates all version surfaces, release notes, generated public
  docs, and tracking-issue release metadata from live `git` / `gh` state.
- `promote.sh`: opens or resumes the promotion PR and reports merge status.
- `tag.sh`: verifies `main`, creates/pushes the release tag, and records source
  provenance.
- `monitor.sh`: finds and monitors the cargo-dist release workflow for the tag.
- `verify.sh`: verifies private/public releases, assets, provenance, package
  manager publication state, and install site health.
- `closeout.sh`: performs release-branch cleanup (when `stabilisation`
  strategy ran) and closes the tracking issue after verification. There is no
  back-merge step — `main` is the single integration target.

## Entry Flow

1. Verify target command mode is available.
2. Read live state:

```bash
git status --short --branch
git remote -v
gh issue list --repo EddaCraft/anvil-001 --label release --state open --json number,title,url,body
```

3. If an open release issue exists, summarise the likely current phase and ask
   whether to resume it or start a new release.
4. Run assessment against the selected `main` SHA. `--base` is required;
   `--source-sha` carries the exact SHA:

```bash
bash scripts/release/assess.sh --base <previous-tag> --source-sha <main-sha> --json
```

5. Present the assessment to the operator and ask for confirmation or override
   of version, release type, and strategy.

## Release Flow

After operator confirmation, run the commands in order. Each command must be
idempotent: if re-run after a partial failure, it should resume or explain the
conflict.

### 1. Preflight

`preflight.sh` flags are optional; `--base` and `--head` only affect the
JSON record of what was compared.

```bash
bash scripts/release/preflight.sh --base <previous-tag>
```

If this fails, stop. Report the failed gate and ask whether to abort or fix
and retry.

### 2. Prepare

```bash
bash scripts/release/prepare.sh \
  --version <version> \
  --release-type <beta|production> \
  --strategy <direct|stabilisation>
```

Expected output: release tracking issue URL, structured issue metadata state, and
commit SHA for the release preparation commit.

Do not hand-edit version files or changelogs if prepare fails. Fix the command or
stop.

### 3. Promote

```bash
bash scripts/release/promote.sh \
  --version <version> \
  --strategy <direct|stabilisation> \
  --source-sha <promoted-source-sha> \
  --request-readiness \
  --channel <beta|stable> \
  --base-boundary <previous-tag-or-ref>
```

If a PR needs human review/merge, ask the operator to merge it and then re-run or
resume `promote.sh` until it reports merged state.

### 4. Tag

```bash
bash scripts/release/tag.sh --version <version> --source-sha <promoted-source-sha>
```

The command must verify the local `main` HEAD, remote URL, expected version,
release-readiness result for the exact source SHA, and source provenance before
pushing a tag.

### 5. Monitor

```bash
bash scripts/release/monitor.sh --version <version>
```

Block until the workflow finishes unless the operator chooses background polling.
If the workflow fails, report the failed job and ask for retry, recovery, or
abort. Do not patch release workflow state manually unless explicitly approved as
emergency recovery.

Current command-state note: if live workflow monitoring is not yet enabled,
`monitor.sh` returns `blocked` and requires explicit workflow evidence before the
operator proceeds.

### 6. Verify

```bash
bash scripts/release/verify.sh --version <version> --source-sha <promoted-source-sha>
```

Verification must include:

- Private release exists on `EddaCraft/anvil-001`.
- Public release exists on `EddaCraft/anvil`.
- Expected cargo-dist assets are present.
- Release provenance names the build source SHA and workflow run.
- Public `/releases/latest` behaviour matches release policy.
- Homebrew, Scoop, and WinGet publication state is recorded.
- `https://install.eddacraft.ai` returns HTTP 200.

Current command-state note: if live host and publisher checks are not yet
enabled, `verify.sh` returns `blocked`; do not treat prose-only verification as a
published release record.

### 7. Comms Approval

If `verify.sh` produces a comms draft, present it to the operator. Send or record
it only after approval.

### 8. Closeout

```bash
bash scripts/release/closeout.sh \
  --version <version> \
  --tag <version> \
  --source-sha <promoted-source-sha> \
  --verification-record <verification-url> \
  --verification-passed
```

This command owns release branch cleanup, final issue update, and issue closure.
There is no back-merge step because `main` is the single integration target.

After closeout, keep the release plan forward-looking (a deterministic command
does not yet own this, so do it as part of closeout):

1. Confirm the shipped tag's durable record exists at `plans/releases/<tag>.md`.
2. **Prune the shipped window from `RELEASE-PLAN.md`** — that file scopes only the
   single active window, never closed releases.
3. **Scope the next window in `RELEASE-PLAN.md`** (theme, phase plans, cut
   criteria) from the `Ready`/`Accepted` modules and ADRs.

See [`docs/policies/release-cadence.md`](../../../docs/policies/release-cadence.md)
(Operator Checklist) and `RELEASE-PLAN.md` ("How this document works").

## Failure Policy

On any command failure:

1. Stop immediately.
2. Summarise the failed command, exit code, and actionable output.
3. Offer only these choices: retry after fixes, run a command-provided recovery
   mode, or abort.
4. Do not offer generic skip as a normal path for readiness, tag, publish,
   provenance, verification, or closeout gates.
5. Do not improvise manual equivalents of deterministic commands unless the
   operator approves emergency recovery with exact compensating evidence recorded
   in the release issue.

## Resumability

Resume from live state plus structured release metadata. The tracking issue may
point to release records but is not shipped-state authority. The tracking issue
or release metadata block should contain at least:

- `version`
- `releaseType`
- `strategy`
- `sourceSha`
- `mainSha`
- `tagSha`
- `workflowRun`
- `privateReleaseUrl`
- `publicReleaseUrl`
- `homebrew`
- `scoop`
- `winget`
- `installSite`

At resume, run `assess.sh` first and then the appropriate deterministic command
for the next incomplete phase. If expected state and live state disagree, stop
and ask the operator which source should be trusted.

Do not resume promotion, tagging, verification, or closeout from a release record
whose `lifecycleState` is `discarded`, `superseded`, or `yanked`, or from a
legacy compatibility record whose `policyDecisions` includes
`candidate-discard` or `release-yank`. Stop and ask the operator which recovery
playbook applies.

## Emergency Recovery

Emergency manual recovery is allowed only when the operator explicitly approves
it after a deterministic command reports failure. Any manual recovery must be
logged to the tracking issue with:

- reason deterministic recovery was insufficient
- commands run
- repos/tags/releases changed
- follow-up needed to encode the recovery into scripts

For recovery scenarios with established procedures, route the operator to the
matching playbook before improvising:

- Broken integration branch before any tag is pushed:
  [`docs/runbooks/rollback-bad-main.md`](../../../docs/runbooks/rollback-bad-main.md).
- Bad candidate artefact from the release-readiness workflow:
  [`docs/runbooks/rollback-bad-candidate-artefact.md`](../../../docs/runbooks/rollback-bad-candidate-artefact.md).
- Bad published release that needs supersession or yank:
  [`docs/runbooks/rollback-bad-published-release.md`](../../../docs/runbooks/rollback-bad-published-release.md).
- Out-of-band patch release for a regression, security fix, or compliance
  window:
  [`docs/runbooks/emergency-hotfix.md`](../../../docs/runbooks/emergency-hotfix.md).

The skill must not execute these playbooks autonomously: each one requires
explicit operator approval per mutating step.
