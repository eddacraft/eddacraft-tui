<!--
APS Module: Real-time AI-Output Validation
==========================================
Validate code as an AI tool generates it — surface reasoning-pattern
violations and structural drift while a tool like Cursor / Claude Code /
Copilot is mid-edit. Sits on the intercept daemon (INTD) plus surface
drivers (DRVR) per ADR-030. Sibling to LAUNCH (which polishes the
save-time watch flow). Supersedes RTVF in this same change.
See: plans/aps-rules.md
-->

# Real-time AI-Output Validation

| ID   | Owner | Status      | Progress |
| ---- | ----- | ----------- | -------- |
| RTAI | —     | In Progress | 8/9      |

**Last reviewed:** 2026-04-30

> **A1 launch slice:** RTAI-001 (Done), RTAI-002, RTAI-003, RTAI-006, RTAI-008.
> RTAI-004 (TS `DriverClient` envelope) and RTAI-005 (VSCode editor-driver
> mid-edit path) are deferred to the post-A1 editor-driver path. RTAI-007
> (telemetry mirror) and RTAI-009 (architecture doc supersession) remain on
> the H2 roadmap but are not part of the A1 cut.

## Purpose

Anvil's headline capability — and the current launch-blocker — is
validating AI-generated code **as it is being generated**, not after
the agent has finished and saved a file. When Cursor / Claude Code /
Copilot is mid-edit, Anvil should be telling the user (and where
possible the agent itself) that the in-flight change is structurally
or semantically off — appeals to authority in a planning doc,
unjustified precision, missing trade-offs, boundary violations,
suppression bypass — *before* the file lands, not after.

This module re-frames that capability against the architecture Anvil
has actually committed to: the **intercept daemon** (INTD) plus
**surface drivers** (DRVR), per ADR-030. The validation engine is
not new — it lives in `anvil-checks` (and is reached via the
intercept rule registry, INTR). What is new is:

1. A **mid-edit request shape** the daemon understands — content
   provided by the driver from an unsaved buffer or in-flight
   tool-call, not read from disk.
2. **At least two driver implementations** — an editor driver
   (`textDocument/didChange` → daemon mid-edit RPC) and an MCP
   driver (`pre-write` tool-call interception) — proving the
   abstraction holds across surfaces.
3. A **latency budget tight enough to feel real-time** for human
   readers and short enough to be useful as agent feedback,
   distinct from and stricter than the save-time budget LAUNCH /
   the existing editor-driver design already cover.

## Background

### Why this module, why now

Real-time AI-output validation has been parked in two stale Draft
modules — `real-time-validation-simplified` (RTVS) and
`real-time-validation-full` (RTVF) — both pre-dating the daemon +
drivers architecture. RTVS was archived 2026-04-24 against the
retired Ink/TS stack; RTVF is **superseded by this module** in the
same change (see "Disposition of RTVF" below).

Two adjacent modules have landed in the meantime that change the
shape of the problem:

- **LAUNCH (launch-flow-readiness)** — owns *save-time* watch-flow
  polish: glob filters, dashboard stat rollup, post-init analysis,
  doctor remediation. LAUNCH is a prerequisite for end-user trust in
  the watch flow but is **not** real-time AI validation. Save-time
  validation fires after the agent has already finished. RTAI fires
  while the agent is still typing.
- **DRVR (surface-drivers)** — defines what a driver is and what
  surfaces become drivers. The existing
  `editor-and-mcp-driver-design.md` §3.4 only covers the save-time
  path (`didSave` → daemon scan → `publishDiagnostics`). The
  mid-edit / pre-write path is unspecified.

### What's already in place

- `INTD-005` (enforcement decision pipeline) reads file content from
  disk before evaluating rules. RTAI needs an additive code path
  that takes content **from the request** instead of from the
  filesystem.
- `INTR-001..-007` define the rule trait and rule set; secret
  detection, antipattern, path-deny, and regex-content rules already
  conform to it. The reasoning-pattern checks RTVS catalogued (AI-001
  appeal-to-authority, AI-002 unjustified precision, etc.) are
  net-new rule implementations on the same trait — they belong in
  `crates/anvil-checks/src/reasoning/` (decided 2026-04-26, see
  Open Question 3), not in this module.
- `INTD-014` measures save-time RPC latency. RTAI needs a separate
  mid-edit latency measurement because the budget is tighter and
  the call rate is higher.
- `DRVR-001` ships `DriverClient` for editor/future TS driver surfaces. The A1
  MCP path no longer waits for it: RMCP owns the Rust stdio launch shim and uses
  RTAI's validation semantics directly.

## Cross-cutting convention

