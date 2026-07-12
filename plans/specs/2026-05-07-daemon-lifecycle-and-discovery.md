# Daemon Lifecycle, Discovery, and Boundary Policy

**Status:** Superseded (2026-05-07) by
[`2026-05-07-anvil-multilayer-protection-architecture.md`](./2026-05-07-anvil-multilayer-protection-architecture.md)
which extends scope from "where the daemon lives" to the full
multi-layer protection architecture (witness chain, hooks, L4 policy,
baseline, multi-agent coordination, rule distribution). DLIFE work
items proposed here are largely retained; some (DLIFE-008
multi-session-per-worktree) are promoted to v1 there as MLP-014.
This document is preserved for the round-1 design context and the
detailed daemon-discovery / OS-locality / `info.json` mechanics, all
of which the superseding spec references rather than restates.

**Date:** 2026-05-07
**APS:** Proposes new module `daemon-lifecycle` (DLIFE) and updates to INTD,
DRVR, RMCPF.
**Decision:** ADR-036 (`daemon-scope-discovery-and-boundaries.md`)
**Brainstorm:** [2026-05-07-daemon-sessions-surfaces-boundaries.md](../brainstorms/2026-05-07-daemon-sessions-surfaces-boundaries.md)

> **Inner-shape rule.** This spec adds new metadata around the daemon, but
> does not change the canonical [`anvil.diagnostic.v1`][diag] inner shape,
> nor the JSON-RPC method semantics pinned by
> [`2026-05-06-editor-driver-protocol.md`][edp]. New fields live on
> manifests, `info.json`, and status payloads.

[diag]: ./2026-04-26-diagnostic-envelope-coordination.md#canonical-inner-shape-diagnostic
[edp]: ./2026-05-06-editor-driver-protocol.md

---

## 0. What this document is

This spec answers, concretely:

- What the Anvil daemon is **scoped to** (D-1).
- How surfaces **discover** the right daemon (D-3, D-2).
- When a daemon is **reused** vs **started** (D-4).
- How **multi-surface** and **multi-agent** sessions interact (D-6).
- How **cross-OS-boundary** scenarios are detected and refused (D-5).
- What Anvil is **allowed to claim** about protection in each state (D-7).

Implementation work is broken into a new APS module
`daemon-lifecycle.aps.md` (DLIFE) defined at the end. The runtime
behaviour pinned here supersedes the implicit assumptions in
`intercept-daemon.aps.md` §Purpose ("per-user persistent singleton" —
restated and reinforced here, not changed) and
`rust-mcp-launch-shim.aps.md` (which inherits a clearer protection-claim
policy via D-7).

---

## 1. Architecture options assessed

Four options were evaluated. Each section is a structured comparison; the
recommended option is **B (per-user daemon)**, augmented with explicit
discovery metadata, lazy auto-start, and OS-locality fencing. Justification
is in §1.5.

### 1.1 Option A — Per-project daemon

> One daemon per project root.

| Dimension                    | Assessment                                                                                                                                  |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| Simplicity                   | Conceptually clean: one project, one daemon. But N projects = N daemons, N socket paths, N PID files, N watcher trees, N rule registries.    |
| Correctness                  | High. Bounded blast radius: a crash in one project never affects another. Fence state is naturally per-project.                               |
| Multi-surface support        | Trivial within a project (all surfaces talk to the same project daemon). Bad across projects (cross-project status / dashboard hard).         |
| WSL compatibility            | Same as B — each daemon is on the same OS as its project. Cross-OS still fenced.                                                              |
| Security                     | Stronger isolation. A compromised project daemon cannot leak fences across projects.                                                          |
| Performance                  | **Worst** — N rule registries, N tree-sitter parser pools, N watcher trees, N file caches. RAM footprint scales with project count.           |
| Failure modes                | Each daemon must independently auto-start, supervise, log. Diagnostic surface fans out N times.                                               |
| Implementation complexity    | Highest. Requires per-project discovery (lockfile in `.anvil/`), per-project supervisor, per-project log rotation.                            |
| Fit for A2 (RMCP-driven)     | Bad. The MCP shim is a child of the editor and lives outside any project root; it would have to find the right project daemon by `cwd`.       |
| Fit for future DRVR / driver | Bad. A driver in a multi-project editor (VSCode workspace with 4 folders) needs 4 daemons, 4 handshakes, 4 sessions.                          |

### 1.2 Option B — Per-user daemon with project registry **(recommended)**

> One daemon per (uid, OS instance), serving many project sessions.

| Dimension                    | Assessment                                                                                                                                  |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| Simplicity                   | One daemon, one socket path, one PID file, one log per OS instance. Surfaces always know "where" the daemon should be.                        |
| Correctness                  | High once session-per-worktree (current INTD-003) extends to multi-session-per-worktree (v1.5). Daemon-side canonicalisation is authority.    |
| Multi-surface support        | Native. All editors, terminals, MCP shims share one daemon; daemon fans out per-session events to all attached drivers.                       |
| WSL compatibility            | Per-distro daemon (each WSL distro is its own OS instance). Windows native is its own. Cross-boundary fenced by `os_locality_token`.          |
| Security                     | Same-UID trust (`SO_PEERCRED` / DACL). Cross-session redaction already pinned (INTD-015).                                                     |
| Performance                  | **Best** for typical use: one parser pool, one watcher tree (with per-worktree subtrees), one rule registry.                                  |
| Failure modes                | Single point of failure within an OS instance. Mitigated by lazy auto-start, fence-state persistence, and detached log file.                  |
| Implementation complexity    | Moderate. Reuses INTD-001..INTD-016 wholesale; adds `info.json`, `ensure` launcher, `os_locality_token`, refusal codes.                       |
| Fit for A2                   | Direct fit. RMCP shim already opens UDS / pipe; only adds `info.json` read + token check.                                                     |
| Fit for future DRVR / driver | Direct fit. DriverClient (DRVR-001) already does this.                                                                                        |

### 1.3 Option C — Per-surface daemon with coordination

> Each surface starts its own daemon; daemons coordinate via shared
> lockfile / leader election.

| Dimension                    | Assessment                                                                                                                                  |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| Simplicity                   | Looks simple ("each surface owns its daemon") but coordination logic is non-trivial (leader election, fence merge).                          |
| Correctness                  | Worst. Split-brain risk: two daemons with overlapping watchers see different change orderings.                                                |
| Multi-surface support        | Inverted: support is the cost, not the benefit.                                                                                               |
| WSL compatibility            | Same as B.                                                                                                                                    |
| Security                     | Worse: more attack surface (more sockets, more PIDs).                                                                                         |
| Performance                  | Worst: N daemons, leader-election traffic.                                                                                                    |
| Failure modes                | Leader-election bugs, partition handling. Anvil is not a distributed system; introducing one is wrong.                                        |
| Implementation complexity    | Highest. Discarded.                                                                                                                           |
| Fit for A2 / DRVR            | Bad.                                                                                                                                          |

### 1.4 Option D — Hybrid supervisor + per-project workers

> A user-level supervisor process owns discovery and lifecycle; per-project
> worker processes handle validation. Akin to systemd with units.

| Dimension                    | Assessment                                                                                                                                  |
|------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| Simplicity                   | Conceptually elegant — separates "where do I find Anvil" from "who validates this project".                                                   |
| Correctness                  | High. Workers can be restarted without affecting other projects.                                                                              |
| Multi-surface support        | Good. Supervisor is the discovery point.                                                                                                      |
| WSL compatibility            | Same as B.                                                                                                                                    |
| Security                     | Workers can be sandboxed individually. Stronger model.                                                                                         |
| Performance                  | Worker spawn cost on first project touch. Per-worker parser pool is the cost we wanted to avoid in A.                                         |
| Failure modes                | Two-level lifecycle bugs. Worker zombie reaping. Supervisor restart loses state.                                                              |
| Implementation complexity    | Highest of the four. Two binaries, two protocols (sup→worker, surface→sup or surface→worker), one more spec layer.                            |
| Fit for A2                   | Premature. A2 doesn't need worker isolation; only ever protects one project per "wave" of attention.                                          |
| Fit for future DRVR          | Latent option for vNext+ if we ever need stronger sandboxing per project. Not today.                                                          |

### 1.5 Recommendation: B, augmented

**Per-user daemon (B)** is recommended for v1 and v1.5. It maps cleanly to
the existing INTD scaffold (no rewrite), serves the multi-surface story
naturally, and has the smallest implementation diff.

**Reject A** for performance and discovery cost. **Reject C** for
correctness. **Reserve D** as the eventual evolution path if and only if
sandboxing-per-project becomes a hard requirement (e.g., enterprise tenancy
isolation). Re-evaluate at that time.

The augmentations B needs (covered in §§2–8):

- Explicit `info.json` discovery sidecar.
- `os_locality_token` for boundary detection.
- `anvil intercept ensure` lazy launcher.
- v1.5 multi-session-per-worktree via composite session keys.
- Protection-claim policy with named states.

