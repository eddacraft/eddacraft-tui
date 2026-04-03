# Anvil Driver Framework and Host-Local Enforcement Design Spec

**Version:** Draft v1  
**Date:** 2026-04-02  
**Scope:** Runtime control, session model, driver framework, hot-path enforcement, transport, shell/tmux/remote execution surfaces

## 1. Purpose

This document defines the design for Anvil’s driver framework and host-local enforcement model.

It turns a previous “adapter framework” concept into a more explicit architecture focused on **authority, routing, interruption, and control** across:

- local shell-based agent sessions
- tmux and terminal surfaces
- remote SSH/dev-server sessions
- editor integrations
- hosted/web agent environments
- MCP and other lower-authority fallback integrations

The design assumes Anvil is evolving toward a Rust-owned kernel/core with TypeScript integration surfaces and multiple concurrent agent sessions across worktrees and hosts.

## 2. Goals

### Primary goals
- provide ultra-fast local interception from file change to enforcement decision
- support targeted interruption of the relevant session
- model execution host and control surface explicitly
- avoid over-reliance on MCP
- make shell/tmux-based agent workflows first-class
- separate hot-path control from telemetry
- allow capability-relative enforcement across heterogeneous environments

### Secondary goals
- support remote execution hosts uniformly
- support nested sub-agents and multi-agent workflows
- support session/worktree/repo fencing
- support gradual migration from current mixed TS/Rust architecture

### Non-goals
- fully solving all hosted/web-agent hard interrupt semantics in v1
- implementing FUSE-like or deep kernel-level interception in v1
- designing a cross-machine service mesh
- replacing all product-specific integrations immediately

## 3. Terminology

### Execution host
A machine or environment where a session executes.

### Driver
A surface-specific control component that can identify, monitor, and enforce against a session.

### Session
A unit of execution associated with a user workflow, execution host, worktree, and control driver.

### Lease
A permission-bearing token or state issued to a session by the control authority. It can be restricted, blocked, or revoked.

### Fence
A control restriction applied to a boundary such as a session, worktree, repo, or capability.

### Control authority
The daemon/kernel authority responsible for decisions, routing, and lease state.

### Host-local enforcement point
A daemon/sidecar on the execution host responsible for local process control and enforcement execution.

## 4. System Overview

### 4.1 High-level shape

Anvil consists of:

- a **control authority**
- one or more **host-local enforcement points**
- a set of **drivers**
- a shared **session and lease model**
- a **watch/change ingestion path**
- a **deterministic enforcement pipeline**
- a separate **telemetry lane**

### 4.2 Conceptual flow

1. a driver launches or attaches to a session
2. the session registers with the control authority
3. the session receives a lease
4. watch events and other runtime signals are stamped with session/worktree/host provenance
5. the kernel runs deterministic checks
6. the kernel returns a decision:
   - allow
   - warn
   - block
   - interrupt
7. the control authority routes the decision to the owning driver on the owning execution host
8. the driver performs the appropriate enforcement action and acks if required
9. telemetry is emitted separately for UI, TUI, logs, and diagnostics

## 5. Core Design Principles

### 5.1 Detection without authority is insufficient
Anvil must own or attach to a surface that can actually stop or fence execution.

### 5.2 Execution boundary matters more than tool brand
The architecture is driven by where and how a session runs, not by whether the tool is Claude, Codex, Gemini, or something else.

### 5.3 Shell-first for CLI workflows
For local and remote CLI-driven agents, shell launch is a strong universal ingress boundary.

### 5.4 Host-local enforcement
Hard control should be executed on the host where the work is actually running.

### 5.5 Capability-relative semantics
A driver advertises what it can really do. The enforcement layer chooses the strongest safe action available.

### 5.6 Telemetry separation
The control path must remain clean, bounded, and low-latency.

## 6. Architecture

## 6.1 Major components

### A. Control Authority
Responsibilities:
- session registry
- execution host registry
- driver registry
- lease issuance and revocation
- policy evaluation
- deterministic hot-path checks
- routing and escalation
- fence state
- audit/event emission

Likely ownership:
- Rust kernel/core

### B. Host-Local Enforcement Point
Responsibilities:
- local session attachment
- local process-group control
- local watcher correlation
- host-local worktree and repo fencing
- driver execution on that host
- acking enforcement actions

Likely ownership:
- Rust for critical paths
- TS acceptable for some drivers if authority is preserved

