# Anvil Release Runbook

Purpose: ship Anvil with the least manual choreography possible.

`main` is the only permanent product branch following the OPMODEL-012 cutover on
2026-05-11. Releases tag a selected green `main` SHA after release readiness —
there is no `dev -> main` promotion.

Until `RELORCH-011` implements and wires the per-phase commands under
`scripts/release/`, `scripts/release/*.sh` is target architecture, not an
executable contract. In the interim, `scripts/release.sh` is the only checked-in
deterministic release helper; it is preflight-only.

## Golden Rule

Do not hand-edit release state during a normal release.

If a deterministic command fails, fix that command or use its recovery mode.
Only perform manual recovery when the operator explicitly approves it, and log
the manual steps in the release tracking issue. Recovery playbooks live under
[`docs/runbooks/`](../runbooks/) — see
[`rollback-bad-main.md`](../runbooks/rollback-bad-main.md),
[`rollback-bad-candidate-artefact.md`](../runbooks/rollback-bad-candidate-artefact.md),
[`rollback-bad-published-release.md`](../runbooks/rollback-bad-published-release.md),
and [`emergency-hotfix.md`](../runbooks/emergency-hotfix.md).

## Required Tools

### Current Preflight Tool

The only checked-in deterministic release helper today is:

```bash
bash scripts/release.sh
```

It is a preflight-only command. It runs deterministic checks and performs no
git, GitHub, tag, release, or package-manager mutations.

### Target Command Surface

After RELORCH-011 implements and wires the command surface, these commands must
work from the repository root before release day:

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

If any command is missing, the release process is not ready. Do not substitute a
manual checklist for that command surface. Before RELORCH-011 lands, use
`scripts/release.sh` and log any manual judgement or recovery steps in the
release tracking issue.

## Happy Path

### Current Preflight-Only Path

Use this path until RELORCH-011 lands the full command surface:

1. Run `bash scripts/release.sh` from the repository root.
2. Confirm all preflight checks pass.
3. Stop before any branch, tag, GitHub Release, package-manager, tap, or
   install-site mutation.
4. Continue only when the operator provides exact commands and explicit approval
   for each mutating step. Before any tag or publication action, verify clean
   worktree, expected remotes, source SHA, version surfaces, no existing tag,
   and green CI for the source SHA.
5. Record decisions, manual steps, failures, and recovery in the release
   tracking issue.

Do not invoke missing `scripts/release/*` commands. Do not treat this path as
shipped-state evidence; APS shipped state requires a verified release record
once that target mechanism exists.

### Future Command-Driven Path

Use this path only after RELORCH-011 implements the commands. Target-state
inputs select a green `main` SHA directly; there is no `--head dev` promotion
step.

#### 1. Start The Release

Run the release skill:

```text
/release
```

The skill reads live state, checks for open release issues, and runs assessment
against the selected `main` SHA (exact form gated on RELORCH-011's final command
contract):

```bash
bash scripts/release/assess.sh --sha <main-sha> --json
```

Approve or override the proposed:

- version
- release type: `beta` or `production`
- strategy: `direct` or `stabilisation`

#### 2. Run Preflight

```bash
bash scripts/release/preflight.sh --sha <main-sha>
```

This must pass before release prep starts. It owns formatting, linting,
typechecking, tests, and release-tool pin checks.

#### 3. Prepare Release State

```bash
bash scripts/release/prepare.sh \
  --version <version> \
  --release-type <beta|production> \
  --strategy <direct|stabilisation>
```

This command owns:

- version surfaces
- release notes
- generated public docs
- release metadata
- release tracking issue
- release preparation commit

Do not manually edit these files if prepare fails.

#### 4. Open Release PR (stabilisation strategy only)

```bash
bash scripts/release/promote.sh --version <version> --strategy stabilisation
```

Used only when the `stabilisation` strategy applies — `main` cannot be tagged
directly and a short-lived `release/*` branch carries hardening commits. Direct
strategy skips this step and tags `main` immediately.

If the command opens a PR, review and merge it through GitHub. Re-run the
command after merge so it records the merged state.

