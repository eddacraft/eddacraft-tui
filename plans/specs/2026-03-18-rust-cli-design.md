# Rust CLI Design Spec

**Date:** 2026-03-18
**ADR:** 012-rust-cli-replacement
**Status:** Draft

## Overview

Replace the Node.js Anvil CLI with a Rust binary (`anvil`) using clap for
argument parsing and the existing Ratatui TUI surfaces for interactive output.
Big bang migration with Tier 1 commands (core workflow, TUI surfaces,
architecture, auth, policy) shipping first.

---

## 1. Crate Layout

### 1.1 New Crates

**`crates/anvil-cli/`** — binary crate

```
src/
├── main.rs                 clap App, global args, subcommand dispatch
├── commands/
│   ├── mod.rs
│   ├── init.rs             anvil init (uses Init TUI surface)
│   ├── watch.rs            anvil watch (uses Watch TUI surface)
│   ├── gate.rs             anvil gate <plan> (uses Gate TUI surface)
│   ├── status.rs           anvil status (uses Status TUI surface)
│   ├── doctor.rs           anvil doctor [--fix] (uses Doctor TUI surface)
│   ├── audit.rs            anvil audit (uses Audit TUI surface)
│   ├── tutorial.rs         anvil tutorial [--reset] (uses Tutorial TUI surface)
│   ├── welcome.rs          anvil start (uses Welcome TUI surface)
│   ├── new.rs              anvil new (uses Browser TUI surface)
│   ├── wizard.rs           anvil wizard (uses Wizard TUI surface — APS onboarding)
│   ├── auth.rs             anvil auth {login,logout,whoami}
│   ├── admin.rs            anvil admin {approve}
│   ├── policy.rs           anvil policy {list,explain,diff,validate,test,...}
│   ├── architecture.rs     anvil architecture {validate,watch}
│   ├── hooks.rs            anvil hooks {install,status}
│   └── export.rs           anvil export
├── auth/
│   ├── mod.rs
│   ├── device_flow.rs      device code grant (POST /auth/device/*)
│   ├── otp_flow.rs         email OTP (POST /auth/otp/*)
│   ├── credentials.rs      XDG credential storage
│   └── client.rs           AnvilClient (reqwest + token management)
├── services/
│   ├── mod.rs
│   ├── repo_scanner.rs     codebase analysis
│   ├── template_generator.rs  file scaffolding
│   └── historical_analyser.rs  git history analysis
├── output/
│   ├── mod.rs
│   ├── plain.rs            text tables and lists
│   └── json.rs             --json structured output
└── tui.rs                  run_surface() + run_watch() — setup, shell, teardown
```

**`crates/anvil-policy/`** — library crate

```
src/
├── lib.rs
├── config.rs               policy config loading, starter profiles
├── library.rs              built-in policy catalogue
├── evaluator.rs            policy evaluation engine
├── bundle.rs               bundle management, inheritance, versions
├── exceptions.rs           exception approval workflow
└── metadata.rs             policy pack manifest, validation
```

**`crates/anvil-architecture/`** — library crate

```
src/
├── lib.rs
├── definition.rs           architecture definition schema (Zod → serde)
├── boundaries.rs           module boundary enforcement
├── import_rules.rs         import restriction rules
├── file_rules.rs           file placement rules
├── validator.rs            architecture validation
└── config_diagnostics.rs   config health checks
```

### 1.2 Dependencies

```toml
# crates/anvil-cli/Cargo.toml
[dependencies]
anvil-tui = { path = "../anvil-tui" }
anvil-kernel = { path = "../anvil-kernel" }
anvil-kernel-types = { path = "../anvil-kernel-types" }
anvil-policy = { path = "../anvil-policy" }
anvil-architecture = { path = "../anvil-architecture" }
clap = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
crossterm = { workspace = true }
ratatui = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
dirs = { workspace = true }
```

Note: `anvil-tui` re-exports `eddacraft_tui` types (theme, keyboard, widgets).
The CLI does not depend on `eddacraft-tui` directly. Similarly, `anvil-checks`
is invoked through the kernel pipeline, not directly by CLI commands.