### C. Drivers
Responsibilities:
- session identity capture
- capability reporting
- launch wrapping or surface attachment
- warning/interruption/block implementation
- lease heartbeat
- local ack/nack reporting

### D. Telemetry Subscribers
Responsibilities:
- TUI
- UI
- logs
- diagnostics
- observability

## 6.2 Driver taxonomy

### shell-local
Primary for:
- local zsh/bash/fish
- local CLI agent sessions

### shell-remote
Primary for:
- remote shells over SSH
- remote tmux sessions
- remote dev-server agent workflows

### process-driver
Used for:
- process-group lifecycle control
- hard interrupt

### tmux-driver
Used for:
- pane/window/session metadata
- UX feedback
- fallback pane-oriented signalling

### editor-driver
Used for:
- editor-side session control
- save blocking
- warnings

### web-session-driver
Used for:
- hosted/browser-based agent sessions
- lease revocation
- soft interrupt
- future-action blocking

### mcp-driver
Used for:
- compatibility
- best-effort soft control
- non-primary fallback

## 7. Session Model

## 7.1 Session schema

```ts
type SessionDescriptor = {
  sessionId: string
  parentSessionId?: string
  executionHostId: string
  driverId: string
  driverType: DriverType
  toolSurface: string
  repoRoot?: string
  repoId?: string
  worktreeRoot?: string
  worktreeId?: string
  cwd?: string
  pid?: number
  pgid?: number
  paneId?: string
  windowId?: string
  environmentClass: "local" | "remote-shell" | "hosted-web" | "editor" | "other"
  capabilities: EnforcementCapabilities
}
```

## 7.2 Driver types

```ts
type DriverType =
  | "shell-local"
  | "shell-remote"
  | "process"
  | "tmux"
  | "editor"
  | "web-session"
  | "mcp"
  | "other"
```

## 7.3 Enforcement capabilities

```ts
type EnforcementCapabilities = {
  canWarn: boolean
  canInterruptSoft: boolean
  canInterruptHard: boolean
  canBlockFutureActions: boolean
  canBlockWrites: boolean
  canFenceSession: boolean
  canFenceWorktree: boolean
  canFenceRepo: boolean
  canKillProcessTree: boolean
  canSuspendSession: boolean
  canRevokeToken: boolean
}
```

## 7.4 Lease schema

```ts
type SessionLease = {
  leaseId: string
  sessionId: string
  state: "active" | "restricted" | "blocked" | "revoked"
  issuedAt: string
  expiresAt?: string
  revocationReason?: string
}
```

## 8. Change and Provenance Model

## 8.1 Canonical change event

```ts
type ChangeEvent = {
  changeId: string
  observedAt: string
  sequence: number
  executionHostId: string
  repoId?: string
  repoRoot?: string
  worktreeId?: string
  worktreeRoot?: string
  sessionId?: string
  parentSessionId?: string
  driverId?: string
  toolSurface?: string
  process?: {
    pid?: number
    ppid?: number
    pgid?: number
    executable?: string
    argv?: string[]
    cwd?: string
  }
  changes: FileDeltaRef[]
  cause: ChangeCause
  hotPathDeadlineMs: number
}
```

## 8.2 File delta

```ts
type FileDeltaRef = {
  path: string
  event: "create" | "modify" | "delete" | "rename"
  hashBefore?: string
  hashAfter?: string
}
```

## 8.3 Change cause

```ts
type ChangeCause =
  | "save"
  | "tool-write"
  | "generated-write"
  | "rename"
  | "delete"
  | "unknown"
```

## 9. Enforcement Model

## 9.1 Decision contract

```ts
type EnforcementDecision = "allow" | "warn" | "block" | "interrupt"
```

## 9.2 Decision result

```ts
type EnforcementResult = {
  changeId: string
  sessionId?: string
  executionHostId?: string
  decision: EnforcementDecision
  reasons: DecisionReason[]
  ackRequired: boolean
  escalation?: EnforcementEscalation
}
```

## 9.3 Decision reason

```ts
type DecisionReason = {
  policyId: string
  severity: "info" | "low" | "medium" | "high" | "critical"
  message: string
  file?: string
  symbol?: string
  source: "rule" | "regex" | "policy" | "graph-hot-read" | "other"
}
```

## 9.4 Escalation

