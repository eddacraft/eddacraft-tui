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

At entry, detect which release model is executable:

1. **Target command mode:** all `scripts/release/*.sh` commands listed below
   are present and invokable with `bash`, and `RELORCH-011` is Complete.
2. **Preflight-only mode:** target commands are missing but
   `scripts/release.sh` exists, or target commands exist before
   `RELORCH-011` ratifies them.

The OPMODEL-012 cutover completed 2026-05-11; `main` is the only permanent
product branch. Release execution targets a green `main` SHA — there is no
`dev -> main` promotion. Until `RELORCH-011` implements and wires the target
commands, use preflight-only mode.

In preflight-only mode, run only the checked-in deterministic preflight unless
the operator supplies exact release mutation commands for an explicitly
approved release or emergency recovery:

```bash
bash scripts/release.sh
```

After preflight, stop before mutating branches, tags, GitHub Releases, package
registries, taps, or installer state. Mutating steps are operator-owned in
preflight-only mode: require explicit per-step approval, exact commands, live
precondition checks, and release-issue logging. Do not improvise missing
`scripts/release/*` responsibilities. Do not claim preflight-only-mode issue
notes are release records or APS shipped-state evidence.

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

If any command is missing in target command mode, stop and report the missing
command. Do not fall back to manually performing that command's responsibility.
If the repository is still in preflight-only mode, report that target commands
are not implemented yet and use `bash scripts/release.sh` as the only
deterministic release helper.

If target commands exist before `RELORCH-011` ratifies them, do not use them
as the normal release path. Treat them as implementation under validation
unless the operator explicitly approves a bounded migration exercise.

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

1. Select target command mode or preflight-only mode.
2. Read live state:

```bash
git status --short --branch
git remote -v
gh issue list --repo EddaCraft/anvil-001 --label release --state open --json number,title,url,body
```

3. If an open release issue exists, summarise the likely current phase and
   ask whether to resume it or start a new release.
4. In preflight-only mode, run `bash scripts/release.sh`, then stop before
   release mutation. Continue only when the operator provides exact commands
   and approval for each mutating step. Do not run the target command
   sequence below.
5. In target command mode, run assessment against the selected `main` SHA
   (exact form gated on RELORCH-011's final command contract):

```bash
bash scripts/release/assess.sh --sha <main-sha> --json
```

6. Present the assessment to the operator and ask for confirmation or
   override of version, release type, and strategy.

## Release Flow

After operator confirmation in target command mode, run the commands in order.
Each command must be idempotent: if re-run after a partial failure, it should
resume or explain the conflict.

### 1. Preflight

```bash
bash scripts/release/preflight.sh --sha <main-sha>
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
  --strategy <direct|stabilisation>
```

If a PR needs human review/merge, ask the operator to merge it and then re-run or
resume `promote.sh` until it reports merged state.

### 4. Tag

```bash
bash scripts/release/tag.sh --version <version>
```

The command must verify the local `main` HEAD, remote URL, expected version, and
source provenance before pushing a tag.

### 5. Monitor

```bash
bash scripts/release/monitor.sh --version <version>
```

Block until the workflow finishes unless the operator chooses background polling.
If the workflow fails, report the failed job and ask for retry, recovery, or
abort. Do not patch release workflow state manually unless explicitly approved as
emergency recovery.

### 6. Verify

```bash
bash scripts/release/verify.sh --version <version>
```

Verification must include:

- Private release exists on `EddaCraft/anvil-001`.
- Public release exists on `EddaCraft/anvil`.
- Expected cargo-dist assets are present.
- Release provenance names the build source SHA and workflow run.
- Public `/releases/latest` behaviour matches release policy.
- Homebrew, Scoop, and WinGet publication state is recorded.
- `https://install.eddacraft.ai` returns HTTP 200.

### 7. Comms Approval

If `verify.sh` produces a comms draft, present it to the operator. Send or record
it only after approval.

### 8. Closeout

```bash
bash scripts/release/closeout.sh --version <version>
```

This command owns back-merge, release branch cleanup, final issue update, and
issue closure.

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

Resume from live state plus structured release metadata. In preflight-only
mode, the tracking issue is the durable operator log. In target command mode,
the tracking issue may point to release records but is not shipped-state
authority. The tracking issue or release metadata block should contain at
least:

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

At resume in preflight-only mode, read live state and the tracking issue,
rerun `bash scripts/release.sh` if a fresh preflight is needed, then stop
before any mutation unless the operator provides exact commands and approval.
At resume in target command mode, run `assess.sh` first and then the
appropriate deterministic command for the next incomplete phase. If expected
state and live state disagree, stop and ask the operator which source should
be trusted.

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
explicit operator approval per mutating step, and mutating release commands
remain operator-owned in preflight-only mode.
