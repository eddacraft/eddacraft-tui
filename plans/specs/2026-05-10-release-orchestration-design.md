# Release Orchestration Command Surface Design

Date: 2026-05-10

Status: Complete

APS work item: RELORCH-001

Supersedes, in part:
[`2026-04-20-relmgmt-agent-driven-release-design.md`](./2026-04-20-relmgmt-agent-driven-release-design.md)

Depends on:
[`2026-05-10-release-record-schema.md`](./2026-05-10-release-record-schema.md),
[`2026-05-10-release-readiness-workflow.md`](./2026-05-10-release-readiness-workflow.md)

## Purpose

Define the deterministic `scripts/release/*.sh` command contract used by the
release skill and release runbook.

This specification replaces the single compatibility helper
`scripts/release.sh` with one checked-in Bash command per release phase:
`assess`, `preflight`, `prepare`, `promote`, `tag`, `monitor`, `verify`, and
`closeout`.

The design preserves the RELMGMT Phase 3 trade-off: release commands do not use a
persistent local manifest or side-channel state directory. They reconstruct state
from `git`, `gh`, CI, release assets, argv, stdin, and structured comments on the
GitHub release tracking issue.

## Authority Boundary

| Concern | Authority | RELORCH command role |
| --- | --- | --- |
| Operator decisions and recovery narrative | GitHub tracking issue | Create, append, parse, and summarise structured comments. |
| Release readiness | CI run for the exact source SHA | Request, locate, and validate readiness evidence; do not replace CI. |
| Released source snapshot | Annotated `v*` tag | Verify and create only after readiness evidence matches the SHA. |
| Published artefacts | GitHub Release assets | Locate, verify, and record assets after cargo-dist publishes them. |
| Shipped-state reconciliation | Release record | Emit or locate the canonical record; never infer shipped state from issue prose. |
| Agent/operator interface | `/release` skill | Consume command JSON and ask judgement questions; do not reimplement command logic. |

After the OPMODEL main-first cutover, release commands use target-state inputs
such as `--source-sha <sha>` for mutating phases. Assessment may still compare
arbitrary refs, but command output must not describe `dev -> main` promotion as
the target-state release topology.

## Load-Bearing Constraints

The command surface must honour these constraints.

1. **No persistent on-disk handoff state.** No `.release/manifest.json`, no
   command cache directory, and no generated local state file that a later command
   depends on. Single-process temporary files are allowed only under `mktemp` and
   must be removed on every exit path.
2. **Tracking issue as durable operator log.** Commands may write structured
   comments to the GitHub release tracking issue. The issue records narrative and
   resumability metadata; it is not release authority or shipped-state evidence.
3. **Idempotency by phase.** Commands are safe to re-run before irreversible side
   effects. `tag.sh` is split into pre-push idempotency and post-push recovery;
   re-running after a remote tag exists requires `--recover`.
4. **Cross-platform Bash.** Commands run on macOS and Linux with portable shell,
   `git`, `gh`, `jq`, `cargo`, and `pnpm`. GNU-only flags need portable fallbacks.

## Command Naming And Entry Points

Every command lives under `scripts/release/` and is invokable with `bash` from the
repository root:

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

`--help` exits `0` and prints usage without checking network state. `--json`
prints only JSON to stdout. Human-readable mode may print summaries to stdout and
diagnostics to stderr, but must not be the parser contract.

## Common Arguments

All commands support these arguments unless explicitly marked not applicable.

| Argument | Meaning |
| --- | --- |
| `--json` | Emit only the common JSON envelope. |
| `--repo <owner/name>` | Private source repository; defaults to `eddacraft/anvil-001`. |
| `--public-repo <owner/name>` | Public release repository; defaults to `eddacraft/anvil`. |
| `--tracking-issue <number-or-url>` | Existing release issue to append to or resume from. |
| `--version <vX.Y.Z[-suffix]>` | Release version or tag. Required after assessment chooses a version. |
| `--base <ref>` | Comparison base, usually `main` or previous tag. |
| `--head <ref>` | Comparison head in compatibility mode, usually `dev`. |
| `--source-sha <sha>` | Exact release candidate source SHA in target mode. |
| `--dry-run` | Report planned mutations without applying them. |
| `--recover` | Enter a command-specific recovery path after an irreversible or remote side effect. |

