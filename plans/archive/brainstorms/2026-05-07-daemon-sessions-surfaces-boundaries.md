# Daemon, Sessions, Surfaces, and Boundaries — Planning Brainstorm

> **Round-1 brainstorm** focused on daemon scope, discovery, and OS
> boundaries. The session continued into a much wider scope (defense-in-
> depth layers, witness chain, hooks, L4 policy, baseline, multi-agent
> coordination, rule distribution). The continuation is captured in
> [`2026-05-07-anvil-multilayer-protection-brainstorm.md`](./2026-05-07-anvil-multilayer-protection-brainstorm.md)
> (round 2) and consolidated as the spec
> [`2026-05-07-anvil-multilayer-protection-architecture.md`](../specs/2026-05-07-anvil-multilayer-protection-architecture.md).
> Recommendations in §10 / §11 of this doc are partially superseded by
> those follow-on artefacts; the scenario inventory and council
> personas remain useful reference.

**Date:** 2026-05-07
**Status:** Brainstorm (input to spec `2026-05-07-daemon-lifecycle-and-discovery.md` and decision ADR-036; continued in round-2 brainstorm above).
**Author:** Planning council (Claude Opus 4.7, 1M ctx) under user direction.
**Scope:** Daemon identity, surface discovery, lifecycle, multi-surface
coordination, and OS/process/filesystem boundaries (Linux, macOS, Windows,
WSL, future remote/dev-container).
**Why now:** The current model — single per-user daemon, single session per
worktree, manual foreground start, algorithm-derived socket path — was
adequate for A1 (RMCP stdio shim, embedded fallback). A2 wants to claim
**daemon-backed validation reach** across editor drivers and MCP, while
several real-world scenarios are not addressed at all in current planning
(see "Known Unknowns" 1–10 in the inventory below). We need a v1/vNext model
before committing to A2 protection claims.

---

## 1. Current-state inventory

Sourced from the modules and specs cited; nothing here is invented.

### 1.1 Daemon assumptions

- **Process model:** per-user persistent singleton, written in Rust under
  `crates/anvil-intercept/`, supporting crates `anvil-intercept-proto` and
  `anvil-intercept-win32`
  (`plans/archive/modules/intercept-daemon.aps.md` §Purpose, INTD-001..INTD-016).
- **Single-instance guard:** PID file at
  `$XDG_RUNTIME_DIR/anvil/intercept.pid` (or
  `$HOME/.local/state/anvil/intercept.pid` fallback). Exclusive create;
  refuses a second daemon. No socket-bind race protection beyond the PID
  file (INTD-001, INTD-002).
- **Lifecycle:** **manual foreground only** — `anvil intercept start
  --foreground`. Backgrounded launch and supervisor integration (systemd
  user, launchd, Windows Service) are deferred post-A1 (INTD-001 notes).
- **Embedded mode:** synchronous in-process pipeline available for CI /
  testing; **deliberately not** a silent fallback for a failed daemon — the
  function signature pins this at compile time (INTD-009,
  `embedded_does_not_auto_promote_from_failed_daemon_path`).

### 1.2 Transport / IPC assumptions

- **Linux / macOS:** Unix domain socket at
  `$XDG_RUNTIME_DIR/anvil/intercept.sock` (fallback
  `$HOME/.local/state/anvil/intercept.sock`). Directory 0700, socket 0600,
  symlink-refused via `lstat`, owner-and-mode verified before `bind()`,
  `fchmod` to 0600 before `listen()` (INTD-002).
- **Windows:** named pipe `\\.\pipe\anvil-intercept-{user_sid}` with
  explicit `SECURITY_DESCRIPTOR` (owner-only DACL) and
  `PIPE_REJECT_REMOTE_CLIENTS` (INTD-002, `anvil-intercept-win32`).
- **No TCP loopback.** Deliberately excluded in v1 per
  `2026-05-06-editor-driver-protocol.md` §1.
- **Discovery:** algorithm-derived path; surfaces compute the path the same
  way the daemon does. No service registry, no advertised endpoint, no
  `info.json` (`2026-05-06-editor-driver-protocol.md` §1).
- **Same-UID trust:** `SO_PEERCRED` uid match (Unix) or owner-only DACL
  (Windows) (INTD-002, DRVR-007).

### 1.3 Project / session assumptions

- **Session-per-worktree, v1.** A worktree is the canonicalised root
  containing `.anvil.yaml` (or, in its absence, the registered cwd). Single
  session per worktree (INTD-003).
