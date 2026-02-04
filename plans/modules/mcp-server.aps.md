<!--
APS Module: MCP Server
======================
Exposes Anvil validation as MCP tools for AI assistants.
See: plans/aps-rules.md
Target MCP spec: 2025-11-25
-->

# MCP Server

| ID  | Owner | Status |
| --- | ----- | ------ |
| MCP | —     | Ready  |

## Purpose

Enable AI assistants (Claude Code, Cursor, Windsurf, VS Code Copilot) to
validate code during generation by exposing Anvil's check, gate, architecture
analysis, and deterministic auto-fix as MCP tools. This shifts validation left —
catching issues before code is accepted rather than after commit.

**Target spec:** MCP 2025-11-25 (Agentic AI Foundation / Linux Foundation)

## In Scope

- MCP server package (`@eddacraft/anvil-mcp-server`)
- **Tools** (actions with side effects or computation):
  - `anvil_check` — File validation with warnings
  - `anvil_gate` — Full quality gate execution
  - `anvil_status` — Project health summary
  - `anvil_explain` — Warning deep-dive with fixes
  - `anvil_fix` — Deterministic auto-fix for safe patterns (AP-004 `@ts-ignore`
    → `@ts-expect-error`, AP-003 `: any` → `: unknown`, etc.)
  - `anvil_suppress` — Insert suppression comment at a specific location
  - `anvil_query_boundary` — Pre-generation query: "can module A import from
    module B?"
- **Resources** (read-only context for AI):
  - `anvil://baseline` — Current architecture baseline
  - `anvil://boundaries` — Boundary rules and allowed/forbidden edges
  - `anvil://patterns` — Anti-pattern catalogue with explanations
  - `anvil://suppressions` — Active suppressions and their expiry dates
  - `anvil://config` — Current Anvil configuration
  - `anvil://constraints` — Aggregated llms.txt-style constraints (integrates
    with LLMS module)
  - `anvil://drift` — Current drift status vs baseline
  - Resource templates: `anvil://file/{path}/warnings` for per-file results
  - Resource subscriptions with `listChanged` for baseline/config updates
- **Prompts** (reusable templates):
  - `fix-violation` — Guided prompt for resolving a specific warning
  - `suppress-violation` — Guided prompt for adding a time-boxed suppression
  - `architecture-review` — Prompt template for reviewing code against
    architecture rules
  - `pre-generation` — Constraints prompt to prepend before code generation
- **Roots** support: Accept workspace root from client to scope analysis
- Stdio transport (primary), Streamable HTTP transport (secondary)
- Configuration generators for Claude Code, Cursor, Windsurf, and VS Code
- Tool annotations (readOnlyHint, destructiveHint, idempotentHint per
  2025-06-18 spec)
- Structured tool output schemas

## Out of Scope

- Remote multi-tenant deployment (v2.1+)
- Sampling-based AI explanations (future enhancement — server requesting LLM
  completions from client for richer `anvil_explain` output)
- Elicitation (future enhancement — requesting user input mid-session for
  template selection or suppression reason)
- Custom tool creation API (users extend via adapters)

## Interfaces

**Depends on:**

- `@eddacraft/anvil-core` — CheckRunner, GateRunner, ArchitectureService,
  SuppressionService, BaselineManager
- `@eddacraft/anvil-cli` — Configuration loading, project detection
- `@modelcontextprotocol/sdk` (v1.x) — MCP server implementation
- `@eddacraft/anvil-llms-export` — ConstraintExporter (for `anvil://constraints`
  resource)

**Exposes:**

- `@eddacraft/anvil-mcp-server` — Standalone MCP server binary
- All tools, resources, and prompts listed in In Scope
- Server capabilities object advertising: `tools` (with `listChanged`),
  `resources` (with `subscribe`, `listChanged`), `prompts` (with `listChanged`),
  `logging`

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [x] MCP spec version pinned (2025-11-25)
- [x] All six MCP primitives evaluated (tools, resources, prompts, sampling,
      roots, elicitation)

## Tasks

### MCP-001: Package scaffold and basic server

- **Intent:** Establish MCP server package with working stdio transport
- **Expected Outcome:** `npx @eddacraft/anvil-mcp-server` starts, completes
  3-step MCP handshake (initialize → response → initialized), and advertises
  capabilities
- **Validation:** `pnpm -F mcp-server test`
- **Files:** `packages/mcp-server/src/index.ts`,
  `packages/mcp-server/src/server.ts`
- **Dependencies:** None (foundational)
- **Confidence:** high
- **Notes:** Target `@modelcontextprotocol/sdk` v1.x. Pin version in
  package.json. Server must accept `roots` from client and use the first root as
  project directory.

