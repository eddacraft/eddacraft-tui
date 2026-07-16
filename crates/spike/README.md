# anvil-spike

Validation spikes for evaluating Rust ecosystem libraries before committing to
them in the kernel.

## Binaries

- **`spike-treesitter`** — tree-sitter parsing and query experiments
- **`spike-notify`** — notify-rs file watcher behaviour validation
- **`spike-petgraph`** — petgraph dependency graph modelling
- **`spike-rtai-mid-edit`** — RTAI-001 in-process mid-edit round-trip floor (see
  `plans/specs/2026-04-26-rtai-001-spike-report.md`)
- **`spike-rtai-005-lsp-vs-mcp`** — RTAI-005 LSP vs MCP wire-protocol overhead,
  against a real daemon (see
  `plans/specs/2026-07-16-rtai-005-lsp-vs-mcp-spike-report.md`)

These are throwaway exploration binaries, not production code.

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/spike`).
