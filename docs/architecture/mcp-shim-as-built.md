# anvil MCP Shim — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                       |
| -------- | --------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | RMCP  | Live   | Last reviewed 2026-07-14 against ADR-106, `crates/anvil-cli/src/activation/agent_registry.rs`, and `crates/anvil-cli/src/commands/mcp*.rs`; prior enforcement review 2026-07-04 against ADR-098 |

| Upstream                                                                                       | Downstream                                                                |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `crates/anvil-cli`, `crates/anvil-intercept`, `crates/anvil-intercept-proto`, ADR-033, ADR-106 | First-wave MCP registry, activation orchestrator, public integration docs |

> **Status:** Live (beta) **Last reviewed:** 2026-07-04 (ADR-098 AD-3:
> enforcement vocabulary unified on the shared `EnforcementMode`; decision
> vocabulary gains `fence`/`interrupt`/`unknown`; `isError`/safe-default gate on
> `is_veto()`; MCP default posture `Interrupt`, the `block` alias); prior sweep
> 2026-07-02 (tool registry now 14 tools incl. 6 GCTX graph-context tools,
> repinned registry.rs line refs) against main `d1fded280`; delta review
> 2026-06-10 (RMCPF-010..-012 tool registry, validate_write schema additions,
> gap register) against main `45dd1047a`; full review 2026-05-07 against
> `v0.6.0-beta` slate (HEAD `97b61fd0`) **Crate / location:**
> `crates/anvil-cli/src/mcp/` (+ `commands/mcp*.rs`) **Module owner (APS):**
> RMCP (`plans/archive/modules/rust-mcp-launch-shim.aps.md`, 8/8 complete)
> **Used by:** first-wave MCP clients (call the shim's 14 registry tools over
> stdio; `anvil_validate_write` is the load-bearing pre-write gate); the
> activation orchestrator
> (`crates/anvil-cli/src/activation/orchestrator/mod.rs:112`) writes the MCP
> entries during `anvil start`

## 1. Overview

The MCP shim is the MCP server bundled with the `anvil` binary that exposes
Anvil's MCP tools (headlined by `anvil_validate_write`) over stdio for Cursor
and Claude Code, routing validation through the local intercept daemon when
available and a correctness-equivalent embedded scanner when not. It is the
validation-write surface that AI editors call **before** AI-generated writes hit
disk; honouring a `block` decision is what turns the editor's write tool from
"AI guesses" into "AI guesses that the daemon already vetted".

The shim's registry ships **14 tools**
(`crates/anvil-cli/src/mcp/tools/registry.rs:30-148`; the count is pinned by
`assert_eq!(tools.len(), 14)` at `registry.rs:166`). The original eight are
`anvil_validate_write`, `anvil_apply_patch`, `anvil_status`, `anvil_check`,
`anvil_gate`, `anvil_query_boundary`, `anvil_suppress`, and `anvil_fix` — the
RMCPF-010 / -011 / -012 port of the catalogue whose prior home was the legacy
Node MCP server (`@eddacraft/anvil-mcp-server`), now superseded in-shim. Six
read-only **GCTX graph-context tools** were added under GCTX-010..023 / ADR-084:
`anvil_search_symbols`, `anvil_find_dependents`, `anvil_find_callers`,
`anvil_impact_of_change`, `anvil_affected_tests`, and `anvil_symbol_context`
(registry rows at `registry.rs:96-147`). The six GCTX tools set
`charges_graph_egress: true` (`ToolDefinition` field at `registry.rs:15`) — a
successful payload is charged against the per-session `graph://` egress byte
ceiling, the same credit `resources/read` spends (CIB-091d). Auth is data-driven
via `ToolDefinition.requires_auth` (`registry.rs:10`), gated at
`commands/mcp.rs:365-367`; `anvil_suppress` and `anvil_fix` keep
`requires_auth: false` for parity with the archived TS server pending the
RMCPF-011 authority review (`registry.rs:73-91`), and the six GCTX tools keep
`requires_auth: false` because their real authority gate is the daemon-side
workspace-root admission (ADR-084 C3 / CE-8), not the MCP auth cache
(`registry.rs:92-95`). See `docs/public/anvil/integrations/mcp.md` for the
public-side comparison.

The shim sits at the trust boundary between the editor and the daemon:

- The editor speaks JSON-RPC over stdin/stdout to the shim
  (`crates/anvil-cli/src/commands/mcp.rs:193-226`).
- The shim either talks UDS to the daemon
  (`crates/anvil-cli/src/mcp/validation.rs:131-141`) or runs the embedded
  scanner in-process (`validation.rs:385-405`).
- Either path returns a canonical `anvil.mcp.validate-write.v1` response
  carrying a decision, redacted diagnostics, and a correlation envelope.

## 2. Architecture diagram

