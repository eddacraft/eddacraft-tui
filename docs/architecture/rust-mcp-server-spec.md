# Rust MCP Server Parity Spec

| Type | Authority     | Owner       | Status | Freshness                                                                                                                                                   |
| ---- | ------------- | ----------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Spec | Authoritative | RMCPF/MCP26 | Live   | Last reviewed 2026-08-17 against CIB-343 handshake ladder in `docs/architecture/mcp-shim-as-built.md` §10; prior ADR-113 / MCP26 dual-era review 2026-07-29 |

| Upstream                                                                                                             | Downstream                                                                    |
| -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| ADR-033, ADR-113, DRVR-006/DRVR-007, `docs/architecture/mcp-shim-as-built.md`, `anvil-archive/anvil-mcp-server/src/` | RMCPF parity, MCP26 dual-era conformance, eventual official Rust SDK adoption |

This spec defines the architecture of the Rust MCP server in the `anvil` binary.
The parity port is complete enough to expose the anvil-owned tool and resource
catalogue, and MCP26 adds a typed dual-era stdio protocol boundary. The current
runtime is documented in `docs/architecture/mcp-shim-as-built.md`.

## Goals

- Preserve the single-binary MCP story established by RMCP.
- Port or explicitly retire the archived TypeScript MCP tools, resources, and
  prompts from `anvil-archive/anvil-mcp-server/src/`.
- Keep daemon authority narrow: MCP handlers either translate to existing daemon
  RPCs or compose locally against Rust CLI/library code.
- Keep graph-context expansion out of RMCPF; new graph tools remain owned by
  GV2/GCTX.
- Make TypeScript-server retirement testable through fixture-backed parity,
  migration docs, and client configuration cutover.

## Non-Goals

- Replacing `anvil_validate_write` or weakening its pre-write contract.
- Introducing daemon RPCs only to satisfy MCP parity prose.
- Reinstating the archived TypeScript scanner, suppression parser, or runtime
  export collector as active implementation dependencies.
- Making Streamable HTTP a default runtime path before RMCPF-021 records demand
  and support requirements.

## Command Layout

The Rust MCP server remains under `anvil mcp`:

| Command                   | Purpose                                                                                                                                                                             | Owner                |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| `anvil mcp serve --stdio` | Default MCP server for editor/agent clients. Exposes the Rust-owned tools and resources through the dual-era protocol adapter.                                                      | RMCP + RMCPF + MCP26 |
| `anvil mcp serve --http`  | Optional Streamable HTTP mode only if RMCPF-021 keeps HTTP support. It must be localhost-bound by default and retain explicit auth/rate-limit controls if exposed beyond localhost. | RMCPF-021            |
| `anvil mcp config`        | Client configuration helper; continues to point generated configs at the Rust binary.                                                                                               | RMCP/RMCPF cutover   |

Parity work adds modules under `crates/anvil-cli/src/mcp/` rather than creating
a new package. The expected shape is:

```text
crates/anvil-cli/src/mcp/
  tools/
  resources/
  prompts/
  transports/
  redaction.rs
  workspace.rs
```

The module names are illustrative, not mandatory, but the ownership boundary is
mandatory: MCP transport framing stays in the CLI host, reusable analysis or
state logic lives in the Rust crates that already own that domain.

## Protocol Support

The Rust server supports two stdio eras over the same anvil-owned domain
handlers. The versions are sealed rather than negotiated by echoing arbitrary
client input:

| Era    | Versions                                               | Required behaviour                                                                                                                                                                                                         |
| ------ | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Modern | `2026-07-28`                                           | `server/discover`; protocol version and client capabilities in `params._meta` on every request; `resultType` and server identity on successful results; private cache hints on discovery, list, and resource-read results. |
| Legacy | `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` | `initialize` selects a supported version for the child process; `notifications/initialized`, `ping`, `shutdown`, and `exit` retain initialise-era behaviour.                                                               |

Modern requests do not use `initialize` and do not create a protocol session.
They are independently versioned and end when the client closes stdin. Legacy
non-lifecycle methods require a successful `initialize`, and a legacy `exit`
notification is honoured only in such a process. The transport applies a four
MiB limit to each newline-delimited frame and discards the remainder of an
oversize line before reading the next frame.

The supported method surface is:

| Capability      | Modern                             | Legacy                     | Notes                                                                                           |
| --------------- | ---------------------------------- | -------------------------- | ----------------------------------------------------------------------------------------------- |
| Discovery       | `server/discover`                  | `initialize`               | Both declare tools and resources; only modern discovery carries modern result and cache fields. |
| Tools           | `tools/list`, `tools/call`         | Same, after initialisation | One shared registry and handler set.                                                            |
| Resources       | `resources/list`, `resources/read` | Same, after initialisation | One shared resource implementation and process-local egress accounting.                         |
| Prompts         | Not advertised                     | Not advertised             | Prompt policy remains explicit; no hidden prompt source of architecture authority.              |
| Streamable HTTP | Out of scope                       | Out of scope               | Stdio remains the supported Rust transport.                                                     |

`crates/anvil-cli/src/mcp/protocol/` owns typed request metadata, era selection,
wire errors, lifecycle differences, result envelopes, and trace extraction. It
returns to era-neutral domain handlers for tool and resource work. Those
handlers retain anvil's authority over credentials, workspace containment,
redaction, daemon calls, validation, and the process-local graph-egress budget.
This adapter boundary is temporary under ADR-113: the official Rust MCP SDK may
replace protocol framing after its stable release passes anvil's adoption gate,
without moving domain policy into the SDK layer.

All agent-visible schemas are versioned and fixture-backed. Where Rust changes a
shape intentionally, the compatibility matrix and migration docs must name the
change rather than silently claiming parity.

## Tool Architecture

DRVR-006 resolved the daemon/local split as **option (b) Distinguish**. RMCPF
adopts that split with one recorded Phase 1 amendment: `anvil_status` starts as
local workspace-health composition because no approved daemon `status.query`
surface exists, and RMCPF must not invent daemon RPCs just for parity prose.

| Tool                   | Class                                                                                         | Authority                                                          | Required behaviour                                                                                                                                              |
| ---------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `anvil_validate_write` | Daemon-RPC translator with embedded fallback                                                  | Existing RMCP shim                                                 | Preserve the current pre-write contract and `validation.backend` honesty field.                                                                                 |
| `anvil_check`          | Daemon-RPC translator                                                                         | `scan.files` / `scan_buffer`                                       | Use daemon scan surfaces when available; use the existing correctness-equivalent embedded fallback only where RMCP already permits it.                          |
| `anvil_status`         | MCP-driver-local composition (Phase 1); daemon-RPC candidate after INTD adds status authority | Rust CLI/local workspace status; future `status.query` if approved | Report archived project-health fields from validated workspace-local data, with explicit local/no-daemon provenance. Do not invent `status.query` inside RMCPF. |
| `anvil_suppress`       | Daemon-RPC translator                                                                         | `suppression.apply`                                                | Let the daemon validate and normalise ADR-004 suppression format.                                                                                               |
| `anvil_fix`            | MCP-driver-local composition                                                                  | Rust CLI/library fixer path                                        | Run deterministic fix logic in-process or through the CLI; do not add `fix.apply` to the daemon just for MCP.                                                   |
| `anvil_gate`           | MCP-driver-local composition                                                                  | `anvil gate` / `GateRunner`                                        | Keep npm audit, OPA evaluation, and coverage reads local to the MCP host/CLI path.                                                                              |
| `anvil_query_boundary` | MCP-driver-local composition                                                                  | `crates/anvil-architecture` query path                             | Query architecture boundaries from Rust-owned local data; no daemon round-trip.                                                                                 |

If an implementation discovers that a daemon-local boundary above is wrong, the
change must be recorded as an amendment to the DRVR-006 table before code lands.
RMCPF must not create a new daemon RPC as an implementation detail of a parity
tool.

## Validation Paths

MCP handlers use three validation paths:

1. Daemon-backed validation for scan/suppression authority, and for status only
   after an approved daemon status RPC exists.
2. Existing embedded fallback for RMCP-compatible scan paths when the daemon is
   genuinely unavailable.
3. MCP-driver-local composition for status, fix, gate, and boundary queries.

Every response that leaves the MCP transport must carry enough provenance for a
client to understand which path was used. For validation responses this includes
the existing `validation.backend` distinction between `daemon` and `embedded`.
For local-composition tools, the response must identify the CLI/library path or
equivalent local source used by the handler.

The degraded-state rule is fail-clear rather than fail-open: daemon-RPC
translator tools return a structured retriable daemon-unavailable error when no
approved fallback exists. Local-composition tools may continue when their local
inputs are present and validated.

## Redaction And Workspace Safety

