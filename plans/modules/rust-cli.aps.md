<!--
APS Module: Rust CLI
=========================
Replace Node.js CLI with Rust binary using clap + Ratatui.
Big bang migration — Tier 1 commands first.

Scopes: RCLI (main)
-->

# Rust CLI

| ID   | Owner | Status     |
| ---- | ----- | ---------- |
| RCLI | —     | In Progress |

## Purpose

Replace the Node.js Anvil CLI (`apps/anvil-cli/`, Commander.js + Ink) with a
Rust binary (`crates/anvil-cli/`, clap + Ratatui). Single binary, single
runtime, native kernel integration.

**Why:** The kernel, TUI surfaces, and policy engine are all Rust. The Node.js
CLI is a thin shell around Rust work, requiring IPC, binary distribution hacks,
and two runtimes. A Rust CLI eliminates this overhead and provides same-process
`mpsc` for watch/gate event streaming.

**ADR:** [012-rust-cli-replacement](../decisions/012-rust-cli-replacement.md)
**Spec:** [2026-03-18-rust-cli-design](../specs/2026-03-18-rust-cli-design.md)

## In Scope

- Binary crate `crates/anvil-cli/` with `anvil` binary
- Library crate `crates/anvil-policy/` (policy config, evaluation, bundles)
- Library crate `crates/anvil-architecture/` (definitions, boundaries, validation)
- `Surface` trait formalising the TUI contract
- TUI runner functions (`run_surface`, `run_watch`)
- Auth flows (device code + OTP via reqwest)
- Three output modes (TUI, plain text, JSON)
- Tier 1 commands: core workflow, TUI surfaces, architecture, auth, policy
- Archival of Node.js CLI and Ink TUI

## Out of Scope

- Tier 2 commands (utilities — separate module)
- Tier 3 commands (subsystems — separate module)
- MCP server migration (stays Node.js)
- Website/dashboard changes
- Windows support (Linux + macOS only)

## Interfaces

**Depends on:**

- KERN — kernel watcher, parser, graph (event emission for watch/gate)
- RATS — Ratatui TUI surfaces + shell chrome
- PORT — Ink-to-Ratatui surface ports (all 10 surfaces must be ready)
- `anvil-kernel-types` — EngineEvent protocol
- `apps/anvil-api/` — Hono API (auth endpoints, admin endpoints)

**Exposes:**

- `anvil` binary — single entry point for all CLI commands
- `crates/anvil-policy/` — reusable policy evaluation library
- `crates/anvil-architecture/` — reusable architecture analysis library

## Constraints

- Big bang migration — no hybrid period with two CLIs
- Credentials interchangeable with Node.js CLI during transition
- Exit codes: 0 success, 1 error, 2 gate fail, 3 auth required, 4 config error
- `anyhow` for application code, `thiserror` for library crates
- TTY detection via `std::io::IsTerminal` (not `atty`)
- Async only for HTTP (auth commands); TUI and kernel stay synchronous
- Node.js CLI archived to `archive/`, not deleted

## Ready Checklist

Change status to **Ready** when:

- [x] Design spec approved (ADR-012)
- [x] All 10 TUI surfaces have `help_text()` and `surface_name()` methods
- [x] Shared shell chrome implemented (header + footer)
- [x] EddaCraft design system colours applied
- [x] `Surface` trait defined and implemented on all 10 states
- [x] CLI binary builds and all 16 commands wired up

---

## Phase 1 — Foundation

### RCLI-001: Scaffold crates

- **Status:** Proposed
- **Intent:** Create `crates/anvil-cli/`, `crates/anvil-policy/`,
  `crates/anvil-architecture/` with Cargo.toml, workspace registration, and
  stub `lib.rs`/`main.rs` files
- **Expected Outcome:** All three crates compile, workspace builds cleanly,
  clippy passes
- **Validation:** `cargo check --workspace` passes with new crates
- **Files:** `Cargo.toml`, `crates/anvil-cli/Cargo.toml`,
  `crates/anvil-policy/Cargo.toml`, `crates/anvil-architecture/Cargo.toml`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### RCLI-002: Surface trait

- **Status:** Proposed
- **Intent:** Define the `Surface` trait in `crates/anvil-tui/src/surface.rs`
  and implement it on all 10 existing surface states. The trait formalises
  `surface_name`, `help_text`, `handle_key`, `should_quit`, and `render`