### MCP-002: anvil_check tool implementation

- **Intent:** Expose core validation as MCP tool
- **Expected Outcome:** AI can invoke `anvil_check` with file paths or content,
  receives warnings with locations, explanations, and suggestions as structured
  JSON
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="check tool"`
- **Files:** `packages/mcp-server/src/tools/check.tool.ts`
- **Dependencies:** MCP-001
- **Confidence:** high
- **Notes:** Include tool annotations: `readOnlyHint: true`,
  `destructiveHint: false`, `idempotentHint: true`. Define output schema so
  clients know the response shape.

### MCP-003: anvil_gate and anvil_status tools

- **Intent:** Expose gate runner and status summary as tools
- **Expected Outcome:** AI can run full gate or get quick health check
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="gate|status"`
- **Files:** `packages/mcp-server/src/tools/gate.tool.ts`,
  `packages/mcp-server/src/tools/status.tool.ts`
- **Dependencies:** MCP-002
- **Confidence:** high
- **Notes:** `anvil_gate` may take >2s on large repos. Return progress
  notifications via MCP logging. Consider async task pattern (2025-11-25 spec)
  for repos with >1000 files.

### MCP-004: anvil_fix and anvil_suppress tools

- **Intent:** Expose deterministic auto-fix and suppression insertion as tools
- **Expected Outcome:** AI can apply safe, mechanical fixes (AP-004
  `@ts-ignore` → `@ts-expect-error`, AP-003 `: any` → `: unknown`, AP-001
  broad disable → next-line disable) and insert time-boxed suppression comments
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="fix|suppress"`
- **Files:** `packages/mcp-server/src/tools/fix.tool.ts`,
  `packages/mcp-server/src/tools/suppress.tool.ts`
- **Dependencies:** MCP-002
- **Confidence:** high
- **Notes:** `anvil_fix` annotations: `readOnlyHint: false`,
  `destructiveHint: false`, `idempotentHint: true`. Only deterministic,
  non-AI fixes — no heuristic rewrites. `anvil_suppress` must require a reason
  string and default to 30-day expiry.

### MCP-005: anvil_query_boundary tool

- **Intent:** Let AI ask "can module A import from module B?" before writing code
- **Expected Outcome:** AI queries boundary rules pre-generation, gets
  allow/deny with explanation of the architectural constraint
- **Validation:**
  `pnpm -F mcp-server test -- --testNamePattern="query.boundary"`
- **Files:** `packages/mcp-server/src/tools/query-boundary.tool.ts`
- **Dependencies:** MCP-002
- **Confidence:** high
- **Notes:** This is the most impactful tool for preventing violations during
  generation. Annotations: `readOnlyHint: true`, `idempotentHint: true`.

### MCP-006: Resources with subscriptions

- **Intent:** Expose baseline, config, patterns, suppressions, constraints,
  and drift as readable MCP resources with URI templates and subscriptions
- **Expected Outcome:** AI can read project context via `anvil://` URIs.
  Clients subscribed to `anvil://baseline` receive notifications when baseline
  changes on disk.
- **Validation:**
  `pnpm -F mcp-server test -- --testNamePattern="resource|subscribe"`
- **Files:** `packages/mcp-server/src/resources/`
- **Dependencies:** MCP-001
- **Confidence:** medium
- **Notes:** Implement `resources/subscribe` capability. Use file watchers
  on `.anvil/` directory to detect baseline/config changes. Resource template
  `anvil://file/{path}/warnings` enables per-file queries. Integrate with
  LLMS module's ConstraintExporter for `anvil://constraints`.

### MCP-007: Prompt templates

- **Intent:** Provide reusable prompt templates for common workflows
- **Expected Outcome:** AI tools can list and use guided prompts for fixing
  violations, adding suppressions, reviewing architecture, and pre-generation
  constraint loading
- **Validation:**
  `pnpm -F mcp-server test -- --testNamePattern="prompt"`
- **Files:** `packages/mcp-server/src/prompts/`
- **Dependencies:** MCP-006
- **Confidence:** medium
- **Notes:** The `pre-generation` prompt is key — it outputs all architecture
  constraints so the AI can include them in its context before writing code.
  Prompts should support dynamic arguments (e.g., warning ID for `fix-violation`).

### MCP-008: Streamable HTTP transport

- **Intent:** Support remote server mode via Streamable HTTP (MCP 2025-03-26+)
- **Expected Outcome:** Server runs on a single `/mcp` endpoint, supports
  `Mcp-Session-Id` headers, and handles both POST and GET (SSE streaming)