```text
   ┌─────────────────────┐         ┌─────────────────────┐
   │  Cursor / Claude    │         │  anvil start         │
   │  Code (MCP client)  │         │  (activation)        │
   └─────────┬───────────┘         └──────────┬──────────┘
             │ stdio JSON-RPC                  │ writes editor config
             │                                 ▼
             │                 ┌─────────────────────────────┐
             │                 │ ~/.cursor/mcp.json          │
             │                 │ ~/.claude.json              │
             │                 │ (point at anvil mcp serve)  │
             │                 └──────────┬──────────────────┘
             │                            │ editor restart spawns shim
             ▼                            ▼
   ┌──────────────────────────────────────────────────────┐
   │ anvil mcp serve --stdio                               │
   │ commands/mcp.rs::run_stdio_server (193-226)            │
   │  • initialize / tools/list / tools/call               │
   │  • auth gate (commands/mcp.rs:364-368)                │
   │  • dispatch to validate_write::call (mcp.rs:368)      │
   └────────────────────┬─────────────────────────────────┘
                        │
                        ▼
   ┌──────────────────────────────────────────────────────┐
   │ mcp::tools::validate_write::call_with_validation_     │
   │ client (tools/validate_write.rs:117-183)              │
   │  • parse + workspace-escape reject                    │
   │  • read .anvil.yaml enforcement.mode                  │
   │  • call DaemonValidationClient                        │
   │  • redact secrets in response (374-401)               │
   │  • build correlation envelope (320-333)               │
   └────────────────────┬─────────────────────────────────┘
                        │
       ┌────────────────┴────────────────┐
       │                                  │
       ▼                                  ▼
   ┌──────────────────┐         ┌────────────────────────┐
   │ LocalDaemon-     │         │ embedded_validate_     │
   │ ValidationClient │         │ pre_write              │
   │ (validation.rs   │         │ (validation.rs         │
   │ 126-148)         │         │ 385-405)               │
   └──────┬───────────┘         └──────┬─────────────────┘
          │ UDS, Unix only             │ in-process,
          │ cfg(unix) gate at          │ EnforcementPipeline
          │ validation.rs:142-148      │ (anvil-intercept)
          ▼                            │
   ┌──────────────────┐                │
   │ anvil-intercept  │                │
   │ daemon           │                │
   │ scan_buffer JSON-│                │
   │ RPC method       │                │
   └──────────────────┘                │
          │                            │
          └────────────┬───────────────┘
                       ▼
        ValidationResult { backend, daemon_status, diagnostics }
```

The activation side-arm (top right) is documented at
`docs/architecture/activation-as-built.md` §"MCP install (LAUNCH-009)".

## 3. Process model

The shim is a **child of the editor**. Each editor restart spawns a fresh
`anvil mcp serve --stdio` process; the shim itself is not a long-lived daemon.
That role belongs to `anvil-intercept` (see
`docs/architecture/intercept-as-built.md` §3 Process model).

The shim's lifetime is one editor session. It blocks on stdin reading
NDJSON-framed JSON-RPC frames (`commands/mcp.rs:198-223`); on EOF or an `exit`
notification (`commands/mcp.rs:220-222, 250-254`) it exits. There is no PID
file, no socket, no fence state — the shim owns no persistent state of its own.
All persistent state lives on the other side of the IPC boundary (the daemon
owns it; see intercept-as-built §7 fence persistence and §10 registry).

**Trust boundary:** the shim runs as the editor's user. The peer-cred check that
the daemon enforces (`crates/anvil-intercept/src/ipc.rs:251-295`) sees the
shim's UID, which is the user's UID; same-UID trust is the only IPC contract.
The shim has no remote surface, no TLS, no signed manifests. Cross-link:
intercept-as-built §5 "Authentication and trust boundary".

**Stdio frame budget.** `MAX_STDIO_FRAME_BYTES = 4 MiB` (`commands/mcp.rs:24`).
Validate-write caps `proposedContent` at 1 MiB (`validate_write.rs:20`); the
larger frame budget covers worst-case JSON string escaping and the JSON-RPC
envelope. Oversize lines are discarded with the rest of the line
(`commands/mcp.rs:261-299`) so the next frame is parsed cleanly.

## 4. Tool surface

The shim exposes MCP tools through the registry at
`crates/anvil-cli/src/mcp/tools/registry.rs` (`registry.rs:30-148`). Each tool
supplies its descriptor, dispatch function, and auth policy
(`ToolDefinition.requires_auth`, `registry.rs:10`) plus a graph-egress flag
(`ToolDefinition.charges_graph_egress`, `registry.rs:15`). Pins below are
relative to `crates/anvil-cli/src/mcp/tools/`. The six GCTX rows (from
`anvil_search_symbols` down) set `charges_graph_egress: true`; the original
eight set it `false`:

| Tool                     | Pin                      | Auth | Purpose                                                                    |
| ------------------------ | ------------------------ | ---- | -------------------------------------------------------------------------- |
| `anvil_validate_write`   | `validate_write.rs:19`   | yes  | Pre-write validation gate over proposed content (deep-dive below)          |
| `anvil_apply_patch`      | `apply_patch.rs:16`      | yes  | Validate a unified diff before applying it                                 |
| `anvil_status`           | `status.rs:9`            | no   | Read-only workspace-health summary                                         |
| `anvil_check`            | `check.rs:14`            | no   | Antipattern validation; architecture-check parity deferred (`check.rs:21`) |
| `anvil_gate`             | `gate.rs:15`             | no   | Quality gate / planless antipattern scan                                   |
| `anvil_query_boundary`   | `query_boundary.rs:38`   | no   | Can-file-import-file boundary query                                        |
| `anvil_suppress`         | `suppress.rs:37`         | no   | Time-boxed suppression comment (default 30 days, max 365)                  |
| `anvil_fix`              | `fix.rs:33`              | no   | Deterministic auto-fixes for AP-001 / AP-003 / AP-004                      |
| `anvil_search_symbols`   | `search_symbols.rs:26`   | no   | GCTX identity-only symbol search (charges `graph://` egress; GCTX-010)     |
| `anvil_find_dependents`  | `find_dependents.rs:29`  | no   | GCTX file-keyed dependents traversal (charges `graph://` egress; GCTX-011) |
| `anvil_find_callers`     | `find_callers.rs:30`     | no   | GCTX symbol-keyed caller traversal (charges `graph://` egress; GCTX-014)   |
| `anvil_impact_of_change` | `impact_of_change.rs:29` | no   | GCTX change-impact report over changed paths (charges egress; GCTX-012)    |
| `anvil_affected_tests`   | `affected_tests.rs:30`   | no   | GCTX affected-tests + coverage-gap report (charges egress; GCTX-013)       |
| `anvil_symbol_context`   | `symbol_context.rs:25`   | no   | GCTX bounded symbol-context slice (charges `graph://` egress; GCTX-023)    |

