# anvil-intercept — As-Built

| Type     | Authority | Owner     | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                |
| -------- | --------- | --------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| As-built | Derived   | INTD, DSV | Live   | Last reviewed 2026-07-02 (as-built drift sweep: `ALL_ANVIL_METHODS` now 19 methods incl. witness + GCTX, `stop`/`unblock` CLI subcommands shipped, repinned lib.rs/intercept.rs/protocol.rs line refs) against main `d1fded280`; prior delta review 2026-06-10 (DSV save-time validation arc, ADR-070 peer-SID gate, MLP2-071 subscriber surface) against main `a1c41e284`; full review 2026-05-07 against `v0.6.0-beta` |

| Upstream                                                                                                                                                                             | Downstream                                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-intercept`, `crates/anvil-intercept-proto`, `crates/anvil-intercept-rules`, `crates/anvil-intercept-win32`, `crates/anvil-graph-cache`, `crates/anvil-checks`, ADR-015 | MCP shim validation client (RMCP), driver framework clients (DRVR), CLI intercept surface, embedded fallback path |

> **Status:** Live (beta) **Last reviewed:** 2026-07-02 (as-built drift sweep:
> `ALL_ANVIL_METHODS` now 19 methods incl. witness + GCTX, `stop`/`unblock` CLI
> subcommands shipped, repinned lib.rs/intercept.rs/protocol.rs line refs)
> against main `d1fded280`; prior delta review 2026-06-10 (DSV save-time
> validation arc, ADR-070 peer-SID gate, MLP2-071 subscriber surface) against
> main `a1c41e284`; full review 2026-05-07 against `v0.6.0-beta` slate (HEAD
> `8bbe65b9`) **Crate:** `crates/anvil-intercept` (+ `anvil-intercept-proto`,
> `anvil-intercept-rules`, `anvil-intercept-win32`) **Module owner (APS):** INTD
> (`plans/archive/modules/intercept-daemon.aps.md`, 16/16 complete), DSV
> (`plans/modules/daemon-save-time-validation.aps.md`), INTL
> (`plans/archive/modules/intercept-launcher.aps.md`, Complete 9/9) **Used by:**
> `anvil intercept` CLI surface (`crates/anvil-cli/src/commands/intercept.rs`),
> `anvil-cli/src/mcp/validation.rs` (daemon-backed validation client), driver
> framework (proto + auth surface)

## 1. Overview

`anvil-intercept` is the per-user singleton daemon that mediates pre-write
validation of AI-driven file changes and — since the DSV arc — save-time
validation of just-written change sets. It runs as a foreground process for the
v1 cut, holds the trust boundary for owner-only IPC and the AD-7
fence-on-failure invariant, and is the authoritative producer for the
control-lane session graph (sessions, worktrees, fences, attribution).

Concretely the daemon:

- Owns the in-memory session registry (`crates/anvil-intercept/src/registry.rs`)
  — single session per worktree, 30 s heartbeat TTL, evicted from a 250 ms tick
  (`lib.rs:1955-1970`).
- Speaks NDJSON-framed JSON-RPC over a Unix domain socket (Linux/macOS) or a
  Windows named pipe (`crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-intercept-win32/src/lib.rs`).
- Persists fence state to disk so an interrupted enforcement decision survives
  daemon crash, machine reboot, or a Ctrl-C / SIGTERM-driven shutdown followed
  by a fresh `start --foreground` (`crates/anvil-intercept/src/fence.rs`).
- Runs the Unix SIGINT → SIGTERM → SIGKILL ladder (or Windows Job Object
  termination) with a PID-reuse defence, falling back to a fence on any
  uncertainty (`crates/anvil-intercept/src/interrupt.rs`).
- Exposes the `scan_buffer` JSON-RPC method that the MCP shim's
  `LocalDaemonValidationClient` calls in daemon-backed mode
  (`crates/anvil-intercept/src/midedit.rs`,
  `crates/anvil-cli/src/mcp/validation.rs`).
- Serves the save-time `validate_paths` verdict verb: certifies a client's
  change set against the resident warm graph cache via the GV2 hot-read index
  (`HotReadApi::certify`, `gv2-hotindex-v1`), folding each path into a
  per-worktree workspace-assurance state
  (`crates/anvil-intercept/src/validate_paths.rs`,
  `crates/anvil-intercept/src/save_time.rs`). The wire is frozen across DSV
  sub-phases (ADR-061); only the cache backing swaps underneath it. See §4a.
- Provides an in-process `embedded_evaluate` API that produces the same
  `anvil.diagnostic.v1` envelope the daemon-backed path emits
  (`crates/anvil-intercept/src/embedded.rs`).

The trust boundary is **same-UID, local-IPC**. There is no remote surface, no
cross-UID surface, no TLS, no signed manifests in v1; see §5 below and
`docs/archive/runbooks/v0.6.0-beta-security-note.md` for the four HIGH
trade-offs the release council surfaced inside that boundary.

## 2. Architecture

```text
                 ┌──────────────────────────────┐
                 │       anvil intercept         │
                 │  CLI commands (start, status) │
                 │  crates/anvil-cli/src/        │
                 │  commands/intercept.rs         │
                 └───┬───────────────┬───────────┘
                     │ Unix UDS      │ Windows named pipe
                     │ intercept.sock│ \\.\pipe\anvil-intercept-<sid>
                     │               │
                     ▼               ▼
        ┌──────────────────────────────────────────┐
        │         anvil-intercept (daemon)          │
        │  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
        │  │   IPC    │  │ Registry │  │ Watcher │ │
        │  │ listener │◄─│ (in-mem) │◄─│ (kernel│ │
        │  │ (NDJSON) │  │          │  │ events)│ │
        │  └────┬─────┘  └────┬─────┘  └────┬────┘ │
        │       │             │             │       │
        │  ┌────▼─────────────▼─────────────▼────┐ │
        │  │     Enforcement pipeline +          │ │
        │  │     Interrupt ladder (AD-7)         │ │
        │  └────┬───────────────────┬────────────┘ │
        │       │                   │              │
        │  ┌────▼──────┐       ┌────▼───────┐     │
        │  │  Fence    │       │ Telemetry +│     │
        │  │  store    │       │  fanout    │     │
        │  │  (on-disk)│       │  (filter)  │     │
        │  └───────────┘       └────────────┘     │
        └──────────────────────────────────────────┘
                     ▲
                     │ peer-cred boundary
                     │ (SO_PEERCRED / getpeereid / pipe owner)
                     │
        ┌────────────┴─────────────────────────────┐
        │  Same-UID local clients only:             │
        │   - anvil intercept CLI                   │
        │   - MCP shim's LocalDaemonValidationClient│
        │     (crates/anvil-cli/src/mcp/validation.rs)│
        │   - future driver clients (DRVR-001)      │
        └───────────────────────────────────────────┘

        Embedded path (no daemon):
        crates/anvil-intercept/src/embedded.rs
        ↳ same EnforcementPipeline, same diagnostic envelope, no IPC.
