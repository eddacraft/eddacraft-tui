# Notification Telemetry Stream Contract

## Purpose

Design output for `NOTIFY-008`. This document defines the stream contract for
notification subscribers on the telemetry lane. It specifies the payload shape,
envelope metadata, and delivery semantics that logs, dashboards, and future
daemon-era consumers must agree on.

It builds on:

- `2026-04-22-notification-framework-discovery.md`
- `2026-04-22-notification-taxonomy-and-priority-design.md`
- `2026-04-22-notification-delivery-architecture.md`
- `2026-04-22-notification-execution-slices.md`
- `plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md`

## Scope

In scope:

- payload shape for notification events on the telemetry lane
- envelope metadata subscribers can rely on
- transport guidance that matches the driver-framework telemetry lane
  (NDJSON-or-equivalent, best-effort, lossy-tolerant)
- backwards-compatibility and evolution rules

Out of scope:

- the control-lane ack/request-response protocol (owned by the driver
  framework, `enforcement.*` family)
- specific storage or retention policies for observability sinks
- wire-level framing for a specific transport (stdout, unix socket,
  WebSocket) — the contract is payload-shape oriented

## Design Principles

1. **Canonical taxonomy on the wire.**
   Subscribers see canonical `NotificationClass` and `NotificationPriority`
   values. They must not need to parse surface-specific wording to understand
   what happened.
2. **Lossy-tolerant by default.**
   The telemetry lane may drop events under backpressure. Consumers must not
   treat the stream as a source of truth for state — it is a delivery channel,
   not a ledger.
3. **Correlation is mandatory, identity is optional.**
   Every event carries enough metadata to correlate it with a session,
   worktree, or run id. It does not have to carry stable user or host identity
   to be useful.
4. **Schema evolves forward, not sideways.**
   New fields are added with `skip_serializing_if = "Option::is_none"`.
   Existing fields are not removed or repurposed without a version bump.
5. **Control lane is mirrored, not merged.**
   `allow`/`warn`/`block`/`interrupt` decisions are mirrored onto this stream
   as notifications; they do not replace the enforcement-lane ack traffic.

## Event Envelope

Each telemetry event is a single JSON object on one line (NDJSON-compatible).
It has the following shape:

```json
{
  "schema": "anvil.notification.v1",
  "producer_instance_id": "pi_01HW3Q8F7P4X2K8TJ3ZQ9N7M0",
  "seq": 142,
  "timestamp": "2026-04-22T14:03:17.482Z",
  "correlation": {
    "session_id": "sess_01HW...",
    "worktree": "feat/notify",
    "run_id": "run_2026-04-22T14:03:17Z",
    "source": "watch"
  },
  "notification": {
    "class": "finding",
    "priority": "high",
    "title": "src/api/user.ts",
    "message": "cross-layer import detected",
    "context": {
      "file": "src/api/user.ts",
      "source": "watch"
    }
  },
  "grouping": {
    "key": "watch:src/api/user.ts",
    "transition": null
  },
  "mirror": null
}
```

### Top-level fields

| Field | Type | Required | Purpose |
| --- | --- | :---: | --- |
| `schema` | string | yes | Schema identifier + version. Current value: `anvil.notification.v1`. |
| `producer_instance_id` | string | yes | Opaque id unique to each producer process-lifetime (e.g. ULID generated at startup). Combined with `seq`, lets subscribers distinguish a producer restart (id changed, seq reset) from a backpressure-induced drop (id unchanged, seq gap). |
| `seq` | integer | yes | Monotonic per-producer-instance sequence number, starting at 1 when `producer_instance_id` is minted. Resets to 1 only when `producer_instance_id` changes. |
| `timestamp` | string | yes | RFC 3339 timestamp of event creation at the producer. |
| `correlation` | object | yes | Routing and attribution metadata. See below. |
| `notification` | object | yes | The canonical notification payload (same shape as `anvil_kernel_types::Notification`). |
| `grouping` | object | optional | Deduplication and transition hints. Omitted when not applicable. |
| `mirror` | object | optional | Control-lane mirror metadata when the event mirrors a driver decision. Null or omitted otherwise. |