- **Validation:**
  `pnpm -F mcp-server test -- --testNamePattern="http|streamable"`
- **Files:** `packages/mcp-server/src/transports/streamable-http.ts`
- **Dependencies:** MCP-001
- **Confidence:** medium
- **Notes:** Use `@modelcontextprotocol/node` or `@modelcontextprotocol/express`
  helpers. Bind to localhost by default. HTTPS and OAuth 2.1 required for
  non-localhost deployments (per spec). Do NOT implement deprecated SSE
  transport.

### MCP-009: Configuration generators and CLI integration

- **Intent:** Generate config files for Claude Code, Cursor, Windsurf, and
  VS Code; add `anvil mcp-config` CLI command
- **Expected Outcome:** `anvil mcp-config --target claude-code` outputs valid
  JSON config; `anvil mcp-config --target cursor` outputs valid
  `.cursor/mcp.json`
- **Validation:** `pnpm -F mcp-server test -- --testNamePattern="config"`
- **Files:** `packages/mcp-server/src/config/`,
  `apps/anvil-cli/src/commands/mcp-config.ts`
- **Dependencies:** MCP-008
- **Confidence:** high
- **Notes:** Each target has its own config format:
  - Claude Code: `.claude/mcp.json`
  - Cursor: `.cursor/mcp.json`
  - Windsurf: Windsurf settings
  - VS Code: `.vscode/mcp.json`

### MCP-010: Error handling and JSON-RPC compliance

- **Intent:** Implement proper JSON-RPC 2.0 error responses with structured
  error data
- **Expected Outcome:** All tool/resource errors return correct JSON-RPC error
  codes (`-32602` for invalid params, `-32603` for internal errors) with
  helpful `data` fields
- **Validation:**
  `pnpm -F mcp-server test -- --testNamePattern="error"`
- **Files:** `packages/mcp-server/src/errors/`
- **Dependencies:** MCP-001
- **Confidence:** high
- **Notes:** Never expose internal stack traces. Include actionable guidance
  in error data (e.g., "file not found — did you mean src/index.ts?").

## Execution

Steps: [../execution/MCP-001.steps.md](../execution/MCP-001.steps.md)

## Risks

| Risk                          | Impact | Mitigation                                         |
| ----------------------------- | ------ | -------------------------------------------------- |
| MCP SDK breaking changes      | High   | Pin v1.x, monitor releases, stable v2 expected Q1 2026 |
| Slow response on large repos  | Medium | Incremental check, caching, async tasks for >2s ops |
| AI misinterprets tool output  | Medium | Structured JSON with output schemas, clear errors  |
| Tool poisoning (supply chain) | Medium | Signed tool definitions, integrity checks (see Aegis brainstorm) |
| Streamable HTTP security      | Medium | Bind localhost by default, require OAuth for remote |

## Resolved Questions

- **`anvil_watch` → Resource with subscription.** The MCP spec supports resource
  subscriptions (`resources/subscribe`). Watch mode maps to subscribing to
  `anvil://baseline` or `anvil://file/{path}/warnings` resources. The server
  sends `notifications/resources/updated` when files change. No need for a
  polling tool.

- **`anvil_fix` → Yes, for deterministic fixes only.** Safe, mechanical
  transforms (AP-004 `@ts-ignore` → `@ts-expect-error`, AP-003 `: any` →
  `: unknown`) are no riskier than ESLint `--fix`. AI does not generate the
  fix — Anvil's deterministic engine does. Added as MCP-004.

- **MCP registry → Use server discovery.** The 2025-11-25 spec supports server
  discovery via well-known URLs. Publish a discovery document rather than
  listing in a third-party registry.

## Spec Compliance Notes

The following MCP 2025-11-25 features are evaluated:

| Primitive     | Status      | Notes                                                |
| ------------- | ----------- | ---------------------------------------------------- |
| Tools         | In scope    | 7 tools with annotations and output schemas          |
| Resources     | In scope    | 8 resource URIs with templates and subscriptions     |
| Prompts       | In scope    | 4 prompt templates with dynamic arguments            |
| Sampling      | Deferred    | Could enhance `anvil_explain` with AI-generated fixes; requires client LLM access |
| Roots         | In scope    | Accept workspace root from client                    |
| Elicitation   | Deferred    | Could request suppression reason or template choice; low priority |
| Async Tasks   | In scope    | For long-running gate checks on large repos          |
| Logging       | In scope    | Progress notifications during gate execution         |
| OAuth 2.1     | In scope    | Required for non-localhost Streamable HTTP            |
| Server Discovery | Deferred | Publish identity document at well-known URL          |
