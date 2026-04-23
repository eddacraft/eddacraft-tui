<!--
APS Module: Surface Drivers
===========================
Cut VSCode and MCP over to drivers on the anvil-intercept daemon,
superseding TSRET-003/-004. Per ADR-030. See: plans/aps-rules.md
-->

# Surface Drivers

| ID   | Owner | Status |
| ---- | ----- | ------ |
| DRVR | —     | Draft  |

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
  (daemon mode, Phase 5, deferred) are upstream dependencies for the
  stable socket transport this module rides on.

## Scope

**In scope:**

- Editor driver (VSCode extension) cut over from
  `@eddacraft/anvil-core/antipattern` to a JSON-RPC client of the
  daemon. LSP for the parts the protocol covers (`textDocument/diagnostic`,
  `textDocument/codeAction`); custom Anvil extensions for what it
  doesn't (suppression state, gate results, nudge metadata).
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

- **INTD-004** (IPC listener, NDJSON framing) — required for DRVR-001
- **INTD-005** (session registry) — required before any driver can
  register
- **INTD-010** (violation emission) or equivalent telemetry surface —
  required before the editor driver can render diagnostics
- **KERN-050** (Unix socket transport, JSON-RPC 2.0) — retained from
  KERN Phase 5; required if the daemon re-hosts kernel events over the
  same transport rather than bridging internally
- **ADR-030** — this module's authority
- **Driver-framework ADR** (`plans/specs/anvil-driver-framework/`) —
  defines driver capabilities, enforcement ladder, and the two-lane
  transport model

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
- **Scope:** `packages/anvil-driver-client/`
- **Dependencies:** INTD-004, INTD stable IPC wire-format doc
- **Validation:** Unit tests cover the JSON-RPC framer, NDJSON split,
  reconnection under dropped sockets, and happy-path request/response
  roundtrip against a fake daemon. Integration test connects to a real
  daemon binary and exercises `session.register` + `session.heartbeat`.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft

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
- **Scope:** `plans/specs/`, shared contracts package
- **Dependencies:** DRVR-001 (transport), driver-framework ADR
- **Validation:** Reviewed by one member each of: architect,
  pragmatic-lead, operations-reviewer. Matches the driver-framework
  ADR's capability vocabulary.
- **Confidence:** medium
- **Priority:** High
- **Status:** Draft

---

### DRVR-003: VSCode extension cut over to editor driver

- **Intent:** Every scanner-adjacent call path in the extension
  (`embeddedAnalysis.ts`, diagnostics service, nudge code actions) goes
  through the driver client instead of `@eddacraft/anvil-core/antipattern`.
- **Expected Outcome:** `packages/vscode-extension/src/services/embeddedAnalysis.ts`
  no longer imports `@eddacraft/anvil-core/antipattern`. Diagnostics,
  code actions, and pattern-registry queries route through
  `DriverClient`. Save-time latency budget (< 200ms p95 for files under
  the standard fixture sizes) held or justified. Existing extension
  tests pass after refactor; one new test covers the fallback path
  when the daemon is unreachable.
- **Scope:** `packages/vscode-extension/`
- **Dependencies:** DRVR-001, DRVR-002, INTD-004, INTD violation stream
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
- **Dependencies:** DRVR-001, DRVR-002, INTD-004, INTD rule evaluation
  surface exposed over RPC
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

## Risks

- **INTD slippage.** DRVR is blocked on the intercept daemon shipping
  a stable IPC surface. If INTD slips, DRVR slips with it. Mitigation:
  each DRVR work item pins to a specific INTD deliverable
  (INTD-004 / -005 / -010) rather than the module as a whole, and
  DRVR-001 can start against a mock daemon to decouple the TS-side
  work from daemon progress.
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
