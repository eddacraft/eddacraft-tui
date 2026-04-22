# Notification Framework

| ID    | Owner | Status |
|-------|-------|--------|
| NOTIFY | —    | Draft  |

## Purpose

Define how Anvil handles multiple concurrent streams of user-facing and
operator-facing information before the full intercept daemon and interruption
framework land. Today, different surfaces already emit warnings, findings,
nudges, watch updates, audit issues, setup diagnostics, and future block/
interrupt states. Without a shared notification model, each new surface will
invent its own event vocabulary, priority rules, and delivery path.

This module is discovery-first. It establishes the architecture for a unified
notification plane that can support current CLI/TUI surfaces and later extend
cleanly into daemon-driven interruption and multi-surface delivery.

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
- Define how current warning-first behaviour evolves toward future block/
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
- `intercept-daemon` (INTD) — future interrupt and fence architecture
- `intercept-rules` (INTR) — future rule-driven enforcement decisions
- `ratatui-tui` / current TUI surfaces — existing delivery surfaces
- `kindling-integration` / observability foundation — event and telemetry needs

**Exposes:**

- Notification taxonomy for current and future Anvil surfaces
- Delivery architecture for terminal/TUI/daemon-era notifications
- Follow-on execution slices for implementation work once the design is agreed

## Acceptance Criteria

- [ ] A current-state inventory exists for all notification-like outputs in the
      CLI/TUI and active plans
- [ ] Notification classes and priorities are defined with unambiguous meanings
- [ ] Current warning/nudge/failure outputs are mapped onto the notification
      taxonomy
- [ ] Future block/interrupt/fence events are mapped into the same model rather
      than a parallel one
- [ ] Follow-on implementation work is identified for runtime surfaces and
      daemon-era transport

## Constraints

- Must preserve the warning-first product philosophy until the intercept loop is
  explicitly active for a given surface
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
- **Status:** Ready

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
- **Status:** Ready

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
- **Status:** Ready

### NOTIFY-004: Define follow-on execution slices

- **Intent:** Convert the notification design into bounded implementation work
- **Expected Outcome:** Follow-on APS tasks or modules are proposed for runtime
  event normalisation, TUI delivery, machine-readable event output, and daemon
  transport integration
- **Files:** `plans/modules/`, `plans/specs/`, `plans/index.aps.md`
- **Dependencies:** NOTIFY-003
- **Validation:** Follow-on work items are listed with scope and validation
- **Confidence:** medium
- **Status:** Ready