TTY detection uses `std::io::IsTerminal` (stable since Rust 1.70), not the
deprecated `atty` crate. This implies a minimum supported Rust version (MSRV)
of 1.70+ for this project, which must be documented in the root `Cargo.toml`
and/or README so developers use a compatible toolchain.

### 1.3 Surface-to-Command Mapping

All 10 TUI surfaces have a corresponding CLI command:

| Surface | Command | Notes |
|---------|---------|-------|
| Welcome | `anvil start` | First-run quick-start menu |
| Tutorial | `anvil tutorial` | Guided walkthrough |
| Doctor | `anvil doctor` | Environment diagnostics |
| Status | `anvil status` | Project overview |
| Gate | `anvil gate <plan>` | Check explorer |
| Watch | `anvil watch` | Live file-watch dashboard (kernel integration) |
| Init | `anvil init` | Project initialisation wizard |
| Wizard | `anvil wizard` | APS onboarding wizard |
| Audit | `anvil audit` | Repository scan results |
| Browser | `anvil new` | Template catalogue browser |

---

## 2. CLI Entry Point

```rust
// main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "anvil",
    version,
    about = "Deterministic governance for probabilistic AI workflows"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON instead of human-readable text
    #[arg(long, global = true)]
    json: bool,

    /// Disable TUI, use plain text output
    #[arg(long, global = true)]
    no_tui: bool,

    /// Enable verbose output
    #[arg(long, short, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialise a new Anvil project
    Init(commands::init::Args),
    /// Watch for file changes and run checks
    Watch(commands::watch::Args),
    /// Run gate checks against a plan
    Gate(commands::gate::Args),
    /// Show project status
    Status(commands::status::Args),
    /// Run environment diagnostics
    Doctor(commands::doctor::Args),
    /// Audit the repository
    Audit(commands::audit::Args),
    /// Interactive tutorial
    Tutorial(commands::tutorial::Args),
    /// First-run welcome screen
    Start(commands::welcome::Args),
    /// Browse and scaffold templates
    New(commands::new::Args),
    /// APS onboarding wizard
    Wizard(commands::wizard::Args),
    /// Authentication
    Auth(commands::auth::Args),
    /// Admin operations
    Admin(commands::admin::Args),
    /// Policy management
    Policy(commands::policy::Args),
    /// Architecture enforcement
    Architecture(commands::architecture::Args),
    /// Git hook management
    Hooks(commands::hooks::Args),
    /// Export constraints
    Export(commands::export::Args),
}
```

### 2.1 Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid args, I/O failure, unexpected) |
| 2 | Gate failed (checks did not pass) |
| 3 | Authentication required (no valid token) |
| 4 | Configuration error (missing/invalid .anvil.yaml) |

Exit codes are defined as constants in `src/main.rs` and used consistently
across all commands. CI consumers (GitHub Actions, git hooks) depend on code 2
for gate failures.

---

## 3. TUI Integration

### 3.1 Surface Trait

Added to `crates/anvil-tui/src/surface.rs` (registered as `pub mod surface` in
`lib.rs`):

```rust
use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

pub trait Surface {
    fn surface_name(&self) -> &'static str;
    fn help_text(&self) -> &'static str;
    fn handle_key(&mut self, action: Action);
    fn should_quit(&self) -> bool;
    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme);
}
```

Note: `render` takes `&self`; surfaces that need to track render-time state
(scroll offsets, cursor blink, animation ticks) can use interior mutability as
appropriate.

All 10 existing surface states implement this trait. Each impl delegates to:
- `self.help_text()` and `self.surface_name()` — already exist on all states
- `self.handle_key(action)` — already exists on all states
- `self.should_quit` — existing field on all states
- `render::render(frame, area, self, theme)` — existing render function

### 3.2 TUI Runner

Two runner functions in `crates/anvil-cli/src/tui.rs`:

**`run_surface`** — for all non-watch surfaces:

```rust
use std::io::{self, IsTerminal};

pub fn run_surface<S: Surface>(mut state: S) -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    loop {
        terminal.draw(|frame| {
            let core = render_shell(
                frame, frame.area(),
                state.surface_name(), state.help_text(), &theme,
            );
            state.render(frame, core, &theme);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                let action = KeyHandler::map(key_event);
                state.handle_key(action);
                if state.should_quit() {
                    break;
                }
            }
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

**`run_watch`** — for the watch command, which drains kernel events:

```rust
pub fn run_watch(mut state: WatchState, event_rx: &mpsc::Receiver<EngineEvent>) -> Result<()> {
    // Same crossterm/terminal setup as run_surface...
    let mut adapter = WatchEventAdapter::new();

    loop {
        // Drain all pending kernel events
        while let Ok(event) = event_rx.try_recv() {
            adapter.handle_event(&event, &mut state.data);
        }

        terminal.draw(|frame| {
            let core = render_shell(
                frame, frame.area(),
                state.surface_name(), state.help_text(), &theme,
            );
            watch::render::render(frame, core, &state, &theme);
        })?;

        if event::poll(Duration::from_millis(50))? {  // faster poll
            if let Event::Key(key_event) = event::read()? {
                let action = KeyHandler::map(key_event);
                state.handle_key(action);
                if state.should_quit {
                    break;
                }
            }
        }
    }

    // Same teardown...
}
```

The watch command does NOT use the `Surface` trait or `TuiApp`. It creates a
`WatchState` and `WatchEventAdapter` directly and passes the kernel's
`mpsc::Receiver<EngineEvent>` to `run_watch`. This avoids the need for `TuiApp`
to implement `Surface` and keeps the kernel integration explicit.

### 3.3 Command Pattern

Every TUI-capable command follows this pattern:

```rust
use std::io::IsTerminal;

pub fn run(args: Args, global: &GlobalArgs) -> Result<()> {
    let data = gather_data(&args)?;

    if global.json {
        output::json::print(&data)?;
    } else if !global.no_tui && std::io::stdout().is_terminal() {
        tui::run_surface(SurfaceState::new(data))?;
    } else {
        output::plain::print(&data)?;
    }
    Ok(())
}
```

### 3.4 Watch Command (Kernel Integration)

```rust
pub fn run(args: Args, global: &GlobalArgs) -> Result<()> {
    let config = resolve_watch_config(&args)?;
    // Use a standard-library, blocking channel here because the kernel watch loop
    // runs on a dedicated thread and events are consumed synchronously by the CLI.
    // This keeps the implementation simple and avoids pulling in additional async
    // channel dependencies (e.g. `tokio::sync::mpsc`, `crossbeam-channel`) until
    // we have a concrete need for async-aware backpressure or multiplexing.
    let (tx, rx) = std::sync::mpsc::channel();

    let kernel_handle = std::thread::spawn(move || {
        anvil_kernel::watch(config, tx)
    });

    if !global.no_tui && std::io::stdout().is_terminal() {
        let state = WatchState::new(WatchData::default());
        tui::run_watch(state, rx)?;
    } else {
        for event in rx {
            output::plain::print_event(&event)?;
        }
    }

    kernel_handle.join().map_err(|_| anyhow::anyhow!("kernel thread panicked"))??;
    Ok(())
}
```

### 3.5 Migration Module Cleanup

At cutover, `crates/anvil-tui/src/migration.rs` is removed. The `TuiBackend`
enum and `select_backend()` function become dead code once the Node.js CLI is
archived — there is no Ink backend to select. The `TuiApp` struct in `app.rs`
is also removed; its responsibilities (terminal validation, event draining) are
absorbed by the CLI's `tui.rs` runner functions.

The `compat.rs` module (terminal size detection/validation) is retained.

---

## 4. Auth & API Client

### 4.1 Credential Storage

Location: `$XDG_CONFIG_HOME/anvil/` (defaults to `~/.config/anvil/`)

```
~/.config/anvil/
├── credentials.json   { "token": "...", "email": "...", "expires_at": "..." }
└── config.json        { "api_url": "https://api.anvil.eddacraft.ai" }
```

Same paths as the Node.js CLI — credentials are interchangeable during
transition. Malformed or expired credential files are handled gracefully: the
CLI prints a diagnostic and prompts re-authentication. File permission errors
are reported with the specific path and required permissions.

### 4.2 Device Code Flow

```
CLI                              API (/auth/device/)
 │                                │
 ├── POST /code ────────────────► │
 │◄──────────── device_code,      │
 │              user_code,        │
 │              verification_uri  │
 │                                │
 │  Print user_code               │
 │  Open browser                  │
 │                                │
 ├── POST /token (poll) ────────► │
 │◄──────────── pending / token   │
 │  (repeat until approved)       │
 │                                │
 │  Store token                   │