```

## 3. Process model

The daemon is a **per-user singleton** enforced by an exclusive PID file. There
is one supported launch shape in v1 — foreground.

**PID file location** (resolved by `default_pid_file_path` at
`crates/anvil-intercept/src/lib.rs:699-742`; `ANVIL_HOME` re-roots the file
under its prefix per DISTRIB-006 / ADR-060, else the ordering below):

1. `$XDG_RUNTIME_DIR/anvil/intercept.pid` if `XDG_RUNTIME_DIR` is set.
2. `%LOCALAPPDATA%\anvil\intercept.pid` on Windows.
3. `$HOME/.local/state/anvil/intercept.pid` (Unix fallback).

The same path is used by both foreground and (future) backgrounded launch, so a
second instance refuses to start while the first is alive. The PID guard records
`process::id()` plus a `start_time=` line so a stale PID file from a crashed
daemon can be proven stale before being recovered (`lib.rs:925-1000`). Linux
reads `/proc/PID/stat` field 22; macOS reads process creation time via
`anvil-intercept-macos::process_start_time`; Windows reads `GetProcessTimes` via
`anvil-intercept-win32::process_creation_time`; other platforms return `None`
and the recovery path falls through to a liveness probe (`lib.rs:1174-1195`).
PID-file directory is checked at mode `0700` owned by the current UID and
refuses symlink parents (`lib.rs:869-930`).

**Foreground only.** `anvil intercept start` requires `--foreground` and bails
otherwise:

```
crates/anvil-cli/src/commands/intercept.rs:1191-1196
    if !args.foreground {
        anyhow::bail!(
            "`anvil intercept start` requires --foreground; this is the \
             low-level operator/debugging daemon surface. Backgrounded daemon \
             launch is provided to `anvil start` / `anvil watch` via the \
             daemon-lifecycle ensure primitive (DLIFE, ADR-082)."
        );
    }
```

`anvil intercept start` remains the low-level operator/debugging surface;
backgrounded launch is now delivered through `anvil start` / `anvil watch` (the
DLIFE daemon-lifecycle ensure primitive, ADR-082), not by daemonising this
command. The runbook (`docs/archive/runbooks/v0.6.0-beta-release-runbook.md` §1)
records this as the only launch mode the release council validated for
`v0.6.0-beta`. Operators running under systemd / launchd run the binary in
foreground mode under the manager's supervision rather than
double-backgrounding.

**Shutdown.** `wait_for_shutdown_signal` (`lib.rs:1374-1400`) races SIGINT and
(on Unix) SIGTERM. Windows hooks Ctrl+C only; Job Object termination is the
process-manager analogue and is INTD-006's territory. Shutdown is cooperative
via a `tokio::sync::watch` channel (`Shutdown` / `ShutdownToken`,
`lib.rs:1282-1360`); in-flight IPC handlers drain with a 250 ms deadline
(`ipc.rs:72`, `SHUTDOWN_DRAIN_DEADLINE`).

## 4. IPC surface

### 4.1 Transport

- **Unix:** Unix domain socket. Path is `<socket_dir>/intercept.sock` where
  `<socket_dir>` resolves to `$XDG_RUNTIME_DIR/anvil` if set, else
  `$HOME/.local/state/anvil` (`ipc.rs:189-216`). Never `/tmp`.
- **Windows:** named pipe at `\\.\pipe\anvil-intercept-<current-user-sid>`. The
  SID — not the env username — is the suffix so account-name spoofing cannot
  move the rendezvous point (`crates/anvil-intercept-win32/src/lib.rs:96-99`).

The listener side ships in `anvil-intercept-win32`'s
`create_owner_only_pipe_server` (`anvil-intercept-win32/src/lib.rs:50-63`),
which builds the pipe with an explicit owner-only DACL granting `0x12019f`
(deliberately less than `GENERIC_ALL`) and sets `reject_remote_clients(true)`.
The synchronous client side ships from the same crate
(`connect_owner_only_pipe_client`, `lib.rs:121`) — it is what the CLI's Windows
status path calls (`crates/anvil-cli/src/commands/intercept.rs:143-148`).

### 4.2 Owner-only socket permissions (Unix)

`IpcListener::bind` runs the owner-only ladder:

- Socket directory must be a real directory at mode `0700` owned by the daemon
  UID; symlinks rejected (`ipc.rs:372-416`).
- Socket file is `0600` owned by the daemon UID; the listener `fchmod`s the file
  after `bind()` to close the umask race (`ipc.rs:418-434`, bind ladder around
  `ipc.rs:520-602`).
- `validate_socket_path_for_client` (`ipc.rs:218-245`) is the same check from
  the client side, used by the CLI status path (`intercept.rs:89-105`).

### 4.3 Wire protocol

Owned by `anvil-intercept-proto`. Each line on the wire is one `IpcEnvelope`
(`anvil-intercept-proto/src/lib.rs:98-130`). The envelope flattens an
`IpcCommand` payload (`lib.rs:61-86`) — `RegisterSession`, `Heartbeat`,
`UnregisterSession`, `ListSessions`, `QueryStatus` — plus an optional
JSON-RPC-style `id` for request/response correlation.

JSON-RPC method names and the capability lattice live in
`anvil-intercept-proto/src/protocol.rs`. **Nineteen** `anvil/`-namespaced
methods are defined (`ALL_ANVIL_METHODS`, `protocol.rs:297-317`): the original
six driver methods, the three DSV save-time verbs, the witness-append verb, and
the nine GCTX read-only graph-context verbs.

- `anvil/publishDiagnostics` (`ANVIL_PUBLISH_DIAGNOSTICS`, `protocol.rs:117`) —
  server → client diagnostic notification.
- `anvil/scan_buffer` (`ANVIL_SCAN_BUFFER`, `protocol.rs:125`) — client → server
  mid-edit buffer scan. The legacy bare `scan_buffer` method is dual-routed.
- `anvil/enforcement/ack` (`ANVIL_ENFORCEMENT_ACK`, `protocol.rs:132`) — client
  → server enforcement ack. DRVR-008's load-bearing method: drivers that omit it
  from their manifest are capped at `Capability::Attached`.
- `anvil/gate/request` (`ANVIL_GATE_REQUEST`, `protocol.rs:138`).
- `anvil/suppression/apply` (`ANVIL_SUPPRESSION_APPLY`, `protocol.rs:145`).
- `anvil/status/query` (`ANVIL_STATUS_QUERY`, `protocol.rs:150`).
- `anvil/validate_paths` (`ANVIL_VALIDATE_PATHS`, `protocol.rs:159`) — client →
  server, the save-time verdict verb (ADR-061 / DSV-002); certifies a change set
  against the warm graph cache. The wire is frozen across DSV sub-phases — only
  the cache backing swaps. See §4a.
- `anvil/workspace_status` (`ANVIL_WORKSPACE_STATUS`, `protocol.rs:164`) —
  client → server, read-only `WorkspaceAssurance` snapshot without submitting a
  change set.
- `anvil/request_full_scan` (`ANVIL_REQUEST_FULL_SCAN`, `protocol.rs:170`) —
  client → server, drive a full scan that warms the graph cache and rebuilds the
  baseline. Since DSV-045 (ADR-085) the daemon's full-scan executor dequeues
  this and drives `Pending → Running → Clean` (or `Bounded` when the worktree
  exceeds the post-`.gitignore` walk cap); the daemon also auto-warms from cold
  on first contact (`validate_paths` / `workspace_status` /
  `request_full_scan`), so a fresh session reaches a useful graph without a
  manual save. Repeated calls coalesce to one scan.
- `anvil/witness/append` (`ANVIL_WITNESS_APPEND`, `protocol.rs:179`) — client →
  server, append a witness line to a worktree's chain through the daemon so a
  single writer owns the chain across worktrees/sessions (MLP2-005). The daemon
  derives `(seq, prev_line_hash)` and appends atomically; the hook falls back to
  an embedded append when the daemon is unreachable.
- **GCTX read-only graph-context verbs (GCTX-010..030, ADR-084).** Nine
  identity-only projections the daemon answers with sealed egress DTOs — it
  performs the projection itself, so the MCP consumer never holds a graph. Each
  dispatches on its own read-only `GctxDispatch` arm, never the save-time
  `validate_paths` path: `anvil/gctx/search_symbols` (`protocol.rs:186`),
  `anvil/gctx/find_dependents` (`protocol.rs:194`), `anvil/gctx/find_callers`
  (`protocol.rs:201`), `anvil/gctx/get_snippet` (`protocol.rs:210`),
  `anvil/gctx/symbol_context` (`protocol.rs:218`), `anvil/gctx/impact_of_change`
  (`protocol.rs:226`), `anvil/gctx/affected_tests` (`protocol.rs:235`),
  `anvil/gctx/graph_stats` (`protocol.rs:242`), and `anvil/gctx/graph_edges`
  (`protocol.rs:249`). Six of these back the MCP shim's GCTX tools (see
  `mcp-shim-as-built.md` §4); `get_snippet` and the two `graph_*` verbs back the
  `graph://` MCP resources.

