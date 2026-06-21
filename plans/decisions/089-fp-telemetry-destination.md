# ADR-089: False-positive reporting telemetry destination

## Status

Accepted

## Date

2026-06-21

## Context

OPSUP-007 (false-positive reporting channel) defines `anvil report-fp
<check-id> <file:line>`, a CLI command that records a structured false-positive
report keyed on the OPSUP-001 stable check ID, with a hashed file path and no
source content by default. The CLI surface and anonymisation policy are
decidable from first principles, but the work item has been **Blocked (design)**
on one cross-cutting question:

> Where does the FP telemetry go — reuse the Kindling pipeline, or stand up a
> new endpoint?

Two architectural facts constrain the answer:

- **Anvil has a hard air-gap guarantee.** The core protection loop (`anvil
  start`, `baseline`, `intercept ensure`, `hook *`, `audit`, and by extension
  the gate path) makes **zero network calls** in normal operation. This is a
  documented product promise (`docs/runbooks/anvil-air-gapped.md`) enforced
  behaviourally by a network-namespace test harness
  (`crates/anvil-cli/tests/air_gapped.rs`). The only outbound calls today are
  user-initiated auth/licence and an opt-in, off-by-default update advisory.
- **Kindling is a local, immutable system-of-record — not a transmission
  pipeline.** It persists observations to a local SQLite database
  (`.anvil/kindling.db`) plus NDJSON sidecars; its contract has no HTTP or
  network transport (`packages/kindling-integration/`). Usage analytics
  (USAGE-001) follow the same local-only, anonymise-by-default posture
  (`docs/observability/usage-analytics.md`).

A default-on "phone home" endpoint would directly contradict the air-gap
guarantee and the privacy-by-default posture, and could not be adopted without
a separate ADR superseding that stance. The decision must therefore settle the
destination in a way consistent with those promises, so OPSUP-007 can move from
Draft to Ready.

## Decision

**The destination for a false-positive report is the local Kindling record. No
telemetry leaves the user's machine as part of OPSUP-007.**

Concretely:

- `anvil report-fp` records a new `false_positive_reported` Kindling
  observation kind (the 13th, alongside the existing 12), carrying: the stable
  `ANV-*` check ID, a **hashed** file path (a salted, domain-separated SHA-256
  digest under the USAGE-001 per-deployment salt — never a plaintext path; see
  `usage::hash_file_path`), the
  rule context, and a timestamp. Source snippets are opt-in only and never
  enabled in the default config (fail-closed on anonymisation).
- The report is validated against the OPSUP-001 registry: an unknown check ID
  is rejected (reusing the registry lookup, consistent with OPSUP-002).
- The command makes **no network call** and is safe under the air-gap harness.

**Egress is explicitly deferred and de-scoped from OPSUP-007.** Any mechanism
that transmits FP reports off the machine (an opt-in `export` bridge, a managed
upload, a support-bundle attachment) is a separate future work item with its own
design and, because it crosses the air-gap boundary, its own opt-in contract and
ADR. It is **not** a new always-on endpoint.

## Rationale

This is the only option consistent with the air-gap guarantee and the existing
privacy posture, and it is the smallest correct scope that unblocks OPSUP-007:

- It reuses infrastructure that already exists and is already air-gap-safe —
  the Kindling observation contract, redaction deny-lists, `ArgShape` shape-only
  encoding, and the USAGE-001 salted-hash model — rather than building a new
  outbound surface.
- It keeps the decidable parts of OPSUP-007 (CLI surface, anonymisation policy)
  shippable now, and isolates the genuinely cross-cutting, harder-to-reverse
  part (data leaving the machine) behind an explicit, deferred, opt-in decision.
- A local-first record is independently useful: it gives the user an auditable
  log of what they flagged, and is the natural source any future opt-in export
  would read from.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Local Kindling record; egress deferred (chosen)** | Preserves air-gap guarantee; reuses existing redaction/hashing/contract; smallest correct scope; unblocks OPSUP-007 now; local record is independently useful | FP signal stays on the user's machine until a future opt-in export exists, so Anvil gets no aggregate FP data yet |
| Local record + opt-in `export` bridge in OPSUP-007 | Same local record plus an actual (opt-in) transmission path | Larger scope; forces the destination URL/contract and an egress opt-in contract to be decided now; couples a cross-air-gap decision into this item |
| New dedicated telemetry endpoint | Maximum aggregate FP signal | Breaks the air-gap guarantee unless strictly opt-in; needs an ADR superseding the air-gap stance; duplicates redaction/hashing; largest new outbound surface |

## Consequences

- **Positive:** OPSUP-007 moves Draft → Ready with a clear, air-gap-safe scope.
  The implementation reuses existing primitives and adds no network surface, so
  it stays inside the behavioural air-gap test. The local record is a clean
  seam any future export reads from.
- **Negative:** Anvil collects no aggregate false-positive signal from this
  work item alone; that value is deferred until an opt-in export work item
  ships. Production FP signal continues to arrive only via support channels in
  the interim.
- **Risks:** A future export work item must not weaken the anonymisation
  established here (hashed paths, no source by default); and it must remain
  opt-in to preserve the air-gap guarantee.
- **Mitigations:** The anonymisation policy is fixed at record time (fail-closed,
  no plaintext path, no source by default), so any later export inherits an
  already-redacted record rather than re-deriving redaction at the egress
  boundary.

## References

- APS modules: OPSUP-007 (false-positive reporting channel), OPSUP-001 (check-ID
  registry, Done), OPSUP-002 (registry-backed resolution, Merged #2824)
- Related: USAGE-001 usage analytics (`docs/observability/usage-analytics.md`),
  air-gap guarantee (`docs/runbooks/anvil-air-gapped.md`), Kindling integration
  (`packages/kindling-integration/`)
- Anonymisation: `usage::hash_file_path` + `anonymise_principal`
  (`crates/anvil-cli/src/usage.rs`, USAGE-001 salt model),
  redaction (`crates/anvil-observability/src/redaction.rs`)