```ts
type EnforcementEscalation =
  | "none"
  | "soft-interrupt"
  | "workflow-fence"
  | "hard-interrupt"
  | "capability-revocation"
```

## 9.5 Enforcement ladder

### Warn
- notify only
- no mandatory ack

### Block
- deny future actions or writes
- ack required if active driver intervention occurs

### Interrupt
Driver-specific action, potentially escalating through:
1. soft interrupt
2. session fence
3. worktree fence
4. process-group interrupt
5. hard kill or capability revocation

Ack required.

## 10. Driver Contract

## 10.1 Driver registration interface

```ts
interface DriverRegistration {
  registerSession(input: SessionDescriptor): Promise<SessionLease>
  unregisterSession(sessionId: string): Promise<void>
  heartbeat(sessionId: string): Promise<void>
  getCapabilities(sessionId: string): Promise<EnforcementCapabilities>
}
```

## 10.2 Enforcement interface

```ts
interface EnforcementDriver {
  warn(input: WarnCommand): Promise<AckResult>
  interruptSoft?(input: InterruptCommand): Promise<AckResult>
  interruptHard?(input: InterruptCommand): Promise<AckResult>
  blockFutureActions?(input: BlockCommand): Promise<AckResult>
  fenceSession?(input: FenceSessionCommand): Promise<AckResult>
  fenceWorktree?(input: FenceWorktreeCommand): Promise<AckResult>
  fenceRepo?(input: FenceRepoCommand): Promise<AckResult>
  killProcessTree?(input: KillCommand): Promise<AckResult>
  suspendSession?(input: SuspendCommand): Promise<AckResult>
  revokeToken?(input: RevokeTokenCommand): Promise<AckResult>
}
```

## 10.3 Ack result

```ts
type AckResult = {
  ok: boolean
  sessionId?: string
  driverId?: string
  action: string
  acknowledgedAt: string
  detail?: string
}
```

## 11. Shell Driver Design

## 11.1 Why shell is a primary driver
A shell-driven workflow gives Anvil:

- universal ingress for local CLI agent sessions
- repo/worktree and cwd context
- process and process-group lineage
- consistent launch wrapper semantics
- practical interruption and relaunch control

## 11.2 Shell launch pattern

Agent commands should be wrapped through Anvil rather than launched directly.

Example conceptual pattern:

```bash
anvil-shell-launch --tool claude --cwd "$PWD" --tmux-pane "$TMUX_PANE" -- "$@"
```

The launcher should:
1. resolve repo/worktree context
2. create session descriptor
3. register with control authority
4. receive lease
5. launch process in its own process group
6. maintain heartbeat and local control hooks
7. block launch if worktree or repo is fenced

## 11.3 Shell-first but not shell-only
The shell driver should be combined with:
- process-group control
- tmux metadata support
- host-local enforcement daemon
- optional editor/MCP/web fallback drivers

## 12. tmux Driver Design

tmux should not generally be the sole enforcement owner, but it is valuable for:

- pane/session/window identity
- attaching session metadata
- surfacing warnings in the right pane
- sending targeted messages
- providing fallback interventions where appropriate

Potential captured fields:
- tmux session id
- tmux window id
- tmux pane id
- pane title
- remote/local indicator if available

## 13. Remote Host Design

## 13.1 Remote shell control
A remote SSH workflow should be treated as a session on a different execution host.

This implies:
- Anvil sidecar or daemon on the remote host
- remote shell driver
- remote watcher ownership and process control
- session registration routed back to the control authority

## 13.2 Host identifiers
Each host must have a stable `executionHostId`.

Examples:
- `host_local_macbook`
- `host_devserver_01`
- `host_claude_web_env_abc`

## 13.3 Remote control principle
The local machine should not pretend to directly own remote process control. It should route enforcement to the remote host-local enforcement point.

## 14. Hosted/Web Session Design

Hosted or browser-based agent environments generally have weaker enforcement capabilities.

Therefore these drivers should focus on:
- soft interrupt where supported
- lease revocation
- blocking future actions or accepted outputs
- warnings and quarantining
- capability-relative enforcement

Hard interrupt is not assumed.

## 15. Transport Design

## 15.1 Control lane
Use JSON-RPC 2.0 over:
- Unix domain sockets on Unix-like systems
- named pipes on Windows

Characteristics:
- small request/response messages
- bounded timeout
- explicit ack semantics
- targetable commands

