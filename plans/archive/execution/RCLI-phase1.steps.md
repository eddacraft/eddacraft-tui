# RCLI Phase 1: Foundation (RCLI-001 through RCLI-004)

> **For agentic workers:** Use superpowers:subagent-driven-development (if
> subagents available) or superpowers:executing-plans to implement this plan.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold the Rust CLI crate structure, define the Surface trait,
implement TUI runners, and wire up the clap entry point — producing a compilable
`anvil` binary that can launch any of the 10 existing TUI surfaces.

**Architecture:** Three new crates (`anvil-cli`, `anvil-policy`,
`anvil-architecture`), a `Surface` trait in `anvil-tui`, and TUI runner
functions in the CLI crate that extract the demo binary's event loop into
reusable `run_surface()` and `run_watch()`.

**Tech Stack:** Rust 2024 edition, clap 4 (derive), ratatui 0.30, crossterm
0.29, anyhow 1, dirs 5

**Spec:** `plans/specs/2026-03-18-rust-cli-design.md`
**APS:** `plans/modules/rust-cli.aps.md`

---

## Task 1: RCLI-001 — Scaffold crates (parallel-safe)

**Files:**
- Modify: `Cargo.toml` (workspace root — add members + deps)
- Create: `crates/anvil-cli/Cargo.toml`
- Create: `crates/anvil-cli/src/main.rs`
- Create: `crates/anvil-policy/Cargo.toml`
- Create: `crates/anvil-policy/src/lib.rs`
- Create: `crates/anvil-architecture/Cargo.toml`
- Create: `crates/anvil-architecture/src/lib.rs`

- [ ] **Step 1: Add workspace dependencies to root Cargo.toml**

Add to `[workspace.dependencies]`:
```toml
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
anyhow = "1"
dirs = "5"
```

Add to `[workspace] members`:
```toml
"crates/anvil-cli",
"crates/anvil-policy",
"crates/anvil-architecture",
```

Note: `tokio` is already in workspace deps with `features = ["full"]`.

- [ ] **Step 2: Create `crates/anvil-policy/`**

`Cargo.toml`:
```toml
[package]
name = "anvil-policy"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Anvil policy configuration, evaluation, and bundle management"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

`src/lib.rs`:
```rust
//! Anvil policy configuration, evaluation, and bundle management.
```

- [ ] **Step 3: Create `crates/anvil-architecture/`**

`Cargo.toml`:
```toml
[package]
name = "anvil-architecture"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Anvil architecture definition, boundary enforcement, and validation"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

`src/lib.rs`:
```rust
//! Anvil architecture definition, boundary enforcement, and validation.
```

- [ ] **Step 4: Create `crates/anvil-cli/`**

`Cargo.toml`:
```toml
[package]
name = "anvil-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Anvil CLI — deterministic governance for probabilistic AI workflows"

[[bin]]
name = "anvil"
path = "src/main.rs"

[dependencies]
anvil-tui = { path = "../anvil-tui" }
anvil-kernel = { path = "../anvil-kernel" }
anvil-kernel-types = { path = "../anvil-kernel-types" }
anvil-policy = { path = "../anvil-policy" }
anvil-architecture = { path = "../anvil-architecture" }
eddacraft-tui = { path = "../eddacraft-tui" }
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

[lints]
workspace = true
```

`src/main.rs`:
```rust
fn main() {
    println!("anvil — not yet implemented");
}
```

- [ ] **Step 5: Verify workspace builds**

Run: `cargo check --workspace`
Expected: compiles with zero errors