Commands that perform mutations must reject ambiguous inputs. For example,
`tag.sh` must require either a validated `--source-sha` or enough live state to
derive and verify one unambiguously.

## Common JSON Envelope

Every command in `--json` mode emits exactly one JSON object matching this shape.
Command-specific fields live under `data`.

```json
{
  "schemaVersion": "1.0.0",
  "command": "assess",
  "phase": "assessment",
  "mode": "compatibility",
  "status": "success",
  "startedAt": "2026-05-10T00:00:00Z",
  "endedAt": "2026-05-10T00:00:01Z",
  "repository": "eddacraft/anvil-001",
  "inputs": {
    "base": "main",
    "head": "dev",
    "version": null,
    "sourceSha": null,
    "trackingIssue": null
  },
  "trackingIssue": {
    "repository": "eddacraft/anvil-001",
    "number": 1234,
    "url": "https://github.com/eddacraft/anvil-001/issues/1234",
    "metadataCommentUrl": "https://github.com/eddacraft/anvil-001/issues/1234#issuecomment-1"
  },
  "releaseRecord": {
    "lifecycleState": "candidate",
    "recordUrl": null,
    "sha256": null
  },
  "data": {},
  "warnings": [],
  "failures": [],
  "next": {
    "command": "preflight",
    "reason": "Assessment accepted; run readiness gates next."
  }
}
```

Allowed envelope values:

| Field | Values |
| --- | --- |
| `mode` | `compatibility`, `target`, `migration-exercise` |
| `status` | `success`, `noop`, `blocked`, `failed`, `recoverable`, `needs-operator` |
| `phase` | `assessment`, `preflight`, `prepare`, `promote`, `tag`, `monitor`, `verify`, `closeout` |

`warnings` are non-blocking. `failures` are blocking unless the command exits `0`
with `status: noop` to report that no release action is warranted.

## Failure Object

Failures use a stable machine-readable shape.

```json
{
  "code": "validation-failed",
  "message": "cargo test failed",
  "retryable": true,
  "recovery": "fix-and-rerun",
  "evidence": {
    "command": "cargo test --workspace",
    "url": null,
    "path": null
  }
}
```

Common failure codes:

| Code | Meaning |
| --- | --- |
| `invalid-input` | Arguments, refs, version, or issue identifiers are malformed. |
| `auth-failed` | `gh` or `git` cannot authenticate or lacks required scope. |
| `dirty-worktree` | A mutating command needs a clean worktree and found changes. |
| `stale-source` | The requested SHA no longer matches the expected ref or readiness evidence. |
| `validation-failed` | Local or CI validation failed. |
| `artifact-build-failed` | Candidate or release artefact build failed. |
| `integrity-failed` | Checksums, assets, provenance, or record hashes are inconsistent. |
| `remote-conflict` | Tag, PR, release, issue, or package-manager state already exists incompatibly. |
| `infra-failed` | Network, GitHub API, runner, or cache failure. |
| `operator-required` | The command needs an explicit operator decision before proceeding. |
| `contract-drift` | Command output, schema, or tracking issue metadata is unparsable. |

## Exit Codes

Exit code semantics are deterministic and harnessed.

| Exit code | Meaning |
| --- | --- |
| `0` | Success, noop, or no release warranted. JSON `status` distinguishes these. |
| `1..125` | Command-specific recoverable or validation failures. For `preflight.sh`, the code equals the number of failed gates. |
| `126` | Command cannot execute a required local tool. |
| `127` | Required command or executable not found. |
| `128` | Git or GitHub remote/auth precondition failed. |
| `129` | Invalid CLI usage. |
| `130` | Interrupted by operator signal. |
| `131` | Contract or schema drift. |
| `132` | Irreversible side effect detected; recovery mode required. |

Commands must include at least one `failures[]` entry for any non-zero exit.

## Tracking Issue Metadata Comment

Commands append structured comments to the release tracking issue using a marker
and a fenced JSON block. The latest valid comment for a given `version` and
`phase` is the resume source for that phase.

