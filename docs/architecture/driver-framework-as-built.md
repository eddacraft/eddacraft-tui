# Driver Framework + intercept-proto — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| -------- | --------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | DRVR  | Live   | Last reviewed 2026-07-02 (as-built drift sweep: `ALL_ANVIL_METHODS` reconciled to 19 constants incl. DSV + witness + GCTX, repinned protocol.rs line refs to agree with intercept-as-built) against main `d1fded280`; prior delta review 2026-06-10 (INTR-003/-005/-007 rule set + config, §8.4 panic-policy correction) against main `a1c41e284`; full review 2026-05-07 against `v0.6.0-beta` and `crates/anvil-intercept-proto`, `crates/anvil-intercept-rules`, `packages/anvil-driver-client` |

| Upstream                                                                                                                                         | Downstream                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| `crates/anvil-intercept-proto`, `crates/anvil-intercept-rules`, `crates/anvil-intercept-win32`, `packages/anvil-driver-client`, ADR-030, ADR-033 | intercept daemon (INTD), MCP shim (RMCP), CLI intercept surface, future editor / CI driver integrations |

> **Status:** Live (beta) for the proto + driver-client TypeScript glue;
> framework spec is partially shipped (DRVR Waves 1-3 active; Wave 4 deferred
> per ADR-033). **Last reviewed:** 2026-06-10 (targeted delta review:
> INTR-003/-005/-007 rule set + config, §8.4 panic-policy correction) against
> main `a1c41e284`; full review 2026-05-07 against `v0.6.0-beta` slate (HEAD
> `d223b8d9`; 2026-07-02 drift sweep against `d1fded280`). **Crates /
> locations:** `crates/anvil-intercept-proto`, `crates/anvil-intercept-rules`,
> `packages/anvil-driver-client` (+ Win32 primitives in
> `crates/anvil-intercept-win32`). **Module owner (APS):** DRVR
> (`plans/archive/modules/surface-drivers.aps.md`, 5/5 active — 2 superseded, 1
> deferred under ADR-033), with downstream spec at
> `plans/specs/anvil-driver-framework/`. Adjacent modules: INTR
> (`plans/archive/modules/intercept-rules.aps.md`, Complete 8/8) for the
> hot-path rule registry, and INTD
> (`plans/archive/modules/intercept-daemon.aps.md`, Complete 16/16) for the
> daemon side that consumes the proto. **Used by:** `anvil intercept status` CLI
> surface (`crates/anvil-cli/src/commands/intercept.rs`); MCP shim's
> daemon-backed validation client (`crates/anvil-cli/src/mcp/validation.rs`);
> the in-tree daemon-side capability negotiator
> (`crates/anvil-intercept/src/auth.rs`); future editor / CI driver integrations
> consuming `@eddacraft/anvil-driver-client`; the Win32 named-pipe synchronous
> client (`crates/anvil-intercept-win32::connect_owner_only_pipe_client`).

## 1. Overview

The "driver framework" is the wire-format crate plus driver-client glue that
lets external tools speak to the `anvil-intercept` daemon over JSON-RPC 2.0 with
peer-credential authentication, capability negotiation, and a versioned
status-snapshot shape.

Three deliverables ship in `v0.6.0-beta`:

- `crates/anvil-intercept-proto` — the Rust crate that owns the authoritative
  wire vocabulary: session IDs, `IpcEnvelope`, the `anvil/`-namespaced JSON-RPC
  method-name constants, the `Capability` lattice, the `DaemonStatusV1` snapshot
  shape, and the shared `.anvil.yaml` enforcement-config decoders. The Rust side
  is the single source of truth (`protocol.rs:11-26`).
- `packages/anvil-driver-client` — the TypeScript / Node companion package. It
  ships the `DriverClient` class with auto-selecting transport (UDS on Unix,
  named pipe on Windows), JSON-RPC 2.0 over NDJSON framing, transparent
  reconnect with exponential backoff, a reliability-budget quarantine ledger
  keyed on the daemon-minted identity, and a mid-edit validation helper. TS
  bindings mirror the Rust constants byte-for-byte
  (`packages/anvil-driver-client/src/protocol/types.ts:1-32`).
- `crates/anvil-intercept-rules` — the rule trait, registry, and v1 rule set
  (secret detection, launch reasoning-pattern). The daemon's enforcement
  pipeline composes rules from this crate; it is part of the framework only
  insofar as drivers do not own rule code — the rules execute behind the same
  `scan_buffer` / `validate_write` RPCs drivers consume (§8 below).

The framework is consumed today by:

- `anvil intercept status` — a Rust client of the proto running in the CLI
  binary, switching transport per OS
  (`crates/anvil-cli/src/commands/intercept.rs:77-148`).
- The MCP shim's `LocalDaemonValidationClient`, which speaks `scan_buffer`
  directly to the daemon (`crates/anvil-cli/src/mcp/validation.rs`; see
  `mcp-shim-as-built.md` §5).
- Future editor / CI driver integrations that import
  `@eddacraft/anvil-driver-client` and speak the same protocol.

The framework spec at `plans/specs/anvil-driver-framework/` (covering shell,
remote-shell, tmux, process, web-session, editor, and MCP drivers) is the design
north star. The shipping code corresponds to a slice of that spec — see §12 for
the spec → code reconciliation table.

## 2. Architecture diagram

```text
   external driver (editor / CI / custom tool)
              │
              │  // imports
              ▼
   ┌──────────────────────────────────────────┐
   │ DriverClient (TS) or proto consumer (Rs) │
   │  - JSON-RPC 2.0 envelopes                │
   │  - NDJSON framer (1 line = 1 envelope)   │
   │  - reconnect + reliability budget        │
   └──────────────┬───────────────────────────┘
                  │
                  ▼
        ┌─────────────────────┐
        │  Transport (auto)   │
        │  Unix UDS  │  Win   │
        │  intercept │  named │
        │   .sock    │  pipe  │
        └────────┬───┴────┬───┘
                 │        │
                 ▼        ▼
         ┌────────────────────────────────┐
         │   anvil-intercept (daemon)     │
         │                                │
         │   (handshake)                  │
         │   ┌─ DRVR-007 allowlist        │
         │   │  is_driver_allowed         │
         │   │  + drivers.allow path      │
         │   └─ DRVR-008 capability nego  │
         │      negotiate_capability      │
         │      (Attached / Participating)│
         │                                │
         │   (steady state)               │
         │   - dual-routes legacy +       │
         │     anvil/-namespaced methods  │
         │   - status / scan_buffer /     │
         │     enforcement-ack / gate /   │
         │     suppression / publish     │
         └────────────────────────────────┘

  ── Same-UID local IPC only ──
  Unix:    SO_PEERCRED / getpeereid
           + 0700 dir / 0600 socket
  Windows: kernel ACL on the named pipe
           (SID-derived rendezvous,
            reject_remote_clients(true))
```

The two gates the driver crosses BETWEEN connect and active session are the
daemon-side pieces of the framework's auth surface (§5). The daemon's owner-only
IPC layer itself is documented in `intercept-as-built.md` §4.2 / §5.

## 3. Wire protocol (`anvil-intercept-proto`)

### 3.1 Frame format

Every line on the wire is one NDJSON envelope. The proto crate owns two envelope
shapes:

- **Legacy NDJSON** — `IpcEnvelope` flattening an `IpcCommand` payload
  (`anvil-intercept-proto/src/lib.rs:98-130`). Variants: `RegisterSession`,
  `Heartbeat`, `UnregisterSession`, `ListSessions`, `QueryStatus`. Externally
  tagged as `{"command": "...", ...}` with kebab-case discriminators
  (`anvil-intercept-proto/src/lib.rs:62-86`). Optional JSON-RPC-style `id` for
  request/response correlation; an explicit `null` id and an absent id both
  deserialise as a notification (`lib.rs:235-255`, pinned by
  `explicit_null_id_deserialises_as_notification`).