```

### 4.3 OTP Flow

```
CLI                              API (/auth/otp/)
 │                                │
 │  Prompt email                  │
 ├── POST /send ────────────────► │
 │◄──────────── ok                │
 │                                │
 │  Prompt 6-digit code           │
 ├── POST /verify ──────────────► │
 │◄──────────── token             │
 │                                │
 │  Store token                   │
```

### 4.4 AnvilClient

```rust
pub struct AnvilClient {
    http: reqwest::Client,
    api_url: String,
    token: Option<String>,
}

impl AnvilClient {
    pub fn new() -> Result<Self>;
    pub fn authenticated() -> Result<Self>;

    // Auth
    pub async fn request_device_code(&self) -> Result<DeviceCodeResponse>;
    pub async fn poll_device_token(&self, device_code: &str) -> Result<TokenResponse>;
    pub async fn send_otp(&self, email: &str) -> Result<()>;
    pub async fn verify_otp(&self, email: &str, code: &str) -> Result<TokenResponse>;

    // Admin
    pub async fn approve_user(&self, user_id: &str) -> Result<()>;
}
```

HTTP via `reqwest` with `rustls-tls`. Async only for API calls — auth commands
use `#[tokio::main]` at the command level, not globally.

---

## 5. Output Modes

Every command supports three output modes:

| Mode | When | Implementation |
|------|------|---------------|
| TUI | TTY + no `--no-tui` + no `--json` | `tui::run_surface()` |
| Plain | Non-TTY or `--no-tui` | `output::plain::*` |
| JSON | `--json` flag | `output::json::*` (serde_json) |

Plain text uses simple indented lists and tables (no external table crate).
JSON output serialises the same data structures the TUI surfaces consume.

---

## 6. Command Tier Classification

### Tier 1 — Initial Release

Core workflow, TUI surfaces, architecture, auth, policy.

| Command | Module | TUI Surface |
|---------|--------|-------------|
| `anvil init` | `commands/init.rs` | Init |
| `anvil watch` | `commands/watch.rs` | Watch |
| `anvil gate <plan>` | `commands/gate.rs` | Gate |
| `anvil status` | `commands/status.rs` | Status |
| `anvil doctor [--fix]` | `commands/doctor.rs` | Doctor |
| `anvil audit` | `commands/audit.rs` | Audit |
| `anvil tutorial [--reset]` | `commands/tutorial.rs` | Tutorial |
| `anvil start` | `commands/welcome.rs` | Welcome |
| `anvil new` | `commands/new.rs` | Browser |
| `anvil wizard` | `commands/wizard.rs` | Wizard |
| `anvil auth login` | `commands/auth.rs` | — |
| `anvil auth logout` | `commands/auth.rs` | — |
| `anvil auth whoami` | `commands/auth.rs` | — |
| `anvil admin approve` | `commands/admin.rs` | — |
| `anvil policy list` | `commands/policy.rs` | — |
| `anvil policy explain` | `commands/policy.rs` | — |
| `anvil policy diff` | `commands/policy.rs` | — |
| `anvil policy validate` | `commands/policy.rs` | — |
| `anvil policy test` | `commands/policy.rs` | — |
| `anvil architecture validate` | `commands/architecture.rs` | — |
| `anvil architecture watch` | `commands/architecture.rs` | — |
| `anvil hooks install` | `commands/hooks.rs` | — |
| `anvil hooks status` | `commands/hooks.rs` | — |
| `anvil export` | `commands/export.rs` | — |

