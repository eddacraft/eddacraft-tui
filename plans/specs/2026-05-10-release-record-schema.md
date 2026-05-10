# Release Record Schema Specification

Date: 2026-05-10

Status: Proposed

APS work item: OPMODEL-004

## Purpose

Define the release record as the canonical shipped-state artefact for Anvil.

The release record answers one question: what exact source, APS work, artefacts,
verification evidence, and release decisions shipped? It is consumed by APS
reconciliation and drift checks. It is not an operator log, command handoff file,
or replacement for CI, tags, or GitHub Releases.

## Authority Boundary

| Concern | Authority | Notes |
| --- | --- | --- |
| Operator decisions and recovery narrative | GitHub release tracking issue | Durable human/operator log and resumability trail. |
| Release command implementation | RELORCH | `RELORCH-001` defines command outputs and how commands emit or locate records. |
| Released source snapshot | Annotated `v*` tag | Tag must identify the source commit being released. |
| Validation truth | CI result for commit SHA | Local logs are supporting evidence only. |
| Distributed artefacts | GitHub Release assets | Assets and checksums must match the record. |
| Shipped-state reconciliation | Release record | Canonical join across tag, APS items, assets, and verification. |

The GitHub tracking issue may link to release records and quote summaries from
them, but it must not become the canonical shipped-state source. The release
record may link back to the tracking issue, but it must not contain mutable
operator narrative.

## Storage And Emission

Target-state release tooling should emit a release record after post-publish
verification succeeds.

Acceptable storage locations:

- GitHub Release asset attached to the private release
- GitHub Release asset attached to the public release, if safe to publish
- repository-tracked record under a future `plans/releases/` or equivalent path,
  if RELORCH ratifies a committed-record workflow
- CI artefact for candidate/pre-release dry runs, explicitly marked non-canonical

Canonical shipped-state requires a verified release record associated with the
published tag. Candidate records may use the same schema but must set
`lifecycleState` to `candidate` and must not be consumed for APS shipped-state
updates.

## Schema

The schema is intentionally JSON-compatible so RELORCH commands, CI workflows,
and APS reconciliation can consume it without parsing prose.

```json
{
  "schemaVersion": "1.0",
  "lifecycleState": "published",
  "supersededBy": null,
  "version": "v0.6.1-beta",
  "channel": "beta",
  "source": {
    "repository": "eddacraft/anvil-001",
    "commitSha": "0123456789abcdef0123456789abcdef01234567",
    "tag": "v0.6.1-beta",
    "previousTag": "v0.6.0-beta"
  },
  "releaseIntent": {
    "changeTypes": ["fix"],
    "releaseScope": "patch",
    "versionOverride": null,
    "versionOverrideReason": null
  },
  "aps": {
    "items": [
      {
        "id": "MOD-001",
        "title": "Short APS item title",
        "module": "MOD",
        "changeType": "fix",
        "releaseIntent": "candidate",
        "releaseScope": "patch"
      }
    ],
    "sourceIndexPath": "plans/index.aps.md",
    "reconciledAt": "2026-05-10T00:00:00Z"
  },
  "artifacts": [
    {
      "name": "anvil-x86_64-unknown-linux-gnu.tar.gz",
      "kind": "archive",
      "platform": "x86_64-unknown-linux-gnu",
      "url": "https://github.com/eddacraft/anvil/releases/download/v0.6.1-beta/anvil-x86_64-unknown-linux-gnu.tar.gz",
      "sha256": "hex-encoded-sha256",
      "sizeBytes": 12345678
    }
  ],
  "releases": {
    "private": {
      "repository": "eddacraft/anvil-001",
      "url": "https://github.com/eddacraft/anvil-001/releases/tag/v0.6.1-beta"
    },
    "public": {
      "repository": "eddacraft/anvil",
      "url": "https://github.com/eddacraft/anvil/releases/tag/v0.6.1-beta"
    }
  },
  "verification": {
    "verifiedAt": "2026-05-10T00:00:00Z",
    "ciRunUrl": "https://github.com/eddacraft/anvil-001/actions/runs/123",
    "checks": [
      {
        "name": "release-readiness",
        "status": "pass",
        "url": "https://github.com/eddacraft/anvil-001/actions/runs/123"
      }
    ],
    "installSmoke": {
      "status": "pass",
      "command": "curl -fsSL https://... | sh"
    }
  },
  "policyDecisions": [
    {
      "decision": "version-override",
      "value": "none",
      "reason": "computed version accepted"
    }
  ],
  "trackingIssue": {
    "repository": "eddacraft/anvil-001",
    "number": 1234,
    "url": "https://github.com/eddacraft/anvil-001/issues/1234"
  }
}
```