`anvil_validate_write` remains the load-bearing tool of this surface and is the
deep-dive for the rest of this section. It is defined at
`crates/anvil-cli/src/mcp/tools/validate_write.rs:19-79`:

```text
name:        "anvil_validate_write"
schema:      "anvil.mcp.validate-write.v1"
description: "Pre-write validation gate. Call this tool before EVERY file
              write … honour `block` decisions; do not write files the tool
              refuses."
annotations: { readOnlyHint: true, destructiveHint: false,
               idempotentHint: true, openWorldHint: false }
```

The MCP tool annotations block is at `validate_write.rs:72-77`.

`anvil_status` is defined at `crates/anvil-cli/src/mcp/tools/status.rs`. It
returns a local, read-only workspace-health summary from a canonicalised
workspace root under the MCP server root:

```text
name:        "anvil_status"
description: "Quick project health summary. Returns available checks,
              configuration info, and baseline status."
```

### 4.1 Request shape (input schema)

| Field             | Type                                            | Required                                    | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------- | ----------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `workspaceRoot`   | string (absolute path)                          | no                                          | Defaults to the shim's cwd. Must match the shim cwd if provided; a mismatch yields `untrusted-workspace-root` whose error payload carries `expectedWorkspaceRoot` — the shim's **canonicalised** cwd (resolved symlinks, `/private`-prefixed on macOS, etc.) — so callers can self-correct (CIB-007). The canonical form is intentional and may differ from the path the caller supplied; trust boundary is unchanged.                                                                                  |
| `path`            | string                                          | yes                                         | Workspace-relative or absolute-inside-`workspaceRoot`.                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `operation`       | enum: `create` / `update` / `delete` / `rename` | yes                                         | Only `create` and `update` consume post-image content; `delete` and `rename` ignore both `proposedContent` and `patch`. See `Operation::requires_content` in `validate_write.rs`.                                                                                                                                                                                                                                                                                                                       |
| `proposedContent` | string (UTF-8)                                  | for `create` / `update` when `patch` absent | Full post-operation file content. Capped at 1 MiB. NUL bytes rejected. May be omitted when `patch` is supplied (CIB-005).                                                                                                                                                                                                                                                                                                                                                                               |
| `patch`           | string                                          | optional                                    | Unified diff. When supplied without `proposedContent`, the validator reads the on-disk file at `workspaceRoot`+`path`, applies the patch in memory, and validates the post-image through the same pipeline as a full-content payload — the disk file is never written. Apply failures surface as `patch-apply-failed`; an unreadable target surfaces as `patch-target-unreadable`. When both fields are supplied `proposedContent` is authoritative and `patch` is correlation metadata only (CIB-005). |
| `contentSha256`   | string (hex)                                    | optional                                    | SHA-256 of the full proposed content. Paired with `preview` to send a slim payload without `proposedContent`; the shim flags such requests with `correlation.partialScan` (`validate_write.rs:51-58`).                                                                                                                                                                                                                                                                                                  |
| `preview`         | string                                          | optional                                    | First lines of the proposed content, used for partial validation when `proposedContent` is omitted (`validate_write.rs:51-58`).                                                                                                                                                                                                                                                                                                                                                                         |
| `contentEncoding` | enum: `utf-8`                                   | optional                                    | Anything else rejected with `unsupported-encoding`. See `ValidateWriteRequest::parse` in `validate_write.rs`.                                                                                                                                                                                                                                                                                                                                                                                           |
| `client`          | object                                          | optional                                    | Free-form passthrough.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

### 4.2 Response shape

```jsonc
{
  "schema": "anvil.mcp.validate-write.v1",
  "decision": "allow" | "warn" | "block" | "fence" | "interrupt" | "unknown",
  "summary": { "total": N, "bySeverity": { "error": N, "warning": N, "info": N } },
  "diagnostics": [ /* anvil.diagnostic.v1 envelopes, redacted for secret category */ ],
  "correlation": {
    "id": "corr_mcp_<sanitised-path>",
    "surface": "mcp",
    "mode": "preWrite",
    "backend": "daemon" | "embedded",
    "daemonStatus": "available" | "not-wired" | "unavailable",
    "path": "<workspace-relative-path>",
    "enforcementMode": "off" | "warn" | "fence" | "interrupt",
    "partialScan": true                // present iff slim preview payload
  },
  "safeDefault": "do-not-write",       // present iff decision.is_veto() (block | fence | interrupt)
  "protection_claim": { /* … */ }      // present iff daemonStatus == "available"
}
```

Built by `validation_payload_with_decision` at `validate_write.rs:440-498`. Two
additions landed after the v0.6.0-beta review:

- `correlation.partialScan: true` is set when a slim `contentSha256` + `preview`
  payload was validated without full `proposedContent`
  (`validate_write.rs:473-475`); the field is omitted otherwise.
- Top-level `protection_claim` (MLP2-051b) is present iff
  `daemonStatus == "available"`. It is fetched via `query_protection_claim`
  under a 500 ms budget (`MCP_PROTECTION_CLAIM_QUERY_TIMEOUT`,
  `validation.rs:47-49`) and attached at `validate_write.rs:489-498`. The field
  is wire-additive: the trait default returns `None` (`validation.rs:162`), so
  embedded / no-daemon responses omit it entirely — pinned by the fixture test
  at `validate_write.rs:1458-1467`.

The `decision` enum is `anvil_kernel_types::diagnostics::ControlDecision`. The
MCP transport wraps this payload in the standard
`{ content: [{type: "text", text: <json>}], isError }` shell at
`validate_write.rs:185-197`.

### 4.3 JSON-RPC method names (editor → shim)

The shim implements a narrow MCP subset over JSON-RPC 2.0
(`commands/mcp.rs:228-248`):

