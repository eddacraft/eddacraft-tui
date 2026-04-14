# ADR: Anvil Driver Framework and Host-Local Enforcement Control Plane

**Status:** Proposed  
**Date:** 2026-04-02  
**Decision Owner:** Joshua Boys / eddacraft  
**Related Initiative:** Anvil Rust Kernel, TS integration surfaces, watch/daemon/session control

## Context

Anvil is evolving toward a Rust-owned kernel/core with multiple integration surfaces across local developer machines, remote development hosts, editors, terminals, and hosted agent environments.

The original transport question was whether Anvil should use gRPC or JSON-RPC for communication between the Rust kernel/core and the TypeScript adapter framework. As the runtime model became clearer, the central problem was shown not to be generic service-to-service communication, nor eventual notification delivery.

The real runtime requirement is a **machine-local interception control loop**:

1. one or more agents make file changes or saves
2. Anvil Watch detects those changes immediately
3. the daemon/kernel performs deterministic mechanical checks very quickly
4. if a violation is found, Anvil must be able to interrupt, block, or otherwise stop the relevant session before it continues further

This is fundamentally a **local control plane** problem, not a distributed service mesh problem.

The initial thinking around “adapters” treated integrations as product-specific surfaces, for example Claude Code, Codex, Gemini, MCP, editor plugins, and so on. That framing is insufficient because the hard problem is not merely identifying which product originated a change. The hard problem is whether Anvil has **enforceable authority** over the execution boundary that produced the change.

In practice, developers may run:

- local shells under zsh/bash/fish
- tmux sessions and panes
- WezTerm tabs and panes
- multiple local worktrees
- remote tmux sessions over SSH on development servers
- remote hosted or web-based agent environments
- nested sub-agents and multi-agent councils

This means Anvil must reason not only about a session and a tool, but also about the **execution host**, the **control surface**, and the **capabilities available on that surface**.

## Problem

Detection without enforceable authority is only observability.

A model based purely on thin adapters or MCP integrations is insufficient because:

- not all agent surfaces are reachable or reliable through MCP
- MCP does not provide strong hard-interrupt guarantees
- multiple worktrees and nested sessions complicate generic integration approaches
- file-level provenance alone does not guarantee that the owning execution surface can actually be stopped
- interruption semantics vary radically between local shell processes, remote shells, and hosted web environments

Anvil therefore requires a more explicit model:

- host-local enforcement points
- surface-specific control drivers
- session registration and leases
- capability-aware routing
- multi-level enforcement and escalation

## Decision

Anvil will adopt a **driver framework** rather than a passive adapter framework.

The system will be designed as a **control authority plus host-local enforcement model**:

- **Rust kernel/core** owns the hot path, watch ingestion, deterministic checks, session registry, routing, and enforcement decisions
- **Drivers** provide attachment to concrete execution boundaries and expose the capabilities available to interrupt, block, warn, or fence a session
- **Execution hosts** are first-class entities in the session model
- **Session leases** are first-class control objects that can be restricted, blocked, or revoked
- **Interrupt** is treated as a driver-relative capability rather than a universal primitive

For local IPC and host-local communication, Anvil will continue with:

- **JSON-RPC 2.0 over Unix domain sockets** on Unix-like systems
- **JSON-RPC 2.0 over named pipes** on Windows

Communication will be split into two lanes:

1. **Control / enforcement lane**  
   Small synchronous request-response commands with ack semantics for critical actions

2. **Telemetry / event lane**  
   NDJSON or equivalent streaming channel for progress, snapshots, diagnostics, and UI/event subscribers

MCP will be treated as a **secondary or fallback driver**, not the foundational control plane.

## Architectural Principles

### 1. Authority surfaces over integration surfaces

Drivers are designed around what they can **control**, not merely what they can observe.

### 2. Host-local control first

Hard interrupt, process-group control, and write fencing are strongest when exercised on the execution host where the work is actually running.

### 3. Shell-first, not shell-only

For local and remote CLI-driven workflows, the shell is a strong session ingress and provenance boundary. However, the shell alone is not sufficient as the only enforcement plane.

### 4. Capability-relative interruption

The decision contract is stable (`allow`, `warn`, `block`, `interrupt`) but its implementation is driver-specific.

### 5. Fencing as a first-class concept

When direct interrupt is weak or unreliable, Anvil must be able to fence:

- sessions
- worktrees
- repositories
- capabilities

### 6. Telemetry must not contaminate the hot path

Critical enforcement actions must not compete with progress, snapshot, or UI event traffic.

## Chosen Model

### Core concepts

#### Execution host
A machine or environment on which a session actually executes.

Examples:
- local laptop
- remote dev server
- hosted agent environment

#### Driver
A control surface attached to a session or host.

Examples:
- local shell driver
- remote shell driver
- process-group driver
- tmux metadata driver
- editor driver
- web-session driver
- MCP fallback driver

#### Session
A concrete unit of execution tied to an execution host, worktree, repo, and driver.

#### Session lease
A control object issued by the daemon/kernel that gives a session permission to continue operating. A lease can be restricted, blocked, or revoked.

#### Fence
A control state applied to a session, worktree, repo, or capability boundary to prevent continued unsafe operation.

## Driver Taxonomy

