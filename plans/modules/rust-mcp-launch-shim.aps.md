# Rust MCP Launch Shim

| ID   | Owner | Status      | Progress |
| ---- | ----- | ----------- | -------- |
| RMCP | —     | In Progress | 5/8      |

**Last reviewed:** 2026-04-28

## Purpose

Ship the launch-critical MCP path in the single Rust `anvil` binary without
porting the whole existing TypeScript MCP server.

The release-sized path is:

```text
anvil mcp install
  -> editor launches anvil mcp serve --stdio
  -> Rust MCP server handles pre-write validation
  -> Rust validation returns canonical diagnostics
  -> MCP response warns or blocks before the write lands
```

**Why:** The current release needs a credible Cursor / Claude Code demo where a
developer installs Anvil, configures MCP, and gets pre-write validation from the
same shipped binary. The generated MCP config already points at
`anvil mcp serve --stdio`; making that command real in Rust avoids adding a
Node.js sidecar, a TS `DriverClient`, and another packaging path to the launch
critical flow.

## In Scope

- `anvil mcp serve --stdio` in Rust as a narrow stdio MCP server
- Minimal MCP protocol subset needed by Cursor and Claude Code for the A1 path
- Minimal pre-write / validate tool surface for proposed file writes
- Validation via the Rust daemon when available, or a shared Rust validation path
  when the daemon is not yet available in the current release slice
- Canonical diagnostic response shape aligned with AIGUARD/RTAI/INTD envelopes
- Warn/block decision mapping for MCP clients before they write content
- `anvil mcp install --client cursor|claude-code` wiring and verification for
  the Rust stdio command
- Release/demo smoke tests for Cursor and Claude Code configuration

## Out of Scope

- Porting all existing `packages/mcp-server` tools, resources, and prompts
- Streamable HTTP transport for the Rust MCP server
- Full DRVR-004 GateRunner replacement
- Building a TS `packages/anvil-driver-client/` bridge only for MCP
- A general MCP platform rewrite
- Graph context tools, `graph://` resources, or assistant context slicing
- Auto-fix, suppression editing, gate running, status queries, boundary queries,
  and prompt surfaces beyond the minimal launch path

## Interfaces

**Depends on:**

- `crates/anvil-cli` — command dispatch, MCP config generation, packaging
- `crates/anvil-checks` — launch rule execution, including secret detection and
  the A1 reasoning-pattern rule
- `crates/anvil-kernel-types` — canonical `Diagnostic` / `anvil.diagnostic.v1`
  shape published by AIGUARD-002
- RCLI3-016 — existing `mcp-config` command that writes
  `anvil mcp serve --stdio`
- RCLI3-016b — `anvil mcp install` wrapper, pulled forward to A1
- RTAI — pre-write/mid-edit validation semantics
- AIGUARD-002 / diagnostic envelope coordination — response schema alignment
- INTD when available — daemon-backed validation path

**Exposes:**

- `anvil mcp serve --stdio`
- `anvil mcp install --client cursor|claude-code`
- Minimal MCP tool for validating proposed writes before they land
- Canonical JSON diagnostics suitable for Cursor / Claude Code tool responses

## Constraints

- UK English spelling in all plan text and user-facing docs
- The command must live in the Rust binary shipped by the current release
- The launch path must not require Node.js, pnpm, or `packages/mcp-server`
- Stdio framing must be deterministic and must never print human logs to stdout
- The server must fail closed for validation errors when a write would otherwise
  proceed unsafely; operational errors return structured retriable errors
- Tool names and response fields must be stable enough for the release demo but
  may be superseded by the next-release full MCP port
- The implementation must be small enough to delete or subsume when RMCPF lands

## Prerequisites

- RCLI3-016 complete: generated configs already point to
  `anvil mcp serve --stdio`
- RCLI3-016b promoted or implemented for the install wrapper
- Diagnostic envelope coordination resolves the canonical fields returned by the
  validate tool
- At least one launch rule exists in Rust (`secret-detection` plus the A1
  reasoning-pattern rule if available)

## Implementation Start Checklist

This module is **In Progress** because the scope is intentionally narrow and the
generated config already depends on this command existing. Re-check before
starting implementation:

- [ ] Tool name and request shape agreed with RTAI owner
- [x] Canonical diagnostic envelope fields agreed via `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`
- [ ] Cursor and Claude Code config paths verified by RCLI3-016/RCLI3-016b
- [ ] Decision recorded on daemon-first vs embedded-fallback validation order
- [ ] Demo runbook updated to name this module as the MCP launch path

---

## Phase 0 — Scope Lock

### RMCP-001: Rust MCP launch-shim contract

- **Status:** Complete
- **Intent:** Freeze the launch-sized MCP contract so implementation does not
  drift into a full server port.
- **Expected Outcome:** A short spec names the supported MCP methods, minimal
  tool surface, diagnostic response shape, daemon/shared-validation fallback,
  and explicit non-goals.
- **Validation:** Spec review confirms no existing TS MCP resources/prompts/tools
  beyond pre-write validation are in scope
- **Files:** `plans/specs/rust-mcp-launch-shim.md`,
  `plans/modules/rust-mcp-launch-shim.aps.md`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** None

---

## Phase 1 — Rust Stdio Server

### RMCP-002: `anvil mcp serve --stdio` command surface

- **Status:** Complete
- **Intent:** Add the Rust CLI command that editor configs already reference.
- **Expected Outcome:** `anvil mcp serve --stdio` starts a stdio MCP server,
  reserves stdout for protocol frames, routes logs to stderr, and exits cleanly
  on EOF/shutdown.
- **Validation:** CLI integration test launches the command, sends a minimal MCP
  initialise frame, and observes a valid JSON-RPC response
- **Files:** `crates/anvil-cli/src/main.rs`,
  `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** RMCP-001

---

### RMCP-003: Minimal MCP protocol subset over stdio

- **Status:** Complete
- **Intent:** Implement only the MCP protocol surface required for Cursor and
  Claude Code to discover and call the launch validation tool.
- **Expected Outcome:** Server handles initialise/ready flow, `tools/list`,
  `tools/call`, and shutdown/exit using JSON-RPC over stdio with clear
  structured errors for unsupported methods.
- **Validation:** Protocol tests cover valid frames, malformed JSON, unsupported
  methods, and clean shutdown without stdout log pollution
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/tests/mcp_serve_stdio.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** RMCP-002

---

### RMCP-004: Minimal pre-write validation tool

- **Status:** Complete
- **Intent:** Expose one MCP tool that validates a proposed write before the
  client applies it.
- **Expected Outcome:** Tool accepts path, proposed content or patch, operation
  type, and optional workspace root; returns `allow`, `warn`, or `block` with
  canonical diagnostics and correlation metadata.
- **Validation:** Unit tests cover clean content, secret-detection block,
  reasoning-pattern warning/block, missing path, binary/oversize content, and
  workspace escape rejection
- **Files:** `crates/anvil-cli/src/mcp/tools/validate_write.rs`,
  `crates/anvil-kernel-types/src/diagnostics.rs`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** RMCP-003, diagnostic envelope coordination

---

## Phase 2 — Validation Backend

### RMCP-005: Daemon or shared validation path adapter

- **Status:** Complete
- **Intent:** Route MCP pre-write requests to Rust validation without requiring a
  TypeScript bridge.
- **Expected Outcome:** Adapter exposes a `DaemonValidationClient` trait. The
  default implementation returns `Unavailable` for the launch slice because no
  concrete daemon IPC listener or pre-write RPC exists yet; MCP then falls back
  to the embedded Rust rule pipeline. The concrete daemon client lands with
  RTAI-002 and INTD-002 once the RPC contract and IPC listener are pinned.
- **Validation:** Tests run the same fixture through daemon-backed and
  embedded-fallback validation and assert matching diagnostics
- **Files:** `crates/anvil-cli/src/mcp/validation.rs`,
  `crates/anvil-checks/src/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** RMCP-004, RTAI validation semantics; RTAI-002 and INTD-002
  own the concrete daemon client and are out of scope for RMCP-005
- **Notes:** 2026-04-28 council review narrowed RMCP-005 to the launch-slice
  daemon seam plus embedded fallback; production daemon RPC wiring is deferred
  until the downstream RPC and IPC work exists.

