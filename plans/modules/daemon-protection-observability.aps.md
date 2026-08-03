<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Daemon-Protection Observability

| ID  | Owner      | Status   | Progress |
| --- | ---------- | -------- | -------- |
| DPO | @eddacraft | In Progress | 2/6   |

> **DRAFT** — authored via planning-workflow on 2026-06-20 (producer-first
> sequencing, new-module placement); design-gated by planning council
> `plan-a50aa93d` the same day. The archived
> [Kindling Daemon Sink](../archive/modules/kindling-daemon-sink.aps.md) (KDS)
> proved a command-only daemon/spool path; it is a merged precursor, not the
> active owner or blocker for this module. KFIT now owns the typed sink,
> durable-store migration, and query foundation; DPO retains the dashboard
> consumer ownership. The observation-kind taxonomy is decided in
> [ADR-088](../decisions/088-dpo-observation-kind-taxonomy.md) (**Accepted**
> 2026-06-20); the remaining design resolutions are in
> [Design decisions](#design-decisions) below. DPO-001/-002 are **Merged**
> (2026-06-20 via #2833) producer seams, not proof of durable admission.
> DPO-003/-004/-005 remain blocked on the relevant KFIT foundation and DPO
> ordering described below.

## Cross-cutting convention

Observations are produced behind the existing `KindlingObservationSink` trait
(`crates/anvil-intercept/src/kindling_observation.rs`). This module **adds new
producers** (the save-time and fence emitters that previously emitted nothing).
It does not own transport or storage. Emission stays trait-only inside
`anvil-intercept`; the runtime and sink implementation stay in `anvil-cli`, and
no networking client is added to the daemon crate (ADR-064 boundary preserved).

## Purpose

When DPO was planned, only the **mid-edit** intercept path emitted a
`gate_evaluated` observation. The **save-time daemon** validation path
(`validate_paths` in `crates/anvil-intercept/src/ipc.rs`) and **fence/cascade**
engagement (`crates/anvil-intercept/src/fence.rs`) left only live tracing
telemetry. DPO-001/-002 subsequently merged the missing producer seams. The
developer's "am I protected?" surface remains incomplete because the current
sink landscape does not yet provide one admitted, durable record of what was
validated, what verdict was returned, or when a fence engaged.

Per [ADR-035](../decisions/035-three-pipe-observability-rule.md) (D-035, the
three-pipe rule) selected governance-shaped facts belong on Kindling. This
module closed the **producer-side** seam for save-time verdicts and fence
events, but the merged producer code does not by itself prove durable storage:
current production paths still include legacy NDJSON, no-op, and command-only
gaps. KFIT owns the cutover to admitted, queryable evidence; DPO consumes that
foundation through the read semantics and dashboard views that TDASH/TUIDASH
deferred "until their data persists".

**Sequencing (producer-first, reconciled by KFIT-001):** DPO-001/-002 are merged
producer seams behind the transport-free trait. KFIT-007 makes selected events
durably admissible through one typed sink, KFIT-009 migrates and retires the
parallel sidecars, and KFIT-010 supplies the governance query and status
foundation. DPO-003 consumes that foundation; DPO-004/-005 follow DPO-003 and
remain DPO-owned dashboard work. The archived KDS command-only path is evidence
for this sequence, not an active dependency.

## Design decisions

Resolved by planning council `plan-a50aa93d` (architect proposing, adversarial
reviewer refuting). Owner decisions: emit pass+fail rate-capped; ship both
emitters now; config-gated path inclusion; a short kind-taxonomy ADR.

- **Observation kinds (ADR-088, as amended by ADR-116):** save-time verdicts
  use canonical wire kind `gate_evaluated` with a pinned `SAVE_TIME_GATE_ID`;
  ADR-088's `gate.evaluated` spelling is a migration/dual-read alias only.
  Fence/cascade events are a **distinct** kind
  (`constraint_applied`) via a new defaulted
  `KindlingObservationSink` method — **not** `gate_evaluated/Fail` (which would
  count fence lockouts as rule violations).
- **Save-time emission seam:** thread the sink through `SaveTimeState` (not
  `SaveTimeConn`), emitting inside `validate_paths` on the `ANVIL_VALIDATE_PATHS`
  arm only. The merged producer may build a candidate from its verdict and raw
  workspace context, but that path is not admission authority. KFIT-008 must
  resolve an owner-authorised, daemon-registered repository/worktree identity
  before KFIT-007 admits it. An unregistered, ambiguous, symlinked, or
  cross-root candidate is not admitted under the raw path and must update the
  crash-durable unknown-scope recording-gap ledger rather than disappear.
- **Bounded admission (ADR-031):** DPO's merged producer boundary uses a bounded
  channel so a slow sink cannot back-pressure `validate_paths`. Under the KFIT
  contract, however, drop-on-full is not an audit record: a selected event
  counts as recorded only after Kindling or the bounded outage spool accepts
  it. Admission failure never changes the Anvil verdict, but KFIT-007 must
  expose it as `recording_gap` health and query evidence. The CI latency check
  on `validate_paths` with an injected deliberately-slow sink remains the
  save-time budget gate.
- **Fence emit points:** emit a `constraint_applied` candidate on every fence
  engage; a cascade engage emits an additional `cascade`-flagged candidate.
  Emit-before-persist remains the merged, duplicate-tolerant ordering, but a
  candidate becomes durable governance evidence only after Kindling or the
  bounded spool admits it. Inject the sink into `FenceStore` via
  `with_observation_sink` mirroring `with_telemetry` (ADR-064-clean), and key by
  `FenceStore`'s canonical-worktree/alias contract.
- **Selection and rate:** warnings/failures and explicit audit/pre-push
  `gate_evaluated` outcomes, plus `constraint_applied`, `action_executed`, and
  `false_positive_reported`, are selected for durable admission. More exactly,
  every gate `fail`/`error` row is retained; `pass` and `skipped` are retained
  only for explicitly invoked audit/pre-push modes. Enforcement describes the
  configured action on failure and does not independently select a row; existing
  valid combinations such as `pass + blocking` remain compatible.
  Routine successful or skipped high-frequency mid-edit/save-time/pre-commit
  checks remain tracing-only.
  The merged per-worktree `RateWindow` (approximately 20/60s) is historical
  producer behaviour, not the terminal retention or admission contract.
- **Legacy NDJSON:** the existing sidecar is bounded at a 7-day max-age and
  64 MiB per worktree (lazy trim-on-append), but it is not the terminal
  governance source of truth. KFIT-009 owns explicit migration and retirement;
  KFIT's outage spool retains the same 7-day/64 MiB bound.

Residual risks carried into implementation: the latency guarantee is only as
strong as the CI test plus bounded local admission; the kind/`gate_id`
namespace has no compile-time duplicate guard (ADR-088 is the registry);
fence emit-before-persist can produce a rare duplicate candidate on crash; and
KFIT-007 must make every admission failure visible without changing verdicts.

## In scope

- A save-time producer seam for `gate_evaluated` candidates (a `gate_id`
  distinguishing save-time from `midEdit`), via a builder mirroring
  `from_midedit_response`, off the latency-critical path. KFIT-007 applies the
  selected-event durable-admission policy; routine successful high-frequency
  checks remain tracing-only.
- A fence producer: fence engage / cascade transitions emitted as
  `constraint_applied` governance observations carrying worktree, reason, and
  timestamp; fences are never represented as `gate_evaluated/Fail`.
- TRACE-003 redaction applied before emit; the privacy-first default preserved
  (capture stays opt-in / locally scoped, consistent with USAGE).
- DPO-owned governance-history semantics over the KFIT-010 query foundation
  (today only the command-usage path is first-class).
- The deferred TDASH watch-session / gate-summary dashboard and a TUIDASH
  save-time-protection component, as thin readers of the authoritative store.

## Out of scope

- The Kindling runtime, typed sink, and NDJSON-to-store migration (KFIT-007 and
  KFIT-009 own them).
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
- KFIT-007 — the typed sink and selected-event durable-admission contract,
  including visible `recording_gap` evidence.
- KFIT-009 — explicit sidecar migration and retirement of parallel writers.
- KFIT-010 — the governance query and operational-status foundation consumed by
  DPO-003; DPO-004/-005 follow DPO-003.
- Archived KDS — historical proof of the command-only daemon/spool precursor,
  not an active dependency.
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
- A `gate_evaluated` read surface.
- Save-time protection dashboard views.

## Ready Checklist

- [x] Design/council pass on producer coverage — council `plan-a50aa93d`
      (2026-06-20)
- [x] Fence observation kind decided — distinct kind, not `gate_evaluated`
      (ADR-088 Decision 2)
- [x] Privacy approach decided — config-gated path inclusion + always-normalised
      fence `reason` (ADR-088 Decision 4)
- [x] ADR-088 ratified by the owner (Decision 2 accepted, 2026-06-20)
- [x] Save-time producer seam verified bounded via a CI latency check on
      `validate_paths` with an injected slow sink (DPO-001, merged via #2833)
- [ ] KFIT-007 typed sink and durable admission landed for selected governance
      evidence
- [ ] KFIT-009 migration/cutover and KFIT-010 query foundation landed before
      DPO-003; DPO-004/-005 remain ordered after DPO-003

## Work Items

> DPO-001/-002 are Merged (2026-06-20 via #2833; implemented per ADR-088 +
> council `plan-a50aa93d`, emitting candidates through the existing sink
> trait). They establish producer coverage, not durable-store completion.
> DPO-003 is blocked on KFIT-007/-009/-010; DPO-004/-005 are ordered after
> DPO-003. The archived KDS module is not an active blocker.

### DPO-001: Emit save-time validation verdicts as `gate_evaluated`

- **Intent:** The save-time daemon validation path emits each verdict as a
  typed governance observation candidate without crossing the ADR-064 transport
  boundary.
- **Expected Outcome:** `validate_paths` emits a `gate_evaluated` observation
  per verdict with the pinned `SAVE_TIME_GATE_ID` (ADR-088 Decision 1), built via
  a new builder, with the sink threaded through `SaveTimeState` and emission
  decoupled from the correlation/session gate. Both pass and fail verdicts are
  emitted by the merged producer seam, rate-capped per worktree (`RateWindow`
  approximately 20/60s) such that a fail is never rate-dropped and a pass is
  sampled at most once per window. The seam is bounded so a slow sink cannot
  back-pressure `validate_paths`; file paths are config-gated (ADR-088 Decision
  4), and `anvil-intercept` gains no networking dependency. This historical
  producer contract is not evidence that a row is durably recorded: KFIT-007
  narrows durable retention to selected outcomes, routes routine successful
  high-frequency checks to tracing only, and records downstream admission
  failures as visible `recording_gap` evidence without changing the verdict.
- **Validation:** unit tests asserting pass and fail verdicts each produce a
  `gate_evaluated` row carrying `SAVE_TIME_GATE_ID`; a rate-cap test confirming
  fails survive a saturated window while passes are sampled; a CI latency check
  on `validate_paths` with an **injected deliberately-slow sink** confirming the
  ADR-031 budget holds; the `daemon_dep_boundary` guard stays green.
- **Status:** Merged 2026-06-20 via PR #2833
- **Files:** `crates/anvil-intercept/src/kindling_observation.rs`,
  `crates/anvil-intercept/src/save_time.rs`, `crates/anvil-intercept/src/ipc.rs`
- **Dependencies:** ADR-088 ratification (kind/`gate_id` taxonomy)
- **Confidence:** medium

### DPO-002: Emit fence / cascade engagement as governance observations

- **Intent:** Fence and cascade engagement produce typed governance candidates,
  not just live tracing signals.
- **Expected Outcome:** every fence engage emits a `constraint_applied`
  observation (ADR-088 Decision 2) carrying worktree, normalised `reason`, and
  timestamp; a cascade engage emits an additional `cascade`-flagged row. The sink
  is injected into `FenceStore` via `with_observation_sink` (mirroring
  `with_telemetry`, ADR-064-clean) and keyed by the existing canonical-worktree/
  alias contract. Candidates emit **before** the fence-file persist
  (duplicate-tolerant over silent loss). The free-form `reason` is always
  normalised before emit; emission never changes the fence verdict. Under
  KFIT-007, the event counts as recorded only after Kindling or the bounded
  spool accepts it; failure becomes visible `recording_gap` evidence.
- **Validation:** unit tests asserting a single (non-cascading) fence engage
  produces one `constraint_applied` row, and a cascade engage adds a flagged row;
  a test asserting a normalised `reason` (no verbatim operator text); boundary
  guard green.
- **Status:** Merged 2026-06-20 via PR #2833
- **Files:** `crates/anvil-intercept/src/fence.rs`,
  `crates/anvil-intercept/src/kindling_observation.rs`
- **Dependencies:** ADR-088 ratification (defines the `constraint_applied` kind);
  shares the kind decision with DPO-001
- **Confidence:** medium

### DPO-003: Read surface for `gate_evaluated` observations

- **Intent:** A developer can query the admitted save-time and fence governance
  facts through DPO-owned view semantics.
- **Expected Outcome:** a read surface (an `anvil gate show` command or a
  governance-history view) consumes the KFIT-010 bounded query foundation and
  returns admitted `gate_evaluated` and `constraint_applied` evidence from the
  canonical Kindling store. It does not build a bespoke NDJSON reader. View
  semantics and output shape are documented, including `recording_gap`
  visibility supplied by KFIT-007/-010.
- **Validation:** the surface returns rows produced by DPO-001/-002 after
  durable admission; output shape is documented; the read path uses KFIT-010,
  not a bespoke NDJSON reader; admission gaps remain distinguishable from
  recorded evidence.
- **Status:** Blocked
- **Files:** `crates/anvil-cli/src/commands/kindling.rs` (or a new command)
- **Dependencies:** DPO-001, DPO-002, KFIT-007, KFIT-009, KFIT-010
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
- **Dependencies:** DPO-003, KFIT-010
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
- **Dependencies:** DPO-003, KFIT-010
- **Confidence:** low

### DPO-006: Producer observability hardening (council MINORs)

- **Intent:** Close the producer-side observability gaps surfaced as council
  MINORs during DPO-001/-002 implementation (PR #2833).
- **Expected Outcome:** (a) emit an `Outcome::Error` row on the
  `validate_paths` `Err` path so sustained client errors are visible in the
  observation stream; (b) enforce a per-path length cap on `changed_files` when
  `ANVIL_OBSERVATION_INCLUDE_PATHS=1`; (c) surface
  `NonBlockingObservationSink::dropped_count()` in `intercept status` via an
  additive `DaemonStatusV1` field (mirroring `telemetry_dropped_envelopes`); (d)
  an IPC-level integration test for the save-time emit path through
  `handle_save_time_jsonrpc` (unit coverage is strong; the wiring is untested).
- **Validation:** tests for each sub-outcome; `cargo test -p eddacraft-anvil-intercept
  -p eddacraft-anvil` green.
- **Status:** Proposed
- **Files:** `crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-intercept/src/kindling_observation.rs`,
  `crates/anvil-cli/src/commands/intercept.rs`
- **Dependencies:** DPO-001, DPO-002
- **Confidence:** high

## Implementation notes

DPO-001 + DPO-002 producer seams were implemented on
`feat/dpo-producer-coverage` (Option C), Merged 2026-06-20 via PR #2833. Their
legacy sink wiring is not durable-governance end-to-end evidence: current
production paths still include NDJSON, no-op, and command-only gaps. The module
stays In Progress until KFIT supplies the admitted-store/query foundation and
DPO-003/-004/-005 land in order.

- **Producers** (`anvil-intercept`): `from_validate_paths` + `SaveTimeObservationEmitter`
  (fail always emitted, pass sampled per-worktree via `RateWindow`), emitted at
  the `ANVIL_VALIDATE_PATHS` arm; `constraint_applied` kind + `from_fence`
  emitted on every fence engage (cascade flagged), before persist.
- **Bounded producer boundary** (council T2 / ADR-031):
  `NonBlockingObservationSink` — bounded channel + one background drain thread
  owning the inner sink; `try_emit*` is a non-blocking `try_send` with a
  rate-limited warning on saturation. Verified by a slow-sink latency test.
  `std` thread + `mpsc` only (ADR-064). This protects latency but is not the
  KFIT durable-admission proof; KFIT-007 must surface rejected selected events
  as `recording_gap` evidence.
- **Activation** (`anvil-cli`): `DaemonObservationSink` persists save-time
  `gate_evaluated` rows (wire `kind`; gate-id-filtered) + `constraint_applied`
  to legacy `usage.ndjson`, **extending the USAGE-004 sink contract**, with
  7-day/64 MiB lazy trim-on-append retention. That sidecar is neither the
  canonical governance store nor an outage-admission receipt; KFIT-009 owns its
  explicit migration and retirement.
- **Privacy**: file paths (save-time `changed_files`, fence `worktree`) gated
  off by default behind `ANVIL_OBSERVATION_INCLUDE_PATHS`; fence `reason`
  always normalised.
- **Operability**: `ANVIL_INTERCEPT_DISABLE_OBSERVATION` kill-switch;
  `ANVIL_USAGE_SIDECAR_NO_TRIM` retention escape hatch; dropped-row warns.
- **Validation**: `cargo test -p eddacraft-anvil-intercept -p eddacraft-anvil`
  green, `clippy --all-targets -D warnings` clean, `fmt --check` clean; design
  reviewed by council `plan-a50aa93d` (kernel/adversarial/operations), MAJOR
  findings addressed.
- **DPO-006** (Proposed) tracks the council MINOR producer hardening follow-ups
  from PR #2833 — see the work item above.

## Risks

| Risk                                                              | Impact | Mitigation                                                                                          |
| ----------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------------------------- |
| Save-time admission adds latency / regresses the ADR-031 gate     | High   | Keep the transport-free bounded producer seam; KFIT-007 benchmarks bounded local Kindling/spool admission and exposes `recording_gap` without changing verdicts |
| `constraint_applied` mapping drifts from ADR-088 taxonomy         | Medium | Keep ADR-088's distinct kind; KFIT-007 maps it into the Rust-authoritative `anvil.governance.v1` envelope |
| Read surface is coupled to a legacy sidecar                       | Medium | Block DPO-003 on KFIT-007/-009/-010 and forbid a bespoke NDJSON reader; DPO-004/-005 follow DPO-003 |
| Save-time / fence observations name files (privacy)               | Low    | Config-gated path inclusion (ADR-088 Decision 4); fence `reason` always normalised; local-only default |
| Networking client leaks into `anvil-intercept` (ADR-064)          | High   | Emission stays trait-only; sink minted + implemented in `anvil-cli`; `daemon_dep_boundary` guard enforced |
| Sub-cascade fence emits nothing (audit-trail gap)                 | High   | Emit on **every** fence engage, not only cascade (council fix); cascade adds a flagged row            |

## Open questions

1. ~~**Fence observation kind**~~ — RESOLVED: a distinct `constraint_applied`
   kind, not `gate_evaluated/Fail` (ADR-088 Decision 2; council `plan-a50aa93d`).
2. ~~**Save-time emit placement**~~ — RESOLVED: an emitter threaded through
   `SaveTimeState`, emitting on the `ANVIL_VALIDATE_PATHS` arm, decoupled from
   the correlation/session gate (council `plan-a50aa93d`). A shared post-verdict
   hook is deferred until a second save-time producer exists.
3. **Read-surface shape** (DPO-003, Blocked) — a dedicated `anvil gate show`
   command versus a governance-history view. Decide against the bounded
   KFIT-010 query contract; do not extend the legacy usage-sidecar reader.
4. ~~**Coordination with KDS**~~ — RESOLVED by KFIT-001: KDS is an archived
   command-only precursor. KFIT-007 owns typed durable admission,
   `recording_gap`, and the `constraint_applied` mapping; KFIT-009 owns sidecar
   migration; KFIT-010 owns the query/status foundation. DPO retains the
   dashboard consumer work.