- **JSON-RPC 2.0** —
  `{"jsonrpc": "2.0", "method": "...", "id": ..., "params": ...}`. The driver
  client emits **only** JSON-RPC 2.0 envelopes; the legacy NDJSON form is
  retained for the launcher
  (`packages/anvil-driver-client/src/framing/jsonrpc.ts:1-18`).

The daemon's IPC layer accepts both. Legacy frames are routed to the session
registry; JSON-RPC frames route by method name through the mid-edit, status,
gate, and ack handlers (`crates/anvil-intercept/src/ipc.rs:1810-1865`).

The framer caps a single line at
`MAX_LINE_BYTES = (CONTENT_SIZE_CAP_BYTES_USIZE * 6) + 64 KiB` to absorb
worst-case JSON-string encoding for a 1 MiB `scan_buffer` payload
(`intercept-as-built.md` §4.4). The TS framer mirrors the same default
(`packages/anvil-driver-client/src/framing/ndjson.ts:30-39`).

### 3.2 JSON-RPC method names

Pinned in `crates/anvil-intercept-proto/src/protocol.rs` and re-exported
(byte-for-byte) in `packages/anvil-driver-client/src/protocol/types.ts:44-83`.
The **driver-facing** surface is the six `anvil/`-namespaced methods below; the
full `ALL_ANVIL_METHODS` slice has since grown to **19 constants** (the six here
plus the three DSV save-time verbs, the witness-append verb, and the nine GCTX
read-only verbs — see `intercept-as-built.md` §4.3). The TS driver client
mirrors only these six driver-facing methods; the DSV / witness / GCTX verbs are
daemon-internal surfaces the driver client does not consume:

| Constant                    | Wire string                | Direction                      | Purpose                                                                           |
| --------------------------- | -------------------------- | ------------------------------ | --------------------------------------------------------------------------------- |
| `ANVIL_PUBLISH_DIAGNOSTICS` | `anvil/publishDiagnostics` | server → client (notification) | Diagnostic stream carrying `anvil.diagnostic.v1` (`protocol.rs:117`).             |
| `ANVIL_SCAN_BUFFER`         | `anvil/scan_buffer`        | client → server                | Mid-edit buffer scan (`protocol.rs:125`).                                         |
| `ANVIL_ENFORCEMENT_ACK`     | `anvil/enforcement/ack`    | client → server                | Confirm enforcement decision (`protocol.rs:132`); DRVR-008's load-bearing method. |
| `ANVIL_GATE_REQUEST`        | `anvil/gate/request`       | client → server                | Stream / one-shot gate-result snapshot (`protocol.rs:138`).                       |
| `ANVIL_SUPPRESSION_APPLY`   | `anvil/suppression/apply`  | client → server                | Validate and normalise an `@anvil-ignore` comment (`protocol.rs:145`).            |
| `ANVIL_STATUS_QUERY`        | `anvil/status/query`       | client → server                | Snapshot of session / fence / driver state (`protocol.rs:150`).                   |

The daemon also declares two **legacy** bare-name aliases for continuity with
the launcher and the pre-namespacing CLI:

- `scan_buffer` — the bare-name companion to `ANVIL_SCAN_BUFFER`. RTAI-002 /
  RTAI-008 contract fixtures pinned this name; the daemon dual-routes both the
  bare and namespaced forms to the same handler
  (`crates/anvil-intercept/src/ipc.rs:1810-1837`).
- `query_status` — the bare-name companion to `ANVIL_STATUS_QUERY`. Same
  dual-route at `ipc.rs:1839-1865`. The constant ships at
  `crates/anvil-intercept/src/ipc.rs::LEGACY_QUERY_STATUS_METHOD`
  (`ipc.rs:1884`).

The Unix `intercept status` path uses the legacy `query_status` name; the
Windows path uses the canonical `anvil/status/query` form because it is the
first new client written against the namespaced protocol
(`crates/anvil-cli/src/commands/intercept.rs:121-132, 240`,
`LEGACY_QUERY_STATUS_METHOD` at `:311`).

### 3.3 Method namespace policy

Per the editor-and-mcp design spec §3.2 and the proto crate's module docs
(`protocol.rs:28-60`): **no new `anvil/` method without a concrete editor or
driver feature that cannot be expressed in stock LSP**. Stock LSP methods
(`textDocument/publishDiagnostics`, `textDocument/codeAction`,
`workspace/applyEdit`, `window/showMessage`, `initialize` / `initialized`) are
pinned by the LSP spec and are **not** re-declared in the proto crate; drivers
speak both languages over the same transport, and the daemon routes by method
name at the JSON-RPC layer.

`ALL_ANVIL_METHODS` (`protocol.rs:297-317`) is the canonical list — **19 method
constants** (the six driver-facing methods above, the three DSV save-time verbs,
`anvil/witness/append`, and the nine GCTX read-only verbs). The TS mirror
(`protocol/types.ts:73-80`) tracks only the driver-facing subset; the
daemon-internal DSV / witness / GCTX verbs are not re-exported to driver
clients.

### 3.4 `DaemonStatusV1` shape

`crates/anvil-intercept-proto/src/status.rs` owns the wire snapshot returned by
`anvil/status/query` / `query_status`. Top-level fields (`status.rs:22-38`):

| Field       | Type                    | Notes                                                                                  |
| ----------- | ----------------------- | -------------------------------------------------------------------------------------- |
| `sessions`  | `Vec<SessionRecord>`    | Live registered sessions (`SessionRecord` from `lib.rs:136-152`).                      |
| `worktrees` | `Vec<WorktreeStatusV1>` | Per-worktree overlay: worktree, session id, fenced flag (`status.rs:40-45`).           |
| `fences`    | `Vec<FenceStateV1>`     | One entry per persisted fence (`status.rs:47-52`).                                     |
| `health`    | `HealthStateV1`         | `uptime_seconds`, `version`, `ipc_state` (`Serving` / `Draining`) (`status.rs:54-68`). |
| `latency`   | `LatencyMidEditMapV1`   | ADR-031 rollups; `mid_edit` is `Option<LatencyRollupV1>` (`status.rs:70-91`).          |

**Cross-platform parity contract.** The CLI's `query_daemon_status` shape is
identical on Unix and Windows
(`crates/anvil-cli/src/commands/intercept.rs:77-148`). Both paths use the same
shared frame builder and response parser (`build_query_status_frame_bytes` at
`intercept.rs:322-331`, `parse_query_status_response_bytes` at
`intercept.rs:338-371`); the only daemon-bound difference is the JSON-RPC method
name (Unix: `query_status`; Windows: `anvil/status/query`). `DaemonStatusV1`
deserialises identically on either OS, and `--json` returns the same shape — see
`intercept-as-built.md` §4.5 and runbook §2 for the operator framing.

**Versioning policy.** `DaemonStatusV1`'s name carries the explicit `V1` suffix.
Two forward-compat properties are pinned at the type level:

- `LatencyMidEditMapV1` lets `mid_edit` be `None` on the wire (`null` or
  absent); consumers MUST treat both as "no traffic" and MUST NOT default to
  zero (`status.rs:74-81`).
- The struct is deliberately **not** `deny_unknown_fields`. The
  `deserialise_tolerates_unknown_top_level_fields` test (`status.rs:152-174`)
  pins this — a future Wave 4 daemon emitting additional top-level keys is
  forward-compat with today's parsers.

A bump to `DaemonStatusV2` would happen when an existing field's semantics
changes — adding fields is additive and stays at V1.

### 3.5 Session record + envelope

`SessionRecord` is the wire mirror of the daemon's in-memory session registry
entry (`lib.rs:136-152`). Timestamps are `u64` Unix seconds rather than
`Instant` so the format is stable across daemon restart and JSON-serialisable
(`lib.rs:130-136`). `SessionStatus` is a two-variant kebab-case enum (`active` /
`ended`); the `list-sessions` result never contains `Ended` records — eviction
removes the entry outright (`lib.rs:156-165`).

