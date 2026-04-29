# @eddacraft/anvil-mcp-server (Archived)

> **Archived (2026-04-29) under [ADR-033](../../plans/decisions/033-park-ide-mcp-retire-ts-scanner.md).**
> This package was moved from `packages/mcp-server/` to
> `archive/anvil-mcp-server/`. It is **not built, tested,
> released, or published to npm**. The
> `pnpm-workspace.yaml` `'!archive/**'` glob excludes it from the
> active workspace. RMCPF reads `archive/anvil-mcp-server/src/`
> as frozen contract source for the Rust port.
>
> **Use this instead.** The launch-critical MCP path ships in the
> single Rust `anvil` binary via
> [RMCP](../../plans/modules/rust-mcp-launch-shim.aps.md):
>
> ```bash
> anvil mcp install --client cursor      # or claude-code
> # editor / agent then launches:  anvil mcp serve --stdio
> ```
>
> RMCP covers pre-write validation against Anvil's authoritative
> rule set with the canonical diagnostic envelope. Full feature
> parity with this TS server is queued under
> [RMCPF](../../plans/modules/rust-mcp-full-port.aps.md), executed
> against the Rust binary — not by reviving this package.
>
> **Why archived rather than evolved in place:** The TS server
> imports the TS scanner; carrying both alive while RMCP already
> covers the launch path is dual-engine cost without realised
> benefit. ADR-033 archives this package and retires the TS
> scanner in the same change.
>
> The documentation below describes the pre-archive feature set
> and is preserved for historical context.

---

MCP (Model Context Protocol) server that exposes Anvil's validation, analysis,
and configuration as tools and resources for AI coding assistants. Supports
Claude Code, Cursor, Windsurf, and VS Code via generated configuration.

## Status

Archived 2026-04-29 per ADR-033. Replaced for the launch path by the Rust
shim ([RMCP](../../plans/modules/rust-mcp-launch-shim.aps.md));
parity port queued under
[RMCPF](../../plans/modules/rust-mcp-full-port.aps.md). Pre-archive
status was *Active -- shippable*.

## Installation

```bash
npm install @eddacraft/anvil-mcp-server
```

Or run directly via the CLI binaries:

```bash
# stdio transport (default for Claude Code)
anvil-mcp-server

# HTTP transport (Streamable HTTP)
anvil-mcp-server-http
```

## Tools

- **check** -- Run Anvil checks on files
- **fix** -- Auto-fix violations
- **gate** -- Run the full gate pipeline
- **query-boundary** -- Query architecture boundaries
- **status** -- Get workspace validation status
- **suppress** -- Suppress specific warnings

## Resources

- **baseline** -- Current baseline snapshot
- **boundaries** -- Architecture boundary definitions
- **config** -- Anvil configuration
- **constraints** -- Exported constraint summaries (llms.txt format)
- **drift** -- Drift detection results
- **file-warnings** -- Per-file warning details
- **patterns** -- Antipattern definitions
- **suppressions** -- Active suppressions

## Prompts

- **architecture-review** -- Review architecture for violations
- **fix-violation** -- Generate a fix for a specific violation
- **pre-generation** -- Pre-generation context for code assistants
- **suppress-violation** -- Suppress a violation with rationale

## Config Generation

Generate MCP configuration for your editor:

```ts
import { generateMcpConfig } from '@eddacraft/anvil-mcp-server/config';

const config = generateMcpConfig('claude-code');
// Also supports: 'cursor', 'windsurf', 'vscode'
```

## API Surface

```ts
import { createAnvilMcpServer } from '@eddacraft/anvil-mcp-server';
import {
  generateMcpConfig,
  SUPPORTED_TARGETS,
} from '@eddacraft/anvil-mcp-server/config';
import { startHttpServer } from '@eddacraft/anvil-mcp-server';
```

## Consumers

- Claude Code (via stdio transport)
- Cursor, Windsurf, VS Code (via generated config)

## Development

```bash
pnpm --filter @eddacraft/anvil-mcp-server build
pnpm --filter @eddacraft/anvil-mcp-server test
```
