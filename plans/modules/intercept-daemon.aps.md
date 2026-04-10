# Intercept Daemon

| ID | Owner | Status |
|----|-------|--------|
| INTD | @aneki | Draft |

## Purpose

The intercept daemon is the core enforcement authority for the Anvil Intercept
Loop. It runs as a per-user persistent singleton, maintains the session registry,
ingests file-system change events, evaluates deterministic rules, resolves change
ownership, and issues enforcement decisions. It exposes an NDJSON-over-IPC
interface for session registration, heartbeats, and status queries. Fence state is
persisted to disk to survive restarts.

## In Scope

- Daemon binary with start/stop/restart lifecycle (PID file, signal handling)
- NDJSON-over-UDS (Linux/macOS) and named-pipe (Windows) IPC listener
- In-memory session registry (register, unregister, heartbeat, list)
- File-system watcher integration (fan out from existing ChangeBatch channel)
- Change-to-session ownership resolution (worktree-based, single session per
  worktree for v1)
- Enforcement decision pipeline (evaluate rules, resolve owner, issue decision)
- Process-group interrupt ladder (SIGINT -> SIGTERM -> SIGKILL with timeouts;
  Job Object termination on Windows)
- PGID/Job Object ownership verification before signal delivery
- Worktree fence state (in-memory + disk-persisted)
- Blocked-worktree query and manual unblock commands
- Enforcement configuration loading from `.anvil.yaml`
- Embedded mode (in-process, no socket) for CI environments
- Cross-platform support: Linux, macOS, Windows

## Out of Scope

- Remote host enforcement or sidecar deployment
- Full driver framework or driver capability negotiation
- Session leases with expiry and renewal
- Dual-lane transport (control vs telemetry split)
- Graph-assisted hot-path checks
- MCP as a control surface
- Editor or web-session drivers
- Per-rule enforcement granularity
- TUI or dashboard surfaces for daemon status

## Interfaces

- **Depends on:** anvil-intercept-proto (shared NDJSON message types, session
  model, IPC command enum), anvil-checks (secret detection, antipattern
  scanning), anvil-kernel (watcher ChangeBatch channel), intercept-rules (INTR,
  rule trait and rule set)
- **Exposes:** IPC interface for session lifecycle and worktree status; in-process
  API for embedded mode; disk-persisted fence state readable by launcher

## Tasks

### INTD-001: Daemon Binary Scaffold

- **Intent:** Establish the daemon binary crate with signal handling, PID file
  management, and graceful shutdown
- **Expected Outcome:** A `crates/anvil-intercept/` crate that starts, writes a
  PID file, handles SIGTERM/SIGINT, and exits cleanly on all three platforms;
  an `anvil-intercept-proto` library module (or sibling crate) is created first
  containing NDJSON message types, session model structs, and IPC command enum
  shared by both daemon and launcher
- **Validation:** `cargo build -p eddacraft-anvil-intercept && cargo test -p eddacraft-anvil-intercept`
- **Status:** Draft

### INTD-002: IPC Listener

- **Intent:** Accept NDJSON connections over Unix domain sockets (Linux/macOS)
  and named pipes (Windows) with restricted permissions
- **Expected Outcome:** Daemon listens on a platform-appropriate socket path,
  accepts connections, parses NDJSON frames, and dispatches to command handlers
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib ipc`
- **Status:** Draft

### INTD-003: Session Registry

- **Intent:** Maintain an in-memory registry of active sessions with their
  worktree mappings, PIDs, PGIDs, and status
- **Expected Outcome:** Sessions can be registered, updated with process info,
  queried by worktree, listed, and unregistered; single session per worktree
  constraint enforced; session heartbeat TTL enforced -- if no heartbeat received
  within 30s, session marked ended and removed from active registry (handles
  crashed launchers that never call unregister, since Drop guards do not fire on
  SIGKILL or TerminateProcess)
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib registry`
- **Status:** Draft