Example methods:
- `session.register`
- `session.unregister`
- `session.heartbeat`
- `watch.change`
- `enforcement.warn`
- `enforcement.block`
- `enforcement.interrupt`
- `enforcement.ack`

## 15.2 Telemetry lane
Use NDJSON or equivalent event stream.

Characteristics:
- non-blocking
- best-effort
- lossy tolerant
- suitable for TUI/UI/logs/diagnostics

Example event families:
- progress
- snapshot
- policy violation telemetry
- diagnostics
- health

## 15.3 Why not gRPC
Not selected because the dominant problem is local control, host-local enforcement, and explicit authority. gRPC adds complexity without materially solving those issues.

## 16. Hot Path Design

## 16.1 Ownership
The hot path should be Rust-owned.

Responsibilities:
- watch ingestion
- coalescing/debouncing
- provenance stamping
- session lookup
- deterministic checks
- cheap graph reads
- decision generation
- routing to the owning driver

## 16.2 Graph use
Only cheap graph reads should be allowed on the hot path.

### Tier 0
Always safe:
- regex
- static path policies
- rule matching
- secret detection

### Tier 1
Allowed if warmed and constant-time or near-constant-time:
- boundary membership lookup
- symbol ownership lookup
- known-edge existence
- precomputed architectural index checks

### Tier 2
Not hot-path eligible:
- full graph recompute
- expensive transitive analysis
- enrichment and explanation workloads

## 17. Routing and Ownership Resolution

## 17.1 Routing key
At minimum, route by:
- execution host id
- session id
- worktree id
- driver id

## 17.2 Ownership hierarchy
Preferred ownership order:
1. explicit session mapping from launch registration
2. host-local process/pgid mapping
3. worktree-scoped inference
4. fallback to lower-authority drivers for warning/fencing only

## 17.3 Targeted enforcement
Do not broadcast interrupt notifications. Route to the owning driver on the owning host.

## 18. Fencing Model

## 18.1 Fence scopes
- session fence
- worktree fence
- repo fence
- capability fence

## 18.2 Fence effects
Examples:
- deny new launches
- deny further writes
- reject future tool calls
- require manual unblock
- revoke lease
- quarantine session outputs

## 19. Failure Modes and Fallbacks

### Driver unavailable
- downgrade to warning or host-level fence if possible
- emit telemetry
- preserve audit trail

### Remote host offline
- revoke lease centrally
- mark session unresolved
- quarantine outputs if needed

### Hosted environment weak interrupt
- prefer lease revocation and future-action blocking
- use soft interrupt where supported

### Ambiguous ownership
- prefer safe fencing at worktree scope over incorrect process kill

## 20. Security and Trust Notes

- drivers must authenticate to the control authority
- leases should be scoped to host and session
- control actions should be auditable
- high-severity interrupt actions should include reason metadata
- remote host trust boundaries must be explicit

## 21. Recommended Initial Implementation

### Phase 1
- define schemas for host, session, lease, capabilities, change event, decision
- split control and telemetry lanes
- implement shell-local driver
- implement process-group control
- implement session registry

### Phase 2
- implement tmux metadata driver
- add session lease enforcement
- add session/worktree fencing
- add ack semantics for block/interrupt

### Phase 3
- implement remote host-local sidecar/daemon
- support shell-remote driver over SSH/dev-server environments
- add host registry

### Phase 4
- implement editor and web-session drivers
- keep MCP as fallback driver only

### Phase 5
- evaluate stronger write-fencing/interception strategies if still needed

## 22. Open Questions

- how strongly should remote control authority be centralised versus host-local autonomy?
- should lease state be cached locally for fail-safe host behaviour?
- what is the preferred auth mechanism between authority and host-local enforcement points?
- how far should write fencing go before considering deeper FS interception?
- what minimum capabilities must a driver expose to be considered enforcement-grade?

## 23. Summary

This design changes Anvil from a passive watch-and-notify architecture into a **driver-based, host-aware enforcement control plane**.

The key architectural moves are:

- use drivers rather than passive adapters
- treat execution host as first-class
- use shell and process control as primary authority surfaces for CLI workflows
- use leases and fences as core control mechanisms
- keep Rust in charge of the hot path
- use JSON-RPC over local sockets for control
- keep telemetry separate
- treat MCP and hosted/web surfaces as lower-authority but still useful drivers