### Local shell driver
Primary driver for:
- local zsh/bash/fish sessions
- local CLI agent launches
- local tmux/terminal workflows

Typical capabilities:
- session registration
- cwd/repo/worktree detection
- pid/pgid capture
- process-group interrupt
- launch refusal if fenced

### Remote shell driver
Primary driver for:
- remote shells over SSH
- remote tmux sessions
- agent sessions on dev servers

Requires:
- Anvil sidecar or daemon on the remote host
- host-local control on that remote machine

Typical capabilities:
- same broad class as local shell driver, but exercised remotely

### Process driver
Specialist driver for:
- process tree control
- hard interrupt
- SIGINT / SIGTERM / SIGKILL / suspend/resume

### tmux driver
Support driver for:
- pane/session/window metadata
- UX signalling
- targeted pane messaging
- fallback interventions

### Editor driver
Driver for:
- editor-managed sessions
- warnings
- save blocking
- session fencing within the editor surface

### Web-session driver
Driver for:
- hosted or browser-based agent environments
- soft interrupt via API
- lease revocation
- blocking future actions/results

### MCP driver
Fallback driver for:
- structured warning
- best-effort cancellation
- compatibility integrations

Not suitable as the primary hard-control surface.

## Control Model

### Control authority
Anvil maintains a control authority that owns:

- policy evaluation
- deterministic checks
- lease state
- session registry
- routing
- enforcement decisions
- escalation policy

### Host-local enforcement
Each execution host has a host-local enforcement point that owns:

- local shell integration
- local watcher correlation
- local process control
- local worktree fencing
- host-local enforcement execution

### Enforcement ladder

#### Level 1: Warn
- show warning
- annotate UI
- mark session risky

#### Level 2: Soft interrupt
- request cancellation
- send structured stop
- reject future tool calls
- mark session blocked

#### Level 3: Workflow fence
- deny further writes
- deny future actions
- fence session/worktree/repo
- revoke lease

#### Level 4: Hard interrupt
- SIGINT process or process group
- SIGTERM process or process group
- SIGKILL as last resort
- suspend session

#### Level 5: Capability revocation
- revoke tokens
- revoke permissions
- detach driver
- quarantine session outputs/results

## Transport Decision

### Why JSON-RPC 2.0 over local sockets remains the right choice

The burden of proof for moving to gRPC is high and not met.

JSON-RPC over local sockets remains appropriate because:

- the primary problem is machine-local control
- the required control messages are small and simple
- a mixed Rust/TypeScript environment benefits from a lightweight wire contract
- the system needs explicit methods and acknowledgements, not broad schema-heavy service definitions
- Unix sockets/named pipes fit the execution model
- local control planes do not need the full complexity of gRPC

### Transport lanes

#### Control lane
Request-response, synchronous, small payloads.

Examples:
- `session.register`
- `session.unregister`
- `session.heartbeat`
- `watch.change`
- `enforcement.warn`
- `enforcement.block`
- `enforcement.interrupt`
- `enforcement.ack`

#### Telemetry lane
Event stream, best effort, lossy-tolerant.

Examples:
- progress
- snapshots
- diagnostics
- UI subscriptions
- non-critical events

## Consequences

### Positive
- aligns architecture with real execution control boundaries
- supports local and remote hosts uniformly
- removes over-dependence on MCP
- gives a practical path to strong interrupt semantics
- makes shell/tmux workflows first-class
- preserves lightweight local IPC
- keeps Rust in control of the hot path

### Negative
- introduces more explicit session and host modelling
- requires driver capability definitions
- requires remote-side deployment for remote shell control
- hosted/web environments still have weaker enforcement semantics
- increases control-plane complexity compared with passive adapters

## Rejected Alternatives

### 1. gRPC as the default transport
Rejected because the core problem is not service-mesh communication or cross-machine API interoperability. It adds complexity without solving the hard problems of authority, routing, or enforcement.

### 2. MCP as the primary control plane
Rejected because MCP is too weak and inconsistent as the main interrupt mechanism, especially for multi-worktree and nested-session scenarios.

### 3. Tool-specific adapters as the primary abstraction
Rejected because product identity is less important than execution boundary and enforceable authority.

### 4. Shell-only solution
Rejected because shell wrapping is powerful for ingress and some enforcement, but insufficient alone for multi-host, editor, and hosted-agent scenarios.

### 5. Deep OS-level mount/interception as the starting point
Deferred. Mount-like or FUSE-style approaches may become relevant later, but they are too heavyweight and platform-specific for the initial control-plane architecture.

## Implementation Guidance

Prioritise the following:

1. define execution host, session, lease, and driver schemas
2. implement shell driver and process-group control
3. add host-local enforcement daemon model
4. split control and telemetry lanes
5. treat MCP as fallback only
6. add remote host support through remote sidecar/daemon deployment
7. add weaker web-session drivers for hosted environments

## Decision Summary

Anvil will evolve from a passive adapter model into a **driver-based, host-aware local control plane**.

The system will use:
- Rust-owned hot-path enforcement
- JSON-RPC over host-local sockets for control
- separate telemetry streaming
- session leases
- driver capabilities
- host-local enforcement points
- shell/process-driven interruption where available
- fallback web/MCP drivers where hard control is impossible