### INTD-004: Watcher Integration

- **Intent:** Consume file-system change events and correlate them to registered
  sessions via worktree mapping
- **Expected Outcome:** Change batches from the watcher are received, coalesced,
  and forwarded to the enforcement pipeline with session attribution
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib watcher`
- **Status:** Draft

### INTD-005: Enforcement Decision Pipeline

- **Intent:** Evaluate intercepted changes against the configured rule set and
  produce allow or interrupt decisions
- **Expected Outcome:** Changes flow through registered rules; first violation
  short-circuits to an interrupt decision; decisions include the triggering rule
  and affected file paths; content reading is performed in the enforcement
  pipeline before rule evaluation, with a hard size cap (1 MB) above which
  content-dependent rules are skipped; binary detection (null byte check)
  short-circuits content rules; deleted files pass only path-based rules
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib enforcement`
- **Status:** Draft

### INTD-006: Process-Group Interrupt

- **Intent:** Deliver interrupt signals to the correct process group with
  verification and escalation
- **Expected Outcome:** PGID ownership verified before signalling: process group
  leader PID matches stored session PID AND process creation time matches stored
  start time (defeats PGID/PID reuse; Linux: /proc/PID/stat starttime, macOS:
  proc_pidinfo, Windows: GetProcessTimes); SIGINT sent first, then SIGTERM after
  timeout, then SIGKILL as last resort; on Windows, Job Object termination used;
  fence applied immediately on any delivery failure
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib interrupt`
- **Status:** Draft

### INTD-007: Fence Persistence

- **Intent:** Persist blocked-worktree state to disk so fences survive daemon
  restarts
- **Expected Outcome:** Fence state written to a platform-appropriate user data
  directory; fences survive until manually unblocked via explicit command,
  regardless of session liveness -- auto-clear is never performed; on daemon
  restart, fences are loaded from disk and re-asserted before accepting
  connections
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib fence`
- **Status:** Draft

### INTD-008: Configuration Loading

- **Intent:** Read enforcement configuration from project `.anvil.yaml` and
  user-level config, merging with stricter-wins semantics
- **Expected Outcome:** Daemon resolves mode (warn/fence/interrupt),
  on_ambiguous_ownership (warn/fence), and observe_only flag per worktree;
  ambiguous ownership hard-capped at fence regardless of config
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib config`
- **Status:** Draft

### INTD-009: Embedded Mode

- **Intent:** Allow the enforcement pipeline to run in-process without socket
  setup for CI and testing environments
- **Expected Outcome:** A library API that accepts change events and returns
  decisions synchronously, reusing the same rule evaluation and session logic
  as the daemon
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib embedded`
- **Status:** Draft

### INTD-010: Unregistered Change Handling

- **Intent:** Handle file changes that cannot be attributed to any registered
  session safely
- **Expected Outcome:** Unattributed changes tagged `attribution:unknown-agent`;
  enforcement policy applied (warn or fence per configuration); worktree fenced
  if configured for fence-on-unknown
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib unregistered`
- **Status:** Draft

### INTD-011: Daemon Status and Diagnostics

- **Intent:** Expose daemon health, active sessions, and fence state for
  debugging and operational visibility
- **Expected Outcome:** IPC commands for session list, worktree status, fence
  list, and daemon health; output suitable for consumption by the launcher and
  future CLI status commands
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib status`
- **Status:** Draft

### INTD-012: Windows CI Matrix

- **Intent:** Ensure all intercept crates build and pass tests on Windows from
  day one, preventing platform parity regressions
- **Expected Outcome:** All intercept crates (anvil-intercept, anvil-run,
  anvil-intercept-rules) added to the windows-latest matrix in
  `.github/workflows/rust.yml`; this task blocks all other tasks from being
  marked Complete
- **Validation:** `gh run list --workflow=rust.yml` shows passing Windows jobs
- **Status:** Draft