| Method                      | Action                                                                                                                                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `initialize`                | Returns `protocolVersion`, capabilities `{ tools: {} }`, instructions, and `serverInfo` (`mcp.rs:301-325`). Default protocol version is `2024-11-05` (`mcp.rs:16`). |
| `notifications/initialized` | No-op.                                                                                                                                                              |
| tools/list                  | Returns descriptors from the MCP tool registry (`mcp.rs:327-338`).                                                                                                  |
| tools/call                  | Looks up the named registry tool, applies the tool-specific auth gate when required, then dispatches to the registered handler (`mcp.rs:340-369`).                  |
| `ping`                      | Returns `{}`.                                                                                                                                                       |
| `shutdown`                  | Returns null result; does not exit.                                                                                                                                 |
| `exit`                      | If sent as a notification (no `id`), the loop breaks and the shim exits. Sent as a request, returns Invalid Request.                                                |

Anything else returns `-32601 Method not found` (`mcp.rs:245`).

### 4.4 JSON-RPC method name (shim → daemon)

The shim sends `scan_buffer` (bare, not `anvil/scan_buffer`) to the daemon
(`crates/anvil-cli/src/mcp/validation.rs:217-227`):

```jsonc
{
  "jsonrpc": "2.0",
  "method": "scan_buffer",
  "params": {
    "path": "<workspace-relative-path>",
    "text": "<proposedContent>",
    "version": 1, // SCAN_BUFFER_REQUEST_VERSION
    "mode": "preWrite",
  },
  "id": "mcp-prewrite-validation", // DAEMON_REQUEST_ID
}
```

The daemon dual-routes the bare and the namespaced form
(`anvil-intercept-proto/src/protocol.rs:125`, `ANVIL_SCAN_BUFFER`);
intercept-as-built §4.3 captures the daemon-side view. The shim hard-pins the
response correlation `id` to the request id (`validation.rs:300-313`); a
mismatched id is treated as `OperationalFailure`, not silently demoted.

The shim deliberately does not use the save-time `validate_paths` verb;
`validate_write` is a pre-write gate over proposed content the daemon has not
read (DSV-007, `validation.rs:99-103`). See
`docs/architecture/intercept-as-built.md` for `validate_paths`.

## 5. Validation routing — daemon-backed vs embedded

`LocalDaemonValidationClient` (routing impl at
`crates/anvil-cli/src/mcp/validation.rs:210-230`) is the routing primitive.
`validate_pre_write` (`validation.rs:555-578`) walks three branches:

1. **Unix with daemon reachable** — `cfg(unix)` arm at `validation.rs:131-141`
   resolves the socket path via `ipc::resolve_socket_path()`, hands off to
   `SocketDaemonValidationClient::validate_pre_write` (`validation.rs:151-164`),
   which calls `request_daemon_diagnostics` (`validation.rs:178-268`).
   Successful response → `DaemonValidationOutcome::Diagnostics(_)` →
   `DaemonStatus::Available`, `ValidationBackend::Daemon`
   (`validation.rs:366-370`).
2. **Unix with daemon unreachable / socket missing** — connect or path-validate
   failure on the `Unavailable` arm (`validation.rs:189-194`) returns
   `DaemonValidationOutcome::Unavailable`, which the shim silently demotes to
   embedded (`validation.rs:371-380`) with `DaemonStatus::NotWired`,
   `ValidationBackend::Embedded`. The "not-wired" naming reflects the v1
   stub-default semantic: from the response's perspective, no daemon was
   consulted.
3. **Windows (`cfg(not(unix))`)** — the `cfg(not(unix))` arm at
   `validation.rs:226-230` always returns `DaemonValidationOutcome::Unavailable`
   unconditionally. The same demotion logic at `validation.rs:371-380` runs,
   producing `DaemonStatus::NotWired`. **MCP enforcement still happens** via the
   embedded scanner — only the correlation envelope's `daemonStatus` field is
   wrong on Windows. See §13 G-01 and intercept-as-built §16 gap 9.

Operational failures from the daemon path (parse errors, timeouts, peer-cred
rejection, mismatched JSON-RPC id, truncated response) do **not** auto-promote
to embedded. They propagate as `DaemonValidationOutcome::OperationalFailure`
(`validation.rs:151-164`), which the validate-write tool maps to
`backend_failure_payload` (`validate_write.rs:225-257`): a hard `block` decision
with `daemonStatus: "unavailable"` (distinct from `not-wired`) and a structured
`error` payload. This is RMCP's fail-closed contract — silently demoting from a
wired-but-broken daemon to embedded would mask exactly the failures the operator
needs to see.

The mapping from `DaemonValidationOutcome` to `DaemonStatus` lives at
`validation.rs:361-383`:

| Outcome                 | `daemon_status` | `backend`  | Decision-side effect      |
| ----------------------- | --------------- | ---------- | ------------------------- |
| `Diagnostics(_)`        | `Available`     | `Daemon`   | normal pipeline           |
| `Unavailable`           | `NotWired`      | `Embedded` | embedded fallback runs    |
| `OperationalFailure(_)` | `Unavailable`   | `Daemon`   | hard `block`, no fallback |

`DaemonStatus::as_str` (`validation.rs:69-78`) emits the camelCase wire strings
`available` / `not-wired` / `unavailable`.

## 6. Embedded fallback path

`embedded_validate_pre_write` (`validation.rs:385-405`) builds an
`anvil_intercept::enforcement::EnforcementPipeline::default()`, wraps the
proposed write in a `ProposedChange`, and calls
`diagnostics_for_proposed_changes` against `Mode::Unknown("pre-write")`. The
pipeline is the same one the daemon uses; the difference is that the
daemon-backed path also goes through the per-user singleton's
fence/registry/telemetry layers (see intercept-as-built §6, §7, §10, §11), while
the embedded path is a single-shot in-process evaluation with no side effects.

