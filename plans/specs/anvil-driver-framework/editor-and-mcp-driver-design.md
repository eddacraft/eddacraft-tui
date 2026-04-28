# Editor Driver and MCP Driver — Expanded Design

**Status:** Draft
**Date:** 2026-04-23
**Companion to:**
[`anvil-driver-framework-design-spec.md`](./anvil-driver-framework-design-spec.md),
[`anvil-driver-framework-adr.md`](./anvil-driver-framework-adr.md),
[`../../decisions/030-surface-drivers-supersede-napi-cutover.md`](../../decisions/030-surface-drivers-supersede-napi-cutover.md)
**APS module:** DRVR

---

## 1. Why this doc exists

The driver-framework design spec covers the shell driver, tmux driver,
remote-shell driver, and the web-session driver in depth. It treats the
**editor-driver** and **mcp-driver** in one paragraph each (§6.2). DRVR
is the module that lands them first, so the missing detail has to exist
somewhere. This doc is that somewhere.

It also covers cross-cutting driver concerns that the framework spec
leaves implicit: lifecycle, identity, version negotiation, failure
isolation, and correlation between driver-side diagnostics and
daemon-side violations. Those are not editor- or MCP-specific, but
DRVR is where the first real driver integrations happen, so pinning
them now prevents per-driver drift later.

---

## 2. Shared driver machinery

Everything in this section is driver-type-agnostic. The goal is to
write it once so editor-driver and mcp-driver (and later
shell-remote, tmux, etc.) don't each reinvent it.

### 2.1 Driver lifecycle

Five states. A driver is always in exactly one.

```
             ┌───────────┐
             │  Unbound  │  ← process exists, no daemon connection
             └─────┬─────┘
                   │  connect() over UDS/named pipe
                   ▼
             ┌───────────┐
             │ Handshake │  ← capability negotiation in flight
             └─────┬─────┘
                   │  daemon.acceptDriver(manifest)
                   ▼
             ┌───────────┐      daemon.revoke / transport drop
             │  Attached ├────────────────────────────┐
             └─────┬─────┘                            │
                   │  driver opts into enforcement    │
                   ▼                                  │
             ┌───────────┐                            │
             │Participating│ ← eligible for enforcement acks
             └─────┬─────┘                            │
                   │  graceful shutdown               │
                   ▼                                  ▼
             ┌─────────────────────────────────────────┐
             │              Detached                   │
             └─────────────────────────────────────────┘
```

Transitions:

- **Unbound → Handshake:** driver opens the transport. Connection is
  not enough to be considered attached; the driver must present a
  manifest and receive an accept from the daemon.
- **Handshake → Attached:** daemon validates the manifest (see §2.3
  identity) and responds with a session token scoped to the driver.
  Driver now receives subscribed events.
- **Attached → Participating:** driver explicitly sets
  `capability.enforcement = true` in a `driver.capabilities.update`
  call. Default across all drivers is **Attached** (read-only).
  Participating drivers can ack enforcement decisions and their
  refusal to ack escalates per §2.5.
- **Any → Detached:** either side may initiate. Clean detach releases
  the session token and closes subscriptions. Unclean detach (socket
  drop, process crash) is detected by the daemon's heartbeat and
  reaped after the configured timeout.

The editor-driver and mcp-driver both inherit this machine. They
differ in which transitions they're allowed to request and how long
they hold participation.

### 2.2 Driver manifest

Sent as the first frame after transport connect.

```ts
type DriverManifest = {
  driverType: 'editor' | 'mcp' | 'shell-local' | 'shell-remote' | 'tmux'
            | 'process' | 'web-session'
  driverName: string        // e.g. "anvil-vscode", "anvil-mcp-server"
  driverVersion: string     // semver
  protocolVersion: number   // daemon's driver-protocol major version
  capabilities: {
    read: boolean           // will subscribe to events
    enforcementCandidate: boolean  // may request enforcement later
    supportedDecisions: Array<'warn' | 'block' | 'interrupt'>
  }
  context: {
    host: HostDescriptor    // see framework spec §7.1
    pid: number
    startedAt: string       // ISO 8601
    workspaceRoots: string[]
  }
  auth: {
    method: 'uds-peer' | 'token'
    // uds-peer: daemon checks SO_PEERCRED / equivalent
    // token: driver presents a token obtained out of band (future)
    token?: string
  }
}
```

