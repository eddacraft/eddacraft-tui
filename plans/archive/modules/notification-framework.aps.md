<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Notification Framework

| ID     | Owner | Status   | Progress |
|--------|-------|----------|----------|
| NOTIFY | —     | Complete | 9/9      |

## Purpose

Define how Anvil handles multiple concurrent streams of user-facing and
operator-facing information before the full intercept daemon and interruption
framework land. Today, different surfaces already emit warnings, findings,
nudges, watch updates, audit issues, setup diagnostics, and future block /
interrupt states. Without a shared notification model, each new surface will
invent its own event vocabulary, priority rules, and delivery path.

This module began as discovery-first work and now carries the bounded follow-on
implementation slices defined by `NOTIFY-004`. It establishes the architecture
for a unified notification plane that can support current CLI/TUI surfaces and
later extend cleanly into daemon-driven interruption and multi-surface
delivery.

## In Scope

- Inventory current notification-like outputs across CLI, TUI, watch, tutorial,
  audit, doctor, gate, and plan/intercept surfaces
- Define a canonical event taxonomy for quality and workflow notifications
- Define priority/severity semantics across warnings, nudges, failures, blocks,
  interrupts, and informational updates
- Identify delivery sinks: terminal inline output, TUI panels, status surfaces,
  machine-readable events, future daemon/IPC channels
- Define deduplication, grouping, and escalation rules for concurrent event
  streams
- Define how current warning-first behaviour evolves toward future block /
  interrupt behaviour without splitting the mental model

## Out of Scope

- Building the full daemon or intercept transport
- Web/mobile push notifications or SaaS alerting
- Replacing Kindling/observability event storage
- UX polish for every existing surface
- Non-Anvil editor integrations beyond identifying interface needs

## Interfaces

**Depends on:**

- `check-language-and-onboarding` (CLAR) — canonical quality language
- `plans/specs/anvil-driver-framework/` — control/telemetry split and
  enforcement decision model
- `intercept-daemon` (INTD) — future interrupt and fence architecture
- `intercept-rules` (INTR) — future rule-driven enforcement decisions
- current CLI/TUI surfaces — existing delivery surfaces
- `kindling-integration` / observability foundation — event and telemetry needs

**Exposes:**

- Notification taxonomy for current and future Anvil surfaces
- Delivery architecture for terminal/TUI/daemon-era notifications
- Follow-on execution slices for implementation work once the design is agreed

## Acceptance Criteria

- [x] A current-state inventory exists for notification-like outputs in the
      CLI/TUI and active plans
- [x] Notification classes and priorities are defined with unambiguous meanings
- [x] Current warning/nudge/failure outputs are mapped onto the notification
      taxonomy
- [x] Future block/interrupt/fence events are mapped into the same model rather
      than a parallel one
- [x] Follow-on implementation work is identified for runtime surfaces and
      daemon-era transport

## Constraints

- Must preserve the warning-first product philosophy until the intercept loop is
  explicitly active for a given surface
- Must align with the driver-framework split between control/enforcement and
  telemetry/event delivery
- Must support both human-facing and machine-facing delivery
- Must avoid conflating findings with notifications: findings are domain
  results, notifications are delivery artefacts carrying those results
- Must compose with the checks -> findings -> gates model rather than replacing
  it

## Tasks

### NOTIFY-001: Inventory current notification streams

- **Intent:** Map every existing warning, nudge, issue, status, and progress
  output path that already behaves like a notification stream
- **Expected Outcome:** A discovery document lists each source, payload shape,
  audience, sink, and priority model
- **Files:** `crates/anvil-cli/`, `crates/anvil-tui/`, `plans/modules/`,
  `plans/specs/`, `docs/architecture/`
- **Validation:** `plans/specs/YYYY-MM-DD-notification-framework-discovery.md`
  exists with source inventory
- **Confidence:** high
- **Status:** Complete

### NOTIFY-002: Define notification taxonomy and priority model

- **Intent:** Define the canonical classes for informational updates, findings,
  nudges, warnings, failures, blocks, interrupts, and fence states
- **Expected Outcome:** A design note defines event classes, required fields,
  severity/priority rules, and escalation semantics
- **Files:** `plans/specs/`, `docs/architecture/`
- **Dependencies:** NOTIFY-001
- **Validation:** Discovery/design doc includes taxonomy table and priority
  rules
- **Confidence:** medium
- **Status:** Complete

### NOTIFY-003: Define delivery architecture for current and future surfaces

- **Intent:** Describe how notifications flow to terminal output, TUI surfaces,
  machine-readable output, and future daemon-era delivery channels
- **Expected Outcome:** A design note documents sink-specific behaviour,
  deduplication/grouping rules, and the handoff path into intercept transport
- **Files:** `plans/specs/`, `docs/architecture/`, `plans/modules/intercept-daemon.aps.md`
- **Dependencies:** NOTIFY-002
- **Validation:** Delivery architecture section exists with sink mapping and
  future intercept bridge
- **Confidence:** medium
- **Status:** Complete

### NOTIFY-004: Define follow-on execution slices