**Diagnostic-envelope parity is the load-bearing property.** The two paths
return the same `anvil.diagnostic.v1` shape on the same fixture. The parity test
pin at `validate_write.rs:1034-1063`
(`daemon_and_embedded_paths_emit_identical_diagnostic_envelopes`) and the
live-daemon variant at `validate_write.rs:1070-1116`
(`live_daemon_mcp_tool_call_matches_embedded_diagnostic_envelope`) prove that
for an MCP-only deployment without a running daemon, the embedded path produces
the same enforcement decision as the daemon-backed one.

What the embedded path does **not** carry:

- Fence side-effects — embedded mode does not fence a worktree. The MCP shim
  cannot fence on its own; that is daemon-side state.
- Cross-session telemetry redaction — there is no fan-out from a per-shim,
  per-editor process.
- Latency rollups in `anvil intercept status` — the shim does not report to the
  daemon's latency aggregator (`anvil-intercept/src/latency.rs`) on the embedded
  path.

## 7. Correlation envelope

The response carries `correlation` at `validate_write.rs:325-333`:

| Field             | Value                                           | Notes                                                                                                                              |
| ----------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `id`              | `corr_mcp_<sanitised-path>`                     | `correlation_id` at `validate_write.rs:451-453` calls `sanitise_id_part` (`validation.rs:407-426`).                                |
| `surface`         | `"mcp"`                                         | Hardcoded.                                                                                                                         |
| `mode`            | `"preWrite"`                                    | Hardcoded; matches the daemon-side `PRE_WRITE_MODE` (`validation.rs:13`).                                                          |
| `backend`         | `"daemon"` / `"embedded"`                       | Drives the operator-visible "which path served this?" answer.                                                                      |
| `daemonStatus`    | `"available"` / `"not-wired"` / `"unavailable"` | The Council finding 3 demotion signal — distinguishes "embedded by design" from "daemon was expected and couldn't answer".         |
| `path`            | workspace-relative                              | Slash-normalised at `validate_write.rs:757-765`.                                                                                   |
| `enforcementMode` | `"block"` / `"warn"` / `"off"`                  | Resolved per-workspace (§ 8).                                                                                                      |
| `partialScan`     | `true` (omitted otherwise)                      | Present iff a slim `contentSha256` + `preview` payload was validated without full `proposedContent` (`validate_write.rs:473-475`). |

The fields are tool-local in v1. RTAI-007 / DRVR-002 may promote
`enforcementMode` and `daemonStatus` to the canonical correlation envelope in
`anvil-kernel-types`; the comment at `validate_write.rs:314-319` calls this out.

## 8. Enforcement mode resolution

Since ADR-098 AD-3 the posture is the shared
`anvil_kernel_types::EnforcementMode { Off, Warn, Fence, Interrupt }`, read
per-workspace from `<workspace_root>/.anvil.yaml`'s `enforcement.mode` field via
the shared `anvil_intercept_proto::enforcement_config::AnvilConfigFile`
deserialiser. Parsing keeps every posture distinct (no parse-time
`fence`/`interrupt` → `block` collapse); `block` is an alias for `interrupt`.
Missing file, missing field, malformed YAML, or unknown mode all fall back to
`MCP_DEFAULT_ENFORCEMENT` — `Interrupt` (the `block` alias) — the fail-closed
veto-on-error default, now recording the true decision.

The mode-to-decision matrix (`V` is the posture's veto decision — `Fence` →
`fence`, `Interrupt` → `interrupt` — recorded at action time, not collapsed to
`block`):

| Mode                  | Diagnostics observed    | Tool decision | `safeDefault`  |
| --------------------- | ----------------------- | ------------- | -------------- |
| `fence` / `interrupt` | any `Error`             | `V` (veto)    | `do-not-write` |
| `fence` / `interrupt` | only `Warning` / `Info` | `warn`        | (none)         |
| `fence` / `interrupt` | none                    | `allow`       | (none)         |
| `warn`                | any non-empty           | `warn`        | (none)         |
| `warn`                | none                    | `allow`       | (none)         |
| `off`                 | any                     | `allow`       | (none)         |
| `off`                 | none                    | `allow`       | (none)         |

The MCP no-config default posture is `Interrupt` (the `block` alias).
Diagnostics are returned to the caller verbatim regardless of mode — only the
`decision` flag changes. Tool-level errors (malformed input, oversize content,
workspace escape) always block regardless of mode (`enforcement.rs:1-31`
discusses the contract; pinned by
`enforcement_mode_off_still_rejects_malformed_input` at
`validate_write.rs:1460-1476`).

INTD-008 aliases `interrupt`/`fence` (→ `Block`) and `advisory`/`proceed` (→
`Off`) are accepted (`enforcement.rs:84-91`) for forward compatibility with the
daemon-side vocabulary.

## 9. Redaction filter (§4.4 contract)

Secret-rule diagnostics are redacted before the response leaves the shim.
`normalise_response_diagnostics` (`validate_write.rs:374-401`) walks each
diagnostic; for `Category::Secret` entries it:

- Replaces `id` with `redact_secret_id` (regex sweep, then SHA-256 hash fallback
  if no pattern matched, `validate_write.rs:403-413`).
- Sets `summary` to the static
  `"Potential secret detected; remove it from the proposed write."`
- Sweeps `location.file`, `source.rule_id`, `source.source_module`, and any
  `Mode::Unknown(_)` value through `redact_secret_values` — the default
  secret-pattern regex set substitutes matches with `[REDACTED]`
  (`validate_write.rs:415-424`, using
  `anvil_checks::secret::patterns::DEFAULT_COMPILED_PATTERNS`).
- Replaces `remediation_hint` with the static
  `"Remove the secret from the proposed write; use a placeholder or environment variable instead."`

