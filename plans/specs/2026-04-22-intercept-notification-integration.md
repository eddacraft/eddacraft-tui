# Intercept Control Integration With Notification Model

## Purpose

Design output for `NOTIFY-009`. This document is the reference binding
between the intercept control lane and the shared notification model. It
captures the mapping, envelope, and sequencing rules so intercept/control
work reuses the canonical taxonomy rather than inventing parallel semantics.

It builds on:

- `plans/specs/2026-04-22-notification-taxonomy-and-priority-design.md`
- `plans/specs/2026-04-22-notification-delivery-architecture.md`
- `plans/specs/2026-04-22-notification-telemetry-stream-contract.md`
- `plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md`
- `plans/modules/intercept-daemon.aps.md` (INTD-013)

## Principle

The intercept daemon acts on the **control lane**. Its decisions
(`allow` / `warn` / `block` / `interrupt`) and fence state changes are the
authoritative control-plane output. To keep one mental model across Anvil,
the same decisions are **mirrored** onto the telemetry lane as canonical
notifications. Operators, TUIs, and subscribers consume the telemetry lane;
drivers and enforcement engines consume the control lane.

This split preserves two invariants from the notification taxonomy:

1. **Findings are not notifications** — findings come from checks, notifications
   are delivery artefacts of those findings and of control decisions.
2. **Control and telemetry lanes are separate** — control needs bounded,
   ack-driven semantics; telemetry needs stream-friendly, lossy-tolerant
   delivery.

## Fixed Mapping

Control-lane outcome to notification class/priority:

| Control decision | Notification class | Priority | Companion notification |
| --- | --- | --- | --- |
| `allow` | `info` (usually suppressed) | `low` | none |
| `warn` | `warning` | `high` | none |
| `block` | `block` | `critical` | `fence-state` if fence applied |
| `interrupt` | `interrupt` | `critical` | `fence-state` if fence applied; `health` if driver interaction failed |

Fence state transitions (independent of a specific decision):

| Transition | Notification class | Priority | Grouping transition field |
| --- | --- | --- | --- |
| `active -> fenced` | `fence-state` | `critical` | `{ "from": "active", "to": "fenced" }` |
| `fenced -> active` | `fence-state` | `normal` | `{ "from": "fenced", "to": "active" }` |

Daemon health events (no control decision):

| Event | Notification class | Priority |
| --- | --- | --- |
| daemon unreachable | `health` | `high` |
| driver capability missing for required decision | `health` | `high` |
| session attribution unknown | `health` | `normal` |

## Envelope

Intercept-produced events use the v1 envelope from
`2026-04-22-notification-telemetry-stream-contract.md` with these constraints:

- `correlation.source = "intercept"`.
- `correlation.session_id` and `correlation.worktree` are required whenever
  the daemon has enough context to populate them. Events that cannot be
  attributed use `correlation.source = "intercept"` and omit `session_id`.
- `mirror.decision` is set for every event that originates from a control
  decision. It MUST match the decision that went out on the control lane
  for the same event.
- `mirror.driver` identifies the driver capability id that produced the
  decision (`intercept-daemon-v1` for the built-in daemon; external drivers
  use their negotiated capability id).
- `mirror.ack_required` is `true` for `block` and `interrupt` mirrors, and
  `false` otherwise. It reflects the control-lane ack requirement, not a
  requirement on telemetry subscribers.
- `mirror.control_correlation_id` joins a telemetry event to the specific
  control-lane message that produced it, when one exists.

## Sequencing

The intent is a single, well-defined transition point so that subscribers
never observe a mirror for an action that was not actually delivered to a
driver, without serialising telemetry behind control-lane ack latency.

1. The intercept daemon makes a decision on the control lane.
2. The control-lane transport's `send()` call is invoked against the owning
   driver. The transport-level write is the **transition point**: after it
   returns `Ok`, the decision has been handed off to the driver even if the
   driver has not yet acknowledged. Before it returns, nothing was delivered.