The capability lattice is `Attached < Participating` (`protocol.rs:265-292`); v1
only ever downgrades, never promotes implicitly.

### 4.4 NDJSON framing and DoS budgets

`MAX_LINE_BYTES` is sized at `(CONTENT_SIZE_CAP_BYTES_USIZE * 6) + 64 KiB` so a
1 MiB `scan_buffer` content payload survives worst-case JSON string encoding
(`ipc.rs:60`). Lines larger than the cap tear the connection down with
`IpcError::OversizedLine`. Frame-size enforcement runs **before parsing** — the
listener never feeds an oversized blob to `serde_json` because deeply nested
JSON is itself an attack surface (`dos.rs:41-50`).

DoS budgets are owned by `crates/anvil-intercept/src/dos.rs` (INTD-016) and
sourced from the resolved enforcement config
(`crates/anvil-intercept/src/config.rs::Resolved::ipc_limits`). Pinned defaults
(`dos.rs:75-85`):

| Limit                        | Default   |
| ---------------------------- | --------- |
| `max_concurrent_connections` | 64        |
| `rps_sustained`              | 100 req/s |
| `rps_burst`                  | 1000 req  |
| `handshake_timeout`          | 5 s       |
| `idle_timeout`               | 60 s      |
| `control_frame_max_bytes`    | 64 KiB    |

Project + user `enforcement.dos.*` blocks merge **stricter-wins**: the smaller
connection cap, smaller RPS, smaller timeouts, smaller frame cap each win
(`config.rs:390-423`). The clamp invariant is at `dos.rs:127-147` —
`max_connections = 0` is clamped to 1 so the operator can always recover.

**RPS exhaustion does not close the connection.** When a peer's bucket is empty,
the listener returns `-32005 Server busy: rate limit exceeded` and lets the
connection continue (`dos.rs:31-39`). Killing on rate-limit would cause innocent
retries to escalate against the connection cap.

### 4.5 `intercept status` cross-platform shape

The CLI's `query_daemon_status` has Unix and Windows branches
(`intercept.rs:77-148`). Both speak the same wire shape, and `--json` returns
the same `DaemonStatusV1` on either OS. The Unix arm connects to the UDS path
resolved by `validate_socket_path_for_client`. The Windows arm connects to the
pipe resolved by `anvil_intercept::ipc::resolve_pipe_name` (install-root aware
since CIB-106; see §15) via `connect_owner_only_pipe_client` and runs through
`query_daemon_status_windows_at` (`intercept.rs:143-148`, `:170+`). The
hard-fail-on-Windows error message that earlier drafts of the runbook quoted is
no longer in the code. The remaining Windows gap is **MCP-side only**
(`correlation.daemonStatus` always `not-wired`); see §12 and §16 gap 9 for the
framing.

## 4a. Save-time validation (`validate_paths`)

> Numbered `4a` rather than renumbering §5–§18: the section postdates the
> 2026-05-07 full review (DSV arc) and the doc's `§N` cross-references are
> load-bearing.

The save-time arc (DSV, `plans/modules/daemon-save-time-validation.aps.md`) adds
a second validation surface beside the pre-write `scan_buffer` path: the daemon
certifies a **just-written** change set against its resident warm graph cache
and answers with a verdict-shaped `ValidatePathsResponse`. `anvil watch` is the
consuming client; the MCP pre-write path stays on `scan_buffer` (§12).

### 4a.1 Verdict core

`validate_paths` (`crates/anvil-intercept/src/validate_paths.rs:297-396`) is a
pure function of the request, the warm cache, and the assurance machine:

- Coalesces the change set last-writer-per-path (`validate_paths.rs:318-325`).
- Reads each content-bearing path under the guarded anchor and computes the
  daemon's **own** content hash — the client's `content_hash` hint is never used
  (`validate_paths.rs:195-207`; pinned by
  `tests::evaluated_echoes_daemon_computed_hash_not_client_hint`,
  `validate_paths.rs:736-779`).
- Certifies a `ContentModify` against the warm cache via the GV2 hot-read index
  — `HotReadApi::certify` over `KernelGraphCache::with_graphs`
  (`validate_paths.rs:501-507`).
- Runs the antipattern family over the guarded bytes on an injected rayon pool
  (`run_antipattern_check_bytes`, `validate_paths.rs:348-353`).
- Folds graph-certifiability into the `AssuranceMachine`
  (`validate_paths.rs:381`).

`coverage = Certified` iff every path is graph-certifiable self-contained AND
the antipattern scan passed (`validate_paths.rs:383-387`). `check_families` is
frozen as `[antipattern]` — `certified` is **never** an unscoped
structural-safety claim (`protocol.rs:315-318`).

### 4a.2 Symbol feed (ADR-064 — the daemon never parses)

Symbols come from an injected `SymbolParser` (the tree-sitter impl lives in
`anvil-cli`) called with the **exact** guarded bytes the daemon read and hashed
(`save_time.rs:26-34`; `lib.rs:428-435`, `with_symbol_parser`). With no parser
injected every verdict is a safe `Partial(CrossFileResolutionNeeded)` — the
daemon warns at startup so the degraded mode is observable (`lib.rs:1151`).

### 4a.3 Read-safety anchor (DSV-003/ADR-061 §5; DSV-010/ADR-068)

Unix reads go through `openat2` with `RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH`
(`path_safety.rs:187-193`); Windows uses a directory-handle + `OBJ_DONT_REPARSE`
ladder (`save_time.rs` module doc, lines 36-40). A refused root never reaches
the filesystem and an admitted root cannot be retargeted after admission
(`workspace_admission.rs`, security C2/C3).

### 4a.4 Parse-size DoS cap (DSV-006)

Files past `caps.max_parse_bytes` are skipped **before** parse/scan/hash with a
coverage diagnostic, not a finding (`validate_paths.rs:220-242`, `:424-456`) —
the save still proceeds; the path simply cannot certify.

### 4a.5 Diagnostic parity (DSV-009/ADR-061 §8)

Every surface orders findings by `(path, rule_id, span, summary)` before the
envelope is built, so daemon and fallback emit byte-identical envelopes
(`sort_diagnostics`, `validate_paths.rs:172-193`).

### 4a.6 `ANVIL_WATCH_DAEMON` routing (DSV-021)

Client-side posture lives in
`crates/anvil-cli/src/commands/watch_save_time.rs:95-114`: unset =
default-on-when-live (route only after an initial status probe finds a live
daemon serving the save-time verbs); `0`/`false`/`off`/`no` = opt-out;
`1`/`true`/`on`/`yes` = forced. There is no auto-start; daemon-absent folds to a
scoped check reporting `unavailable{daemon-absent}`.

## 5. Authentication and trust boundary

The daemon enforces a same-UID, local-IPC trust boundary. There are three
checks:

