# Rust MCP Full Port Inventory

**Status:** Accepted for RMCPF-001  
**Date:** 2026-05-13  
**APS:** RMCPF-001

## Purpose

Inventory the archived TypeScript MCP server so the Rust full-port work can make
explicit port, defer, and retirement decisions without treating the archive as a
live sidecar.

Source inspected:

- `archive/anvil-mcp-server/package.json`
- `archive/anvil-mcp-server/src/`
- `crates/anvil-cli/tests/mcp_config.rs`

The archive is frozen reference material under
[ADR-033](../decisions/033-park-ide-mcp-retire-ts-scanner.md). The Rust port
must preserve useful contracts or explicitly retire them, but it must not revive
the TypeScript package as a runtime dependency.

## Summary

| Surface | Archived count | Rust full-port disposition |
| --- | ---: | --- |
| Tools | 6 | All ported under RMCPF-010 / RMCPF-011. `anvil_suppress` ships as the daemon-RPC translator's correctness-equivalent embedded fallback pending an INTD-owned `suppression.apply` (see RMCPF-011 disposition below). |
| Resources | 8 | Port with Rust-owned sources of truth |
| Prompts | 4 | Retired under RMCPF-012 (2026-05-14). Rust MCP server does not advertise the `prompts` capability and `prompts/list` returns JSON-RPC `Method not found`. See "Prompts — RMCPF-012 disposition" below for the per-prompt migration notes. |
| Transports | 2 | Keep stdio canonical for Phase 1; retire Streamable HTTP unless supported-client demand is proven before RMCPF-021 |
| Client config targets | 4 | Support Claude Code and Cursor first; defer Windsurf and VS Code |

## Server Entrypoints

| Surface | File | Contract | Disposition |
| --- | --- | --- | --- |
| Library exports | `archive/anvil-mcp-server/src/index.ts` | Exports server factory, config generation, and HTTP transport helpers. | Do not port as a Node-style package API. Rust equivalents live behind `anvil mcp` commands. |
| Server factory | `archive/anvil-mcp-server/src/server.ts` | `createAnvilMcpServer(options)` registers every tool, prompt, and resource. `projectRoot` constrains read surfaces; fix and suppress use the server workspace root. | Port the registration shape into the Rust MCP server, but only advertise surfaces that the Rust binary owns. |
| Stdio binary | `archive/anvil-mcp-server/src/bin.ts` | Starts the MCP server on stdio and exits on `SIGINT` / `SIGTERM`. | Extend the existing Rust `anvil mcp serve --stdio` path. |
| HTTP binary | `archive/anvil-mcp-server/src/bin-http.ts` | Starts the Streamable HTTP transport using `ANVIL_MCP_PORT` and `ANVIL_MCP_HOST`. | Defer until the Streamable HTTP decision is made. |

## Tools

Tool registrations are exported from `archive/anvil-mcp-server/src/tools/index.ts`.

