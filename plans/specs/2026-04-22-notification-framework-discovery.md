# Notification Framework Discovery

## Purpose

Discovery output for `NOTIFY-001`. This document inventories the current
notification-like outputs across Anvil's CLI, TUI, and active plans so future
work can define one shared notification framework instead of letting each
surface evolve its own event and escalation model.

## Scope

This pass focuses on:

- current Rust CLI/TUI surfaces
- current user-visible terminal and TUI outputs
- active plans that introduce future block, interrupt, fence, or stream-like
  behaviour

It intentionally does not design the final taxonomy yet. The goal is to capture
what streams already exist and where drift is likely.

## Driver Framework Relevance

The driver-framework design should have been part of the initial read set for
this discovery because it already establishes two critical constraints:

- a split between **control / enforcement** and **telemetry / event** lanes
- a shared decision ladder of `allow`, `warn`, `block`, and `interrupt`

Notification design therefore cannot be treated as generic UI messaging alone.
It has to map cleanly onto the driver-framework control model while still
supporting lower-authority human-facing surfaces such as CLI and TUI output.

## Working Definitions

- **Finding:** a domain result emitted by a check
- **Notification:** a delivery artefact that carries findings, status, progress,
  or escalation to a human or machine consumer

This distinction matters: several current surfaces already deliver findings,
status, and progress, but they do so with different shapes and priorities.

## Current Notification Sources

### 1. Gate CLI output

Source:

- `crates/anvil-cli/src/commands/gate.rs`

Current behaviour:

- emits progress lines when `--progress` is enabled
- emits per-check PASS/FAIL lines in plain mode
- emits detailed failure text when checks fail
- emits an overall quality-gate summary line

Audience:

- human operator in terminal

Sink:

- plain terminal output
- JSON output in machine mode

Priority model:

- implicit only: pass/fail and score

### 2. Check CLI output

Source:

- `crates/anvil-cli/src/commands/check.rs`

Current behaviour:

- emits findings directly rather than an aggregated gate judgement
- uses warning-oriented language and threshold-driven blocking logic
- supports machine-readable JSON output with structured warning payloads

Audience:

- human operator or machine consumer

Sink:

- plain terminal output
- JSON output

Priority model:

- severity threshold drives blocking vs non-blocking behaviour

### 3. Doctor diagnostics

Source:

- `crates/anvil-cli/src/commands/doctor.rs`
- `crates/anvil-tui/src/surfaces/doctor/`

Current behaviour:

- emits setup diagnostics as `DiagnosticCheck` items
- supports pass / warn / fail / skipped style states
- TUI supports expansion and fix requests

Audience:

- human operator

Sink:

- terminal output
- TUI list/detail surface

Priority model:

- explicit status enum but scoped to environment/setup health

### 4. Audit issues

Source:

- `crates/anvil-cli/src/commands/audit.rs`
- `crates/anvil-tui/src/surfaces/audit/`

Current behaviour:

- emits `AuditIssue` items rather than checks/findings/gates terminology
- includes severity and category
- includes next-step summarisation

Audience:

- human operator

Sink:

- terminal output
- TUI multi-panel surface

Priority model:

- issue severity levels with broad repo-review framing

### 5. Watch-mode status stream

Source:

- `crates/anvil-cli/src/tui.rs`
- `crates/anvil-tui/src/surfaces/watch/`

Current behaviour:

- consumes engine events and converts them into:
  - current status
  - queued changes
  - run history
  - stats
- acts like a live notification feed already

Audience:

- human operator during continuous monitoring

Sink:

- TUI dashboard panels

Priority model:

- implicit through status changes, queue ordering, history, and panel focus

### 6. Tutorial guidance and overlay notices

Source:

- `crates/anvil-cli/src/commands/tutorial.rs`
- `crates/anvil-tui/src/surfaces/tutorial/`

Current behaviour:

- emits static notices when interactive capabilities are unavailable
- emits discovery findings during onboarding/tutorial flows
- emits guided overlay text in watch demo
- persists in-progress/resume state

Audience:

- first-run or learning user

Sink:

- TUI instructional copy, overlays, notices

Priority model:

- instructional rather than severity-first, but still effectively a notification
  stream

### 7. Welcome/onboarding status messaging

Source:

- `crates/anvil-cli/src/commands/welcome.rs`

Current behaviour:

- transient loading messages
- warning fallback messages when discovery fails
- completion summaries after init
- quick-action transitions between gate/audit/doctor/watch/tutorial surfaces

