---
id: telemetry
title: Anonymous usage telemetry
description:
  Inspect and control anvil's narrow anonymous fleet beacon, including its exact
  next payload, identity, retention, and permanent off switches.
sidebar_position: 6
docgov:
  type: 'Public docs'
  authority: 'Authoritative'
  owner: 'FLEET'
  status: 'Live'
  freshness:
    'Last reviewed 2026-07-16 against `v0.9.0-beta`, ADR-107, and
    `crates/anvil-cli/src/telemetry.rs`'
  upstream:
    '`plans/decisions/107-fleet-telemetry-consent-posture.md`,
    `crates/anvil-cli/src/telemetry.rs`, and the `/api/v1/telemetry` schema'
  downstream: 'anvil users, privacy reviews, and fleet telemetry operators'
---

# Anonymous usage telemetry

Anvil sends a narrow anonymous usage beacon from eligible `anvil start`
sessions. It is disclosed opt-out: the first eligible interactive start shows
the notice before any beacon can be sent. The network request runs separately
with a short timeout, so it does not delay the command; failures are silent and
are not queued for retry.

## See the exact next payload

```bash
anvil telemetry
anvil --json telemetry
```

When sending is allowed, both forms show the exact canonical body the worker
will send. On first allowed inspection, anvil creates the random install ID so
the preview is literal. When sending is blocked, the command does not create an
ID and names the exact blocking reason instead.

The body has exactly these top-level fields:

| Field                   | Value                                                                                                          |
| ----------------------- | -------------------------------------------------------------------------------------------------------------- |
| `schema_version`        | Version of this strict beacon wire format.                                                                     |
| `install_id`            | Random UUID v4, derived from nothing about the user, account, hardware, or repository.                         |
| `version`               | Version of the running anvil binary.                                                                           |
| `install_method`        | One closed-set label: `homebrew`, `scoop`, `winget`, `cargo_dist`, `cargo_install`, `dev_build`, or `unknown`. |
| `platform`              | Platform target triple.                                                                                        |
| `channel`               | Release channel such as `stable`, `beta`, or `nightly`.                                                        |
| `flag_snapshot_version` | Active flag snapshot version; `0` means no remote snapshot is installed.                                       |
| `features`              | Sorted `{key, count}` pairs for feature flags exercised since the last successful beacon.                      |

It never contains paths, repository names, command names, arguments, findings,
file contents, output, hostnames, emails, account or licence identity, the local
salted principal, stack traces, or free-form diagnostic text. Adding a field
requires a dated amendment to ADR-107.

## Identity and frequency

The install ID is stored owner-only beside anvil's other user-scoped state. It
is not derived from a machine or person. A reservation and success commit
enforce at most one successful beacon per install per 24 hours. Feature counts
remain eligible until a beacon succeeds; a failed body is not spooled.

Rotate the ID at any time:

```bash
anvil telemetry reset-id
```

Rotation makes beacons under the old and new IDs unjoinable.

## Turn it off

Any one of these is a permanent hard off:

```bash
anvil telemetry off
export ANVIL_TELEMETRY=off
export DO_NOT_TRACK=1
```

`DO_NOT_TRACK=1` also disables local command-usage collection. A non-default
`ANVIL_HOME`, an unreadable consent file, and a non-terminal first run that
could not mark the notice as shown fail closed. Turn persisted telemetry back on
with `anvil telemetry on`; environment hard offs still take precedence.

## Storage and retention

The API does not retain source IP addresses. Raw beacon rows are retained for 90
days. Daily aggregates are retained indefinitely for active-install, retention,
version, install-method, platform, channel, and feature-adoption views. Access
to those views is operator-only.

## Local Kindling data is separate

Anvil's detailed `command.invoked` and governance observations remain in the
local Kindling pipe and are not uploaded. The beacon derives only the
allowlisted feature-key counts; it never sends a Kindling row or the local
salted principal. See the
[usage analytics privacy contract](../../../observability/usage-analytics.md)
for the full boundary.