This module follows the **cross-cutting module convention** trialled
by [LAUNCH](./launch-flow-readiness.aps.md) (see its "Cross-cutting
convention" section). Concretely:

1. **Owns its own work items.** Every `RTAI-NNN` task is owned and
   counted here. Progress is local.
2. **Cross-references via callouts.** Where this module relates to
   work owned elsewhere, it uses one of three callouts inside the
   relevant task:
   - **Coordinates with:** referenced item benefits this work but
     does not block it.
   - **Blocks on:** this task cannot land until the referenced item
     completes.
   - **Superseded by:** this task should be closed (with a pointer)
     if the referenced item lands first under a wider scope.
3. **Cleans up after itself.** Cross-references are prose. When a
   task is closed (or this module is archived), the closer reads
   each callout and either confirms the reference still resolves or
   deletes it.

> ⚠️ This is the second use of the convention; the cross-module
> coordination it imposes is real (RTAI depends on INTD, DRVR,
> anvil-checks, and coordinates with LAUNCH). Per LAUNCH's
> "Do not copy this convention to a second module" warning, this
> *is* the second use, and the trigger to promote the pattern to a
> first-class APS primitive (with machine-readable callouts) has
> been reached. Filed as Open Question 1 below.

## Scope

**In scope:**

- A **mid-edit RPC surface** on the intercept daemon
  (`scan_buffer`, per the RTAI-001 spike decision)
  that accepts unsaved buffer content and a path, runs the
  configured rule set without touching disk, and returns
  diagnostics. Distinct from the save-time path.
- A **latency budget** for the interactive buffer class defined by
  ADR-031: `mode = midEdit` and `mode = preWrite` use
  `validation.service` p95 <= 50 ms and `validation.roundtrip` p95 <=
  80 ms on a warm daemon. Save-time uses ADR-031's separate
  interactive save-time class; RTAI must not invent local numbers.
- **Debounce / dedup at the driver edge** — drivers must not flood
  the daemon. The default debounce is 80ms (a typing cycle); the
  daemon also computes a content hash and short-circuits identical
  consecutive requests within a sliding window.
- **Errors as first-class responses** — the mid-edit response
  carries either `diagnostics: [...]` (possibly empty) or a
  structured `error: { ... }`. There is no third state. Drivers
  must surface daemon errors as a degraded mode, not as silent
  pass.
- **Editor-driver mid-edit path** — VSCode extension (and any other
  LSP-shaped client) wires `textDocument/didChange` into a
  driver-side debouncer that calls the new RPC. Diagnostics are
  rendered via `publishDiagnostics` in the same channel as
  save-time results, with a marker distinguishing in-flight from
  on-disk findings.
- **MCP pre-write path** — for the current release, RMCP owns the
  narrow Rust `anvil mcp serve --stdio` launch shim that validates
  proposed writes before they hit disk. Full MCP-driver parity remains
  downstream RMCPF/DRVR work. Failures are surfaced to the agent as a
  structured tool result (not a silent log) so the agent can re-plan
  in-loop.
- **Observability** — every mid-edit decision emits a notification
  on the telemetry lane via the same envelope INTD-013 defined,
  with `correlation.source = "intercept"` and a new
  `mirror.path = "midEdit"` discriminator so subscribers can
  distinguish in-flight from save-time decisions.
- **Phase-0 spike** that proves the architecture end-to-end on a
  thin slice (one driver, one rule, one fixture) **before** this
  module is promoted from Proposed to Ready.

**Out of scope:**

- The validation engine internals (which rule fires, how
  reasoning-pattern detectors are written, false-positive tuning,
  the AI-001..AI-007 catalogue). Owned by **anvil-checks**
  (specifically `crates/anvil-checks/src/reasoning/` per the
  2026-04-26 decision; see Open Question 3). RTAI consumes
  whatever rules are registered in INTR.
- Save-time watch-flow polish — owned by **LAUNCH**.
- The save-time gate itself, the editor diagnostics-on-save
  channel, code actions, suppression UI — owned by **DRVR-002 /
  DRVR-003 / RMCPF** depending on surface.
- LSP / HTTP / stdin server framing implementation — RMCP owns the
  current-release Rust stdio MCP launch shim; DRVR/RMCPF own broader
  driver and MCP-server parity work. RTAI owns validation semantics,
  not the server process.
- Notification fan-out across terminal / desktop / Slack — owned
  by **NOTIFY** (RTAI emits onto the telemetry lane;
  presentation is downstream).
- Auto-fix or AI-powered remediation. Drivers may forward
  diagnostics back to the originating agent; suggesting a fix is
  not RTAI's job.
- Editors beyond VSCode and the MCP server. Cursor and Claude Code
  reach Anvil via either an editor driver (where they speak LSP)
  or the Rust MCP launch shim / future MCP driver path (where they
  speak MCP). No bespoke "Cursor driver" ships in v1.
- Hot-path graph / boundary checks — those are queued behind
  INTR's "no graph recomputation on hot path" rule. Reasoning
  patterns and content checks first; structural checks later if
  the budget permits.

## Dependencies

- **Blocks on:** [INTD-002](../archive/modules/intercept-daemon.aps.md) (IPC
  listener), [INTD-003](../archive/modules/intercept-daemon.aps.md) (session
  registry), [INTD-005](../archive/modules/intercept-daemon.aps.md) (enforcement
  decision pipeline), [INTD-013](../archive/modules/intercept-daemon.aps.md)
  (telemetry mirror), [INTD-014](../archive/modules/intercept-daemon.aps.md)
  (JSON-RPC conformance + latency benchmark — RTAI's mid-edit
  benchmark extends it).
- **Blocks on for editor-driver tasks only:** [DRVR-001](./surface-drivers.aps.md)
  (`DriverClient`) and [DRVR-002](./surface-drivers.aps.md)
  (editor-driver protocol — RTAI extends the method table with
  the mid-edit RPC). The A1 RMCP path does not block on these.
- **Coordinates with:** [DRVR-003](./surface-drivers.aps.md) (VSCode
  extension cutover) — the editor-driver mid-edit path is most
  cheaply built once DRVR-003 is in flight, but RTAI's spike
  (RTAI-001) does not need to wait for DRVR-003 to complete.
- **Coordinates with:** [rust-mcp-launch-shim](./rust-mcp-launch-shim.aps.md)
  (RMCP) — current-release MCP path for pre-write validation in the
  single Rust binary.
- **Coordinates with:** [rust-mcp-full-port](./rust-mcp-full-port.aps.md)
  (RMCPF) and DRVR — next-release full MCP parity and driver-framework
  alignment.
- **Coordinates with:** [LAUNCH](./launch-flow-readiness.aps.md)
  (save-time watch flow) — RTAI is the in-flight sibling. The two
  must produce diagnostics that look the same on the wire so
  consumers don't branch.
- **Coordinates with:** `crates/anvil-checks` — the
  `reasoning/` submodule (per the 2026-04-26 decision; see Open
  Question 3) holds rule implementations the daemon evaluates. The
  reasoning-pattern catalogue from the archived RTVS module belongs
  there, not here.
- **Coordinates with:** [INTR](./intercept-rules.aps.md) — rules
  registered in INTR are what the mid-edit pipeline evaluates.
- **References:** [ADR-030](../decisions/030-surface-drivers-supersede-napi-cutover.md)
  (drivers-on-daemon architecture authority).

## Work Items

> Status: Ready. RTAI-001 completed the Phase-0 spike and promoted this module
> to Ready. The current-release path is RMCP-first: RTAI owns validation
> semantics and contracts, RMCP owns the Rust MCP stdio server/tool surface, and
> DRVR remains the editor/future-driver path.

### RTAI-001: Phase-0 architecture spike (one driver, one rule, one fixture)

- **Intent:** Prove end-to-end that an in-flight buffer change in
  one surface can reach the daemon, evaluate one rule against
  in-memory content, and return a diagnostic inside the
  mid-edit latency budget — before any further task in this
  module is committed to.
- **Expected Outcome:** A throwaway branch demonstrates the full
  loop on a single fixture: a `didChange` event in a fake LSP
  client (or a recorded MCP `apply_edit` payload) reaches a
  prototype daemon endpoint, runs one existing rule (e.g. the
  secret-detection wrapper from INTR-002) against the buffer
  content, and returns a `diagnostics` envelope. p95 round-trip
  measured on the spike fixture; numbers fed back into the
  RTAI-002 budget. Decision recorded inline (in this module or
  as a brief design note under `plans/specs/`) on (a) whether
  to extend the existing daemon RPC or add a new method, and
  (b) whether driver-side debounce belongs in `DriverClient`
  or in each driver.
- **Coordinates with:** INTD-002 (a partial IPC listener is
  enough to spike against; the spike informs INTD's mid-edit
  shape but does not require INTD to be Ready).
- **Validation:** Spike branch lands a working demo; numbers and
  decisions written up; module promotion from Proposed → Ready
  is gated on this task closing.
- **Confidence:** medium
- **Status:** Done — landed on `feat/RTAI-spike` (commit `ad4f0400`).
  Spike measured p95 1.4 ms round-trip on the in-process loop fixture
  (vs ADR-031 mid-edit p95 budget of 80 ms), with one diagnostic per
  round-trip on `secret-detection`. Decisions recorded in
  [`plans/specs/2026-04-26-rtai-001-spike-report.md`](../specs/2026-04-26-rtai-001-spike-report.md):
  (a) single `scan_buffer` RPC method discriminated by `Mode`, not
  per-mode methods; (b) `DriverClient` owns the debouncer, drivers
  parameterise the window. Spike binary: `crates/spike/src/rtai_mid_edit.rs`.

---

### RTAI-002: Daemon mid-edit RPC surface

- **Intent:** Extend the daemon's IPC surface with a mid-edit
  validation RPC that accepts unsaved buffer content and returns
  diagnostics without touching disk.
- **Expected Outcome:** A new JSON-RPC method
  (`scan_buffer`, per the RTAI-001 spike decision) accepts
  `{ path, text, version, mode }` with `mode = midEdit` and returns
  `{ version, diagnostics: [...], truncated }` or a JSON-RPC
  `{ error: { code, message, data? } }`. The enforcement
  pipeline grows a content-injection variant that bypasses the
  disk-read step in INTD-005 but reuses the same rule registry,
  the 1 MB content cap, the binary-detection short-circuit, and
  the existing rule short-circuit semantics. The IPC listener carries
  the configured scan service instead of a static registry and caps
  each response with `truncated = true` when extra diagnostics are
  dropped. The new path is conformance-tested as JSON-RPC 2.0
  alongside INTD-014.
- **Blocks on:** INTD-002, INTD-005, RTAI-001 (decision on
  extend-vs-new-method).
- **Coordinates with:** DRVR-002 (the method must appear in the
  protocol's method table).
- **Validation:** `cargo test -p eddacraft-anvil-intercept midedit &&
  cargo test -p eddacraft-anvil-intercept --test jsonrpc_conformance
  scan_buffer` covers (a) happy-path diagnostics, (b) over-cap
  rejection, (c) binary content short-circuit, (d) malformed
  request returns structured error, (e) rule-registry parity
  with the on-disk path against a fixture matrix, (f) configured
  listener rule-set injection, and (g) worst-case JSON escaping for a
  valid 1 MB buffer.
- **Status:** Complete — merged 2026-04-29 via PR #1186 (`feat/RTAI-002-midedit-rpc`):
  daemon `scan_buffer` JSON-RPC method, content-injection enforcement variant,
  1 MiB cap + binary short-circuit, `ScanBufferService` semaphore + truncation,
  conformance fixtures alongside INTD-014.

---

### RTAI-003: Mid-edit latency benchmark + budget enforcement

- **Intent:** Pin the mid-edit p50/p95 latency the design depends
  on and surface a regression signal in CI before users feel it.
- **Expected Outcome:** A criterion benchmark measures the
  daemon-side cost of a single mid-edit RPC against a
  representative fixture set labelled with ADR-031's required
  dimensions, including small, medium, near-cap, binary,
  Unicode-heavy, and dirty-secret buffers. Recorded baseline
  numbers establish the `mode = midEdit` `validation.service` and
  `validation.roundtrip` p50 / p95 / p99 values against ADR-031's
  interactive buffer SLO. **CI baseline-comparison gating is split
  off as a follow-up** (eddacraft/anvil-001#1191) so this slice
  ships the harness and recorded numbers without taking on the
  workflow-wiring scope.
- **Blocks on:** RTAI-002 (Done — landed as PR #1186).
  **Coordinates with:** INTD-014. The benchmark lives at
  `crates/anvil-intercept/benches/midedit_roundtrip.rs` as a
  standalone harness rather than extending `ipc_roundtrip.rs`,
  because the mid-edit harness needs its own ADR-031 corpus,
  warm-up, and percentile sampler that would have bloated the
  generic IPC bench. Production drivers reuse a persistent
  connection — the round-trip harness mirrors that and references
  `ipc_roundtrip.rs` for the cold-connect cost.
- **Validation:** `cargo bench -p eddacraft-anvil-intercept --bench
  midedit_roundtrip` records baseline locally; CI baseline-comparison
  is tracked under #1191.
- **Confidence:** medium
- **Status:** Complete — merged 2026-04-30 via PR #1189; bench + corpus
  landed (7-case canonical corpus including dirty-secret, binary,
  Unicode, near-cap). CI baseline-comparison gating deferred to
  follow-up issue #1191.

---

### RTAI-004: DriverClient mid-edit envelope + debouncer

- **Release note:** Deferred from A1 after the 2026-04-28 Rust MCP launch-shim
  split. RMCP does not use the TS `DriverClient`; this task remains for the
  editor-driver/future TS driver path.
- **Intent:** Extend `DriverClient` so any driver can emit
  mid-edit requests with a built-in debouncer and content-hash
  dedup, without re-implementing either in each surface.
- **Expected Outcome:** `DriverClient` exposes
  `validateMidEdit({ uri, content, workspaceRoot })` returning
  `Promise<DiagnosticsResult | DaemonError>`. Default debounce
  80ms, configurable per-call. Identical consecutive requests
  (by content hash) within a sliding window short-circuit
  client-side without a round-trip. Cancellation on transport
  drop returns the same `retriable: true` structured error
  DRVR-001 already defines.
- **Blocks on:** DRVR-001, RTAI-002. **Coordinates with:**
  DRVR-002 (typed envelope must match the protocol).
- **Validation:** Unit tests cover: debounce coalesces a typing
  burst into one request, identical content within window
  short-circuits, transport drop cancels in-flight cleanly,
  daemon error surfaces structured (not as a thrown exception).
- **Confidence:** medium
- **Status:** Complete — merged 2026-05-06 via PR #1311 (`a2/wave3-rtai-mid-edit-envelope`); merge gated on DRVR-002 protocol/envelope compatibility, which landed first via PR #1310

---

### RTAI-005: Editor mid-edit path (LSP server surface)

- **Reframed 2026-06-02:** from "Editor-driver mid-edit path (VSCode +
  LSP shape)" to a **generic LSP server** surface. Rationale: an
  `anvil lsp` server is write-once leverage — VS Code, Neovim, Helix,
  Emacs (eglot / lsp-mode), Zed and any LSP client get mid-edit
  validation off one implementation, with no per-editor extension to
  publish and maintain. It fits the single-Rust-binary posture
  (ADR-012) and the drivers-on-daemon architecture (ADR-030): the LSP
  server is a thin frontend over the shipped `scan_buffer` RPC
  (RTAI-002). A VS Code extension, if wanted, becomes an optional thin
  wrapper for richer UX (status bar, code actions) — not the
  foundation. Still parked under ADR-033.
- **Intent:** Expose Anvil's mid-edit validation to any editor that
  speaks LSP by shipping a generic language-server surface (`anvil
  lsp`) that turns `textDocument/didChange` into a daemon `scan_buffer`
  call and publishes the resulting diagnostics — rather than a
  VS Code-specific extension.
- **Expected Outcome:** `anvil lsp --stdio` (daemon-fronted, falling
  back to the embedded engine when no daemon is running) registers a
  `textDocument/didChange` handler, debounces at the driver edge, calls
  the mid-edit `scan_buffer` RPC (RTAI-002), and returns results via
  `textDocument/publishDiagnostics` with a marker (e.g. `data: { phase:
  "midEdit" }`) distinguishing in-flight from on-disk diagnostics. The
  surface is **advisory-only** by nature — LSP `didChange` shows the
  user their own keystrokes, so Anvil cannot *refuse* a write over LSP;
  the refusable lane stays the MCP pre-write path (RTAI-006), per the
  "bypass asymmetry" risk below. A daemon-down state degrades to no
  in-flight diagnostics (clients that surface it show a degraded
  indicator) **without** suppressing save-time diagnostics. Editor
  wiring (a thin VS Code extension, Neovim `lspconfig`, etc.) consumes
  this one server.
- **Blocks on:** RTAI-004 (mid-edit envelope + debouncer). The
  LSP-server reframing **decouples this item from a VS Code-specific
  editor driver** — it no longer hard-blocks on DRVR-002 / DRVR-003 (the
  VS Code extension surface), which become optional thin-wrapper
  follow-ups.
- **Coordinates with:** DRVR-008 (LSP clients that do not advertise the
  mid-edit capability are capped at save-time-only — the `anvil lsp`
  server declares the mid-edit capability in its server capabilities;
  RTAI must extend DRVR-008's handshake, not bypass it).
- **Validation:** An editor-agnostic integration test drives a fake LSP
  `didChange` stream against the `anvil lsp` server (over a live daemon)
  and asserts `publishDiagnostics` arrives within the ADR-031 mid-edit
  budget; a daemon-down test asserts in-flight diagnostics stop while
  save-time still works.
- **Confidence:** medium
- **Status:** Proposed (parked under ADR-033 — IDE/MCP surface
  sequencing)

---

### RTAI-006: MCP pre-write validation semantics

- **Intent:** Define the validation semantics RMCP uses when an MCP client asks
  Anvil to validate a proposed write before it hits disk.
- **Expected Outcome:** RMCP's validation tool accepts proposed write content,
  evaluates it through the daemon or shared Rust validation path, and returns a
  structured tool response the agent can read. If diagnostics are returned at or
  above a configured severity, the response either blocks the write, warns, or
  proceeds with diagnostics attached. The choice is governed by the same
  `.anvil.yaml` enforcement block INTD-008 loads. RTAI owns the semantic
  contract; RMCP owns the Rust stdio MCP server/tool implementation. Full
  per-client write-tool inventory for the next-release parity server belongs to
  RMCPF.
- **Blocks on:** RTAI-002 and RMCP-004/RMCP-005 (Rust MCP launch shim must expose
  the validate tool and validation adapter before this path can be proven).
- **Coordinates with:** RMCP-006 (canonical diagnostics and redaction), RMCPF
  (full MCP parity), and DRVR-007 (driver trust boundary for any future
  driver-participating MCP surface).
- **Validation:** RMCP E2E test drives a fake MCP validate-write call carrying
  content known to trigger the secret-detection rule; asserts the tool response
  carries structured diagnostics and honours the configured enforcement mode.
- **Confidence:** medium
- **Status:** Complete — merged 2026-04-30 via PR #1190 (`feat/RTAI-006-mcp-prewrite`). The MCP
  `validate_write` tool now consults a workspace-level `EnforcementMode`
  (`block` | `warn` | `off`) loaded from `.anvil.yaml` and applies the
  RTAI-006 mapping table (see
  `crates/anvil-cli/src/mcp/enforcement.rs`): `block` rejects on
  `Severity::Error` and warns on lower severities, `warn` always returns
  `warn` when diagnostics are non-empty, `off` always returns `allow` while
  still surfacing diagnostics. Default is `block` — matches the pre-RTAI-006
  behaviour. INTD-008's full `.anvil.yaml` loader stays Draft; the loader
  here reads the same `enforcement.mode` field so the contract remains
  daemon-shareable when INTD-008 lands. E2E coverage in
  `crates/anvil-cli/tests/mcp_validate_write_enforcement.rs` drives
  `anvil mcp serve --stdio` against fixture `.anvil.yaml` files for all
  three modes plus the missing-file default.
- **Reconciliation note (2026-04-30):** RMCP-004/-005/-006 implement the launch
  shim's embedded `anvil_validate_write` path and structured MCP response, but
  the default daemon client still returns `Unavailable` and the write decision
  prior to RTAI-006 was hard-coded from diagnostic severity rather than the
  INTD-008 enforcement block. RTAI-006 closes that semantic gap on the
  embedded path; the daemon-backed path takes over transparently once the
  launch shim's `DaemonValidationClient` is wired to the shipped `scan_buffer`
  RPC (RTAI-002) — a follow-up RMCP/RMCPF task.

---

### RTAI-007: Mid-edit telemetry mirror

- **Intent:** Mirror every mid-edit decision onto the telemetry
  lane using the canonical INTD-013 envelope, so the
  observability story stays "one shape across surfaces".
- **Expected Outcome:** Each mid-edit decision emits an
  `anvil.notification.v1` envelope with `correlation.source =
  "intercept"`, `mirror.decision` set to the outcome class
  (allow / warn / block — interrupt does not apply mid-edit),
  and a new `mirror.path = "midEdit"` discriminator so
  subscribers can split in-flight from save-time without
  parsing the rule id. Cross-session redaction from INTD-015
  applies unchanged.
- **Blocks on:** RTAI-002, INTD-013, INTD-015.
- **Validation:** `cargo test -p eddacraft-anvil-intercept --lib
  midedit_telemetry` asserts envelope shape, discriminator
  presence, and that cross-session subscribers receive
  redacted excerpts per INTD-015.
- **Confidence:** medium
- **Status:** Merged 2026-06-02 via PR #2227 — `MirrorPath` discriminator +
  `midedit_envelope` builder in `crates/anvil-intercept/src/telemetry.rs`; every
  mid-edit decision mirrors onto the `anvil.notification.v1` lane with
  `mirror.path = "midEdit"`, wire-additive (save-time bytes unchanged), advisory
  `ack_required = false`, and INTD-015-redaction-safe. Builder-only, matching the
  save-time `delivered_envelope_for_decision` posture (the live broadcaster is
  MLP2-071 Phase 2). (Promoted to Ready 2026-06-02 — all three `Blocks on`
  items are Complete: RTAI-002, INTD-013, and INTD-015 (merged 2026-05-06 via PR #1305).
  The cross-session redaction this relies on is shipped, so the TRACE R1 risk
  caveat ("revisit when INTD-015 reaches Ready") is discharged. The Wave-4
  deferral was A2-brief sequencing, not a technical block. Daemon-independent
  0.8.0 freight — one envelope-shape addition in `anvil-intercept`.)

---

### RTAI-008: Errors-as-first-class contract test

- **Intent:** Lock down the contract that a mid-edit response is
  always either `diagnostics` (possibly empty) or `error`
  (structured) — never silent pass on failure — before any
  driver consumes it in production.
- **Expected Outcome:** A shared contract test fixture covers
  every documented error code (over-cap content, malformed
  request, daemon-side rule panic isolated, transport timeout,
  cross-session subscription rejection) and every driver that
  consumes the mid-edit RPC must run it. New drivers fail CI
  if they swallow an error into "no diagnostics".
- **Blocks on:** RTAI-002 and RMCP-004/RMCP-006 for the A1 Rust MCP path;
  RTAI-004 is an additional dependency only for future TS `DriverClient`
  consumers.
- **Coordinates with:** RTAI-005, RTAI-006 (consumers of the
  contract — both must run the fixture in their test suites).
- **Validation:** Contract fixture lives in
  `crates/anvil-intercept/tests/midedit_contract.rs` (Rust
  side) and is consumed by RMCP. A TS consumer fixture is added later when
  RTAI-004/DRVR-001 land; CI fails if any active consumer drifts.
- **Confidence:** medium
- **Status:** Complete — merged 2026-04-30 via PR #1188 (`feat/RTAI-008-errors-contract`).
  Public fixtures pin the response
  envelope for over-cap content (`-32602`), malformed request (`-32602`),
  daemon-side rule panic (isolated to empty diagnostics on
  `panic="unwind"` builds), transport timeout (`-32001` / `-32603`),
  and a busy invariant check (`-32000`). The cross-session-rejection
  fixture is gated behind `#[ignore]` because `scan_buffer` does not
  yet take a `sessionId`; resume when session-scoped enforcement is
  wired. The fixture module exposes public `*_request` / `assert_*_response`
  pairs and `FIXTURE_NAMES` so any future cross-crate consumer (the planned
  `crates/anvil-rmcp/` once that crate is created, plus the TS / VSCode
  drivers when their consumer crates land) can drive the same envelope shapes
  through their own transport. As of merge there is no cross-crate consumer
  yet — this contract is the standalone source of truth and the rust harness
  is the only active driver.

---

### RTAI-009: Architecture doc + supersession cross-links

- **Intent:** Update the architecture diagrams and any docs that
  still describe a "validation server" or RTVF-style LSP/HTTP
  path; record the supersession of RTVF in the decision log.
- **Expected Outcome:**
  `docs/architecture/anvil-full-architecture.md` and
  `docs/architecture/rust-architecture-endstate.md` show the
  mid-edit path as drivers → daemon, sharing the rule registry
  with the save-time path. `plans/decisions/DECISION-LOG.md`
  references RTAI as the realisation of the in-flight
  validation thesis. Any onboarding doc that mentions a
  separate validation server is updated or removed.
- **Blocks on:** RTAI-005 and RTAI-006 (so docs reflect a
  shipped state, not an aspiration).
- **Validation:** `grep -r "validation-server\|anvil-server\|RTVF"
  docs/` returns only historical references under archive paths.
- **Confidence:** high
- **Status:** Merged 2026-06-02 via PR #2227 — architecture docs now show the
  mid-edit path as drivers → daemon sharing the rule registry
  (`anvil-full-architecture.md` mid-edit subsection +
  `rust-architecture-endstate.md` Phase-5 note), the stale `RTVS + RTVF` diagram
  node is replaced with RTAI, and `DECISION-LOG.md` records the module-level
  supersession by RTAI as the in-flight validation thesis realisation. Validation
  grep clean. (Promoted to Ready 2026-06-02, scoped to shipped
  surfaces — the RTVF/"validation-server" cleanup, the supersession cross-link, and the
  drivers→daemon MCP path (RTAI-006, Complete) are documentable now. The
  remaining `Blocks on` item RTAI-005 (editor-driver VSCode+LSP path) stays
  **parked under ADR-033** (IDE surface archived, Proposed) — document that path
  as parked/forthcoming rather than gating the whole doc on it. Doc-only,
  daemon-independent 0.8.0 freight.)

## Risks

- **INTD slippage cascades.** RTAI is third in the dependency
  chain (INTD → DRVR → RTAI). If INTD slips, DRVR slips, RTAI
  slips. Mitigation: RTAI-001 (the spike) is deliberately
  designed to start against a partial INTD and inform INTD's
  mid-edit shape before INTD-005 is finalised. Do not let the
  spike become a full implementation in disguise — it ships on
  a throwaway branch.
- **Latency budget still needs production-path evidence.** RTAI-001
  proved the thin in-process loop fits well inside ADR-031's
  interactive buffer SLO, but real IPC transport, representative
  corpus size, and wider rule-set cost remain unmeasured. RTAI-003
  records the production-path numbers using ADR-031's mode,
  boundary, and dimension labels. If the real numbers are worse,
  scope contracts (fewer rules on the hot path, larger debounce,
  etc.) before the budget is loosened.
- **Reasoning-pattern false-positive blast radius.** RTVS
  catalogued seven reasoning patterns with explicit < 10% FP
  rate targets. Mid-edit firing means an FP appears while the
  user is mid-keystroke — significantly more annoying than at
  save time. Mitigation: the rule catalogue is **out of scope
  here**; reasoning patterns ship as `severity: info` by default
  in their owning crate, and project config opts up. RTAI does
  not pick severities.
- **Driver flooding the daemon.** A misbehaving editor or MCP
  client without the debouncer (or with a broken one) can DoS
  the daemon. INTD-016 already plans connection caps and rate
  limits; RTAI-004's client-side debounce is the cooperative
  layer, INTD-016 is the enforcement layer. RTAI must not
  assume the cooperative layer is sufficient.
- **MCP pre-write semantics differ from LSP didChange.** A
  `didChange` is informational (the editor is showing the
  change to the user; Anvil is advisory). A pre-write tool
  call is intercepting an action the agent is about to take
  (Anvil can refuse). The two paths share the RPC but the
  surfaces around them differ. RTAI-005 and RTAI-006 must each
  spell out their consumer contract; conflating them in
  prose-only docs has burned this module's predecessors.
- **Bypass asymmetry between LSP advisory and MCP refusable.**
  Cursor in-buffer edits that skip the MCP tool call route
  through LSP `didChange` (advisory only) — the daemon cannot
  refuse the write because there is no pending tool call to
  refuse. The same physical keystroke can be demo-stable
  (when Cursor routes via `apply_edit` through the Rust MCP shim)
  or demo-fragile (when Cursor edits the buffer directly and
  only fires `didChange`). RTAI-002's protocol must document
  this asymmetry explicitly so demo operators understand which
  scenarios are demo-stable (MCP-routed) vs. demo-fragile
  (in-buffer); the runbook §4.3.iii is the operator-facing
  surface of the same problem. Mitigation: prefer MCP for the
  headline demo path; surface the degraded-mode indicator
  loudly on the editor-driver path so an operator can see
  in real time when a Cursor edit went in-buffer and dodged
  the refusable surface.

## Open questions

1. **Promote the cross-cutting convention to APS itself?** This is
   the second module to use the LAUNCH convention. LAUNCH's own
   warning called for promotion at this trigger. Defer or do-now?
   Filing here rather than fudging it inline.
2. **Does mid-edit ever reach `block` / `interrupt`, or is it
   always advisory?** The MCP pre-write path makes blocking
   technically possible (refuse the tool call). The LSP didChange
   path does not (Anvil cannot prevent the editor from showing
   the user their own keystrokes). If the answer is "MCP can
   block, editor cannot", that asymmetry needs to be in the
   protocol from RTAI-002 — not bolted on later.
3. **AI-001 reasoning-pattern rule home: RESOLVED 2026-04-26 —
   Option (a) decided AND landed.** Extended `crates/anvil-checks`
   with a new `reasoning/` submodule alongside `secret/`,
   `antipattern/`, and `command_safety/`. AI-001
   (appeal-to-authority) ships at
   `crates/anvil-checks/src/reasoning/appeal_to_authority.rs`,
   registers through `run_reasoning_check`, emits canonical
   `Diagnostic` values (`anvil.diagnostic.v1`) tagged
   `Category::Reasoning` per the diagnostic envelope coordination
   spec, and honours `@anvil-ignore AI-001` via the shared
   ADR-029 `parse_suppression` parser. Comment-region only
   (`//`, `/* … */`, `#`, `<!-- … -->`); string content with the
   same prose does not match. Required by the RTAI launch demo
   (Scenario B in the runbook at
   `plans/specs/2026-04-26-rtai-demo-runbook.md`) — without it the
   demo headline degrades to "secret-detection mid-edit". 21 unit
   tests + 2 integration tests + 1 fixture cover positive
   matches, string-content negatives, suppression, and the four
   comment families. Future AI-002..AI-007 land in the same
   submodule on the same registration path. Tracked as task #24
   (now closed). If the reasoning corpus later grows
   infrastructure-heavy (NLP helpers, classifier deps), extract
   on real evidence — `cargo new` + module move is a small
   reverse op. Premature crate split was rejected.
4. **Confidence scoping is uniformly medium.** All RTAI-002
   onwards tasks are marked `Confidence: medium` against a
   stack (INTD + DRVR) that does not yet exist. RTAI-001 will
   raise or lower these collectively; do not promote this
   module to Ready before that calibration runs.
5. **Mid-edit + suppression UX.** The save-time path has a
   suppression model (per DRVR-002). What does suppression mean
   on a mid-edit diagnostic that has no on-disk anchor yet? Out
   of scope for v1 — but the protocol must not preclude it.