- **Project identity:** canonicalised worktree path. No use of git
  toplevel, inode/device ID, repo origin URL, or explicit project ID.
- **Per-worktree enforcement config:** `<workspace_root>/.anvil.yaml`
  merged stricter-wins with optional user-level config (INTD-008).
- **Fence state:** persisted to disk per worktree, survives daemon restart
  (INTD-007).
- **Multi-client per session:** asynchronous fan-out to all attached
  drivers; per-driver bounded telemetry channels with overflow drop
  (INTD-013, `editor-and-mcp-driver-design.md` §2.8).

### 1.4 RMCP / MCP assumptions

- **RMCP shim** (`anvil mcp serve --stdio`) is launched per editor by the
  editor's MCP config (Cursor, Claude Code). Each shim is a short-lived
  child process owned by the editor; it talks to the daemon over the same
  per-user socket — but in **A1 release the daemon-backed path is
  unavailable**, so the shim runs **embedded** validation through
  `anvil-checks` (RMCP-005, `rust-mcp-launch-shim.aps.md`).
- **A2 wants** the same shim to call `scan_buffer` against the per-user
  daemon (RTAI-002). The pre-write/mid-edit RPC contract is locked
  (RTAI-001 spike).

### 1.5 Surface-driver assumptions

- **DriverClient (TS, `packages/anvil-driver-client/`)** auto-selects UDS
  on Unix, named pipe on Windows. Refuses sockets/pipes not owned by
  current user. Reconnection with backoff (DRVR-001).
- **Five-state lifecycle:** Unbound → Handshake → Attached (read-only) →
  Participating (enforcement opt-in, allowlist-gated) → Detached
  (`editor-and-mcp-driver-design.md` §2.1).
- **Capability negotiation (DRVR-008):** drivers that omit
  `anvil/enforcement/ack` from `supported_anvil_methods` are capped at
  Attached; daemon emits `anvil/capability/downgrade`. Stock LSP clients
  cannot be silently fenced.
- **Reliability quarantine:** keyed off binary hash / install-time UUID
  (vNext); not driver self-declared name
  (`editor-and-mcp-driver-design.md` §2.6).

### 1.6 Cross-platform / WSL signals

- **WSL is not mentioned** anywhere in `plans/modules/`,
  `plans/specs/anvil-driver-framework/`, or any ADR. The Windows path is
  explicitly **native Windows** (named pipe + Job Object); the Linux path
  is `XDG_RUNTIME_DIR`. There is no current model for what happens when
  the surface and the daemon are on opposite sides of the WSL boundary.
- Per the inventory, this is **Known Unknown #3**.

### 1.7 Documented limitations (already known)

From inventory "Known Unknowns" 1–10:

1. Daemon auto-start / lifecycle supervision — not specified.
2. Multi-agent / concurrent-write same-worktree — not specified.
3. WSL / remote-host reach — not specified.
4. Daemon reconnection / in-flight-request replay on crash — partial.
5. Monorepo / sub-policy multiplexing — not specified.
6. Discovery path override / lockfile / service registry — not specified.
7. Graph v2 hot-read API contract — out of scope for v1.
8. Per-driver vs per-session enforcement decisions — partial.
9. Stale-socket reaper / lease renewal — not specified.
10. Telemetry-log redaction policy — partial.

---

## 2. Scenario brainstorm

Each scenario lists: surfaces, project location, daemon location, discovery
path, file-path treatment (shared / translated / incompatible), and v1
classification. **v1** = MUST work in next release. **later** = vNext or
beyond. **unsupported** = explicitly out, with a polite refusal message.

