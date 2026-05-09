---
name: release
description: Agent-driven Anvil release. Thin operator wrapper around deterministic release commands. Owns judgement gates, approvals, monitoring, and failure decisions; executable release mechanics live in scripts/release/*.
---

# Release — Agent-Orchestrated Anvil Release

You are the operator interface for an Anvil release. Deterministic release work
must be performed by checked-in commands, not by re-implementing the workflow in
agent instructions.

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
- Dev branch: `dev`
- Issue label: `release`
- Install site: `https://install.eddacraft.ai`

## Required Commands

The release skill depends on deterministic helper commands under
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

- `assess.sh`: fetches refs/tags, compares `main` and `dev`, reports candidate
  version, release type, touched areas, risk, and recommended branch strategy.
- `preflight.sh`: runs deterministic local gates and verifies tool/version pins.
- `prepare.sh`: updates all version surfaces, release notes, generated public
  docs, and release metadata from one release manifest.
- `promote.sh`: opens or resumes the promotion PR and reports merge status.
- `tag.sh`: verifies `main`, creates/pushes the release tag, and records source
  provenance.
- `monitor.sh`: finds and monitors the cargo-dist release workflow for the tag.
- `verify.sh`: verifies private/public releases, assets, provenance, package
  manager publication state, and install site health.
- `closeout.sh`: performs back-merge/branch cleanup and closes the tracking
  issue after verification.

## Entry Flow

1. Verify required commands exist.
2. Read live state:

```bash
git status --short --branch
git remote -v
gh issue list --repo EddaCraft/anvil-001 --label release --state open --json number,title,url,body
```

3. If an open release issue exists, summarise the likely current phase and ask
   whether to resume it or start a new release.
4. Run assessment:

```bash
bash scripts/release/assess.sh --base main --head dev --json
```

5. Present the assessment to the operator and ask for confirmation or override
   of version, release type, and branch strategy.

## Release Flow

After operator confirmation, run the commands in order. Each command must be
idempotent: if re-run after a partial failure, it should resume or explain the
conflict.

### 1. Preflight

```bash
bash scripts/release/preflight.sh --base main --head dev
```

If this fails, stop. Report the failed gate and ask whether to abort or fix and
retry.

### 2. Prepare

```bash
bash scripts/release/prepare.sh \
  --version <version> \
  --release-type <beta|production> \
  --strategy <direct|stabilisation>
```

Expected output: release tracking issue URL, release metadata path or issue
state block, and commit SHA for the release preparation commit.

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
   mode, skip with issue-log justification, or abort.
4. Do not improvise manual equivalents of deterministic commands.

## Resumability

Resume from live state plus structured release metadata. The tracking issue or
release metadata block should contain at least:

- `version`
- `releaseType`
- `strategy`
- `sourceSha`
- `devSha`
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

## Emergency Recovery

Emergency manual recovery is allowed only when the operator explicitly approves
it after a deterministic command reports failure. Any manual recovery must be
logged to the tracking issue with:

- reason deterministic recovery was insufficient
- commands run
- repos/tags/releases changed
- follow-up needed to encode the recovery into scripts
