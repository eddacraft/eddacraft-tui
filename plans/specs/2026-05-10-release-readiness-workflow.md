# Release Readiness And Candidate Artefact Workflow Specification

Date: 2026-05-10

Status: Complete

APS work item: OPMODEL-005

## Purpose

Define the target CI workflow that proves a selected source SHA is ready to tag
and, when requested, builds non-publishing candidate artefacts before the release
tag exists.

This specification defines the workflow contract and authority boundary. It does
not implement the workflow or the release command surface. RELORCH owns the
commands that request, interpret, and publish release readiness evidence.

## Authority Boundary

| Concern | Authority | Notes |
| --- | --- | --- |
| Release intent and included work | APS | APS supplies release metadata and item eligibility. |
| Readiness proof | CI result for exact commit SHA | Local command output is supporting evidence only. |
| Candidate artefacts | CI artefact storage | Candidate artefacts are non-canonical and non-published. |
| Published artefacts | GitHub Release assets | Only tag-driven release workflows publish assets. |
| Shipped-state proof | Release record | Candidate readiness must not mark APS items Released/Shipped. |
| Operator narrative | GitHub tracking issue | The issue may link readiness runs but is not validation truth. |

Until `OPMODEL-012` completes the branch cutover, this is target-state design.
Current executable work still branches from `dev` and PRs target `dev`. Target
release readiness is keyed by a selected `main` SHA after cutover; compatibility
probes may accept a `dev` SHA only when explicitly labelled as migration-only.
Migration-only readiness is non-canonical and must not populate release-record
verification fields.

## Workflow Shape

The target workflow is a manually triggered CI workflow with two modes:

| Mode | Purpose | Publishes? | Canonical? |
| --- | --- | --- | --- |
| `readiness` | Prove a selected SHA can be tagged. | No | CI result is readiness authority for that SHA. |
| `candidate-artifacts` | Build release-like assets without tag publication. | No | Artefacts are temporary validation evidence only. |

Inputs:

| Input | Required | Meaning |
| --- | --- | --- |
| `sourceSha` | Yes | Full commit SHA to validate. Target-state value must be reachable from `main`. |
| `mode` | Yes | `readiness` or `candidate-artifacts`. |
| `channel` | Yes | Release channel such as `beta` or `stable`. |
| `expectedReachableFrom` | Yes | `main` for canonical readiness, or `migration-dev` for explicit compatibility probes. |
| `baseBoundary` | Yes | Comparison boundary for affected-path and release-content decisions, usually previous release tag or previous green product SHA. |
| `requestedVersion` | Yes for tag-intended readiness | Candidate version string when version computation is being checked. Omit only for non-tag exploratory runs. |
| `trackingIssue` | No | GitHub issue number for operator-log cross-linking. |
| `apsItems` | No | Explicit APS item allowlist when testing a constrained candidate. |
| `retentionDays` | No | Candidate artefact retention; defaults to the workflow value. |

The workflow must checkout and validate the exact `sourceSha`. It must fail if
the checked-out commit differs from the requested SHA. Final release tagging must
use the exact SHA from a successful canonical readiness run; any source change
after readiness requires a new readiness run.

## Permissions And Trust Boundary

Release readiness and candidate artefact jobs must run with least privilege:

- default workflow permissions are `contents: read`
- no release, package-registry, tap, OIDC, or deployment secrets are available
- requests from untrusted forks are refused
- tag, release, registry, and tap mutation permissions are unavailable
- publish-capable jobs require a separate tag-driven release workflow

Implementation may add a protected environment or maintainer allowlist for manual
readiness requests, but it must not grant publishing credentials to readiness or
candidate artefact jobs.

## Required Checks

`readiness` mode must include the same validation classes required for a release
candidate. Validation defaults fail closed: if the workflow cannot determine that
a class is safe to skip from `baseBoundary`, `sourceSha`, and deterministic path
rules, it must run that class.

- format and lint checks for source and documentation
- TypeScript typecheck and affected tests when source paths require them
- Rust check, tests, policy tests, and release-relevant packaging checks when
  Rust or release surfaces require them
- release metadata checks for version, changelog, APS release intent, and release
  record preconditions
- release workflow dry planning, without creating a tag or GitHub Release

`candidate-artifacts` mode additionally builds release-like artefacts through a
non-publishing cargo-dist invocation that shares release build inputs with
`.github/workflows/release.yml`, but it must not push tags, create GitHub
Releases, publish package registries, update public taps, or mark APS items as
shipped. Candidate jobs must assert that no tag, release, registry, or tap
mutation occurred.

