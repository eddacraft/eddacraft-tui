# Editor-Driver Protocol — JSON-RPC Methods, Notifications, Capability Handshake

**Status:** Draft
**Date:** 2026-05-06
**APS:** DRVR-002 / DRVR-008
**Consumers:** DRVR-001 (TS client), DRVR-003 (deferred VSCode driver), RMCPF
(parity port). Telemetry handoff to INTD-013 / INTD-015 / RTAI-007.
**Source-of-truth siblings:**
[`anvil-driver-framework/editor-and-mcp-driver-design.md`](./anvil-driver-framework/editor-and-mcp-driver-design.md),
[`2026-04-26-diagnostic-envelope-coordination.md`](./2026-04-26-diagnostic-envelope-coordination.md),
[`anvil-driver-framework/anvil-driver-framework-design-spec.md`](./anvil-driver-framework/anvil-driver-framework-design-spec.md).

> **Inner-shape rule.** This protocol carries the canonical
> [`anvil.diagnostic.v1`][diag] payload defined in
> `crates/anvil-kernel-types/src/diagnostics.rs`. The inner shape is
> locked. Protocol-specific metadata (URI, document version, ack id,
> correlation id) lives on the OUTER JSON-RPC envelope —
> `params` / `result` — never on `Diagnostic` itself.

[diag]: ../specs/2026-04-26-diagnostic-envelope-coordination.md#canonical-inner-shape-diagnostic

---

## 0. What this document is

The editor-and-mcp design spec covers the conceptual shape of editor and MCP
drivers (lifecycle, manifest, identity, capability stages, redaction). DRVR-002
pins the JSON-RPC wire vocabulary that turns those concepts into a working
daemon ↔ driver protocol:

- The exact list of methods and notifications the protocol declares.
- Their parameter and result shapes.
- The state machine for driver capability and the rules for transitioning
  between states.
- Failure modes (daemon drop, transport timeout, malformed frame) and the
  protocol's response.
- The mapping between LSP primitives and Anvil primitives.

This doc resolves the seven council-review items the DRVR-002 task body lists
(M2, M3, M4, M6, M12, S6, S7). Each resolution is called out in the section it
belongs to and cross-referenced in §10 ("Council resolution map").

The protocol ships in two halves:

- **DRVR-002 (this doc + Rust + TS bindings):** method names, capability
  handshake, parameter shapes, state machine.
- **DRVR-008 (negotiation enforcement):** `supported_anvil_methods` manifest
  field and the `negotiate_capability` daemon-side gate that prevents a stock
  LSP client from being silently fenced.

Both halves land together because DRVR-008 extends DRVR-002's handshake; a
driver that does not advertise `anvil/enforcement/ack` cannot reach
`Participating` regardless of `.anvil.yaml`. Splitting them across releases
would create a window where stock LSP clients are fenceable.

---

## 1. Transport recap

