# Anvil Intercept Loop — Architecture Summary

**Planning Council:** plan-ba94d8a5  
**Prior Review:** council-3695036e  
**Date:** 2026-04-03  
**Status:** Proposed  
**Full specs:** `plans/specs/anvil-driver-framework/`  
**APS modules:** INTD, INTL, INTR  
**Architecture decision:** `plans/decisions/015-intercept-loop-enforcement.md`

---

## The Thesis

Anvil can already detect policy violations at save time. The Intercept Loop
proves it can **stop them** -- interrupting the right AI agent session, on the
right machine, fast enough to matter.

Detection without enforceable authority is only observability. The intercept loop
turns Anvil from a passive watcher into an active enforcement system.

---

## What We're Building

Three Rust crates that compose into a local enforcement loop:

```
                          Developer Machine
                          ==================

  claude ─┐
  codex  ─┤  zsh alias    ┌─────────────────┐
  gemini ─┤──────────────▶│    anvil-run     │
  aider  ─┘               │                  │
                           │ 1. resolve cwd   │
                           │ 2. check fences  │
                           │ 3. register sess │
                           │ 4. setpgid+exec  │
                           │ 5. heartbeat     │
                           └────────┬─────────┘
                                    │ NDJSON / IPC
                                    ▼
                           ┌─────────────────────────┐
                           │   anvil-intercept        │
                           │   (daemon singleton)     │
                           │                          │
                           │  Session    Fence Store  │
                           │  Registry   (disk)       │
                           │      │          │        │
  File System ────────────▶│  Enforcement Pipeline    │
  Watcher     ChangeBatch  │  ┌────────────────────┐  │
  (notify)                 │  │   Rule Registry    │  │
                           │  │ (anvil-intercept   │  │
                           │  │      -rules)       │  │
                           │  └────────┬───────────┘  │
                           │           ▼              │
                           │   allow / interrupt      │
                           │           │              │
                           │     Mode Mapping         │
                           │   warn  → log            │
                           │   fence → block worktree │
                           │   interrupt → verify     │
                           │     PGID → SIGINT →      │
                           │     SIGTERM → SIGKILL     │
                           └──────────────────────────┘
```

---

## The Loop in 7 Steps

1. User runs `claude` -- zsh alias calls `anvil-run --tool claude -- claude`
2. anvil-run connects to daemon, checks worktree not fenced
3. anvil-run registers session (tool, worktree, cwd), spawns agent in own PGID
4. Agent writes files in the worktree
5. Daemon's watcher detects changes, resolves owning session by worktree
6. Enforcement pipeline runs cheap rules (secret scan, antipattern, path deny)
7. If violation: maps through enforcement mode -- warn / fence / kill(PGID)

---

## Three Crates

| Crate | Type | Responsibility |
|-------|------|---------------|
| **anvil-intercept-rules** | Library | `InterceptRule` trait + rule registry. Wraps existing `anvil-checks` (secrets, antipatterns). Adds path-deny and regex-content rules. Pure logic, no I/O. |
| **anvil-intercept** | Daemon binary + library | Singleton daemon. Owns: session registry, watcher fan-out, enforcement pipeline, process-group interrupt, fence persistence, IPC listener, config loading. Also exposes library API for embedded/CI mode. |
| **anvil-run** | Launcher binary | Session ingress. Wraps agent launch: resolve context, check fences, register, `setpgid` + exec, heartbeat, cleanup. Shell functions delegate here. |

Shared types (IPC messages, session model) live in `anvil-intercept-proto` -- a
module within `anvil-intercept` that both the daemon and launcher depend on.

---

## The Rule Pipeline

Rules are cheap and deterministic. No graph, no AST, no network calls.

```
ChangeBatch (file paths + events)
    │
    ▼ read content (<=1MB, skip binary)
    │
    ├── SecretDetection    ← wraps existing anvil-checks
    ├── AntipatternScan    ← wraps existing anvil-checks
    ├── PathDenyList       ← configurable glob patterns
    └── RegexContent       ← configurable regex (compiled once)
    │
    ▼
  allow | interrupt
    │
    ▼ map through .anvil.yaml enforcement.mode
    │
    ├── warn mode      → log violation, continue
    ├── fence mode     → block worktree, prevent relaunches
    └── interrupt mode → verify PGID → SIGINT → SIGTERM → SIGKILL
```