`SessionId` is an opaque newtype on `String`, deliberately accepting the empty
string at the proto layer. Validation is the daemon-side registry's
responsibility (`lib.rs:35-50, 178-185`). The "single authority on uniqueness"
comment is the contract pin.

## 4. `enforcement_config` — `.anvil.yaml` wire shape

`crates/anvil-intercept-proto/src/enforcement_config.rs` owns the shared decoder
for the `.anvil.yaml` `enforcement` block. Both the MCP launch shim (RTAI-006)
and the intercept daemon (INTD-008) deserialise the same `EnforcementConfigFile`
struct so the two consumers cannot drift on which keys are accepted.

Top-level shape (`AnvilConfigFile` at `enforcement_config.rs:204-220`):

```yaml
enforcement:
  mode: warn | fence | interrupt | block | off | advisory | proceed
  on_ambiguous_ownership: warn | fence
  observe_only: true | false
  dos:
    max_connections: 64
    rps_sustained: 100
    rps_burst: 1000
    handshake_timeout_seconds: 5
    idle_timeout_seconds: 60
    control_frame_max_bytes: 65536
telemetry:
  allow_cross_session: false
```

What the driver / shim can request, and what the daemon can override:

- The proto layer is intentionally forgiving — every key is `Option<...>`,
  unknown top-level keys are silently ignored, and malformed values surface as a
  deserialise error rather than a default. Each consumer maps the raw strings
  onto its own resolved enum (`enforcement_config.rs:1-50`).
- `mode` aliases (`enforcement_config.rs:71-91`): `block` and `interrupt` both
  collapse to `Mode::Interrupt` on the daemon side; `fence` is `Mode::Fence`;
  `warn` is `Mode::Warn`; `off` / `advisory` / `proceed` map to `Mode::Warn` on
  the daemon (no `off` mode by spec) but to `EnforcementMode::Off` on the MCP
  shim. The aliases are case-folded, trimmed, and shared by both consumers.
- `on_ambiguous_ownership` is **hard-capped at `fence`** by the daemon's
  resolved-policy code (AD-3 in
  `plans/decisions/015-intercept-loop-enforcement.md`); operators who set `warn`
  here see the daemon refuse to interrupt on ambiguous attribution regardless.
  The proto layer accepts the value verbatim; the daemon's `Resolved` loader is
  where the cap fires (`crates/anvil-intercept/src/config.rs`; cross-link
  `intercept-as-built.md` §4.4).
- `observe_only: true` makes the daemon evaluate rules and emit telemetry but
  never fence or interrupt — the AD-3 dry-run path
  (`enforcement_config.rs:106-118`).
- `enforcement.dos.*` — INTD-016 budgets reserved at the proto layer in INTD-008
  and consumed by `IpcLimits::from_config` in INTD-016
  (`enforcement_config.rs:120-176`). RTAI-006 ignores this field.
- `telemetry.allow_cross_session: false` is the safe default per the 2026-04-24
  council M5 review; cross-session events are dropped entirely unless explicitly
  opted in (`enforcement_config.rs:179-202`).

**Stricter-wins merging** between project `.anvil.yaml` and user config is the
daemon's responsibility — the proto layer does no IO and no merge. Cross-link
`intercept-as-built.md` §4.4 for the resolved policy ladder; the merge
invariants are pinned at `crates/anvil-intercept/src/config.rs:316-423`.

## 5. Driver registration and trust boundary

The daemon's same-UID, local-IPC trust boundary is documented in
`intercept-as-built.md` §5. This section documents what a driver **must
provide** to clear the two driver-specific gates that sit between connect and
active session.

### 5.1 DRVR-007 allowlist gate

`crates/anvil-intercept/src/auth.rs::is_driver_allowed` (`auth.rs:227-267`) is a
pure-function policy:

- **Default location.** `default_allowlist_path()` resolves to
  `~/.config/anvil/drivers.allow` on Unix (`$XDG_CONFIG_HOME` then
  `$HOME/.config`) or `%APPDATA%\anvil\drivers.allow` on Windows
  (`auth.rs:155-192`).
- **Format.** Newline-delimited absolute paths; lines that are blank or start
  with `#` after trimming are ignored. Stale entries (paths that fail to
  canonicalise) are silently skipped — not treated as a match
  (`auth.rs:215-264`).
- **Match policy.** Equality on canonicalised paths. No lexical fallback:
  `/usr/local/bin/anvil-vscode` and `/usr/local/bin/../bin/anvil-vscode` would
  otherwise count as distinct (`auth.rs:222-226`,
  `allowlist_canonicalises_entries_for_matching`).
- **Default deny.** Missing allowlist closes the gate (`Ok(false)`); unreadable
  allowlist surfaces as `AuthError::AllowlistUnreadable`; un-canonicalisable
  driver path surfaces as `AuthError::DriverPathInvalid` (`auth.rs:228-249`).

The allowlist is consulted only when a driver requests
`capability.enforcementCandidate: true`. Same-UID `SO_PEERCRED` is the floor;
the allowlist is the next layer.

### 5.2 DRVR-007 manifest workspace-roots validation

`DriverManifest` (`auth.rs:297-330`) is the v1 slice of the §2.2 manifest the
daemon's auth crate cares about: the claimed `workspace_roots: Vec<PathBuf>` and
the advertised `supported_anvil_methods: Vec<String>`. The full §2.2 manifest
decoder lands with DRVR-001 (deferred per the auth.rs module docs); this slice
is what the daemon needs to run the contracts.

`validate_workspace_roots` (`auth.rs:412-453`) is the three-way semantic from
§2.3a:

1. **Empty claim** (`workspace_roots: []`) — the "any-workspace observer" case
   (e.g. a diagnostic-only sidecar). Returns `Ok(())`; capability scoping is
   enforced at the negotiation layer instead.
2. **Non-empty claim, ≥1 match** — at least one claimed path canonicalises to an
   active session worktree. Returns `Ok(())`; the consumer (DRVR-001) drops the
   unmatched roots from the effective scope.
3. **Non-empty claim, 0 matches** — every claimed path either fails to
   canonicalise or matches no live session. Returns
   `AuthError::NoMatchingWorkspaceRoot { claimed }` with the original
   pre-canonical paths echoed so a driver author can debug the rejection.

The pre-`v0.6.0-beta` implementation discarded the boolean match result and
accepted any non-empty manifest; this contract change is called out at
`auth.rs:398-402` and pinned by `manifest_with_only_unknown_roots_returns_error`
and the zero-session symmetric test.

### 5.3 DRVR-008 capability negotiation

`negotiate_capability(requested, manifest)` (`auth.rs:558-580`) is a **pure
function** of `(Capability, &DriverManifest)`. The contract:

| Requested       | Manifest advertises `anvil/enforcement/ack`? | Result                                                                             |
| --------------- | -------------------------------------------- | ---------------------------------------------------------------------------------- |
| `Attached`      | (any)                                        | `(Attached, None)` — read-only floor.                                              |
| `Participating` | yes                                          | `(Participating, None)` — promoted.                                                |
| `Participating` | no                                           | `(Attached, Some(CapabilityDowngrade))` with reason `MissingEnforcementAckMethod`. |

The daemon emits a structured `CapabilityDowngrade` event (`auth.rs:506-518`)
with `requested`, `negotiated`, the reason, and the `advertised_methods`
snapshot captured at downgrade time. The operator-facing log is at WARN — the
operator configured enforcement and is not getting it (`auth.rs:587-596`,
`capability_downgrade_reason_strings_are_kebab_case` pins the wire vocabulary).

**Why the manifest, not `.anvil.yaml`, is the floor:** an LSP client that does
not implement `anvil/enforcement/ack` cannot honour enforcement decisions. If
`.anvil.yaml` could override the manifest, a team-mandated enforcement policy
would silently fence Neovim users whose plugins do not speak the namespace
(`auth.rs:540-547`).