---

## 2. Daemon identity and project identity

### 2.1 Daemon identity

A daemon instance is uniquely identified by:

```
DaemonId := (uid, os_locality_token)
```

There is **at most one** daemon per `DaemonId` per machine. PID-file
exclusive create on the local runtime FS guarantees this; second-instance
attempts exit with `daemon-already-running`.

`os_locality_token` is the SHA-256 prefix (8 bytes, lowercase hex) of:

| Platform        | Source string                                                                  |
|-----------------|--------------------------------------------------------------------------------|
| Linux native    | `linux\0<uid>\0<machine-id>` where machine-id is `/etc/machine-id`             |
| macOS           | `macos\0<uid>\0<IOPlatformUUID>` from `IOPlatformExpertDevice`                 |
| Windows native  | `windows\0<user-sid>\0<MachineGuid>` from `HKLM\Software\Microsoft\Cryptography\MachineGuid` |
| WSL distro      | `wsl\0<uid>\0<host-machine-guid>\0<distro-name>` — see derivation rules below |

**WSL distro-name derivation (hardened against same-UID spoofing — F-A1).**
The distro name is **not** taken from `WSL_DISTRO_NAME` (env var, user-
writable) or `/etc/wsl.conf` (root-writable inside the distro but the
daemon often runs as the user). Source order:

1. Read `/proc/sys/kernel/osrelease` and confirm it contains the literal
   substring `WSL2` (or `Microsoft` for WSL1; reject WSL1 with
   `daemon-locality-untrusted: wsl1-unsupported`).
2. Resolve the distro identifier from `/proc/mounts`: the `9p` rootfs
   line carries an immutable mount-tag (`drvfs` for `/mnt/c`,
   distro-named tags for the root). The exact tag form is captured in
   DLIFE-002's per-platform fixture set.
3. As a tertiary cross-check, compare against `WSL_DISTRO_NAME` if
   present; **mismatch is logged but does not change the token**
   (env var is advisory only).

This makes the input to the hash sourced from kernel-controlled state
inside the distro VM, not from user-writable surfaces. A same-UID
attacker setting `WSL_DISTRO_NAME=Ubuntu` from inside Debian still
derives the Debian token because step 2 is authoritative.

**Why hash-and-truncate:** the token is a stable, opaque identifier. We
don't want to leak hostname or user identifiers into log lines; an 8-byte
prefix is sufficient to make accidental collisions vanishing. Surface
liveness check is exact-match against `info.json`, not a partial match.

**Token derivation site:** the daemon computes the token **once** at
startup and writes it into `info.json`. Surfaces re-derive their own
token using the same algorithm at attachment time and compare against
`info.json` for equality. Surfaces never trust `info.json`'s token as
ground truth — they verify it matches their own derivation.

**Identity uniqueness caveat (F-B1).** `(uid, os_locality_token)`
uniqueness holds **only on a non-containerised, non-namespace-remapped
local filesystem**. In rootless Docker, user-namespace remapping, and
some bind-mounted dev-container scenarios, two daemons may compute the
same token and contend for the same runtime path. v1 explicitly does
not support these (§9.3); ADR-036 records the caveat. Beyond the
PID-file exclusive-create guard, no further mitigation is in v1.

### 2.2 Project identity (worktree key)

A worktree is keyed by:

```
WorktreeKey := canonicalise(workspace_root_path)
```

where `workspace_root_path` is the directory containing `.anvil.yaml`, or
the registered cwd if none. **The daemon is the canonicalisation
authority** — surfaces send their original path; the daemon resolves it
(symlinks, case folding, trailing slashes) and replies with the canonical
key. This resolves T-4 (surfaces canonicalising differently).

**Composite identity is deferred.** Adding `git_dir_inode` or
`repo_origin_url` to the key is a vNext consideration:

- Repo-origin would group worktrees of the same repo across machines (no
  v1 use).
- Inode would distinguish bind mounts of the same path (rare, decision
  cost > benefit).

v1: **path-only**, daemon-canonicalised, with a documented caveat:
"two paths that both canonicalise to the same string share a session.
Use distinct paths for distinct worktrees."

### 2.3 Session key (v1 vs v1.5)

- **v1:** `(WorktreeKey)` — single session per worktree (current INTD-003).
- **v1.5:** `(WorktreeKey, AgentTag)` where `AgentTag` is a daemon-minted
  opaque id assigned at session-register, derived from
  `(driver_id, claimed_agent_id, pid_starttime)`. The driver-supplied
  `claimed_agent_id` is purely a user-facing label; it does not affect
  identity for trust purposes. This allows Cursor + Claude Code in the
  same worktree to be two distinct sessions (scenario 7).

The composite-key migration is additive: the daemon accepts old (single-
session) registrations as `AgentTag = "default"` and emits a deprecation
log line.

---

## 3. Discovery proposal

### 3.1 Where discovery metadata lives

Two locations:

1. **Runtime sidecar (authoritative):**
   `<runtime_dir>/anvil/intercept.info.json`, written by the daemon at
   `listen()` time and atomically replaced on restart. Same parent as
   the socket / pipe.
2. **Optional project-level pointer (advisory only):** `.anvil/runtime`
   (a regular file containing the daemon's `os_locality_token`). Used
   by editors that want to detect "this project was last validated by
   another OS" — for example, a Linux editor opening a project last
   touched on Windows. This is **never** an attachment authority — the
   surface still discovers via the runtime sidecar.

Both files are subject to the lstat ladder (INTD-002).

### 3.2 `info.json` shape

```jsonc
{
  "schema": "anvil.daemon.info.v1",
  "pid": 17341,
  "ready": true,
  "started_at": "2026-05-07T12:34:56Z",
  "started_at_starttime_ticks": 234809123,
  "starttime_source": "linux:/proc/<pid>/stat[22]",
  "version": "0.6.0",
  "proto_version": 2,
  "os_locality_token": "linux:8d3f1a2c",
  "transport": {
    "kind": "unix-socket",
    "path": "/run/user/1000/anvil/intercept.sock"
  },
  "log_path": "/home/u/.local/state/anvil/intercept.log",
  "spawn_log_path": "/run/user/1000/anvil/intercept-spawn-fail.log",
  "panic_log_path": "/home/u/.local/state/anvil/intercept-panic.log",
  "supports": {
    "scan_buffer": true,
    "anvil_methods": [
      "anvil/scan_buffer",
      "anvil/publishDiagnostics",
      "anvil/enforcement/decision",
      "anvil/enforcement/ack",
      "anvil/gate/request",
      "anvil/suppression/apply",
      "anvil/status/query"
    ]
  }
}
```

**`ready` field (F-D1).** The daemon writes `info.json` in two phases:

1. **Phase 1 — at `listen()` time:** write `info.json` with
   `ready: false`. Surfaces that read this MUST poll for `ready: true`
   within the bounded startup budget (§4.2).
2. **Phase 2 — after init complete:** atomically replace `info.json`
   (rename-over-temp-sibling) with `ready: true`. Init-complete means:
   rule registry loaded, fence state restored from disk, watcher
   subscribed, IPC accept loop running.

A surface that connects to a `ready: false` daemon receives
`-32099 daemon-not-ready` on any non-`status` request (and may retry).

**Per-platform `starttime_source` (F-D2 / Ops-B).** The daemon records
which mechanism produced `started_at_starttime_ticks` so that surfaces
re-implement the *same* check:

| Platform | `starttime_source` value                    | Mechanism                                        |
|----------|---------------------------------------------|--------------------------------------------------|
| Linux    | `linux:/proc/<pid>/stat[22]`                | Read `/proc/<pid>/stat`, field 22 (jiffies)      |
| macOS    | `macos:proc_pidinfo:pbi_start_tvsec`        | `proc_pidinfo(PROC_PIDTBSDINFO)` start time      |
| Windows  | `windows:GetProcessTimes:ftCreation`        | `GetProcessTimes()` creation time, FILETIME     |
| WSL      | `linux:/proc/<pid>/stat[22]`                | Same as Linux native                            |

The starttime helper is part of DLIFE-001 and is shared with
INTD-006's process-termination ladder (the spec must verify INTD-006
covers all three platforms; if it does not, DLIFE-001 grows the
helper rather than copying it).

**Temp-file safety (F-F1).** The atomic-replace uses a non-predictable
temp sibling, created with `O_EXCL | O_NOFOLLOW | O_CREAT` mode 0600
inside the already-0700 runtime dir, with a per-write nonce in the
filename (`intercept.info.json.<random8>.tmp`). DLIFE-001's tests
include "temp file path is not a predictable constant" and
"pre-placed symlink at predictable temp path is not used".

**`spawn_log_path` and `panic_log_path` (Ops-A, Ops-C).**