- **Linux:** `SO_PEERCRED` via `getsockopt(stream, PeerCredentials)`. Rejects if
  `peer_uid != current_uid` with `IpcError::SocketPeerPermissions`
  (`ipc.rs:251-268`).
- **macOS:** `getpeereid(2)` via `nix::unistd::getpeereid`. Same reject
  semantics — and parity with Linux landed in #1331 / issue #1327
  (`ipc.rs:278-295`).
- **Windows:** owner-only DACL on the named pipe + `reject_remote_clients(true)`
  rejects cross-host and cross-SID access in the kernel before the daemon ever
  sees the connection (`anvil-intercept-win32/src/lib.rs:50-63`).
- **Windows, defence in depth (DSV-010b/ADR-070):** beyond the kernel DACL, the
  accept loop runs an explicit peer-SID compare per connection —
  `GetNamedPipeClientProcessId` → client token user SID vs the daemon's own SID
  (`named_pipe_client_is_owner`, `anvil-intercept-win32/src/lib.rs:136-141`;
  accept-loop gate `crates/anvil-intercept/src/ipc.rs:1100-1148`). Fails closed
  on a non-owner SID, a validation error, or a task-join failure; runs on a
  blocking thread so a slow same-UID peer cannot stall the reactor. The
  **client** side still intentionally skips owner validation in v1
  (`anvil-intercept-win32/src/lib.rs:206-210`) — the daemon-side DACL + SID gate
  is the trust model.

Above that same-UID floor sit three further layers, all daemon-side:

- **Driver allowlist** (DRVR-007, `crates/anvil-intercept/src/auth.rs`).
  `is_driver_allowed` (`auth.rs:227-267`) checks a driver binary's canonicalised
  path against a newline-delimited allowlist file (default:
  `~/.config/anvil/drivers.allow`). Missing allowlist closes the gate
  (`Ok(false)`), unreadable allowlist surfaces an error.
- **Manifest-driven capability lattice** (DRVR-008, `auth.rs:520-580`).
  `negotiate_capability` is a pure function of `(requested, manifest)`: a driver
  requesting `Participating` without `anvil/enforcement/ack` in its advertised
  methods is downgraded to `Attached` with a structured `CapabilityDowngrade`
  event. `.anvil.yaml` cannot override this — the manifest is the floor.
- **Workspace confinement / admission** (DSV-008, ADR-061 §7,
  `crates/anvil-intercept/src/confinement.rs` + `workspace_admission.rs`). Each
  save-time verb authorises its `workspace_root` against a per-connection
  `AdmittedRoots` set; the default `Open` mode admits on first contact,
  `Allowlist` confines to operator-listed roots (`confinement.rs:1-30`). The
  confinement config is read owner-only from the daemon's own home prefix, never
  from a repo `.anvil.yaml` — a checked-in file cannot widen the boundary
  (`confinement.rs:11-14`); Windows verifies operator-config ownership at read
  time (`read_trusted_config`, `anvil-intercept-win32/src/lib.rs:806`).

Telemetry identity is daemon-minted, not driver-claimed.
`correlation.originating_driver_id` is computed from peer credentials, never
from a driver-supplied `driverName` (`fanout.rs:24-37`, `telemetry.rs:38-44`). A
same-UID peer setting `"driverName": "vscode"` cannot impersonate the real
VSCode driver in fan-out decisions.

What v1 does **not** enforce: no remote surface, no TLS, no signed manifests.
See `docs/archive/runbooks/v0.6.0-beta-security-note.md` for the framing.

## 6. Fence-on-failure invariant (AD-7)

AD-7 (`crates/anvil-intercept/src/interrupt.rs:26-43`) makes one rule absolute:
**any signal-delivery failure ends in a fence, immediately**. `run_unix_ladder`
(`interrupt.rs:202-284`) returns `InterruptOutcome::FenceImmediately` whenever:

- the leader has no PID on the session record (`LeaderPidUnknown`),
- the PID-reuse defence rejects the start-time match (`PidReuseMismatch`),
- any signal call returns an error other than `ESRCH`
  (`SignalDeliveryFailed { stage, error }`),
- the Windows `TerminateJobObject` returns non-zero
  (`JobObjectTerminationFailed`).

The PID-reuse defence (`interrupt.rs::SystemInterruptOps::verify_leader`,
`interrupt.rs:321-358`) reads the OS-reported start time and compares against
`record.started_at_unix`. If the times disagree, the daemon refuses to signal
and surfaces `PidReuseMismatch` to the caller.

`current_process_start_time` (`interrupt.rs:407-432`) is implemented for Linux
only — it reads `/proc/PID/stat` field 22 (`interrupt.rs:408-418`). On **macOS**
the helper unconditionally returns `None` (`interrupt.rs:419-431`), which
`verify_leader` treats as the conservative "start time unreadable" path: surface
`PidReuseMismatch`. Combined with AD-7 this means **the macOS interrupt ladder
is fence-first** for any session with a non-zero recorded start time. The fence
is the safety primitive, not the signal. See gap 3 in §16 and security note H4.

The daemon's enforcement pipeline (`crate::enforcement`) reads the
`InterruptOutcome` and applies the fence; this module does not own fence state.

## 7. Fence persistence

Fence state is owned by `crates/anvil-intercept/src/fence.rs` (INTD-005 +
INTD-007). Once the daemon decides to fence a worktree, the decision is written
to disk before any IPC response goes back to the caller; the daemon re-reads the
fence file on startup before binding the IPC listener (`lib.rs:1560-1575`).

**Default path** (`fence.rs:398-423`):

- `%LOCALAPPDATA%\anvil\intercept-fences.json` on Windows.
- `$XDG_STATE_HOME/anvil/intercept-fences.json` if set.
- `$HOME/.local/state/anvil/intercept-fences.json` (Unix fallback).

The store file is JSON v1 (`fence.rs:15`) containing absolute, deduplicated
worktree paths plus optional aliases (the original pre-canonical path so a
deleted worktree can still be queried by its original input). Writes go through
a tmp-and-rename ladder with `fsync` + parent dir sync on Unix
(`fence.rs:207-233`); Windows uses a `.bak` backup that recovery reads on load
(`fence.rs:317-380`). Store-parent directory is checked for `0700` mode

- correct owner and refuses symlinks (`fence.rs:480-518`); the file itself is
  written `0600` on Unix (`fence.rs:306-315`).

**Restart does NOT release fences.** This is the most common operator
expectation gap — the runbook
(`docs/archive/runbooks/v0.6.0-beta-release-runbook.md` §3) records that
operators reaching for "restart the daemon" hit this design. Fence state
survives Ctrl-C / SIGTERM shutdown followed by a fresh `start --foreground`,
daemon crash, machine reboot, and any combination thereof
(`fence.rs::tests::fenced_worktree_survives_store_reload`, `fence.rs:557-570`).
The fence is meant to outlive ungraceful daemon shutdown so an interrupted
enforcement decision is not silently undone.

Fences are checked on `RegisterSession` via the wrapping `RegistryDispatcher`
(`lib.rs:125-260`): a session attempting to register against a fenced worktree
fails with `RegistryError::WorktreeFenced` and the registry never holds a record
for it.

## 8. Recovery (`unblock`)

The data path is owned by `FenceStore::unblock_worktree` (`fence.rs:187-205`):
it canonicalises (or accepts the absolute path verbatim for deleted worktrees),
removes the record, persists the result, and returns the `FenceRecord` that was
removed (or `None` if no fence existed).