Audience:

- first-run or returning user

Sink:

- terminal/TUI transitional messaging

Priority model:

- ad hoc today: warning text, loading states, success summaries

## Existing Event-Like Structures

### Engine and watcher events

Current TUI loops already consume event streams from the kernel and watcher
layers. These are not yet a unified notification framework, but they are the
closest current substrate.

Observed shapes include:

- engine events for watch/dashboard updates
- file change batches for tutorial auto-verification
- watch queue/history records as derived event summaries

### Dirty-state redraw model

Several TUI surfaces already use a `dirty` flag plus event draining / redraw
cycle. This is relevant because any future notification framework will need to
coexist with or build on this event-delivery pattern rather than fight it.

## Forward-Looking Notification Sources

### Intercept daemon

Source:

- `plans/modules/intercept-daemon.aps.md`

Future outputs implied by the plan:

- allow / interrupt decisions
- fence state
- blocked-worktree status
- daemon health
- session attribution / unknown-agent states

This is the strongest future source of block/interrupt notifications.

### Driver framework

Source:

- `plans/specs/anvil-driver-framework/anvil-driver-framework-design-spec.md`
- `plans/specs/anvil-driver-framework/anvil-driver-framework-adr.md`

Future outputs implied by the design:

- control-lane decisions with ack semantics
- telemetry/event-lane updates for UI, TUI, logs, and diagnostics
- capability-relative warning/interruption/block behaviour by driver
- host-local enforcement status and routing outcomes

This means the eventual notification framework must separate:

- domain findings
- enforcement decisions
- telemetry/event delivery

### Tier 2 and Tier 3 CLI work

Sources:

- `plans/modules/rust-cli-tier2.aps.md`
- `plans/modules/rust-cli-tier3.aps.md`

Future outputs implied by the plans:

- nudge coaching in `anvil check --interactive`
- PR comment generation from gate results
- richer `status`, `validate`, and explanation outputs

These will increase the number of user-visible information streams if not
normalised.

### Weave / agent event work

Source:

- `plans/modules/weave.aps.md`

Future outputs implied by the plan:

- typed lifecycle notifications
- streamed provider events
- kernel violation event triggers

This suggests Anvil will eventually need both quality notifications and agent /
system lifecycle notifications within a coherent overall framework.

## Inventory Table

| Source | Current Shape | Human/Machine | Primary Sink | Priority Model |
| --- | --- | --- | --- | --- |
| `gate` | per-check results + overall judgement | both | terminal / JSON | pass/fail/score |
| `check` | findings + blocking threshold | both | terminal / JSON | severity threshold |
| `doctor` | diagnostic checks | human | terminal / TUI | pass/warn/fail/skip |
| `audit` | issues + next steps | human | terminal / TUI | severity + category |
| `watch` | live status / queue / history / stats | human | TUI | status transitions |
| `tutorial` | notices / overlays / findings / resume state | human | TUI | instructional + contextual |
| `welcome` | loading / warning / success summaries | human | terminal / TUI | ad hoc |
| intercept daemon (planned) | allow/warn/block/interrupt/fence | both | IPC / status / future UI | enforcement actions |

## Initial Gaps

1. There is no shared notification taxonomy yet.
2. Similar concepts surface under different nouns: warning, issue, finding,
   status, overlay, message, queue item, history item.
3. Severity and delivery are mixed together differently per surface.
4. There is no documented escalation path from today's warning-first surfaces to
   tomorrow's block/interrupt/fence surfaces.
5. Machine-readable output exists in some CLI commands, but not as one coherent
   event stream contract.
6. The discovery must explicitly account for the driver-framework split between
   control/enforcement traffic and telemetry/event traffic.

## Architectural Implication

The next design step should define notifications as a separate layer from
findings:

- checks emit findings
- gates produce workflow judgement
- notification surfaces deliver, prioritise, group, and escalate those results

It should also distinguish between:

- **control-plane decisions** (`allow`, `warn`, `block`, `interrupt`)
- **telemetry/event notifications** consumed by CLI, TUI, logs, and future UIs

That model also needs to absorb non-finding streams such as progress, queue,
history, and daemon health.

## Recommendation For NOTIFY-002

The taxonomy work should define at least:

- informational update
- progress update
- finding notification
- nudge
- warning
- failure
- block
- interrupt
- fence-state notification
- system/daemon health notification

And it should describe how those map across:

- plain terminal output
- TUI surfaces
- JSON / machine output
- future IPC/NDJSON transport
