# Anvil Architecture Evolution

**This document supersedes ADR-011 and defines the Current → H1 → H2 migration
path.**

## Anvil Single-Binary + Optional Daemon, with Phased Rust Adoption

---

## 0. Purpose

Define the target architecture and rollout plan for Anvil as:

- **Developer-first:** frictionless install + immediate value
- **Kernel-first:** persistent, incremental semantic runtime ("semantic
  guardrail runtime") — see [Rust Kernel Spec](rust-kernel-spec.md)
- **Protocol-first:** stable event contract enabling parallel engines and safe
  cutover
- **Not mutually exclusive:** single-binary distribution and always-on daemon
  behaviour can coexist

---

## 1. Definitions

### 1.1 Surfaces

User-facing interfaces:

- CLI
- TUI
- VS Code extension
- CI integration
- Agent connectors (future)

### 1.2 Engine

Abstract semantic runtime exposed to surfaces. Engines accept inputs
(repo/config/plan context) and emit structured events.

### 1.3 Kernel

The high-performance Rust implementation of the Engine:

- File watching
- Incremental parsing
- Persistent semantic graphs
- Policy evaluation
- Streaming events

See [Rust Kernel Spec](rust-kernel-spec.md) for full technical specification.

### 1.4 Core

Deterministic domain semantics:

- Event types + schema
- Invariant categories (structural/evolution/procedural law) — see
  [Constitutional Engineering](../vision/constitutional-engineering.md)
- Policy evaluation semantics
- Graph model types (symbol/dependency/trust/plan overlays)

Core is not a watcher; it defines meaning.

---

## 2. Guiding Principles

- **Deterministic over probabilistic** — AI may explain; AI never decides truth.
  See [Rust Kernel Spec](rust-kernel-spec.md).
- **Incremental over full rescans** — see
  [Rust Kernel Spec](rust-kernel-spec.md)
- **Streaming over batch** — especially in watch. See
  [Rust Kernel Spec](rust-kernel-spec.md).
- **Protocol-first boundaries** — surfaces render events; engines produce them
- **Migration safety** — dual-run parity before defaults change

---

## 3. User-Level Outcomes (Why We're Doing This)

### 3.1 Invariant Violation Streaming

As you type, Anvil streams structural violations and guidance (not lint noise).
See [Aspirational Ultimate Feature](../vision/aspirational-ultimate-feature.md).

### 3.2 Behavioural Diff Review ("What Changed in Behaviour?")

Instead of text diffs, Anvil summarises semantic deltas: public surface growth,
new external calls, privilege expansion. See
[Aspirational Ultimate Feature](../vision/aspirational-ultimate-feature.md) and
[Rust Kernel Spec](rust-kernel-spec.md).

### 3.3 Constitutional Enforcement for Humans and AI

Same invariant model applies to human-authored and agent-authored code. See
[Constitutional Engineering](../vision/constitutional-engineering.md).

---

## 4. Current Architecture (Baseline)

Today: TypeScript CLI is the main surface; the "engine" is effectively TS
runtime logic that performs checks in batch/partial watch.

```
             +---------------------+
             |   TS CLI Surface    |
             |  (commands + UX)    |
             +----------+----------+
                        |
                        v
             +---------------------+
             |  TS Runtime/Engine  |
             |  (scan/check/gate)  |
             +----------+----------+
                        |
                        v
             +---------------------+
             |   Repo / Git FS     |
             +---------------------+
```

Key properties:

- Fast iteration for beta users
- But limited by Node for persistent, incremental graph runtime and
  ultra-low-latency watch loops (the "alive" feel) — see
  [Aspirational Ultimate Feature](../vision/aspirational-ultimate-feature.md)
  and [Rust Kernel Spec](rust-kernel-spec.md)

---

## 5. H1 Architecture: Rust Kernel Introduced Behind a Stable Protocol

**H1 goal:** Keep beta velocity while building the Rust kernel as a parallel
engine implementation.

### 5.1 Core Idea: Engine Event Protocol (Thin Waist)

Surfaces consume events only. Engines emit events.

```
                 +-----------------------+
                 |       Surfaces        |
                 | CLI / TUI / VSCode    |
                 +-----------+-----------+
                             |
                             v
                 +-----------------------+
                 |  Engine Event Protocol|
                 |   (schema + framing)  |
                 +-----------+-----------+
                             |
              +--------------+--------------+
              |                             |
              v                             v
    +-------------------+         +-------------------+
    | Legacy TS Engine  |         | Rust Kernel Engine |
    | (adapter emits    |         | (watch+parse+graph |
    |  protocol events) |         |  + policy + stream)|
    +---------+---------+         +---------+---------+
              |                             |
              +--------------+--------------+
                             v
                     +---------------+
                     | Repo / Git FS |
                     +---------------+
```

### 5.2 Rust Kernel Responsibilities (H1 v1)

Implement the watcher-kernel pipeline as specified in the
[Rust Kernel Spec](rust-kernel-spec.md):

```
[ File Watcher ] -> [ Queue + Debounce ] -> [ Incremental Parser ]
        -> [ Symbol Graph ] -> [ Dependency Graph ] -> [ Trust Graph ]
        -> [ Policy Engine ] -> [ Streaming Output ]
```

Minimum buildable scope (v1):

- File watcher + debounce/backpressure
- Incremental parse cache (tree-sitter)
- Symbol + dependency graph
- Minimal trust metadata (enough for a small rule set)
- Policy evaluation on affected subgraphs
- Stream protocol events

### 5.3 Engine Selection and Dual-Run

Expose:

- `anvil --engine legacy`
- `anvil --engine rust`
- `anvil --engine dual` (internal/dogfood)

Dual mode runs both engines against same change stream and diffs normalised
event streams.

---

## 6. Single Binary Install + Optional Daemon (Applies to H1 and H2)

This is the "not mutually exclusive" design:

- **Distribution:** single binary (`anvil`)
- **Lifecycle:** supports both one-shot and long-lived daemon

### 6.1 Command Model

**One-shot (no daemon required):**

- `anvil check` / `anvil gate` / `anvil scan`
- Runs engine and exits

**Foreground long-lived:**

- `anvil watch`
- Runs kernel loop in foreground and streams events

**Background daemon (optional):**

- `anvil daemon start [--repo <path>]`
- `anvil daemon status`
- `anvil daemon stop`
- `anvil daemon logs`

### 6.2 Daemon Responsibilities

Daemon owns:

- Watch loop
- Debounce/backpressure
- Incremental graph state
- Policy evaluation scheduling
- Event fan-out to clients

Clients (CLI/VSCode/agents) own:

- Requesting sessions
- Rendering events
- User interactions

### 6.3 Local Transport

Define a local socket:

- Unix domain socket on Linux/macOS
- Named pipe / TCP loopback on Windows

Two channels:

- **Control RPC** (start/stop/status/subscribe)
- **Event stream** (NDJSON or length-prefixed binary frames)

---

## 7. Engine Event Protocol (H1/H2 Stable Contract)

### 7.1 Event Framing

- **Default:** NDJSON (`\n` delimited JSON objects)
- **Optional:** length-prefixed frames for higher throughput

### 7.2 Required Event Types (H1 MVP)

```yaml
Progress:
  stage: string
  message: string
  percent?: number
  detail?: object

Snapshot:
  graph_hash: string
  files_indexed: number
  symbols_indexed: number
  duration_ms: number

Violation:
  policy_id: string
  severity: info|warn|error
  file: string
  symbol?: string
  reasoning: string
  suggested_remediation?: string
  refs?: { rule_url?: string, doc?: string }
```

These map cleanly to
[Invariant Violation Streaming](../vision/aspirational-ultimate-feature.md) and
the [Rust Kernel Spec](rust-kernel-spec.md).

### 7.3 Planned Event Types (H1.5/H2)

```yaml
BehaviouralDelta:
  summary: string
  new_external_calls: number
  new_public_methods: number
  privilege_scope_expansion: boolean
  notes?: [string]

PlanDrift:
  file: string
  symbol?: string
  expected_module?: string
  actual_location?: string
  reasoning: string
```

---

## 8. H2 Architecture: Rust CLI/TUI Across EddaCraft Stack

**H2 goal:** Promote Rust from "engine only" to a full Rust-native surface set
(CLI + TUI), while keeping protocol compatibility for other clients.

This is explicitly a surface replacement decision, not required for H1.

### 8.1 H2 System Diagram

```
                    +---------------------+
                    |  Rust CLI/TUI       |
                    |  (primary surface)  |
                    +----------+----------+
                               |
                               v
                    +---------------------+
                    |  Rust Kernel Engine |
                    |  (daemon-capable)   |
                    +----------+----------+
                               |
             +----------------+----------------+
             |                                 |
             v                                 v
   +-------------------+            +-------------------+
   | VS Code Extension  |            | Agent/CI Clients  |
   | (protocol client)  |            | (protocol client) |
   +---------+----------+            +---------+----------+
             \___________________________/
                         |
                         v
               +---------------------+
               | Engine Protocol     |
               | (stable events)     |
               +---------------------+
```

### 8.2 Ratatui Diagrams as "Delight Multiplier"

The diagram renderer becomes a TUI widget library that consumes graph
snapshots/deltas and produces the "how on earth did they do that in a terminal?"
reaction. See
[Diagram Rendering for Ratatui](../research/diagram-rendering-for-ratatui.md).

Diagrams remain downstream of the kernel; they are renderers, not analysers.

---

## 9. Phased Delivery Plan (Developer-First, Investor-Wow)

### Phase A (Now → Beta Stability)

- Refactor legacy TS output to emit canonical protocol events
- Ship improved UX with minimal invariants (high precision)

### Phase B (H1 Kernel MVP)

- Rust kernel v1: watch + incremental parse + symbol/deps graphs + 3–5
  invariants
- `--engine rust` hidden/opt-in
- Dual-run harness for parity

### Phase C (First Investor "Wow")

- Behavioural diff MVP: "what changed in behaviour" summary — see
  [Aspirational Ultimate Feature](../vision/aspirational-ultimate-feature.md)
- Live invariant streaming demo: immediate, semantic, deterministic — see
  [Aspirational Ultimate Feature](../vision/aspirational-ultimate-feature.md)

### Phase D (H2 Surfaces)

- Rust CLI/TUI becomes primary surface
- Optional daemon mode polished
- VS Code + agent clients consume protocol

---

## 10. Non-Goals (For Sanity)

- Distributed watcher mesh (future) — see
  [Aspirational Ultimate Feature](../vision/aspirational-ultimate-feature.md)
- Cross-repo awareness (enterprise mode later) — see
  [Rust Kernel Spec](rust-kernel-spec.md)
- Full deep dataflow analysis in v1
- AI in enforcement path (never) — see [Rust Kernel Spec](rust-kernel-spec.md)

---

## 11. Success Criteria

### H1 Success Criteria

- Rust kernel incremental update under ~100ms on single-file changes in a medium
  repo (target) — see [Rust Kernel Spec](rust-kernel-spec.md)
- Output parity with legacy engine for supported policies
- Developers report the tool "feels alive" (invariant streaming) — see
  [Aspirational Ultimate Feature](../vision/aspirational-ultimate-feature.md)

### H2 Success Criteria

- Single-binary install (platform builds)
- Rust TUI provides materially better experience (diagrams, navigation)
- Daemon mode reduces latency and enables multi-client usage (VS Code + CLI
  simultaneously)

---

## Appendix A: Current vs H1 vs H2 at a Glance

```
Current:
  TS CLI -> TS runtime checks -> output

H1:
  Surfaces -> Protocol -> (Legacy TS engine OR Rust kernel engine)

H2:
  Rust CLI/TUI -> Rust kernel (daemon-capable)
        + other clients (VSCode/CI/agents) via protocol
```