Run: `cargo clippy --workspace --all-targets`
Expected: zero warnings from new crates

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/anvil-cli/ crates/anvil-policy/ crates/anvil-architecture/
git commit -m "feat(rcli): scaffold anvil-cli, anvil-policy, anvil-architecture crates (RCLI-001)"
```

---

## Task 2: RCLI-002 — Surface trait + theme fix (parallel-safe with Task 1)

**Files:**
- Create: `crates/anvil-tui/src/surface.rs`
- Modify: `crates/anvil-tui/src/lib.rs` (add `pub mod shell; pub mod surface;`)
- Create: `crates/anvil-tui/src/shell.rs`
- Modify: `crates/eddacraft-tui/src/theme/eddacraft.rs` (fix brand colours)
- Modify: `crates/eddacraft-tui/src/theme/traits.rs` (add `border()`)
- Modify: all 10 `crates/anvil-tui/src/surfaces/*/mod.rs` (add trait impls)
- Modify: all 10 `crates/anvil-tui/src/surfaces/*/render.rs` (strip title/help)

### Step group A: Fix theme colours

- [ ] **Step A1: Add `border()` to Theme trait**

In `crates/eddacraft-tui/src/theme/traits.rs`, add `fn border(&self) -> Color;`
to the `Theme` trait. Update `border_unfocused()` to use `self.border()` instead
of `self.muted()`.

- [ ] **Step A2: Update eddacraftTheme to brand colours**

Replace all colour constants in `crates/eddacraft-tui/src/theme/eddacraft.rs`
with the eddacraft Terminal Standard:

```rust
const VOID: Color = Color::Rgb(13, 13, 15);        // bg
const STRUCTURE: Color = Color::Rgb(42, 42, 46);    // border
const OFF_WHITE: Color = Color::Rgb(235, 235, 235); // fg
const GHOST_GREY: Color = Color::Rgb(133, 133, 138);// muted
const ANVIL_EMBER: Color = Color::Rgb(204, 85, 0);  // accent
const EDDA_GROWTH: Color = Color::Rgb(46, 139, 87); // success
const BRICK_RED: Color = Color::Rgb(201, 74, 74);   // error
const DULL_AMBER: Color = Color::Rgb(208, 140, 56); // warning
```

Implement `border()` returning `STRUCTURE`. Update test assertions for new
values. Add `border()` to the `theme_colours_are_distinct` test array.

- [ ] **Step A3: Verify theme compiles and tests pass**

Run: `cargo test -p eddacraft-tui`
Expected: all tests pass

### Step group B: Shell module

- [ ] **Step B1: Create `crates/anvil-tui/src/shell.rs`**

Render function with signature:
```rust
pub fn render_shell(
    frame: &mut Frame,
    area: Rect,
    surface_name: &str,
    help_text: &str,
    theme: &eddacraftTheme,
) -> Rect
```

Layout: Header (Length 9), Core (Min 10), Footer (Length 5). Returns core Rect.

Header: 7-line Anvil block logo in accent, line 4 appends "a n v i l" in bold
fg then "// {surface_name}" in muted. 1 line padding top and bottom.

Footer: left 80% shows help text (key tokens highlighted in accent), right 20%
shows `[ ■ ] e d d a c r a f t` watermark in muted/border with version line.

### Step group C: Surface trait + impls

- [ ] **Step C1: Create `crates/anvil-tui/src/surface.rs`**

```rust
use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::eddacraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

pub trait Surface {
    fn surface_name(&self) -> &'static str;
    fn help_text(&self) -> &'static str;
    fn handle_key(&mut self, action: Action);
    fn should_quit(&self) -> bool;
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &eddacraftTheme);
}
```

- [ ] **Step C2: Register modules in lib.rs**

Add `pub mod shell;` and `pub mod surface;` to
`crates/anvil-tui/src/lib.rs`.

- [ ] **Step C3: Add `help_text()` and `surface_name()` to all 10 surfaces**

Each surface state gets two new methods. `help_text` is context-sensitive
(varies by phase/mode/step). `surface_name` returns spaced-out lowercase
(e.g. `"t u t o r i a l"`).

Reference the design spec §3.1 for exact help text strings per surface and
phase.

- [ ] **Step C4: Implement `Surface` trait on all 10 states**

Each impl delegates to existing methods. Example for `TutorialState`:

```rust
impl Surface for TutorialState {
    fn surface_name(&self) -> &'static str { self.surface_name() }
    fn help_text(&self) -> &'static str { self.help_text() }
    fn handle_key(&mut self, action: Action) { self.handle_key(action) }
    fn should_quit(&self) -> bool { self.should_quit }
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &eddacraftTheme) {
        render::render(frame, area, self, theme);
    }
}
```

- [ ] **Step C5: Strip `render_title`/`render_help` from all surface renders**

Remove title and help `Constraint::Length` chunks from the layout in each
`render.rs`. Remove `render_title` and `render_help` helper functions. Surfaces
now use the full area for content only. The shell handles chrome.

- [ ] **Step C6: Verify everything compiles and tests pass**

Run: `cargo test -p eddacraft-anvil-tui -p eddacraft-tui`
Expected: all tests pass

Run: `cargo clippy -p eddacraft-anvil-tui -p eddacraft-tui --all-targets`
Expected: zero new warnings

- [ ] **Step C7: Commit**

```bash
git add crates/eddacraft-tui/ crates/anvil-tui/
git commit -m "feat(tui): Surface trait, shell chrome, brand colours (RCLI-002)"
```

---

## Task 3: RCLI-003 — TUI runners

**Files:**
- Create: `crates/anvil-cli/src/tui.rs`
- Modify: `crates/anvil-cli/src/main.rs` (add `mod tui;`)

**Depends on:** Task 1 (crate exists), Task 2 (Surface trait exists)

- [ ] **Step 1: Create `crates/anvil-cli/src/tui.rs`**

Two public functions:

`run_surface<S: Surface>(state: S) -> anyhow::Result<()>` — crossterm raw
mode, alternate screen, ratatui terminal, event loop with `render_shell` +
surface render + keyboard dispatch. 100ms poll. Teardown on quit.

`run_watch(state: WatchState, event_rx: mpsc::Receiver<EngineEvent>) ->
anyhow::Result<()>` — same setup but drains `event_rx` via
`WatchEventAdapter::handle_event` each loop iteration. 50ms poll for
responsiveness.

Both functions handle terminal cleanup in all exit paths (including panics —
use a drop guard or explicit cleanup before `?` propagation).

- [ ] **Step 2: Wire module in main.rs**

Add `mod tui;` to `crates/anvil-cli/src/main.rs`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p eddacraft-anvil`
Expected: compiles (tui.rs is a module but not yet called from main)

