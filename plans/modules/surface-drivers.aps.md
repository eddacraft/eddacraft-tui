<!--
APS Module: Surface Drivers
===========================
Cut VSCode and MCP over to drivers on the anvil-intercept daemon,
superseding TSRET-003/-004. Per ADR-030. See: plans/aps-rules.md
-->

# Surface Drivers

| ID   | Owner | Status | Progress |
| ---- | ----- | ------ | -------- |
| DRVR | —     | Draft  | 0/8      |

**Last reviewed:** 2026-04-26

## Purpose

Anvil's integration surfaces (VSCode extension, MCP server, future
editors) currently either import the TS scanner in-process
(`@eddacraft/anvil-core/antipattern`) or — under the superseded TSRET
plan — were intended to import a napi binding of the Rust scanner.
ADR-030 supersedes that plan: both surfaces become **drivers** that
attach to the `anvil-intercept` daemon over JSON-RPC 2.0 + NDJSON, per
the driver-framework ADR (`plans/specs/anvil-driver-framework/`).

This module delivers the first two surface drivers — **editor-driver**
(VSCode, LSP-shaped where possible) and **mcp-driver** (JSON-RPC
consumer of the daemon from within the existing MCP server). Once both
land, TSRET-005 (delete TS scanner + retire parity harness) can
execute because no surface imports scanner code.

## Background

- ADR-015 introduced the intercept daemon with an NDJSON IPC interface
  (UDS on Linux/macOS, named pipes on Windows). INTD v1 deliberately
  excludes editor and MCP drivers.
- The driver-framework ADR (proposed) formalises editors as a
  first-class driver class and MCP as a fallback driver. Both are
  v2 relative to INTD v1.
- TSRET-001 (napi spike) is retained as an internal acceleration path
  for the CLI. TSRET-002's remaining residual (publish flip, OOB
  tests, provenance) is dropped by ADR-030 — the daemon is the runtime
  bundling point and the napi crate stays private.
- The Rust kernel already exposes the scan, graph, and policy surfaces
  the daemon needs (KERN module, 22/25 done). KERN-050/-051/-052
  (Phase 5 daemon-mode items — Unix socket, JSON-RPC protocol, client
  session management) are **superseded by INTD-002, INTD-003, and
  INTD-013** (KERN-050 → INTD-002; KERN-051 → INTD-002 + INTD-013
  telemetry mirror; KERN-052 → INTD-003): `anvil-intercept` is the
  same long-running Rust process the kernel daemon was going to be,
  so the intercept daemon's IPC surface *is* the stable socket
  transport this module rides on.

## Scope

**In scope:**

- Editor driver (VSCode extension) cut over from
  `@eddacraft/anvil-core/antipattern` to a JSON-RPC client of the
  daemon. LSP for the parts the protocol covers
  (`textDocument/publishDiagnostics` — push model, matches the daemon's
  telemetry-lane event emission; `textDocument/codeAction`); custom
  Anvil extensions for what it doesn't (suppression state, gate
  results, nudge metadata).
- MCP driver: the MCP server's `check.tool.ts`, `fix.tool.ts`,
  `gate.tool.ts`, and related surfaces re-implemented as JSON-RPC
  callers against the daemon. Existing MCP wire contract with agents
  preserved.
- Shared TS client library (`packages/anvil-driver-client/` or similar)
  that both surfaces use for JSON-RPC framing, reconnection, and
  typed method/response envelopes. Prevents reimplementing the
  transport in two places.
- "Read-only diagnostic mode" vs "enforcement-participating mode" as
  distinct driver capabilities — the editor starts read-only; opting
  into enforcement is explicit.
- Fallback path when the daemon is unreachable (stale warning banner,
  no diagnostics, no block). Must fail soft: an editor that can't
  reach the daemon must not crash or refuse to open a file.
- Update `docs/architecture/anvil-full-architecture.md` to show editor
  and MCP as drivers on the daemon in the proposed-end-state diagram.

**Out of scope:**

- Remote-shell, tmux, process, or web-session drivers. Separate
  modules downstream of the driver-framework ADR.
- Any changes to the daemon's enforcement authority or ladder. This
  module is a pure consumer of the IPC surface.
- TSRET-005's actual deletion of the TS scanner. Retained in TSRET as
  the closing work item once DRVR is complete.
- Publishing `@eddacraft/anvil-checks-native` to npm. The napi crate
  stays private; see ADR-030.

## Interfaces

**Depends on:**

- **INTD-002** (IPC Listener — NDJSON over UDS / named pipe) —
  required for DRVR-001