| #  | Scenario                                                  | Surfaces                                       | Project FS                          | Daemon                       | Discovery                                              | Path treatment           | Classification |
|----|-----------------------------------------------------------|------------------------------------------------|-------------------------------------|------------------------------|--------------------------------------------------------|--------------------------|----------------|
| 1  | Single Linux terminal, single project                     | one terminal (Claude Code)                     | local Linux                         | Linux per-user               | `$XDG_RUNTIME_DIR/anvil/`                              | shared                   | **v1**         |
| 2  | Two Linux terminals, same project, two CLIs               | terminal A (Claude Code) + terminal B (codex)  | local Linux                         | Linux per-user               | same socket; two sessions same worktree                | shared                   | **v1**         |
| 3  | Editor + embedded terminal (Zed on Linux)                 | Zed driver + Zed-embedded terminal             | local Linux                         | Linux per-user               | same socket; embedded terminal is just a child PTY     | shared                   | **v1**         |
| 4  | Editor + external terminal (Cursor on macOS)              | Cursor driver (LSP-shape) + iTerm (Claude Code)| local macOS                         | macOS per-user               | same socket                                            | shared                   | **v1**         |
| 5  | Cursor + Claude Code in two terminals, same project       | Cursor MCP shim + Claude Code MCP shim         | local Linux/macOS                   | Linux/macOS per-user         | both shims invoke `scan_buffer` against same daemon    | shared                   | **v1**         |
| 6  | Two embedded terminals in one editor, two agents          | Zed + 2 child PTYs (Claude Code + codex)       | local                               | per-user                     | same socket, 2 sessions same worktree                  | shared                   | **v1.5**       |
| 7  | Multiple agents editing same project concurrently         | terminal A + B + Cursor MCP shim (3 agents)    | local                               | per-user                     | same socket, 3 sessions, daemon-side serialisation     | shared                   | **v1.5**       |
| 8  | Native Windows editor (Cursor) + Windows project          | Cursor (Windows) MCP shim                      | Windows NTFS                        | Windows per-user             | named pipe `\\.\pipe\anvil-intercept-{sid}`            | shared                   | **v1**         |
| 9  | WSL terminal in WSL distro, project inside WSL FS         | terminal inside WSL2 distro                    | WSL ext4 (`/home/...`)              | WSL Linux per-user (per distro) | `$XDG_RUNTIME_DIR/anvil/` inside the distro          | shared                   | **v1**         |
| 10 | Windows editor + WSL terminal, project on `/mnt/c/...`    | Cursor (Windows) + WSL terminal                | Windows NTFS via 9P (`/mnt/c/...`)  | both? → split-brain          | Windows daemon vs WSL daemon — **two daemons**         | translated, lossy        | **unsupported (v1)** |
| 11 | Windows editor + WSL terminal, project inside WSL FS      | Cursor (Windows) + WSL terminal                | WSL ext4 (`\\wsl.localhost\...`)    | WSL daemon authoritative     | Windows surface cannot reach WSL UDS without bridge    | translated, lossy        | **unsupported (v1)** |
| 12 | VSCode Remote-WSL: editor UI on Windows, server in WSL    | VSCode UI (Windows) + VSCode-Server (WSL)      | wherever the WSL workspace lives    | WSL per-user (where server runs) | VSCode-Server is a Linux process — same as #9        | shared (server side)     | **v1**         |
| 13 | VSCode Remote-SSH                                         | VSCode UI (local) + remote VSCode-Server       | remote Linux                        | remote Linux per-user        | server-side; no bridge across SSH                      | shared (server side)     | **later**      |
| 14 | Dev container / Codespaces (project inside container)     | editor outside, project + tools inside         | container                           | container Linux per-user     | container-internal socket; bind-mounted                | shared (inside container) | **later**     |
| 15 | Editor on Windows + Cursor MCP shim spawned by editor     | Cursor (Windows native) MCP child              | Windows NTFS                        | Windows per-user             | shim is child of editor; reuses Windows pipe           | shared                   | **v1**         |
| 16 | macOS editor + Docker desktop project (volume-mounted)    | editor on host, agent in container             | NTFS-like host volume + container ext4 | host macOS per-user **or** container | mismatch — same as dev-container case          | translated               | **later**      |
| 17 | Multiple worktrees of same repo, same user                | terminal A in `~/proj-main`, B in `~/proj-pr`  | local                               | per-user                     | same socket, two distinct sessions (per-worktree)      | shared                   | **v1**         |
| 18 | Daemon crashes mid-session                                | any driver                                     | local                               | restart needed               | DriverClient reconnect with backoff (DRVR-001)         | shared                   | **v1**         |
| 19 | Two `anvil intercept start` invocations race              | two terminal launchers                         | local                               | one wins (PID-file exclusive) | second exits with PID-conflict error                  | shared                   | **v1**         |
| 20 | Daemon binary upgraded mid-session (version skew)         | old DriverClient + new daemon                  | local                               | per-user                     | manifest-handshake version check; downgrade or refuse  | shared                   | **v1**         |

**Notes on the classification:**

