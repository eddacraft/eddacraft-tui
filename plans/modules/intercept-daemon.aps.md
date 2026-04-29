# Intercept Daemon

| ID   | Owner  | Status      | Progress                                                |
| ---- | ------ | ----------- | ------------------------------------------------------- |
| INTD | @aneki | In Progress | 2/16 complete, 2 committed (INTD-002, INTD-014) |

**Last reviewed:** 2026-04-29

> **A1 launch slice (cherry-picked, not the whole module):** INTD-001,
> INTD-002, INTD-003, INTD-005, INTD-013, INTD-014. The remaining INTD work
> items (INTD-004, -006..-012, -015, -016) ship after A1 alongside DRVR.
> No `crates/anvil-intercept` crate exists yet on `dev`; A1 kickoff begins
> with INTD-001 scaffolding (and the parser-concurrency decision recorded
> inline at INTD-001 review per the LANGTS K3 deferral).

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
- Graph-assisted hot-path checks in v1; future graph reads must go through GV2's
  warmed hot-read API and remain constant-time or near-constant-time
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

## Graph v2 Coordination

INTD is not blocked by Graph v2. For the current release it remains a
deterministic daemon over sessions, file changes, rules, fences, and telemetry.

When GV2 lands, INTD becomes the authoritative producer for the control/session
graph: hosts, drivers, sessions, worktrees, leases/fences, attribution, and
control decisions. INTD may later consume GV2 hot-path indexes for boundary
membership, symbol ownership, known-edge existence, or architectural index
checks, but only through the GV2-022 hot-read API and only within ADR-031
latency budgets. Full graph recompute, transitive traversal, context slicing,
and explanation workloads stay outside INTD's hot path.

RMCP does not make MCP a control surface. For A1, the Rust MCP launch shim may
call the daemon or the shared Rust validation path to validate proposed content,
but session control, fencing, interruption, and attribution authority still live
with INTD and the broader driver framework.

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
  shared by both daemon and launcher. `anvil intercept start --foreground`
  runs the daemon in the current process for dev / demo / triage use — no
  double-fork, no PID-file handoff, logs stream to stdout/stderr, SIGINT in
  the controlling terminal stops it cleanly. This is the path the demo
  runbook §4.1 falls back to when the backgrounded daemon fails to start
  and the operator needs to see the real error
