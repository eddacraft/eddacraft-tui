<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Daemon-Protection Observability

| ID  | Owner      | Status   | Progress |
| --- | ---------- | -------- | -------- |
| DPO | @eddacraft | In Progress | 0/5   |

> **DRAFT** — authored via planning-workflow on 2026-06-20 (producer-first
> sequencing, new-module placement); design-gated by planning council
> `plan-a50aa93d` the same day. Sibling to
> [Kindling Daemon Sink](kindling-daemon-sink.aps.md) (KDS): KDS owns the
> **sink backend**; DPO owns **producer coverage + the read/dashboard
> surface**. The observation-kind taxonomy is decided in
> [ADR-088](../decisions/088-dpo-observation-kind-taxonomy.md) (**Accepted**
> 2026-06-20); the remaining design resolutions are in
> [Design decisions](#design-decisions) below. DPO-001/-002 are **In Progress**
> (`feat/dpo-producer-coverage`); DPO-003/-004/-005 stay Blocked on KDS.

## Cross-cutting convention

Observations are produced behind the existing `KindlingObservationSink` trait
(`crates/anvil-intercept/src/kindling_observation.rs`). This module **adds new
producers** (the save-time and fence emitters that today emit nothing) — it does
NOT change the sink backend (KDS owns that), the Kindling store, or the existing
mid-edit / `command.invoked` producers. Emission stays trait-only inside
`anvil-intercept`; no networking client is added to the daemon crate
(ADR-064 boundary preserved).

## Purpose

Today only the **mid-edit** intercept path emits a `gate.evaluated` observation.
The **save-time daemon** validation path (`validate_paths` in
`crates/anvil-intercept/src/ipc.rs`) and **fence/cascade** engagement
(`crates/anvil-intercept/src/fence.rs`) emit nothing to Kindling — they leave
only live tracing telemetry. As a result the developer's "am I protected?"
surface is live-only and terse: the `anvil status` Save-time line and
`anvil intercept status` show current state, but there is no durable record of
what was validated, what verdict was returned, or when a fence engaged.

Per [ADR-035](../decisions/035-three-pipe-observability-rule.md) (D-035, the
three-pipe rule) these are governance-shaped facts and belong on Kindling. This
module closes the **producer-side** gap so save-time verdicts and fence events
become durable governance observations, then surfaces them through a read
command and the dashboard views that TDASH/TUIDASH explicitly deferred "until
their data persists".

**Sequencing (producer-first):** DPO-001/-002 (the emitters) are independent of
KDS — they emit through the existing trait to whatever sink is active
(`usage.ndjson` today, the Kindling daemon once KDS lands). DPO-003/-004/-005
(read surface + dashboards) are **gated behind KDS** so they read from the
authoritative SQLite store and reuse the KDS-004 read path, rather than building
a throwaway reader against the NDJSON workaround KDS-005 retires.

## Design decisions

Resolved by planning council `plan-a50aa93d` (architect proposing, adversarial
reviewer refuting). Owner decisions: emit pass+fail rate-capped; ship both
emitters now; config-gated path inclusion; a short kind-taxonomy ADR.

- **Observation kinds (ADR-088):** save-time verdicts are `gate.evaluated` with
  a pinned `SAVE_TIME_GATE_ID`; fence/cascade events are a **distinct** kind
  (`constraint_applied`, working name) via a new defaulted
  `KindlingObservationSink` method — **not** `gate.evaluated/Fail` (which would
  count fence lockouts as rule violations).
- **Save-time emission seam:** thread the sink through `SaveTimeState` (not
  `SaveTimeConn`), emitting inside `validate_paths` on the `ANVIL_VALIDATE_PATHS`
  arm only. The KDS emit decision is decoupled from the correlation/session gate
  — a verdict + workspace root is enough; an unregistered cross-root session
  must not silently drop the row.
- **Non-blocking (ADR-031):** DPO owns a bounded, drop-on-full emission boundary
  in `anvil-intercept` so a slow/blocking sink can never back-pressure
  `validate_paths`; sink errors are logged and dropped, never propagated. DPO
  *requires* the KDS sink `try_emit` to enqueue-only onto a background bounded
  channel. The Ready gate is a CI latency check on `validate_paths` with an
  injected deliberately-slow sink — a tracing span alone is not enforcement.
- **Fence emit points:** emit on **every** fence engage (the single
  non-cascading fence is the most common and most important record); a cascade
  engage emits an additional `cascade`-flagged row. Fence rows are audit-grade,
  so emit **before** the fence-file persist (accept a rare duplicate on crash
  over silent loss; dedup is a KDS at-least-once follow-up). Inject the sink into
  `FenceStore` via `with_observation_sink` mirroring `with_telemetry`
  (ADR-064-clean), and key by `FenceStore`'s canonical-worktree/alias contract.
- **Rate-cap:** per-worktree `RateWindow` (≈20/60s); a **fail is always emitted**
  (never rate-dropped); a **pass is sampled at most once per window**. No
  eviction, no heartbeat row. "Validation stopped" liveness belongs to
  `AssuranceState` telemetry, not the DPO stream.
- **NDJSON retention:** while KDS is blocked, DPO bounds the sidecar at a 7-day
  max-age **and** a 64 MiB per-worktree size cap (lazy trim-on-append). KDS-005
  supersedes this when it retires the NDJSON writer — an explicit hand-off seam.

Residual risks carried into implementation: the non-blocking guarantee is only
as strong as the CI latency test plus KDS honouring enqueue-only; the
kind/`gate_id` namespace has no compile-time duplicate guard (ADR-088 is the
registry); fence emit-before-persist can produce a rare duplicate row on crash.

## In scope

- A save-time producer: each `validate_paths` verdict emitted as a
  `gate.evaluated` observation (a `gate_id` distinguishing save-time from
  `midEdit`), via a builder mirroring `from_midedit_response`, off the
  latency-critical path.
- A fence producer: fence engage / cascade transitions emitted as governance
  observations (kind resolved in the Ready Checklist — `constraint_applied`
  versus `gate.evaluated` with a fence `gate_id`) carrying worktree, reason, and
  timestamp.
- TRACE-003 redaction applied before emit; the privacy-first default preserved
  (capture stays opt-in / locally scoped, consistent with USAGE).
- A read surface for `gate.evaluated` rows (today only `command.invoked` is
  queryable via `anvil kindling usage`).
- The deferred TDASH watch-session / gate-summary dashboard and a TUIDASH
  save-time-protection component, as thin readers of the authoritative store.

## Out of scope

- The Kindling sink backend and the NDJSON→daemon store migration (KDS owns it).
- `validate_paths` verdict semantics or the DSV save-time validation flow (DSV
  owns it; DPO only adds emission at the call site).
- Mid-edit emission (already shipped) and the `command.invoked` producers (USAGE
  owns them).
- The Kindling daemon, its schema, or its storage (upstream, eddacraft/kindling).

## Interfaces

**Depends on:**

- The existing `KindlingObservationSink` trait + observation types in
  `anvil-intercept` (unchanged) — the emit seam.
- DSV (`validate_paths` save-time path) — the save-time emission call site.
- The intercept fence store (`fence.rs`) — the fence emission call site.
- USAGE (Done) — the producer convention, privacy contract, and TRACE-003
  redaction deny-list.
- KDS (Proposed) — the authoritative SQLite store the read surface and
  dashboards consume; DPO-003/-004/-005 depend on KDS reaching Ready/landing,
  DPO-001/-002 do not.
- [ADR-035](../decisions/035-three-pipe-observability-rule.md) / D-035 —
  governance facts route to Kindling.
- ADR-031 latency CI gate — save-time emission must not regress it.
- ADR-064 daemon dependency boundary — emission stays trait-only in
  `anvil-intercept`; the sink implementation stays in `anvil-cli`.
- [TUIDASH](../archive/modules/tui-dashboard-render.aps.md) (ADR-054) /
  [TDASH](../archive/modules/native-tui-dashboards.aps.md) — the dashboard
  consumers reopened as DPO-004/-005 follow-ups.

**Exposes:**

- Save-time and fence governance observations in the Kindling stream.
- A `gate.evaluated` read surface.
- Save-time protection dashboard views.

## Ready Checklist

- [x] Design/council pass on producer coverage — council `plan-a50aa93d`
      (2026-06-20)
- [x] Fence observation kind decided — distinct kind, not `gate.evaluated`
      (ADR-088 Decision 2)
- [x] Privacy approach decided — config-gated path inclusion + always-normalised
      fence `reason` (ADR-088 Decision 4)
- [x] ADR-088 ratified by the owner (Decision 2 accepted, 2026-06-20)
- [ ] Save-time emission verified non-blocking via a CI latency check on
      `validate_paths` with an injected slow sink (design decided; verification
      is a DPO-001 acceptance gate)
- [ ] KDS landed/Ready for the read-surface + dashboard items
      (DPO-003/-004/-005)

## Work Items

> DPO-001/-002 are In Progress (design-complete per ADR-088 + council
> `plan-a50aa93d`; they emit through the existing sink trait). DPO-003/-004/-005
> are Blocked on KDS per the producer-first sequencing decision.

### DPO-001: Emit save-time validation verdicts as `gate.evaluated`

- **Intent:** The save-time daemon validation path records each verdict as a
  durable Kindling governance observation.
- **Expected Outcome:** `validate_paths` emits a `gate.evaluated` observation
  per verdict with the pinned `SAVE_TIME_GATE_ID` (ADR-088 Decision 1), built via
  a new builder, with the sink threaded through `SaveTimeState` and emission
  decoupled from the correlation/session gate. Both pass and fail verdicts are
  emitted, rate-capped per worktree (`RateWindow` ≈20/60s) such that a fail is
  never dropped and a pass is sampled at most once per window. Emission is
  bounded drop-on-full so a slow sink cannot back-pressure `validate_paths`; sink
  errors are logged and dropped. File paths are config-gated (ADR-088 Decision
  4). `anvil-intercept` gains no networking dependency.
- **Validation:** unit tests asserting pass and fail verdicts each produce a
  `gate.evaluated` row carrying `SAVE_TIME_GATE_ID`; a rate-cap test confirming
  fails survive a saturated window while passes are sampled; a CI latency check
  on `validate_paths` with an **injected deliberately-slow sink** confirming the
  ADR-031 budget holds; the `daemon_dep_boundary` guard stays green.
- **Status:** In Progress
- **Files:** `crates/anvil-intercept/src/kindling_observation.rs`,
  `crates/anvil-intercept/src/save_time.rs`, `crates/anvil-intercept/src/ipc.rs`
- **Dependencies:** ADR-088 ratification (kind/`gate_id` taxonomy)
- **Confidence:** medium

### DPO-002: Emit fence / cascade engagement as governance observations

- **Intent:** Fence and cascade engagement become durable governance facts, not
  just live tracing signals.
- **Expected Outcome:** every fence engage emits a `constraint_applied`
  observation (ADR-088 Decision 2) carrying worktree, normalised `reason`, and
  timestamp; a cascade engage emits an additional `cascade`-flagged row. The sink
  is injected into `FenceStore` via `with_observation_sink` (mirroring
  `with_telemetry`, ADR-064-clean) and keyed by the existing canonical-worktree/
  alias contract. Rows emit **before** the fence-file persist (audit-grade,
  duplicate-tolerant over silent loss). The free-form `reason` is always
  normalised before emit; emission never propagates errors.
- **Validation:** unit tests asserting a single (non-cascading) fence engage
  produces one `constraint_applied` row, and a cascade engage adds a flagged row;
  a test asserting a normalised `reason` (no verbatim operator text); boundary
  guard green.
- **Status:** In Progress
- **Files:** `crates/anvil-intercept/src/fence.rs`,
  `crates/anvil-intercept/src/kindling_observation.rs`
- **Dependencies:** ADR-088 ratification (defines the `constraint_applied` kind);
  shares the kind decision with DPO-001
- **Confidence:** medium

### DPO-003: Read surface for `gate.evaluated` observations

- **Intent:** A developer can query the recorded save-time and fence governance
  facts.
- **Expected Outcome:** a read surface (an `anvil gate show` command or a new
  `anvil kindling usage` view) returns `gate.evaluated` and fence rows from the
  authoritative store, reusing the KDS-004 read path rather than the NDJSON
  workaround KDS-005 retires. View semantics and output shape are documented.
- **Validation:** the surface returns rows emitted by DPO-001/-002; output shape
  documented; the read path is the KDS-004 authoritative path, not a bespoke
  NDJSON reader.
- **Status:** Blocked
- **Files:** `crates/anvil-cli/src/commands/kindling.rs` (or a new command)
- **Dependencies:** DPO-001, DPO-002, KDS-004
- **Confidence:** low

### DPO-004: TDASH watch-session / gate-summary dashboard

- **Intent:** Surface save-time protection history in the native TUI dashboard
  TDASH deferred.
- **Expected Outcome:** the deferred TDASH watch-session / gate-summary
  dashboard renders save-time verdicts and fence events read from the
  authoritative store, following the existing `anvil dashboard` precedent.
- **Validation:** the dashboard renders recorded events; the TDASH dashboard
  test pattern is extended to cover the new view.
- **Status:** Blocked
- **Files:** `crates/anvil-tui`, `crates/anvil-cli`
- **Dependencies:** DPO-003, KDS
- **Confidence:** low

### DPO-005: TUIDASH save-time protection component

- **Intent:** A json-render domain component for save-time protection state.
- **Expected Outcome:** a TUIDASH domain component (alongside `GateResultCard` /
  `WarningList`) renders save-time assurance plus recent verdicts and fences from
  the store, with an example spec bundled like the gate-summary example.
- **Validation:** the component renders from a sample spec + recorded data,
  following the TUIDASH component test pattern.
- **Status:** Blocked
- **Files:** `crates/anvil-tui`, `packages/libs/render`
- **Dependencies:** DPO-003, KDS
- **Confidence:** low

## Risks

| Risk                                                              | Impact | Mitigation                                                                                          |
| ----------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------- |
| Save-time emission adds latency / regresses the ADR-031 gate      | High   | Bounded drop-on-full emission boundary in `anvil-intercept` (cannot back-pressure); CI latency check with an injected slow sink (DPO-001) |
| `constraint_applied` kind churns the Kindling schema (KDS + upstream) | Medium | Kind fixed in ADR-088; coordinate the KDS sink mapping + Kindling schema entry before DPO-002 ships  |
| Read surface built against NDJSON becomes throwaway under KDS-005 | Medium | Gate DPO-003/-004/-005 behind KDS-004; DPO bounds the sidecar (7-day / 64 MiB) until KDS-005 retires it |
| Save-time / fence observations name files (privacy)               | Low    | Config-gated path inclusion (ADR-088 Decision 4); fence `reason` always normalised; local-only default |
| Networking client leaks into `anvil-intercept` (ADR-064)          | High   | Emission stays trait-only; sink minted + implemented in `anvil-cli`; `daemon_dep_boundary` guard enforced |
| Sub-cascade fence emits nothing (audit-trail gap)                 | High   | Emit on **every** fence engage, not only cascade (council fix); cascade adds a flagged row            |

## Open questions

1. ~~**Fence observation kind**~~ — RESOLVED: a distinct `constraint_applied`
   kind, not `gate.evaluated/Fail` (ADR-088 Decision 2; council `plan-a50aa93d`).
2. ~~**Save-time emit placement**~~ — RESOLVED: an emitter threaded through
   `SaveTimeState`, emitting on the `ANVIL_VALIDATE_PATHS` arm, decoupled from
   the correlation/session gate (council `plan-a50aa93d`). A shared post-verdict
   hook is deferred until a second save-time producer exists.
3. **Read-surface shape** (DPO-003, Blocked) — a dedicated `anvil gate show`
   command versus a new `anvil kindling usage <view>`. Decide when KDS-004 lands.
4. **Coordination with KDS** — the new `constraint_applied` kind needs a KDS sink
   mapping and an upstream Kindling schema entry; and DPO makes `gate.evaluated`
   high-volume + dashboard-backing, which argues for daemon routing (KDS
   open-Q#4). Both should feed the KDS decision.
