# ADR-096: Diagnostic `severity`/`category` are forward-compatible wire enums

## Status

Accepted 2026-07-02 (owner)

## Date

2026-07-02

## Context

The canonical diagnostic wire shape `anvil_kernel_types::Diagnostic`
(`anvil.diagnostic.v1`, `crates/anvil-kernel-types/src/diagnostics.rs`) carried
two **closed** serde enums:

- `Severity` = `Info | Warning | Error`
- `Category` = `Secret | Antipattern | Boundary | Policy | Reasoning |
  CommandSafety | Architecture | Other`

Deserialising an unrecognised value (e.g. a newer producer's
`severity: "fatal"`) failed the **entire** `Diagnostic` parse — the diagnostic
was dropped, not surfaced.

This directly violated the governing envelope spec, which the type's own module
doc-comment cites
([`plans/specs/2026-04-26-diagnostic-envelope-coordination.md`](../specs/2026-04-26-diagnostic-envelope-coordination.md)):

> Subscribers MUST treat unknown `severity` values as `warning`, unknown
> `category` values as `other`, and unknown `mode` values as "render and pass
> through" rather than dropping.

That MUST was **unimplemented**. A full-workspace search found no "consumer
policy above this type" layer that mapped an unknown severity/category to a
fallback; the pin test `diagnostic_schema_unknown_severity_value_fails` asserted
the opposite (whole-`Diagnostic` parse error), and its comment described the
consumer-policy mapping as an aspiration, not existing code. Because serde fails
the containing struct on the closed-enum field before any consumer wrapper can
run, the "policy above the type" the comment imagined is not achievable without a
change to the type itself.

The strictness was also inconsistent with every sibling wire enum, each of which
is already forward-compatible:

- `Mode` — `Known(KnownMode) | Unknown(String)` (`diagnostics.rs`), with the same
  "surface the diagnostic anyway" rationale in its doc-comment.
- `StaleReason` — carries a `#[serde(other)]` fallback.
- `AssuranceState::Bounded` — added with a `#[serde(other)]` fallback expressly
  "so it is genuinely additive" (ADR-085).
- `telemetry::MirrorPath::Unknown` — a `#[serde(other)]` arm that self-documents
  as "mirroring the `Mode::Unknown` forward-compat pattern".

The concern became load-bearing when the shared `DiagnosticEnvelope`
(`= Vec<Diagnostic>`, proto crate, B3) and ADR-061's `validate_paths` widened the
set of cross-surface consumers that deserialise `Diagnostic` from the wire.

This ADR resolves the spec's Open Question #1 ("should `category` be open-ended
or closed?"), which explicitly reserved the right to reopen "if rule-family churn
proves it wrong."

## Decision

Make `Severity` and `Category` **forward-compatible** by adding a
`#[serde(other)] Unknown` unit variant to each, bringing the code into
conformance with the spec's existing MUST.

- An unrecognised `severity`/`category` string deserialises to
  `Severity::Unknown` / `Category::Unknown` instead of failing the whole
  `Diagnostic` parse.
- Consumers treat `Severity::Unknown` as `warning` and `Category::Unknown` as
  `other`, per the spec. The mid-edit severity→decision map is the single
  source of that treatment (`telemetry::midedit_severity_class`); the other
  consumers (SARIF level, enforcement class, gate summary counts, watch render)
  each fold `Unknown` into their warning arm.
- Both enums keep `Copy` (the `#[serde(other)]` variant is a unit, unlike
  `Mode`'s `Unknown(String)`), so no by-value call site churns.

### Why `#[serde(other)]` unit rather than `Unknown(String)`

`Severity`/`Category` are `Copy` value types matched pervasively; an
`Unknown(String)` carrier would remove `Copy` and ripple through every by-value
use. The unit variant matches the most recent accepted precedent
(`AssuranceState::Bounded`, `MirrorPath::Unknown`) and the spec's requirement is
"treat as warning/other," which needs the *fact* of unknownness, not the exact
original string. The variant is visible to telemetry/debugging (a diagnostic
whose severity is `Unknown` is distinguishable from a genuine `Warning`), which a
"fold silently into `Warning`/`Other`" alternative would lose.

### Wire-version impact

This stays on `anvil.diagnostic.v1`. Per the spec's versioning rules, *adding* a
pre-declared value keeps `v1`; this change only makes an older consumer tolerant
of values a newer producer may add, which is the additive direction the spec
already blesses. `Severity::Unknown` / `Category::Unknown` serialise to the
`"unknown"` tag and round-trip.

## Consequences

- The pin test `diagnostic_schema_unknown_severity_value_fails` is replaced by
  `diagnostic_schema_unknown_severity_value_round_trips_as_unknown` (and a
  `..._category_...` sibling) asserting the tolerant behaviour, mirroring the
  existing `Mode` unknown round-trip test.
- A newer producer can introduce a severity/category value without breaking an
  older consumer's deserialize — diagnostics are surfaced (as warning/other),
  never silently dropped.
- Six exhaustive `Severity` match sites gained an `Unknown` arm (SARIF level,
  mid-edit decision class, enforcement class, gate summary counts, watch render);
  `Category` had no exhaustive consumer matches.
- Envelope-spec Open Question #1 is resolved: `category` is open at the consumer
  (tolerant deserialize), while the spec's Category list remains the declared set
  producers emit from.

## Alternatives considered

- **Stay strict; amend the spec to delete the MUST.** Keep the closed enums and
  commit to lockstep producer/consumer versioning. Rejected: it discards a
  forward-compat guarantee the spec already promised, keeps `Diagnostic` the lone
  strict outlier among the wire enums, and makes a newer producer's diagnostic
  vanish on an older consumer — the opposite of "surface it anyway."
- **Fold unknown into `Warning`/`Other` via a custom `Deserialize`** (no new
  variant). Rejected: loses the distinction between a real `warning` and an
  unrecognised value (telemetry can't see forward-compat pressure), and diverges
  from how every sibling enum expressed tolerance.
- **`Unknown(String)` carrier mirroring `Mode`.** Rejected: removes `Copy` from
  two pervasively value-matched enums for a string the "treat as warning/other"
  contract does not need.