JSON-RPC 2.0 over NDJSON, framed per
[INTD-014](../modules/intercept-daemon.aps.md#intd-014). Transport selection:

- Linux / macOS: Unix domain socket at `$XDG_RUNTIME_DIR/anvil/intercept.sock`
  (or operator-overridden via `ForegroundOpts::with_pid_file_and_ipc_socket`).
- Windows: named pipe at `\\.\pipe\anvil-intercept-{user}`.
- TCP-localhost: deliberately not in v1. (See §6.1 for the open-question
  resolution.)

Every frame carries `jsonrpc: "2.0"` and is one of:

- A **request** with `id` set and `method` + `params`.
- A **notification** with `id` absent (or wire-level `null` per
  `IpcEnvelope::notification`) and `method` + `params`.
- A **response** with `id` set, plus exactly one of `result` or `error`.

INTD-014's conformance suite (33 tests) is the source of truth for envelope
edge cases (batch, null id, malformed frame). This doc layers method semantics
on top.

---

## 2. Method list

The full table — both directions, both languages over the same transport.

### 2.1 LSP methods (subset — pinned by the LSP spec, not re-specified)

| Method                              | Direction        | Role in Anvil                                                   |
|-------------------------------------|------------------|-----------------------------------------------------------------|
| `initialize` / `initialized`        | client → server  | Light wrapper around the driver-manifest handshake (§4.1)       |
| `textDocument/publishDiagnostics`   | server → client  | LSP-shape diagnostics for clients that cannot consume `anvil/`  |
| `textDocument/codeAction`           | bidirectional    | "Rethink" nudge + "Suppress with reason" actions                |
| `workspace/applyEdit`               | server → client  | Used by deterministic fixers + suppression-comment insertion    |
| `window/showMessage`                | server → client  | Surface enforcement state changes (fence applied, quarantined)  |

The LSP methods are pinned by the LSP spec; the table above is descriptive, not
authoritative. Anvil does not extend or restrict LSP semantics for these.

### 2.2 Anvil methods (`anvil/` namespace — authoritative below)

| Method                              | Direction       | Section | Wire-string constant            |
|-------------------------------------|-----------------|---------|---------------------------------|
| `anvil/publishDiagnostics`          | server → client | §3.1    | `ANVIL_PUBLISH_DIAGNOSTICS`     |
| `anvil/scan_buffer`                 | client → server | §3.2    | `ANVIL_SCAN_BUFFER`             |
| `anvil/enforcement/ack`             | client → server | §3.3    | `ANVIL_ENFORCEMENT_ACK`         |
| `anvil/gate/request` *(M3 resolved)*| client → server | §3.4    | `ANVIL_GATE_REQUEST`            |
| `anvil/suppression/apply`           | client → server | §3.5    | `ANVIL_SUPPRESSION_APPLY`       |
| `anvil/status/query`                | client → server | §3.6    | `ANVIL_STATUS_QUERY`            |

> **M3 resolved.** `anvil/gate/request` is now in the method table (it was
> previously referenced in §3.7 of the editor-and-mcp design spec but missing
> from §3.2). The wire-string constants live in
> `crates/anvil-intercept-proto/src/protocol.rs` (Rust authoritative) and
> `packages/anvil-driver-client/src/protocol/types.ts` (TS mirror, byte-for-byte).

The design rule from §3.2 of the parent spec is preserved verbatim: **no new
`anvil/` method without a concrete editor feature that cannot be expressed in
LSP**.

### 2.3 Server → client notifications

In addition to `textDocument/publishDiagnostics` (LSP) and
`anvil/publishDiagnostics` (this protocol), the daemon emits:

| Notification                       | Section | Purpose                                                            |
|------------------------------------|---------|--------------------------------------------------------------------|
| `anvil/capability/downgrade`       | §4.4    | DRVR-008 structured warning when participation is refused          |
| `anvil/enforcement/decision`       | §3.3    | Daemon → participating driver enforcement-decision payload         |

`enforcement/decision` is sibling to the `anvil/enforcement/ack` request: the
daemon notifies; the driver acks via the request method.

---

## 3. Method semantics

Each subsection lists the canonical params and result shape, the daemon-side
authority, and how the method couples to the inner `Diagnostic` envelope.
Field names are snake_case to match the Rust serde default; the TS mirror
preserves snake_case identifiers exactly so a driver in Lua / Helix / Zed
spells the same field name.

### 3.1 `anvil/publishDiagnostics` (notification, server → client)

Server-emitted notification carrying `Diagnostic[]` for a document.

```jsonc
{
  "jsonrpc": "2.0",
  "method": "anvil/publishDiagnostics",
  "params": {
    "uri": "file:///workspace/src/api/client.ts",
    "version": 17,
    "diagnostics": [
      // canonical anvil.diagnostic.v1; mode = "save-time" or "mid-edit"
    ]
  }
}
```

- `uri` — `file://` URI matching the LSP convention so a driver can route the
  notification into the right buffer.
- `version` — optional document version; drivers drop stale diagnostics by
  comparing to their in-flight buffer version (LSP convention).
- `diagnostics` — array of canonical `Diagnostic` payloads. Rust importers MUST
  use `anvil_kernel_types::Diagnostic`; TS importers MUST use the type
  re-exported from `@eddacraft/anvil-driver-client`.

The notification is fire-and-forget — the daemon does not buffer it for
acknowledgement. Drivers in `Participating` ack the inbound enforcement
decision (§3.3), not the diagnostic stream.

### 3.2 `anvil/scan_buffer` (request, client → server)

Mid-edit buffer scan. Same handler as the existing un-namespaced `scan_buffer`
method (RTAI-002 substrate); the namespaced alias is what drivers advertise in
the §2.2 manifest so the capability-negotiation layer (DRVR-008) can confirm
both ends speak the namespaced form.

```jsonc
// Request
{
  "jsonrpc": "2.0",
  "id": "req-42",
  "method": "anvil/scan_buffer",
  "params": {
    "path": "src/api/client.ts",
    "text": "const config = { api_key: 'AKIA...' };\n",
    "version": 17,
    "mode": "mid-edit"
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "req-42",
  "result": {
    "version": 17,
    "diagnostics": [ /* anvil.diagnostic.v1, mode: "mid-edit" */ ],
    "truncated": false
  }
}
```

- `version` echo — drivers drop stale replies whose `version` is older than
  the buffer's current version.
- `truncated` — daemon sets to `true` when the result was capped by the
  per-scan diagnostic budget (INTD-016). Drivers SHOULD surface a "more
  diagnostics suppressed" hint rather than silently rendering only the cap.
- `mode` is fixed to `"mid-edit"`; future modes (e.g. `pre-write`) will get
  their own request method to keep `scan_buffer`'s contract narrow.

### 3.3 `anvil/enforcement/ack` (request, client → server) + `anvil/enforcement/decision` (notification, server → client)

The two-step contract from §2.5 of the editor-and-mcp design.

```jsonc
// Server → client (notification): enforcement decision targets THIS driver.
{
  "jsonrpc": "2.0",
  "method": "anvil/enforcement/decision",
  "params": {
    "decision_id": "dec_01HX...",
    "correlation_id": "corr_01HX...",
    "decision": "block",
    "rule_id": "secret-aws-access-key",
    "diagnostics": [ /* anvil.diagnostic.v1 */ ]
  }
}

// Client → server (request): driver acks (or refuses) the decision.
{
  "jsonrpc": "2.0",
  "id": "ack-99",
  "method": "anvil/enforcement/ack",
  "params": {
    "decision_id": "dec_01HX...",
    "correlation_id": "corr_01HX..."
  }
}

// Server → client (response): bookkeeping confirmation.
{
  "jsonrpc": "2.0",
  "id": "ack-99",
  "result": { "recorded": true }
}
```

A refusal is a separate method `anvil/enforcement/refuse` (params: same shape
plus a `reason` string). The protocol exposes both so daemon can distinguish
"driver chose not to act" from "driver did not respond" (§2.5: timeout is
treated as refuse).

**DRVR-008 gate.** A driver can RECEIVE `anvil/enforcement/decision` only when
the daemon has placed it in `Participating`. The negotiation that decides this
is in §4.

### 3.4 `anvil/gate/request` (request, client → server)

Asks the daemon for a gate-result snapshot or stream. Pins the §3.7 reference.

```jsonc
// Request
{
  "jsonrpc": "2.0",
  "id": "gate-1",
  "method": "anvil/gate/request",
  "params": {
    "workspace_root": "/home/dev/repo",
    "profile": "ai"
  }
}

// Response (one-shot)
{
  "jsonrpc": "2.0",
  "id": "gate-1",
  "result": {
    "schema": "anvil.gate-result.v1",
    "exit_code": 0,
    "diagnostics": [ /* anvil.diagnostic.v1, mode: "gate" */ ]
  }
}
```

`profile` mirrors the `--profile` flag on `anvil gate`. Daemon-side, this
method routes to the same `gate.run` path the CLI uses; the result wrapper is
the canonical `anvil.gate-result.v1` outer envelope (return-value form per the
diagnostic-envelope spec).

Streaming form: the daemon emits subsequent `anvil/gate/result` notifications
on the telemetry lane keyed on the request's `id` so a driver can render
progress. The `streaming` field on the response (`true` / `false`) tells the
driver whether to expect follow-up notifications.

### 3.5 `anvil/suppression/apply` (request, client → server)

Daemon validates and normalises the proposed `@anvil-ignore` comment per
ADR-004; driver applies the returned comment via `workspace/applyEdit`.

```jsonc
// Request
{
  "jsonrpc": "2.0",
  "id": "supp-1",
  "method": "anvil/suppression/apply",
  "params": {
    "file": "src/api/client.ts",
    "rule_id": "secret-aws-access-key",
    "reason": "test fixture only",
    "scope": { "kind": "line", "line": 42 }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "supp-1",
  "result": {
    "comment": "// @anvil-ignore secret-aws-access-key — test fixture only",
    "anchor": { "file": "src/api/client.ts", "line": 42 }
  }
}
```

`scope.kind` accepts `line | block | file` matching ADR-004's vocabulary. The
daemon owns the suppression contract; the editor is a pure UI per §3.6 of the
parent design.

### 3.6 `anvil/status/query` (request, client → server)

Single-shot view of the daemon's authoritative state for a worktree.

```jsonc
// Request
{
  "jsonrpc": "2.0",
  "id": "status-1",
  "method": "anvil/status/query",
  "params": { "workspace_root": "/home/dev/repo" }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "status-1",
  "result": {
    "session": { "id": "sess_01HX...", "active": true },
    "fence": { "fenced": false },
    "drivers": [
      {
        "id": "driver_01HX...",
        "type": "editor",
        "capability": "attached"
      }
    ]
  }
}
```

Workspace-relative paths and daemon-minted ids only — see DRVR-007's redaction
contract; absolute paths are not crossed by status responses.

---

## 4. Capability handshake & state machine

This section addresses the **M2** council finding (fail-soft vs
enforcement-participating contradiction) and the **M4** finding (multi-window
fan-out), and pins the DRVR-008 negotiation contract.

### 4.1 Manifest (§2.2 slice extended for DRVR-008)

The handshake's first frame is the §2.2 manifest. DRVR-008 extends it with one
field:

```ts
type DriverManifest = {
  driverType: 'editor' | 'mcp' | /* ... */;
  driverName: string;
  driverVersion: string;
  protocolVersion: number;
  capabilities: {
    read: boolean;
    enforcementCandidate: boolean;
    supportedDecisions: Array<'warn' | 'block' | 'interrupt'>;
  };
  context: {
    host: HostDescriptor;
    pid: number;
    startedAt: string;       // ISO 8601
    workspaceRoots: string[];
  };
  auth: { method: 'uds-peer' | 'token'; token?: string };
  // DRVR-008 (new):
  supported_anvil_methods: string[];  // wire-format snake_case
};
```

The Rust slice that the daemon's auth module reads is in
`crates/anvil-intercept/src/auth.rs::DriverManifest`. DRVR-001 will own the
full §2.2 decoder; the slice carries the two fields the daemon's trust
boundary actually consults today (`workspace_roots` for INTD-015 scoping;
`supported_anvil_methods` for DRVR-008 capability negotiation). Both names use
snake_case on the wire so the Rust serde default applies cleanly.

`supported_anvil_methods` lists the `anvil/` JSON-RPC methods the driver
implements. An empty list models a stock LSP client speaking only the LSP
subset; this is a legitimate value, not a bug.

### 4.2 Capability lattice (§3.3 expanded)

```
            ┌──────────┐     ┌────────────────┐
Handshake → │ Attached │ ──► │ Participating  │
            └──────────┘     └────────────────┘
                 ▲                   │
                 └───── downgrade ◄──┘
                       (DRVR-008)
                 │
                 │  daemon-drop grace expires
                 ▼
              Degraded (driver-local UX state, see §5)
```

States — both serialise as kebab-case `attached` / `participating` for the
wire vocabulary:

- **`attached`** — read-only floor. Subscribes to telemetry, renders
  diagnostics, applies suppressions. Default after successful handshake.
- **`participating`** — enforcement-candidate. Receives
  `anvil/enforcement/decision` events; ack-or-refuse contract per §2.5;
  subject to the reliability budget §2.6.

Promotion to `participating` requires BOTH:

1. The DRVR-007 allowlist gate (`is_driver_allowed`) — the driver binary path
   is on `~/.config/anvil/drivers.allow`.
2. The DRVR-008 method advertisement — the manifest's
   `supported_anvil_methods` includes `anvil/enforcement/ack`.

Each transition is guarded:

| Transition                         | Guard                                                                              |
|------------------------------------|------------------------------------------------------------------------------------|
| Unbound → Handshake                | Transport connect; SO_PEERCRED matches daemon UID                                  |
| Handshake → Attached               | Manifest valid; workspace_roots non-empty (`AuthError::NoWorkspaceRootsClaimed`)   |
| Attached → Participating           | DRVR-007 allowlist passes AND DRVR-008 method advertised                           |
| Participating → Attached           | Reliability budget exceeded OR driver requests downgrade OR manifest re-validation |
| Any → Detached                     | Clean shutdown OR transport drop OR daemon revoke                                  |
| Attached / Participating → Degraded| Heartbeat lost past grace window (§5)                                              |

`Degraded` is a driver-local UX state, not a daemon-side capability. The daemon
does not see it; the driver renders it when it loses heartbeat. See §5.

### 4.3 Negotiation function (DRVR-008)

The daemon-side gate is `negotiate_capability` in
`crates/anvil-intercept/src/auth.rs`:

```rust
pub fn negotiate_capability(
    requested: Capability,
    manifest: &DriverManifest,
) -> (Capability, Option<CapabilityDowngrade>);
```

Contract:

- `requested == Attached` → returns `(Attached, None)`. The read-only floor is
  always available.
- `requested == Participating`:
  - Manifest advertises `anvil/enforcement/ack` → returns
    `(Participating, None)`.
  - Manifest does NOT advertise it → returns
    `(Attached, Some(CapabilityDowngrade { reason: MissingEnforcementAckMethod, ... }))`.

When a downgrade fires the daemon emits a structured `tracing::warn` log AND
sends an `anvil/capability/downgrade` notification (§4.4) so the driver can
render a status surface telling the operator why enforcement was refused.

**Why the manifest is the floor, not `.anvil.yaml`.** This is the central
DRVR-008 property: an LSP client speaking only stock LSP cannot honour
`anvil/enforcement/ack`. If the workspace config could override the manifest,
team-mandated enforcement would silently fence Neovim users whose plugins do
not implement the namespace. The manifest is what the driver itself signed up
for; `.anvil.yaml` decides *whether to request* enforcement from drivers that
can support it.

**Reconnect survival.** `negotiate_capability` is a pure function of `(request,
manifest)`. Two calls with the same inputs produce the same outputs; there is
no daemon-side state for the negotiation result. A reconnecting driver MUST
re-present its manifest, and the daemon MUST re-run negotiation. A driver
cannot smuggle a stale `Participating` capability across a reconnect by
relying on the daemon to remember the previous handshake.

**Identity is daemon-minted.** The reliability-budget quarantine ledger
(DRVR-001 / Wave 2) keys on the daemon's mint of `originating_driver_id`
(SO_PEERCRED + binary path), not on the self-declared `driverName`. A hostile
peer cannot reset its own quarantine by reconnecting under a different name.

### 4.4 Capability-downgrade notification

```jsonc
{
  "jsonrpc": "2.0",
  "method": "anvil/capability/downgrade",
  "params": {
    "requested": "participating",
    "negotiated": "attached",
    "reason": "missing-enforcement-ack-method",
    "advertised_methods": ["anvil/publishDiagnostics", "anvil/scan_buffer"]
  }
}
```

The reason is one of two kebab-case strings:

- `not-enforcement-candidate` — driver advertised it does not want
  enforcement; daemon honours that. (Not a fault; informational.)
- `missing-enforcement-ack-method` — DRVR-008 central case.

The driver's status surface (status bar in VSCode-class drivers; structured
error metadata in MCP-class drivers) renders the reason verbatim plus a link to
the operator-facing remediation in the §6 FAQ.

