# anvil-cli

The Anvil CLI binary — structural governance for AI-assisted development.

## Usage

```bash
cargo run -p anvil-cli -- --help
```

Produces the `anvil` binary with 16+ subcommands covering TUI surfaces,
governance gates, policy management, and authentication.

## Structure

- **`commands/`** — clap-derived subcommand handlers
- **`auth/`** — device-flow and session authentication
- **`output/`** — structured output formatting (JSON, table, plain)
- **`tui.rs`** — TUI runner functions (`run_surface`, `run_watch`)

## Part of

[EddaCraft Anvil](../../README.md) monorepo (`crates/anvil-cli`).