The **CLI front-end for `unblock` is shipped** (RCLI3-017b / MLP2-026).
`crates/anvil-cli/src/commands/intercept.rs:41-66` declares the `Unblock`
subcommand (alongside `Start`, `Status`, and `Stop`), which wires
`FenceStore::unblock_worktree` to clap in three modes:

- `anvil intercept unblock --worktree <PATH>` — per-fence clear of a single
  worktree (`intercept.rs:276-316`). Idempotent — re-running on an unfenced
  worktree exits zero with an informational note.
- `anvil intercept unblock --all` — clear every fenced worktree, implemented
  client-side as one `unblock-worktree` dispatch per fence
  (`intercept.rs:318-348`).
- `anvil intercept unblock <WORKTREE> --acknowledge-cascade` — the legacy
  positional cascade-clear for a `degraded:fence-cascade` engaged state
  (`intercept.rs:253-274`). On Windows the per-fence and cascade dispatches are
  not yet supported (MLP2-028 peer-credential work) and bail with a clear
  message.

Both non-cascade modes honour `--dry-run` (preview without mutating daemon
state). The operator-facing shape is recorded in
`plans/specs/2026-04-26-rtai-demo-runbook.md` §3.1. See §16 gap 1 (resolved).

The **hard reset** path documented in
`plans/specs/2026-04-26-rtai-demo-runbook.md` §3.2 remains available as the
blunt fallback: stop the foreground daemon (Ctrl-C in its terminal,
`anvil intercept stop`, or SIGTERM by PID), then
`rm -rf ${XDG_DATA_HOME:-$HOME/.local/share}/anvil` (or `%LOCALAPPDATA%\anvil`
on Windows), then re-launch. That destroys **all** fence state for the user;
prefer the worktree-scoped `unblock` above for targeted recovery.

## 9. Interrupt ladder

**Unix** (`interrupt.rs::run_unix_ladder`, lines 202-284):

| Stage | Signal  | Polling cadence   | Total budget                                      |
| ----- | ------- | ----------------- | ------------------------------------------------- |
| 1     | SIGINT  | 10 ms / 500 ms    | `sigint_to_sigterm = 500 ms` (`interrupt.rs:136`) |
| 2     | SIGTERM | 50 ms / 1 s       | `sigterm_to_sigkill = 1 s` (`interrupt.rs:137`)   |
| 3     | SIGKILL | always-final stop | n/a                                               |

Each stage signals the leader PID first (ESRCH there is treated as success — the
process exited before we got to it), then `killpg` against the registered
process group (ESRCH there is also acceptable). The cadence is lifted from
`endevco/pitchfork@cea18d7`'s `src/procs.rs::kill` (MIT, see
`ACKNOWLEDGEMENTS.md`); the **PID-reuse defence is original** — pitchfork relies
on its supervisor never reusing PIDs between launch and signal, an assumption
the intercept daemon cannot make.

**Windows** (`interrupt.rs::windows_impl::run_windows_termination`, lines
453-491): `TerminateJobObject` stops every process in the job atomically, so
there is no SIGINT/SIGTERM analogue. The PID-reuse defence still runs first
(creation-time match against `record.started_at_unix` via
`anvil-intercept-win32::process_creation_time`); a mismatch fences without
terminating.

**macOS:** as noted in §6, the missing `proc_pidinfo`-backed
`current_process_start_time` branch forces `verify_leader` into the conservative
reject path on every session with a recorded start time, so the practical
interrupt outcome is fence-first.

All `unsafe` for the actual signal / job calls is quarantined in `nix` (Unix)
and `anvil-intercept-win32` (Windows); the `anvil-intercept` crate keeps
`#![forbid(unsafe_code)]` (`lib.rs:29`).

## 10. Registry

`SessionRegistry` (`crates/anvil-intercept/src/registry.rs`) is the single
authority on which sessions are active and which worktree each owns. The
registry is **deliberately synchronous** — the daemon's `run_foreground` loop
owns scheduling and ticks `evict_stale` from the 250 ms interval
(`lib.rs:1955-1970`). Spawning a background eviction task here would couple the
registry to a runtime; the council pinned this layer as a synchronous data
structure (`registry.rs:1-13`).

**Per-session record** (`SessionRecord` from `anvil-intercept-proto`):

- `id: SessionId` — opaque string minted by the launcher.
- `worktree: PathBuf` — canonicalised worktree path; the authority key for
  "single session per worktree" (registered at `registry.rs:234-...`).
- `pid: Option<u32>`, `pgid: Option<i32>` — populated by `update_process_info`
  (`registry.rs:276`); arrive after registration.
- `started_at_unix: u64` — registration time, used as the registry's expected
  start time for the PID-reuse defence.
- `last_heartbeat_unix: u64` — refreshed by `heartbeat` (`registry.rs:303`).
- `status: SessionStatus` — `Active` / `Stale`.

**Concurrency:** a `Mutex<Inner>` protects two indexes (id → entry, worktree →
id) so duplicate registration and double-registration of the same worktree are
both rejected at constant time. Lock-poisoning is recovered with
`repair_after_poison` (`registry.rs:472`).

**Eviction** (`evict_stale`, `registry.rs:441`): walks the index, removes any
entry whose last heartbeat is older than the TTL (`DEFAULT_HEARTBEAT_TTL`, 30 s,
`registry.rs:26`), and returns the evicted ids for the caller to log.

**Attribution** (`attribute_path`, `registry.rs:350`) takes a watched file
change and returns `Attribution::Owned(SessionRecord)` if the change falls under
a registered worktree, or `Attribution::Unknown` otherwise. The unattributed
path goes to `crate::unregistered::UnregisteredHandler` (INTD-010, §14 in
`unregistered.rs`), which fences regardless of the operator's
`on_ambiguous_ownership` config — AD-3 hard-cap.

## 11. Telemetry and DoS budgets

Telemetry envelopes (`anvil.notification.v1`,
`crates/anvil-intercept/src/telemetry.rs:15`) carry a `TelemetryCorrelation`
with two daemon-minted scoping fields:

- `originating_session_id` — the **load-bearing scoping key** the fanout reads
  for cross-session redaction (`telemetry.rs:27-37`).
- `originating_driver_id` — daemon-minted from peer credentials
  (`telemetry.rs:38-44`).

The fan-out (`crates/anvil-intercept/src/fanout.rs`, INTD-015) is a deny-by-
default per-event filter (`fanout.rs:1-100`). For each `(envelope, subscriber)`
it returns one of three `Delivery` outcomes:

- `Allow` — subscriber owns the originating session; full envelope.
- `Redact` — cross-session subscription is enabled; subscriber sees
  `{ rule_id, hash_of_path }` only.
- `Deny` — cross-session subscription is disabled (default); subscriber sees
  nothing.

The cross-session policy is sourced from
`Resolved::telemetry_allow_cross_session` (`config.rs:233`), which defaults to
`false`. Project + user merge is stricter-wins: any side requesting `false` wins
(`config.rs:316-324`). See security note H2 for the unsalted-SHA-256 hash
trade-off in the `Redact` arm.

**The subscriber/broadcast surface shipped (MLP2-071).** The IPC accept loop
routes `subscribe-telemetry` / `unsubscribe-telemetry` frames ahead of the
generic dispatcher (`ipc.rs:1251-1263` method matchers, routing
`ipc.rs:1493-1623`) to a daemon-minted `SubscriberId` — never a wire-supplied
field. On Unix the id is minted from `SO_PEERCRED` peer credentials
(`mint_subscriber_id`, `ipc.rs:4211-4219`); the non-Unix mint is a fail-closed
`None` stub until the `GetNamedPipeClientProcessId`-backed mint lands (MLP2-028
follow-up, `ipc.rs:4221-4228`). Subscribers register via `TelemetryBroadcaster`
(`crates/anvil-intercept/src/broadcaster.rs`), which wraps `Fanout::register`
and owns the delivery half: per-subscriber bounded channels, non-blocking — a
full channel drops-and-counts rather than stalling the producer (INTD-016).
Dropping a connection unregisters via `Subscription`'s `Drop` impl
(`ipc.rs:1225-1245`). The remaining slice is the **producer call sites** for
real assurance/fence transition envelopes — DSV-044's territory
(`broadcaster.rs` module doc).