| Tool | File | Input contract | Output and behaviour | Tests | Owner class | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `anvil_check` | `archive/anvil-mcp-server/src/tools/check.tool.ts` | `{ files: string[], workspaceRoot: string, checks?: ("architecture" \| "antipattern")[] }` | Runs file analysis and returns JSON text with `warnings`, `summary`, `executionTimeMs`, `checksRun`, and `hasBlockingWarnings`; tool failures return `{ error }` with `isError: true`. | `archive/anvil-mcp-server/src/tools/check.tool.test.ts` | Daemon-RPC translator. | Port in RMCPF-010; preserve the response shape or document a versioned successor. |
| `anvil_gate` | `archive/anvil-mcp-server/src/tools/gate.tool.ts` | `{ workspaceRoot: string, targetFiles?: string[], skipChecks?: string[], failFast?: boolean }` | Runs planless analysis when target files are provided; otherwise loads config and runs full gate evaluation. Returns mode, warnings or checks, timing, cache, score, and summary fields. | `archive/anvil-mcp-server/src/tools/gate.tool.test.ts` | MCP-driver-local composition. | Port in RMCPF-010 using Rust gate/config paths. |
| `anvil_status` | `archive/anvil-mcp-server/src/tools/status.tool.ts` | `{ workspaceRoot: string }` | Returns `{ status, workspaceRoot, availableChecks, config, hasBaseline, version }`; config load failures are non-fatal. | `archive/anvil-mcp-server/src/tools/status.tool.test.ts` | MCP-driver-local composition for Phase 1; daemon-RPC candidate only after an approved status authority exists. | Port in RMCPF-010. |
| `anvil_fix` | `archive/anvil-mcp-server/src/tools/fix.tool.ts` | `{ filePath: string, warningId: string, line: number }` | Mutates a workspace file with deterministic fixes for selected AP warnings, validates path containment, uses a lock file, and returns fixed/description/before/after fields. | `archive/anvil-mcp-server/src/tools/fix.tool.test.ts` | MCP-driver-local composition. | Port in RMCPF-011 only after Rust fixer semantics are explicit. |
| `anvil_suppress` | `archive/anvil-mcp-server/src/tools/suppress.tool.ts` | `{ filePath: string, warningId: string, line: number, reason: string, expiryDays?: number }` | Inserts an `@anvil-ignore-until` comment, defaults expiry to 30 days, sanitises reason text, and validates path containment. | `archive/anvil-mcp-server/src/tools/suppress.tool.test.ts` | Open: plan classifies as daemon-RPC translator, archive mutates locally. | Port in RMCPF-011 after choosing daemon-authorised mutation or local driver composition. |
| `anvil_query_boundary` | `archive/anvil-mcp-server/src/tools/query-boundary.tool.ts` | `{ sourceFile: string, targetFile: string, workspaceRoot: string }` | Loads architecture baseline and returns `{ allowed, reason, message, sourceLayer?, targetLayer?, violation? }`; missing baseline and unassigned layers allow by default with rationale. | `archive/anvil-mcp-server/src/tools/query-boundary.tool.test.ts` | MCP-driver-local composition. | Port in RMCPF-011 using Rust architecture/baseline crates. |

## Resources

Resource registrations are exported from
`archive/anvil-mcp-server/src/resources/index.ts` and covered by
`archive/anvil-mcp-server/src/resources/resources.test.ts`.

| Resource | File | Contract | Current tests | Owner / follow-on item | Disposition |
| --- | --- | --- | --- | --- | --- |
| `anvil://baseline` | `archive/anvil-mcp-server/src/resources/baseline.resource.ts` | Returns `.anvil/architecture.json` or structured no-baseline/load-failed errors. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust architecture/baseline model. | **Ported (RMCPF-020)** via `anvil_architecture::baseline::load_baseline`. |
| `anvil://boundaries` | `archive/anvil-mcp-server/src/resources/boundaries.resource.ts` | Returns layers and boundaries derived from the baseline. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust architecture/baseline model. | **Ported (RMCPF-020)** — baseline `layers`/`boundaries` serde shapes. |
| `anvil://patterns` | `archive/anvil-mcp-server/src/resources/patterns.resource.ts` | Returns built-in anti-pattern catalogue entries and count. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust checks/rules catalogue. | **Ported (RMCPF-020)** via `anvil_checks::antipattern::patterns::all_patterns`. |
| `anvil://suppressions` | `archive/anvil-mcp-server/src/resources/suppressions.resource.ts` | Reads `.anvil/suppressions.json` and returns suppression list plus active/expired summary. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust suppression readers. | **Ported (RMCPF-020)** via `services::suppressions::load_suppressions_report` (active set + total/active/expired summary). |
| `anvil://config` | `archive/anvil-mcp-server/src/resources/config.resource.ts` | Returns loaded config, source, default flag, and errors. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; `crates/anvil-config` and CLI config loader. | **Ported (RMCPF-020)** via `anvil_config::{discover, parse_file}`. |
| `anvil://constraints` | `archive/anvil-mcp-server/src/resources/constraints.resource.ts` | Returns aggregated constraints from the archived TS runtime collector. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust export/constraint surfaces. | **Ported (RMCPF-020)** via `commands::export::collect_constraints` (the `anvil export constraints` aggregator — Rust-owned, not the TS collector). |
| `anvil://drift` | `archive/anvil-mcp-server/src/resources/drift.resource.ts` | Returns no-snapshot, single-snapshot, or comparison drift state from latest snapshots. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust drift model and CLI drift command. | **Ported (RMCPF-020)** via `commands::drift` snapshot readers + `compare_snapshots`. |
| `anvil://file/{path}/warnings` | `archive/anvil-mcp-server/src/resources/file-warnings.resource.ts` | Validates a workspace path, analyses one file, and returns warnings, summary, checks run, and blocking status. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; same Rust path as `anvil_check`. | **Retired (RMCPF-020, 2026-06-19)** — folded into the shipped `anvil_check` MCP tool, which performs the identical per-file anti-pattern scan. Clients pass the single file in `anvil_check`'s `files` array. Not advertised in `resources/list`. |