- `spawn_log_path`: a temporary file the launcher pre-creates before
  `posix_spawn` / `CreateProcess`, used to capture early daemon stderr
  before the operational log is open. Renamed-and-cleaned on
  successful daemon startup; persisted (and surfaced by
  `anvil doctor`) on spawn failure.
- `panic_log_path`: separate from the rotated operational log. Rust
  panic backtrace and any abort traces land here so they survive
  log rotation. `anvil doctor` reads its tail when reporting exit
  codes 1 / 3.

**Lifecycle of `info.json`:**

- Written `O_EXCL | O_NOFOLLOW`, mode 0600 (Unix) / owner-only DACL
  (Windows), in the same dir as the socket/pipe (already 0700 / owner-
  only).
- Replaced atomically on restart via `rename()` over a temp sibling.
- Removed on graceful shutdown.
- A surface that finds an `info.json` whose `pid` is dead OR whose
  `started_at_starttime_ticks` does not match the live `/proc/<pid>/stat`
  (or platform equivalent) treats it as **stale**.
- A surface that finds `info.json` missing but the socket present treats
  the daemon as **legacy** (pre-DLIFE) and connects with proto_version
  = 1, with a downgrade warning.

### 3.3 How surfaces find the daemon

Surface discovery sequence:

1. Compute `runtime_dir` per platform (see §5).
2. Compute expected `os_locality_token` for the running surface.
3. `open(runtime_dir/anvil/intercept.info.json, O_NOFOLLOW)` and read.
4. **Token check:** if `info.json.os_locality_token != expected_token`,
   refuse with `cross-boundary-detected` (see §3.4 and §8).
5. **Liveness check:** signal-0 `pid`; on Unix, also re-stat
   `/proc/<pid>/stat` field 22 against
   `started_at_starttime_ticks` to defeat PID reuse.
6. **Version check:** compare `proto_version` against surface's bound
   range; if outside, refuse with `proto-version-mismatch` and offer
   "run `anvil intercept ensure` to upgrade daemon" guidance.
7. Connect to `transport.path`.
8. Send `initialize` with surface's manifest.

### 3.4 Cross-boundary detection (the OS-locality token in action)

If a Windows surface (Cursor.exe) inspects
`\\wsl.localhost\Ubuntu\run\user\1000\anvil\intercept.info.json` (assuming
it's even reachable), it will read `os_locality_token = wsl:...` while
its expected token is `windows:...`. The surface refuses to attach,
emits a structured `cross-boundary-detected` event with both tokens, and
falls back to embedded mode for MCP / read-only mode for editor.

**Refusal codes:**

| Code                             | Meaning                                                                  |
|----------------------------------|--------------------------------------------------------------------------|
| `cross-boundary-detected`        | `os_locality_token` mismatch; surface and daemon are on different OSes   |
| `daemon-stale`                   | `info.json` PID is dead or starttime mismatch                            |
| `proto-version-mismatch`         | `proto_version` outside surface's supported range                        |
| `daemon-not-running`             | Neither `info.json` nor socket exists                                    |
| `daemon-locality-untrusted`      | `info.json` ownership / mode invalid (lstat ladder failed)               |
| `daemon-already-running`         | Second `intercept ensure` couldn't elect winner (rare, race log only)    |

These codes appear in `anvil status`, `anvil doctor`, the MCP response's
`validation.backend` field, and editor-driver `anvil/capability/downgrade`
notifications.

### 3.5 Auth/trust local model

- **Default (v1):** same-UID via `SO_PEERCRED` (Unix) / pipe DACL
  (Windows). Carried over from INTD-002 + DRVR-007 verbatim.
- **Out of scope:** cross-UID, cross-host, TCP transports. ADR amendment
  required to add any of these.

---

## 4. Lifecycle proposal

### 4.1 Who starts the daemon?

The **launcher** does, via `anvil intercept ensure`. The launcher is
invoked by:

- Surfaces' driver-client startup (`DriverClient.connect()` calls it
  internally — DRVR-001 amendment).
- The MCP shim (`anvil mcp serve --stdio`) on first request, before
  attempting daemon connection.
- The user explicitly via `anvil start` (LAUNCH-006 alias).
- Optionally a platform supervisor (systemd user, launchd, Task
  Scheduler) on session login — vNext.

### 4.2 `anvil intercept ensure` semantics

Idempotent. Pseudocode (reconciled with §4.8 race handling — F-Prag-B):

```
ensure():
  loop up to BOUNDED_STARTUP_BUDGET (ADR-031 rubric):
    info = read_info_json()      // returns None on missing, stale, or
                                  // ownership/lstat-ladder violation
    if info is Some and confirmed_alive(info) and info.ready == true:
      if info.proto_version not in surface_supported_range:
        return Err(ProtoVersionMismatch{
          hint: "run `anvil intercept restart` to upgrade the daemon"
        })
      if info.os_locality_token != self.expected_token():
        return Err(CrossBoundaryDetected{
          surface_token, daemon_token: info.os_locality_token,
          remediation: "run `anvil doctor --explain-boundary`"
        })
      return Ok(info)

    if info is Some and not confirmed_alive(info):
      reap_stale_runtime_files()  // shared predicate (F-F2)

    spawn_outcome = spawn_daemon_detached_with_safe_env()  // F-E1
    // Possible outcomes:
    //   Ok(child_pid)              — spawn succeeded, our daemon may
    //                                 win or lose the PID-file race
    //   Err(SpawnFailed)           — exec failed before daemon ran;
    //                                 read spawn_log_path and bubble up
    //   Err(AlreadyRunningExit)    — child exited with
    //                                 daemon-already-running (loser of
    //                                 race); fall through to next loop
    if spawn_outcome is Err(SpawnFailed):
      return Err(StartFailed{spawn_log_tail: ...})
    // For both Ok and AlreadyRunningExit, loop and re-read info.json

  return Err(StartTimedOut)


confirmed_stale(info):                     # shared predicate F-F2
  return signal_zero(info.pid) is dead OR
         starttime_check(info.pid, info.started_at_starttime_ticks,
                         info.starttime_source) != match
```

`confirmed_alive` is the negation of `confirmed_stale`. Both
`ensure()` and `anvil doctor --reap` MUST call this same predicate;
DLIFE-001 owns it.

**Spawn mode (with hardened environment — F-E1):**

- Linux/macOS: `posix_spawn` of `anvil intercept start --background
  --log-file <path>`; `setsid()`; stdio redirected to `spawn_log_path`
  initially, then to `log_path` once the daemon opens it. **Environment
  is cleared with `env_clear()` then selectively re-added**: `PATH`
  (sanitised to a known-good value if untrusted), `HOME`, `XDG_*`
  (filtered), `TZ`, `LANG`, and an explicit allowlist; `LD_PRELOAD`,
  `LD_LIBRARY_PATH`, `DYLD_INSERT_LIBRARIES`,
  `DYLD_FORCE_FLAT_NAMESPACE`, and `DYLD_LIBRARY_PATH` are dropped.
- Windows: `CreateProcess` with `DETACHED_PROCESS |
  CREATE_NEW_PROCESS_GROUP`, no console window. `lpEnvironment` built
  from the same allowlist; `PATH` sanitised. The launcher resolves the
  daemon binary via `GetModuleFileNameW` of the calling CLI binary
  (sibling-resolution), not by `PATH` search, to defeat search-order
  hijacking (Ops-G).
- WSL: same as Linux.

**Daemon binary resolution (Ops-G).** The launcher resolves the daemon
binary as a sibling of the calling CLI (`anvil`). Per platform:

| Platform | Resolution                                                  |
|----------|-------------------------------------------------------------|
| Linux    | `readlink /proc/self/exe` → `dirname` → `<dir>/anvil`       |
| macOS    | `_NSGetExecutablePath` → `dirname` → `<dir>/anvil`          |
| Windows  | `GetModuleFileNameW(NULL)` → `dirname` → `<dir>\anvil.exe`  |

Documented in DLIFE-004. If sibling-resolution fails, fall back to
`std::env::current_exe()` per Rust convention; do not search `PATH`.

**Bounded polling.** The wall-clock budget for the entire `ensure()`
call (including spawn, init, and `ready: true` flip) is owned by
ADR-031's latency rubric and recorded as a measured constant on the
dashboard, not as text in this spec.

### 4.3 Who owns the daemon?

The `(uid, os_locality_token)` pair owns the daemon. No surface "owns"
it — surfaces attach and detach freely. The daemon survives:

- Terminal exit (it's detached).
- Editor exit (same).
- All-surface detach (it stays running, idle).

It exits on:

- `SIGTERM` / `SIGINT` (graceful — INTD-001).
- `anvil intercept stop` (sends `SIGTERM`).
- Idle timeout, **only** if configured (`enforcement.daemon.idle_exit:
  3600` — vNext, default disabled).

### 4.4 Multiple-client tracking

Already covered by INTD-013, INTD-015, and `editor-and-mcp-driver-design.md`
§2.5–2.8. New: each client connection is tagged with its `manifest.driver_id`
+ `pid` + accept-time so `anvil status` lists "Cursor (pid 4923,
attached 12 min)".