### `correlation` sub-fields

| Field | Type | Required | Purpose |
| --- | --- | :---: | --- |
| `session_id` | string | optional | Driver-framework session id if the emitter runs inside a managed session. |
| `worktree` | string | optional | Worktree or branch identifier for routing. |
| `run_id` | string | optional | Gate/check run id for correlating multiple events from one invocation. |
| `source` | string | yes | Producer surface name (`check`, `gate`, `watch`, `doctor`, `audit`, `tutorial`, `onboarding-hooks`, `intercept`). Matches `notification.context.source`. |

### `notification` sub-fields

Exactly the `anvil_kernel_types::Notification` shape:

```
{
  "class": "info" | "progress" | "finding" | "nudge" | "warning"
         | "failure" | "block" | "interrupt" | "fence-state" | "health",
  "priority": "low" | "normal" | "high" | "critical",
  "title": string,
  "message": string,
  "context": { "file": string?, "source": string? }?
}
```

Subscribers must accept additional string values for future classes and
priorities; they should degrade unknown classes to `info` and unknown
priorities to `normal` rather than dropping the event.

### `grouping` sub-fields

| Field | Type | Required | Purpose |
| --- | --- | :---: | --- |
| `key` | string | optional | Stable deduplication key. Same value means same underlying transition. |
| `transition` | object | optional | Present when this event represents a meaningful state transition. See below. |

`transition` is `{ "from": "<class>", "to": "<class>" }` using canonical class
names. Producers should emit this for transitions listed in the delivery
architecture doc: `warn -> block`, `block -> interrupt`, `active -> fenced`,
`failing -> passing`. Deduplication consumers must preserve events with a
`transition` even when the `key` matches a prior suppressed event.

### `mirror` sub-fields

Present only when the notification mirrors a control-lane decision:

```json
"mirror": {
  "decision": "block",
  "driver": "intercept-daemon-v1",
  "ack_required": true,
  "control_correlation_id": "ctrl_01HW..."
}
```

| Field | Type | Required | Purpose |
| --- | --- | :---: | --- |
| `decision` | string | yes | One of `allow`, `warn`, `block`, `interrupt`. |
| `driver` | string | yes | Driver capability id that produced the decision. |
| `ack_required` | bool | yes | Whether the corresponding control-lane message requires an ack. |
| `control_correlation_id` | string | optional | Id that can be joined against control-lane traffic for analysis. |

## Subscriber Contract

### Subscribers MUST

- Accept events in arrival order, but not rely on strict per-source ordering.
- Handle `seq` gaps without crashing. A gap paired with an unchanged
  `producer_instance_id` is a backpressure-induced drop; a reset paired with
  a changed `producer_instance_id` is a producer restart. Subscribers MUST
  distinguish the two cases when reporting telemetry health.
- Render or route the event based on `notification.class` + `priority`, not
  on surface-specific strings.
- Treat `mirror != null` as "this is an enforcement event" and preserve the
  `decision` verbatim. They MUST NOT map `block`/`interrupt` into generic
  `error`/`warning` strings.

### Subscribers MAY

- Dedupe on `grouping.key`, unless `grouping.transition` is present.
- Retain history by `correlation.session_id` or `correlation.worktree`.
- Filter by `notification.source` for surface-specific views.
- Drop `info`/`progress` events if overloaded.

### Subscribers MUST NOT

- Infer control-lane ack state from telemetry alone.
- Use this stream as the source of truth for fence or session state.
- Require new fields that are not in the current schema.

## Producer Contract

### Producers MUST

- Set `schema` to the current version string.
- Mint a fresh `producer_instance_id` on each process start (ULID recommended)
  and include it on every emitted event. Never reuse an instance id across
  restarts.