## Required Fields

All records require these fields:

| Field | Required | Meaning |
| --- | --- | --- |
| `schemaVersion` | Yes | Schema version for deterministic parsers. |
| `lifecycleState` | Yes | `candidate`, `published`, or `superseded`. |
| `source.repository` | Yes | Repository containing the released source snapshot. |
| `source.commitSha` | Yes | Exact commit SHA validated and tagged. |
| `releaseIntent` | Yes | Versioning and change-shape summary. |
| `aps.items` | Yes | APS work items included in this release. Empty only for explicit no-APS emergency releases. |

Published and superseded records additionally require these fields:

| Field | Required | Meaning |
| --- | --- | --- |
| `version` | Yes | Published semantic version tag, including prefix/suffix. |
| `source.tag` | Yes | Immutable release tag. |
| `source.previousTag` | Yes | Previous release boundary used for release contents. |
| `artifacts` | Yes | Distributed release assets and integrity metadata. |
| `releases.private` | Yes | Private release location. |
| `releases.public` | Yes when public artefacts exist | Public release location. |
| `verification.verifiedAt` | Yes | Verification timestamp. |
| `verification.ciRunUrl` | Yes | CI evidence for the released SHA or tag. |
| `trackingIssue` | Yes | Operator log reference; not shipped-state authority. |

Candidate records may omit published-release fields that do not exist yet, such
as final tag URL, published release URLs, and post-publish verification
timestamp. Candidate records must still identify the candidate source SHA and APS
items they were generated from.

Each published artifact entry requires:

| Field | Required | Meaning |
| --- | --- | --- |
| `name` | Yes | Asset name as published. |
| `kind` | Yes | Asset category such as `archive`, `installer`, `checksum`, or `metadata`. |
| `url` | Yes | Fetchable asset URL or release-local asset location. |
| `sha256` | Yes, unless `integrityRef` is present | Hex-encoded SHA256 digest. |
| `integrityRef` | Yes, when `sha256` is absent | Pointer to equivalent integrity metadata. |
| `sizeBytes` | Yes, when available from the release host | Published asset size. |

## Lifecycle States

| State | Meaning | APS effect |
| --- | --- | --- |
| `candidate` | Candidate metadata or artefacts exist but are not published. | Do not mark APS items Released/Shipped. |
| `published` | Tag, assets, checksums, and verification are complete. | Eligible to mark included APS items Released/Shipped. |
| `superseded` | A later release replaces or repairs this release. | Preserve historical shipped state and link the successor. |

`superseded` records must set `supersededBy` to the replacement release version,
tag, and release-record location once that successor exists.

## APS Reconciliation Rules

APS reconciliation may mark an item `Released/Shipped` only when all are true:

1. A `published` release record exists for the tag.
2. The record includes the APS item ID.
3. The record's `source.commitSha` is reachable from the release tag.
4. Required artefacts have checksums or equivalent integrity metadata.
5. Verification checks passed or an explicit policy decision records an accepted
   exception.

APS reconciliation must not infer shipped state from:

- a merged PR
- changelog prose
- GitHub tracking issue comments
- local release logs
- chat/session memory

## RELORCH Integration

OPMODEL defines the authority boundary and minimum record shape. RELORCH owns the
command surface that creates, locates, validates, and publishes release records.

`RELORCH-001` should consume this specification when defining:

- command JSON output fields
- tracking-issue comment references
- release-record emission location
- validation and harness fixtures
- recovery behaviour when a record is missing, malformed, or inconsistent

## Open Decisions For RELORCH

These are intentionally left to RELORCH implementation design:

- whether published records are stored as private release assets, committed files,
  public release assets, or a combination
- exact JSON Schema file path, if generated
- how candidate records are retained and expired
- whether `policyDecisions` becomes a closed enum or a flexible decision log
- whether public records redact private release URLs