The workflow may skip validation classes only through deterministic path or mode
rules. Each skip must be visible in the CI summary with the `baseBoundary` used
to decide the skip.

## Candidate Metadata

Every successful run must emit machine-readable candidate metadata in a CI
artefact named `release-candidate-metadata-<mode>-<short-sha>-<run-id>`, with a
stable file path inside the artefact: `release-candidate-metadata.json`.

Minimum JSON fields:

```json
{
  "schemaVersion": "1.0.0",
  "lifecycleState": "candidate",
  "sourceSha": "0123456789abcdef0123456789abcdef01234567",
  "expectedReachableFrom": "main",
  "baseBoundary": "v0.6.0-beta",
  "mode": "readiness",
  "channel": "beta",
  "requestedVersion": "v0.7.0-beta",
  "resolvedVersion": "v0.7.0-beta+candidate.123.0123456",
  "apsItems": ["MOD-001"],
  "trackingIssue": 1234,
  "workflowRunUrl": "https://github.com/eddacraft/anvil-001/actions/runs/123",
  "workflowRef": "eddacraft/anvil-001/.github/workflows/release-readiness.yml@0123456",
  "manifestSha256": "hex-encoded-sha256",
  "createdAt": "2026-05-10T00:00:00Z"
}
```

Candidate metadata may share field names with the release record schema, but it
is not a release record. It must set `lifecycleState` to `candidate` and must not
be consumed for APS shipped-state reconciliation.

Tag-intended readiness must produce a `resolvedVersion`. Candidate artefact
builds must use an unmistakable candidate version marker, such as build metadata
containing the run id and short SHA, so leaked binaries are not confused with a
published release.

## Candidate Artefact Rules

Candidate artefacts are temporary validation evidence.

Rules:

1. Artefact names must include the candidate source SHA or a short SHA plus the
   workflow run id.
2. Artefacts must include checksums or a manifest that records checksums, and the
   candidate metadata must include the manifest digest.
3. Retention defaults to 7 days and must not exceed 30 days. Longer retention
   requires a reviewed workflow or APS policy change, not an ad hoc run summary.
4. Artefacts must not be attached to a GitHub Release.
5. Artefacts must not be referenced by install commands or `/releases/latest`.
6. Readiness and candidate artefacts must be rerun or rebuilt after any source
   SHA change.

## Failure Handling

Failures are classified so agents and operators know the next action:

| Failure class | Meaning | Next action |
| --- | --- | --- |
| `invalid-input` | SHA, version, mode, or APS allowlist is malformed. | Fix request and rerun. |
| `stale-source` | SHA is not reachable from the expected branch. | Select a current SHA or record migration exception. |
| `validation-failed` | Required checks failed. | Fix source or metadata before tagging. |
| `artifact-build-failed` | Candidate artefact build failed. | Fix packaging or cargo-dist surface. |
| `integrity-failed` | Checksums or manifests are missing or inconsistent. | Rebuild or repair integrity metadata. |
| `infra-failed` | Runner, cache, network, or GitHub API failure. | Rerun only after identifying transient evidence. |

The workflow summary must include the requested SHA, resolved SHA, mode,
`expectedReachableFrom`, `baseBoundary`, failure class, failed job links, and
whether a rerun is safe.

## Release Record Integration

Release readiness does not create the canonical release record. It provides input
evidence for the later tag-and-publish flow.

When a release is published, the release record should reference the canonical
readiness run through `verification.ciRunUrl` or a check entry if the readiness
run was the validation authority for the tagged SHA. Only readiness runs with
`expectedReachableFrom: main` may serve as release-record validation evidence.
Candidate artefact metadata may be cited as supporting evidence, but only
published artefacts attached to GitHub Releases belong in the release record's
`artifacts` array.

## RELORCH Integration

RELORCH command design should consume this specification when defining:

- how operators request readiness for a SHA
- how command JSON reports readiness run URLs and failure classes
- how candidate metadata is located and validated
- how readiness evidence is carried into the final release record
- how tracking issue comments link to CI runs without becoming validation truth

## Implementation Notes For CI

Implementation should prefer a new workflow such as
`.github/workflows/release-readiness.yml` rather than overloading the tag-driven
`Release` workflow.

The existing `.github/workflows/release.yml` remains the publishing workflow. It
already has a pull-request planning path and a tag publishing path; readiness
implementation may reuse its cargo-dist planning and build commands, but must
keep the non-publishing candidate mode mechanically separate from tag-driven
publication.