DoS budgets (§4.4) defend the daemon against same-UID peers attempting to starve
the listener; they live in `dos.rs` and are configured per-connection (no global
rate limit — the connection cap is the global safeguard).

## 12. Embedded validation path

`crates/anvil-intercept/src/embedded.rs` ships `embedded_evaluate` — a
synchronous in-process API that produces the same `EnforcementDecision` the
daemon-backed path produces. The MCP shim's `LocalDaemonValidationClient`
(`crates/anvil-cli/src/mcp/validation.rs`) routes between daemon-backed and
embedded based on `DaemonValidationOutcome`:

- `Available` → daemon-backed scan_buffer.
- `Unavailable` → embedded fallback.
- `OperationalFailure` → propagate the error (NOT auto-promote to embedded —
  pinned by
  `embedded.rs::tests::embedded_does_not_auto_promote_from_failed_daemon_path`,
  `embedded.rs:38-44`).

**Diagnostic-envelope parity** is the load-bearing property. The
`anvil.diagnostic.v1` shape returned by embedded mode is byte-identical to the
daemon-backed path on the same fixture (`embedded.rs:21-25`,
`crates/anvil-cli/src/mcp/validation.rs::tests::local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`).
Changing the embedded diagnostics without updating the daemon path breaks the
test.

**Honoured config** (`embedded.rs:46-65`):

| Resolved mode        | Embedded behaviour                                    |
| -------------------- | ----------------------------------------------------- |
| `Mode::Warn`         | Always `Allow` with diagnostics on the side channel.  |
| `Mode::Fence`        | Pipeline result returned as-is; no fence side-effect. |
| `Mode::Interrupt`    | Pipeline result returned as-is; no interrupt.         |
| `observe_only: true` | Always `Allow` regardless of `mode`.                  |

Embedded mode does not have a fence store or a process group; the caller applies
the side effect (CI / MCP shim).

**Windows special case in v1.** The MCP validation client's `validate_pre_write`
is gated `#[cfg(unix)]` in `crates/anvil-cli/src/mcp/validation.rs`. On Windows
the `cfg(not(unix))` arm returns `DaemonValidationOutcome::Unavailable`
unconditionally, which the caller maps to `DaemonStatus::NotWired`. This is
recorded in the runbook (`docs/archive/runbooks/v0.6.0-beta-release-runbook.md`
§2): on Windows `correlation.daemonStatus` in `validate_write` MCP responses is
always `not-wired` in v1 — it cannot distinguish daemon-up from daemon-down.

**The MCP pre-write path intentionally stays on `scan_buffer`** — not the
save-time `validate_paths` verb (DSV-007,
`crates/anvil-cli/src/mcp/validation.rs:99-103`): `validate_write` is a
pre-write gate over **proposed** content the daemon has not read, whereas
`validate_paths` re-reads written bytes under the guarded anchor.
`validate_paths` is the save-time surface consumed by `anvil watch`; see §4a.

## 13. §4.4 redaction filter

The §4.4 daemon-side redaction contract spans three MCP tool surfaces:
secret-detection content excerpts masked to `<<redacted: secret>>`, absolute
paths rewritten to workspace-relative, and `fix.apply` diff payloads
pre/post-masked
(`plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md` §4.4).

In `v0.6.0-beta` the filter is **wired only for `validate_write`**
(`crates/anvil-cli/src/mcp/tools/validate_write.rs:374-424`,
`normalise_response_diagnostics` + `redact_secret_values`). For other MCP tool
surfaces (`scan.files`, `fix.apply`, `status.query`) the contract is
**spec-only** — the runtime filter integration is owned by RMCPF-010 and lands
in a later tag. See security note H3
(`docs/archive/runbooks/v0.6.0-beta-security-note.md:143-185`) for the operator
framing: an MCP client an operator does not fully trust will see un-redacted
absolute paths and un-redacted secret-rule excerpts for those three tools in v1.

The redaction primitive `hash_of_path` (`fanout.rs:436-441`) is unsalted
SHA-256; per-startup HMAC is tracked for the next tag (security note H2).

## 14. CLI surface (operator view)

`crates/anvil-cli/src/commands/intercept.rs:41-66` declares four subcommands
(`Start`, `Status`, `Unblock`, `Stop`):

- `anvil intercept start --foreground` — starts the daemon in foreground mode.
  Without `--foreground` the command bails with the actionable message at
  `intercept.rs:1191-1196`. Ctrl+C and Unix SIGTERM stop it cleanly via
  `wait_for_shutdown_signal`.
- `anvil intercept status [--json]` — issues a JSON-RPC `query_status` request
  and prints the rendered snapshot. On Unix it speaks the legacy `query_status`
  method; on Windows it speaks the canonical `anvil/status/query` form. The
  daemon dual-routes both names so the rendered output is identical.
- `anvil intercept unblock` — clears fence state: `--worktree <PATH>` for a
  single worktree, `--all` for every fence (both honour `--dry-run`), or the
  positional `<WORKTREE> --acknowledge-cascade` cascade-clear (§8). Non-dry-run
  unblocks dispatch a daemon `unblock-*` verb that records the authoritative
  usage row (USAGE-004); the CLI-side row is suppressed to avoid a double-count.
- `anvil intercept stop` — stops the per-user daemon recorded in the PID file
  (V060F-002 / ACTMO-008, `intercept.rs:128-175`). Unix sends SIGTERM so the
  daemon flushes fence state and unbinds its IPC listener; Windows terminates
  the headless process and clears the PID file. Idempotent — a missing or stale
  PID file exits zero with an informational line, and it warns how many
  registered worktrees will lose protection (ACTMO-017).

Both `stop` and `unblock` are shipped CLI commands; the operator-runbook
substitutes they once replaced (Ctrl-C / SIGTERM by PID; `rm -rf` of the anvil
state dir) remain available as blunt fallbacks. See §16 gap 1 (resolved).

## 15. Win32 listener

`anvil-intercept-win32` ships the Windows-specific Win32 boundary
(`crates/anvil-intercept-win32/src/lib.rs`):

- `create_owner_only_pipe_server(pipe_name, instance)` — builds the named-pipe
  server with an owner-only DACL granting `0x12019f` (avoids `GENERIC_ALL` to
  preserve a same-user trust scope) and `reject_remote_clients(true)`
  (`lib.rs:39-63`). This is what the daemon-side `IpcListener` calls on Windows;
  the listener integration arrived in #1325.
- `current_user_sid_string()` — the current process token's user SID string
  (`lib.rs:682-685`), the identity anchor for the pipe-name rendezvous. The SID
  is used, not the env username, so account-name spoofing cannot move the
  rendezvous. The canonical pipe name itself is derived by
  `anvil_intercept::ipc::resolve_pipe_name` / `derive_pipe_name`
  (`anvil-intercept/src/ipc.rs:718-760`, CIB-106): with `ANVIL_HOME` unset/blank
  the legacy `\\.\pipe\anvil-intercept-<sid>` name, byte-for-byte; with a
  non-empty `ANVIL_HOME` a stable bounded `-r<16-hex FNV-1a-64>` suffix hashed
  from the absolutised install root, so a candidate daemon and the production
  daemon get distinct pipes and coexist (the Windows half of DISTRIB-006 /
  ADR-060) without leaking the raw path into the locally enumerable pipe
  namespace.