````markdown
<!-- anvil-release-metadata:v1 -->
```json
{
  "schemaVersion": "1.0.0",
  "version": "v0.7.0-beta",
  "releaseType": "beta",
  "strategy": "direct",
  "phase": "prepare",
  "phaseStatus": "success",
  "sourceSha": "0123456789abcdef0123456789abcdef01234567",
  "devSha": "fedcba9876543210fedcba9876543210fedcba98",
  "mainSha": "0123456789abcdef0123456789abcdef01234567",
  "tagSha": null,
  "workflowRun": null,
  "privateReleaseUrl": null,
  "publicReleaseUrl": null,
  "homebrew": "not-started",
  "scoop": "not-started",
  "winget": "not-started",
  "installSite": "not-started",
  "releaseRecord": {
    "lifecycleState": "candidate",
    "recordUrl": null,
    "sha256": null
  },
  "updatedAt": "2026-05-10T00:00:00Z"
}
```
````

The marker line is mandatory. The JSON block is the parser contract. Human prose
may appear before or after the block, but commands must ignore it for state
reconstruction.

The metadata block carries the resumability fields named by the release skill:
`version`, `releaseType`, `strategy`, `sourceSha`, `devSha`, `mainSha`, `tagSha`,
`workflowRun`, `privateReleaseUrl`, `publicReleaseUrl`, `homebrew`, `scoop`,
`winget`, and `installSite`. Additional fields are allowed only when they are
schema-versioned and ignored safely by older commands.

## Release Readiness Contract

Release readiness is CI authority for the exact source SHA that may be tagged.
`preflight.sh` proves local deterministic gates only; it does not create canonical
readiness evidence. RELORCH commands consume the workflow contract in
[`2026-05-10-release-readiness-workflow.md`](./2026-05-10-release-readiness-workflow.md).

Readiness ownership is split across commands:

| Command | Responsibility |
| --- | --- |
| `assess.sh` | Select the candidate `sourceSha`, previous boundary, channel, and requested version inputs for readiness. |
| `prepare.sh` | Create or locate the tracking issue, then request or resume pre-promotion readiness and candidate artefact runs when not already present. |
| `promote.sh` | After compatibility promotion reaches `main`, request or resume canonical readiness for the final `main` SHA that may be tagged. |
| `tag.sh` | Refuse to tag unless canonical readiness succeeded for the exact `sourceSha` and expected branch reachability. |
| `verify.sh` | Carry canonical readiness evidence into the published release record. |

`prepare.sh` must support two readiness modes:

- `--request-readiness`: trigger or resume a `readiness` workflow run for the
  selected SHA.
- `--request-candidate-artifacts`: trigger or resume a `candidate-artifacts` run
  when operator policy requires candidate binaries before tag publication.

Both modes are idempotent. If a matching in-progress or completed workflow run is
found for the same `sourceSha`, `mode`, `channel`, `expectedReachableFrom`,
`baseBoundary`, and `requestedVersion`, the requesting command must report that
run instead of creating another one.

Compatibility promotion has a second readiness point. If `promote.sh` observes a
merge, squash, or rebase result whose `mergedSha` differs from the pre-promotion
candidate SHA, it must request canonical `readiness` for `mergedSha` with
`expectedReachableFrom: main` before returning a taggable state. `tag.sh` must use
that final readiness run, not any pre-promotion or `migration-dev` run, as the
tagging gate.

Readiness metadata in command JSON uses this shape:

```json
{
  "readiness": {
    "required": true,
    "status": "success",
    "mode": "readiness",
    "sourceSha": "0123456789abcdef0123456789abcdef01234567",
    "expectedReachableFrom": "main",
    "baseBoundary": "v0.6.0-beta",
    "requestedVersion": "v0.7.0-beta",
    "resolvedVersion": "v0.7.0-beta",
    "workflowRunUrl": "https://github.com/eddacraft/anvil-001/actions/runs/123",
    "candidateMetadataArtifact": "release-candidate-metadata-readiness-0123456-123",
    "candidateMetadataSha256": "hex-encoded-sha256",
    "failureClass": null,
    "safeToRerun": false
  }
}
```