**Reconnect-survival.** The negotiation function reads only the manifest passed
in — no daemon-side state for the negotiation result. A reconnecting driver must
re-present its manifest and the daemon re-runs the gate (`auth.rs:550-556`,
`negotiate_capability_is_pure_recompute`).

**Telemetry identity is daemon-minted, not driver-claimed.**
`correlation.originating_driver_id` is computed from peer credentials
(`crates/anvil-intercept/src/fanout.rs:24-37`, `telemetry.rs:38-44`) — see
`intercept-as-built.md` §5 for the full framing. The reliability-budget ledger
in the TS client uses this daemon-minted id as its key, never `driverName`
(`packages/anvil-driver-client/src/reliability/budget.ts:1-51`).

## 6. Driver client (TypeScript / Node — `packages/anvil-driver-client`)

Published as `@eddacraft/anvil-driver-client` (private, `PROPRIETARY`-licensed;
ESM `"type": "module"`, `packages/anvil-driver-client/package.json:1-37`).
DRVR-001 brief.

### 6.1 What the package exposes

Top-level barrel at `packages/anvil-driver-client/src/index.ts:1-127`:

- `DriverClient` — the public surface (`src/client/driver-client.ts:89-671`).
  Public methods: `connect()`, `request<R>(method, params, options)`,
  `notify(method, params)`, `subscribe(topic, handler)`, `on(event, handler)`,
  `close()`, `validateMidEdit(params)`, `setDriverIdentity(identity)`,
  `reliabilitySnapshot()`.
- `DriverClientOptions` — constructor knobs: `socketPath` / `pipeName`,
  `transportFactory`, `timeoutsMs.{readOnly, enforcementAck}`, `reconnect.*`,
  `reliabilityBudget`, `driverIdentity`, `enforcementAckMethods`, `scheduler`,
  `midEdit` (`src/client/types.ts:114-162`).
- Default constants: `DEFAULT_READ_TIMEOUT_MS = 10_000`,
  `DEFAULT_ENFORCEMENT_ACK_TIMEOUT_MS = 500`,
  `DEFAULT_RECONNECT_INITIAL_MS = 200`, `DEFAULT_RECONNECT_CAP_MS = 30_000`,
  `DEFAULT_RECONNECT_MAX_ATTEMPTS = 5` (`src/client/types.ts:36-46`). Backoff is
  exponential with ±20% jitter; the brief pins this regime
  (`src/client/driver-client.ts:579-602`).
- The full structured-error contract: `DriverClientError`, `DriverError`,
  `DriverErrorCode` — eight stable discriminators, notably
  `anvil-daemon-timeout`, `anvil-daemon-unavailable`,
  `anvil-daemon-transport-drop`, `anvil-daemon-wrong-owner`,
  `anvil-daemon-error`, `anvil-driver-quarantined`, `anvil-driver-closed`
  (`src/errors.ts:21-51`).
- The framing helpers: `buildRequest`, `buildNotification`, `classifyIncoming`,
  `errorFromResponse`, `encodeNdjsonLine`, `NdjsonFramer` (`src/framing/`).
- The transport layer: `UnixSocketTransport`, `WindowsNamedPipeTransport`,
  `defaultTransportFactory`, `resolveDefaultSocketPath`,
  `validateUnixSocketOwnership`, `validateWindowsPipeName` (`src/transport/`).
- The reliability budget: `ReliabilityBudget`, `QUARANTINE_PERSISTENCE_NOTE`
  (`src/reliability/`).
- The diagnostic vocabulary mirror: `Diagnostic`, `DiagnosticLocation`,
  `DiagnosticSource`, `Mode`, `Category`, `Severity`, `KnownMode`,
  `DIAGNOSTIC_SCHEMA_VERSION` (`src/diagnostics/types.ts`).
- The protocol vocabulary mirror: the six `ANVIL_*` constants, `Capability`
  literal union, `CapabilityDowngrade`, `CapabilityDowngradeReason`,
  `DriverManifestSlice`, plus per-method param/result interfaces
  (`AnvilPublishDiagnosticsParams`, `AnvilScanBufferParams`,
  `AnvilScanBufferResult`, `AnvilEnforcementAckParams`,
  `AnvilGateRequestParams`, `AnvilSuppressionApplyParams`,
  `AnvilSuppressionApplyResult`, `AnvilStatusQueryParams`)
  (`src/protocol/types.ts:35-235`).
- The mid-edit helper: `createMidEditValidator`, `MidEditDebouncer`,
  `validateMidEdit` (RTAI-004) — debouncer + content-hash dedup + structured
  error envelope (`src/midedit/`).

### 6.2 Schemas

The package does **not** ship Zod or any schema-validation library. The TS types
are hand-mirrored from the Rust authoritative crate
(`src/protocol/types.ts:1-32`) and pinned by unit tests; CI fails if either side
drifts. The diagnostic surface uses an explicit `DIAGNOSTIC_SCHEMA_VERSION`
literal mirror of the Rust `anvil_kernel_types::diagnostics` schema version
(`src/diagnostics/types.ts`).

### 6.3 Lifecycle, in summary

1. Construct with `DriverClientOptions` (transport defaults, reconnect defaults,
   reliability ledger).
2. `await client.connect()` — opens transport, performs the wrong-owner check
   pre-connect, attaches the framer (`driver-client.ts:170-183, 461-484`).
3. `await client.request(method, params, options)` — allocates a sequential
   `req-N` JSON-RPC id, schedules a timeout (10 s for read-only, 500 ms for
   enforcement-ack methods or when `options.enforcementAck === true`), writes
   the encoded line, and resolves on the matched response or rejects with a
   structured `DriverClientError` (`driver-client.ts:195-224, 386-444`).
4. Subscribe via `client.subscribe(method, handler)` for daemon notifications
   (e.g. `anvil/publishDiagnostics`) (`driver-client.ts:279-301`); event-loop
   hooks via `client.on(...)` (`driver-client.ts:309-329`).
5. `client.close()` is idempotent: cancels in-flight pendings with
   `anvil-driver-closed`, tears down the transport, drops subscribers /
   listeners (`driver-client.ts:336-362`).

Reconnection is transparent. The client schedules an exponential backoff up to
`maxAttempts`, emits `reconnecting` / `reconnect_failed` events, cancels
pendings with `anvil-daemon-transport-drop` (`retriable: true`), and resumes
once the transport is re-bound (`driver-client.ts:554-622`).

### 6.4 Test infrastructure

Vitest 4. Unit tests for every module, plus
`packages/anvil-driver-client/src/__tests__/` for the integration fixtures.
Integration test target is `pnpm test:integration` (`package.json:18`). The
transport seam is mockable via `DriverClientOptions.transportFactory` so the
protocol layer can be exercised without a real daemon.

### 6.5 Build / publish target

`tsc -p tsconfig.lib.json` emits to `./dist`, with `dist` and `README.md` as the
only published artefacts (`package.json:14, 27`). The `main` / `types` entry is
`./dist/index.js` / `./dist/index.d.ts`.

## 7. Driver client (Rust — embedded in proto consumers)

The `query_daemon_status` helper in
`crates/anvil-cli/src/commands/intercept.rs:77-148` is itself a Rust client of
the proto. It is the in-tree reference implementation for how a Rust caller
speaks the protocol without the TS client. The same shape applies to any Rust
consumer (a future `anvil-driver-client-rs` crate, the embedded validation
client, etc.):

1. **Resolve the transport.** Unix:
   `anvil_intercept::ipc::resolve_socket_path()` and
   `validate_socket_path_for_client()` to enforce the 0700 dir / 0600 socket
   ladder before connect (`intercept.rs:87-105`). Windows:
   `anvil_intercept_win32::pipe_name_for_current_user()` then
   `connect_owner_only_pipe_client()` (`intercept.rs:144-148`).
2. **Connect and validate the peer.** Unix:
   `validate_connected_peer_for_client(&stream)` runs `SO_PEERCRED`
   /`getpeereid` after `connect()` (`intercept.rs:112-113`). Windows: the kernel
   ACL on the named pipe is the authoritative gate; defence-in-depth pipe-owner
   validation is intentionally skipped in v1
   (`anvil-intercept-win32/src/lib.rs:114-120`).
