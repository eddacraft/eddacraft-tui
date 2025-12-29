# DRIFT_REPORT_SPEC — Drift and Architecture Health

## Purpose

Reflective visibility into how the system’s structure changes over time, without
policing every PR.

## Principles

- Separate NEW violations from existing drift.
- Prefer product/runtime language when possible.
- Keep artifacts machine-readable.

## Report types

1. Snapshot

- counts by category/severity
- top boundary crossings
- suppression counts and reason tags

2. Compare (baseline vs now)

- new dependency edges introduced
- new suppressions introduced
- drift trend indicators

## Delivery

- CI artifact + optional PR summary
- scheduled run support later
