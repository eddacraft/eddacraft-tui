# Anvil Release Runbook

Purpose: ship Anvil with the least manual choreography possible.

This runbook separates current compatibility execution from the target
command-driven release model. Until `RELORCH-011` implements and wires the
per-phase commands, `scripts/release/*` is target architecture, not an
executable contract.

Until `OPMODEL-012` completes branch cutover, release execution also remains in
the current `dev -> main` compatibility model. Target examples that tag a green
`main` SHA do not authorise main-first release execution before that cutover.

## Golden Rule

Do not hand-edit release state during a normal release.

If a deterministic command fails, fix that command or use its recovery mode.
Only perform manual recovery when the operator explicitly approves it, and log
the manual steps in the release tracking issue.

## Required Tools

### Current Compatibility Tool

The only checked-in deterministic release helper today is:

```bash
bash scripts/release.sh
```

It is a preflight-only command. It runs deterministic checks and performs no
git, GitHub, tag, release, or package-manager mutations.

### Target Command Surface

After RELORCH implements the command surface, these commands must work from the
repository root before release day:

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
manual checklist for that command surface. Before RELORCH lands, use the current
compatibility tool above and log any manual judgement or recovery steps in the
release tracking issue.

## Happy Path

### Current Compatibility Path

Use this path until RELORCH command implementation and OPMODEL branch cutover
are complete:

1. Run `bash scripts/release.sh` from the repository root.
2. Confirm all preflight checks pass.
3. Stop before any branch, tag, GitHub Release, package-manager, tap, or install
   site mutation.
4. Continue only when the operator provides exact commands and explicit approval
   for each mutating step. Before any tag or publication action, verify clean
   worktree, expected remotes, source SHA, target branch SHA, version surfaces,
   no existing tag, and green CI for the source SHA.
5. Keep release actions in the `dev -> main` compatibility model unless the
   operator explicitly approves an emergency exception.
6. Record decisions, manual steps, failures, and recovery in the release
   tracking issue.

Do not invoke missing `scripts/release/*` commands. Do not treat this
compatibility path as shipped-state evidence; APS shipped state still requires a
verified release record once that target mechanism exists.

### Future Command-Driven Compatibility Path

Use this path only after RELORCH implements the commands. Before OPMODEL-012,
these examples remain compatibility commands for `dev -> main` promotion and are
not the post-cutover target path.

Post-cutover target releases must tag a selected green `main` SHA after release
readiness. They must not require `--head dev`, a normal promotion PR, or a
back-merge. OPMODEL-012 and RELORCH follow-up work must update this section when
that path becomes executable.

### 1. Start The Release

Run the release skill:

```text
/release
```

The skill reads live state, checks for open release issues, and runs:

```bash
bash scripts/release/assess.sh --base main --head dev --json
```

Approve or override the proposed:

- version
- release type: `beta` or `production`
- strategy: `direct` or `stabilisation`

### 2. Run Preflight

```bash
bash scripts/release/preflight.sh --base main --head dev
```

This must pass before release prep starts. It owns formatting, linting,
typechecking, tests, and release tool pin checks.

### 3. Prepare Release State

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

### 4. Promote To Main

```bash
bash scripts/release/promote.sh \
  --version <version> \
  --strategy <direct|stabilisation>
```

If the command opens a PR, review and merge it through GitHub. Re-run the
command after merge so it records the merged state.

### 5. Tag

```bash
bash scripts/release/tag.sh --version <version>
```

The tag command must verify:

- local and remote `main` agree
- the version on `main` is correct
- the remote is `EddaCraft/anvil-001`
- provenance state is recorded before tag push

### 6. Monitor Publishing

```bash
bash scripts/release/monitor.sh --version <version>
```

This watches the cargo-dist release workflow until it finishes. If it fails,
stop and decide whether to retry, use deterministic recovery, or abort. Do not
skip a failed publishing workflow except as explicitly approved emergency
recovery with compensating evidence recorded in the release issue.

### 7. Verify The Release

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

### 8. Approve Comms

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

### 9. Close Out

```bash
bash scripts/release/closeout.sh --version <version>
```

This owns:

- back-merge or sync PR
- release branch cleanup
- final release issue update
- release issue closure

## Strategy Guide

Use `direct` when:

- `dev` is already green
- the release diff is small
- no release hardening branch is needed

Use `stabilisation` when:

- the diff is large
- release-critical Rust, installer, auth, infra, or packaging code changed
- the release needs hardening commits before `main`

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

## Done Definition

A release is done only when all are true:

- `verify.sh` passed
- public release assets are present
- install site returns 200
- downstream package-manager state is recorded
- comms are approved or explicitly skipped
- `closeout.sh` completed
- release issue is closed

In the current compatibility path, the tracking issue is an interim operator log
only. Link live verification evidence from the issue; do not treat the issue as
release authority or shipped-state authority. If a compatibility release ships
before canonical release records exist, mark it as a legacy release without a
canonical release record and track any backfill or reconciliation follow-up
explicitly.