- **INTD-003** (Session Registry) — required before any driver can
  register
- **INTD-005** (Enforcement Decision Pipeline) and **INTD-013**
  (Mirror Enforcement Decisions Onto Notification Telemetry) —
  required before the editor driver can render diagnostics; INTD-013
  is the canonical telemetry-lane emission point drivers subscribe to
- **ADR-030** — this module's authority
- **Driver-framework ADR** (`plans/specs/anvil-driver-framework/`) —
  defines driver capabilities, enforcement ladder, and the two-lane
  transport model
- *(KERN-050/-051/-052 — Unix socket, JSON-RPC, client session
  management — were previously listed here. They are now superseded
  by INTD-002 and INTD-003: the intercept daemon is the same
  long-running Rust process the kernel daemon was going to be, so
  its IPC surface is the transport DRVR rides on. See KERN module
  archive for the supersession note.)*

**Exposes:**

- Editor-driver contract: methods an editor driver implements (connect,
  handshake, subscribe to diagnostics, ack enforcement decisions,
  report local state)
- MCP-driver contract: methods the MCP layer translates between its own
  tool calls and daemon RPC
- `packages/anvil-driver-client/` — shared transport and typed API for
  TS consumers

## Tasks

### DRVR-001: Shared TS driver-client library

- **Intent:** One place implements JSON-RPC 2.0 + NDJSON framing,
  reconnection, transport selection (UDS / named pipe), and typed
  method envelopes. Both editor-driver and mcp-driver depend on this;
  no direct `net.createConnection` from either consumer.
- **Expected Outcome:** `packages/anvil-driver-client/` (or a similar
  path matching monorepo-structure conventions) exports a typed
  `DriverClient` class: `connect()`, `request<M, R>(method, params)`,
  `subscribe<E>(topic, handler)`, `close()`. Transport auto-selects by
  platform. Reconnection is transparent with a documented backoff.
  Partial-failure surface is explicit, not left to the implementer:
  - NDJSON framer on parse error discards the frame, emits a
    `framing-error` event, and preserves the connection.
  - Each `request<M, R>` call carries a timeout with a configurable
    default (suggested: 10 s for read-only, 500 ms for enforcement
    ack). Hitting the timeout rejects the promise with a structured
    `{ error: "anvil-daemon-timeout", retriable: true }`.
  - In-flight requests on transport drop are cancelled with the
    same structured error (`retriable: true`) so MCP tool responses
    preserve the retriable flag across reconnects.
  - Driver-side, `DriverClient.connect()` refuses a socket / named
    pipe that is not owned by the current user
    (pairs with INTD-002 daemon-side permissioning).
- **Scope:** `packages/anvil-driver-client/`
- **Dependencies:** INTD-002 (IPC Listener), INTD stable IPC
  wire-format doc
- **Validation:** Unit tests cover the JSON-RPC framer, NDJSON split,
  reconnection under dropped sockets, and happy-path request/response
  roundtrip against a fake daemon. Integration test connects to a real
  daemon binary and exercises `session.register` + `session.heartbeat`.
  Failure-path tests cover: partial frame, hung daemon past timeout,
  in-flight request cancellation on transport drop, and
  wrong-owner socket refused on connect.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft
- **Council review (2026-04-24):** Partial-failure surface bullet added
  for M13 (operations-reviewer); driver-side socket-owner check for
  M8 (security-analyst).

---

### DRVR-002: Editor-driver protocol definition

- **Intent:** Pin the JSON-RPC methods, notifications, and capability
  handshake the editor driver and the daemon agree on. Separates the
  LSP-shaped surface (diagnostics, code actions) from Anvil-specific
  extensions (suppression state, gate results, nudges, enforcement
  acks).