- Emit `seq` starting at 1 and monotonically increasing per
  `producer_instance_id`.
- Populate `notification.context.source` and `correlation.source` with the
  same value.
- Mirror control-lane decisions when they happen in the same process, with
  `mirror.decision` matching the enforcement-lane outcome.

### Producers MAY

- Omit `correlation.session_id` when running outside a managed session.
- Omit `grouping` when the event is not a dedupable repeat.
- Batch events if the underlying transport supports it, provided each record
  remains a complete JSON object.

### Producers MUST NOT

- Collapse control-lane decisions into `message` strings without the `mirror`
  object.
- Emit events with unknown `class` values in the current schema. New classes
  require a schema bump.

## Backpressure and Loss

- Telemetry is **best-effort**. Producers should drop the oldest `low`/
  `normal` priority events first.
- `high` and `critical` events should be preserved over `low`/`normal` when
  shedding load.
- Mirrored control events (`mirror != null`) should be preserved above all
  other classes when shedding, because they document enforcement actions
  humans may need to investigate after the fact.

## Versioning

- Additive changes (new optional fields, new enum variants with known defaults)
  stay on `anvil.notification.v1`.
- Removing a field, changing a field type, or repurposing a field bumps the
  schema to `anvil.notification.v2`. Producers may emit both schemas during a
  transition window.

## Current and Future Producers

| Producer | Source value | Emits today? | Notes |
| --- | --- | :---: | --- |
| `anvil check` | `check` | yes | JSON output carries `notifications[]`. Add envelope wrapping when streamed. |
| `anvil gate` | `gate` | yes | JSON output carries `notifications[]`. Same wrapping. |
| `anvil doctor` | `doctor` | yes | JSON output carries `notifications[]`. Per-check notifications use `Warning` (Warn), `Failure` (Fail), `Info` (Skipped); Pass and Running are suppressed and represented only by the summary. Summary is `Health` on all-pass, `Warning` when only warnings, `Failure` when any fail. |
| `anvil audit` | `audit` | yes | JSON output carries `notifications[]`. Per-issue `Finding` class; summary is `Failure` (critical present), `Warning` (high/medium/low/info present), or `Info` (empty). Priorities cap at `High` — `Critical` is reserved for control-plane events. |
| TUI watch | `watch` | yes | In-process only. Add stream emission when watch exposes telemetry. |
| TUI tutorial | `tutorial` | yes | Exposes notifications via `NotificationSource`. |
| TUI onboarding | `onboarding-hooks` | yes | Exposes notifications via `NotificationSource`. |
| Intercept daemon | `intercept` | no (future) | Must emit with `mirror` set; see `NOTIFY-009`. |

## Privacy and Attribution

`correlation.worktree`, `correlation.source`, and `notification.title` are
descriptive strings and MAY carry project-sensitive content — branch names
routinely embed customer identifiers, CVE numbers, or unreleased feature
tags. Producers crossing an organisational trust boundary (exporting to a
third-party observability sink) SHOULD offer a `correlation_mode` option to
hash or drop `worktree`, and SHOULD NOT default to forwarding raw values to
external sinks.

`correlation.session_id` and `mirror.control_correlation_id` are opaque ids
minted by the producer and are safe to forward without hashing provided the
subscriber honours the producer's retention policy.

## Open Questions For Follow-on Work

1. Which transport do we ship first: stdout NDJSON, unix socket, or both
   behind a flag?
2. Do we need a standard `anvil.health.v1` event family for non-notification
   health telemetry, or does `class: "health"` cover it?

## Recommendation

Adopt `anvil.notification.v1` as the first stable notification telemetry
envelope. Wrap current CLI JSON `notifications[]` arrays with this envelope
when they are streamed rather than returned as a command's structured output,
and require the intercept daemon to produce events in this shape when control
integration lands in `NOTIFY-009`.