> **RMCPF-020 implementation note (2026-06-19):** the seven state resources
> above are **ported** and advertised in `resources/list`, each sourced from its
> canonical Rust reader (impl in `crates/anvil-cli/src/mcp/resources/anvil.rs`);
> `file/{path}/warnings` is **retired** into `anvil_check`. They are local
> workspace-file reads (mirroring the MCP tools' `std::env::current_dir()` root
> contract), kept architecturally separate from the GCTX `graph://` egress
> resources. Payload shapes follow the Rust-owned source models; archived TS
> shapes are compatibility reference (deltas recorded for the RMCPF-030 harness).

## Prompts

Prompt registrations are exported from `archive/anvil-mcp-server/src/prompts/index.ts`
and covered by `archive/anvil-mcp-server/src/prompts/prompts.test.ts`.

| Prompt | File | Args | Behaviour | Current tests | Owner / follow-on item | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `fix-violation` | `archive/anvil-mcp-server/src/prompts/fix-violation.prompt.ts` | `{ warningId, filePath, line?, message? }` | Returns fix guidance and reminds the client to run `anvil_check`; sanitises newlines/backticks. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012. | **Retired** (2026-05-14). Guidance now lives in the `anvil_fix` tool description and in `docs/public/anvil/integrations/mcp.md`. |
| `suppress-violation` | `archive/anvil-mcp-server/src/prompts/suppress-violation.prompt.ts` | `{ warningId, filePath, line, reason? }` | Guides suppression decisions and the `anvil_suppress` call. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012; coordinates with RMCPF-011 suppress authority. | **Retired** (2026-05-14). Suppression policy is owned by ADR-004 and surfaced through the `anvil_suppress` tool description; MCP prompts are not the right place for policy text. |
| `architecture-review` | `archive/anvil-mcp-server/src/prompts/architecture-review.prompt.ts` | `{ filePath, workspaceRoot? }` | Produces a boundary-review checklist and suggests `anvil_query_boundary` / `anvil_check`. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012. | **Retired** (2026-05-14). Replaced by `anvil_query_boundary` (RMCPF-011) and `docs/architecture/rust-mcp-server-spec.md`. |
| `pre-generation` | `archive/anvil-mcp-server/src/prompts/pre-generation.prompt.ts` | `{ workspaceRoot, targetFile? }` | Emits generic architecture and anti-pattern constraints before generation. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012. | **Retired** (2026-05-14). The constraints belong in tool descriptions and authoritative docs, not in a transient prompt list that bypasses ADR review. |

### Prompts — RMCPF-012 disposition (2026-05-14)

All four archived prompts are **retired**. The Rust MCP server does not ship a
prompts surface for these reasons:

1. **Phase 0 demand check is negative.** RMCPF-003 confirmed that supported
   Phase 1 clients (Claude Code and Cursor) do not depend on the archived
   prompts to call `anvil_check` / `anvil_gate` / `anvil_suppress`. The prompts
   were convenience text in the TS server, not policy.
2. **Policy belongs in ADRs and docs.** `docs/architecture/rust-mcp-server-spec.md`
   §"Prompt Strategy" warns that prompt content "must not become a hidden source
   of architecture policy. Durable policy remains in ADRs, APS, docs, schemas,
   and code." Re-porting the prompts would re-introduce that hidden surface.