---

## 5. Failure modes

This section addresses the **M2** finding by picking a single behaviour for
each failure path. The previous draft gave contradictory answers in §3.3
(Degraded UX, no enforcement) and §3.5 (fence locally on participation drop);
this doc resolves them by separating "Degraded UX state" (driver-local) from
"daemon-side fence authority" (daemon-side, never replicated by the driver).

### 5.1 Daemon drop mid-session (M2 resolved)

**Resolution: fail-soft on the driver side; the daemon retains fence authority.**

When a driver's transport drops:

1. Driver enters `Degraded` UX state immediately.
2. Heartbeat-loss grace window (5s default, configurable per-driver-type)
   delays diagnostic teardown so a brief daemon blip does not flap the UI.
3. After the grace window, diagnostics clear and the status surface shows
   "Anvil: offline (last seen <timestamp>)".
4. The driver does NOT block saves on its own. The driver does NOT
   pseudo-fence the worktree. Enforcement without a control authority is
   unsafe — the daemon is the single enforcement authority per §2.5 and
   ADR-015.
5. On reconnect the driver re-presents its manifest. The daemon re-runs
   `negotiate_capability` (§4.3). The driver returns to `Attached`; promotion
   to `Participating` is gated on the same DRVR-007 + DRVR-008 contract as
   before.