The daemon validates `protocolVersion` against its supported range.
Mismatch → `handshake.reject` with a machine-readable reason so the
driver can surface "please upgrade / please downgrade".

### 2.3 Driver identity

Who is allowed to attach. Spoofing matters because a hostile driver
could request enforcement acks it has no intent to honour.

- **Default (v1):** Unix peer credential check via `SO_PEERCRED`
  (Linux) / `LOCAL_PEERCRED` (macOS) / named-pipe ACL inspection
  (Windows). The daemon trusts any driver running under the same UID
  as the daemon itself. This matches the daemon's own security model
  (per-user singleton; fences are per-user).
- **Future (v2+):** Signed manifest or OIDC-style token. Necessary
  only when drivers start running cross-user or across hosts; the
  remote-shell driver is the likely first caller.
- **Never:** reliance on `driverName` alone. It's metadata, not an
  auth factor.

### 2.4 Version negotiation

`protocolVersion` is a single integer. Breaking changes bump it.
Drivers advertise the exact version they speak. The daemon exposes a
supported range (e.g. `[2, 3]`) and accepts any driver in-range.
Drop-dead deprecation of old versions follows the same cadence as
ADR-020 (lockstep versioning for core).

Non-breaking additions (new methods, new event types) are advertised
via the `capabilities` object. Drivers that don't advertise a
capability don't receive the corresponding event; daemon never
*assumes* a driver can handle an event it didn't announce.

### 2.5 Enforcement ack & refusal

A Participating driver receives `enforcement.decision` events. For each
one it must respond with either `enforcement.ack` (carried out) or
`enforcement.refuse` (could not carry out, with reason).

- **Ack within timeout (default 250ms for warn/block, 500ms for
  interrupt):** daemon records the action as applied.
- **Refuse:** daemon escalates per the framework spec's §9.4 — next
  eligible driver on the same session, or a higher-tier action
  (interrupt → fence).
- **Neither within timeout:** treated as refuse. Daemon logs a
  `driver.unresponsive` telemetry event and re-scores the driver's
  reliability budget (see §2.6).

### 2.6 Reliability budget

Drivers don't get unlimited refusals or timeouts. Each driver
instance carries a sliding-window reliability score. Below threshold:

1. Daemon drops the driver from `Participating` back to `Attached`
   (revokes enforcement capability for this session).
2. Future handshakes from the same driver name are auto-downgraded to
   read-only for a cooldown window.