- **Expected Outcome:** All surfaces implement `Surface`; existing tests still
  pass
- **Validation:** `cargo test -p anvil-tui` passes; trait is importable from
  `anvil_tui::surface::Surface`
- **Files:** `crates/anvil-tui/src/surface.rs`, `crates/anvil-tui/src/lib.rs`,
  all 10 `surfaces/*/mod.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RCLI-001

---

### RCLI-003: TUI runners

- **Status:** Proposed
- **Intent:** Implement `run_surface()` and `run_watch()` in
  `crates/anvil-cli/src/tui.rs`. Extracted from the demo binary event loop.
  Includes crossterm setup/teardown, shell chrome rendering, and keyboard
  dispatch
- **Expected Outcome:** Any `Surface` impl can be launched with
  `tui::run_surface(state)`. Watch surfaces use `tui::run_watch(state, rx)`
  with kernel event draining
- **Validation:** Demo binary refactored to use `run_surface`; compiles and
  runs
- **Files:** `crates/anvil-cli/src/tui.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RCLI-001, RCLI-002

---

### RCLI-004: clap entry point and global args

- **Status:** Proposed
- **Intent:** Implement `main.rs` with clap derive API, `Commands` enum,
  `GlobalArgs` struct (`--json`, `--no-tui`, `--verbose`), exit code constants,
  and dispatch skeleton
- **Expected Outcome:** `anvil --help` works, all Tier 1 subcommands listed,
  dispatch stubs return `Ok(())`
- **Validation:** `anvil --help` shows all commands; `anvil --version` shows
  version
- **Files:** `crates/anvil-cli/src/main.rs`, `crates/anvil-cli/src/commands/mod.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RCLI-001

---

## Phase 2 — Static Surface Commands

Commands that launch TUI surfaces without kernel integration.

### RCLI-005: tutorial command

- **Status:** Proposed
- **Intent:** Port `anvil tutorial [--reset]`. Manages progress file at
  `~/.anvil/tutorial-progress.json`. Launches Tutorial surface via
  `run_surface`
- **Expected Outcome:** `anvil tutorial` launches interactive tutorial with
  4 paths; `--reset` clears progress
- **Validation:** Tutorial starts, paths selectable, progress persists across
  runs
- **Files:** `crates/anvil-cli/src/commands/tutorial.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RCLI-003

---

### RCLI-006: status command

- **Status:** Proposed
- **Intent:** Port `anvil status`. Gathers hook status, profile info, recent
  gate runs from `.anvil/` directory. Supports TUI, plain, and JSON output
- **Expected Outcome:** `anvil status` shows project health in all three modes
- **Validation:** Output matches Node.js CLI output for same project state
- **Files:** `crates/anvil-cli/src/commands/status.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-003

---

### RCLI-007: doctor command

- **Status:** Proposed
- **Intent:** Port `anvil doctor [--fix]`. Runs diagnostic checks (Node.js
  version, Rust toolchain, config file, hooks, etc.). Supports auto-fix for
  fixable issues
- **Expected Outcome:** `anvil doctor` runs all diagnostics; `--fix` applies
  auto-fixes
- **Validation:** Check results match Node.js CLI for same environment
- **Files:** `crates/anvil-cli/src/commands/doctor.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-003

---

### RCLI-008: welcome command

- **Status:** Proposed
- **Intent:** Port `anvil start`. First-run quick-start menu. Launches Welcome
  surface
- **Expected Outcome:** `anvil start` shows welcome screen with menu options
- **Validation:** Menu navigable, options launch correct subcommands
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RCLI-003

---

### RCLI-009: audit command

- **Status:** Proposed
- **Intent:** Port `anvil audit`. Runs repo scanner, displays results. Supports
  TUI, plain, and JSON output
- **Expected Outcome:** `anvil audit` scans repository and presents findings
- **Validation:** Issue counts and severities match Node.js CLI
- **Files:** `crates/anvil-cli/src/commands/audit.rs`,
  `crates/anvil-cli/src/services/repo_scanner.rs`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RCLI-003

---

### RCLI-010: init command

- **Status:** Proposed
- **Intent:** Port `anvil init`. Multi-step wizard for project initialisation.
  Includes mode selection, format, directory, checks, and summary. Supports
  post-init analysis
- **Expected Outcome:** `anvil init` creates `.anvil.yaml` with guided
  configuration