3. **Tool descriptions carry the actionable text.** The new RMCPF-011 tools
   (`anvil_fix`, `anvil_suppress`, `anvil_query_boundary`) include the
   limitations and call-out hints inline so any MCP client surfaces them
   automatically — clients do not need a separate `prompts/get` round trip.

The Rust server enforces the disposition in two ways:

- `initialize` capabilities omit `prompts`, so MCP clients negotiate without it.
  Integration test:
  `crates/anvil-cli/tests/mcp_serve_stdio.rs::mcp_serve_stdio_initialize_does_not_advertise_prompts_capability`.
- `prompts/list` returns JSON-RPC error code `-32601 Method not found`, so any
  caller that ignores the capability negotiation sees a clear failure rather
  than an empty list. Integration test:
  `crates/anvil-cli/tests/mcp_serve_stdio.rs::mcp_serve_stdio_prompts_list_returns_method_not_found`.

The archived TS prompts remain in `archive/anvil-mcp-server/src/prompts/` as
frozen reference until RMCPF-031 closes out the archive itself. RMCPF-030 must
include the retirement in its compatibility matrix and migration docs.

If a supported client surfaces fresh demand for any of these prompts, re-open
RMCPF-012 with the client name, the contract the client expects, and an ADR
update covering whose responsibility the policy is. **Do not** re-add prompts
silently.

## Transports

| Transport | File | Contract | Current tests | Owner / follow-on item | Disposition |
| --- | --- | --- | --- | --- | --- |
| stdio | `archive/anvil-mcp-server/src/bin.ts` | MCP stdio via the SDK; stdout is reserved for protocol frames. | `archive/anvil-mcp-server/src/server.test.ts` exercises in-memory registration; RMCP covers Rust stdio launch. | RMCPF-010/RMCPF-020 extend the existing RMCP stdio server. | Canonical Rust transport. Extend for full parity. |
| Streamable HTTP | `archive/anvil-mcp-server/src/transports/streamable-http.ts` | Express `/mcp` POST/GET/DELETE, `/health`, per-session transport, optional bearer auth with `ANVIL_MCP_API_KEY`, optional rate limit with `ANVIL_MCP_RATE_LIMIT`, security headers. | `archive/anvil-mcp-server/src/transports/streamable-http.test.ts` | RMCPF-021. | Defer until RMCPF-021 confirms a supported client requires HTTP. Otherwise retire with migration note. |

## Client Config Targets

The archived config dispatcher is `archive/anvil-mcp-server/src/config/index.ts`.
It lists `claude-code`, `cursor`, `windsurf`, and `vscode`.

| Target | File | Archived config shape | Current tests / Rust evidence | Owner / follow-on item | Disposition |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `archive/anvil-mcp-server/src/config/claude-code.ts` | `.claude/mcp.json`; stdio launches `npx @eddacraft/anvil-mcp-server`; HTTP uses a local `/mcp` URL. | Archived: `archive/anvil-mcp-server/src/config/config.test.ts`; Rust: `crates/anvil-cli/tests/mcp_config.rs` expects `anvil mcp serve --stdio`. | RMCPF-030 compatibility harness; RMCPF-031 migration docs. | Support first. |
| Cursor | `archive/anvil-mcp-server/src/config/cursor.ts` | `.cursor/mcp.json`; stdio launches the npm package; HTTP uses a local `/mcp` URL. | Archived: `archive/anvil-mcp-server/src/config/config.test.ts`; Rust: `crates/anvil-cli/tests/mcp_config.rs` expects `anvil mcp serve --stdio`. | RMCPF-030 compatibility harness; RMCPF-031 migration docs. | Support first. |
| Windsurf | `archive/anvil-mcp-server/src/config/windsurf.ts` | `~/.codeium/windsurf/mcp_config.json`; stdio or HTTP config. | Archived: `archive/anvil-mcp-server/src/config/config.test.ts`; Rust: `crates/anvil-cli/tests/mcp_config.rs` rejects the target. | RMCPF-002 supported-client decision. | Defer until protocol and client behaviour are revalidated. |
| VS Code | `archive/anvil-mcp-server/src/config/vscode.ts` | `.vscode/mcp.json`; `servers.anvil` with `type: "stdio"` or `type: "http"`. | Archived: `archive/anvil-mcp-server/src/config/config.test.ts`; Rust: `crates/anvil-cli/tests/mcp_config.rs` rejects the target. | RMCPF-002 supported-client decision. | Defer until a supported extension path exists. |