- [ ] **Step 4: Commit**

```bash
git add crates/anvil-cli/src/
git commit -m "feat(cli): TUI runner functions — run_surface and run_watch (RCLI-003)"
```

---

## Task 4: RCLI-004 — clap entry point and global args

**Files:**
- Modify: `crates/anvil-cli/src/main.rs`
- Create: `crates/anvil-cli/src/commands/mod.rs`
- Create: `crates/anvil-cli/src/commands/tutorial.rs` (first real command)

**Depends on:** Task 3 (tui runners exist)

- [ ] **Step 1: Define GlobalArgs and exit codes**

In `main.rs`:
```rust
pub struct GlobalArgs {
    pub json: bool,
    pub no_tui: bool,
    pub verbose: bool,
}

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_GATE_FAIL: i32 = 2;
pub const EXIT_AUTH_REQUIRED: i32 = 3;
pub const EXIT_CONFIG_ERROR: i32 = 4;
```

- [ ] **Step 2: Define Cli and Commands enums with clap derive**

Full `Commands` enum with all 16 Tier 1 subcommands as per spec §2. Each
variant wraps a `commands::<name>::Args` struct. For now, all `Args` structs
are empty `#[derive(clap::Args)]` stubs.

- [ ] **Step 3: Create command stubs in `commands/mod.rs`**

One module per command group. Each module exports:
```rust
#[derive(clap::Args)]
pub struct Args {}

pub fn run(_args: Args, _global: &crate::GlobalArgs) -> anyhow::Result<()> {
    anyhow::bail!("not yet implemented")
}
```

Create stub files for: `tutorial.rs`, `status.rs`, `doctor.rs`, `welcome.rs`,
`audit.rs`, `init.rs`, `wizard.rs`, `new.rs`, `gate.rs`, `watch.rs`,
`auth.rs`, `admin.rs`, `policy.rs`, `architecture.rs`, `hooks.rs`,
`export.rs`.

- [ ] **Step 4: Wire dispatch in main()**

```rust
fn main() {
    let cli = Cli::parse();
    let global = GlobalArgs { ... };

    let result = match cli.command {
        Commands::Tutorial(args) => commands::tutorial::run(args, &global),
        Commands::Status(args) => commands::status::run(args, &global),
        // ... all 16
    };

    match result {
        Ok(()) => std::process::exit(EXIT_OK),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(EXIT_ERROR);
        }
    }
}
```

- [ ] **Step 5: Implement `tutorial` command as proof of integration**

In `commands/tutorial.rs`:
```rust
use anvil_tui::surfaces::tutorial::TutorialState;
use anvil_tui::surface::Surface;
use std::io::IsTerminal;

#[derive(clap::Args)]
pub struct Args {
    /// Reset tutorial progress
    #[arg(long)]
    reset: bool,
}

pub fn run(args: Args, global: &crate::GlobalArgs) -> anyhow::Result<()> {
    let state = TutorialState::new();

    if !global.no_tui && std::io::stdout().is_terminal() {
        crate::tui::run_surface(state)?;
    } else {
        println!("Tutorial requires an interactive terminal. Use without --no-tui.");
    }

    Ok(())
}
```

- [ ] **Step 6: Verify the full binary works**

Run: `cargo build -p eddacraft-anvil`
Expected: produces `target/debug/anvil` binary

Run: `target/debug/anvil --help`
Expected: shows all 16 subcommands with descriptions

Run: `target/debug/anvil --version`
Expected: shows `anvil 0.1.0`

Run: `target/debug/anvil status`
Expected: exits with "error: not yet implemented"

- [ ] **Step 7: Verify clippy passes**

Run: `cargo clippy -p eddacraft-anvil --all-targets`
Expected: zero warnings

- [ ] **Step 8: Commit**

```bash
git add crates/anvil-cli/
git commit -m "feat(cli): clap entry point with all Tier 1 command stubs (RCLI-004)"
```

---

## Verification Gate

After all 4 tasks complete:

- [ ] `cargo check --workspace` passes
- [ ] `cargo clippy --workspace --all-targets` passes (zero new warnings)
- [ ] `cargo test --workspace` passes (all existing + new tests)
- [ ] `target/debug/anvil --help` lists all 16 commands
- [ ] `target/debug/anvil tutorial` launches the interactive tutorial TUI with
  shell chrome (header logo + footer watermark) — if running in a terminal

Once verified, update APS statuses:
- RCLI-001: Proposed → Done
- RCLI-002: Proposed → Done
- RCLI-003: Proposed → Done
- RCLI-004: Proposed → Done
