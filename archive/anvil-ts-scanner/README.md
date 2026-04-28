# Anvil TS Scanner (Archived)

> **Archived (2026-04-29) under [ADR-033](../../plans/decisions/033-park-ide-mcp-retire-ts-scanner.md).**
> This directory carries the retired TypeScript anti-pattern
> scanner, suppression parser, gate runner, constraint collector,
> and parity harness. None of it builds, runs, tests, or installs.
> The `pnpm-workspace.yaml` `'!archive/**'` glob excludes it from
> the active workspace.

## What's in here

This is a **multi-source archive** — unlike `archive/anvil-cli-node/`
which is a single self-contained package, this directory carves
the TS scanner ecosystem out of multiple packages:

```
archive/anvil-ts-scanner/
├── README.md                ← this file
├── core-antipattern/        ← was packages/anvil/core/src/antipattern/
├── core-suppression/        ← was packages/anvil/core/src/suppression/
├── runtime-gate/            ← was packages/anvil/runtime/src/gate/
├── runtime-export/          ← was packages/anvil/runtime/src/export/constraint-collector*
└── scanner-parity/          ← was tests/scanner-parity/
```

Internal imports inside these files (e.g. `@eddacraft/anvil-core/antipattern`,
`./scanner.js`) will not resolve any more — the active workspace
no longer maps those paths. Treat the contents as read-only
reference material.

## Why archived

The TypeScript scanner stack existed to feed two consumers — the
VSCode extension and the TypeScript MCP server — both of which
are archived under
[`archive/anvil-vscode-extension/`](../anvil-vscode-extension/)
and [`archive/anvil-mcp-server/`](../anvil-mcp-server/). With
both consumers archived, the TS scanner had no active surface to
serve, while still costing dual-engine maintenance (every rule
change re-validated against TS, the parity harness CI on every
PR, regex-engine divergence as user-visible UX risk).

The Rust scanner in `crates/anvil-checks/` is now the sole
implementation; the Rust binary serves the launch MCP path via
`anvil mcp serve --stdio` (RMCP). RMCPF will port the full TS MCP
server feature set into Rust as next-release work; the editor
return path is DRVR-003 on the intercept daemon (DRVR module).

## Use this instead

For anti-pattern checks and suppressions today, use the `anvil`
CLI:

```bash
anvil check          # one-shot validation
anvil watch          # save-time watcher
anvil mcp install    # configure MCP for Cursor / Claude Code
```

The Rust scanner reads the same compiled
`patterns/compiled/registry.json` that this archive used to
consume, so rule content is preserved across the migration.

## Reference paths

- ADR-033 — [`plans/decisions/033-park-ide-mcp-retire-ts-scanner.md`](../../plans/decisions/033-park-ide-mcp-retire-ts-scanner.md)
- ADR-026 (Rust scanner authoritative) — [`plans/decisions/026-rust-scanner-authoritative.md`](../../plans/decisions/026-rust-scanner-authoritative.md)
- TSRET module — [`plans/modules/anvil-ts-scanner-retirement.aps.md`](../../plans/modules/anvil-ts-scanner-retirement.aps.md)
- Rust scanner — `crates/anvil-checks/`
- Rust suppression parser — `crates/anvil-checks/src/antipattern/scanner.rs`
- RMCP module — [`plans/modules/rust-mcp-launch-shim.aps.md`](../../plans/modules/rust-mcp-launch-shim.aps.md)
- DRVR module — [`plans/modules/surface-drivers.aps.md`](../../plans/modules/surface-drivers.aps.md)