- `connect_owner_only_pipe_client(pipe_name)` — synchronous client used by the
  CLI's Windows status path (`lib.rs:121`). All `unsafe` for `CreateFileW`,
  `WriteFile`, `ReadFile`, `CloseHandle` is quarantined to this crate so
  `anvil-intercept` keeps `#![forbid(unsafe_code)]`.
- `JobObject` + `terminate_job_object` — Windows interrupt path. Used by
  `interrupt.rs::windows_impl::run_windows_termination`.
- `process_creation_time(pid)` — Windows analogue of Linux `/proc/PID/stat`
  field 22; used by the PID-reuse defence and by `process_start_time` in
  `lib.rs`.

PID-file path on Windows: `%LOCALAPPDATA%\anvil\intercept.pid`
(`lib.rs:289-292`). Fence-state path on Windows:
`%LOCALAPPDATA%\anvil\intercept-fences.json` (`fence.rs:404-409`).

The Windows cross-compile CI matrix is covered in
`docs/runbooks/intd-012-windows-evidence.md`. Trigger gating (`main`-only) is
documented in §16 gap 5.

## 16. Known gaps (dated 2026-05-07)

1. **(Resolved 2026-07-02.)** Originally tracked the missing
   `anvil intercept stop` / `anvil intercept unblock` CLI front-ends. Both are
   now shipped: `crates/anvil-cli/src/commands/intercept.rs:41-66` declares four
   subcommands (`Start`, `Status`, `Unblock`, `Stop`). `unblock` (RCLI3-017b /
   MLP2-026) wires `FenceStore::unblock_worktree` to clap in per-fence / `--all`
   / cascade modes (§8); `stop` (V060F-002 / ACTMO-008) stops the daemon
   recorded in the PID file (§14). The runbook's recovery paths now match the
   code. Residual: on Windows the per-fence and cascade `unblock` dispatches
   bail pending MLP2-028 peer-credential support.
2. **(Resolved 2026-05-07.)** Originally tracked the runbook's stale "Windows
   hard-fails" framing for `anvil intercept status`. The runbook was corrected
   to match HEAD: the Windows status client ships at
   `crates/anvil-cli/src/commands/intercept.rs:143-148` and `:170+`
   (`query_daemon_status_windows_at`, via `connect_owner_only_pipe_client`) and
   operators on Windows get a parity status response. The residual Windows MCP
   gap is gap 9 below (`daemonStatus: not-wired` from the `cfg(unix)`-gated MCP
   validation client).
3. **macOS `current_process_start_time` branch missing.**
   `crates/anvil-intercept/src/interrupt.rs:419-431` returns `None`
   unconditionally on macOS. AD-7 forces a fence on every interrupt decision
   against a session with a recorded start time, so the macOS interrupt ladder
   is fence-first in v1.
4. **`drivers.allow` file mode not verified before read** (security note H1,
   `v0.6.0-beta-security-note.md:73-104`). `is_driver_allowed`
   (`crates/anvil-intercept/src/auth.rs:227-267`) opens the allowlist with
   `fs::read_to_string` without an `lstat` for owner / mode. Operators must
   manually `chmod 0600 ~/.config/anvil/drivers.allow`. Tracked for the next
   tag.
5. **Cross-session telemetry redaction hash is unsalted SHA-256** (security note
   H2, `v0.6.0-beta-security-note.md:108-139`). `hash_of_path`
   (`crates/anvil-intercept/src/fanout.rs:436-441`) is plain
   `format!("[redacted:{}]", hex_sha256(input))`. Treat `Delivery::Redact` as a
   policy-compliance signal, not a confidentiality boundary; default is
   `telemetry.allow_cross_session: false`. Per-startup HMAC tracked for the next
   tag.
6. **§4.4 redaction filter is spec-only outside `validate_write`** (security
   note H3, `v0.6.0-beta-security-note.md:143-185`). Wired filter today is
   `crates/anvil-cli/src/mcp/tools/validate_write.rs:374-424`; `scan.files`,
   `fix.apply`, and `status.query` ship un-redacted absolute paths and
   un-redacted secret-rule excerpts in v1. RMCPF-010 wires the runtime parity in
   the next tag.
7. **Linux PID-reuse defence has microsecond-scale TOCTOU; macOS interrupt
   ladder is fence-only** (security note H4,
   `v0.6.0-beta-security-note.md:189-251`). The Linux window between
   `read_to_string("/proc/PID/stat")` (`interrupt.rs:410-415`) and the actual
   `kill()` is intrinsic to the syscall shape; AD-7's fence-on-failure invariant
   is the documented mitigation. macOS gap is item 3 above.
8. **Windows CI cross-compile is `main`-only**
   (`docs/runbooks/intd-012-windows-evidence.md`). Since the `dev` branch was
   retired (OPMODEL-012, 2026-05-11) the repo is trunk-only: the Windows matrix
   runs on pushes to `main` (and PRs into it), while feature-branch PRs skip it
   to save cost. Windows-affecting drift on a feature branch is therefore only
   caught once the PR reaches `main` — run the cross-compile locally
   (`cargo check --target x86_64-pc-windows-gnu`) before opening the PR.
   Deliberate cost/coverage trade-off.
9. **MCP `daemonStatus` always `not-wired` on Windows.**
   `crates/anvil-cli/src/mcp/validation.rs:142-148`'s `cfg(not(unix))` arm
   returns `DaemonValidationOutcome::Unavailable`; the caller maps that to
   `DaemonStatus::NotWired` at lines `:371-382`. The MCP `validate_write`
   correlation envelope cannot distinguish daemon-up from daemon-down on Windows
   in v1, regardless of whether `intercept status` itself works over the named
   pipe.
10. **Daemon-version skew not addressed** (`v0.6.0-beta-release-runbook.md`
    "Edge cases and known gaps"). v1 is a single-version daemon: no rolling
    upgrade story, no peer compatibility matrix, no driver-version negotiation
    that survives daemon restart. Operators should run a single tagged binary on
    a single user account.
11. **Background launch not validated in v1.** The daemon binary supports the
    mechanics of a backgrounded launch (the PID file guards against it), but
    `anvil intercept start --foreground` is the only mode the release council
    validated for `v0.6.0-beta` (`v0.6.0-beta-release-runbook.md:58-63`).
    Tracked outside this tag.

## 17. Source references

### `crates/anvil-intercept/src/`

