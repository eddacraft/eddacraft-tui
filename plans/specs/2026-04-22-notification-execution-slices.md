# Notification Execution Slices

## Purpose

Design output for `NOTIFY-004`. This document converts the notification
discovery, taxonomy, and delivery architecture into bounded implementation
slices.

Inputs:

- `2026-04-22-notification-framework-discovery.md`
- `2026-04-22-notification-taxonomy-and-priority-design.md`
- `2026-04-22-notification-delivery-architecture.md`

## Slicing Principles

1. Do not attempt one global notification rewrite across every surface.
2. Normalise the shared model before introducing daemon-era transport.
3. Start with the highest-leverage human-facing surfaces that already carry
   multiple streams.
4. Keep control-lane work separate from telemetry-lane rendering work.
5. Preserve the warning-first philosophy until a surface explicitly opts into
   stronger enforcement semantics.

## Proposed Execution Slices

### Slice 1: Notification Normalisation Layer

**Why first:** Everything else depends on a shared representation.

**Scope:**

- runtime-side notification types
- mapping helpers from findings/gate results/status updates into notification
  classes
- source metadata and grouping keys

**Expected Outcome:**

- one canonical notification envelope exists for current surfaces to target
- findings, progress updates, and control decisions can all be mapped into it
  without collapsing into free-form strings

**Validation:**

- a shared notification type or contract exists and is exercised by unit tests

### Slice 2: CLI and JSON Delivery Alignment

**Why second:** terminal and machine output are the most visible current sinks.

**Scope:**

- `gate`
- `check`
- `doctor`
- `audit`
- machine-readable output where already present

**Expected Outcome:**

- CLI output consistently distinguishes finding notifications, failures, and
  warnings
- JSON output gains stable notification metadata where appropriate

**Validation:**

- targeted CLI outputs and JSON fixtures align with the taxonomy

### Slice 3: TUI Notification Model

**Why third:** TUI already behaves like a notification-rich environment.

**Scope:**

- watch surface
- tutorial/welcome notices and overlays
- future shared notification widgets or panels

**Expected Outcome:**

- TUI surfaces use shared notification concepts rather than one-off status and
  overlay wording
- high/critical notifications have a consistent presentation model

**Validation:**

- targeted TUI surfaces share notification classes or rendering rules

### Slice 4: Telemetry Stream Contract

**Why fourth:** telemetry subscribers need a stable stream before daemon work
lands.

**Scope:**

- streamable notification/event payload shape
- correlation identifiers
- subscriber-facing event semantics

**Expected Outcome:**

- canonical telemetry notification events are defined for logs, future UIs, and
  observability consumers

**Validation:**

- event schema/documented payload shape exists and is referenced by follow-on
  work

### Slice 5: Control-Lane Integration For Intercept

**Why fifth:** this depends on the model above and should stay separate from the
human-facing UX cleanup.

**Scope:**

- mapping `allow` / `warn` / `block` / `interrupt` into notification mirrors
- ack-required semantics
- fence-state and daemon-health notifications

**Expected Outcome:**

- intercept/control work can reuse the notification framework without redefining
  event classes

**Validation:**

- intercept-facing design or implementation references the canonical taxonomy
  and delivery model

## Recommended Follow-On APS Work Items

### NOTIFY-005: Implement shared notification model

- **Intent:** Create the shared notification envelope and source-mapping layer
- **Expected Outcome:** Runtime surfaces can emit canonical notification classes
  instead of ad hoc status/warning strings only
- **Validation:** shared notification types and unit tests exist

### NOTIFY-006: Align CLI and JSON outputs to notification taxonomy

- **Intent:** Bring current CLI and machine-readable outputs into alignment with
  the notification framework
- **Expected Outcome:** `gate`, `check`, `doctor`, and `audit` expose coherent
  notification semantics
- **Validation:** targeted output tests and fixture updates pass

### NOTIFY-007: Add shared TUI notification model

- **Intent:** Introduce a shared TUI notification approach for watch, tutorial,
  and related surfaces
- **Expected Outcome:** high/critical and grouped notifications are represented
  consistently in TUI surfaces
- **Validation:** targeted TUI tests and snapshots pass

### NOTIFY-008: Define notification telemetry stream contract

- **Intent:** Define the stream/event contract for notification subscribers
- **Expected Outcome:** telemetry-lane payload shape is documented and reusable
- **Validation:** stream schema or contract doc exists and is referenced by
  follow-on work

### NOTIFY-009: Integrate intercept control decisions with notification model

- **Intent:** Ensure intercept/control-plane decisions reuse the shared
  notification taxonomy and delivery split
- **Expected Outcome:** block/interrupt/fence notifications mirror control-lane
  outcomes without inventing parallel semantics
- **Validation:** intercept-facing planning or implementation references the
  shared model

## Sequencing Recommendation

1. `NOTIFY-005` shared notification model
2. `NOTIFY-006` CLI/JSON alignment
3. `NOTIFY-007` TUI notification model
4. `NOTIFY-008` telemetry stream contract
5. `NOTIFY-009` intercept integration

This sequence keeps current-surface cleanup ahead of daemon-era transport while
still making the eventual intercept integration straightforward.

## Recommendation

Treat `NOTIFY` as a parent discovery/design module and execute the work in two
lanes:

- **Current-surface lane:** shared model, CLI/JSON, TUI
- **Future-control lane:** telemetry stream contract and intercept integration

That preserves forward motion on present UX without blocking on the daemon.