- **v1 (the "honest reach" set):** scenarios 1–5, 8, 9, 12, 15, 17, 18, 19,
  20. All share the property: surface and daemon live on the same OS
  instance and see the same canonical filesystem.
- **v1.5 (multi-agent same-worktree):** scenarios 6, 7. The daemon
  registry already supports multiple drivers per session; what is missing
  is the contract for how concurrent `scan_buffer` requests interleave and
  how enforcement decisions fan out to multiple agents.
- **later:** scenarios 13, 14, 16. Same-OS instance assumption holds (the
  daemon runs server-side / inside-container), so no new boundary
  problem — only operational complexity (bind mounts, port forwarding,
  install packaging).
- **unsupported (v1):** scenarios 10, 11. Cross-Windows/WSL boundary with
  a single daemon. v1 will detect and refuse with a clear claim, **not
  pretend to protect**.

---

## 3. Planning council

Six personas. Each lists concerns, recommended model, risks, non-negotiables,
and questions requiring an explicit decision.

### 3.1 Product / activation lead

**Concerns**

- A1 already shipped with the operator-visible status line `latency: p50
  Xms p95 Yms (mid-edit)` (INTD-011). If A2 says "daemon-backed
  protection across surfaces", the user must be able to *see* which
  surfaces are attached, which mode is active, and which scenarios are
  unsupported.
- Wow-start activation hard-rules (`2026-05-04-launch-a1-execution.md`)
  forbid theatre and false claims. The first-minute experience must
  match what's actually running.
- Cursor + Claude Code is the *only* protection claim today. Anything
  broader needs to be earned scenario-by-scenario.

**Recommended model**

- One daemon per user per OS instance.
- Surfaces self-report to the daemon during handshake; daemon emits an
  `anvil status` block listing attached surfaces, transport, mode (mid-edit
  vs save-time), and per-surface protection level.
- "Surface attached" is a verb the daemon is allowed to say; "project
  protected" is only allowed when (a) at least one Participating driver
  is attached for the worktree, OR (b) MCP shim daemon-backed path
  succeeded at least once.

**Risks**

- Saying "Anvil protects this project" when the only attached surface is
  read-only Attached and the agent writes through a non-MCP path.
- Saying "Anvil protects all surfaces" when one surface is on Windows and
  another is in WSL.

**Non-negotiables**

- Status output must distinguish `attached` from `participating`.
- WSL-boundary scenarios MUST display "Cross-boundary surface — not
  protected" when detected; protection claim MUST be downgraded.
- No silent embedded-fallback on a Participating driver in A2 — must be
  surfaced as `mode: embedded-fallback (daemon unreachable)`.

**Questions**

- **Q-PROD-1:** What's the canonical CLI surface? `anvil status` already
  exists for daemon. Do surfaces also call `anvil surfaces` for the list?
- **Q-PROD-2:** When the user opens a second editor against a project, do
  we auto-launch the daemon (if not running) or refuse and ask? (See
  Q-LIFE-1.)
- **Q-PROD-3:** What protection state do we claim during the daemon-start
  window before any surface attaches?

### 3.2 Systems architect

**Concerns**

- Identity is currently *implicit* in path canonicalisation. A worktree is
  the canonical path of the cwd's `.anvil.yaml` ancestor (or cwd if
  none). This breaks under symlinks, bind mounts, and case-insensitive
  filesystems on macOS / Windows.
- Daemon scope is per-user per-OS, but discovery is path-derived without
  a sidecar `info.json`. A surface cannot tell whether the daemon at the
  socket is a different version than itself without a handshake round-trip
  — fine for editor drivers, expensive for short-lived MCP shims.
- The IPC listener owns DoS budgets at the connection level (INTD-016).
  Multiple short-lived MCP shims + long-lived editor drivers + ad-hoc CLI
  status calls will share that budget.

**Recommended model**

- Promote **discovery** from "surface re-derives path" to "daemon writes
  `info.json` next to the socket". Surface reads `info.json`, gets:
  `pid`, `socket_path`, `pipe_path`, `version`, `proto_version`,
  `started_at`, `os_label`, `os_locality_token`. Surface verifies pid is
  alive and `os_locality_token` matches its own (described §6).
- Adopt **explicit project identity** = `(canonical_worktree_path,
  os_locality_token, optional git_dir_inode)`. This is the daemon-side
  composite key used in the session registry. Path-only identity remains
  the user-facing handle.