6. Any fence state the daemon held survives daemon restart via the persistent
   fence store. A reconnecting driver will see the fence in `anvil/status/query`
   and SHOULD surface "this worktree is fenced" via `window/showMessage`.

The §3.5 "fence locally on drop" wording in the previous draft is overruled by
this resolution. The driver is a UX surface; fence authority lives only with
the daemon.

### 5.2 Transport timeout

Default timeouts inherited from DRVR-001's `DriverClient`:

- Read-only requests (`scan_buffer`, `gate/request`, `status/query`,
  `suppression/apply`): 10 s.
- Enforcement ack (`enforcement/ack`): 500 ms — per §2.5's "ack within
  timeout" rule.

On timeout the client rejects the in-flight promise with the structured error
DRVR-001 minted: `{ error: "anvil-daemon-timeout", retriable: true }`. The
daemon's enforcement pipeline treats a missed ack as a refusal and escalates
per §2.5.

### 5.3 Malformed frame

Per INTD-014: the JSON-RPC framer discards the malformed frame, emits a
`framing-error` event, and preserves the connection. Drivers MUST treat
`framing-error` as a non-fatal diagnostic and continue accepting subsequent
frames; reconnecting on a single malformed frame would let a hostile peer DoS
the connection by injecting bad lines.

### 5.4 Multi-window fan-out (M4 resolved)