This is the only MCP tool surface where the §4.4 redaction filter is **wired**
in `v0.6.0-beta`. The broader §4.4 spec — covering `scan.files`, `fix.apply`,
and `status.query` daemon-side responses — is shipped as **specification only**;
runtime parity is owned by RMCPF-010 and lands in a later tag. Cross-link:
`docs/archive/runbooks/v0.6.0-beta-security-note.md` §H3 (lines 143-185) for the
operator framing; `crates/anvil-intercept/src/ipc.rs` carries the daemon-side
filter wiring once RMCPF-010 graduates.

The redaction-hash primitive itself
(`crates/anvil-intercept/src/fanout.rs:436-441`) is unsalted SHA-256 in v1 —
security note H2 (lines 108-139). The shim inherits this when it falls back to
`redact_secret_id`'s hash arm, but the shim's fallback only triggers when no
regex match was found; the dominant path is the regex sweep, which substitutes a
literal `[REDACTED]` rather than a hash.

## 10. Installation surface (`anvil mcp install`)

The shim is the same binary as the rest of the CLI, so install is just writing
the editor config to point at `anvil mcp serve --stdio`. The install command
lives in the same file as the serve loop:
`crates/anvil-cli/src/commands/mcp.rs:88-167`.

```bash
anvil mcp install --client cursor          # writes ~/.cursor/mcp.json
anvil mcp install --client codex
anvil mcp install --client zed --scope project
anvil mcp install --client opencode --verify
```

`AgentClientId` in `crates/anvil-cli/src/activation/agent_registry.rs` is the
canonical identity registry. Skill discovery and MCP configuration remain
independent capability fields. `crates/anvil-cli/src/commands/mcp_installer.rs`
owns first-wave path, config-shape, semantic-merge, atomic-write, verification,
and restart-guidance adapters. Cursor and Claude Code retain the full activation
diagnostic ladder; `anvil start --mcp-client <client>` installs additional
first-wave config without promoting a live-protection claim.

The written entry is the canonical Rust stdio shape:

```jsonc
{
  "mcpServers": {
    "anvil": {
      "command": "anvil",
      "args": ["mcp", "serve", "--stdio"],
      "env": {},
    },
  },
}
```

The `--command` override lets tests and unusual deployments substitute a
non-PATH binary. The `--workspace` override overrides the selected scope root
(default: home directory for global scope).

For the drift policy (`UpToDate` / `SafeDrift` / `UnsafeDrift` / `NotPresent`),
atomic-write contract, and symlink-parent guard, see
`docs/architecture/activation-as-built.md` §"MCP install (LAUNCH-009)"; the same
`mcp_config` module backs both surfaces.

## 11. `anvil mcp-config`

The advanced surface (`crates/anvil-cli/src/commands/mcp_config.rs`, RCLI3-016 /
LAUNCH-009.5) handles the cases `anvil mcp install` does not:

```bash
anvil mcp-config --target claude-code              # print preview to stdout
anvil mcp-config --target claude-code --write      # write to workspace path
anvil mcp-config --target claude-code --verify     # parse + report
anvil mcp-config --target cursor --transport http --port 7616 --write
```

Stdio targets come from the shared first-wave registry: Claude Code, Cursor,
Codex, OpenCode, Gemini CLI, Antigravity, OpenClaw, VS Code project config,
Copilot CLI, Grok Build, Warp, and project-only Zed. Windsurf remains removed.
HTTP preview remains limited to Claude Code and Cursor.

`mcp-config` does **not** install the shim binary. It emits, writes, or verifies
the editor config that points at the shim. The shim itself is already part of
the same `anvil` binary.

The `--transport http` flag is advertised on `mcp-config`; the runtime HTTP
server it points at is the legacy Node MCP server
(`@eddacraft/anvil-mcp-server`,
`docs/public/anvil/integrations/mcp.md:140-164`), not the Rust shim. The Rust
shim is stdio-only in `v0.6.0-beta`.

## 12. Decision pipeline (the actual call path)

When an MCP client calls `anvil_validate_write`:

1. **Editor frames a JSON-RPC request** — newline-delimited JSON over stdin to
   the shim child process.
2. **Shim reads frame** — `read_frame` at `commands/mcp.rs:261-282` enforces the
   4 MiB stdio frame budget; oversize lines are discarded.
3. **Shim parses JSON-RPC** — `handle_message` at `commands/mcp.rs:228-248`. Bad
   requests return `-32600`; bad JSON returns `-32700`.
4. **Shim auth-gates the call** — `mcp_tool_auth_ok` at
   `commands/mcp.rs:371-389` checks credentials and (for edict credentials) a
   1-minute-TTL verify cache (`mcp.rs:395-457`). On failure, returns
   `mcp_auth_required_result` at `mcp.rs:459-488` — a `block` decision with
   `error.code = "authentication-required"`.
5. **Shim dispatches to validate_write** — `tools_call_response`
   (`commands/mcp.rs:340-369`) verifies the tool name and calls
   `validate_write::call(arguments)` (`mcp.rs:368`).
6. **validate_write parses + validates input** — `ValidateWriteRequest::parse`
   (`validate_write.rs:476-527`). Workspace-escape (`reject_symlink_escape` at
   `validate_write.rs:732-748`), oversize content, NUL bytes, and non-UTF-8
   encodings all short-circuit to a structured `block`.
7. **validate_write resolves enforcement mode** — `.anvil.yaml` lookup via
   `WorkspaceEnforcementResolver::resolve` (`validate_write.rs:111-115`) →
   `enforcement::load_for_workspace` (`enforcement.rs:135-149`).
8. **validate_write calls the validation backend** —
   `validate_pre_write(&request, &LocalDaemonValidationClient)`
   (`validate_write.rs:151-157`). See §5 for the routing.
9. **validate_write redacts the response** — `normalise_response_diagnostics`
   (`validate_write.rs:374-401`, §9).
