# ADR-088: Daemon-protection observation kind taxonomy

## Status

**Accepted** — 2026-06-20, Josh. Owner ratified **Decision 2** (fence events get
a distinct `constraint_applied` kind, not `gate.evaluated`); the other decisions
implement binding choices the owner made during the council. Synthesised by a
planning council (`plan-a50aa93d`) for the [Daemon-Protection Observability
(DPO)](../modules/daemon-protection-observability.aps.md) module.

## Date

2026-06-20

## Context

Today only the **mid-edit** intercept path emits a `gate.evaluated` observation
to Kindling (`crates/anvil-intercept/src/kindling_observation.rs`,
`from_midedit_response`, gated `MIDEDIT_GATE_ID`). The **save-time** daemon
validation path (`validate_paths`, routed through the `SaveTimeDispatch` trait
in `crates/anvil-intercept/src/save_time.rs`) and **fence/cascade** engagement
(`crates/anvil-intercept/src/fence.rs`) emit nothing — they leave only live
tracing telemetry. DPO closes that producer-side gap so save-time verdicts and
fence events become durable governance facts on Kindling
([ADR-035](035-three-pipe-observability-rule.md) / D-035), consumed later by a
read surface and the deferred TDASH/TUIDASH dashboards.

The load-bearing decision is **how the two new event classes are typed** in the
Kindling stream, because the kind / `gate_id` namespace is queried by every
downstream consumer (the read surface filters by it; dashboards aggregate on it)
and changing it after rows exist is a breaking change. A planning council
(systems architect proposing, adversarial reviewer refuting) converged on a
**split**: a save-time verdict genuinely *is* a gate evaluation, but a fence
engage is *not* — it is a protective state transition, and modelling it as a
failed gate evaluation would pollute violation metrics.

Code facts the council relied on:

- `GateEvaluatedObservation` carries `inputs.changed_files`, `rules_evaluated`,
  `rules_violated`, `enforcement` (`blocking`/`warning`/`informational`) and an
  `outcome`. A fence engage has none of these (no rules, no file-level
  enforcement) — modelling it as `outcome: Fail` with empty `rules_evaluated`
  is an internally inconsistent row, and any query aggregating
  `gate.evaluated WHERE outcome = fail` would count fence lockouts as rule
  violations.
- `KindlingObservationSink` already sets the precedent for distinct kinds via a
  defaulted trait method: `try_emit_action_executed` emits `action_executed`,
  separate from `gate.evaluated`.
- `try_emit` is synchronous; its non-blocking property is delegated to the sink
  implementor (KDS, not yet written). `validate_paths` is on the
  [ADR-031](031-validation-latency-rubric.md) latency-gated path.

## Decision

**Decision 1 — Save-time verdicts use `gate.evaluated` with a pinned
`gate_id`.** A `validate_paths` verdict is a gate evaluation; reuse the existing
`gate.evaluated` kind, builder family, and read path, with a new pinned
`SAVE_TIME_GATE_ID` constant distinguishing it from `MIDEDIT_GATE_ID` and
`AUDIT_CHAIN_GATE_ID`. Missing fields are populated honestly (real verdict
outcome, real changed files subject to Decision 4).

**Decision 2 — Fence / cascade events use a distinct observation kind.**
Introduce a `constraint_applied` (working name) kind via a new defaulted
`KindlingObservationSink` method (mirroring `try_emit_action_executed`),
carrying `worktree`, normalised `reason`, `timestamp`, and a `cascade` flag.
Fences are **not** modelled as `gate.evaluated/Fail`. This keeps the
gate-failure dashboard and any `outcome = fail` aggregation free of non-gate
events, at the cost of a new payload type plus a KDS-side mapping and a Kindling
schema entry.

**Decision 3 — One registry for the kind / `gate_id` namespace.** The
`SAVE_TIME_GATE_ID` constant and the new fence kind are declared in one place
(`kindling_observation.rs`) and listed in this ADR; there is no compile-time
guard against duplicate `gate_id` strings, so the ADR is the human registry.

**Decision 4 — Config-gated path inclusion; fence `reason` always
normalised.** File paths in `changed_files` are gated by the
opt-in/local-only capture flag (off → no paths; on → full paths). The free-form
fence `reason` string is always normalised/redacted to a bounded vocabulary
before emit (operator-supplied text never lands verbatim).

## Consequences

- The read surface (DPO-003) filters by `gate_id` for save-time and by kind for
  fences; the two event classes never alias.
- Decision 2 obliges KDS to map the new kind in its sink and obliges the
  upstream Kindling schema to accept it — a coordination point flagged on the
  KDS module (KDS open-Q#4: whether `gate.evaluated` routes to the daemon now
  gains a sibling question for the fence kind).
- Non-blocking emission, fence emit-point/ordering, NDJSON retention, and the
  rate-cap are **implementation decisions recorded in the DPO module**, not this
  ADR; this ADR is deliberately scoped to the kind taxonomy (the
  expensive-to-reverse part).
- If the owner rejects Decision 2 in favour of reusing `gate.evaluated`, the
  fallback is a distinct `gate_id` (e.g. `daemon.fence-cascade`) plus a
  documented "exclude this `gate_id` before violation aggregation" contract —
  the council judged this fragile and recommends against it.

## References

- [DPO module](../modules/daemon-protection-observability.aps.md) — work items
  and the implementation-level Ready Checklist
- [ADR-035](035-three-pipe-observability-rule.md) — three-pipe rule (governance
  facts → Kindling)
- [ADR-031](031-validation-latency-rubric.md) — latency gate on the save-time path
- [ADR-064](064-intercept-graph-cache-crate-boundary.md) — daemon dependency
  boundary (the Kindling sink is minted in `anvil-cli`, never `anvil-intercept`)
- [KDS module](../modules/kindling-daemon-sink.aps.md) — the sink backend that
  must map the new kind
- Planning council session `plan-a50aa93d`