3. The telemetry mirror is emitted **immediately after the transport `send()`
   returns `Ok`** — still before the driver's ack. Subscribers therefore
   cannot see a mirror for a decision that was never handed off.
4. The mirror MUST NOT wait for the control-lane ack. Telemetry delivery is
   best-effort and must not serialise behind control-lane latency.
5. If the control-lane `send()` returns `Err` (transport unreachable, driver
   missing, capability downgrade, etc.), the daemon MUST emit a `health`
   notification carrying the attempted decision context, and MUST NOT emit
   the mirror for the failed decision. See "Failed-send health events" below
   for the required shape.
6. If `send()` returns `Ok` but a later asynchronous write failure is
   detected (connection dropped mid-stream, driver process died), the daemon
   SHOULD emit a follow-up `health` notification referencing the already-
   emitted mirror via `mirror.control_correlation_id`, so subscribers can
   reconcile decisions that reached the transport boundary but not the
   driver.

## Failed-send health events

When step 5 fires, the `health` notification must preserve the same
attribution an operator would have seen if the mirror had been emitted.
This keeps audit trails actionable — an operator seeing a `health` event
must not have to correlate with the control lane to know what the daemon
was trying to do.

Required shape:

- `notification.class = "health"`, `priority = "high"`.
- `correlation.source = "intercept"` and the same `session_id` /
  `worktree` / `run_id` the mirror would have carried.
- `mirror` MUST be populated with:
  - `mirror.decision` = the attempted decision (`allow` / `warn` /
    `block` / `interrupt`).
  - `mirror.driver` = the target driver capability id.
  - `mirror.ack_required` = the ack requirement the decision would have
    carried.
  - `mirror.control_correlation_id` = the id the failed control-lane
    message was minted with, so a later successful retry can be joined
    against this health event.
- `notification.message` MUST name the attempted decision and target,
  e.g. `"block of src/api/user.ts (session sess_01HW...) failed: driver
  capability intercept-daemon-v1 unreachable"`.

This is the only case where a `health` notification carries a `mirror`
payload for a decision that did not reach the driver. Subscribers MUST
treat `class = "health"` with a populated `mirror` as an attempted-but-
undelivered decision, not as a successful enforcement event.

## Dedup and Transitions

- Repeated `allow` decisions for the same file in the same session should be
  suppressed on the telemetry lane entirely, or deduped with
  `grouping.key = "intercept:allow:<session_id>:<file>"`.
- Repeated `warn` decisions with identical rule + file + session should use
  the same `grouping.key` and no `grouping.transition`, so dashboards can
  collapse repeats.
- `block`, `interrupt`, and `fence-state` transitions MUST populate
  `grouping.transition` when representing a change of state, even if a prior
  event with the same key was suppressed. This preserves the
  "deduplication must not erase meaningful transitions" rule from the
  delivery architecture doc.

## Acknowledgement

Ack semantics remain entirely on the control lane. Telemetry subscribers do
not ack mirror events, and the daemon does not treat absence of a telemetry
consumer as an enforcement problem. If a human-in-the-loop ack flow is
introduced later, it lands as a control-lane extension; the notification
model only represents the resulting decisions.

## Validation

An intercept implementation satisfies this integration when:

- Every control-lane decision that passes the transport boundary is mirrored
  as a telemetry notification with the mapping above.
- The emitted event validates against `anvil.notification.v1`, including the
  `producer_instance_id` + `seq` pairing.
- Fence transitions are distinguishable from repeated fence notifications via
  `grouping.transition`.
- Failed `send()` invocations emit `class = "health"` notifications with a
  populated `mirror` payload per "Failed-send health events" above, and do
  not emit a success mirror for the same decision.
- Tests cover each row of the mapping table, a representative failed-send
  event, and a fence transition (see INTD-013).

## Recommendation

Freeze this mapping as the contract intercept work must honour. Any future
enforcement outcome that does not fit `allow`/`warn`/`block`/`interrupt` plus
`fence-state`/`health` must either be represented through those classes or
trigger a schema bump (`anvil.notification.v2`) with a new class defined on
the notification-framework side first.