- Keep per-user singleton — but make it **lazily auto-started** by an
  idempotent launcher (see §3.5).
- Treat the WSL boundary as a hard fence: each WSL distro is an OS
  instance; Windows native is its own OS instance. Two daemons, one each,
  if both are in use.

**Risks**

- Stale `info.json` after crash (PID reused). Requires liveness probe and
  starttime check (already present for INTD-006 process termination —
  reuse the helper).
- Path canonicalisation drift on macOS HFS+/APFS case-folding. Already
  handled with `attribute_path` longest-prefix match (INTD-004).

**Non-negotiables**

- The daemon MUST publish enough metadata for a surface to refuse
  attachment without a full handshake when versions are incompatible.
- The session registry MUST allow >1 driver per session (already true)
  AND MUST allow >1 session per worktree when keyed by (driver_id,
  agent_id, pid) — currently it does not.

**Questions**

- **Q-ARCH-1:** Is project identity `(canonical_path)`,
  `(canonical_path, git_dir_inode)`, or `(canonical_path, repo_origin)`?
- **Q-ARCH-2:** Where does `info.json` live? Same dir as socket? `.anvil/`
  in the project? `~/.config/anvil/runtime/`? (See §6.)
- **Q-ARCH-3:** Do we keep the per-user singleton, or allow multiple
  daemons per user (one per project) for blast-radius isolation?

### 3.3 Security reviewer

**Concerns**

- Same-UID trust is fine for v1, but the moment a daemon is
  auto-started by *any* surface, a malicious child process under the
  same UID can pre-create the socket directory or PID file with attacker
  contents and wait. INTD-002's lstat-based ladder defends the daemon
  side; the surface side is defended by DRVR-001's owner-check, but only
  if the surface uses `DriverClient`.
- WSL boundary: a Windows process accessing `\\wsl.localhost\...` and a
  Linux process inside the distro accessing `/home/...` of the same files
  see different security contexts. Cross-trusting them is a privilege
  confusion.
- TCP loopback was rejected in v1 (`2026-05-06-editor-driver-protocol.md`
  §6.1). It MUST stay rejected. No loopback, no localhost-with-token, no
  "convenience bridge" without TLS + cert pinning + clear scope.
- The MCP shim runs as a child of the editor, inheriting the editor's UID
  but potentially a different security context (sandboxed app on macOS,
  for example). On macOS Cursor in App Sandbox cannot bind a UDS in
  `/Users/<u>/`; must use the app's container path.

**Recommended model**

- Keep same-UID trust as the v1 default (`SO_PEERCRED` / DACL). Do not
  add cross-UID, cross-user, or cross-host without an ADR.
- Document the **trust boundary table** as part of the spec (each surface
  type → what it can do at Attached / Participating).
- v1: `os_locality_token` is hashed `(uid, hostname, kernel_release,
  windows_session_sid_or_wsl_distro_id)`. Surface refuses to use a
  socket whose `info.json` token does not match.