---

### RMCP-006: Canonical diagnostics and decision mapping

- **Status:** Ready
- **Intent:** Ensure MCP responses use the same diagnostic vocabulary as the
  Rust CLI, daemon, and AI guardrail profile.
- **Expected Outcome:** MCP response carries canonical diagnostics plus a clear
  decision: `allow`, `warn`, or `block`. Blocking findings are phrased so agents
  can re-plan without seeing secret content.
- **Validation:** Golden JSON fixtures match the canonical diagnostic envelope;
  secret fixtures redact sensitive excerpts by default
- **Files:** `crates/anvil-kernel-types/src/diagnostics.rs`,
  `crates/anvil-cli/src/mcp/tools/validate_write.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** RMCP-004, AIGUARD-002

---

## Phase 3 — Install and Verification

### RMCP-007: `anvil mcp install` integration for Cursor and Claude Code

- **Status:** Ready
- **Intent:** Make the install command configure clients to launch the Rust MCP
  shim and verify the entry is usable.
- **Expected Outcome:** `anvil mcp install --client cursor|claude-code` writes an
  `anvil` server entry using command `anvil` and args
  `mcp serve --stdio`, is idempotent, and provides `--verify` output that fails
  if the configured command is missing or malformed.
- **Validation:** Fresh install then verify exits zero for Cursor and Claude
  Code fixtures; idempotent re-run leaves config byte-identical
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/src/commands/mcp_config.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** RCLI3-016, RCLI3-016b, RMCP-002

---

### RMCP-008: Launch smoke tests and demo runbook update

- **Status:** Ready
- **Intent:** Prove the release path end-to-end and keep the operator runbook
  aligned with the Rust server.
- **Expected Outcome:** E2E smoke starts `anvil mcp serve --stdio`, lists tools,
  calls the validate tool on a safe fixture and a blocked fixture, and verifies
  the demo runbook references the Rust MCP path rather than the TS sidecar.
- **Validation:** `pnpm --filter @eddacraft/anvil-e2e test:smoke` includes the
  MCP launch shim when the Rust binary is available; manual runbook pass against
  Cursor or Claude Code recorded
- **Files:** `apps/e2e/src/`,
  `plans/specs/2026-04-26-rtai-demo-runbook.md`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RMCP-007

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Scope expands into full MCP server parity | Medium | High | RMCP-001 locks the launch contract; RMCPF owns full port next release |
| MCP clients do not reliably call the validation tool before writes | Medium | High | Demo runbook pins client behaviour; unsupported in-buffer edits remain a known RTAI risk |
| Daemon dependency slips the release path | Medium | High | RMCP-005 allows an embedded shared Rust validation fallback with identical response semantics |
| Stdio logs corrupt JSON-RPC frames | Medium | High | RMCP-002/RMCP-003 reserve stdout for protocol and test log pollution |
| Diagnostics diverge from CLI/daemon schemas | Medium | Medium | RMCP-006 depends on AIGUARD/RTAI diagnostic envelope coordination |
| Secret content leaks to remote agent context | Medium | High | RMCP-006 redacts sensitive excerpts by default and returns actionable rule metadata |

## Decisions

1. **Rust first for launch** — the current release uses the shipped `anvil`
   binary for MCP stdio instead of a Node/TS sidecar.
2. **Launch shim, not full port** — only pre-write validation ships now. The
   existing TS MCP server remains the owner of legacy tools/resources/prompts
   until RMCPF.
3. **Daemon preferred, embedded fallback allowed** — validation should use the
   daemon when present, but the release path may call the shared Rust validation
   pipeline directly if daemon scope would slip A1.
4. **Canonical diagnostics only** — MCP responses do not invent their own schema.
5. **Full port next release** — full parity with `packages/mcp-server` is planned
   separately in RMCPF.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Scope Lock | 1 | Complete |
| 1 — Rust Stdio Server | 3 | Complete |
| 2 — Validation Backend | 2 | In Progress |
| 3 — Install and Verification | 2 | Ready |
| **Total** | **8** | **5/8 Done** |