Allowed readiness statuses are `not-requested`, `requested`, `in-progress`,
`success`, `failed`, and `stale`. `failureClass` uses the release-readiness
workflow classes: `invalid-input`, `stale-source`, `validation-failed`,
`artifact-build-failed`, `integrity-failed`, and `infra-failed`.

Canonical release-record integration rules:

- only readiness runs with `expectedReachableFrom: main` may populate
  `verification.ciRunUrl` or equivalent release-record checks
- compatibility `expectedReachableFrom: migration-dev` runs may be logged on the
  tracking issue but must not populate canonical release-record verification
- candidate artefact metadata may be cited as supporting evidence but must not be
  listed as published release artefacts
- any source SHA change after readiness requires a new readiness run before
  `tag.sh` can push

## Release Record Contract

`verify.sh` owns published release-record emission after post-publish
verification passes. The canonical schema is
[`2026-05-10-release-record-schema.md`](./2026-05-10-release-record-schema.md).

Storage decision:

- The canonical published record is attached as
  `anvil-release-record-<version>.json` to the private GitHub Release on
  `eddacraft/anvil-001`.
- A redacted copy may be attached to the public GitHub Release on
  `eddacraft/anvil` when it does not expose private-only URLs or issue details.
- Candidate records are CI artefacts or tracking-issue metadata references only;
  they are not canonical shipped-state evidence.

The record hash is computed as SHA256 over the exact JSON bytes uploaded as the
private release asset. `verify.sh` emits the asset URL and hash in its envelope.
`closeout.sh` must refuse issue closure if verification passed but the published
record is missing, malformed, or inconsistent with the tag and artefacts.

## Command-Specific Contracts

### `assess.sh`

Purpose: inspect live `git` and `gh` state and propose whether a release is
warranted.

Required inputs: `--base`, `--head` in compatibility mode, or `--source-sha` and
previous release boundary in target mode.

Required `data` fields:

```json
{
  "candidateVersion": "v0.7.0-beta",
  "releaseType": "beta",
  "recommendedStrategy": "direct",
  "previousTag": "v0.6.0-beta",
  "sourceSha": "0123456789abcdef0123456789abcdef01234567",
  "changedPaths": ["crates/anvil-cli/src/main.rs"],
  "apsItems": ["RELORCH-001"],
  "riskSignals": [],
  "releaseWarranted": true
}
```

Exit `0` with `status: noop` and `releaseWarranted: false` when no release is
warranted.

### `preflight.sh`

Purpose: run deterministic local gates and verify pinned tool versions.

Required gates:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `pnpm format:check`
- `pnpm lint:check`
- `pnpm typecheck`
- `pnpm test`
- release tool pin checks for `cargo`, `pnpm`, `node`, `gh`, `git`, and OPA when
  policy tests are in scope

Required `data` fields:

```json
{
  "gates": [
    { "name": "cargo fmt", "status": "pass", "durationMs": 1000 }
  ],
  "failedGateCount": 0,
  "toolVersions": {
    "git": "2.x",
    "gh": "2.x",
    "pnpm": "10.x"
  }
}
```

Exit code equals `failedGateCount`, capped at `125`. Tool execution failures use
the reserved exit codes above.

### `prepare.sh`

Purpose: create or resume the release tracking issue, perform release-time edits,
and create the release preparation commit or report that no prep commit is needed.

Required behaviour:

- reconstruct prior state from the tracking issue and live repo state
- reject dirty worktrees unless `--dry-run` is used
- use `mktemp` only for single-process edit staging when needed
- write a structured metadata comment after each successful phase
- emit candidate release metadata, but not a canonical release record

Required `data` fields include `prepCommitSha`, `changedFiles`,
`trackingIssueUrl`, `candidateMetadata`, and `idempotencyKey`.

### `promote.sh`

Purpose: open or resume the compatibility promotion PR, or report that target mode
does not require promotion.

Required behaviour:

- in compatibility mode, create or locate the release/promotion PR and report
  review, conflict, and merge state
- after merge, request or resume canonical readiness for the final `main` SHA
  before reporting a taggable `merged` state