### 4.5 Session attachment

A surface attaches by:

1. `ensure()` (§4.2) — daemon up.
2. `initialize` request with manifest.
3. `register_session` request with `(workspace_root_path, claimed_agent_id?)`.
4. Daemon canonicalises path, returns `WorktreeKey` and `AgentTag`.
5. From here on, surface refers to its session by `(WorktreeKey,
   AgentTag)`.

A surface detaches by closing the connection or sending `unregister`. The
daemon TTL-evicts dead sessions (INTD-003, 30s heartbeat).

### 4.6 Stale-daemon recovery

`anvil doctor --reap` removes stale runtime files:

- `intercept.info.json` whose `pid` is dead.
- `intercept.pid` whose pid is dead AND starttime mismatch.
- `intercept.sock` whose accept side has no listener (connection refused
  on test).

Reap is **opt-in** because automatic reap can race with a daemon currently
starting up. The launcher (§4.2) does opportunistic reap only on confirmed
staleness.

### 4.7 Daemon upgrade

- A new `anvil` binary on disk + an old running daemon = `version` field
  in `info.json` may differ from `anvil --version`.
- `ensure()` reads `info.json`; if `proto_version` is outside the
  surface's range, the surface refuses.
- The user's recovery: `anvil intercept restart` (sends SIGTERM, waits,
  re-runs ensure with new binary).
- Auto-restart on version skew is **not** in v1: it's a footgun (mid-
  edit interruption).

### 4.8 Two-`ensure` race

Both surfaces call `ensure()` simultaneously. Both see no `info.json`,
both spawn. The winner of `O_EXCL` PID-file create binds the socket and
writes `info.json`; the loser's daemon exits with
`daemon-already-running`. The loser's launcher detects exit, re-reads
`info.json`, finds the winner, and returns Ok.

This requires the launcher to read `info.json` after a spawn fails as
well as after spawn succeeds. Pseudocode in §4.2 covers it.

---

## 5. OS-specific runtime paths

### 5.1 Linux

| Item            | Path                                                                |
|-----------------|---------------------------------------------------------------------|
| Runtime dir     | `$XDG_RUNTIME_DIR/anvil/` (fallback `$HOME/.local/state/anvil/`)    |
| Socket          | `<runtime_dir>/intercept.sock`                                      |
| `info.json`     | `<runtime_dir>/intercept.info.json`                                 |
| PID file        | `<runtime_dir>/intercept.pid`                                       |
| Log             | `$XDG_STATE_HOME/anvil/intercept.log` (fallback `~/.local/state/anvil/intercept.log`); rotated on size cap |
| Supervisor v1   | none — `ensure` launcher is the lifecycle owner                     |
| Supervisor vNext| systemd user unit `anvil.service`                                   |

### 5.2 macOS

| Item            | Path                                                                |
|-----------------|---------------------------------------------------------------------|
| Runtime dir     | `~/Library/Application Support/Anvil/runtime/`                     |
| Socket          | `<runtime_dir>/intercept.sock`                                      |
| `info.json`     | `<runtime_dir>/intercept.info.json`                                 |
| PID file        | `<runtime_dir>/intercept.pid`                                       |
| Log             | `~/Library/Logs/Anvil/intercept.log`; rotated                       |
| Supervisor v1   | none                                                                |
| Supervisor vNext| launchd user agent `~/Library/LaunchAgents/io.eddacraft.anvil.plist`|

### 5.3 macOS — App Sandbox edge case

Cursor (and other Mac App Store / sandboxed editors) cannot reach UDS
outside their sandbox container. v1 behaviour:

- The MCP shim `anvil mcp serve --stdio` runs as a child of the sandboxed
  editor and inherits the sandbox.
- The shim's discovery attempts to open
  `~/Library/Application Support/Anvil/runtime/intercept.info.json` and
  fails with EPERM.
- Shim emits `daemon-locality-untrusted: sandbox-isolated` and falls
  back to embedded validation.
- `anvil status` (run from outside the sandbox) sees the daemon up but
  reports the editor as "sandbox-isolated" via correlation ID written to
  the MCP response.

vNext options (out of scope here): shared App Group container (requires
notarisation cooperation with editor vendors), or XPC bridge with
documented entitlements.

### 5.4 Windows