#### 5. Tag

```bash
bash scripts/release/tag.sh --version <version>
```

The tag command must verify:

- local and remote `main` agree
- the version on `main` is correct
- the remote is `EddaCraft/anvil-001`
- provenance state is recorded before tag push

#### 6. Monitor Publishing

```bash
bash scripts/release/monitor.sh --version <version>
```

This watches the cargo-dist release workflow until it finishes. If it fails,
stop and decide whether to retry, use deterministic recovery, or abort. Do not
skip a failed publishing workflow except as explicitly approved emergency
recovery with compensating evidence recorded in the release issue.

#### 7. Verify The Release

```bash
bash scripts/release/verify.sh --version <version>
```

Verification must confirm:

- private GitHub release exists on `EddaCraft/anvil-001`
- public GitHub release exists on `EddaCraft/anvil`
- expected cargo-dist assets are present
- provenance names the build source SHA and workflow run
- Homebrew, Scoop, and WinGet publication state is recorded
- `https://install.eddacraft.ai` returns HTTP 200

#### 8. Approve Comms

If `verify.sh` produces a release announcement draft, approve or edit it before
posting.

Suggested minimum message:

```text
Anvil CLI <version> is live.

Install:
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/EddaCraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh \
  | sh
```

#### 9. Close Out

```bash
bash scripts/release/closeout.sh --version <version>
```

This owns:

- release branch cleanup (when `stabilisation` strategy ran)
- final release issue update
- release issue closure

There is no back-merge step — `main` is the single integration target.

## Strategy Guide

Use `direct` (default) when:

- `main` is already green at a SHA you want to ship
- the release diff is small
- no release hardening commits are needed

Use `stabilisation` when:

- the diff is large
- release-critical Rust, installer, auth, infra, or packaging code changed
- the release needs hardening commits in a `release/*` branch with explicit
  expiry before tag

## Failure Policy

When a command fails:

1. Stop.
2. Read the command output.
3. Choose one path: retry after fix, run command recovery mode, or abort.
4. Do not skip readiness, tag, publish, provenance, verification, or closeout
   gates as normal release procedure.
5. Do not manually perform the failed command's job unless this is emergency
   recovery approved by the operator with exact compensating evidence recorded
   in the release issue.

## Emergency Recovery

Manual recovery must be logged in the release issue with:

- what failed
- why deterministic recovery was insufficient
- exact commands run
- repos, tags, releases, or package-manager records changed
- follow-up needed to encode the recovery into `scripts/release/*`

For specific failure modes, route to the matching playbook before improvising:

- Broken `main` before any tag is pushed:
  [`docs/runbooks/rollback-bad-main.md`](../runbooks/rollback-bad-main.md).
- Bad candidate artefact from the release-readiness workflow:
  [`docs/runbooks/rollback-bad-candidate-artefact.md`](../runbooks/rollback-bad-candidate-artefact.md).
- Bad published release that needs supersession or yank:
  [`docs/runbooks/rollback-bad-published-release.md`](../runbooks/rollback-bad-published-release.md).
- Out-of-band patch release for a regression, security fix, or compliance
  window:
  [`docs/runbooks/emergency-hotfix.md`](../runbooks/emergency-hotfix.md).

## Done Definition

A release is done only when all are true:

- `verify.sh` passed (or, in the current preflight-only path, the operator has
  recorded equivalent manual verification evidence on the tracking issue)
- public release assets are present
- install site returns 200
- downstream package-manager state is recorded
- comms are approved or explicitly skipped
- `closeout.sh` completed (or, in the current path, closeout actions are
  recorded on the tracking issue)
- release issue is closed

In the current preflight-only path, the tracking issue is an interim operator
log only. Link live verification evidence from the issue; do not treat the issue
as release authority or shipped-state authority. If a release ships before
canonical release records exist, mark it as a legacy release without a canonical
release record and track any backfill or reconciliation follow-up explicitly.
RELORCH-001's release-record schema and the corresponding publication step in
`verify.sh` retire this interim arrangement.