10. **validate_write builds the correlation envelope** — `validation_payload` →
    `validation_payload_with_decision` (`validate_write.rs:285-349`).
    `decision_for` evaluates the diagnostics against the resolved enforcement
    mode (`enforcement.rs:96-119`).
11. **Shim writes response to stdout** — `write_message`
    (`commands/mcp.rs:525-530`) serialises and flushes.

`isError` is set when the decision is a veto — `ControlDecision::is_veto()`
(`block | fence | interrupt`), not a `decision == "block"` string compare — or
an `error` field is present, so a fence-vetoed write cannot report
`isError: false` (ADR-098 AD-3 amendment 1). MCP clients that key off the
standard `isError` flag pattern-match the same way as those that inspect the
`decision` field directly.

## 13. Cross-cutting concerns

### Trust boundary

The shim runs as the user. Same-UID local IPC to the daemon (`SO_PEERCRED` on
Linux, `getpeereid` on macOS, owner-only DACL + `reject_remote_clients` on
Windows). Cross-link: `docs/architecture/intercept-as-built.md` §5
"Authentication and trust boundary". The shim has no remote surface, no TLS, no
signed manifests in v1. An MCP client an operator does not fully trust still
runs inside this trust boundary — the shim cannot distinguish "Cursor" from "a
same-UID third-party agent that called the tools/call method".

### Determinism

Same input + same `anvil` version + same daemon state → same decision. The
embedded path is pure
(`EnforcementPipeline::default()::diagnostics_for_proposed_changes` takes only
`(changes, mode)`); the daemon-backed path adds the daemon's session/fence
state, but for `validate_write` the daemon's `scan_buffer` handler is itself
stateless against the buffer (intercept-as-built §12 "Embedded validation path"
pins the parity).

### Fail-closed

The shim's failure semantics are explicit:

- Malformed input → `block` (`validate_write.rs:124-132`).
- Workspace escape → `block` (`validate_write.rs:1287-1330` test pins).
- Server cwd deleted → `block` with `server-cwd-unavailable` error
  (`validate_write.rs:75-89, 259-283`).
- Daemon-wired-but-failed (`OperationalFailure`) → `block`, no fallback
  (`validate_write.rs:225-257`, `validation.rs:381`).
- Auth missing/expired → `block` with `authentication-required`
  (`commands/mcp.rs:459-488`).
- Daemon unavailable on Unix or `cfg(not(unix))` → silent demote to embedded,
  decision still computed (`validation.rs:371-380`).

The only "soft" path is daemon-not-running-on-Unix, which is the expected steady
state for an MCP-only deployment.

### No telemetry to Anvil servers from the MCP path

The shim does not call `anvil-api`. The auth gate (`mcp.rs:371-457`) does call
`client.verify_edict()` for edict credentials — that is a licence verify, not
telemetry, and it is cached for 1 minute (`EDICT_VERIFY_CACHE_TTL`,
`mcp.rs:395`). No `validate_write` call itself produces an outbound HTTP request
from the shim.

### Stdout discipline

The shim's stdout is reserved for JSON-RPC frames. The enforcement-mode loader
and the validation client write diagnostic context to **stderr** only (e.g.
`eprintln!("anvil-mcp: connecting to daemon validation socket {…}")` at
`validation.rs:183`). A writeln to stdout from a non-protocol path would corrupt
the JSON-RPC stream — the comment at `enforcement.rs:67-72` records this
contract.

## 14. Known gaps (dated 2026-05-07; G-01 and G-04 updated 2026-06-10)

### G-01: Windows `correlation.daemonStatus` always `not-wired`

`crates/anvil-cli/src/mcp/validation.rs:226-230`'s `cfg(not(unix))` arm returns
`DaemonValidationOutcome::Unavailable` unconditionally. The caller maps that to
`DaemonStatus::NotWired`. The MCP `validate_write` correlation envelope cannot
distinguish daemon-up from daemon-down on Windows, regardless of whether
`intercept status` itself works over the named pipe. The embedded scanner still
runs and the enforcement decision is still computed; only the correlation field
is wrong.

**Risk:** Low — the embedded path is correctness-equivalent to the daemon-backed
path on the same fixture. **Fix (partial, 2026-06-10):** Windows named-pipe IPC
**was** plumbed for the separate `query_protection_claim` surface — the
`#[cfg(windows)]` `WindowsPipeDaemonValidationClient` impl at
`validation.rs:276-290` (MLP2-075) — but its `validate_pre_write` deliberately
returns `Unavailable`; the validation-routing arm remains stubbed. Closing the
gap means wiring pre-write validation itself over the pipe (the same helper the
`intercept status` Windows path uses). See
`docs/archive/runbooks/v0.6.0-beta-release-runbook.md` §2.

### G-02: §4.4 redaction filter spec-only outside `validate_write`

The wired filter today is at `validate_write.rs:374-424`. The broader §4.4
contract (covering `scan.files`, `fix.apply`, `status.query`) is shipped as
specification only; runtime parity is owned by RMCPF-010. Operators who install
third-party MCP clients into the v1 cut see un-redacted absolute paths and
un-redacted secret-rule excerpts on those three legacy-Node tools.

**Risk:** Medium for deployments with untrusted same-UID MCP clients; low
otherwise. **Fix:** RMCPF-010 wires the runtime filter against the daemon's MCP
transport. See `docs/archive/runbooks/v0.6.0-beta-security-note.md` §H3
(143-185).

### G-03: Redaction hash unsalted across sessions

`hash_of_path` (`crates/anvil-intercept/src/fanout.rs:436-441`) is unsalted
SHA-256. The shim does not directly invoke this primitive on the dominant
`validate_write` path (the regex sweep substitutes `[REDACTED]` rather than a
hash), but the hash fallback in `redact_secret_id` (`validate_write.rs:403-413`)
inherits the same primitive. A same-UID subscriber who can guess at the
deployment's repository tree can rainbow-table `(rule_id, hashed_path)` pairs.