| Item            | Path                                                                |
|-----------------|---------------------------------------------------------------------|
| Runtime dir     | `%LOCALAPPDATA%\Anvil\runtime\` (typically `C:\Users\<u>\AppData\Local\Anvil\runtime\`) |
| Pipe            | `\\.\pipe\anvil-intercept-{user_sid}` (already INTD-002)            |
| `info.json`     | `<runtime_dir>\intercept.info.json`                                 |
| PID file        | `<runtime_dir>\intercept.pid`                                       |
| Log             | `%LOCALAPPDATA%\Anvil\Logs\intercept.log`; rotated                  |
| Supervisor v1   | none                                                                |
| Supervisor vNext| Task Scheduler "at logon" task                                      |

### 5.5 WSL (per distro)

Each WSL distro is a separate OS instance. Inside the distro it looks
identical to Linux (§5.1). The runtime dir is `/run/user/<uid>/anvil/`,
the socket is local to the distro, and `os_locality_token` is
`wsl:<host-machine-guid>:<distro-name>`. Windows surfaces cannot reach
this socket.

### 5.6 Per-platform `os_locality_token` examples

| Platform        | Token form                                                             |
|-----------------|------------------------------------------------------------------------|
| Linux native    | `linux:e8a91c33`                                                        |
| macOS           | `macos:7d3f102b`                                                        |
| Windows         | `windows:1c4ab2d0`                                                      |
| WSL Ubuntu      | `wsl:1c4ab2d0:Ubuntu` (note: hex hash differs from Windows token because WSL distro name is in the source string) |

---

## 6. WSL and cross-boundary deep-dive

### 6.1 Can a Windows process talk to a Linux UDS inside WSL?

**Not directly, in any portable way.**

- WSL2 runs each distro inside a lightweight VM. The Windows host can
  see the distro's filesystem via `\\wsl.localhost\<distro>\` (a 9P
  bridge), but the bridge does **not** support `connect()` on a UNIX
  socket file. Opening `\\wsl.localhost\Ubuntu\run\user\1000\anvil\intercept.sock`
  as a regular file returns ENOTSUP / "the file cannot be accessed" or
  similar; it is not a usable AF_UNIX endpoint.
- There is **no reverse: Linux → Windows named pipe** without a
  forwarder either; named pipes are Windows-IPC only.
- WSL2 mirrored network mode (Windows 11 23H2+) makes localhost-TCP
  trivially shared, but TCP is rejected in v1 (§3.5,
  `2026-05-06-editor-driver-protocol.md` §6.1).
- `socat` / `wsl --exec` bridges exist but are user-managed and not a
  v1 protection mechanism.

### 6.2 Should Windows-hosted editors use a Windows daemon, WSL daemon, or bridge?

**Match the tooling.** Specifically:

- If the agent / CLI / build tool runs on Windows → **Windows daemon**.
- If the agent / CLI runs in WSL → **WSL daemon (in that distro)**.
- If both — the project genuinely lives in two worlds — **v1 detects and
  refuses to claim cross-boundary protection**.

The user-visible rule: **the daemon lives where the writes happen.**
Anvil watches the file system; if writes happen via the WSL ext4
mount, they're observed by an inotify watcher in the WSL distro.
Windows writes via `\\wsl.localhost` go through the 9P bridge and
**do not generate inotify events on the Linux side reliably**. This
is the technical reason — not just a policy reason — that
cross-boundary "protection" is a lie.

### 6.3 What happens when the project path is `C:\repo` from Windows and `/mnt/c/repo` from WSL?

Two different canonical paths, two different `WorktreeKey`s, two
different sessions. The user can run separate Anvil daemons in each OS
and protect each side independently — but that's two protection claims,
not one.

The project-level `.anvil/runtime` pointer (§3.1) lets the WSL surface
notice "this project was last touched by `windows:1c4ab2d0`" and
*warn*: "you opened this in WSL, but Anvil last validated it from
Windows. Cross-boundary edits are not protected — pick one side."

This is the v1 honesty mechanism: Anvil refuses to lie. It tells the
user which boundary they crossed.

### 6.4 What happens when the project lives inside the WSL filesystem?

If the user opens `\\wsl.localhost\Ubuntu\home\u\proj` in Cursor on
Windows, Cursor sees Windows paths (`\\wsl.localhost\Ubuntu\...`). A
Windows daemon would inotify on… nothing useful (9P bridge doesn't
forward inotify reliably). A WSL daemon would inotify on
`/home/u/proj` and miss Cursor's edits (which happen via 9P, not
through Linux syscalls).

This is the canonical broken-by-design case. **v1: refuse to protect
this configuration.** Doctor explains why and recommends running the
editor inside WSL (Remote-WSL — scenario 12) or natively against a
Windows-side checkout.

### 6.5 Should daemon identity be based on canonical path, repo identity, git root, inode/device, or explicit project ID?

**Canonical path, daemon-side canonicalised** — for v1 (§2.2). The other
options:

- **Git root:** breaks for non-git projects; gives nothing canonical
  doesn't already give.
- **Inode/device:** distinguishes bind mounts of the same path. Real
  but rare. Punt.
- **Repo origin URL:** lets us cluster "the same repo on multiple
  machines". Out of v1 scope.
- **Explicit project ID** (e.g. UUID in `.anvil/project-id`):
  high engineering cost for a problem we don't have. Out of v1 scope.

If a future scenario needs richer identity, extend `WorktreeKey` to
`(canonical_path, optional_repo_origin, optional_project_id)` with the
extra fields default-null in v1 and ignored by daemon-side equality.

### 6.6 Is path translation safe enough?

**No, and v1 will not attempt it.** Translating `C:\repo` ↔ `/mnt/c/repo`
is straightforward syntactically; the failure modes are:

- Case folding (Windows case-insensitive vs Linux case-sensitive).
- Symlink semantics (NTFS junctions ≠ Linux symlinks).
- Permission semantics (NTFS ACL ≠ POSIX mode).
- Watcher semantics (inotify on a 9P mount is unreliable).
- Concurrent writes (no shared lock between the two sides).

The right v1 answer is: **don't translate, refuse**. v2 can revisit
with a documented bridge driver and a clear protection-claim
downgrade.

### 6.7 Should v1 explicitly avoid cross-boundary claims?

**Yes.** This is the pivotal v1 stance: Anvil is allowed to say "this
configuration is unprotected" and the user is supposed to act on that.
Saying "protected" when one side of the boundary is unwatched is the
worst failure mode (false confidence).

### 6.8 What would a future bridge look like?

For the record, not for v1:

- A **bridge daemon** in the "weak" environment that proxies validation
  RPCs (with size-bounded request/response framing) to the "strong"
  daemon. e.g. a Windows-side `anvil-bridge.exe` that forwards
  `scan_buffer` RPCs over `\\.\pipe\` to a known WSL distro's `anvil-
  bridge` listener, which forwards to its local UDS daemon. This is a
  *protocol bridge*, not a transport tunnel.
- The bridge would have to advertise itself with a **distinct**
  `os_locality_token` (`bridge:windows->wsl:Ubuntu:...`) and the
  daemon side would have to recognise bridge-attached sessions as a
  trust-distinct class.
- It would require explicit user opt-in (`anvil bridge enable
  --target wsl:Ubuntu`) and audit logging.

**This is vNext+, with its own ADR.**

---

## 7. Concurrency and correctness model

### 7.1 Multiple clients, one session (v1)

Already supported (INTD-013, fan-out). Unchanged.

### 7.2 Multiple sessions, same worktree (v1.5)

The daemon allows N sessions per worktree, keyed by `AgentTag`. Each
session has its own:

- Heartbeat / TTL.
- Manifest / capability set.
- `claimed_agent_id` (label).

Shared per worktree:

- Fence state.
- Rule set (loaded from `.anvil.yaml`).
- Watcher subtree.

When a fence triggers, **all sessions for that worktree** receive the
fence event simultaneously — there is no "fence Cursor but not
Claude Code". This is intentional: a fence is a worktree-level state.

### 7.3 Concurrent `scan_buffer` requests

The daemon serves `scan_buffer` requests with bounded per-connection
RPS (INTD-016: 100 sustained / 1000 burst). Across drivers, the
enforcement pipeline serialises only as much as the parser pool requires.
Two sessions in the same worktree calling `scan_buffer` in parallel:

- Get independent results (each request carries proposed content; no
  shared mutation of disk state).
- Are not ordered relative to each other (no causal relation between
  the two AI agents' edits).

### 7.4 Watcher behaviour

Per-worktree subtree (INTD-004). Multiple sessions don't multiply
watchers. Removing a session reduces the watcher only if no other
session covers that subtree.

### 7.5 Project state cache

INTD already caches per-worktree resolved enforcement config. Cache key
is `WorktreeKey`. Multi-session doesn't affect this.

### 7.6 Ordering guarantees

- Within a connection: JSON-RPC request → response is ordered (id
  matched).
- Across connections: no ordering guarantee.
- `scan_buffer` results are deterministic for the same `(path, text,
  rules)` triple regardless of which session asked.

### 7.7 Race handling

- Same-worktree fence write: persisted to disk via a transactional
  rename (INTD-007). Two sessions cannot both fence with conflicting
  reasons; the second write coalesces or is rejected (existing
  behaviour).
- Daemon spawn race: §4.8.

### 7.8 Split-brain prevention

- One daemon per `(uid, os_locality_token)`. PID-file exclusive create.
- Cross-boundary detected and refused, so a second daemon on the other
  side of WSL cannot accidentally co-protect.

### 7.9 Locking strategy

Inherited from INTD: the enforcement pipeline owns its synchronisation;
session registry is `Mutex<HashMap>`; fence store is on-disk transactional.

### 7.10 Backpressure

INTD-013 already documents per-driver bounded telemetry channel with
overflow drop. Extended: when a driver is in overflow-drop, its session's
`status.query` reports `telemetry: degraded` and the operator-visible
status line says "1 surface degraded".

### 7.11 Many agents, one project

Yes (v1.5, scenario 7). Bounded by IPC connection cap (default 64,
INTD-016) and per-worktree session cap (proposed default 16,
configurable via `enforcement.session.per_worktree_max: 16`). Beyond the
cap, attach is refused with `session-cap-exceeded`.

---

## 8. Protection-claim policy

This section is the most important v1 deliverable. **Anvil's user-facing
status MUST be one of these states**, with no overlap or ambiguity. If
operators or test frameworks see a string outside this set, that's a
contract bug.

### 8.1 Per-surface status states

| State                        | Meaning                                                                                              |
|------------------------------|------------------------------------------------------------------------------------------------------|
| `unbound`                    | Surface not yet attached; `ensure()` not yet called or failed.                                       |
| `attached`                   | Driver connected, manifest accepted, read-only.                                                      |
| `participating`              | Driver advertised `anvil/enforcement/ack` AND was admitted past the participating gate (DRVR-008).   |
| `embedded-fallback`          | Surface is operating without a daemon link; using in-process validation only. MCP-only.              |
| `degraded`                   | Surface attached but telemetry overflow / partial failure; treats decisions as best-effort.          |
| `cross-boundary-refused`     | Surface and daemon are on different OS instances; attachment refused.                                |
| `quarantined`                | Reliability quarantine (`editor-and-mcp-driver-design.md` §2.6) — read-only with cooldown.           |
| `detached`                   | Was attached, now disconnected; transient.                                                           |

### 8.2 Per-worktree protection states

The `pre-write-*` states are split (F-C1) so a worktree served only by
embedded-fallback MCP shims cannot claim daemon-backed protection.
`degraded` rolls up to a worktree state explicitly (Ops-E).

| State                  | Required conditions                                                                                          | Allowed claim                          |
|------------------------|---------------------------------------------------------------------------------------------------------------|----------------------------------------|
| `unprotected`          | No daemon, no embedded fallback. `ensure()` failed.                                                           | "Anvil is not running for this project." |
| `warming`              | Daemon started in last bounded startup window, no surface attached yet, OR daemon present with `info.ready: false`. | "Anvil is starting; not yet protecting." |
| `pre-write-embedded`   | At least one MCP shim is active **and** every active MCP shim's `validation.backend` is `embedded`. No daemon-backed shim, no editor driver. | "Anvil pre-write protection active (embedded fallback — daemon unreachable) for AI tools." |
| `pre-write-daemon`     | At least one MCP shim is active **and** at least one shim's `validation.backend` is `daemon`. No editor driver attached. | "Anvil pre-write protection (daemon-backed) for AI tools." |
| `save-time-only`       | Editor driver Participating; no MCP attached.                                                                  | "Anvil save-time protection active in editor."             |
| `full`                 | At least one MCP shim daemon-backed AND at least one Participating editor driver, all on `daemon-backed` mode. | "Anvil pre-write + save-time protection active."           |
| `degraded-protection`  | Any state above `pre-write-embedded` where at least one surface is `degraded` (telemetry overflow, partial transport failure). | The previous claim, suffixed with "(one or more surfaces degraded — best-effort enforcement)." |
| `cross-boundary-mixed` | Multiple surfaces detected on different `os_locality_token`s. Reachable only via the `.anvil/runtime` advisory pointer or doctor invocation; **not** auto-detected at attach time (F-A3). | "Anvil detected surfaces on different operating-system boundaries. Protection cannot be claimed across the boundary. Open the surface alongside the daemon, or accept partial coverage on each side." |
| `multi-daemon-detected`| Two `info.json` records observed in the same logical context (rare; e.g. a diagnostic tool reporting both Windows and WSL daemons). | "Anvil sees two daemons (`<token-a>`, `<token-b>`). Each protects its own surfaces only." |
| `path-uncertain`       | Daemon's canonicalisation reported a different path than the surface registered with.                          | "Anvil canonicalised your project path; verify the working directory is correct." |

**`pre-write-embedded` is not a fallback synonym for `pre-write-daemon`.**
Tooling, CI, and the contract test suite (DLIFE-009) MUST treat them as
distinct. A worktree where the daemon is reachable but every shim
happens to be `embedded` (e.g., daemon went `ready: false` mid-session
or the shim couldn't connect after `ensure()`) is `pre-write-embedded`,
not `pre-write-daemon`.

**Reachability of `cross-boundary-mixed` (F-A3).** This state is **not**
auto-detected at attach time — a Windows surface reading a Windows
`info.json` and a WSL surface reading a WSL `info.json` will each
match their own token without ever observing the other. The detection
paths are:

1. The advisory `.anvil/runtime` pointer (§3.1) records the last
   validating `os_locality_token`. A subsequent surface with a
   different token raises an advisory warning, not a refusal.
2. `anvil doctor --explain-boundary` (DLIFE-007) explicitly probes
   the project for both Windows and WSL daemons (cross-OS-aware) and
   produces the `cross-boundary-mixed` verdict.

The auto-attach refusal path (`cross-boundary-detected`) only fires
when a surface tries to read a foreign `info.json`, which is the
**unusual** case (e.g., a surface that follows `.anvil/runtime`'s
hint to a non-local socket). The protection-claim copy in the spec
must not imply auto-detection at attach time for the common case.

### 8.3 What Anvil is NOT allowed to say

- "Anvil is protecting this project" when the only surface is `attached`
  (read-only) and not Participating.
- "Anvil is protecting this project" when the only validation backend is
  embedded **without** explicitly saying "(embedded fallback)".
- "Anvil pre-write protection ON" without naming the surfaces it
  applies to.
- Anything implying cross-OS coverage when `os_locality_token`s differ.
- Anything implying file-watcher coverage of `\\wsl.localhost\` paths.

### 8.4 How surfaces report their backend

The MCP `validate_write` response gains a metadata field:

```jsonc
{
  "decision": "allow|warn|block",
  "diagnostics": [/* canonical Diagnostic[] */],
  "validation": {
    "backend": "daemon" | "embedded",
    "daemon_version": "0.6.0",                  // present when daemon-backed
    "os_locality_token": "linux:8d3f1a2c"       // present when daemon-backed
  }
}
```

Editor drivers carry the equivalent in the `initialized` ack.

**Result-time honesty rule (F-C2).** The `validation.backend` field MUST
be set at **result-generation time** from the actual code path that
produced the diagnostics, not from the connection state at request
dispatch time. Concretely: the MCP shim records which path returned
each result (daemon RPC success vs embedded pipeline) and stamps
`backend` from that record. A daemon crash between `ensure()` and
`scan_buffer()` results in fallback to embedded, and the response
carries `backend: embedded`. DLIFE-009 contract test:
"`when daemon crashes between ensure and scan_buffer, the MCP response
carries backend: embedded`".

This guards against the false-confidence failure mode where the shim
*intends* daemon-backed validation but a transport blip silently
produces an embedded result tagged as daemon-backed.

### 8.5 `anvil status` output

Human-readable form:

```
Anvil 0.6.0
Daemon: linux:8d3f1a2c (pid 17341, up 2h13m)
Mode: full

