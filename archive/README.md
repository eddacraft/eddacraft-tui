# archive/ — moved to `anvil-archive`

The retired code that used to live here has been **moved to a sibling
repository**: [`anvil-archive`](../../anvil-archive) (`../anvil-archive`
relative to the repo root).

Nothing in this directory builds or ships — it was reference-only dead code
(`pnpm-workspace.yaml` and `nx.json` already excluded `archive/**`). It was
relocated to keep the main repo focused on the live Rust engine.

## Where things went

| Was `archive/…`           | Now at `anvil-archive/…`  | What it was                                  |
| ------------------------- | ------------------------- | -------------------------------------------- |
| `anvil-ts-scanner/`       | `anvil-ts-scanner/`       | Original TypeScript scanner (ADR-033)        |
| `anvil-mcp-server/`       | `anvil-mcp-server/`       | Node MCP server (ADR-033, → RMCP)            |
| `anvil-vscode-extension/` | `anvil-vscode-extension/` | VS Code extension (ADR-033, → `anvil lsp`)   |
| `anvil-cli-node/`         | `anvil-cli-node/`         | Legacy Node.js `anvil` CLI                   |
| `admin-cli-node/`         | `admin-cli-node/`         | Node operator CLI (→ `anvil edda`/`ember`)   |
| `anvil-tui-ink/`          | `anvil-tui-ink/`          | Ink TUI (ADR-011, → Ratatui)                 |
| `eddacraft-tui-local/`    | `eddacraft-tui-local/`    | Pre-publication fork of `eddacraft-tui`      |
| `tools-node/`             | `tools-node/`             | Node build/dev tooling                       |

Source comments that still reference `archive/<project>/` paths point at the
same relative layout, now under `anvil-archive/`.