**Risk:** Low for `validate_write` (regex sweep dominates); medium for the
broader fan-out path. **Fix:** Per-startup HMAC salt minted on daemon launch.
See `docs/archive/runbooks/v0.6.0-beta-security-note.md` §H2 (108-139).

### G-04: Tool surface limited to `anvil_validate_write` (resolved)

**Resolved (RMCPF-010/011/012):** the six former Node-only tools were ported to
the Rust shim (`registry.rs:30-148`; §4 registry table). **Residual:**
`anvil_check` architecture-check parity is deferred (`check.rs:21`); the INTR
MCP-path content cap remains deferred (not shipped).

### G-05: Install surface narrower than config surface (resolved)

**Resolved (MCPX / ADR-106):** `anvil mcp install` and stdio `anvil mcp-config`
now share the first-wave typed registry and config adapters. VS Code remains
project-file or vendor-profile managed, Zed remains project-only, and Windsurf
remains unsupported. Those constraints are machine-visible errors and are
documented in `docs/public/anvil/integrations/mcp.md`.

### G-06: HTTP transport mentioned but not served by the Rust shim

`mcp-config --transport http` writes a config that points at the legacy Node MCP
server (`@eddacraft/anvil-mcp-server`,
`docs/public/anvil/integrations/mcp.md:140-164`). The Rust shim does not serve
HTTP in `v0.6.0-beta` — `anvil mcp serve --stdio` is the only shape
(`commands/mcp.rs:185-191` bails without `--stdio`).

**Risk:** Low — the surface works; it just routes to a different binary.
**Fix:** Tracked alongside RMCPF parity (G-04 above).

## 15. Source references

### `crates/anvil-cli/src/mcp/`

| File                                               | Role                                                                                                                                                                                                                                  |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mod.rs`                                           | Module surface — re-exports `enforcement`, `tools`, `validation`.                                                                                                                                                                     |
| `crates/anvil-cli/src/mcp/tools/mod.rs`            | Tool registry — re-exports `validate_write`.                                                                                                                                                                                          |
| `crates/anvil-cli/src/mcp/tools/validate_write.rs` | RMCP-004: the `anvil_validate_write` tool. Descriptor, request parser, workspace-escape guard, redaction filter, response builder, correlation envelope.                                                                              |
| `validation.rs`                                    | RMCP-005: `DaemonValidationClient` trait, `LocalDaemonValidationClient`, `SocketDaemonValidationClient`, `request_daemon_diagnostics` (Unix), embedded fallback, `DaemonStatus` enum, `ValidationBackend` enum, `validate_pre_write`. |
| `enforcement.rs`                                   | RTAI-006: `EnforcementMode` enum, `decision_for` matrix, `load_for_workspace` reading `.anvil.yaml`. The mode-to-decision policy.                                                                                                     |

### `crates/anvil-cli/src/commands/`

| File            | Role                                                                                                                                                                                                                                                             |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mcp.rs`        | RMCP-002 / RMCP-003 / RMCP-007: `anvil mcp serve --stdio` stdio loop, JSON-RPC dispatcher, MCP `initialize` / tools/list / tools/call methods, auth gate, `anvil mcp install --client cursor\|claude-code` wrapper.                                              |
| `mcp_config.rs` | RCLI3-016 / LAUNCH-009.5: `anvil mcp-config` advanced surface — emit, write, verify editor configs for Cursor / Claude Code with stdio or HTTP transport. Shared `install_rust_stdio_target` helper used by `anvil mcp install` and the activation orchestrator. |

### Cross-crate

- `crates/anvil-intercept-proto/src/protocol.rs:125` — `ANVIL_SCAN_BUFFER`
  method constant; the daemon dual-routes the bare `scan_buffer` form the shim
  sends.
- `anvil-intercept::enforcement::EnforcementPipeline` — the embedded evaluator
  the shim falls back to; same pipeline the daemon uses.
- `anvil-kernel-types::Diagnostic` /
  `anvil-kernel-types::diagnostics::ControlDecision` — the canonical diagnostic
  and decision shapes returned in the response.
- `anvil_checks::secret::patterns::DEFAULT_COMPILED_PATTERNS` — the redaction
  regex set used by `redact_secret_values`.

## 16. Related docs

- `docs/architecture/intercept-as-built.md` — daemon side of the validation
  pipeline; §4.3 IPC method names, §5 trust boundary, §12 embedded validation
  path, §13 §4.4 redaction filter status.
- `docs/architecture/activation-as-built.md` — orchestrator that calls
  `mcp install` from `anvil start`; §"MCP install (LAUNCH-009)" carries the
  drift policy and atomic-write contract.
- `docs/public/anvil/integrations/mcp.md` — public-side narrative for the same
  surface; the legacy Node MCP catalogue lives there.
- `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` §2 — the corrected
  Windows `correlation.daemonStatus` framing.
- `docs/archive/runbooks/v0.6.0-beta-security-note.md` §H2 (unsalted hash) and
  §H3 (§4.4 spec-only) — the operator-facing trade-offs the shim inherits.
- `plans/archive/modules/rust-mcp-launch-shim.aps.md` — RMCP-001..RMCP-008, the
  eight A1 work items behind this surface.
- `plans/modules/rust-mcp-full-port.aps.md` — RMCPF, the tracked follow-up that
  wires the broader §4.4 redaction contract and ports the legacy tool catalogue.
- `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md` §2.3a,
  §3.7, §4.4 — driver trust boundary and redaction contract.
- `RELEASE-PLAN.md` §A1, §A2 — Tier A1 (RMCP launch shim) and Tier A2
  (Daemon-Backed RMCP + Driver Reach).