**Resolution: broadcast-and-first-ack with primary-editor nomination.**

Two editor windows on the same worktree are common (a Neovim instance and a
VSCode window editing the same repo). Both attach as drivers; INTD-003's
"single session per worktree" constraint applies to the daemon's session
registry, not to the driver fan-out.

The contract:

1. The daemon broadcasts `anvil/enforcement/decision` to ALL participating
   drivers attached to the worktree's session.
2. The first driver to ack within the §2.5 timeout wins the decision; the
   daemon records the ack and elides further work.
3. Drivers that arrive late receive a `decision_already_acked` notification
   with the `decision_id`; their UX should reflect "already handled by another
   editor window" rather than re-prompting the user.
4. Primary-editor nomination is a workspace-level setting via `.anvil.yaml`'s
   `primary_editor` key (driver-name match). When set, only the primary editor
   receives `enforcement/decision`; secondary editors stay in `Attached` for
   the same worktree even if they advertise enforcement. This addresses the
   spread-of-prompts issue when broadcast-and-first-ack would race UIs.
5. The daemon's reliability budget tracks acks per-driver, not per-worktree, so
   a slow secondary driver does not dock the primary's reliability score.

The earlier draft's "broadcast-and-first-ack OR primary-editor nomination"
either/or is replaced by "broadcast-and-first-ack as default; primary-editor
nomination as opt-in workspace policy" — both behaviours coexist and the
operator picks per-workspace.

