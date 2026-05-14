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
| Tools | 6 | Port all, with `anvil_suppress` authority resolved before mutation ships |
| Resources | 8 | Port with Rust-owned sources of truth |
| Prompts | 4 | Defer unless supported clients prove prompt usage |
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
| `anvil://baseline` | `archive/anvil-mcp-server/src/resources/baseline.resource.ts` | Returns `.anvil/architecture.json` or structured no-baseline/load-failed errors. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust architecture/baseline model. | Port. |
| `anvil://boundaries` | `archive/anvil-mcp-server/src/resources/boundaries.resource.ts` | Returns layers and boundaries derived from the baseline. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust architecture/baseline model. | Port. |
| `anvil://patterns` | `archive/anvil-mcp-server/src/resources/patterns.resource.ts` | Returns built-in anti-pattern catalogue entries and count. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust checks/rules catalogue. | Port if the Rust catalogue remains user-visible. |
| `anvil://suppressions` | `archive/anvil-mcp-server/src/resources/suppressions.resource.ts` | Reads `.anvil/suppressions.json` and returns suppression list plus active/expired summary. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust suppression readers. | Port. |
| `anvil://config` | `archive/anvil-mcp-server/src/resources/config.resource.ts` | Returns loaded config, source, default flag, and errors. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; `crates/anvil-config` and CLI config loader. | Port. |
| `anvil://constraints` | `archive/anvil-mcp-server/src/resources/constraints.resource.ts` | Returns aggregated constraints from the archived TS runtime collector. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust export/constraint surfaces. | Port with Rust-owned source, not TS collector parity. |
| `anvil://drift` | `archive/anvil-mcp-server/src/resources/drift.resource.ts` | Returns no-snapshot, single-snapshot, or comparison drift state from latest snapshots. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; Rust drift model and CLI drift command. | Port with Rust-owned source. |
| `anvil://file/{path}/warnings` | `archive/anvil-mcp-server/src/resources/file-warnings.resource.ts` | Validates a workspace path, analyses one file, and returns warnings, summary, checks run, and blocking status. | `archive/anvil-mcp-server/src/resources/resources.test.ts` | RMCPF-020; same Rust path as `anvil_check`. | Port only if client resource reads are still needed; otherwise fold into `anvil_check`. |

## Prompts

Prompt registrations are exported from `archive/anvil-mcp-server/src/prompts/index.ts`
and covered by `archive/anvil-mcp-server/src/prompts/prompts.test.ts`.

| Prompt | File | Args | Behaviour | Current tests | Owner / follow-on item | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| `fix-violation` | `archive/anvil-mcp-server/src/prompts/fix-violation.prompt.ts` | `{ warningId, filePath, line?, message? }` | Returns fix guidance and reminds the client to run `anvil_check`; sanitises newlines/backticks. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012. | Defer. Port only if Cursor or Claude Code prompt usage is product-critical. |
| `suppress-violation` | `archive/anvil-mcp-server/src/prompts/suppress-violation.prompt.ts` | `{ warningId, filePath, line, reason? }` | Guides suppression decisions and the `anvil_suppress` call. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012; coordinates with RMCPF-011 suppress authority. | Defer with `anvil_suppress` authority decision. |
| `architecture-review` | `archive/anvil-mcp-server/src/prompts/architecture-review.prompt.ts` | `{ filePath, workspaceRoot? }` | Produces a boundary-review checklist and suggests `anvil_query_boundary` / `anvil_check`. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012. | Defer; consider documentation instead of MCP prompt parity. |
| `pre-generation` | `archive/anvil-mcp-server/src/prompts/pre-generation.prompt.ts` | `{ workspaceRoot, targetFile? }` | Emits generic architecture and anti-pattern constraints before generation. | `archive/anvil-mcp-server/src/prompts/prompts.test.ts` | RMCPF-012. | Defer or retire. |

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