- **Validation:** `cargo build -p eddacraft-anvil-intercept && cargo test -p eddacraft-anvil-intercept`
- **Status:** Complete
- **Committed (2026-04-29, PR #1165):** Three crates scaffolded —
  `crates/anvil-intercept-proto/` (NDJSON envelope, `SessionId`,
  `IpcCommand` enum: register/heartbeat/unregister/list),
  `crates/anvil-intercept/` (lib + bin with `run_foreground`,
  cooperative `Shutdown`/`ShutdownToken` watch handles, shared
  `wait_for_shutdown_signal` helper that races SIGINT (Ctrl+C) and
  Unix SIGTERM), and `crates/anvil-intercept-rules/` (initial
  `InterceptRule` trait surface for downstream INTR work). CLI
  surface `anvil intercept start --foreground` is wired through
  `crates/anvil-cli/src/commands/intercept.rs`. Foreground startup
  creates the PID file exclusively at the daemon runtime path, refuses
  a second daemon against the same PID file, and removes the PID file
  on clean shutdown. Proto, daemon, and CLI intercept tests pass locally;
  Windows coverage is provided by the existing `rust.yml` workspace
  build/test matrix because the intercept crates are workspace members.
- **Trigger flag (parser concurrency ADR):** The LANGTS audit
  (`plans/specs/2026-04-26-langts-audit-report.md` §5.3, K3) deferred
  the parser thread-locality ADR conditionally. **At INTD-001 review,
  decide the daemon's parser concurrency model.** If the choice is
  obvious (likely `thread_local!` per option (1) in the audit) and no
  disagreement surfaces, capture the decision inline in this task's
  Notes — no ADR needed. If the choice is contentious, or multi-process
  daemon scenarios materialise, **author the parser thread-locality ADR
  before INTD-001 lands**. The audit's evaluation of the four options
  is the starting point for this discussion.
- **Notes (parser concurrency decision, 2026-04-29):** No ADR required for
  INTD-001. The daemon's future parser-driven hot path will use the audit's
  option (1): one parser pool per worker via `thread_local!`, keeping
  `tree_sitter::Parser` thread-confined while allowing concurrent file
  evaluation. Worker-scope parsing remains the escape hatch if a later
  parser-backed rule proves the memory cost unacceptable; multi-process daemon
  scenarios would re-open the ADR trigger.

### INTD-002: IPC Listener

- **Intent:** Accept NDJSON connections over Unix domain sockets
  (Linux/macOS) and named pipes (Windows) with restricted permissions
- **Expected Outcome:** Daemon listens on a platform-appropriate socket
  path, accepts connections, parses NDJSON frames, and dispatches to
  command handlers. Socket / pipe creation is pinned end-to-end, not
  left to umask / default DACL:
  - Unix: `lstat` the target (or open its parent with `openat` +
    `O_NOFOLLOW` and stat from there) to refuse symlinks and to
    verify that `$XDG_RUNTIME_DIR/anvil` is owned by the current
    user with mode 0700 if it already exists. If absent, create it
    with `mkdir` passing an explicit mode 0700, then re-verify with
    `stat` / `fstat` that owner and mode match. `bind()` inside
    that dir; `fchmod` the socket fd to 0600 before `listen()`. If
    `$XDG_RUNTIME_DIR` is unset, fall through to
    `$HOME/.local/state/anvil/` with the same check-create-verify
    sequence — never `/tmp`.
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
- **Status:** Committed
- **Progress (2026-04-29, `feat/INTD-002`):** `crates/anvil-intercept/src/ipc.rs`
  ships the `SessionDispatcher` trait, `NoopDispatcher`, the Unix
  socket-dir resolution + permission ladder (lstat-based symlink
  refusal, owner-and-mode verification, `mkdir(0o700)` then
  re-verify, `fchmod`+chmod-by-path to `0o600`), the stale-vs-live
  socket handling, NDJSON framing with a 1 MiB cap, malformed-line
  skip, per-connection `JoinSet`, and a 250 ms shutdown drain.
  Validation: `cargo test -p eddacraft-anvil-intercept --lib ipc` —
  21 tests pass (full crate suite: 25 pass). Windows pipe-name
  resolution + DACL binding are scaffolded behind `#[cfg(windows)]`
  with `unimplemented!()` stubs; pipe-name helper is unit-tested on
  Windows builds. PID-file guarding is covered by INTD-001.
- **Reopened (2026-04-29):** A1 now requires the full cross-platform
  contract, including Windows named-pipe binding with an owner-only
  security descriptor and foreground daemon integration with the IPC
  listener and session registry.
- **Committed (2026-04-29, `feat/INTD-002-cross-platform`):** Foreground
  daemon startup now owns a `SessionRegistry`, binds the IPC listener,
  dispatches registration frames into the registry, ticks stale-session
  eviction, and shuts the listener down with bounded drain. Windows
  named-pipe binding is implemented through the Windows-only
  `anvil-intercept-win32` helper crate so `anvil-intercept` remains
  `#![forbid(unsafe_code)]`; the helper creates a local-only pipe with
  `PIPE_REJECT_REMOTE_CLIENTS` and an explicit current-user owner-only
  DACL. Validation: `cargo test -p eddacraft-anvil-intercept` (51 pass),
  `cargo clippy -p eddacraft-anvil-intercept --all-targets -- -D warnings`,
  `cargo clippy -p eddacraft-anvil-intercept --target x86_64-pc-windows-msvc --all-targets -- -D warnings`,
  `cargo clippy -p eddacraft-anvil-intercept-win32 --target x86_64-pc-windows-msvc --all-targets -- -D warnings`,
  `cargo fmt --check`, and `cargo hakari verify`.
- **Council review (2026-04-24):** M8 (security-analyst) pinned the
  end-to-end creation sequence above; see
  PR #1063.

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
- **Status:** Complete
- **Progress (2026-04-29, `feat/INTD-003`):** `SessionRegistry` landed in
  `crates/anvil-intercept/src/registry.rs` with `SessionRecord` /
  `SessionStatus` extended onto the proto crate's wire surface. Synchronous
  `evict_stale(now)` returns the ids it removed; the daemon owns scheduling.
  Worktree paths canonicalised before use as a key; missing paths refused
  via `RegistryError::WorktreePathInvalid`. `SessionDispatcher` trait gives
  INTD-002 an `Arc<dyn>` handle without binding to the concrete type. TTL
  boundary pinned at "exactly TTL alive, TTL + 1ns evicts". 14 registry
  tests + 2 new proto tests pass:
  `cargo test -p eddacraft-anvil-intercept --lib registry`.

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
  short-circuits content rules; deleted files pass only path-based rules. The
  core evaluation step is factored so RTAI/RMCP can validate caller-provided
  proposed content through the same rule pipeline without duplicating rule
  semantics; the daemon's file-change path still reads from disk for v1.
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
  future CLI status commands. `anvil intercept status` MUST include an
  operator-visible **mid-edit p50/p95 latency rollup line** sourced from the
  daemon-side `validation.service` telemetry for `mode = midEdit` (e.g.
  `latency: p50 <X>ms p95 <Y>ms (mid-edit)`), so the demo runbook §1.5
  trust-signal line is real and not estimated. The rollup is computed over a
  sliding window (default last 100 mid-edit calls or last 60 seconds,
  whichever is shorter). The measurement labels and acceptable thresholds are
  owned by ADR-031.
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib status`
  plus an assertion that the status payload carries `latency.midEdit.p50`
  and `latency.midEdit.p95` fields (or their textual rollup) when the
  daemon has observed at least one mid-edit call.
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
  notification. A latency harness records p50 / p95 / p99 using the
  ADR-031 measurement vocabulary, at minimum `validation.service` for
  daemon-handled requests and `validation.roundtrip` where the
  `DriverClient` transport is present. The surface under test is the
  daemon↔driver JSON-RPC boundary (what `DriverClient` in DRVR-001
  talks to) — editors reach the daemon via the editor-driver, not by
  connecting LSP-style directly, so the risk to cover is silent drift
  between the daemon's wire behaviour and the driver client's expected
  request/response semantics. Local latency numbers must not be
  invented here; save-time and buffer/pre-write budgets come from
  ADR-031.
- **Files:** `crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-intercept/tests/jsonrpc_conformance.rs` (new),
  `crates/anvil-intercept/benches/ipc_roundtrip.rs` (new)
- **Dependencies:** INTD-002
- **Validation:** `cargo test -p eddacraft-anvil-intercept --test
  jsonrpc_conformance` passes the local fixture-style JSON-RPC 2.0 conformance
  suite (no published fixture set is present in the workspace); `cargo bench -p
  eddacraft-anvil-intercept --features bench-internals --bench ipc_roundtrip`
  records baseline numbers with ADR-031 dimensions in the workspace bench
  dashboard.
- **Source:** 2026-04-24 council review M1 (adversarial reviewer) —
  tracked in PR #1063.
- **Status:** Committed
- **Progress (2026-04-29, `feat/INTD-014-jsonrpc`):** JSON-RPC 2.0 request /
  notification / batch response handling is pinned at the daemon IPC boundary,
  with local fixture-style conformance coverage for parse errors, error object
  shape, invalid request handling, id semantics including `id: null`, request-only
  batch responses, all-notification batches, and reserved `-32700` / `-32600`..
  `-32603` error codes. `ipc_roundtrip` records `validation.service` separately
  from Unix-socket `validation.roundtrip` and prints ADR-031-style dimensions.

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
  tracked in PR #1063.
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
  PR #1063.
- **Status:** Draft
