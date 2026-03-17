# ADR-012: Rust CLI Replacement

**Status:** Accepted
**Date:** 2026-03-18
**Deciders:** aneki

## Context

The Anvil CLI (`apps/anvil-cli/`) is a Node.js application using Commander.js for
argument parsing and Ink (React) for terminal UI. It contains 67 commands across
~25,420 LOC with ~40 supporting services.

The project has been migrating core infrastructure to Rust:
- `anvil-kernel` — watcher, parser, semantic graph, policy engine
- `anvil-tui` / `eddacraft-tui` — Ratatui TUI surfaces with shared shell chrome
- `anvil-kernel-types` — canonical event protocol
- `anvil-checks` — gate check implementations

The Node.js CLI is increasingly a thin shell around Rust work. Maintaining two
runtimes, an IPC bridge, and binary distribution for a spawn-based approach
creates permanent tech debt without clear benefit.

## Decision

Replace the Node.js CLI with a Rust CLI (`crates/anvil-cli/`) that produces a
single `anvil` binary. Big bang migration — no hybrid period.

### Crate Structure

```
crates/anvil-cli/           Binary crate — clap entry point, commands, auth, output
crates/anvil-policy/        NEW library — policy config, evaluation, bundles
crates/anvil-architecture/  NEW library — definitions, boundaries, validation
crates/anvil-kernel/        EXISTS — watcher, parser, graph, events
crates/anvil-kernel-types/  EXISTS — EngineEvent, shared types
crates/anvil-tui/           EXISTS — surfaces, shell, app controller
crates/eddacraft-tui/       EXISTS — shared widget library
crates/anvil-checks/        EXISTS — gate check implementations
```

Services with clear domain boundaries (policy, architecture) become their own
crates. Thinner glue (repo scanning, template generation, historical analysis)
starts inside the CLI crate and gets extracted if/when it grows.

### Command Priority

**Tier 1** (initial release):
- Core workflow: `init`, `watch`, `gate`, `status`, `doctor`, `audit`
- TUI surfaces: `tutorial`, `start` (welcome), `new` (browser)
- Architecture: `architecture validate/watch`
- Auth: `auth login/logout/whoami`, `admin approve`
- Policy: `policy list/explain/diff/validate/test` + subcommands

**Tier 2** (fast follow): hooks, export, pr-comment, exception, policy-debug,
policy-watch

**Tier 3** (subsequent): edda/ember/stack subsystem commands

### TUI Integration

A `Surface` trait formalises the contract:

```rust
pub trait Surface {
    fn surface_name(&self) -> &'static str;
    fn help_text(&self) -> &'static str;
    fn handle_key(&mut self, action: Action);
    fn should_quit(&self) -> bool;
    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme);
}
```

The shared shell (header with block logo, footer with help keys and watermark)
is rendered by the parent. Surfaces receive only their core `Rect`.

The `watch` command spawns the kernel watcher on a background thread and feeds
`EngineEvent`s to the TUI via `mpsc`. All other TUI commands are synchronous.

### Auth

Device code and OTP flows ported natively using `reqwest` with `rustls-tls`.
Credentials stored at `~/.config/anvil/credentials.json` (XDG-compliant), same
location as the Node.js CLI for interoperability during transition.

The Hono API server (`apps/anvil-api/`) is unchanged — the Rust CLI is a client.

### Distribution

- GitHub Releases with pre-built binaries (x86_64/aarch64 for Linux and macOS)
- Install script: `curl -fsSL https://install.eddacraft.ai | sh`
- `cargo install anvil-cli` for Rust users
- Optional npm wrapper package with postinstall binary download

### Migration

1. Build Rust CLI with Tier 1 commands
2. Test against existing fixtures
3. Ship pre-built binaries
4. Archive Node.js CLI to `archive/anvil-cli-node/` (preserve for reference)
5. Tag/fork the repository before archival for full preservation

## Consequences

**Positive:**
- Single binary, single runtime — no IPC, no binary distribution headaches
- Native kernel integration (same-process `mpsc` for watch/gate)
- TUI surfaces are direct function calls, not spawned processes
- Faster startup, lower memory footprint
- Rust ecosystem consistency (clippy, cargo test, workspace lints)

**Negative:**
- 3-5 week effort for full Tier 1 port
- Architecture service (1,040 LOC) and init wizard (799 LOC) are non-trivial ports
- Requires Rust toolchain for development (already a project requirement)
- npm distribution requires wrapper package pattern

**Neutral:**
- Node.js stays for API server, website, MCP server — not a full runtime removal
- TypeScript domain packages (`packages/`) remain for API/website consumers
