---
id: telemetry
title: Anonymous usage telemetry
description:
  See exactly what anvil's narrow anonymous beacon can send and how to inspect,
  disable, or reset it.
owner: FLEET
upstream:
  - crates/anvil-cli/src/telemetry.rs
  - crates/anvil-cli/src/commands/telemetry.rs
verified_against: 0.9.4-beta
---

# Anonymous usage telemetry

`0.9.1-beta` and later include a narrow anonymous usage beacon. It is separate
from the detailed local observations that support `anvil insights` and kindling:
those rows stay on your machine and are never uploaded.

This anonymous beacon is **not** how anvil tracks a signed-in beta account. When
you log in, the server may record login timestamps on your account for product
access and support. That path is separate from this beacon, does not use the
install identifier below, and never turns the beacon into a named user report.

## Consent and timing

Telemetry uses a disclosed opt-out posture. It starts enabled, but anvil cannot
send a first beacon until an eligible interactive first run has shown the full
notice. A non-interactive first run cannot record that disclosure. After an
interactive run records it, later non-interactive commands may send a beacon. A
non-default `ANVIL_HOME` and unreadable consent state always fail closed.

After disclosure, anvil may send at most one successful beacon per installation
in 24 hours. Network work runs in a detached worker with short timeouts; a
failed send is dropped rather than queued and does not block the command you
ran.

## Exact payload

The beacon contains only:

- a schema version;
- a random install identifier derived from nothing about you or your machine;
- the anvil version and closed-set installation method;
- the platform target and release channel;
- the feature-flag snapshot version; and
- aggregate counts for allowlisted feature keys used since the previous
  successful beacon.

It never contains source code, file paths, repository names, command names or
arguments, findings, output, hostnames, emails, account identity, stack traces,
or free-form diagnostic text. Adding another field requires a reviewed update to
the telemetry decision and public contract.

## Inspect or turn it off

Show the current state and send gate. When a beacon is currently eligible, this
also shows the exact next payload; during the 24-hour cooldown or while another
send is in progress, it explains that delivery-state block instead:

```text
anvil telemetry
```

Persist an off or on choice:

```text
anvil telemetry off
anvil telemetry on
```

Rotate the random install identifier so earlier and later beacons cannot be
joined through it:

```text
anvil telemetry reset-id
```

For machine-readable inspection, use:

```text
anvil --json telemetry
```

`ANVIL_TELEMETRY=off` and `DO_NOT_TRACK=1` always override persisted consent.
`DO_NOT_TRACK=1` also disables the local usage-observation surface.

## Storage and retention

Telemetry database rows do not contain source IP addresses. The API transiently
uses an edge-provided IP address for a best-effort, one-minute in-memory rate
limit; an exceeded limit can include that address in application debug output.
This contract does not describe or limit the hosting provider's infrastructure
logs. The ingest service coarsens beacon arrival time to a date, keeps raw
beacon rows for no more than 90 calendar dates, and keeps date-level aggregate
counts after raw rows expire. The random install identifier is not an
authenticated customer identity, so these aggregates are directional product
evidence rather than verified customer counts.

For the broader local and network data boundary, see
[local data and security](security.md).