3. Telemetry event `driver.quarantined` fires; the editor's status
   bar (or MCP's structured-error surface) reflects the state.

Exact thresholds are daemon-config, not wire-level.

### 2.7 Observability & correlation

Every driver-observable event carries a daemon-emitted `seq` (monotonic
per-session) and `correlationId` (UUID). Driver-side diagnostics rendered
in VSCode or returned in an MCP tool response **must** echo
`correlationId` back to the user-visible surface (hover, detail
panel, JSON response). This is what lets a developer say "Anvil
flagged this at 14:22, here's the correlation ID" and have a daemon
log lookup give the whole chain — watcher → rule → decision →
driver ack.

### 2.8 Failure isolation

One bad driver does not harm another. Concretely:

- Each driver has a dedicated task in the daemon's tokio runtime;
  panic in one task is caught and does not cross into others.
- Subscription fan-out uses bounded channels per driver. A slow
  driver fills its channel and gets dropped from the subscription
  (with a `driver.overflow` event) before the daemon starts buffering
  unboundedly or slowing other drivers.
- Driver refusals never block the enforcement pipeline; escalation
  is asynchronous.

---

## 3. Editor driver (VSCode first, LSP-shaped where possible)

### 3.1 What the editor-driver is

An editor-driver is a driver that:

- Attaches to the daemon from inside an editor process (VSCode, Cursor,
  JetBrains, Zed, Neovim).
- Receives file-change-derived violations and renders them as native
  editor diagnostics.
- Optionally participates in enforcement: surfacing blocks, fencing a
  worktree the user opened, rejecting a save, or stopping a
  user-invoked agent action inside the editor.
- Presents Anvil-specific UI (suppression state, gate results, nudge
  metadata) that stock LSP doesn't have a slot for.

### 3.2 Protocol split — LSP where it fits, Anvil where it doesn't

The editor-driver talks **two** languages over the same transport:

- **LSP subset:** used for everything the LSP spec already models.
- **Anvil extensions:** used for everything it doesn't.

Stock LSP coverage:

| LSP method / notification            | Role                                  |
|--------------------------------------|---------------------------------------|
| `textDocument/publishDiagnostics`    | Render violations as editor diagnostics |
| `textDocument/codeAction`            | Surface the "Rethink" nudge + "Suppress with reason" action |
| `workspace/applyEdit`                | Used by `fix` code actions that Anvil can perform deterministically |
| `window/showMessage`                 | Surface enforcement state changes (fence applied, driver quarantined) |
| `initialize` / `initialized`         | Light wrapper around the driver manifest handshake; LSP fields are a subset of the manifest |

Anvil-specific methods (under the `anvil/` namespace):

| Method                                     | Direction | Purpose                              |
|--------------------------------------------|-----------|--------------------------------------|
| `anvil/driver/capabilities/update`         | client → server | Promote Attached → Participating |
| `anvil/enforcement/decision`               | server → client (notification) | Enforcement decision target = this driver |
| `anvil/enforcement/ack`                    | client → server | Confirm decision carried out |
| `anvil/enforcement/refuse`                 | client → server | Could not carry out; daemon escalates |
| `anvil/suppression/state`                  | server → client | Current suppression map for a file  |
| `anvil/suppression/apply`                  | client → server | User applied `@anvil-ignore` via code action |
| `anvil/gate/result`                        | server → client | Latest gate result snapshot         |
| `anvil/nudge/metadata`                     | server → client | Extra metadata (explanation URL, recommended rewrite) for a warning |
| `anvil/correlation`                        | server → client (embedded in every diagnostic) | Correlation ID for log lookup |

The design rule: **no new `anvil/` method without a concrete editor
feature that can't be expressed in LSP.** DRVR-002 bakes this in so
the Anvil namespace doesn't sprawl.

### 3.3 Capability state in the editor UX

The three states the user can see:

1. **Read-only:** diagnostics render, code actions work, no
   enforcement. This is the default. VSCode status bar: "Anvil: on".
2. **Enforcement-participating:** diagnostics + enforcement acks.
   Opt-in per workspace via `.anvil.yaml` or a one-time
   accept-dialog. Status bar: "Anvil: enforcing".
3. **Degraded:** daemon unreachable. No diagnostics, no gate results.
   Status bar: "Anvil: offline (last seen 14:22)". The user can
   retry from a command palette action.

The transitions between Read-only and Enforcement-participating
happen via `anvil/driver/capabilities/update`. The Degraded state is
driven by the transport heartbeat; loss of heartbeat moves the UX to
Degraded without unmounting diagnostics immediately (grace period,
configurable, default 5s) so a brief daemon blip doesn't flap the UI.

### 3.4 Save-time latency budget

The save-time path is:

```
save → textDocument/didSave → daemon scans → publishDiagnostics
```

Latency requirements are defined by ADR-031 rather than by local
numbers in this design spec. The save-time path uses `mode = save` and
the interactive save-time SLO: warm `validation.service` p95 <= 80 ms
and warm `validation.roundtrip` p95 <= 120 ms over the canonical
corpus. p50 and p99 are reported for context.

If this surface makes a user-visible claim such as "diagnostics appear
within X ms of save", it must also report `validation.visible`. That
surface-owned number includes editor rendering and any surface-specific
work that ADR-031 intentionally keeps outside the daemon SLO. Cold-start
latency is reported separately and should surface a one-time "Anvil
warming up" hint if diagnostics are delayed.

### 3.5 Offline / daemon-down behaviour

If the daemon is unreachable at editor start:

- The extension still loads. Commands that don't need the daemon
  (opening files, rendering existing in-memory state) work.
- A status-bar item shows "Anvil: offline" with a click-to-retry.
- Diagnostics cleared; no scanner runs in-process.
- If the editor was previously Participating, the session fences
  locally (pessimistic) — no enforcement decisions render, but the
  editor does **not** block saves on its own. Enforcement without a
  control authority is unsafe; fencing is the daemon's job.

The ADR's "fail soft" rule: an editor with no daemon must not crash,
must not block the user, and must not silently pretend things are
fine.

### 3.6 Suppression UX

Stock LSP code actions can't render a "Suppress with reason" form.
The editor-driver handles this via:

1. Code action surfaces a quick-pick: "Suppress permanently",
   "Suppress until date…", "Suppress with reason…".
2. User input captured in the editor UI.
3. `anvil/suppression/apply` sent to the daemon with the proposed
   suppression comment, file, range, and reason.
4. Daemon validates (format, ADR-004 compliance), returns the
   normalised comment.
5. Editor applies the comment via `workspace/applyEdit`.

The daemon owns the suppression contract. The editor is a pure UI.

### 3.7 Gate results

VSCode already has a Gate Results tree view. Under the current TS
scanner path, it's populated by running the gate check in-process.
Under DRVR:

- Extension sends `anvil/gate/request` with the workspace root.
- Daemon streams `anvil/gate/result` snapshots (Progress → Snapshot
  → done) over the telemetry lane.
- Extension renders into the tree view as snapshots arrive.

Gate work that needs OPA / external tools still runs server-side in
the daemon or is delegated to the existing `GateRunner` code invoked
by the daemon (transitional). The editor never runs those directly.

---

## 4. MCP driver

### 4.1 What the mcp-driver is

The mcp-driver is how the existing `packages/mcp-server/` keeps
working — unchanged wire contract with agents, different internals.
Instead of importing `@eddacraft/anvil-runtime`'s `GateRunner`
directly, the MCP server is an **mcp-driver** that makes JSON-RPC
calls to the daemon.

Per the framework ADR: **MCP is a fallback / secondary driver, not a
foundational control plane.** That shapes its capability set.

### 4.2 Capability set

What the mcp-driver *can* do:

- Read: subscribe to violations, gate results, suppression state.
- Soft enforcement: relay "this agent's last action violated X" back
  through an MCP response. That's a warn, not a block.
- Lease state reporting: tell an agent "the worktree is fenced,
  here's why, here's how to resolve."

What the mcp-driver *cannot* do:

- Hard-interrupt an agent process. MCP doesn't have the authority to
  signal the agent's process group; the intercept-launcher
  (`anvil-run`) or the shell driver does.
- Block a file write after it's happened. Detection through MCP is
  after-the-fact.
- Fence a worktree. Only the daemon fences, and only on violations it
  saw via the watcher.

### 4.3 Translation model

The MCP server's tool handlers remain the agent-facing contract. Each
handler becomes a thin translator:

| MCP tool          | Before DRVR                                        | After DRVR                                   |
|-------------------|----------------------------------------------------|----------------------------------------------|
| `anvil_check`     | `GateRunner.analyzeFiles(files, root, opts)` in-process | `driver.request('scan.files', {...})` against daemon |
| `anvil_fix`       | In-process deterministic fixers                    | `driver.request('fix.apply', {...})`         |
| `anvil_gate`      | `GateRunner.runGates(...)` in-process              | `driver.request('gate.run', {...})`          |
| `anvil_suppress`  | Write suppression directly to file                 | `driver.request('suppression.apply', {...})` which validates and returns the normalised comment |
| `anvil_status`    | In-process check registry reflection               | `driver.request('status.query')` — daemon returns authoritative state |
| `anvil_query_boundary` | In-process boundary lookup                     | `driver.request('architecture.queryBoundary', {...})` |

All MCP tool input/output schemas remain unchanged. Agents that use
the MCP server continue to work. The change is invisible above the
MCP transport.

### 4.4 Degraded behaviour

MCP driver handling of a daemon-down state differs from the editor
driver:

- MCP tools return a structured error: `{ error:
  "anvil-daemon-unavailable", retriable: true, lastSeen: "..." }`.
- Resources fall back to a stub that returns empty state with the
  same structured-error in the metadata.
- No in-process scanner reinstated. The whole point of DRVR is the
  TS scanner dies; we don't bring it back for a fallback.

Agents are expected to reason about retriable errors. If they can't,
the error at least names the problem clearly.

### 4.5 Distribution

MCP driver ships as part of the existing `packages/mcp-server/`
package, consuming the shared `packages/anvil-driver-client/` library
(DRVR-001). Its npm publication story is unchanged from today.

---

## 5. What becomes possible once both drivers land

Value that exists only when the driver graph is populated:

- **Editor-agnostic Anvil:** every LSP client (Neovim, JetBrains,
  Zed, Helix) can add an Anvil editor-driver by reusing the protocol.
  The VSCode-specific code shrinks to thin LanguageClient wrapping +
  tree views + status bar.
- **Correlated debugging:** a developer reporting "Anvil flagged this
  wrong" carries a correlationId that resolves to the exact daemon
  session, rule evaluation, and driver ack.
- **Consistent enforcement surface:** the same daemon decides warn /
  block / interrupt for CLI, editor, MCP. A team-wide "upgrade to
  enforcement" happens in one place.
- **Observability crossover:** the Edda Stack's Kindling emission
  points (`gate_evaluated`, `decision_made`) fire from the daemon,
  not per-surface. That's the correct level — previously Kindling
  would have had to observe multiple engines.
- **Agent infrastructure alignment:** the forthcoming
  `agent-infrastructure` module (WEAVE / AHARNESS) can hang its
  policy decisions off the same daemon decision contract rather than
  building a parallel one.

---

## 6. Open questions (specific to editor-driver and mcp-driver)

These need answers before DRVR-002 lands.

1. **Editor-driver transport on Windows.** Named pipes are viable but
   VSCode's LanguageClient has a well-trodden path for stdio /
   socket. Do we expose a TCP-localhost fallback in v1 or force named
   pipes? TCP simplifies cross-editor reuse but introduces an extra
   auth consideration.
2. **Per-workspace enforcement opt-in UX.** VSCode's workspace-trust
   prompt already carries a lot of load. Do we piggyback on trust, add
   our own confirm, or honour `.anvil.yaml` without a UI confirm?
3. **Multiple editor windows, same workspace.** Two VSCode windows
   open on the same worktree. Both attach as drivers. The daemon has
   two editor-drivers for one session — how do enforcement decisions
   target them? Broadcast-and-first-ack? Or explicit "primary
   editor" nomination?
4. **MCP-driver and agent attribution.** When an MCP tool call
   produces a daemon RPC, the daemon wants to know which session /
   which agent. MCP has some session context, but not the PGID /
   worktree-provenance the shell driver gives. Do we carry the
   agent's anvil-run session ID through MCP metadata, or accept that
   MCP-driver interactions are less attributable?
5. **Fallback when LSP features lag the `anvil/` namespace.** If a
   future LSP spec version covers something we currently implement
   under `anvil/`, we should migrate. What's our policy for
   deprecating an `anvil/` method in favour of a new LSP one?

---

## 7. Out of scope here

- Shell, remote-shell, tmux, process, web-session drivers — covered
  in `anvil-driver-framework-design-spec.md`.
- Daemon internals (rule evaluation, enforcement ladder
  implementation, fence persistence) — covered in INTD module + ADR-015.
- Agent infrastructure integration — separate WEAVE / AHARNESS work.

---

## 8. Implementation sequencing

Maps to DRVR work items:

| Step | Work item | Blocks on |
|------|-----------|-----------|
| Shared client library | DRVR-001 | INTD-002 stable IPC surface |
| Protocol spec + contracts | DRVR-002 | DRVR-001, this doc, operations review |
| Editor driver cutover | DRVR-003 | DRVR-002, INTD-013 violation stream (telemetry mirror) |
| MCP driver cutover | DRVR-004 | DRVR-002, INTD rule-evaluation RPC |
| Docs + cross-links | DRVR-005 | DRVR-003 and DRVR-004 complete |

DRVR-001 and this design doc are the two things that unblock the
real cutover work. Everything after DRVR-002 is implementation.