RMCPF adopts DRVR-007's redaction contract for every MCP-bound response,
regardless of whether the handler is daemon-backed or local:

- Secret-detection excerpts are redacted before transport emission.
- Absolute paths are converted to workspace-relative paths.
- Fix diffs and remediation hints use the same redaction/path resolver.
- Responses default closed when a field class is not explicitly approved for MCP
  emission.

Workspace roots are resolved once per request from the MCP process cwd or the
client-provided root, then checked before any filesystem read or write. Archived
TypeScript workspace validation is reference evidence only; Rust owns the active
implementation.

## Resource Strategy

Archived TypeScript resources are ported only where they still map to active
Rust-owned data:

| Archived resource       | Rust source of truth                                           | RMCPF disposition                                                       |
| ----------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `anvil://baseline`      | Rust baseline data and commands                                | Port if still useful to clients.                                        |
| `anvil://boundaries`    | Rust architecture/boundary model                               | Port.                                                                   |
| `anvil://patterns`      | Active Rust rule/check metadata                                | Port or retire if superseded by rule distribution metadata.             |
| `anvil://suppressions`  | Active `.anvil/suppressions.json` readers                      | Port.                                                                   |
| `anvil://config`        | Rust config loader                                             | Port.                                                                   |
| `anvil://constraints`   | `crates/anvil-cli/src/commands/export.rs` constraint exporter  | Port from Rust exporter, not TS `runtime-export`.                       |
| `anvil://drift`         | `crates/anvil-cli/src/commands/drift.rs` and Rust drift schema | Port from Rust drift model.                                             |
| `anvil://file-warnings` | Rust scan/diagnostic outputs                                   | Port if clients still consume it; otherwise retire with migration note. |

Resources are read-only. Any resource that would need to mutate state must
become a tool or be retired.

## Prompt Strategy

The archived prompts are `fix-violation`, `suppress-violation`,
`architecture-review`, and `pre-generation`. RMCPF-012 decides prompt-by-prompt:

- Port prompts that still improve agent behaviour and can cite current Rust tool
  names and response schemas.
- Retire prompts that duplicate public docs or teach archived TypeScript server
  behaviour.
- Test the final prompt list against the inventory disposition.

Prompt content must not become a hidden source of architecture policy. Durable
policy remains in ADRs, APS, docs, schemas, and code.

## Transport Strategy

Stdio is the default and required transport. It matches the supported client
registry and keeps the MCP server as a child of the editor or agent process.

Streamable HTTP is deferred and gated by RMCPF-021. Phase 0 found no active
supported-client demand, so the default decision is retirement with migration
notes unless demand appears before implementation. If retained, Rust HTTP parity
must include:

- Localhost binding by default.
- Explicit opt-in for non-localhost binding.
- API-key or stronger authentication for non-localhost use.
- Request size limits, content-type validation, no-store responses, and bounded
  session cleanup.
- Compatibility tests against the archived TypeScript HTTP fixture behaviour.

If no supported client needs HTTP, RMCPF-021 should retire it and RMCPF-030 must
document the migration path.

## TypeScript Server Retirement Gates

`anvil-archive/anvil-mcp-server/` remains frozen reference material until
RMCPF-031. Retirement requires all of the following:

- RMCPF-010, RMCPF-011, RMCPF-012, RMCPF-020, and RMCPF-021 have either shipped
  parity or recorded retirements.
- RMCPF-030 compatibility tests compare archived TypeScript fixture responses
  with Rust responses for every ported surface.
- Generated client configs point at `anvil mcp serve` and no release-critical
  doc tells users to install the TypeScript MCP server.
- Migration docs name every intentional incompatibility and every retired
  surface.
- Release notes state whether the archive stays as historical reference or is
  deleted after the deprecation window.

## Validation Plan

RMCPF implementation items validate this spec in slices:

- RMCPF-010/011: fixture-backed tool parity, daemon/local class assertions, and
  redaction tests.
- RMCPF-012: prompt list and prompt content tests against the inventory matrix.
- RMCPF-020: resource listing/read tests against Rust-owned data sources.
- RMCPF-021: stdio tests plus HTTP transport tests only if HTTP is retained.
- RMCPF-030: side-by-side archived-TypeScript versus Rust compatibility harness
  and migration-doc smoke tests.

Council review for RMCPF-002 should confirm that the tool classification matches
DRVR-006, that no parity-only daemon RPC has been introduced, and that the
single-binary launch path remains intact.
