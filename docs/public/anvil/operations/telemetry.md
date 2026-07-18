---
id: telemetry
title: Upcoming anonymous usage telemetry
description:
  Preview the privacy boundary for anonymous usage telemetry in an unreleased
  anvil build.
public_unlisted: true
---

# Upcoming anonymous usage telemetry

This page describes an unreleased telemetry implementation. It is deliberately
excluded from the current navigation because the public `0.9.0-beta` binary does
not send this beacon and does not include the `anvil telemetry` command.

Do not use this page as current beta setup guidance. For the released product's
network boundary, use [local data and security](security.md).

## Proposed privacy boundary

The unreleased implementation is designed to send a narrow anonymous usage
beacon from eligible beta, release-candidate, and stable builds. Source code,
paths, repository names, command arguments, findings, output, hostnames, emails,
account identity, stack traces, and free-form diagnostic text are excluded.

The proposed body contains only:

- a random install identifier that is not derived from a person or machine;
- the anvil version, release channel, platform, and install method;
- the active feature-flag snapshot version; and
- aggregate feature-key counts.

The implementation is designed to send at most one successful beacon per
installation in 24 hours, retain raw beacon rows for no more than 90 calendar
dates, and avoid retaining source IP addresses. The random identifier is not an
authenticated customer identity, so aggregate metrics are directional product
evidence rather than a verified customer count.

## Proposed controls

The unreleased command surface is designed to preview the exact next payload,
turn sending on or off, and rotate the random install identifier. Environment
hard-offs are designed to include `ANVIL_TELEMETRY=off` and `DO_NOT_TRACK=1`.

These controls become actionable documentation only when a public release
contains them. Until then, the current beta behaviour documented by the
[security guide](security.md) is authoritative.