3. **Build the frame.** `build_query_status_frame_bytes(method, id)`
   (`intercept.rs:322-331`). Centralised so Unix and Windows cannot drift on
   `jsonrpc` / `version` / `id` semantics — both paths emit
   `{"jsonrpc": "2.0", "method": ..., "id": ...}` + `\n`. The Unix path uses the
   legacy `query_status` method; the Windows path uses `ANVIL_STATUS_QUERY`
   (`intercept.rs:125-126, 240`).
4. **Send + read with timeouts.** A 2 s request timeout on either side; line cap
   at 1 MiB (`intercept.rs:85, 317`).
5. **Parse the response.** `parse_query_status_response_bytes(buf, read)`
   (`intercept.rs:338-371`) checks the JSON-RPC envelope's `jsonrpc: "2.0"` pin,
   the `id` round-trip (`REQUEST_ID = "anvil-cli-intercept-status"`), surfaces
   any `error` field as a hard failure, and deserialises `result` into
   `DaemonStatusV1`.

The error wording is operator-friendly: a missing socket / pipe is rendered as
"anvil intercept daemon is not running" with a "start it with
`anvil intercept start --foreground`" hint (`intercept.rs:94-99`). This is the
demo-runbook §1.5 trust signal.

## 8. `anvil-intercept-rules`

The hot-path rule registry. Owned by INTR
(`plans/archive/modules/intercept-rules.aps.md`, Complete 8/8). The crate's
brief is to keep rule code in one place, dep-light enough that the intercept
daemon can compose `Vec<Box<dyn InterceptRule>>` without pulling in the full
kernel.

### 8.1 What it contains

- `lib.rs` — the `InterceptRule` trait, `RuleInput`, `RuleDecision`,
  `InterruptReason`, `ChangeKind` (`anvil-intercept-rules/src/lib.rs`). The
  trait is **object-safe** by construction: only `&self` methods, no generic
  methods, no associated types, `+ 'static` bound. The registry holds
  `Vec<Box<dyn InterceptRule>>` (`lib.rs:142-217, 286-294`).
- `registry.rs` — `RuleRegistry`, `RegistryDecision`, `RegistryError`,
  `RegistryMode` (INTR-006). Composes rules into an ordered evaluation pipeline
  with **first-interrupt short-circuit** semantics, observe-only mode, **panic
  isolation under unwind builds** via `catch_unwind`, **cached rule-ids** so the
  hot path never re-calls `rule_id()`, and **duplicate-id rejection** at
  registration time (`registry.rs:1-90`).
- `secret.rs` — `SecretDetectionRule` (INTR-002). Wraps
  `anvil_checks::secret::scan_content_with_limit` and emits `Diagnostic`s with
  `Category::Other` and the canonical `secret-detection` rule id
  (`secret.rs:1-50`).
- `reasoning.rs` — `LaunchReasoningPatternRule` (INTR-008). Wraps
  `anvil_checks::reasoning::run_reasoning_check_with_limit` for
  appeal-to-authority detection (`reasoning.rs:1-40`).
- `antipattern.rs` — `AntipatternScanRule` (INTR-003). Wraps
  `anvil_checks::antipattern::scan_file` over borrowed content (no disk read, no
  rayon pool) and interrupts on the first finding at or above the configured
  `severity_threshold` that is not inline-suppressed (`@anvil-ignore`, ADR-029).
  Extension-gated via `config.extensions`; `Removed` changes and below-threshold
  / suppressed findings `Allow`. Rule id `antipattern-scan`
  (`antipattern.rs:43, 81-151`).
- `path_deny.rs` — `PathDenyListRule` (INTR-004). Glob-based (`globset`,
  gitignore-flavoured `**`) deny list, and the only **path-only** rule
  (`needs_content()` is `false`) — so the registry can skip content reads when
  it is the sole rule. Globs compile once at construction; malformed patterns
  surface as `PathDenyError::InvalidGlob`. `Removed` changes `Allow`. Rule id
  `path-deny` (`path_deny.rs:22, 42-61`).
- `regex_content.rs` — `RegexContentRule` (INTR-005). Per-line regex matcher,
  the content counterpart to path-deny. Patterns compile eagerly; empty or
  duplicate patterns are rejected at construction
  (`RegexContentError::InvalidPattern`). **First registered pattern wins**;
  lines are split with `str::lines` (CRLF-clean, unlike the antipattern
  scanner's upstream `split('\n')`). Rule id `regex-content`
  (`regex_content.rs:31, 88-119, 146-184`).
- `config.rs` — INTR-007 rule configuration. Builds a populated `RuleRegistry`
  from the `.anvil.<ext>` `enforcement.intercept-rules` block via
  `registry_from_value` / `registry_from_workspace` (`config.rs:187-200`).
  Defaults (absent file / block): secret detection on, antipattern off, no
  path-deny / regex-content patterns (`config.rs:101-110`). Malformed config —
  including unknown rule keys or a typo inside a per-rule object — is a typed
  `RuleConfigError`, never a silent default (`config.rs:139-156, 212-250`).
  Registration order is fixed: path-deny, secret-detection, antipattern,
  regex-content (`config.rs:164-183`).

### 8.2 Role in the framework

The rules crate is the **library** the daemon links against
(`crates/anvil-intercept/src/enforcement.rs:5-12`, `default_rule_registry` at
`enforcement.rs:95-102`); it is **not** a crate drivers link against. Drivers
consume the daemon's `scan_buffer` / `validate_write` RPC surface and receive
the diagnostics rules emit via `anvil/publishDiagnostics`. From a driver's
perspective, the rule set is opaque — the daemon does composition,
short-circuit, redaction, and emission. The daemon's `default_rule_registry`
composes secret-detection and launch-reasoning today; the INTR-007 config path
(`anvil_intercept_rules::config::registry_from_workspace`) assembles the
antipattern, path-deny, and regex-content rules from `.anvil.<ext>` but is not
yet wired into the daemon's default registry construction.

The crate is therefore part of the framework only by virtue of being the source
of the diagnostic envelopes drivers render. It is owned by the INTR module
(`plans/archive/modules/intercept-rules.aps.md`), separate from DRVR.

### 8.3 Latency contract

The trait-level docstring pins **microseconds to hundreds of microseconds** as
the latency envelope. No graph recomputation, no network calls, no expensive AST
analysis (`lib.rs:170-173`). Out-of- scope items are listed in
`plans/archive/modules/intercept-rules.aps.md` — out-of-band rules ride on a
different evaluator.

### 8.4 Panic policy

The trait says rules MUST NOT panic. The registry enforces that by wrapping
every `evaluate` call in `std::panic::catch_unwind` and treating a panicking
rule as `Allow`. This isolation holds in release builds too: the workspace's
`[profile.release]` sets `panic = "unwind"` (ADR-051, chosen because `anvil`
processes untrusted input and a panic must surface as a structured error rather
than a `SIGABRT`), so `catch_unwind` is not a debug-only safeguard
(`Cargo.toml:156-164`, `registry.rs:20-30`). The trait still asks for panic-free
rules by construction as the long-term answer, but the registry no longer
depends on it for crash-safety.

## 9. Windows path (Win32 named pipe)

`anvil-intercept-win32` (`crates/anvil-intercept-win32/src/lib.rs`,
`#![cfg(windows)]`) keeps every `unsafe` Win32 call out of the daemon crate so
`anvil-intercept` retains `#![forbid(unsafe_code)]`.

Driver-relevant primitives:

- `pipe_name_for_current_user()` — `\\.\pipe\anvil-intercept-<sid>` derived from
  `current_user_sid_string()` (`lib.rs:79-99`). The SID — not the env username —
  is the rendezvous suffix so account-name spoofing cannot move the meeting
  point. Stable across calls within a process
  (`pipe_name_for_current_user_is_stable` at `lib.rs:646-660`). The CLI Windows
  status path resolves the pipe name via this helper
  (`crates/anvil-cli/src/commands/intercept.rs:144-148`).
- `connect_owner_only_pipe_client(pipe_name)` — synchronous `CreateFileW` with
  `OPEN_EXISTING`, `GENERIC_READ | GENERIC_WRITE`,
  `FILE_SHARE_READ | FILE_SHARE_WRITE`, no `FILE_FLAG_OVERLAPPED`
  (`lib.rs:121-146`). Returns `OwnerOnlyPipeClient`, an RAII handle that closes
  via `CloseHandle` on drop. The trust gate is the daemon's named-pipe ACL — the
  kernel rejects cross-SID and cross-host clients before user-space ever sees
  the connection (`lib.rs:114-120`).
- `OwnerOnlyPipeClient::write_all` / `OwnerOnlyPipeClient::read` — synchronous
  `WriteFile` / `ReadFile`, with a u32 byte cap and a short-write check so the
  daemon's per-line JSON-RPC framing relies on the request landing as one frame
  (`lib.rs:175-240`).

The daemon-side bind is
`create_owner_only_pipe_server(pipe_name, PipeInstance::First)` (`lib.rs:50-63`)
— explicit owner-only DACL granting `0x12019f` (deliberately less than
`GENERIC_ALL`, see `OWNER_PIPE_RIGHTS` at `lib.rs:46-47`) plus
`reject_remote_clients(true)`. Cross-link `intercept-as-built.md` §15 for the
full Windows boundary description.

**Wire-format parity with the Unix UDS path** is enforced at the CLI layer via
the shared `build_query_status_frame_bytes` and
`parse_query_status_response_bytes` helpers
(`crates/anvil-cli/src/commands/intercept.rs:319-371`); the only daemon-bound
differences are the JSON-RPC method name and the transport. The integration test
`windows_query_daemon_status_round_trips_against_local_pipe`
(`intercept.rs:622+`) injects a per-test pipe name and round-trips the full
status query.

The TS Windows transport path is in
`packages/anvil-driver-client/src/transport/windows.ts`. It validates that the
supplied `pipeName` matches the `\\.\pipe\anvil-intercept-<sid>` pattern and
refuses anything else as `anvil-daemon-wrong-owner`, but the deeper ACL check is
deferred — documented as a gap rather than a false-comfort check that always
returns OK (`windows.ts:7-28`). The path resolver requires the consumer to pass
an explicit `pipeName` on Windows because Node has no cheap way to fetch the SID
(`transport/path.ts:53-77`).

## 10. Capability negotiation

Capabilities a driver can request:

- `Capability::Attached` — the read-only floor. Subscribes to telemetry, renders
  diagnostics, applies suppression edits. Always granted on a successful
  handshake. Wire form: `attached`. (`protocol.rs:265-292`,
  `protocol/types.ts:99-105`.)
- `Capability::Participating` — enforcement-candidate. Receives
  `enforcement.decision` events; ack-or-refuse contract per the editor design
  §2.5; subject to the reliability budget. Wire form: `participating`.
  (`protocol.rs:265-292`.)

What the daemon refuses:

- A `Participating` request from a manifest that does **not** advertise
  `anvil/enforcement/ack`. Auto-downgrades to `Attached` with
  `MissingEnforcementAckMethod`. Stock LSP clients (Neovim built-in LSP, Zed,
  Helix) hit this path and attach as read-only observers regardless of
  `.anvil.yaml`'s enforcement request (DRVR-008 contract, `auth.rs:558-580`,
  `surface-drivers.aps.md:456-504`).
