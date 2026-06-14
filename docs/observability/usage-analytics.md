# Usage Analytics Privacy Contract

| Type  | Authority     | Owner | Status | Freshness                         |
| ----- | ------------- | ----- | ------ | --------------------------------- |
| Guide | Authoritative | USAGE | Live   | Live as of 2026-06-14 (USAGE-001) |

| Upstream                                | Downstream                              |
| --------------------------------------- | --------------------------------------- |
| ADR-035, ADR-041, ADR-019, USAGE module | Usage-analytics producers and reviewers |

> **Status:** Live as of 2026-06-14 (USAGE-001 landed). This is the
> founder-confirmed privacy contract for command-invocation usage observations.
> Any change to the captured / not-captured lists requires founder review.
>
> Normative references:
> [ADR-035](../../plans/decisions/035-three-pipe-observability-rule.md)
> (three-pipe rule — Kindling is the source-of-truth pipe),
> [ADR-041](../../plans/decisions/041-flag-snapshot-usage-join-contract.md)
> (inline `flag_set` join contract), and
> [ADR-019](../../plans/decisions/019-flags-observability-alignment.md)
> (gate-affecting-only Kindling flag facts).

## Purpose

USAGE-001 records one durable usage observation per user-initiated CLI command
so the founder can answer **"who is using what"** for dev-investment decisions.
It records _that_ a command ran — never _what it produced_. The data lives on
Kindling, the source-of-truth pipe (ADR-035), as `command.invoked` rows.

## What is captured

Per invocation, the `command.invoked` row carries:

- **Command name** — the canonical command (e.g. `check`, `status`).
- **Anonymised principal** — a one-way SHA-256 hash of the user's email with a
  per-deployment salt. When no identity is on the call path the literal
  `anonymous` is recorded instead. The raw identity never appears in any field.
- **Timestamp** — RFC 3339.
- **Per-argument shape only** — for each argument: its name, plus shape fields
  (value type, a **coarse length bucket** — `empty` / `short` / `medium` /
  `long`, never the exact length — and presence). **Raw argument values are
  never recorded.** The length is bucketed deliberately so a fixed-length secret
  (a token or digest of known size) cannot be confirmed from an exact count.
- **Redacted sensitive arguments** — for arguments whose _name_ matches
  `anvil_observability::redaction::SENSITIVE_FIELDS`, even the shape fields are
  elided and replaced with the literal `<redacted>` marker (the value of the
  `REDACTED` constant). A sensitive argument's _existence_ (its name) stays
  visible; nothing about its value or length leaks.
- **Inline flag set** — the `flag_set` field defined by ADR-041. USAGE-001
  always emits it as an empty array (never omitted); USAGE-002 owns populating
  it from the resolver at the invocation boundary.
- **`traceparent`** — the W3C cross-pipe correlation context (ADR-035) when one
  is bound on the invocation; omitted otherwise.

## What is NOT captured

- **Raw argument values, ever.** Widening to a fuller capture is a follow-up
  review, not a routine code change.
- **Shape fields for sensitive-named arguments** — these collapse to the
  `<redacted>` marker.
- **Command results, output, stdout, stderr.**
- **File contents** touched by the command.
- **Network traffic.**
- **Stack traces or error messages** — those stay on the tracing pipe.

### Residual risk: secrets under non-sensitive flag names

Redaction keys off the argument _name_ only. A secret passed under a
non-sensitive flag name (e.g. `--output <token>`) or as a positional is **not**
name-redacted: its coarse type and length _bucket_ are recorded (never the
value, and never an exact length). The deny-list is not complete protection
against a user who deliberately puts a secret in an arbitrary flag — the coarse
length bucket is the backstop. Operators should not rely on the deny-list as a
guarantee for arbitrary flag names.

## Anonymisation policy

The principal is a one-way SHA-256 hash of the email with a per-deployment salt
held at `<credentials_dir>/usage.salt` (mode `0600` on Unix), generated once on
first use from 256 bits of OS entropy. Rotating the salt is a deliberate privacy
reset — every historical principal hash becomes unjoinable — not a routine
operation.

## Storage and retention

Usage is a cross-cutting, **user-scoped** signal, so rows are appended to
`<credentials_dir>/kindling/usage.ndjson` (the user/deployment state directory,
which re-roots under a gated `ANVIL_HOME` per DISTRIB-006). This mirrors the
audit-chain NDJSON sidecar; the Kindling-integration consumer tails the file.

On Unix the sidecar is created owner-only (`0600`) under an owner-only parent
(`0700`), matching the salt's posture so a shared host cannot read the usage
history; a symlinked target is refused. No permission restriction is applied on
Windows (the platform state-hardening gap tracked alongside DSV-010/011).

The salt and the state directory are created on **first run regardless of
authentication** — running any command (including help/probe commands)
materialises them, recording an `anonymous` row when no identity is present.

Retention defers to Kindling's policy. The sidecar grows by one line per
invocation with no built-in rotation today. _Open: confirm Kindling's retention
policy is acceptable for usage rows specifically, and add a size/line cap or
rotation if heavy CLI use makes unbounded growth a concern (tracked as a
follow-up)._

## Querying the usage log (dev-investment views)

The sidecar is newline-delimited JSON. The canned dev-investment views USAGE-003
will formalise can be answered today with standard tooling. Examples (read-only,
against `<credentials_dir>/kindling/usage.ndjson`):

- **Top-N commands:** count rows grouped by `.command`, sort descending.
- **Commands never invoked:** diff the registered command set against the
  distinct `.command` values present.
- **Flag-dependent paths exercised vs not:** once USAGE-002 populates
  `flag_set`, group rows by `.flag_set[].key`.

A richer first-class query surface (`anvil`-native canned views) is tracked by
USAGE-003.

## Change control

Any change to the captured / not-captured lists requires founder review. This
contract doc lives outside the code so a PR diff is always visible.

## Scope note: JSON-RPC producer

USAGE-001 wires the producer at the CLI entrypoint only. The JSON-RPC daemon
dispatch boundary has no user principal and no flag resolver on the call path,
so an IPC-side producer is **filed as a follow-up** rather than emitting a
principal-less, asymmetric row (consistent with the USAGE module's out-of-scope
clause on principal minting). See the USAGE module plan for the follow-up item.
