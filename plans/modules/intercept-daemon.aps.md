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
  rule trait and rule set), notification-framework (NOTIFY — canonical
  notification taxonomy and telemetry stream contract)
- **Exposes:** IPC interface for session lifecycle and worktree status; in-process
  API for embedded mode; disk-persisted fence state readable by launcher;
  telemetry-lane notification stream mirroring control-lane decisions

## Notification Model Integration

Enforcement decisions (`allow` / `warn` / `block` / `interrupt`) are mirrored
onto the telemetry lane as canonical notifications so operators, TUIs, and
observability subscribers see the same state transitions the drivers act on.
The mapping is fixed:

| Control decision | Notification class | Priority |
| --- | --- | --- |
| `allow` | usually no notification, else `info` | `low` |
| `warn` | `warning` | `high` |
| `block` | `block` (+ `fence-state` on worktree apply) | `critical` |
| `interrupt` | `interrupt` (+ `fence-state` on worktree apply) | `critical` |

All emitted telemetry events follow the envelope defined in
`plans/specs/2026-04-22-notification-telemetry-stream-contract.md`, with
`correlation.source = "intercept"` and the `mirror` object set to the
originating control decision, driver id, and ack requirement. Fence state
changes additionally carry `grouping.transition` (e.g. `active -> fenced`,
`fenced -> active`) so subscribers can distinguish transitions from repeat
polls.

Intercept-facing tasks must not invent parallel event vocabulary. Block,
interrupt, and fence events use the canonical classes above; anything that
does not fit becomes a `health` notification on the same stream rather than
a new lane.

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

- **Intent:** Accept NDJSON connections over Unix domain sockets
  (Linux/macOS) and named pipes (Windows) with restricted permissions
- **Expected Outcome:** Daemon listens on a platform-appropriate socket
  path, accepts connections, parses NDJSON frames, and dispatches to
  command handlers. Socket / pipe creation is pinned end-to-end, not
  left to umask / default DACL:
  - Unix: `mkdir $XDG_RUNTIME_DIR/anvil` with mode 0700 and
    `O_NOFOLLOW` (refuse if exists with wrong owner or mode); `bind()`
    inside that dir; `fchmod` the socket fd to 0600 before `listen()`.
    If `$XDG_RUNTIME_DIR` is unset, fall through to
    `$HOME/.local/state/anvil/` with the same mode guard — never
    `/tmp`.
  - Windows: named pipe created with an explicit `SECURITY_DESCRIPTOR`
    (owner = current user SID, DACL = generic-all-owner-only),
    `PIPE_REJECT_REMOTE_CLIENTS` set.
  - Driver-side (enforced by `DriverClient` in DRVR-001): stat / open
    the socket/pipe with `O_NOFOLLOW` equivalent and refuse if not
    owned by the current user, to defend against pipe-squatting even
    when the daemon side is correct.
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib ipc`
  plus permission-creation unit tests on each platform (Linux/macOS
  permission bits; Windows ACL).
- **Status:** Draft
- **Council review (2026-04-24):** M8 (security-analyst) pinned the
  end-to-end creation sequence above; see
  `plans/specs/2026-04-24-adr-030-council-findings.md`.

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

### INTD-013: Mirror Enforcement Decisions Onto Notification Telemetry

- **Intent:** Emit every control-lane decision as a canonical notification on
  the telemetry lane so subscribers see one shape across surfaces
- **Expected Outcome:** The enforcement pipeline produces a
  `anvil.notification.v1` envelope per decision with `mirror.decision` set to
  the control-lane outcome, `notification.class` drawn from the fixed mapping
  in the "Notification Model Integration" section, and `correlation.source =
  "intercept"`; fence transitions (`active -> fenced`, `fenced -> active`)
  populate `grouping.transition`
- **Files:** `crates/anvil-intercept/src/enforcement.rs`,
  `crates/anvil-intercept/src/telemetry.rs` (new),
  `plans/specs/2026-04-22-notification-telemetry-stream-contract.md`,
  `crates/anvil-kernel-types/src/notifications.rs`
- **Dependencies:** INTD-005, INTD-007, NOTIFY-008
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib telemetry`
  — tests assert the mapping table, schema value, mirror population, and
  fence-transition grouping
- **Status:** Draft

### INTD-014: JSON-RPC 2.0 Conformance + Round-Trip Latency Benchmark

- **Intent:** Pin the daemon's IPC surface as genuinely JSON-RPC 2.0
  compliant (not just "NDJSON that looks JSON-RPC shaped") and establish
  the end-to-end latency budget the driver-framework design relies on.
  Absorbs the validation spec of the superseded KERN-051 — Phase 5
  supersession preserved the transport surface under INTD-002 but did
  not carry across the conformance-tests and latency-benchmarks clause
  that KERN-051's Validation line carried.