Continue is named in the RMCPF Ready Checklist but has no archived TypeScript
config generator. Treat Continue support as a fresh Rust decision, not a TS
parity requirement.

### Phase 0 client and transport decision (2026-05-14)

RMCPF Phase 1 targets the two clients with both archived TypeScript evidence and
active Rust launch evidence:

- **Claude Code:** archived generator exists in
  `archive/anvil-mcp-server/src/config/claude-code.ts`; Rust config/install tests
  cover `anvil mcp serve --stdio` in `crates/anvil-cli/tests/mcp_config.rs`.
- **Cursor:** archived generator exists in
  `archive/anvil-mcp-server/src/config/cursor.ts`; Rust config/install tests
  cover `anvil mcp serve --stdio` in `crates/anvil-cli/tests/mcp_config.rs`.

The remaining named clients are deferred rather than treated as parity targets:

- **Continue:** no archived TypeScript config generator exists, so support is a
  fresh product decision.
- **VS Code:** archived config existed, but current Rust intentionally rejects the
  target until an active extension path is verified.
- **Windsurf:** archived config existed, but current Rust intentionally rejects
  the target because protocol and config behaviour are unverified.

Stdio is the only required Phase 1 transport. Streamable HTTP remains historical
archive evidence and is deferred to RMCPF-021; it should be retired with a
migration note unless a supported client proves HTTP demand before implementation
starts.

`archive/anvil-mcp-server` remains frozen reference material until RMCPF-031. It
can retire only after retained tools/resources/prompts/transports have Rust parity
or documented retirements, generated client configs point at Rust MCP, and
RMCPF-030 migration/compatibility evidence names every intentional
incompatibility.

## Shared Utilities And Test Gaps

- `archive/anvil-mcp-server/src/utils/validate-workspace.ts` requires an
  absolute existing directory and constrains client-provided roots under the
  configured server root after realpath resolution.
- `archive/anvil-mcp-server/src/utils/validate-workspace.test.ts` covers the
  root validation contract.
- `archive/anvil-mcp-server/vitest.config.ts` includes utils, suppress/fix
  tools, transports, and prompts. Some relevant test files exist outside that
  include set (`server`, `config`, `resources`, `check`, `gate`, `status`, and
  `query-boundary`), so RMCPF-030 must build an explicit compatibility harness
  rather than assuming the archived package test command covers every surface.

## Follow-On Work Mapping

| Follow-on item | Inventory consequence |
| --- | --- |
| RMCPF-002 | Resolve tool classification, Streamable HTTP, prompt, supported-client, and suppression-authority decisions before architecture is accepted. |
| RMCPF-010 | Port `anvil_check`, `anvil_gate`, and `anvil_status`. |
| RMCPF-011 | Port `anvil_fix`, `anvil_suppress`, and `anvil_query_boundary`; block suppress mutation on explicit authority choice. |
| RMCPF-012 | Decide whether prompts are product surface; defer by default. |
| RMCPF-020 | Port resources using Rust-owned sources of truth. |
| RMCPF-021 | Decide whether Streamable HTTP is required; stdio remains canonical. |
| RMCPF-030 | Compare Rust outputs against archived TS fixture behaviour for retained surfaces. |
| RMCPF-031 | Keep `archive/anvil-mcp-server` until all ported and retired surfaces are documented. |

## Open Decisions

1. **Streamable HTTP:** Deferred for RMCPF-021; retire unless a supported client
   requires HTTP before implementation starts.
2. **Supported clients:** Claude Code and Cursor are Phase 1 parity clients;
   Windsurf, VS Code, and Continue need fresh support decisions.
3. **Prompts:** Decide whether MCP prompts are product surface or historical
   convenience text.
4. **Suppression authority:** Choose daemon-authorised mutation or local driver
   composition before porting `anvil_suppress`.
5. **Compatibility exactness:** Preserve response shapes where useful, but cite
   Rust sources as authoritative for config, drift, constraints, suppressions,
   and diagnostics.
