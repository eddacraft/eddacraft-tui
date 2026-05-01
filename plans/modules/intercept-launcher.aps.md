# Intercept Launcher

| ID   | Owner  | Status | Progress |
| ---- | ------ | ------ | -------- |
| INTL | @aneki | Draft  | 0/9      |

**Last reviewed:** 2026-04-26

## Purpose

The intercept launcher is the session ingress boundary for the Anvil Intercept
Loop. It wraps agent process launches in a controlled environment: registering
sessions with the daemon, placing child processes in dedicated process groups
(or Job Objects on Windows), checking worktree fence state before launch, and
cleaning up on exit. Shell integration functions delegate to this binary for
consistent launch semantics across tools.

## In Scope

- Launcher binary (`anvil-run`) that wraps arbitrary commands
- Session ID generation and context resolution (cwd, repo root, worktree root,
  tmux pane if present)
- Daemon connectivity check with fail-closed behaviour
- Worktree fence check before launch (refuse if fenced)
- Session registration via daemon IPC
- Child process launch in dedicated process group (`setpgid` on Unix, Job Object
  on Windows)
- PID/PGID reporting to daemon after launch
- Session unregistration on child exit
- Shell integration functions (zsh, bash) that alias tool commands through
  anvil-run
- Hook side-channel registration for sessions not launched via anvil-run
  (Claude Code PreToolUse hook)
- Cross-platform support: Linux, macOS, Windows

## Out of Scope

- Daemon lifecycle management (start/stop/restart is INTD's responsibility)
- Rule evaluation or enforcement decisions
- tmux-specific UX (pane messaging, status line markers)
- Editor or MCP integration
- Remote session launch
- Fish shell integration (can be added later)

## Interfaces

- **Depends on:** anvil-intercept-proto (shared NDJSON message types, session
  model, IPC command enum), intercept-daemon (INTD, IPC interface for
  registration and fence queries)
- **Exposes:** `anvil-run` binary for shell wrappers; hook registration script
  for side-channel integration; shell function definitions for sourcing

## Tasks

### INTL-001: Launcher Binary Scaffold

- **Intent:** Create the `anvil-run` binary that parses arguments, resolves
  execution context, and delegates to the daemon for session management
- **Expected Outcome:** A `crates/anvil-run/` binary crate that accepts
  `--tool <name> -- <command...>` and resolves cwd, repo root, and worktree
  root; added to root workspace
- **Validation:** `cargo build -p eddacraft-anvil-run && anvil-run --help`
- **Status:** Draft

### INTL-002: Daemon Connectivity and Fence Check

- **Intent:** Verify that the daemon is reachable and the target worktree is
  not fenced before launching an agent
- **Expected Outcome:** Launcher connects to daemon IPC, queries worktree
  status; if daemon unreachable or worktree fenced, launch is refused with a
  clear error message
- **Validation:** `cargo test -p eddacraft-anvil-run --lib preflight`
- **Status:** Draft

### INTL-003: Session Registration Flow

- **Intent:** Register a new session with the daemon before spawning the child
  process
- **Expected Outcome:** Launcher generates a session ID, sends registration
  (tool, worktree, cwd, tmux pane), receives acknowledgement, then proceeds
  to spawn
- **Validation:** `cargo test -p eddacraft-anvil-run --lib register`
- **Status:** Draft

### INTL-004: Process-Group Child Launch

- **Intent:** Spawn the wrapped command in its own process group so the daemon
  can target it for interruption
- **Expected Outcome:** Child process launched with `setpgid(child, child)` on
  Unix; on Windows, a named Job Object created with a deterministic name derived
  from the session ID (not raw HANDLE) -- launcher sends the object name to the
  daemon, which opens the named Job Object independently; launcher reports PID,
  PGID (Unix) or Job Object name (Windows), and process start time to daemon;
  launcher waits for child exit
- **Validation:** `cargo test -p eddacraft-anvil-run --lib spawn`
- **Status:** Draft

### INTL-005: Session Cleanup on Exit

- **Intent:** Unregister the session with the daemon when the child process
  exits, regardless of exit reason
- **Expected Outcome:** On normal exit, signal-based exit, or launcher crash
  (via drop guard), session unregistration sent to daemon; daemon marks session
  ended
- **Validation:** `cargo test -p eddacraft-anvil-run --lib cleanup`
- **Status:** Draft

### INTL-006: Shell Integration Functions

- **Intent:** Provide shell functions that transparently route tool commands
  through anvil-run for consistent session management
- **Expected Outcome:** A sourceable shell script providing functions for
  common tools (e.g. `claude`, `codex`) that delegate to `anvil-run --tool
  <name> -- "$@"`; works in zsh and bash
- **Validation:** Manual: source script, verify function wraps command
- **Status:** Draft

### INTL-007: Hook Side-Channel Registration

- **Intent:** Allow sessions not launched via anvil-run to register with the
  daemon through a lightweight hook mechanism
- **Expected Outcome:** A hook-compatible script or binary that Claude Code
  PreToolUse hooks can invoke to call `daemon register-session` with the
  current context; session inherits the calling process's PID/PGID;
  hook-registered sessions have their maximum enforcement action downgraded to
  fence-only, because the calling PID belongs to the agent process itself rather
  than a controlled wrapper PGID -- the daemon enforces this at registration
  time
- **Validation:** `cargo test -p eddacraft-anvil-run --lib hook`
- **Status:** Draft

### INTL-008: Blocked Launch UX

- **Intent:** Provide clear, actionable output when a launch is refused due
  to a fenced worktree or unavailable daemon
- **Expected Outcome:** Launcher prints the fence reason, affected worktree
  path, and the command needed to unblock; exit code distinguishes fence
  refusal from daemon-unavailable refusal
- **Validation:** Manual: attempt launch in fenced worktree, verify output
- **Status:** Draft

### INTL-009: Session Heartbeat

- **Intent:** Keep the daemon informed that the launcher and child process are
  still alive, enabling stale session reaping for crashed launchers
- **Expected Outcome:** Launcher sends periodic heartbeats to the daemon while
  the child process is running; heartbeat interval is well within the daemon's
  30s TTL window
- **Validation:** `cargo test -p eddacraft-anvil-run --lib heartbeat`
- **Status:** Draft