Surfaces:
  - Cursor MCP (pid 4923)        participating, daemon-backed
  - Claude Code MCP (pid 5001)   participating, daemon-backed
  - Zed editor-driver (pid 4988) attached, daemon-backed

Worktree: /home/u/proj
  Sessions: 3
  Fences:   0
  Last decision: allow (12s ago, secret-detection)
  Latency: p50 4ms p95 18ms (mid-edit)
```

### 8.7 `anvil status --json` (machine-readable, Ops-E)

A stable schema (`anvil.status.v1`) gated by the contract test suite
(DLIFE-009). Tooling (TUI, dashboard, CI assertions) MUST consume this
form, not the rendered text in §8.5.

```jsonc
{
  "schema": "anvil.status.v1",
  "anvil_version": "0.6.0",
  "daemon": {
    "present": true,
    "ready": true,
    "pid": 17341,
    "os_locality_token": "linux:8d3f1a2c",
    "uptime_seconds": 7980,
    "version": "0.6.0",
    "proto_version": 2
  },
  "worktrees": [
    {
      "canonical_path": "/home/u/proj",
      "protection_state": "full",       // §8.2 closed set
      "protection_claim": "Anvil pre-write + save-time protection active.",
      "sessions": [
        {
          "session_id": "sess-9c3f",
          "agent_tag": "default",         // v1.5: composite key
          "claimed_agent_id": "cursor",
          "driver_id": "cursor-mcp",
          "driver_pid": 4923,
          "surface_state": "participating",  // §8.1 closed set
          "validation_backend": "daemon",
          "telemetry_state": "ok"            // ok | degraded
        }
        // ...
      ],
      "fences": [],
      "last_decision": {
        "outcome": "allow",
        "rule_id": "secret-detection",
        "age_seconds": 12
      },
      "latency_mid_edit": { "p50_ms": 4, "p95_ms": 18, "samples": 73 }
    }
  ],
  "warnings": []   // structured warnings (cross-boundary, version-skew, etc.)
}
```

**Schema stability guarantee.** Field additions are minor-version safe;
field removals require a new schema version (`v2`). DLIFE-009 includes
a fixture-comparison test pinning every state in §8.2 to a JSON
snapshot.

### 8.6 `anvil doctor` exit codes

| Exit code | Meaning                                                                                  |
|-----------|------------------------------------------------------------------------------------------|
| 0         | Daemon up, all surfaces healthy, no boundary issues.                                     |
| 1         | Daemon up but at least one surface in `degraded` or `embedded-fallback`.                 |
| 2         | Daemon up, **`cross-boundary-detected`** between surface and daemon, or `cross-boundary-mixed` detected by `--explain-boundary`. |
| 3         | Daemon not running (`unprotected` or `warming` only).                                    |
| 4         | `proto-version-mismatch`: daemon up but version-skewed against this CLI/surface (Ops-D). Hint: run `anvil intercept restart`. |
| 10        | Discovery failed (lstat ladder violation, untrusted runtime dir).                        |

CI / test harnesses fail-fast on exit codes 2, 4, and 10.

**Doctor non-blocking probes (Ops-D, F-DX).** Every probe in `anvil
doctor` (signal-0, starttime check, socket probe, `info.json` read) has
a per-operation wall-clock timeout pinned in DLIFE-007 (default 200 ms
for the socket probe; 50 ms for filesystem reads). Total command
budget is bounded so doctor cannot hang on a dead socket. When a
probe times out, doctor emits the timeout in the report and continues
to the next probe.

**Stale-vs-never-started disambiguation (Ops-B).** Exit 3 is split in
the textual output but not in the exit code:

- `daemon-not-running: never-started` — no `info.json`, no socket, no
  PID file.
- `daemon-not-running: stale-reaped` — `ensure()` reaped a stale
  artefact and the respawn failed; check `spawn_log_path` tail (which
  doctor includes inline).

CI scripts that need to distinguish should consume `--json`
(see §8.7).

---

## 9. v1 / v1.5 / unsupported boundary

### 9.1 v1 (next release after current slate)

Concrete capabilities:

- Per-user daemon (already shipped — INTD complete).
- `info.json` discovery sidecar (new).
- `anvil intercept ensure` lazy launcher (new).
- `os_locality_token` boundary detection + refusal codes (new).
- Protection-claim policy enforced in `anvil status`, MCP response, and
  `anvil doctor` (new).
- Session per worktree (v1) — multi-driver per session (already
  shipped — INTD-013).

Scenarios 1–5, 8, 9, 12, 15, 17–20 must work end-to-end.

### 9.2 v1.5 (release after v1)

Concrete capabilities:

- Multi-session per worktree (`AgentTag` composite key).
- Per-worktree session cap (`enforcement.session.per_worktree_max`).
- Multi-agent enforcement-decision fan-out.

Scenarios 6, 7 unblock.

### 9.3 Explicitly unsupported in v1 and v1.5

- **Cross-Windows ↔ WSL surfaces talking to one daemon** (T-1 + §6).
  Detection + refusal only.
- **Remote SSH editors with daemon on dev box** (scenario 13). Not
  blocked from working manually, but not advertised as protected.
- **Dev container / Codespaces** (scenario 14). Same — works if user
  installs daemon in the container, but no v1 packaging.
- **TCP transport** of any kind.
- **Cross-UID** attachment.

### 9.4 Future (vNext+, ADR required)

- Bridge driver for cross-OS reach (§6.8).
- Per-project worker model (Option D from §1.4).
- Remote/dev-container packaging (scenarios 13, 14, 16).

---

## 10. Council resolution map

This spec was reviewed by the planning council in
`2026-05-07-daemon-sessions-surfaces-boundaries.md` §3 plus a follow-up
review (see §11 below). Mapping of finding → resolution:

| Finding (severity, role)         | Resolution                                                    |
|----------------------------------|---------------------------------------------------------------|
| C-1 (M, security)                | §2.1 + §3.4 — `os_locality_token` definition and refusal code |
| C-2 (M, adversarial)             | §3.4 — `proto-version-mismatch` in refusal-code table         |
| C-3 (S, DX)                      | §8.6 — exit codes 2 / 10                                       |
| C-4 (S, runtime)                 | §5.3 — macOS App Sandbox handling                              |
| C-5 (M, product)                 | §8.2 — `warming` state                                         |
| Q-PROD-1                         | §8.5 — `anvil status` is canonical                             |
| Q-PROD-2 / Q-DX-1                | §4.2 — `ensure()` blocks until daemon up or fixed startup budget elapses |
| Q-PROD-3                         | §8.2 — `warming` state defined                                 |
| Q-ARCH-1                         | §2.2 — path-only key, daemon-canonicalised                     |
| Q-ARCH-2                         | §3.1 + §5 — runtime sidecar dir, per-OS                        |
| Q-ARCH-3                         | §1.5 — keep singleton, reject A; D is vNext+                   |
| Q-SEC-1                          | §3.5 — same-UID attacker is in INTD threat model; no extra     |
| Q-SEC-2                          | §2.1 — distinct `os_locality_token` for each WSL distro        |
| Q-DX-2                           | §5 — per-OS log path                                           |
| Q-DX-3                           | §4.7 — version skew = refuse + suggest restart                 |
| Q-PLAT-1                         | §5.3 — embedded-fallback for App Sandbox in v1                 |
| Q-PLAT-2                         | §5 — supervisors are vNext                                     |
| Q-PLAT-3                         | INTD-002 already                                               |
| Q-ADV-1                          | §2.2 — daemon canonicalisation is authority                    |
| Q-ADV-2                          | RTAI-008 — already pinned                                      |
| Q-ADV-3                          | §3.4 + §8 — refusal code + status state                        |

---

## 11. Follow-up council remediation

A second council review focused specifically on WSL handling, daemon
identity, and protection-claim wording (per the planning brief's "If the
council finds major uncertainty around WSL, daemon identity, or
protection claims, run a follow-up review" instruction). Findings:

1. **F-1 (Major, runtime).** §6.4's refusal of "project on
   `\\wsl.localhost\`" must be reachable from the doctor command on
   *both* sides — a Windows-side surface won't naturally check the WSL
   side. **Resolution:** §3.1's `.anvil/runtime` advisory pointer carries
   the last-known `os_locality_token`; doctor reads it and warns "this
   project last validated by `<other-token>`; you are on `<this-
   token>`". Resolution incorporated into §6.3.
2. **F-2 (Major, security).** The bridge proposal (§6.8) MUST not be
   confused with v1 reality. **Resolution:** §6.8 is labelled
   "vNext+, with its own ADR" and §9.3 explicitly classifies it as
   unsupported.
3. **F-3 (Minor, adversarial).** Two-`ensure` race (§4.8) needs the
   loser's launcher to handle "child-exited-with-already-running" as
   success-after-info.json-reread. **Resolution:** §4.8 documents the
   sequence; pseudocode in §4.2 covers it.
4. **F-4 (Major, product).** Protection-claim states must be testable.
   **Resolution:** §8 enumerates the closed set; §8.6 pins exit codes
   for CI assertions; the DLIFE module (§12) tracks a contract test
   suite.
5. **F-5 (Minor, DX).** `anvil intercept ensure`'s wall-clock budget
   must not be "hardcoded value buried in code". **Resolution:** §4.2
   defers the budget to ADR-031's measurement rubric (the existing
   latency-rubric authority); no wall-clock string in this spec.
6. **F-6 (Minor, architect).** Composite `WorktreeKey` should be
   forward-compatible. **Resolution:** §6.5 documents the additive
   migration shape (extra fields, default-null, ignored by v1
   equality).
7. **F-7 (Minor, security).** App Sandbox path in §5.3 must
   acknowledge the **MCP shim cannot reach the daemon at all** in v1;
   embedded-fallback isn't graceful, it's the only option.
   **Resolution:** §5.3 wording firmed up; §8.2 `pre-write-only` state
   remains the honest claim.

All findings remediated within this draft. No third-round review
required.

---

## 11.B Round-2 council remediation log (post-spec drafting)

A round-2 council review (adversarial, operations, pragmatic) was run
against the v1 draft of this spec. Findings cluster into four buckets:
**identity & boundaries**, **lifecycle robustness**, **observability**,
**scope discipline**. All Major findings are remediated in this
revision; Minor findings have either been incorporated or acknowledged
as deferred. Crosswalk:

| Finding | Severity | Remediation site                                  |
|---------|----------|---------------------------------------------------|
| F-A1: WSL token derivation attacker-controlled | Major | §2.1 hardened (kernel-controlled sources, env-var advisory only) |
| F-A2: `.anvil/runtime` write-unprotected       | Minor | §3.1 wording clarified — file is never an attachment authority and never suppresses a refusal code |
| F-A3: cross-boundary not auto-detected at attach time | Major | §8.2 reachability paragraph + §6.3 noted — only `--explain-boundary` and `.anvil/runtime` advisory raise it for the common case |
| F-B1: identity uniqueness under namespace remap | Major | §2.1 caveat added; §9.3 explicit unsupported list |
| F-B2: macOS user Volumes path divergence       | Minor | §5.2 acknowledges; treats path mismatch as `daemon-not-running` |
| F-C1: `embedded-fallback` not in worktree state machine | Major | §8.2 split into `pre-write-embedded` / `pre-write-daemon` / `degraded-protection` |
| F-C2: `validation.backend` may falsely show daemon | Major | §8.4 result-time honesty rule + DLIFE-009 contract test |
| F-D1: `info.json` written at `listen()` not "ready" | Major | §3.2 two-phase write with `ready` field; `-32099 daemon-not-ready` |
| F-D2: PID-reuse starttime check per-platform   | Minor | §3.2 starttime_source field + table; DLIFE-001 explicit |
| F-E1: env inherited at spawn (LD_PRELOAD etc.) | Major | §4.2 env-clear with allowlist; explicit drops |
| F-F1: temp-sibling rename + symlink squat      | Major | §3.2 nonce in temp filename + `O_EXCL\|O_NOFOLLOW`; DLIFE-001 test |
| F-F2: confirmed-staleness predicate            | Minor | §4.2 shared `confirmed_alive` / `confirmed_stale` predicate |
| F-F3: session cap enforced before v1.5         | Minor | DLIFE-008 acceptance criteria require cap + tests in same PR |
| Ops-A: spawn-failure observability             | Major | §3.2 `spawn_log_path`; §4.2 `Err(StartFailed{spawn_log_tail})`; DLIFE-011 |
| Ops-B: per-platform starttime mechanism        | Major | §3.2 `starttime_source` table; DLIFE-001 explicit |
| Ops-C: log rotation parameters; panic log      | Major | DLIFE-011 owns rotation + panic log + ensure-attempts log |
| Ops-D: version-skew exit code; doctor timeout  | Minor | §8.6 exit code 4 + per-probe timeout pinned in DLIFE-007 |
| Ops-E: `anvil status --json` not specified     | Major | §8.7 + DLIFE-012 |
| Ops-G: daemon binary resolution path           | Minor | §4.2 sibling-resolution table; DLIFE-004 |
| Prag-A: DLIFE-009 must be hard release gate    | Major | DLIFE-009 entry in §12 elevated to hard gate |
| Prag-B: §4.2 vs §4.8 pseudocode reconcile      | Major | §4.2 pseudocode rewritten to include `AlreadyRunningExit` branch |
| Prag-C: nine worktree states acceptable        | n/a   | KEEP — confirmed by adversarial review (C-1 demanded the split) |
| Prag-D: multi-agent UX note                    | Minor | §9.2 kept v1.5 with explicit note: single-session merges status across agents (degraded UX, not error) |
| Prag-E: cross-boundary actionable remediation  | Major | §8.2 entry references `anvil doctor --explain-boundary`; §4.2 `Err(CrossBoundaryDetected)` carries `remediation` hint string |
| Prag-G: log rotation                           | Major | DLIFE-011 |

**Stale-vs-never-started disambiguation (Ops-B follow-up).** Reflected
in §8.6.

**Outstanding deferrals.** DLIFE-010 reclassified to v1.5 (Prag-A
recommendation). DLIFE-005 conceptually first in implementation order
even though listed at position 5 (Prag-A note); module header in
`plans/archive/modules/daemon-lifecycle.aps.md` will document the recommended
landing order: DLIFE-005 → -001 → -002 → -003 → -004 → -011 → -006 →
-012 → -007 → -009. -008 v1.5; -010 v1.5.

No further round of review needed — all Major findings are spec-level
amendments resolved here, with no implementation in flight.

---

## 12. APS module: `daemon-lifecycle` (DLIFE) — proposed work items

A new APS module `plans/archive/modules/daemon-lifecycle.aps.md` is proposed,
owning the runtime work this spec implies. Suggested work items, sized
by scope (file count + test surface, not duration):

### DLIFE-001: `info.json` runtime sidecar

- **Files:** `crates/anvil-intercept/src/info.rs` (new),
  `crates/anvil-intercept-proto/src/info_v1.rs` (new), edits in
  `crates/anvil-intercept/src/ipc.rs` (write on listen, remove on
  shutdown).
- **Tests:** atomic write on bind, atomic replace on restart, removal
  on graceful shutdown, lstat-ladder enforcement on read, schema
  fixture parity (Rust ↔ TS via `anvil-driver-client`).
- **Dependencies:** INTD-002.

### DLIFE-002: `os_locality_token` derivation

- **Files:** `crates/anvil-intercept-proto/src/locality.rs` (new),
  `crates/anvil-intercept/src/locality.rs` (platform-specific reads),
  `packages/anvil-driver-client/src/locality.ts` (TS mirror).
- **Tests:** per-platform fixture (mocked `/etc/machine-id`,
  `IOPlatformUUID`, `MachineGuid`, WSL distro env), determinism,
  hash-prefix length, mismatch detection.
- **Dependencies:** none.

### DLIFE-003: Boundary refusal codes wired into discovery

- **Files:** `packages/anvil-driver-client/src/discovery.ts`,
  `crates/anvil-cli/src/mcp/discovery.rs` (new shared helper),
  edits in `crates/anvil-cli/src/commands/intercept.rs` (status output).
- **Tests:** mismatched-token refuses, stale-pid refuses with the
  right code, version-mismatch refuses.
- **Dependencies:** DLIFE-001, DLIFE-002.

### DLIFE-004: `anvil intercept ensure` lazy launcher

- **Files:** `crates/anvil-cli/src/commands/intercept.rs` (extend),
  `crates/anvil-cli/src/intercept_ensure.rs` (new).
- **Tests:** cold start (no daemon), warm start (idempotent), stale
  reap, two-ensure race winner+loser, version-mismatch refuses with
  guidance, cross-boundary refuses.
- **Dependencies:** DLIFE-001, DLIFE-002, DLIFE-003.

### DLIFE-005: Per-OS runtime path resolution

- **Files:** `crates/anvil-intercept/src/runtime_dir.rs` (new),
  consolidate the existing per-platform path code paths from
  `crates/anvil-intercept/src/ipc.rs` into this module.
- **Tests:** Linux XDG fallback, macOS Application Support path,
  Windows LOCALAPPDATA path, WSL path equivalence to Linux native,
  fallback when env unset.
- **Dependencies:** none (refactor first).

### DLIFE-006: `anvil status` extended output

- **Files:** `crates/anvil-cli/src/commands/intercept.rs` (extend
  status renderer), `crates/anvil-intercept/src/status.rs` (extend
  payload to include surfaces list, mode, OS locality token).
- **Tests:** rendered output matches §8.5 contract; JSON output
  shape stable; `pre-write-only` / `save-time-only` / `full` /
  `cross-boundary-mixed` rendered correctly across fixtures.
- **Dependencies:** DLIFE-001, DLIFE-003.

### DLIFE-007: `anvil doctor`

- **Files:** `crates/anvil-cli/src/commands/doctor.rs` (new).
- **Tests:** exit codes 0/1/2/3/10 reachable; runs in bounded time
  on cold cache; cross-boundary scenario produces exit 2.
- **Dependencies:** DLIFE-006.

### DLIFE-008: Multi-session-per-worktree (`AgentTag`) — **v1.5**

- **Files:** `crates/anvil-intercept/src/registry.rs` (extend
  session key), `crates/anvil-intercept-proto/src/session.rs` (add
  `AgentTag`), `packages/anvil-driver-client/src/session.ts` mirror.
- **Tests:** two sessions same worktree distinguished by AgentTag,
  fence still applies to both, session cap enforced.
- **Dependencies:** DLIFE-001..DLIFE-007.
- **Status target:** v1.5; not blocking v1 release.

### DLIFE-009: Protection-claim contract test suite

- **Files:**
  `crates/anvil-cli/tests/protection_claim_states.rs` (new),
  `apps/e2e/src/protection_claim_states.spec.ts` (new).
- **Tests:** for each state in §8.2 table, drive the system into that
  state and assert the rendered claim matches the contracted string.
- **Dependencies:** DLIFE-006, DLIFE-007.

### DLIFE-010: macOS App Sandbox MCP fallback contract — **v1.5**

- **Files:** `crates/anvil-cli/src/mcp/sandbox_detect.rs` (new),
  edits in `validation.rs`.
- **Tests:** sandbox-isolated detection, fallback emits
  `daemon-locality-untrusted: sandbox-isolated`, MCP response carries
  `validation.backend = embedded`.
- **Dependencies:** DLIFE-003.
- **Status target:** v1.5 — embedded fallback already triggers the
  correct behaviour without explicit detection; this item adds the
  observability surface so doctor / status name it.

### DLIFE-011: Log rotation, panic log, and ensure-attempt log (Ops-C, Prag-G)

- **Files:** `crates/anvil-intercept/src/logging.rs` (new),
  edits in `crates/anvil-intercept/src/main.rs`,
  edits in `crates/anvil-cli/src/intercept_ensure.rs`.
- **Behaviour:**
  - Operational log rotates by size cap (DLIFE-011 pins the cap and
    retained file count) on every write that would exceed the cap;
    rename `intercept.log` → `intercept.log.1` (and bump older files);
    open new `intercept.log`. No mid-write rotation.
  - `intercept-panic.log` is a separate, append-only file written by
    the Rust panic hook; never rotated; truncated only by explicit
    `anvil intercept clear-panic-log`.
  - `ensure-attempts.log` is appended (one structured line per
    `ensure()` invocation: timestamp, outcome code, spawn pid if any,
    stale-reap path if any).
- **Tests:** rotation under size pressure produces N retained files;
  panic survives rotation; ensure-attempt log contains expected codes
  for cold start, warm start, race-loser, version-mismatch.
- **Dependencies:** DLIFE-001, DLIFE-005.

### DLIFE-012: `anvil status --json` schema and parser harness (Ops-E)

- **Files:** `crates/anvil-cli/src/commands/intercept.rs` (extend
  status renderer with `--json`), `crates/anvil-cli/src/status_v1.rs`
  (new schema serde), fixture set under
  `crates/anvil-cli/tests/fixtures/status_v1/`.
- **Tests:** for each protection state in §8.2, assert the JSON output
  matches a pinned fixture; field additions are checked with a
  forward-compat test (extra fields ignored by parser).
- **Dependencies:** DLIFE-006.

### DLIFE-009: Protection-claim contract test suite — **HARD GATE for DLIFE release** (Prag-A)

This entry supersedes the earlier DLIFE-009 listing — same scope,
elevated visibility. Marks the **hard gate** for the DLIFE module: no
DLIFE work item can be marked Complete in `plans/index.aps.md` until
DLIFE-009 is green. Reason: §8 is the public contract A2 protection
claims rest on; shipping any DLIFE item without the contract test
suite would let the claim states drift before the test catches it.

---

## 13. Test surface summary (v1)

- INTD module gains `info.json` write/replace/remove tests.
- DriverClient gains discovery-sequence + refusal-code tests.
- `anvil status` and `anvil doctor` rendering / exit codes contract-pinned.
- Cross-platform CI: Windows (named pipe + `info.json`), Linux (UDS +
  `info.json`), WSL (Linux path inside WSL job — coverage vNext).
- E2E: at least scenarios 1, 5, 8, 9, 18, 20 from the brainstorm covered
  by `apps/e2e`.

## 14. Documentation deliverables

- New top-level user doc: `docs/runbooks/daemon-lifecycle.md` (auto-start,
  status interpretation, doctor exit codes).
- ADR-036 (`plans/decisions/036-daemon-scope-discovery-and-boundaries.md`)
  records the decision.
- `docs/vision/anvil-scope-guard.md` updated with a "Cross-OS boundary"
  paragraph naming the v1 unsupported configurations.
- `intercept-daemon.aps.md` Out-of-Scope section updated to reference
  DLIFE for lifecycle/discovery (was implicit).
- `surface-drivers.aps.md` extended with a `boundary-detection` task
  reference back to DLIFE-003.