- **Expected Outcome:** Design doc at
  `plans/specs/2026-XX-XX-editor-driver-protocol.md` documenting the
  method list, payload shapes, state machine for driver capabilities
  (read-only vs enforcement-participating), failure modes, and the
  mapping between LSP primitives and Anvil primitives. Schemas published
  in a shared contracts package (`packages/anvil/contracts/` or
  similar) as TS types + Rust types.

  The design doc MUST resolve the following council-review items
  before sign-off (council findings are all tracked in
  PR #1063):
  - **Fail-soft vs enforcement-participating contradiction (M2):**
    pick a single behaviour for daemon-drop mid-session. Options:
    fence locally on drop (safe default) or fall through to
    fail-soft and unblock saves (availability default). Current
    design gives contradictory answers in §3.3 and §3.5.
  - **`anvil/gate/request` missing from method table (M3):**
    add it to the §3.2 table or remove the §3.7 reference.
  - **Multi-window fan-out (M4, §6 Q3):** pick
    broadcast-and-first-ack vs primary-editor nomination. Reconcile
    with INTD-003's "single session per worktree" constraint.
  - **MCP redaction contract (M6):** specify allow-list / deny-list
    for payload fields crossing the MCP transport; default-deny on
    secret-detection content excerpts to remote LLM endpoints.
  - **correlationId retention (M12):** specify the daemon-side
    retention window, on-disk store (or explicit non-persistence),
    and Kindling bridge shape. "Daemon log lookup gives the whole
    chain" requires a durable store that survives daemon restart.
  - **Five §6 open questions (S6):** assign owners and deadlines;
    mark which block DRVR-001 API sign-off vs DRVR-002 sign-off vs
    DRVR-003.
  - **End-to-end latency harness (S7):** delete local latency
    numbers from the protocol design and cite ADR-031 instead. The
    harness must record `mode = save` with `validation.roundtrip`
    for the driver-visible SLO and `validation.visible` only when
    making UX claims.
- **Scope:** `plans/specs/`, shared contracts package
- **Dependencies:** DRVR-001 (transport), DRVR-006 (MCP scope
  resolved), driver-framework ADR
- **Validation:** Reviewed by one member each of: architect,
  pragmatic-lead, operations-reviewer. Matches the driver-framework
  ADR's capability vocabulary. Each of the council-review items
  listed above has a concrete answer in the design doc; reviewer
  checklists confirm coverage.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft
- **Council review (2026-04-24):** Expected-outcome expanded with
  M2 / M3 / M4 / M6 / M12 / S6 / S7 prerequisites.

---

### DRVR-003: VSCode extension cut over to editor driver

- **Intent:** Every scanner-adjacent call path in the extension
  (`embeddedAnalysis.ts`, diagnostics service, nudge code actions) goes
  through the driver client instead of `@eddacraft/anvil-core/antipattern`.
- **Expected Outcome:** `packages/vscode-extension/src/services/embeddedAnalysis.ts`
  no longer imports `@eddacraft/anvil-core/antipattern`. Diagnostics,
  code actions, and pattern-registry queries route through
  `DriverClient`. The ADR-031 interactive save-time SLO is held or
  justified using `mode = save` and `validation.roundtrip`; any
  save-to-visible UX claim reports `validation.visible` separately.
  Existing extension tests pass after refactor; one new test covers
  the fallback path when the daemon is unreachable.
- **Scope:** `packages/vscode-extension/`
- **Dependencies:** DRVR-001, DRVR-002, INTD-002 (IPC Listener),
  INTD-013 (telemetry mirror — the canonical violation stream)
- **Validation:** `pnpm --filter anvil-vscode test` passes; manual
  scan in VSCode matches `anvil check` output on the same fixture;
  fallback test asserts no diagnostics appear and a status-bar item
  surfaces the degraded state.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft

---

### DRVR-004: MCP server cut over to MCP driver

- **Intent:** MCP tool handlers stop calling `@eddacraft/anvil-runtime`'s
  `GateRunner` for antipattern/scan work. They become thin adapters
  that translate MCP tool input into daemon RPCs and format responses.
- **Expected Outcome:** `packages/mcp-server/src/tools/check.tool.ts`,
  `fix.tool.ts`, `gate.tool.ts`, and relevant resource handlers use
  `DriverClient`. The MCP wire contract with agents is unchanged. E2E
  MCP tests pass against a live daemon. Documented failure mode when
  the daemon is unreachable (MCP tool returns a structured error that
  agents can reason about).
- **Scope:** `packages/mcp-server/`
- **Dependencies:** DRVR-001, DRVR-002, INTD-002 (IPC Listener),
  INTD-005 (Enforcement Decision Pipeline — the rule-evaluation
  surface MCP tools translate to)
- **Validation:** `pnpm --filter @eddacraft/anvil-mcp test`; E2E
  harness call through the MCP transport returns structurally
  identical results to pre-cutover for the fixture set.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft

---

### DRVR-005: Architecture doc + ADR supersession cross-links

- **Intent:** The proposed-end-state diagrams in
  `docs/architecture/anvil-full-architecture.md` and
  `docs/architecture/rust-architecture-endstate.md` still show napi
  bindings as the VSCode / MCP integration path. Update to show editor
  and MCP as drivers on the daemon. Add pointers to ADR-030 wherever
  TSRET is referenced.
- **Expected Outcome:** Both architecture docs updated. `plans/decisions/DECISION-LOG.md`
  ADR-030 entry stays accurate. TSRET module's -003/-004 entries carry
  a **Superseded by:** ADR-030 / DRVR reference. Any onboarding doc
  that mentions "napi cutover" is updated or removed.
- **Scope:** `docs/architecture/`, `plans/decisions/DECISION-LOG.md`,
  `plans/modules/anvil-ts-scanner-retirement.aps.md`, any doc with a
  "napi" or "TSRET" reference
- **Dependencies:** DRVR-003 and DRVR-004 complete (so docs reflect
  reality, not aspiration)
- **Validation:** `grep -r "napi cutover\|@eddacraft/anvil-checks-native"
  docs/` returns only historical references under `ENGINEERING-HISTORY.md`
  or similar archive paths.
- **Confidence:** high
- **Priority:** Medium
- **Status:** Draft

---

### DRVR-006: Pin MCP daemon-RPC surface (resolve translation-table scope)

- **Intent:** The MCP translation table in
  `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
  §4.3 names six RPCs (`scan.files`, `fix.apply`, `gate.run`,
  `suppression.apply`, `status.query`, `architecture.queryBoundary`)
  that have no backing in INTD-001..-016, and the `GateRunner` they
  replace currently runs `npm audit`, OPA, and coverage JSON reads
  that the daemon does not do. This work item decides which path is
  the actual design. Must complete before DRVR-002 sign-off.
- **Expected Outcome:** Pick and record one of:
  - **(a) Shrink** the §4.3 table to RPCs INTD actually exposes
    (scan + suppression), move the rest to MCP-server-local helpers
    that invoke the CLI / external tools directly.
  - **(b) Distinguish** daemon-round-tripped RPCs from
    MCP-driver-local composition in the table; record which category
    each MCP tool belongs to and why.
  - **(c) Expand** INTD scope with new work items for the missing
    RPCs, acknowledging the cost and slipping the DRVR-004 schedule.
  Whichever path lands, the design-spec §4.3 and the DRVR-004
  expected-outcome are rewritten to match. If (c) is chosen, file
  the new INTD items as part of the DRVR-006 execution work (not
  the current docs PR, which will already be merged by the time
  this decision lands).
- **Scope:** `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`,
  `plans/modules/surface-drivers.aps.md` (DRVR-004 expected outcome),
  `plans/modules/intercept-daemon.aps.md` (only if path (c) is
  chosen)
- **Dependencies:** none (this is a scope-resolution task blocking
  DRVR-002)
- **Validation:** Design spec §4.3 updated, DRVR-004 expected outcome
  updated to match, inline prose contains no references to RPC names
  that lack a backing INTD item (or driver-local helper).
- **Source:** 2026-04-24 council review C2 (adversarial reviewer +
  council-reviewer, judge-upgraded) — tracked in
  PR #1063.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft

---

### DRVR-007: Driver trust + enforcement security contract

- **Intent:** The editor-driver and mcp-driver designs grant drivers
  authority to subscribe to telemetry (potentially exfiltrating
  secret-detection content), ack enforcement decisions (refusing
  escalates to fence, which can DoS active sessions), and promote
  themselves to `Participating`. The current spec trusts any
  same-UID process via `SO_PEERCRED`. Without a written threat model
  and hardening, DRVR-003 / DRVR-004 ship a local privilege-lateral
  path to arbitrary same-UID peers.
- **Expected Outcome:** The `editor-and-mcp-driver-design.md` spec
  gains an explicit "Driver trust boundary (v1)" section enumerating:
  (a) what a same-UID driver can do, (b) what it cannot do,
  (c) which capabilities require stronger identity in v1+ vs
  deferred. Concrete v1 hardening:
  - `capability.enforcementCandidate: true` requires the driver
    binary's path (resolved via `/proc/<pid>/exe` / `proc_pidpath` /
    `QueryFullProcessImageName`) to be on an allowlist configured
    out-of-band (e.g. `~/.config/anvil/drivers.allow`, written only
    by the daemon setup command).
  - `workspaceRoots` in a driver manifest validated against live
    session claims before the driver becomes a routing target;
    unknown roots drop the driver to read-only observer.
  - Reliability-budget quarantine keyed on a stable identity
    (signed capability token, install-time UUID, or binary hash),
    not self-declared `driverName`. Quarantine survives reconnect.
  - MCP-driver response payloads for `scan.files`, `fix.apply`,
    `status.query` have a daemon-side redaction contract — default
    deny on secret-detection content excerpts and absolute paths
    crossing the MCP transport; explicit opt-in per rule family if
    needed.
- **Scope:** `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
  (new "Driver trust boundary" subsection in §2, plus redaction
  contract in §4; both MUST be written before DRVR-002 sign-off),
  `crates/anvil-intercept/src/auth.rs` (new) for v1 allowlist
- **Dependencies:** INTD-003 (session registry for `workspaceRoots`
  claim validation), INTD-015 (daemon-enforced telemetry scoping)
- **Validation:** Spec review by security-analyst covers each of the
  four sub-points; implementation tests (when DRVR-003 / DRVR-004
  land) include a hostile-driver fixture per category.
- **Source:** 2026-04-24 council review M5 / M6 / M7 / M11
  (security-analyst + adversarial-reviewer) — tracked in
  PR #1063.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft

---

### DRVR-008: Non-VSCode LSP client capability negotiation

- **Intent:** Non-VSCode LSP clients (Neovim built-in LSP, Zed,
  Helix) that connect to the daemon without understanding the
  `anvil/` method namespace will silently drop
  `anvil/enforcement/ack` notifications (LSP spec: unknown server
  notifications are ignored). If such a client is in
  enforcement-participating mode, the daemon interprets missing
  acks as refusals, escalates through the enforcement ladder, and
  fences the worktree. A Neovim user with a team-mandated
  `.anvil.yaml` would get fenced because their editor does not
  speak an Anvil-specific method. The "editor-agnostic" reach that
  ADR-030 cites as a headline benefit becomes a fencing hazard
  instead.
- **Expected Outcome:** Driver manifest (§2.2) carries an explicit
  `supportedAnvilMethods` array advertising the `anvil/` methods the
  driver implements. A driver that does not advertise
  `anvil/enforcement/ack` is automatically capped at
  Attached/read-only, regardless of what the workspace
  `.anvil.yaml` requests — with a structured warning surfaced to
  the client and the daemon log so the user knows enforcement
  downgraded. Daemon negotiation during handshake enforces this; a
  driver cannot be promoted to Participating without the
  corresponding capability advertisement. The design spec's
  "every LSP client gets Anvil for free" framing is rewritten to
  "every LSP client gets Anvil *diagnostics* for free;
  enforcement-participation requires explicit `anvil/` support".
- **Scope:** `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
  (§2.2 manifest + §3.3 capability state update),
  `crates/anvil-intercept/src/auth.rs` (capability handshake),
  `plans/decisions/030-surface-drivers-supersede-napi-cutover.md`
  (soften the "every LSP client" claim)
- **Dependencies:** DRVR-002 (protocol definition), DRVR-007
  (shares capability-handshake plumbing)
- **Validation:** Integration test: a fake LSP client speaking only
  stock LSP (no `anvil/` methods) connects and is accepted as
  read-only; if `.anvil.yaml` demands enforcement, the daemon
  returns a structured warning and the driver stays Attached.
- **Source:** 2026-04-24 council review M10
  (adversarial-reviewer) — tracked in
  PR #1063.
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Draft

## Risks

- **INTD slippage.** DRVR is blocked on the intercept daemon shipping
  a stable IPC surface. If INTD slips, DRVR slips with it. Mitigation:
  each DRVR work item pins to a specific INTD deliverable
  (INTD-002 / -003 / -005 / -013) rather than the module as a whole,
  and DRVR-001 can start against a mock daemon to decouple the
  TS-side work from daemon progress.
- **LSP extension sprawl.** Anvil's suppression / gate / nudge surfaces
  don't fit stock LSP cleanly. The editor-driver protocol (DRVR-002)
  must bound what's custom; un-capped extension leads to per-editor
  shim code that reintroduces the problem TSRET wanted to solve.
  Mitigation: DRVR-002's design doc includes an explicit "no new
  custom method without a consumer requirement" rule, and
  operations-reviewer gate on DRVR-002 enforces it.
- **Fallback UX.** Editor and MCP behaviour when the daemon is
  unreachable determines whether users perceive the daemon as
  load-bearing infrastructure (bad) or a nice-to-have accelerator
  (fine). Mitigation: DRVR-003 and DRVR-004 both include explicit
  tests for the daemon-down path, and the fallback surfaces a clear
  "degraded" signal rather than failing silently.
- **Enforcement participation.** Editor drivers with enforcement
  capability can fence a worktree or reject saves. That's a
  significant behaviour delta from today's passive diagnostics. The
  default must be read-only; opting in must be explicit per project
  and auditable. Mitigation: the read-only / enforcement-participating
  distinction is baked into DRVR-002's capability handshake, and the
  default is read-only across all fresh installs.
