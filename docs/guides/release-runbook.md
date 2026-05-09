# Anvil Release Runbook

Purpose: ship Anvil with the least manual choreography possible.

The release process is command-driven. Use `/release` as the operator wrapper,
but deterministic work belongs to `scripts/release/*` commands.

## Golden Rule

Do not hand-edit release state during a normal release.

If a deterministic command fails, fix that command or use its recovery mode. Only
perform manual recovery when the operator explicitly approves it, and log the
manual steps in the release tracking issue.

## Required Tools

From the repository root, these commands must work before release day:

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
manual checklist.

## Happy Path

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

If the command opens a PR, review and merge it through GitHub. Re-run the command
after merge so it records the merged state.

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
stop and decide whether to retry, recover, skip with justification, or abort.

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
3. Choose one path: retry after fix, run command recovery mode, skip with
   issue-log justification, or abort.
4. Do not manually perform the failed command's job unless this is emergency recovery.

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
