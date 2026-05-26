# Intercept Launcher

| ID   | Owner  | Status | Progress |
| ---- | ------ | ------ | -------- |
| INTL | @aneki | Complete | 9/9 |

**Last reviewed:** 2026-05-14 (PR #1528 merged via rebase at
`5d38e546` — `crates/anvil-run/` shipped with INTL-001..-009 covered by
49 unit + 3 shell-integration tests. Schema status moved
**In Progress → Done**; all nine task `Status:` lines moved
**In Progress → Done** so the APS drift / progress tooling counts them
as finished. Narratively the module is **Merged** in the
[`plans/aps-rules.md`](../../aps-rules.md) lifecycle — the cleanup agent
advances **Merged → Released/Shipped → Complete/Archived** when
release-record evidence from the `v0.7.0-beta` runbook lands, at which
point this row will be archived to `plans/archive/modules/`. Two QoL
follow-ups deferred to #1529 (foreground TTY passing + blocked-launch
shell quoting) — they do not block the release claim.)

**Earlier:** 2026-05-13 (Wave 0 readiness review — `AgentTag` stub landed in
`crates/anvil-intercept-proto/src/session.rs`; INTL-003 and INTL-004 promoted
to task-Ready; the other seven tasks remained Draft pending their direct
prerequisites. Module-level **Ready** meant "ready to begin Wave 3
implementation" — not "all individual tasks reviewed". Wave 3 starts after
MLP-002 ships the witness primitive — true today.)

## AgentTag and Session Interface (cross-reference)

INTL is the ingress that produces the `AgentTag` minted by the daemon per
MLP-014. The stub type lives in
[`crates/anvil-intercept-proto/src/session.rs`](../../../crates/anvil-intercept-proto/src/session.rs)
(landed 2026-05-13 as part of Wave 0 closure); behavioural use comes with
MLP-014 (registry key change) and INTL-003 / INTL-004 (launcher-side
propagation). Concretely:

- INTL-003 registers a session keyed by `(WorktreeKey, AgentTag)` rather than
  just `WorktreeKey`; the launcher provides `driver_id`, `claimed_agent_id`,
  and `pid_starttime` so the daemon can mint the tag.
- INTL-004 sets `ANVIL_TASK_ID` and `ANVIL_AGENT_TAG` on the child process
  environment before `exec` (constants: `ANVIL_TASK_ID_ENV`,
  `ANVIL_AGENT_TAG_ENV`), so MLP-014's attribution chain can recover the tag
  from any descendant via the env or, on miss, via process-tree walk.
- INTL-007 side-channel registrations inherit a downgraded `AgentTag` and are
  capped to fence-only enforcement (see `degraded:fence-cascade` mode in
  MLP-014).

### Trust model (read before implementing)

Env vars are **advisory hints, not authenticated identity**. Any same-UID
process can spoof or unset them. The daemon MUST:

1. Cross-check an env-supplied `AgentTag` against the `AgentTag` it issued
   for this pid lineage at INTL-003. A tag that doesn't match the
   registration is treated as missing, not honoured.
2. Fall through to the process-tree walk on env miss (MLP-014); a walk that
   finds no registered ancestor downgrades to worktree-level fence per
   ADR-038 noise-discipline (one terse line, then silent).
3. Treat the witness chain (ADR-037 §D-2) and `validate_at_l4` (ADR-037 §D-5)
   as the authentication backstop — env propagation is correctness for the
   normal path, not a security boundary.

This contract is the carry-forward gate confirmed in Wave 0 of the
[release plan](../../../RELEASE-PLAN.md#wave-0-promote-contracts).

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
- Remote session launch except as coordinated by future SSHREMOTE work
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
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

### INTL-002: Daemon Connectivity and Fence Check

- **Intent:** Verify that the daemon is reachable and the target worktree is
  not fenced before launching an agent
- **Expected Outcome:** Launcher connects to daemon IPC, queries worktree
  status; if daemon unreachable or worktree fenced, launch is refused with a
  clear error message
- **Validation:** `cargo test -p eddacraft-anvil-run --lib preflight`
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

### INTL-003: Session Registration Flow

- **Intent:** Register a new session with the daemon before spawning the child
  process
- **Expected Outcome:** Launcher generates a session ID, sends registration
  (tool, worktree, cwd, tmux pane, `driver_id`, `claimed_agent_id`,
  `pid_starttime`), receives the daemon-minted `AgentTag` (MLP-014) and the
  acknowledgement, then proceeds to spawn. Registration keys the session as
  `(WorktreeKey, AgentTag)` to align with MLP-014.
- **Validation:** `cargo test -p eddacraft-anvil-run --lib register`
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

### INTL-004: Process-Group Child Launch

- **Intent:** Spawn the wrapped command in its own process group so the daemon
  can target it for interruption
- **Expected Outcome:** Child process launched with `setpgid(child, child)` on
  Unix; on Windows, a named Job Object created with a deterministic name derived
  from the session ID (not raw HANDLE) -- launcher sends the object name to the
  daemon, which opens the named Job Object independently; launcher reports PID,
  PGID (Unix) or Job Object name (Windows), and process start time to daemon;
  launcher waits for child exit. Before `exec`, launcher sets
  `ANVIL_TASK_ID` and `ANVIL_AGENT_TAG` on the child env so MLP-014 attribution
  chain works through descendants; absence of those vars triggers the
  process-tree walk fallback at daemon side.
- **Validation:** `cargo test -p eddacraft-anvil-run --lib spawn`
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

### INTL-005: Session Cleanup on Exit

- **Intent:** Unregister the session with the daemon when the child process
  exits, regardless of exit reason
- **Expected Outcome:** On normal exit, signal-based exit, or launcher crash
  (via drop guard), session unregistration sent to daemon; daemon marks session
  ended
- **Validation:** `cargo test -p eddacraft-anvil-run --lib cleanup`
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

### INTL-006: Shell Integration Functions

- **Intent:** Provide shell functions that transparently route tool commands
  through anvil-run for consistent session management
- **Expected Outcome:** A sourceable shell script providing functions for
  common tools (e.g. `claude`, `codex`) that delegate to `anvil-run --tool
  <name> -- "$@"`; works in zsh and bash
- **Validation:** Manual: source script, verify function wraps command
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

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
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

### INTL-008: Blocked Launch UX

- **Intent:** Provide clear, actionable output when a launch is refused due
  to a fenced worktree or unavailable daemon
- **Expected Outcome:** Launcher prints the fence reason, affected worktree
  path, and the command needed to unblock; exit code distinguishes fence
  refusal from daemon-unavailable refusal
- **Validation:** Manual: attempt launch in fenced worktree, verify output
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)

### INTL-009: Session Heartbeat

- **Intent:** Keep the daemon informed that the launcher and child process are
  still alive, enabling stale session reaping for crashed launchers
- **Expected Outcome:** Launcher sends periodic heartbeats to the daemon while
  the child process is running; heartbeat interval is well within the daemon's
  30s TTL window
- **Validation:** `cargo test -p eddacraft-anvil-run --lib heartbeat`
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
