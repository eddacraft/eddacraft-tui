# Notification Delivery Architecture

## Purpose

Design output for `NOTIFY-003`. This document defines how notifications move
from source producers to delivery sinks across current CLI/TUI surfaces and the
future daemon-era control plane.

It builds on:

- `2026-04-22-notification-framework-discovery.md`
- `2026-04-22-notification-taxonomy-and-priority-design.md`
- `plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md`

## Core Principle

Notification delivery must preserve the driver-framework split between:

- **control / enforcement lane**
- **telemetry / event lane**

The same source event may feed both lanes, but the payload shape and delivery
contract differ.

## Delivery Layers

### 1. Source layer

Producers emit raw domain outputs and state changes.

Examples:

- checks emitting findings
- gates producing overall judgement
- watch loops producing queue/history/status changes
- tutorial/welcome flows producing guidance and fallback states
- daemon/intercept producing enforcement decisions

### 2. Notification normalisation layer

A shared notification layer maps source outputs into canonical notification
classes with:

- class
- priority
- audience
- sink hints
- grouping keys
- acknowledgement requirements

This is the layer that should absorb current surface-specific wording drift.

### 3. Delivery adapters

Each sink has a thin adapter that renders or forwards canonical notifications.

Current / future adapters:

- CLI plain output
- CLI JSON output
- TUI local surfaces
- TUI watch/status/dashboard panels
- logs / observability subscribers
- future NDJSON / IPC telemetry subscribers
- future enforcement drivers for control-plane decisions

## Lane Split

### Control / enforcement lane

Use for decisions that can change runtime behaviour.

Examples:

- `warn`
- `block`
- `interrupt`
- fence application / revocation
- ack / nack flows

Requirements:

- bounded, low-latency, request-response or explicit-ack semantics
- driver-capability-aware routing
- no dependency on TUI rendering or human presence

### Telemetry / event lane

Use for delivery of notifications to humans and observers.

Examples:

- progress updates
- finding notifications
- watch queue/history updates
- tutorial notices
- daemon health events

Requirements:

- stream-friendly
- fan-out capable
- safe to consume by CLI, TUI, logs, and future dashboard subscribers

## Sink Architecture

### CLI plain output

Best for:

- single-session human operation
- immediate visibility during commands

Delivery rules:

- low-priority progress updates may be transient or compact
- normal/high-priority finding notifications print inline
- high/critical notifications should be visually explicit and hard to miss
- block/interrupt terms must not be softened into generic warnings

### CLI JSON output

Best for:

- machine consumers
- CI integrations
- PR tooling

Delivery rules:

- emit canonical notification class and priority metadata
- preserve structured source details and references
- do not collapse control-plane decisions into free-text-only errors

### TUI local surfaces

Best for:

- multi-panel, persistent, grouped human workflows

Delivery rules:

- allow grouped queues and history
- support sticky high/critical notifications
- support local filtering by class or priority later
- preserve separation between current-state status and historical notifications

### TUI watch/dashboard surfaces

Best for:

- continuous monitoring
- merged progress + finding + gate-status context

Delivery rules:

- watch queue/history should be treated as derived notification views over event
  streams, not a totally separate model
- high/critical notifications should override purely informational panel churn

### Logs / observability

Best for:

- debugging
- analytics
- audit trail supplementation

Delivery rules:

- receive telemetry-lane notifications only
- subscribe to control-plane decisions via mirrored event emission, not direct
  participation in the control path

### Future NDJSON / IPC subscribers

Best for:

- daemon-era TUI/UI/log subscribers
- external operator tooling

Delivery rules:

- telemetry lane should be streamable
- subscribers should receive canonical classes and priorities
- correlation fields should include session/worktree/source identifiers when
  available

## Delivery Matrix

| Notification class | CLI plain | CLI JSON | TUI | Telemetry stream | Control lane |
| --- | --- | --- | --- | --- | --- |
| informational update | yes | yes | yes | yes | no |
| progress update | yes | optional | yes | yes | no |
| finding notification | yes | yes | yes | yes | no |
| nudge | yes | yes | yes | yes | no |
| warning | yes | yes | yes | yes | sometimes mirrored from control |
| failure | yes | yes | yes | yes | no |
| block | yes | yes | yes | yes | yes |
| interrupt | yes | yes | yes | yes | yes |
| fence-state notification | yes | yes | yes | yes | yes |
| health / system notification | yes | yes | yes | yes | no |

## Grouping and Deduplication Rules

### Group by source and scope

Notifications should support grouping by:

- session
- worktree
- command surface
- file / plan / gate run id where relevant

### Deduplicate by semantic identity

Duplicate notifications should collapse when they represent the same underlying
state transition, not merely when the rendered text matches.

Examples:

- repeated watch-mode pass-rate updates are not distinct notifications
- repeated discovery fallback warnings for the same failed scan should dedupe
- the same fence-state should not re-announce on every poll tick

### Preserve transitions

Deduplication must not erase meaningful transitions such as:

- warn -> block
- block -> interrupt
- active -> fenced
- failing -> passing

## Acknowledgement Model

Acknowledgement should be reserved for control-plane actions and selected
critical notifications.

### No ack required

- informational update
- progress update
- most finding notifications
- most warnings in current warning-first surfaces

### Ack required or candidate

- block
- interrupt
- fence-state changes with active driver intervention
- future human-in-the-loop escalation prompts, if introduced

## Current Implementation Guidance

### Near-term current-surface guidance

- normalise terminal/TUI wording onto the taxonomy before adding more special
  cases
- keep watch queue/history as the de facto prototype of grouped delivery
- do not thread control-lane semantics through ad hoc CLI strings only

### Future daemon guidance

- route `allow` / `warn` / `block` / `interrupt` through the control lane
- mirror them onto the telemetry lane as notifications for subscribers
- keep ack handling out of general-purpose UI event delivery

## Open Questions For NOTIFY-004

1. Which current surfaces should adopt notification normalisation first?
2. Should CLI JSON gain a dedicated notification envelope or embed notification
   metadata into existing outputs incrementally?
3. Which TUI surfaces should share a common notification widget/model first?
4. How should watch history retention differ from generic notification history?

## Recommendation

Implement one shared notification normalisation layer with separate adapters for
CLI, TUI, JSON, and future daemon-era telemetry subscribers.

Keep control-plane enforcement decisions on their own lane, but mirror them into
telemetry as canonical notifications so operators and UIs see the same state
transitions the drivers act on.