---

## Configuration

```yaml
# .anvil.yaml (per-project, committed to repo)
enforcement:
  mode: fence              # warn | fence | interrupt
  on_ambiguous_ownership: warn   # warn | fence (never interrupt)
  observe_only: false      # dry-run: log everything, enforce nothing
```

Daemon also reads `~/.config/anvil/daemon.toml` for user-level config. **Stricter
wins** when merging.

---

## Session Registration -- Two Paths

### Path A: anvil-run (full authority)

anvil-run spawns the agent in a dedicated process group via `setpgid`. Registers
session with daemon including PGID and process start time.

Capabilities: warn, fence, interrupt (SIGINT -> SIGTERM -> SIGKILL).

### Path B: Hook side-channel (fence authority only)

Claude Code PreToolUse hook calls `register-session` with the current context.
The PID belongs to the agent process itself, not a controlled wrapper.

Capabilities: warn, fence only. No interrupt -- cannot safely SIGINT the agent's
own process group without killing the user's terminal session.

### Path C: Unregistered (no authority)

Unknown process writes a file the daemon is watching. Tagged
`attribution:unknown-agent`.

Capabilities: warn or fence per config. No interrupt, no session to target.

---

## Safety Invariants

Hard-coded in the daemon, not configurable:

1. **Never kill on ambiguous ownership** -- fence the worktree instead
2. **Verify PGID before signalling** -- check process start time defeats
   PID/PGID reuse
3. **Fences never auto-clear** -- manual `anvil worktree unblock` required,
   survives daemon restarts
4. **Fence immediately on any enforcement failure** -- signal didn't land? fence
   anyway
5. **Hook-registered sessions are fence-only** -- no interrupt capability

---

## Cross-Platform

| Concern | Linux | macOS | Windows |
|---------|-------|-------|---------|
| IPC | Unix domain socket | Unix domain socket | Named pipe (with DACL) |
| Process isolation | `setpgid` | `setpgid` | Named Job Objects |
| Signal ladder | SIGINT -> SIGTERM -> SIGKILL | SIGINT -> SIGTERM -> SIGKILL | `TerminateJobObject` |
| PGID verification | `/proc/<pid>/stat` | `proc_pidinfo` | `GetProcessTimes` |
| Fence storage | `$XDG_DATA_HOME/anvil/` | `~/Library/Application Support/anvil/` | `%LOCALAPPDATA%\anvil\` |
| Socket path | `$XDG_RUNTIME_DIR/anvil/` | `$TMPDIR/anvil-<uid>/` | `\\.\pipe\anvil-<user>` |

---

## Architecture Decisions (summary)

| # | Decision | Key Choice |
|---|----------|-----------|
| AD-1 | Process-group management | `setpgid` for agents, `setsid` for daemon, Job Objects on Windows |
| AD-2 | Unregistered sessions | Hook side-channel + fence-on-unknown |
| AD-3 | Configurable enforcement | `.anvil.yaml` with warn/fence/interrupt + observe_only |
| AD-4 | IPC transport | NDJSON over UDS / named pipes |
| AD-5 | Daemon lifecycle | Per-user singleton, PID file, persistent fences, embedded CI mode |
| AD-6 | Rule integration | Binary rule output mapped through enforcement mode |
| AD-7 | PGID verification | Verify ownership + start time before signalling |

---

## What's NOT in v1

- No remote hosts, no sidecar deployment
- No driver framework or capability negotiation
- No session leases with expiry
- No graph-assisted checks on the hot path
- No editor or MCP-native drivers (MCP hook is stretch goal)
- No per-rule enforcement granularity
- No TUI for daemon status (CLI commands only)
- No split control/telemetry transport lanes
- No ambient process sentinel scanner (v2)

---

## Build Order

```
Phase 1: INTR (7 tasks)  ← rules trait + wrappers, no dependencies
Phase 2: INTD (12 tasks) ← daemon, depends on INTR
Phase 3: INTL (9 tasks)  ← launcher, depends on INTD IPC
```

28 work items total across 3 APS modules. All Draft.
