# Anvil Update Command Implementation Plan

**Goal:** Add `anvil update` subcommand with hybrid sidecar/library self-update
strategy.
**Architecture:** New `update.rs` command module using a resolution chain
(Homebrew detect → sidecar shell-out → axoupdater library fallback). Wired into
`main.rs` as an auth-bypass command.
**Tech Stack:** Rust, clap, axoupdater crate, std::process::Command

---

## File Map

| File | Responsibility |
|------|----------------|
| `crates/anvil-cli/src/commands/update.rs` | Create: update subcommand — args, resolution chain, sidecar/library paths, output |
| `crates/anvil-cli/src/commands/mod.rs` | Modify: add `pub mod update;` |
| `crates/anvil-cli/src/main.rs` | Modify: add `Update` variant, match arm, auth-bypass |
| `crates/anvil-cli/Cargo.toml` | Modify: add `axoupdater` dependency |

---

## Task 1: Add axoupdater dependency

**Files:**
- Modify: `crates/anvil-cli/Cargo.toml`

- [ ] Add `axoupdater` to `[dependencies]` with `github_releases` feature
- [ ] Run `cargo check -p eddacraft-anvil` — verify it compiles
- [ ] Commit: `chore(cli): add axoupdater dependency for self-update`

---

## Task 2: Scaffold update command with args and wiring

**Files:**
- Create: `crates/anvil-cli/src/commands/update.rs`
- Modify: `crates/anvil-cli/src/commands/mod.rs`
- Modify: `crates/anvil-cli/src/main.rs`

- [ ] Create `update.rs` with `UpdateArgs` struct (clap):
  - `--check`: bool — check only, don't install
  - `--version <VER>`: Option<String> — pin to specific version
  - `--force`: bool — reinstall even if on latest
- [ ] Add `pub fn run(args: &UpdateArgs, global: &GlobalArgs) -> anyhow::Result<()>`
  with a placeholder that prints current version
- [ ] Add `pub mod update;` to `mod.rs`
- [ ] Add `Update(commands::update::UpdateArgs)` to `Commands` enum in `main.rs`
  with doc comment `/// Update anvil to the latest version.`
- [ ] Add `Commands::Update(args) => commands::update::run(args, &cli.global)` to
  the match arm
- [ ] Add `Commands::Update(_)` to the auth-bypass branch in `requires_auth()`
- [ ] Run `cargo test -p eddacraft-anvil` — verify existing tests pass
- [ ] Add test `bypass_auth_update` in `main.rs` tests confirming auth bypass
- [ ] Run `cargo test -p eddacraft-anvil` — verify new test passes
- [ ] Commit: `feat(cli): scaffold anvil update subcommand`

---

## Task 3: Implement Homebrew detection

**Files:**
- Modify: `crates/anvil-cli/src/commands/update.rs`

- [ ] Write `fn is_homebrew_install() -> bool` that checks
  `std::env::current_exe()` path against Homebrew prefixes:
  `/opt/homebrew/`, `/usr/local/Cellar/`, `/home/linuxbrew/`
- [ ] Wire into `run()`: if Homebrew detected, print message and return Ok
- [ ] Add unit tests:
  - `homebrew_prefix_detected` — paths under known prefixes return true
  - `non_homebrew_prefix` — `/usr/local/bin/anvil`, `/home/user/.cargo/bin/anvil`
    return false
- [ ] Run `cargo test -p eddacraft-anvil` — verify tests pass
- [ ] Commit: `feat(cli): detect Homebrew install in anvil update`

---

## Task 4: Implement sidecar resolution and execution

**Files:**
- Modify: `crates/anvil-cli/src/commands/update.rs`

- [ ] Write `fn find_sidecar() -> Option<PathBuf>` that looks for
  `eddacraft-anvil-update` (or `eddacraft-anvil-update.exe` on Windows):
  1. Adjacent to `std::env::current_exe()`
  2. On `PATH` via `which::which` or manual PATH scan
- [ ] Write `fn run_sidecar(path: &Path, args: &UpdateArgs) -> anyhow::Result<()>`
  that shells out via `std::process::Command`, forwards `--check`/`--version`
  flags, streams stdout/stderr, and returns the sidecar's exit code
- [ ] Wire into `run()`: after Homebrew check, try sidecar before library
- [ ] Add unit tests:
  - `find_sidecar_returns_none_when_missing` — in a temp dir with no sidecar
  - `sidecar_args_check_flag` — verify Command is built with correct args
  - `sidecar_args_version_flag` — verify `--version X` is forwarded
- [ ] Run `cargo test -p eddacraft-anvil` — verify tests pass
- [ ] Commit: `feat(cli): sidecar resolution for anvil update`

---

## Task 5: Implement axoupdater library fallback

**Files:**
- Modify: `crates/anvil-cli/src/commands/update.rs`

- [ ] Write `fn run_library_update(args: &UpdateArgs, global: &GlobalArgs) -> anyhow::Result<()>`
  that:
  - Constructs `axoupdater::AxoUpdater` for the current binary
  - Queries latest release from GitHub
  - Compares against `env!("CARGO_PKG_VERSION")`
  - If `--check`: prints version info, returns Ok (exit 0 if up-to-date,
    bail with a marker if update available for exit code 1)
  - If `--force` or version differs: downloads and replaces
  - If `--version`: pins to specific release
- [ ] Wire into `run()` as the fallback after sidecar
- [ ] Handle the "both paths fail" case with manual instructions
- [ ] Add JSON output support: check `global.json` and emit structured output
  for check/update results
- [ ] Run `cargo check -p eddacraft-anvil` — verify it compiles
- [ ] Commit: `feat(cli): axoupdater library fallback for anvil update`

---

## Task 6: Add --check exit code handling

**Files:**
- Modify: `crates/anvil-cli/src/main.rs`
- Modify: `crates/anvil-cli/src/commands/update.rs`

- [ ] Define a custom error type or marker (`UpdateAvailable`) that `run()`
  returns when `--check` finds an update available
- [ ] In `main.rs`, handle `Commands::Update` similarly to `Commands::Gate` —
  catch the marker and exit with code 1 (update available) vs 0 (up-to-date)
- [ ] Add test in `main.rs` for the Update match arm pattern
- [ ] Run `cargo test -p eddacraft-anvil` — verify tests pass
- [ ] Commit: `feat(cli): exit code 1 for anvil update --check when outdated`

---

## Task 7: Integration smoke test

**Files:**
- Modify: `crates/anvil-cli/src/commands/update.rs`

- [ ] Add integration-style test: `anvil update --check` with
  `ANVIL_DEV=1` — verify it runs without panic and produces recognisable
  output (version string present)
- [ ] Run full `cargo test -p eddacraft-anvil`
- [ ] Run `cargo clippy -p eddacraft-anvil` — no warnings
- [ ] Commit: `test(cli): smoke test for anvil update --check`