- For the macOS sandbox edge case, surfaces MUST fall back to embedded
  validation (RMCP shim's existing path) and emit a downgrade event.

**Risks**

- Auto-start race: attacker pre-creates `intercept.sock` directory with
  permissive mode; daemon's `mkdir(0o700)` either succeeds (race won) or
  fails (DoS). INTD-002 already does check-create-verify; keep it.
- Squat on `info.json`: same defence — owner + mode + symlink refusal.
- Cross-boundary path translation creates a "trust the driver's
  translation" surface. v1: refuse cross-boundary attachment. vNext: a
  bridge driver with explicit transport (named pipe forwarder) and audit
  logging.

**Non-negotiables**

- No TCP transport in v1.
- No cross-UID trust in v1.
- Cross-WSL-boundary attachment refused in v1, structured error returned.
- `info.json` validated with same lstat ladder as the socket file.

**Questions**

- **Q-SEC-1:** What is the policy when an attacker has the same UID and
  *also* the right to create files in the runtime dir? (i.e. the local
  attacker assumption is already in their threat model — do we just
  document it?)
- **Q-SEC-2:** Is cross-WSL-distro-as-same-user an attack? (Two distros
  belong to the same Windows user but have separate Linux UIDs. Probably
  fine, but worth declaring.)

### 3.4 Developer experience reviewer

**Concerns**

- Today: user runs `anvil intercept start --foreground` in a spare
  terminal, then runs everything else. **This is unworkable** at scale.
  An auto-started daemon is mandatory for A2.
- Surfaces that can't find the daemon today get a vague "not running"
  error. Need a single command (`anvil doctor`) that diagnoses surface
  ↔ daemon connectivity, version skew, and protection claim mismatch.
- WSL: user opens Cursor on Windows and a WSL terminal, not realising
  they're crossing a boundary. The system MUST notice and explain — not
  silently fail to validate.
- Onboarding the same daemon across 3 simultaneous editors is fine if
  they all attach. The annoying case is when one editor's MCP shim
  silently uses embedded mode while another talks to the daemon — diff
  in protection level not visible.

**Recommended model**

- `anvil intercept ensure` — idempotent launcher: returns immediately if
  daemon is up and version-compatible; spawns it (detached, log to file)
  otherwise. Drivers and CLIs call this on startup.
- `anvil status` already exists for daemon-side; extend the JSON output
  to list attached surfaces, mode, last decision, last error.
- `anvil doctor` (new): runs the discovery algorithm, prints
  `info.json`, attempts a no-op handshake, reports per-surface state.
  Must work even when the daemon is unreachable.
- Recovery path: a surface that can't reach the daemon is allowed to
  emit a `status: degraded` event, attach in read-only "embedded
  fallback" mode for MCP, and reconnect on next backoff window.

**Risks**

- Auto-spawn lifecycle bugs (orphaned daemons after editor close, stale
  PIDs across reboots). Standard problem; existing PID-file + starttime
  check handles it. Add `anvil intercept reap` for explicit cleanup.
- Editors that don't use `DriverClient` (raw LSP) get a worse experience:
  they see only `textDocument/publishDiagnostics`, no participation, no
  auto-spawn. Document this and accept it.

**Non-negotiables**

- Auto-start MUST be idempotent and MUST refuse to start a second
  instance.
- No stdout pollution from the daemon log when run from `ensure`.
- `anvil doctor` MUST complete with a clear verdict in under one second
  on a healthy system; it MUST never hang on a dead socket.

**Questions**

- **Q-DX-1:** Does `anvil intercept ensure` block until the daemon is
  ready, or return early and let the surface poll?
- **Q-DX-2:** Where does the auto-started daemon's log live? Per-user log
  file (`~/.local/state/anvil/intercept.log` rotating)? `journald` user
  unit? Both, configurable?
- **Q-DX-3:** What happens when `anvil` (CLI) and the daemon binary are
  different versions? Auto-start the matching daemon? Refuse? Warn?

### 3.5 Runtime / platform reviewer

**Concerns**

- **Linux:** UDS works, `XDG_RUNTIME_DIR` is reliable on systemd
  systems, fallback path is fine. systemd user units (`anvil.service`) are
  the natural supervisor.
- **macOS:** No `XDG_RUNTIME_DIR` by default. `~/.local/state/anvil/`
  fallback works but is not the macOS convention. `~/Library/Application
  Support/Anvil/runtime/` is more idiomatic. launchd user agents
  (`~/Library/LaunchAgents/io.eddacraft.anvil.plist`) are the natural
  supervisor.
- **macOS App Sandbox** (Cursor, etc.): sandboxed apps cannot reach UDS
  outside their container. Solutions: `~/Library/Group Containers/<group
  id>/anvil/` shared-container path (requires Cursor cooperation) OR
  embedded fallback only.
- **Windows:** named pipe path is fine. No `XDG_RUNTIME_DIR`. PID file in
  `%LOCALAPPDATA%\Anvil\runtime\`. Windows Service is overkill for a
  per-user daemon — Windows Task Scheduler "at logon" or auto-start via
  the explorer shell on first surface launch.
- **WSL:** each distro is a separate Linux OS instance with its own UID
  namespace, its own `$XDG_RUNTIME_DIR` (under `/run/user/<uid>/`). The
  Windows host cannot enter that namespace. There is no portable way to
  reach a UDS in `/run/user/1000/anvil/intercept.sock` from the Windows
  side; `\\wsl.localhost\Ubuntu\run\user\1000\anvil\intercept.sock` is
  not real.
- WSL2's mirrored network mode (Windows 11) makes localhost-TCP between
  Windows and WSL trivial — but TCP is rejected in v1.
- Multiple WSL distros (Ubuntu + Debian) each get their own daemon. They
  share user identity at the Windows level but are separate Linux
  installs.

**Recommended model**

- Platform-specific runtime dir + supervisor:
  - Linux: `$XDG_RUNTIME_DIR/anvil/` + systemd user unit (vNext) /
    auto-start launcher (v1).
  - macOS: `~/Library/Application Support/Anvil/runtime/` + launchd
    user agent (vNext) / auto-start launcher (v1).
  - Windows: `%LOCALAPPDATA%\Anvil\runtime\` + named pipe + Task
    Scheduler at logon (vNext).
  - WSL: same as Linux, scoped per distro.
- `os_locality_token` (introduced §3.2) takes the form
  `linux:<hostname>:<distro>` / `macos:<hostname>` /
  `windows:<sid>:<computername>` / `wsl:<windows-host>:<distro-name>`.
  This is the cross-check that prevents a Windows client from naively
  talking to a WSL UDS proxy.

**Risks**

- Sandboxed editors silently failing to attach. Mitigation: doctor
  command names the missing path; embedded-fallback MCP path remains.
- WSL auto-start: a Linux daemon inside WSL only runs while the distro is
  "warm". If the user closes the last terminal, WSL2 may shut down the
  distro after a short idle. Daemon goes with it. On next surface
  attach, daemon must auto-restart.

**Non-negotiables**

- Each OS gets its own platform-idiomatic runtime path.
- WSL distro is a separate OS instance — no cross-distro reuse.
- Auto-start logic must succeed on cold start (XDG dir doesn't exist
  yet).

**Questions**

- **Q-PLAT-1:** On macOS, do we ship with App Sandbox compatibility for
  Cursor, or accept embedded-fallback as the v1 macOS Cursor path?
- **Q-PLAT-2:** systemd user unit / launchd plist / Task Scheduler — do
  we ship these in v1, or only v1's auto-launcher and let supervisors
  come in vNext?
- **Q-PLAT-3:** Multiple Windows users on the same machine — separate
  daemons (named pipe path includes `{user_sid}`, INTD-002, so yes).

### 3.6 Adversarial reviewer

**Concerns and threats**

- **T-1 — False protection claim, cross-boundary.** User opens Cursor on
  Windows, project on `/mnt/c/repo`, agent works in WSL terminal in same
  repo. Windows daemon protects the file from the Cursor side; WSL
  daemon protects it from the WSL side; **neither sees the other's
  writes** because the file watcher in each runs on a different OS view.
  Anvil status on either side will say "protected". This is a lie.
  Mitigation: detect cross-boundary access, refuse to claim protection.
- **T-2 — Split-brain via two daemons same user.** Two `anvil intercept
  start` race; the loser exits cleanly (PID-file exclusive). But: what
  if the PID-file dir is on a filesystem where exclusive create is racy
  (NFS, some FUSE mounts)? On those filesystems, run-time state shouldn't
  live anyway; require local-FS runtime dir.
- **T-3 — Stale daemon, wrong project.** Daemon was started in repo A,
  user `cd`s to repo B, opens editor B. Surface in repo B handshakes, but
  daemon's session registry was warmed for repo A. Currently this is
  fine — registry is per-worktree. But if `info.json` is written in repo
  A's `.anvil/` (rejected — see §6), surface in repo B can't find it.
- **T-4 — Wrong daemon attached because path canonicalisation differs
  across surfaces.** Surface A canonicalises `~/proj` → `/home/u/proj`,
  surface B → `/Users/u/proj` (case folding, symlinks). Two sessions
  registered for the same actual worktree. Mitigation: daemon-side
  canonicalisation authority — surface sends original path, daemon
  canonicalises, returns canonical worktree id.
- **T-5 — Race on first-time auto-start.** Two surfaces both call
  `ensure` simultaneously. Both see "no daemon", both spawn. PID-file
  exclusive create elects a winner; loser exits, retries connect to
  winner. Acceptable. But: what if winner's startup is slower than
  loser's retry timeout? Current backoff (DRVR-001) handles this, but
  must be bounded so user sees feedback.
- **T-6 — Hostile child process pre-creates `info.json`.** Same-UID
  attacker writes a fake `info.json` pointing to an attacker-controlled
  socket. v1 defence: lstat + owner + mode check on `info.json`; refuse
  symlinks; re-stat after open via `O_NOFOLLOW`. Same as INTD-002.
- **T-7 — Version skew silently degrading protection.** Old DriverClient
  (v0.5.1) connects to new daemon (v0.6.0). Manifest handshake should
  refuse on incompatible proto version. Current model: `proto_version`
  in manifest, mismatch returns structured `anvil/capability/downgrade`
  with code `proto-version-mismatch`. Acceptable.
- **T-8 — Embedded fallback masquerading as daemon-backed.** RMCP shim's
  `DaemonValidationClient` returns `Unavailable`; shim runs embedded.
  User sees "Anvil pre-write validation: ON" without knowing it's
  embedded. Fix: the MCP response carries a `validation.backend` field
  (`daemon` / `embedded`), and `anvil status` distinguishes them.

**Non-negotiables**

- Cross-OS-boundary surfaces MUST be detected and refused.
- Two daemons same user MUST be impossible (PID-file exclusive create on
  local FS only).
- `info.json` MUST go through the same security ladder as the socket.
- Backend mode (`daemon` vs `embedded`) MUST be visible to the user and
  the MCP response.
- A surface that cannot validate `os_locality_token` MUST refuse to
  attach.

**Questions**

- **Q-ADV-1:** When two surfaces canonicalise the same worktree
  differently, who wins — first-registered, or daemon's own
  canonicalisation? (Architect says daemon; confirm.)
- **Q-ADV-2:** What happens when the daemon is killed mid-`scan_buffer`?
  Does the surface see a structured error or a transport drop? (RTAI-008
  pinned this — structured retriable error. Good.)
- **Q-ADV-3:** What is the documented behaviour when a surface attaches
  with `os_locality_token` mismatch? Refusal is non-negotiable; the
  error code and operator-visible message must be agreed.

---

## 4. Cross-cutting decisions surfaced

The questions above cluster into seven explicit decision points:

1. **D-1 — Daemon scope.** Per-user, per-OS-instance singleton (architect
   recommendation). One daemon per (uid, OS instance). WSL distro counts
   as its own OS instance.
2. **D-2 — Project identity.** Daemon-canonicalised path keyed on
   `(canonical_path, os_locality_token)`. No git origin URL, no inode
   composite (deferred to vNext).
3. **D-3 — Discovery.** `info.json` next to the socket / pipe; surfaces
   read it before handshake; same lstat ladder as the socket.
4. **D-4 — Lifecycle.** `anvil intercept ensure` is the launcher; daemon
   detaches and writes log to platform log path; PID-file exclusive
   create elects single-instance winner.
5. **D-5 — Cross-boundary.** Hard fence: each WSL distro is its own OS
   instance. Cross-Windows/WSL surface attachment is **detected and
   refused**, not bridged. Cross-OS reach is vNext + ADR.
6. **D-6 — Concurrency.** v1 keeps single-session-per-worktree; multi-
   session keyed by (driver_id, agent_id, pid) is v1.5. Daemon-side
   serialisation of `scan_buffer` requests per worktree (small fairness
   queue) is v1.
7. **D-7 — Protection-claim policy.** `anvil status` and the MCP
   response distinguish:
   - `attached` (driver connected) vs `participating` (enforcement-ack
     advertised),
   - `daemon-backed` vs `embedded-fallback`,
   - `same-os` vs `cross-boundary detected (refused)`.

These decisions feed directly into the spec
`2026-05-07-daemon-lifecycle-and-discovery.md` and the ADR.

---

## 5. Council remediation log

Findings recorded after the §6 spec council review (see spec doc §10).
Cross-referenced here for traceability.

- **C-1 (M, security):** v1 must explicitly refuse cross-OS attachment by
  `os_locality_token`, not just by path heuristics. Resolved: spec §3.4
  pins token format and refusal code.
- **C-2 (M, adversarial):** version-skew downgrade behaviour must be
  protocol-level, not advisory. Resolved: spec §4.3 adds
  `proto-version-mismatch` to the structured error set; existing
  DRVR-008 capability-downgrade vocabulary covers it.
- **C-3 (S, DX):** `anvil doctor` MUST exit non-zero on cross-boundary
  detection so CI / scripts notice. Resolved: spec §7.2 pins exit codes.
- **C-4 (S, runtime):** macOS App Sandbox path must be named in the spec
  even if the answer is "embedded fallback only for v1". Resolved:
  spec §5.3 calls it out.
- **C-5 (M, product):** the protection-claim policy must include "no
  surface attached yet" — the activation moment. Resolved: spec §8 adds
  `protection: warming` state.

(These are remediation pointers, not the findings themselves; see the
spec council review for the full text.)