- in target mode after OPMODEL cutover, exit `0` with `status: noop` when no
  promotion is required
- never merge a PR directly unless an explicit future option authorises it

Required `data` fields include `pullRequest`, `mergeState`, `mergedSha`, and
`operatorActionRequired`. When `mergeState` is `merged`, `data.readiness` is also
required and must identify the canonical readiness run for `mergedSha`.

### `tag.sh`

Purpose: verify source provenance and create the annotated release tag.

Required behaviour:

- verify remote URL, clean worktree, source SHA, expected version, no conflicting
  tag, and readiness evidence before tag creation
- create an annotated tag locally before push
- after push, re-running without `--recover` exits `132` with `status:
  recoverable`
- `--recover` inspects the remote tag and either resumes monitoring or reports a
  concrete conflict

Required `data` fields include `tag`, `tagSha`, `sourceSha`, `readinessRunUrl`,
`pushed`, and `recoveryRequired`.

### `monitor.sh`

Purpose: locate and monitor the cargo-dist release workflow for a tag.

Required behaviour:

- default mode blocks until terminal workflow state
- `--poll` performs a single state check and exits
- report failed jobs with log URLs and retry suitability

Required `data` fields include `workflowRun`, `status`, `conclusion`, `failedJobs`,
and `safeToRetry`.

### `verify.sh`

Purpose: verify private and public release state, package-manager publication,
install site health, and release record consistency.

Required behaviour:

- confirm private release exists on `eddacraft/anvil-001`
- confirm public release exists on `eddacraft/anvil`
- verify expected cargo-dist assets, checksums, provenance, and source SHA
- record Homebrew, Scoop, and WinGet publication state
- verify `https://install.eddacraft.ai` health and `/releases/latest` behaviour
- emit the canonical private release-record asset after checks pass
- optionally produce a comms draft under `data.commsDraft`

Required `data` fields include `privateRelease`, `publicRelease`, `artifacts`,
`publisherState`, `installSite`, `releaseRecord`, and `commsDraft`.

### `closeout.sh`

Purpose: close the release after verification and record final issue state.

Required behaviour:

- refuse to close if `verify.sh` has not produced a valid published release record
- perform compatibility back-merge or sync cleanup only when compatibility mode
  requires it
- clean release branches after confirming remote state
- append the final metadata comment and close the release tracking issue

Required `data` fields include `closedIssue`, `cleanupActions`, `releaseRecord`,
and `finalSummary`.

## Harness Requirements For RELORCH-002

The harness must test command contracts before command implementation is accepted.

Required fixtures:

- clean repo with no release warranted
- compatibility `main`/`dev` divergence with releasable APS metadata
- existing tracking issue with multiple metadata comments
- malformed metadata comment that must trigger `contract-drift`
- existing remote tag requiring `--recover`
- missing or mismatched release record
- failed local preflight gate
- failed cargo-dist workflow run

Required harness checks:

- every `--json` command emits one parseable object and no non-JSON stdout
- every non-zero exit includes `failures[]`
- `preflight.sh` exit code equals failed gate count
- `kill -9` mid-run followed by re-run either resumes safely or reports a
  deterministic recoverable state
- no command writes persistent local state outside its intended source changes
- tracking issue parser selects the latest valid metadata comment for the phase
- schema drift between command output and release skill expectations fails CI

## Skill And Runbook Integration

The release skill remains a judgement and approval wrapper. It must:

- call commands rather than manually performing command-owned work
- parse only JSON envelopes and tracking issue metadata blocks
- stop on command failure and present retry, command recovery, or abort choices
- never treat tracking issue prose as release-record evidence
- keep compatibility mode until RELORCH-011 and OPMODEL cutover authority allow
  target command mode

The release runbook may describe target commands only as executable once the
corresponding command exists and the harness is green.

## Validation

This spec satisfies RELORCH-001 when:

- it is linked from `plans/archive/modules/release-orchestration.aps.md`
- it explicitly addresses the four load-bearing constraints
- it defines tracking issue metadata comment shape
- it defines release-record storage and emission responsibility
- it defines the common JSON envelope, exit codes, failures, and command-specific
  data shapes for RELORCH-002 to harness
