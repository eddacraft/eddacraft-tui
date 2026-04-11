# anvil-cli

The Anvil CLI binary — structural governance for AI-assisted development.

## Supported Platforms

| OS      | Architecture            | Target Triple               |
| ------- | ----------------------- | --------------------------- |
| macOS   | x86_64                  | `x86_64-apple-darwin`       |
| macOS   | aarch64 (Apple Silicon) | `aarch64-apple-darwin`      |
| Linux   | x86_64                  | `x86_64-unknown-linux-gnu`  |
| Linux   | aarch64                 | `aarch64-unknown-linux-gnu` |
| Windows | x86_64                  | `x86_64-pc-windows-msvc`    |
| Windows | aarch64                 | `aarch64-pc-windows-msvc`   |

## Usage

```bash
cargo run -p eddacraft-anvil -- --help
```

Produces the `anvil` binary (or `anvil.exe` on Windows) with 16+ subcommands
covering TUI surfaces, governance gates, policy management, and authentication.

## Structure

- **`commands/`** — clap-derived subcommand handlers
- **`auth/`** — device-flow and session authentication
- **`output/`** — structured output formatting (JSON, table, plain)
- **`tui.rs`** — TUI runner functions (`run_surface`, `run_watch`)

## Cross-Platform Notes

- File paths are normalised to forward slashes internally
- Credential storage is persisted as JSON on disk under the user's configuration
  directory:
  - macOS: `~/Library/Application Support/anvil/credentials.json`
  - Linux: `~/.config/anvil/credentials.json`
  - Windows: `%APPDATA%\anvil\credentials.json`
- TUI rendering uses crossterm for cross-platform terminal support

## Part of

[EddaCraft Anvil](../../README.md) monorepo (`crates/anvil-cli`).
