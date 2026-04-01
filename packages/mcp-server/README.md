# @eddacraft/anvil-mcp-server

MCP (Model Context Protocol) server that exposes Anvil's validation, analysis,
and configuration as tools and resources for AI coding assistants. Supports
Claude Code, Cursor, Windsurf, and VS Code via generated configuration.

## Status

Active -- shippable

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
import { generateMcpConfig, SUPPORTED_TARGETS } from '@eddacraft/anvil-mcp-server/config';
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