- **Validation:** Generated config is valid; project analysis runs on completion
- **Files:** `crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-cli/src/services/template_generator.rs`
- **Confidence:** medium (largest command port — 799 LOC in Node.js)
- **Priority:** High
- **Dependencies:** RCLI-003

---

### RCLI-011: wizard command

- **Status:** Proposed
- **Intent:** Port APS onboarding wizard. Template selection, project name,
  configuration toggles, scaffold generation
- **Expected Outcome:** `anvil wizard` walks through APS project setup
- **Validation:** Wizard completes, generated project structure is correct
- **Files:** `crates/anvil-cli/src/commands/wizard.rs`
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** RCLI-003

---

### RCLI-012: new command (template browser)

- **Status:** Proposed
- **Intent:** Port `anvil new`. Template catalogue browser with category
  navigation, search, and variable input
- **Expected Outcome:** `anvil new` shows browsable template catalogue
- **Validation:** Templates browsable, search works, variables configurable
- **Files:** `crates/anvil-cli/src/commands/new.rs`
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** RCLI-003

---

## Phase 3 — Kernel-Integrated Commands

### RCLI-013: gate command

- **Status:** Proposed
- **Intent:** Port `anvil gate <plan>`. Runs gate checks via kernel, displays
  results in Gate surface. Exit code 2 on failure for CI integration
- **Expected Outcome:** `anvil gate plan.aps.md` runs checks and shows
  interactive explorer; returns exit code 2 on failure
- **Validation:** Check results match Node.js CLI; exit codes correct for CI
- **Files:** `crates/anvil-cli/src/commands/gate.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-003, KERN (gate execution)

---

### RCLI-014: watch command

- **Status:** Proposed
- **Intent:** Port `anvil watch`. Spawns kernel watcher on background thread,
  feeds `EngineEvent`s to TUI via `mpsc`. Uses `run_watch` runner with event
  draining loop (50ms poll)
- **Expected Outcome:** `anvil watch` shows live dashboard with real-time file
  change detection and gate results
- **Validation:** File changes trigger re-evaluation; TUI updates in real time;
  plain text mode prints events as they arrive
- **Files:** `crates/anvil-cli/src/commands/watch.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-003, KERN (watcher + event emission)

---

## Phase 4 — Auth & API

### RCLI-015: auth commands

- **Status:** Proposed
- **Intent:** Port `anvil auth login`, `anvil auth logout`, `anvil auth whoami`.
  Device code flow and OTP flow via reqwest + rustls-tls. Credentials stored at
  `$XDG_CONFIG_HOME/anvil/credentials.json`
- **Expected Outcome:** Users can authenticate via device code or OTP; tokens
  persist across sessions
- **Validation:** Login succeeds against staging API; credentials interchangeable
  with Node.js CLI; expired token triggers re-auth prompt
- **Files:** `crates/anvil-cli/src/commands/auth.rs`,
  `crates/anvil-cli/src/auth/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-004

---

### RCLI-016: admin commands

- **Status:** Proposed
- **Intent:** Port `anvil admin approve`. Authenticated API call to approve
  waitlisted users
- **Expected Outcome:** `anvil admin approve <user-id>` approves user via API
- **Validation:** API call succeeds; unauthenticated call returns exit code 3
- **Files:** `crates/anvil-cli/src/commands/admin.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RCLI-015

---

## Phase 5 — Policy & Architecture

### RCLI-017: anvil-policy crate

- **Status:** Proposed
- **Intent:** Port policy domain logic from Node.js services into
  `crates/anvil-policy/`. Covers config loading, starter profiles, built-in
  catalogue, evaluation engine, bundle management, exceptions
- **Expected Outcome:** Policy evaluation runs in pure Rust with same results
  as TypeScript implementation
- **Validation:** Policy test fixtures from Node.js pass identically in Rust
- **Files:** `crates/anvil-policy/src/`
- **Confidence:** medium (701 LOC in Node.js policy-config service)
- **Priority:** High
- **Dependencies:** RCLI-001

---

### RCLI-018: policy commands

- **Status:** Proposed
- **Intent:** Port `anvil policy` subcommands: list, explain, diff, validate,
  test. Uses `anvil-policy` crate for domain logic