- **Expected Outcome:** A conformance test suite asserts: error object
  shape (`code`, `message`, `data`), `id` semantics for request vs
  notification, batch request behaviour, `-32600`..`-32603` reserved
  codes, and the distinction between request `id: null` and
  notification. A latency harness measures round-trip p50 / p95 for a
  small RPC (`session.heartbeat`) and a telemetry-emission path
  (`enforcement.decision` round-trip) on a warm daemon and records the
  numbers. Without this, non-VSCode LSP clients (Neovim's built-in,
  Zed, Helix) may reject the connection or silently drop responses, and
  the `editor-and-mcp-driver-design.md` §3.4 save-time budget
  (< 100ms p95 warm) has no factual basis.
- **Files:** `crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-intercept/tests/jsonrpc_conformance.rs` (new),
  `crates/anvil-intercept/benches/ipc_roundtrip.rs` (new)
- **Dependencies:** INTD-002
- **Validation:** `cargo test -p eddacraft-anvil-intercept --test
  jsonrpc_conformance` passes against a published JSON-RPC 2.0 test
  fixture set; `cargo bench -p eddacraft-anvil-intercept --bench
  ipc_roundtrip` records baseline numbers in the workspace bench
  dashboard.
- **Source:** 2026-04-24 council review M1 (adversarial reviewer) —
  tracked in `plans/specs/2026-04-24-adr-030-council-findings.md`.
- **Status:** Draft

### INTD-015: Daemon-Enforced Telemetry Subscription Scoping

- **Intent:** Move per-session event filtering from driver-promised
  (current shape after KERN-052 supersession) to daemon-enforced, so a
  hostile or mis-configured driver cannot subscribe to violations from
  sessions it does not own — including file paths and content excerpts
  flagged by secret detection. Closes an exfiltration channel.
- **Expected Outcome:** Each telemetry envelope carries
  `originating_session_id` and `originating_driver_id`. The fan-out
  layer computes a daemon-side allowlist per subscriber — defaults to
  "events for sessions this driver owns, plus events explicitly
  capability-granted" — and filters outbound delivery against it.
  Global subscription is a daemon config flag, not a driver manifest
  bit. Content excerpts in telemetry for sessions the subscriber does
  not own are redacted (hash-of-path plus rule id) unless
  operator-configured otherwise.
- **Files:** `crates/anvil-intercept/src/telemetry.rs`,
  `crates/anvil-intercept/src/fanout.rs` (new),
  `plans/specs/2026-04-22-notification-telemetry-stream-contract.md`
  (add Subscribers MUST section on cross-session redaction),
  `plans/archive/modules/rust-kernel.aps.md` (update KERN-052
  supersession note to record the daemon-side filter as the
  enforceable replacement, not driver capability)
- **Dependencies:** INTD-003, INTD-013
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib
  fanout` covers (a) cross-session subscribe attempt rejected,
  (b) own-session subscribe honoured, (c) content excerpts redacted
  on cross-session allowlist hit.
- **Source:** 2026-04-24 council review M5 (security-analyst) —
  tracked in `plans/specs/2026-04-24-adr-030-council-findings.md`.
- **Status:** Draft

### INTD-016: DoS Protection Budgets — Connection Cap, Rate Limits, Timeouts

- **Intent:** Re-home the defence-in-depth budgets that the KERN Phase
  5 supersession silently dropped. KERN's future review would have
  enforced them; INTD-002 currently has no written cap on simultaneous
  connections, request rate, handshake timeout, idle timeout, or frame
  size, making the daemon DoS-able by any same-UID peer.
- **Expected Outcome:** Configurable limits with documented defaults:
  concurrent driver connections (64), per-connection RPS (100
  sustained / 1000 burst), handshake timeout from `accept()` to
  manifest (5 s), driver-connection idle timeout separate from session
  heartbeat TTL (60 s), max NDJSON frame size (64 KiB — manifests and
  control-lane messages only; telemetry-lane sizing tracked
  separately), explicit plaintext-local-only TLS stance recorded in
  AD-4 until remote-shell driver arrives. Limits applied at the IPC
  listener level — connection dropped with structured error on budget
  exhaustion. Enforcement pipeline unaffected by exhausted budgets
  (cannot be starved by a misbehaving peer).
- **Files:** `crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-intercept/src/config.rs`,
  `plans/decisions/015-intercept-loop-enforcement.md` (extend AD-4
  with the limits + TLS stance)
- **Dependencies:** INTD-002, INTD-008
- **Validation:** Unit tests assert each budget: slow-loris handshake
  times out, over-cap connection rejected, frame larger than cap
  rejected, RPS bucket exhaustion returns structured error without
  terminating the connection.
- **Source:** 2026-04-24 council review M9 (security-analyst +
  adversarial-reviewer) — tracked in
  `plans/specs/2026-04-24-adr-030-council-findings.md`.
- **Status:** Draft