### Tier 2 — Fast Follow

Utilities and operational commands.

| Command | Notes |
|---------|-------|
| `anvil check` | Run a single check |
| `anvil validate` | Validate config |
| `anvil drift` | Drift snapshot management |
| `anvil policy-debug` | Policy debugging |
| `anvil policy-watch` | Policy file watcher |
| `anvil pr-comment` | PR annotation |
| `anvil exception` | Exception management |
| `anvil gate-config` | Gate configuration |

### Tier 3 — Subsequent

Subsystem and specialised commands.

| Command | Notes |
|---------|-------|
| `anvil edda *` | Edda memory subsystem |
| `anvil ember *` | Ember proposal subsystem |
| `anvil stack *` | Stack integration |
| `anvil plan *` | APS plan management |
| `anvil agent *` | Agent governance |
| `anvil release` | Release management |
| `anvil beta` | Beta programme |
| `anvil authorship` | Authorship tracking |
| `anvil mcp-config` | MCP server configuration |
| `anvil explain` | Top-level explain |

Commands not listed in any tier are reviewed at Tier 2 scoping time and either
assigned to a tier or marked as deprecated.

---

## 7. Migration Plan

### 7.1 Build Sequence

1. Scaffold `crates/anvil-cli/`, `crates/anvil-policy/`, `crates/anvil-architecture/`
2. Define `Surface` trait in `crates/anvil-tui/src/surface.rs`, register in `lib.rs`
3. Implement `Surface` on all 10 surface states
4. Implement `tui.rs` runners (`run_surface` + `run_watch`), extracted from demo binary
5. Port Tier 1 commands in order: tutorial → status → doctor → gate → watch →
   audit → init → wizard → new → auth → admin → policy → architecture →
   hooks → export
6. Port auth flows (device code + OTP)
7. Port services (repo_scanner, template_generator, historical_analyser)

### 7.2 Testing

- Unit tests co-located in each module
- Integration tests in `crates/anvil-cli/tests/` against existing fixtures
- TUI snapshot tests via `insta` (already a workspace dependency)
- Auth flow tests with mock HTTP server
- Exit code tests for gate pass/fail scenarios

### 7.3 Cutover

1. Tag repository: `pre-rust-cli`
2. Create fork/branch for preservation if desired
3. Move `apps/anvil-cli/` to `archive/anvil-cli-node/`
4. Move `apps/anvil-cli/src/tui/` to `archive/anvil-tui-ink/`
5. Remove `crates/anvil-tui/src/migration.rs` and `crates/anvil-tui/src/app.rs`
6. Update workspace `package.json` to remove archived packages
7. Ship Rust binary via GitHub Releases
8. Update install documentation

### 7.4 Distribution

- **GitHub Releases:** pre-built binaries for x86_64/aarch64 Linux + macOS
- **Install script:** `curl -fsSL https://install.eddacraft.ai | sh`
- **Cargo:** `cargo install anvil-cli`
- **npm wrapper (optional):** `@eddacraft/anvil-cli` with postinstall download
- **CI workflow:** `.github/workflows/release.yml` with build matrix for 4
  targets using `cross` for aarch64 cross-compilation

---

## 8. Workspace Dependencies

New workspace-level dependencies to add to root `Cargo.toml`:

```toml
[workspace.dependencies]
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
tokio = { version = "1", features = ["rt", "macros"] }
anyhow = "1"
dirs = "5"
```

`anyhow` is net-new to the workspace. Convention: `anyhow::Result` for
application code (CLI commands, runners), `thiserror` for library code
(anvil-policy, anvil-architecture, anvil-kernel).

---

## 9. What This Spec Does Not Cover

- Tier 2 and Tier 3 command ports (separate specs per tier)
- MCP server migration (stays Node.js)
- Website/dashboard changes
- Windows support (Linux + macOS only for initial release)
- CI pipeline changes beyond the release workflow
- JSON output schema documentation (defined per command during implementation)
