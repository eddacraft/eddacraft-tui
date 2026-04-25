# Notification Taxonomy and Priority Design

## Purpose

Design output for `NOTIFY-002`. This document defines the canonical taxonomy
and priority model for notifications in Anvil, grounded in the `NOTIFY-001`
inventory and aligned with the driver-framework control vs telemetry split.

This is not a UI-only taxonomy. It must work for:

- CLI output
- TUI surfaces
- machine-readable output
- future daemon / IPC streams
- future enforcement decisions and acks

## Design Constraints

This taxonomy must respect four constraints.

1. **Findings are not notifications.**
   Findings are domain results emitted by checks. Notifications are delivery and
   escalation artefacts.
2. **Control and telemetry are separate.**
   Driver-framework control decisions (`allow`, `warn`, `block`, `interrupt`)
   must not be conflated with general event delivery.
3. **Warning-first remains the default.**
   Until intercept is explicitly active for a surface, the product should not
   silently adopt block/interrupt semantics.
4. **One taxonomy, multiple sinks.**
   The same event class should be renderable in terminal, TUI, JSON, and future
   IPC channels.

## Model Overview

The notification model has three layers.

1. **Source layer**
   Checks, gates, watch state, tutorials, daemon status, and other runtime
   producers emit domain results and state changes.
2. **Notification layer**
   Anvil wraps those into canonical notification classes with priority and
   routing metadata.
3. **Sink layer**
   CLI, TUI, JSON output, logs, and future IPC subscribers render or act on the
   notifications.

## Canonical Notification Classes

### 1. Informational update

Used for low-urgency state or guidance that does not imply action.

Examples:

- onboarding completion messages
- tutorial explanatory copy
- passive status summaries

### 2. Progress update

Used for transient execution progress.

Examples:

- gate `--progress` lines
- watch scanning/running states
- loading transitions in welcome/tutorial flows

### 3. Finding notification

Used to deliver one or more findings from checks.

Examples:

- anti-pattern finding in `anvil check`
- boundary violation in gate output
- policy failure details

This is the generic class for human-facing delivery of findings.

### 4. Nudge

Used for advisory coaching that is less severe than a warning and more
actionable than passive info.

Examples:

- future interactive `anvil check` coaching
- contextual remediation hints

### 5. Warning

Used when the user should notice and may need to act, but the system is not yet
preventing progress on this surface.

Examples:

- non-blocking finding summary
- fallback warning when discovery scan fails
- warn-level enforcement decision in a weak-capability driver

### 6. Failure

Used when a surface-specific operation failed or a gate failed, but this is not
yet the same thing as a block or interrupt command.

Examples:

- gate failed
- policy evaluation failed
- tutorial verification failed

### 7. Block

Used for control-plane decisions that deny future actions or writes.

This is primarily a driver/intercept concept, but the taxonomy must reserve it
now so current surfaces do not invent competing terms later.

### 8. Interrupt

Used for control-plane decisions that actively stop or attempt to stop the
relevant session.

This is stronger than `block` and always belongs to the enforcement lane.

### 9. Fence-state notification

Used to communicate that a session, worktree, repo, or capability is fenced.

Examples:

- blocked worktree
- revoked session lease
- capability revocation status

### 10. Health / system notification

Used for daemon, session, driver, or framework health.

Examples:

- daemon health degraded
- no driver available for interrupt
- session attribution unknown

## Priority Model

Use two orthogonal dimensions:

1. **Class** — what kind of notification this is
2. **Priority** — how urgently it should be surfaced

### Priority levels

| Priority | Meaning | Typical behaviour |
| --- | --- | --- |
| `low` | Background or explanatory | can be grouped, collapsed, or delayed |
| `normal` | User should see it in context | show in the current surface |
| `high` | User should notice now | elevate in current surface, sticky until seen |
| `critical` | Workflow or enforcement urgency | prominent delivery, possible ack requirement |

### Recommended defaults

| Notification class | Default priority |
| --- | --- |
| informational update | low |
| progress update | low |
| finding notification | normal |
| nudge | normal |
| warning | high |
| failure | high |
| block | critical |
| interrupt | critical |
| fence-state notification | critical |
| health / system notification | normal or high depending on impact |

## Relationship To Driver-Framework Decisions

Driver-framework defines the stable enforcement decision contract:

- `allow`
- `warn`
- `block`
- `interrupt`

These are **control-plane decisions**, not the full notification taxonomy.

Mapping:

| Control decision | Notification mapping |
| --- | --- |
| `allow` | usually no notification, or low-priority informational/progress update |
| `warn` | warning notification |
| `block` | block notification + possible fence-state notification |
| `interrupt` | interrupt notification + possible fence-state / health follow-up |

This keeps enforcement semantics stable while allowing richer delivery on the
telemetry side.

## Sink Rules

### CLI plain output

- prioritise readability and sequencing
- progress updates should remain transient or compact
- warnings/failures should be visible inline
- block/interrupt language should be explicit and unambiguous

### TUI surfaces

- support grouping, queueing, history, and sticky high-priority items
- allow multiple concurrent low-priority notifications without overwhelming the
  operator
- reserve prominent surface regions for high/critical notifications

### JSON / machine-readable output

- preserve class, priority, source, and actionable fields
- do not flatten block/interrupt into generic error strings

### Future IPC / NDJSON

- separate control-plane ack traffic from telemetry/event streams
- event subscribers should receive canonical notification classes rather than
  surface-specific wording

## Taxonomy Mapping For Current Sources

| Current source | Primary notification class |
| --- | --- |
| gate progress lines | progress update |
| gate per-check result lines | finding notification / failure |
| check warning output | finding notification |
| doctor diagnostics | finding notification or health/system notification |
| audit issues | finding notification |
| watch queue/history/status | progress update, warning, failure, health/system notification |
| tutorial notices/overlays | informational update, nudge, warning |
| welcome transitional copy | informational update, warning |

## Open Questions For NOTIFY-003

1. Which notifications require acknowledgement, and on which surfaces?
2. How should grouped findings differ from grouped notifications?
3. Should watch-mode history be represented as stored notifications or a
   derived view over events?
4. How much of the current TUI dirty-state/event loop becomes the canonical
   delivery substrate?

## Recommendation

Adopt this taxonomy as the stable vocabulary for notification delivery, while
keeping findings and enforcement decisions as separate adjacent models.

That gives Anvil:

- one way to talk about delivery
- one way to talk about enforcement
- one way to talk about domain results

without collapsing them into a single overloaded concept.