| File                     | Role                                                                                                                                                                                                                                                                       |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lib.rs`                 | INTD-001 entry point: `run_foreground`, PID-file guard, `Shutdown` channel, `wait_for_shutdown_signal`. The single-source-of-truth for daemon lifecycle.                                                                                                                   |
| `main.rs`                | Standalone `anvil-intercept` binary entry; calls into `run_foreground`.                                                                                                                                                                                                    |
| `assurance.rs`           | per-`WorktreeKey` `AssuranceMachine`; workspace-assurance state folded from graph certifiability.                                                                                                                                                                          |
| `auth.rs`                | DRVR-007 / DRVR-008: `is_driver_allowed`, `DriverManifest::validate_workspace_roots`, `negotiate_capability`. The driver trust boundary.                                                                                                                                   |
| `broadcaster.rs`         | MLP2-071 Phase 2: `TelemetryBroadcaster` delivery half of the fan-out.                                                                                                                                                                                                     |
| `config.rs`              | INTD-008: `Resolved` enforcement policy loader. Stricter-wins merge between project `.anvil.yaml` and user config; AD-3 ambiguous-ownership cap.                                                                                                                           |
| `confinement.rs`         | DSV-008: operator confinement config (open/allowlist), owner-only read from daemon home prefix.                                                                                                                                                                            |
| `dos.rs`                 | INTD-016: `IpcLimits` + `RpsBucket`. Per-connection token bucket and frame-size cap.                                                                                                                                                                                       |
| `embedded.rs`            | INTD-009: synchronous `embedded_evaluate` API; correctness-equivalent fallback for the MCP shim and CI.                                                                                                                                                                    |
| `enforcement.rs`         | INTD-005: `EnforcementPipeline`, `EnforcementDecision`, `ProposedChange`. Pure rule evaluation.                                                                                                                                                                            |
| `fanout.rs`              | INTD-015: per-event telemetry filter with daemon-minted `SubscriberId`; `Delivery::{Allow, Redact, Deny}`.                                                                                                                                                                 |
| `fence.rs`               | INTD-005 + INTD-007: `FenceStore` on-disk persistence, `FenceState` in-memory view, `FenceRecord` with aliases.                                                                                                                                                            |
| `full_scan_executor.rs`  | DSV-045 (ADR-085): the full-scan executor that drives `Pending → Running → Clean`/`Bounded`, populating the warm cache without a save. `ScanCoordinator` (per-key coalescing CAS + cancel), `prepare_scan`, `run_scan_loop` (background-pool walk/parse/apply + watchdog). |
| `interrupt.rs`           | INTD-006: `run_unix_ladder`, `run_windows_termination`, AD-7 fence-on-failure invariant, PID-reuse defence.                                                                                                                                                                |
| `ipc.rs`                 | INTD-002: NDJSON IPC listener, owner-only socket permission ladder, peer-credential validation, `validate_socket_path_for_client`.                                                                                                                                         |
| `kernel_cache.rs`        | warm `(SymbolGraph, DependencyGraph)` cache (`KernelGraphCache`); `apply_delta`, `with_graphs`.                                                                                                                                                                            |
| `latency.rs`             | INTD-011: sliding-window aggregator for ADR-031 `validation.service` measurements (mid-edit p50/p95).                                                                                                                                                                      |
| `midedit.rs`             | INTD-005 mid-edit surface: `ScanBufferService`, `ScanBufferRequest`/`Response`, `MAX_CONCURRENT_SCAN_BUFFERS`.                                                                                                                                                             |
| `path_safety.rs`         | DSV-003/ADR-068: `openat2` `RESOLVE_NO_SYMLINKS \| RESOLVE_BENEATH` guarded read (Unix) / Windows anchor ladder.                                                                                                                                                           |
| `registry.rs`            | INTD-003: `SessionRegistry` (synchronous), `SessionDispatcher` trait, `Attribution`, `evict_stale`.                                                                                                                                                                        |
| `save_time.rs`           | DSV-005: per-connection save-time verb orchestration; `SaveTimeState`, `AdmittedRoots`, `SymbolParser` injection.                                                                                                                                                          |
| `status.rs`              | INTD-011: `DaemonStatus` snapshot, `DaemonStatusProvider`, wire conversion to `DaemonStatusV1`.                                                                                                                                                                            |
| `telemetry.rs`           | `anvil.notification.v1` envelope construction, `TelemetryEmitter`, `TelemetryCorrelation` with INTD-015 scoping fields.                                                                                                                                                    |
| `unregistered.rs`        | INTD-010: handler for changes that fall outside any registered session — AD-3 always-fence policy.                                                                                                                                                                         |
| `validate_paths.rs`      | DSV-004/-005: pure save-time verdict core; wire↔internal mappings; certify via GV2 hot-read; diagnostic sort-before-envelope.                                                                                                                                              |
| `watcher.rs`             | INTD-004: kernel watcher integration; receives `ChangeBatch`, attributes paths, dispatches to per-session enforcement or `UnregisteredHandler`.                                                                                                                            |
| `workspace_admission.rs` | per-connection `AdmittedRoots`; C2/C3 root-retarget defence.                                                                                                                                                                                                               |

### `crates/anvil-intercept-proto/src/`

| File                    | Role                                                                                             |
| ----------------------- | ------------------------------------------------------------------------------------------------ |
| `lib.rs`                | `SessionId`, `SessionRecord`, `SessionStatus`, `IpcCommand`, `IpcEnvelope`. The wire vocabulary. |
| `protocol.rs`           | `anvil/`-namespaced JSON-RPC method constants and `Capability` lattice.                          |
| `status.rs`             | `DaemonStatusV1` and friends — wire shape for `query_status`.                                    |
| `enforcement_config.rs` | `AnvilConfigFile` / `EnforcementConfigFile` / `DosConfigFile` / `TelemetryConfigFile` decoders.  |

### `crates/anvil-intercept-rules/src/`

The rule registry the daemon evaluates against. Hot-path rules; deliberately
small to keep the daemon's evaluation cost predictable.

| File           | Role                                                              |
| -------------- | ----------------------------------------------------------------- |
| `lib.rs`       | `InterceptRule` trait, `RuleInput`, `RuleDecision`, `ChangeKind`. |
| `registry.rs`  | `RuleRegistry`, `RegistryDecision`, default-deny composition.     |
| `reasoning.rs` | `LaunchReasoningPatternRule` — antipattern detector.              |
| `secret.rs`    | `SecretDetectionRule` — content-bearing secret scanner.           |

### `crates/anvil-intercept-win32/src/`

| File     | Role                                                                                                                                                                                                                                                                                           |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lib.rs` | Owner-only named-pipe server + sync client, `JobObject`, `terminate_job_object`, `process_creation_time`, `current_user_sid_string` (pipe-name identity anchor; the name itself is derived by `anvil_intercept::ipc::resolve_pipe_name`, CIB-106). The Windows boundary; all `unsafe` is here. |

## 18. Related docs

- `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` — operator runbook;
  foreground daemon, fence persistence, macOS fence-first, Windows CI gap. The
  user/operator perspective; this doc is the implementation perspective.
- `docs/archive/runbooks/v0.6.0-beta-security-note.md` — the four HIGH security
  trade-offs (drivers.allow file mode, redaction hash unsalted, §4.4 filter
  spec-only, PID-reuse). Cross-referenced inline.
- `docs/runbooks/intd-012-windows-evidence.md` — Windows CI cross-compile matrix
  coverage; `main`-only trigger gating rationale.
- `docs/architecture/auth-as-built.md` — sibling as-built doc; the shape model
  this doc follows.
- `plans/specs/2026-04-26-rtai-demo-runbook.md` §1.5, §3, §4.1 — operator-
  facing demo runbook; status output contract, reset paths, foreground-daemon
  failure modes.
- `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md` §2.3a,
  §3.2, §3.3, §3.7, §4.4 — driver trust boundary, method namespace policy,
  capability state machine, redaction contract.
- `plans/decisions/015-intercept-loop-enforcement.md` — AD-3 (ambiguous-
  ownership hard cap), AD-4 (plaintext-local-only IPC), AD-7 (fence-on-failure
  invariant).
- `plans/archive/modules/intercept-daemon.aps.md` — INTD-001 through INTD-016
  work items (16/16 complete).
- `plans/archive/modules/intercept-launcher.aps.md` — INTL-001 through INTL-009
  (9/9 complete); the launcher side that registers sessions with the daemon.
- `docs/public/anvil/integrations/mcp.md` — public MCP integration doc; the
  `validate_write` tool and `correlation.daemonStatus` field documented here.