### 5.5 MCP redaction handoff (M6 resolved)

**Resolution: editor-driver protocol does not own MCP redaction; that is a
sibling concern.**

`scan.files`, `fix.apply`, and `status.query` payloads crossing the MCP
transport are subject to the daemon-side redaction contract pinned in
[editor-and-mcp-driver-design.md §4.4](./anvil-driver-framework/editor-and-mcp-driver-design.md#44-redaction-contract-drvr-007-v1).
The contract is owned by RMCP-006 / RMCPF / DRVR-007; this protocol does not
re-litigate it. Editor-driver telemetry follows INTD-015 cross-session scoping
(workspace-relative paths, hash-of-path for cross-session redaction); editor
responses do NOT cross the MCP transport, so the §4.4 default-deny rules do
not apply to editor-driver responses.

The split is operator-visible: an editor driver running in VSCode sees full
file paths in its own `anvil/publishDiagnostics` notifications; an MCP agent
attached to the same daemon sees workspace-relative paths only. Both are
correct; they cross different transports.

### 5.6 correlationId retention (M12 resolved)

**Resolution: explicit non-persistence in v1; deferred persistence is
sequenced.**

Every driver-observable event carries a `correlation_id` minted by the daemon
when the event is generated. The retention policy:

- The daemon retains correlation chains in-memory for a sliding 60-second
  window. A driver can resolve a `correlation_id` to its full event chain via
  the existing daemon log lookup during this window.
- Beyond the window, the chain is dropped. There is no on-disk store in v1.
- The previous design's "daemon log lookup gives the whole chain" wording is
  scoped down: it is true for the in-memory window only.
- A future on-disk correlation store is sequenced as a separate INTD work item;
  it is NOT a DRVR-002 dependency. This protocol pins the wire shape so adding
  durable storage later is an INTD-side change, not a protocol change.
- Kindling bridge: the existing `gate_evaluated` / `decision_made` Kindling
  events already carry `correlation_id`; the bridge shape is unchanged. There
  is no new Kindling event type for DRVR-002.

This is the explicit non-persistence M12 asked for. The trade-off (no daemon
restart correlation lookup) is accepted as v1 cost; durable persistence will
be filed as a separate work item before any consumer surface a "lookup at
arbitrary time" UX.

---

## 6. LSP ↔ Anvil mapping

Stock LSP covers part of the surface; the `anvil/` namespace covers the rest.
The mapping is straightforward enough to enumerate.

### 6.1 What stock LSP carries

| Anvil concept                  | LSP method                                | Notes                                                                 |
|--------------------------------|-------------------------------------------|-----------------------------------------------------------------------|
| Diagnostic rendering (lossy)   | `textDocument/publishDiagnostics`         | LSP shape drops `mode`, `category`, `correlationId` — use `anvil/`    |
| Code actions ("Rethink" nudge) | `textDocument/codeAction`                 | Action payload carries an `anvil/`-namespaced kind                    |
| Suppression-comment insertion  | `workspace/applyEdit`                     | Daemon-normalised comment from `anvil/suppression/apply`              |
| Status notifications           | `window/showMessage`                      | Used for fence-applied / quarantined transitions                      |
| Handshake                      | `initialize` / `initialized`              | Minimal envelope; the full §2.2 manifest rides in an `initializationOptions` field |

### 6.2 What `anvil/` covers (and why)

| Anvil concept                  | Anvil method                              | Why not LSP                                                           |
|--------------------------------|-------------------------------------------|-----------------------------------------------------------------------|
| Diagnostics with full payload  | `anvil/publishDiagnostics`                | LSP `Diagnostic` cannot carry `mode`, `category`, `correlationId`     |
| Mid-edit buffer scan           | `anvil/scan_buffer`                       | LSP has no equivalent (LSP `textDocument/diagnostic` is pull-based)   |
| Enforcement decision + ack     | `anvil/enforcement/{decision,ack,refuse}` | LSP has no notion of enforcement, fences, ack-or-refuse contracts     |
| Gate result                    | `anvil/gate/{request,result}`             | LSP has no gate concept; CI / agent surfaces map to this              |
| Suppression validation         | `anvil/suppression/apply`                 | LSP has no suppression-comment validation primitive                   |
| Capability downgrade           | `anvil/capability/downgrade`              | LSP has no capability-renegotiation surface; introduced by DRVR-008   |
| Status snapshot                | `anvil/status/query`                      | LSP `window/workDoneProgress` not a fit; status is multi-domain       |

### 6.3 Stock-LSP-only clients (DRVR-008 case)

A client speaking only LSP (Neovim built-in LSP, Helix, Zed without an
Anvil-aware plugin) connects, completes the handshake, and reaches `Attached`.
It receives `textDocument/publishDiagnostics` (LSP-shape) for the document.
It does NOT receive `anvil/publishDiagnostics` because it did not advertise
support for the method (per §2.4 of the parent spec: "daemon never *assumes* a
driver can handle an event it didn't announce").

If `.anvil.yaml` requests enforcement, the daemon's
`negotiate_capability` returns `Attached` plus a `capability/downgrade`
notification; the operator sees a clear "your editor does not implement
`anvil/enforcement/ack`; install an Anvil plugin" message rather than getting
silently fenced.

The previous "every LSP client gets Anvil for free" framing in ADR-030 is
softened (commit alongside this spec) to "every LSP client gets Anvil
*diagnostics* for free; enforcement-participation requires explicit `anvil/`
support".

---

## 7. Open questions (assigned + dated — S6 resolved)

The §6 of the editor-and-mcp design spec listed five open questions. This
section assigns owners and deadlines; **S6 is resolved** with these
assignments.

| #  | Question                                          | Owner                  | Resolution deadline | Blocks?                                       |
|----|---------------------------------------------------|------------------------|---------------------|-----------------------------------------------|
| Q1 | Editor-driver transport on Windows (TCP-localhost vs named pipe) | INTD-012 owner       | 2026-05-13          | DRVR-003 sign-off (not DRVR-001/-002)         |
| Q2 | Per-workspace enforcement opt-in UX (workspace-trust piggyback?) | DRVR-003 owner       | 2026-05-20          | DRVR-003 sign-off                             |
| Q3 | Multi-editor windows                              | RESOLVED §5.4         | 2026-05-06         | none — pinned in this spec                    |
| Q4 | MCP-driver agent attribution                      | RMCPF-002 owner        | 2026-05-27          | RMCPF; not DRVR-002                           |
| Q5 | LSP namespace deprecation policy                  | DRVR-002 owner (council) | 2026-05-20        | future LSP-spec migration; not DRVR sign-off  |

Q1 / Q4 — neither blocks DRVR-001 or DRVR-002 sign-off; they block their
respective consumer work items. Q3 is resolved by §5.4.

The transport decision for Q1 is recorded provisionally as **named pipe in
v1** (matching the existing INTD-002 implementation); TCP-localhost is
deferred until a cross-editor reuse case appears.

---

## 8. End-to-end latency harness (S7 resolved)

**Resolution: cite ADR-031; no local numbers in this spec.**

ADR-031 (`plans/decisions/031-validation-latency-rubric.md`) owns the latency
rubric for save-time, mid-edit, pre-write, and gate paths. This protocol does
not pin local latency numbers because:

- Save-time / mid-edit / pre-write SLOs are daemon properties, not
  protocol properties.
- A latency claim in this spec would force a re-bump on every ADR-031 update.
- The harness mode discriminator (`mode = save | midEdit | preWrite | gate`)
  rides on the inner `Diagnostic.mode` field; consumers reading
  `validation.roundtrip` / `validation.service` against the canonical corpus
  see one source of truth for the daemon SLO.

Operations:

- The harness records `mode = save` with `validation.roundtrip` for the
  driver-visible SLO and `validation.service` for the same corpus / run so
  daemon work can be separated from driver / transport work.
- `validation.visible` is recorded only when making UX claims that include
  editor-side rendering (e.g. the wow-start LAUNCH demo). Editor-driver
  protocol latency claims default to `validation.roundtrip`.
- Cold-start latency is reported separately per ADR-031; drivers SHOULD
  surface a one-time "Anvil warming up" hint when cold-start delays
  diagnostics.

---

## 9. Versioning

`protocolVersion` is a single integer in the §2.2 manifest. v1's value is
**1**; any breaking change to:

- A method name (the `anvil/*` constants).
- A method's required parameter or result field.
- A capability lattice value or its serialisation.
- The capability-handshake contract.

requires bumping `protocolVersion` to 2 and the daemon advertising both the
old and new versions during a transition window per §2.4 of the parent spec.

Adding a new optional parameter, a new method, or a new value to
`supported_anvil_methods` does NOT bump the version — the manifest's
capabilities object and the `supported_anvil_methods` list are forwards-compat
by design.

`anvil.diagnostic.v1` evolves on its own version axis (see the envelope
coordination spec). Bumping the diagnostic shape to `v2` does NOT force a
`protocolVersion` bump unless the protocol layer needs to adapt to carry the
new shape.

---

## 10. Council resolution map

| Item | Source                              | Resolution location                                    |
|------|-------------------------------------|--------------------------------------------------------|
| M2   | Fail-soft vs enforcement contradiction | §5.1                                                |
| M3   | `anvil/gate/request` missing from method table | §2.2 + §3.4                                  |
| M4   | Multi-window fan-out                | §5.4                                                  |
| M6   | MCP redaction handoff               | §5.5                                                  |
| M12  | correlationId retention             | §5.6                                                  |
| S6   | Five §6 open questions              | §7                                                    |
| S7   | End-to-end latency harness          | §8                                                    |

---

## 11. Implementation map

| Surface                                                         | Owner                               |
|-----------------------------------------------------------------|-------------------------------------|
| Method-name + capability constants (Rust authoritative)         | `crates/anvil-intercept-proto/src/protocol.rs` |
| Capability negotiation gate (Rust)                              | `crates/anvil-intercept/src/auth.rs::negotiate_capability` |
| Manifest slice (`supported_anvil_methods`) (Rust)               | `crates/anvil-intercept/src/auth.rs::DriverManifest`       |
| Method-name + capability constants (TS mirror)                  | `packages/anvil-driver-client/src/protocol/types.ts`       |
| Method parameter / result interfaces (TS)                       | `packages/anvil-driver-client/src/protocol/types.ts`       |
| `Diagnostic` import (Rust)                                      | `anvil_kernel_types::Diagnostic`    |
| `Diagnostic` import (TS)                                        | `@eddacraft/anvil-driver-client` `diagnostics` re-export   |

The Rust side is authoritative; if Rust and TS drift on a constant or
serialised form, Rust wins and TS is updated. CI tests on both sides pin the
exact wire strings; a drift fails parity tests before merge.

---

## 12. Out of scope

- DRVR-003 (VSCode driver) — deferred per ADR-033; resumes when a new VSCode
  extension package is created on the daemon-driver path.
- RMCPF parity — separate work item; this protocol's editor methods are
  reused by the MCP driver where applicable, but the parity wiring is
  RMCPF-002's responsibility.
- Future capability tiers (`Trusted` for cross-host drivers) — extend the
  lattice in a successor protocol version; v1 has only the two-state lattice.
- Reasoning-pattern catalogue, suppression UX flows, nudge layouts — owned by
  surfacing modules (LANG, AIGUARD, TUI), not the protocol.