- **Intent:** Convert the notification design into bounded implementation work
- **Expected Outcome:** Follow-on APS tasks or modules are proposed for runtime
  event normalisation, TUI delivery, machine-readable event output, and daemon
  transport integration
- **Files:** `plans/modules/`, `plans/specs/`, `plans/index.aps.md`
- **Dependencies:** NOTIFY-003
- **Validation:** Follow-on work items are listed with scope and validation
- **Confidence:** medium
- **Status:** Complete

### NOTIFY-005: Implement shared notification model

- **Intent:** Create the shared notification envelope and source-mapping layer
  for current runtime surfaces
- **Expected Outcome:** Runtime surfaces can emit canonical notification
  classes instead of ad hoc status and warning strings only
- **Files:** `crates/anvil-kernel-types/`, `crates/anvil-tui/`
- **Dependencies:** NOTIFY-004
- **Validation:** Shared notification types and unit tests exist
- **Confidence:** medium
- **Status:** Complete

### NOTIFY-006: Align CLI and JSON outputs to notification taxonomy

- **Intent:** Bring current CLI and machine-readable outputs into alignment with
  the notification framework
- **Expected Outcome:** `gate`, `check`, `doctor`, and `audit` expose coherent
  notification semantics
- **Files:** `crates/anvil-cli/src/commands/check.rs`,
  `crates/anvil-cli/src/commands/gate.rs`, `crates/anvil-cli/src/commands/doctor.rs`,
  `crates/anvil-cli/src/commands/audit.rs`
- **Dependencies:** NOTIFY-005
- **Validation:** Targeted output tests and fixture updates pass
- **Confidence:** medium
- **Status:** Complete — `check`/`gate` shipped via PR #1035; `doctor`
  emits health/failure notifications with per-check mapping and a summary;
  `audit` emits finding notifications per issue and a severity-driven
  summary

### NOTIFY-007: Add shared TUI notification model

- **Intent:** Introduce a shared TUI notification approach for watch, tutorial,
  and related surfaces
- **Expected Outcome:** High/critical and grouped notifications are represented
  consistently in TUI surfaces
- **Files:** `crates/anvil-tui/src/surfaces/notifications.rs`,
  `crates/anvil-tui/src/surfaces/watch/`,
  `crates/anvil-tui/src/surfaces/tutorial/`,
  `crates/anvil-tui/src/surfaces/onboarding/hooks.rs`
- **Dependencies:** NOTIFY-005
- **Validation:** `cargo test -p eddacraft-anvil-tui` — watch, tutorial, and
  onboarding hooks implement `NotificationSource` and expose canonical
  notifications
- **Confidence:** medium
- **Status:** Complete — initial adoption. The `NotificationSource` trait
  lives in `crates/anvil-tui/src/surfaces/notifications.rs` and is
  implemented by the three surfaces that actively carry notification-like
  state today: `WatchState`, `TutorialState`, and onboarding `HooksState`.
  The remaining TUI surfaces (gate, audit, doctor, status, welcome, wizard,
  browser, init) either render read-only data or have no notification
  queue, so there is nothing to expose through the trait. Reopen with a
  new task if a non-onboarding surface gains notification-like state.

### NOTIFY-008: Define notification telemetry stream contract

- **Intent:** Define the stream/event contract for notification subscribers
- **Expected Outcome:** Telemetry-lane payload shape is documented and reusable
- **Files:** `plans/specs/2026-04-22-notification-telemetry-stream-contract.md`
- **Dependencies:** NOTIFY-005
- **Validation:** Envelope (`anvil.notification.v1`) is documented with
  producer/subscriber contracts and referenced by NOTIFY-009 and INTD-013
- **Confidence:** medium
- **Status:** Complete

### NOTIFY-009: Integrate intercept control decisions with notification model

- **Intent:** Ensure intercept/control-plane decisions reuse the shared
  notification taxonomy and delivery split
- **Expected Outcome:** Design contract that binds future block/interrupt/
  fence notifications to the taxonomy so intercept implementation work
  cannot invent parallel semantics. Actual emission code lives under
  `INTD-013` (intercept-daemon module) and is tracked separately.
- **Files:** `plans/modules/intercept-daemon.aps.md`,
  `plans/specs/2026-04-22-intercept-notification-integration.md`
- **Dependencies:** NOTIFY-008
- **Validation:** Three doc-scope checks — all verifiable from this repo
  without the intercept crate existing:
  1. `plans/modules/intercept-daemon.aps.md` declares a `NOTIFY` dependency
     in its Interfaces section.
  2. The same module carries a "Notification Model Integration" section
     with the fixed mapping table for `allow` / `warn` / `block` /
     `interrupt` plus fence-state transitions.
  3. `INTD-013` is tracked as a code work item whose own validation command
     (`cargo test -p eddacraft-anvil-intercept --lib telemetry`) will run
     once the intercept crate is created — this task remains `Draft` under
     INTD and is the gate for the code-level assertion.
- **Confidence:** medium
- **Status:** Complete (doc-only scope; code emission lands with INTD-013)