- A non-empty `workspace_roots` claim with zero matches against the active
  session set (DRVR-007 §2.3a, `auth.rs:412-453`).

The lattice derives `PartialOrd` / `Ord` so callers can compare "is requested
capability higher than what the manifest allows" without re-implementing the
order; v1 only has two states but the comparison is the property the type
checker guarantees (`protocol.rs:265-292`).

## 11. Cross-cutting concerns

### 11.1 Versioning

- `DaemonStatusV1`'s explicit `V1` suffix is the bump signal. Adding new fields
  stays at V1 (`status.rs:152-174` pins the unknown-field-tolerance property). A
  semantic change to an existing field bumps to V2 — at which point the daemon
  must dual-route or consumers must re-pin.
- `LatencyMidEditMapV1` is designed for additive growth: a future `save` /
  `pre_write` / `watch` rollup adds a sibling field without breaking V1
  consumers (`status.rs:70-81`).
- `ALL_ANVIL_METHODS` is the canonical list (`protocol.rs:297-317`; 19 method
  constants as of the DSV + witness + GCTX additions). Adding a method is
  additive; renaming or removing one bumps a protocol version and requires the
  daemon to dual-route during the transition.
- The TS protocol mirror is byte-pinned via tests
  (`packages/anvil-driver-client/src/protocol/types.test.ts`) so a Rust-side
  rename without TS update fails CI before shipping.

### 11.2 Backwards compatibility

The daemon **dual-routes** legacy and canonical method names today:

- `scan_buffer` (legacy, RTAI-002 / RTAI-008 fixtures) and `anvil/scan_buffer`
  (canonical, drivers' manifest advertisement) both route to
  `handle_scan_buffer_jsonrpc` (`crates/anvil-intercept/src/ipc.rs:1810-1837`).
- `query_status` (legacy CLI / 37-fixture conformance suite) and
  `anvil/status/query` (canonical, the Windows CLI path is its first client)
  both route to `handle_query_status_jsonrpc` (`ipc.rs:1839-1865`).

How long? Until every consumer migrates. There is no committed deprecation
window in `v0.6.0-beta`; the design rule is that the canonical name is preferred
for new clients, and removal of the legacy alias is a future protocol-version
bump.

### 11.3 Trust

- Same-UID local IPC only. No remote surface, no TLS, no signed manifests in v1.
  The four HIGH security trade-offs are catalogued in
  `docs/archive/runbooks/v0.6.0-beta-security-note.md` (H1: drivers.allow file
  mode not verified before read; H2: cross-session telemetry redaction hash
  unsalted; H3: §4.4 redaction filter spec-only outside `validate_write`; H4:
  PID-reuse TOCTOU and macOS fence-first interrupt ladder).
- Telemetry identity is daemon-minted: `originating_driver_id` computed from
  peer credentials, never from a driver-supplied `driverName`. Cross-link
  `intercept-as-built.md` §5.

### 11.4 Determinism

The proto carries no clock-dependent fields beyond the explicit timestamps it
declares (`SessionRecord::started_at_unix / last_heartbeat_unix`,
`FenceStateV1::fenced_at_unix`, `HealthStateV1::uptime_seconds`). Status
responses are stable for a given daemon state — the same registry + fence store
produce the same `DaemonStatusV1` byte-for-byte across calls. This is the
property the runbook §2 reset path relies on.

## 12. Spec → code reconciliation

Where the driver design spec at `plans/specs/anvil-driver-framework/`
(specifically `editor-and-mcp-driver-design.md`) and the shipping code agree,
and where they don't:

| Spec promise                                                       | Shipping in `v0.6.0-beta`                                                                                                                                                                                                                                                                               | Code reference                                                                                                          |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| §2.3a allowlist gate, default `~/.config/anvil/drivers.allow`      | Shipped                                                                                                                                                                                                                                                                                                 | `crates/anvil-intercept/src/auth.rs:227-267`                                                                            |
| §2.3a workspace-root validation against live sessions              | Shipped                                                                                                                                                                                                                                                                                                 | `auth.rs:412-453`                                                                                                       |
| §3.2 `anvil/` method name table                                    | Shipped: 6 of the 14 editor-driver spec methods. `ALL_ANVIL_METHODS` now carries **19** constants total — the other 13 are the three DSV save-time verbs, `anvil/witness/append`, and the nine GCTX read-only verbs, which sit outside the editor-driver §3.2 table (see `intercept-as-built.md` §4.3). | `crates/anvil-intercept-proto/src/protocol.rs:117-150` (driver-facing six); `protocol.rs:297-317` (`ALL_ANVIL_METHODS`) |
| §3.2 `anvil/driver/capabilities/update` (Attached → Participating) | **Spec-only.** No method constant, no daemon route.                                                                                                                                                                                                                                                     | n/a                                                                                                                     |
| §3.2 `anvil/capability/downgrade` notification                     | **Spec-only as a method.** Downgrades emit a structured `tracing::warn` log; no JSON-RPC notification ships.                                                                                                                                                                                            | `auth.rs:587-596`                                                                                                       |
| §3.2 `anvil/enforcement/decision` notification                     | **Spec-only.** No method constant.                                                                                                                                                                                                                                                                      | n/a                                                                                                                     |
| §3.2 `anvil/enforcement/refuse`                                    | **Spec-only as a daemon constant.** TS client treats it as enforcement-ack-class for timeout purposes only.                                                                                                                                                                                             | `packages/anvil-driver-client/src/client/types.ts:60-63`                                                                |
| §3.2 `anvil/suppression/state` notification                        | **Spec-only.** No method constant.                                                                                                                                                                                                                                                                      | n/a                                                                                                                     |
| §3.2 `anvil/gate/result` snapshot notification                     | **Spec-only.** No method constant.                                                                                                                                                                                                                                                                      | n/a                                                                                                                     |
| §3.2 `anvil/nudge/metadata`                                        | **Spec-only.** No method constant.                                                                                                                                                                                                                                                                      | n/a                                                                                                                     |
| §3.2 `anvil/correlation` (per-diagnostic embedded)                 | **Spec-only.** Diagnostic envelopes carry correlation via `anvil-kernel-types`; no `anvil/correlation` method.                                                                                                                                                                                          | n/a                                                                                                                     |
| §3.3 Capability state machine (`Attached` ↔ `Participating`)       | Shipped: lattice + negotiation; transitions emitted via downgrade event                                                                                                                                                                                                                                 | `protocol.rs:265-292`, `auth.rs:558-580`                                                                                |
| §3.3 reconnect-survival (negotiation re-runs from manifest)        | Shipped                                                                                                                                                                                                                                                                                                 | `auth.rs:550-556` and the `negotiate_capability_is_pure_recompute` test                                                 |
| §4.4 redaction contract (secret excerpts, absolute paths)          | **Wired only for `validate_write`.** Other MCP tool surfaces (`scan.files`, `fix.apply`, `status.query`) are spec-only. RMCPF-010 wires the rest in a later tag.                                                                                                                                        | `crates/anvil-cli/src/mcp/tools/validate_write.rs:374-424`; cross-link `intercept-as-built.md` §13 + security note H3   |
| §4.5 degraded-behaviour structured error                           | Shipped (TS client + MCP shim use the same code names)                                                                                                                                                                                                                                                  | `packages/anvil-driver-client/src/errors.ts:21-51`                                                                      |
| DRVR Wave 1-3 (DRVR-001/-002/-006/-007/-008)                       | Shipped (PRs #1304, #1307, #1310; remediation #1322)                                                                                                                                                                                                                                                    | `plans/archive/modules/surface-drivers.aps.md:11-13`                                                                    |
| DRVR Wave 4 (RTAI-005, RTAI-007, RTAI-009, DRVR-003)               | **Deferred under ADR-033.** VSCode extension archived; Wave 4 sits behind an extension un-pause decision.                                                                                                                                                                                               | `RELEASE-PLAN.md:77, 263-264`; `plans/decisions/033-park-ide-mcp-retire-ts-scanner.md`                                  |
| Reliability-budget quarantine survives daemon restart              | **Client-side only (in-process).** TS ledger persists across reconnect within a process; cross-process / on-disk ledger is documented but not implemented.                                                                                                                                              | `packages/anvil-driver-client/src/reliability/budget.ts:78-92`                                                          |
| Driver-version negotiation that survives daemon restart            | **Not addressed.** v1 is single-version daemon — no rolling-upgrade story, no peer compatibility matrix.                                                                                                                                                                                                | `intercept-as-built.md` §16 gap 10                                                                                      |

## 13. Known gaps (dated 2026-05-07)

1. **DRVR Wave 4 deferred.** RTAI-005 (editor mid-edit path), RTAI-007
   (telemetry mirror), RTAI-009 (architecture doc + ADR cross-links), and
   DRVR-003 (VSCode editor driver) are out of cut per ADR-033 — the IDE/MCP
   surfaces are archived (`anvil-archive/anvil-vscode-extension/`,
   `anvil-archive/anvil-mcp-server/`). DRVR-003 stays deferred until a new
   extension package is created on the daemon-driver path; the rest of DRVR
   (DRVR-001, -002, -005) continues against existing INTD dependencies. Ref:
   `plans/decisions/033-park-ide-mcp-retire-ts-scanner.md`,
   `plans/archive/modules/surface-drivers.aps.md:17-29`.
2. **Driver-version negotiation that survives daemon restart not addressed.** v1
   is a single-version daemon; no rolling upgrade story, no peer compatibility
   matrix, no driver-version negotiation that survives daemon restart. Operators
   should run a single tagged binary on a single user account. Cross-link
   `intercept-as-built.md` §16 gap 10.
3. **§4.4 redaction filter is spec-only outside `validate_write`.** Wired filter
   today is `crates/anvil-cli/src/mcp/tools/validate_write.rs:374-424`;
   `scan.files`, `fix.apply`, and `status.query` ship un-redacted absolute paths
   and un-redacted secret-rule excerpts in v1. RMCPF-010 wires the runtime
   parity in the next tag. Cross-link `intercept-as-built.md` §16 gap 6,
   security note H3.
4. **Spec-only Anvil methods.** Eight `anvil/`-namespaced methods the editor
   design names in §3.2 do not have proto-crate constants yet
   (`anvil/driver/capabilities/update`, `anvil/capability/downgrade`,
   `anvil/enforcement/decision`, `anvil/enforcement/refuse`,
   `anvil/suppression/state`, `anvil/gate/result`, `anvil/nudge/metadata`,
   `anvil/correlation`). They land with the consumer that needs them — DRVR's
   "no new method without a concrete editor feature" rule prevents speculative
   additions (`protocol.rs:28-75`, the method-namespace-policy module doc). The
   DSV save-time verbs, `anvil/witness/append`, and the nine GCTX read-only
   verbs that have since landed each cleared that rule by shipping with a
   concrete daemon consumer; these eight editor methods have not.
5. **`drivers.allow` file mode not verified before read.** Same gap as
   `intercept-as-built.md` §16 gap 4 / security note H1: the allowlist read path
   uses `fs::read_to_string` without `lstat` for owner / mode (`auth.rs:228`).
   Operators must manually `chmod 0600 ~/.config/anvil/drivers.allow`. Tracked
   for the next tag.
6. **TS Windows transport defers ACL inspection.** `validateWindowsPipeName`
   confirms only that the supplied pipe name follows the daemon's
   `\\.\pipe\anvil-intercept-<sid>` pattern; the deeper ACL check is deferred
   alongside the INTD-012 Windows CI matrix work
   (`packages/anvil-driver-client/src/transport/windows.ts:7-28`). The daemon's
   listener-side ACL is the authoritative gate.
7. **Driver-client reliability ledger is in-process only.** `ReliabilityBudget`
   retains failures across reconnects within one process but does not persist
   across process boundaries. The on-disk ledger schema is documented as
   `QUARANTINE_PERSISTENCE_NOTE`
   (`packages/anvil-driver-client/src/reliability/budget.ts:78-92`) but not
   implemented. A consumer that spawns a fresh process per workspace will not
   carry quarantine state.
8. **Reliability-budget pre-handshake gap.** Pre-handshake failures (before
   `correlation.originating_driver_id` is observed) are deliberately uncounted
   to avoid a self-DoS where a flapping daemon drives the client to permanently
   quarantine itself (`budget.ts:21-26`). Operators triaging "driver thinks
   daemon is fine but every request fails" should check the structured error
   stream, not the reliability snapshot.
9. **Capability-downgrade event is log-only on the wire.** The daemon emits a
   structured `tracing::warn` event when a downgrade fires (`auth.rs:587-596`),
   but there is no JSON-RPC notification shipped to the driver client today. The
   TS client surface `CapabilityDowngrade` type is wired for when DRVR-001's
   full handshake decoder lands and the daemon emits the event over
   `anvil/capability/downgrade`
   (`packages/anvil-driver-client/src/protocol/types.ts:107-133`).

## 14. Source references

### `crates/anvil-intercept-proto/src/`

| File                    | Role                                                                                                                             |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `lib.rs`                | `SessionId`, `IpcCommand`, `IpcEnvelope`, `SessionRecord`, `SessionStatus`. The wire vocabulary.                                 |
| `protocol.rs`           | `anvil/`-namespaced JSON-RPC method-name constants and the `Capability` lattice. Authoritative across both Rust and TS.          |
| `status.rs`             | `DaemonStatusV1` and friends — wire shape for `query_status` / `anvil/status/query`.                                             |
| `enforcement_config.rs` | `AnvilConfigFile` / `EnforcementConfigFile` / `DosConfigFile` / `TelemetryConfigFile` decoders. Shared by INTD-008 and RTAI-006. |

### `crates/anvil-intercept-rules/src/`

| File               | Role                                                                                                                                                                                                                    |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lib.rs`           | `InterceptRule` trait, `RuleInput`, `RuleDecision`, `InterruptReason`, `ChangeKind`. Object-safe by construction.                                                                                                       |
| `registry.rs`      | `RuleRegistry`, `RegistryDecision`, `RegistryError`, `RegistryMode`. First-interrupt short-circuit, `catch_unwind` panic isolation (effective in release: workspace `panic="unwind"`, ADR-051), duplicate-id rejection. |
| `secret.rs`        | `SecretDetectionRule` — wraps `anvil_checks::secret`.                                                                                                                                                                   |
| `reasoning.rs`     | `LaunchReasoningPatternRule` — appeal-to-authority detector.                                                                                                                                                            |
| `antipattern.rs`   | `AntipatternScanRule` (INTR-003) — wraps `anvil_checks::antipattern::scan_file`; severity threshold + `@anvil-ignore` suppression honoured.                                                                             |
| `path_deny.rs`     | `PathDenyListRule` (INTR-004) — glob deny list; the only path-only rule (`needs_content()` is `false`).                                                                                                                 |
| `regex_content.rs` | `RegexContentRule` (INTR-005) — per-line regex matcher; eager-compiled, CRLF-clean.                                                                                                                                     |
| `config.rs`        | INTR-007 — `InterceptRulesConfig`, `registry_from_value`, `registry_from_workspace`; builds a registry from `enforcement.intercept-rules`.                                                                              |

### `packages/anvil-driver-client/src/`

| Path                           | Role                                                                                                                                                       |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `index.ts`                     | Public barrel; the only file consumers import from.                                                                                                        |
| `client/driver-client.ts`      | `DriverClient` class — `connect`, `request`, `notify`, `subscribe`, `on`, `close`, `validateMidEdit`, `setDriverIdentity`, `reliabilitySnapshot`.          |
| `client/types.ts`              | `DriverClientOptions`, `DriverRequestOptions`, `DriverClientEventMap`, `DriverNotificationTopics`, default constants.                                      |
| `errors.ts`                    | `DriverError`, `DriverErrorCode`, `DriverClientError`, `driverError`, `mapDaemonErrorRetriable`.                                                           |
| `framing/jsonrpc.ts`           | `buildRequest`, `buildNotification`, `classifyIncoming`, `errorFromResponse`, `encodeNdjsonLine`.                                                          |
| `framing/ndjson.ts`            | `NdjsonFramer` with discard-don't-crash, oversize-line cap, UTF-8 fatal, partial-frame-on-reset.                                                           |
| `transport/index.ts`           | `defaultTransportFactory` — auto-selects per platform.                                                                                                     |
| `transport/path.ts`            | `resolveDefaultSocketPath`, `PathResolutionError`. Mirrors the daemon-side resolver.                                                                       |
| `transport/unix.ts`            | `UnixSocketTransport`, `validateUnixSocketOwnership` — pre-connect 0700/0600 / current-uid stat ladder.                                                    |
| `transport/windows.ts`         | `WindowsNamedPipeTransport`, `validateWindowsPipeName` — pipe-name pattern check; ACL deferred.                                                            |
| `transport/types.ts`           | `Transport`, `TransportFactory`, `TransportHandlers`, `TransportCloseCause`.                                                                               |
| `reliability/budget.ts`        | `ReliabilityBudget` ledger, `QUARANTINE_PERSISTENCE_NOTE`. Daemon-minted-id keyed.                                                                         |
| `protocol/types.ts`            | TS mirror of `anvil-intercept-proto::protocol`. Constants, `Capability`, `CapabilityDowngrade`, `DriverManifestSlice`, per-method param/result interfaces. |
| `diagnostics/types.ts`         | TS mirror of `anvil_kernel_types::diagnostics`.                                                                                                            |
| `midedit/validate-mid-edit.ts` | RTAI-004 mid-edit helper: scan-buffer wire shape + structured error envelope.                                                                              |
| `midedit/debouncer.ts`         | `MidEditDebouncer` — debounce + content-hash dedup + scheduler injection.                                                                                  |

### `crates/anvil-intercept-win32/src/` (the parts the driver client uses)

| File     | Role                                                                                                                                                                                                                                                                     |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `lib.rs` | `pipe_name_for_current_user`, `current_user_sid`, `connect_owner_only_pipe_client`, `OwnerOnlyPipeClient`, `create_owner_only_pipe_server` (daemon-side bind), `process_creation_time`, `JobObject`, `terminate_job_object`. The Windows boundary; all `unsafe` is here. |

## 15. Related docs

- `docs/architecture/intercept-as-built.md` — primary cross-reference; the
  daemon side that consumes the proto. §4 (IPC), §5 (auth/trust), §13 (CLI
  surface), §15 (Win32 listener) are the cross-linked sections.
- `docs/architecture/mcp-shim-as-built.md` — MCP shim is one driver consumer;
  speaks `scan_buffer` directly to the daemon as a daemon-backed validation
  client.
- `plans/specs/anvil-driver-framework/` — design spec the framework implements
  (`anvil-driver-framework-design-spec.md` for the broader framework,
  `editor-and-mcp-driver-design.md` for the editor / MCP slice this doc
  reconciles against, `anvil-driver-framework-adr.md` for the ADR record).
- `plans/specs/2026-05-06-editor-driver-protocol.md` — DRVR-002's editor-driver
  protocol design doc; the authoritative source for the §3.2 method table
  referenced from the proto crate.
- `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` §2 — operator status
  path, cross-platform parity contract, MCP correlation envelope Windows gap.
- `docs/archive/runbooks/v0.6.0-beta-security-note.md` — security trade-offs; H1
  (drivers.allow file mode), H2 (telemetry redaction hash unsalted), H3 (§4.4
  redaction spec-only), H4 (PID-reuse + macOS fence-first).
- `plans/archive/modules/surface-drivers.aps.md` — DRVR module plan; Wave 1-3
  task records and Wave 4 deferral.
- `plans/archive/modules/intercept-rules.aps.md` — INTR module plan; the rules
  crate's task list.
- `plans/decisions/030-surface-drivers-supersede-napi-cutover.md` — ADR-030, the
  authority that made surfaces drivers rather than napi consumers.
- `plans/decisions/033-park-ide-mcp-retire-ts-scanner.md` — ADR-033, the
  authority that defers Wave 4.
- `docs/public/anvil/integrations/mcp.md` — public-side description of the MCP
  path that uses the proto.