- **Expected Outcome:** All policy subcommands work with same output as Node.js
- **Validation:** Output parity for same policy fixtures
- **Files:** `crates/anvil-cli/src/commands/policy.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-017

---

### RCLI-019: anvil-architecture crate

- **Status:** Proposed
- **Intent:** Port architecture domain logic from Node.js services into
  `crates/anvil-architecture/`. Covers definition schema, module boundaries,
  import rules, file rules, validation, config diagnostics
- **Expected Outcome:** Architecture validation runs in pure Rust
- **Validation:** Architecture test fixtures pass identically
- **Files:** `crates/anvil-architecture/src/`
- **Confidence:** medium (1,040 LOC in Node.js architecture-service)
- **Priority:** High
- **Dependencies:** RCLI-001

---

### RCLI-020: architecture commands

- **Status:** Proposed
- **Intent:** Port `anvil architecture validate` and
  `anvil architecture watch`. Uses `anvil-architecture` crate
- **Expected Outcome:** Architecture validation works with same output
- **Validation:** Validation results match Node.js for same architecture config
- **Files:** `crates/anvil-cli/src/commands/architecture.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-019

---

## Phase 6 — Utilities & Cutover

### RCLI-021: hooks and export commands

- **Status:** Proposed
- **Intent:** Port `anvil hooks install/status` and `anvil export`. Filesystem
  operations for hook installation, constraint export in multiple formats
- **Expected Outcome:** Hook installation and constraint export work identically
- **Validation:** Installed hooks match Node.js CLI output; exported constraints
  are identical
- **Files:** `crates/anvil-cli/src/commands/hooks.rs`,
  `crates/anvil-cli/src/commands/export.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** RCLI-004

---

### RCLI-022: output formatters

- **Status:** Proposed
- **Intent:** Implement `output::plain` and `output::json` modules. Plain text
  uses indented lists and tables. JSON serialises the same structures surfaces
  consume
- **Expected Outcome:** All commands support `--json` and `--no-tui` flags with
  consistent formatting
- **Validation:** JSON output is valid and parseable; plain output is readable
  in CI logs
- **Files:** `crates/anvil-cli/src/output/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RCLI-004

---

### RCLI-023: migration cleanup and archival

- **Status:** Proposed
- **Intent:** Remove `TuiBackend::Ink`, `migration.rs`, and `app.rs` from
  `anvil-tui`. Archive `apps/anvil-cli/` to `archive/anvil-cli-node/` and
  `apps/anvil-cli/src/tui/` to `archive/anvil-tui-ink/`. Tag repo
  `pre-rust-cli` before archival
- **Expected Outcome:** Node.js CLI archived, Rust CLI is sole `anvil` binary,
  workspace builds cleanly
- **Validation:** `cargo build --workspace` passes; `pnpm build` passes without
  archived packages; `anvil --help` works
- **Files:** `archive/`, `crates/anvil-tui/src/`, workspace configs
- **Confidence:** high
- **Priority:** High
- **Dependencies:** All RCLI-001 through RCLI-022

---

### RCLI-024: distribution pipeline

- **Status:** Proposed
- **Intent:** Create GitHub Actions release workflow that builds pre-built
  binaries for x86_64/aarch64 Linux + macOS. Install script at
  `https://install.eddacraft.ai`. Publish to crates.io as `anvil-cli`
- **Expected Outcome:** Tagged releases produce downloadable binaries; install
  script works on supported platforms
- **Validation:** Install script downloads and runs `anvil --version`
  successfully on Linux and macOS
- **Files:** `.github/workflows/release.yml`, install script
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RCLI-023

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Architecture service port complexity (1,040 LOC) | Medium | Medium | Port incrementally; validate against existing fixtures |
| Init wizard port complexity (799 LOC) | Medium | Medium | Extract reusable analysis logic into services |
| Auth flow edge cases (token refresh, network errors) | Low | Medium | Test against staging API; graceful error messages |
| Binary size (Rust + reqwest + ratatui) | Low | Low | Release builds with LTO; strip symbols |
| Missing Node.js command parity | Medium | High | Full tier classification in spec; explicit "not ported" list |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Foundation | 4 | Complete |
| 2 — Static Surface Commands | 8 | Complete |
| 3 — Kernel-Integrated Commands | 2 | Complete |
| 4 — Auth & API | 2 | Complete |
| 5 — Policy & Architecture | 4 | Complete |
| 6 — Utilities & Cutover | 4 | In Progress |
| **Total** | **24** | — |
