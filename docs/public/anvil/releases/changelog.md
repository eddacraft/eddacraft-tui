---
id: changelog
title: Changelog
description: Release history for anvil.
sidebar_position: 1
---

# Changelog

All notable changes to anvil are documented here.

## [Unreleased]

### Changed (breaking)

- **`anvil doctor --json` output shape** — the root changed from a bare JSON
  array of check objects to `{ "checks": [...], "notifications": [...] }`.
  Consumers that iterated the array must switch to `data.checks`.

### Added

- **`anvil audit --json` notifications** — audit now includes a
  `notifications[]` field alongside `issues[]` and `next_steps[]`.
- **Shared notification envelope** — `check`, `gate`, `audit`, `doctor`, and TUI
  surfaces are converging on one notification taxonomy for machine consumers.

## [0.3.3-beta] — WinGet Distribution & Windows UX

### Added

- **WinGet distribution** — Windows users can now install and upgrade anvil via
  WinGet.
- **Scoop support** — Scoop bucket install guidance is now part of the supported
  Windows distribution surface.
- **Admin operator tooling** — the separate `anvil-admin` operator CLI now
  covers `list`, `show`, `approve`, `invite`, `audit`, `revoke`, and
  `send-migration` for beta access operations.
- **Nightly stress test workflow** — CI benchmark coverage expanded to catch
  native-engine regressions earlier.

### Fixed

- **Windows TUI input** — duplicate keypress handling on Windows was removed.
- **Discovery and tutorial UX** — onboarding and tutorial completion flows were
  stabilised after the Rust cutover.
- **Admin approval and migration flows** — reliability and error handling were
  tightened across the beta-access operator tooling.

### Improved

- **Release automation** — Windows signing groundwork, public-release promotion,
  and release-script hardening all landed as part of the `0.3.3-beta` cycle.

## [0.3.2-beta] — Update Command & Onboarding Completion

### Added

- **`anvil update`** — the native binary now ships with an in-place updater via
  the `anvil update` command.
- **Welcome/onboarding completion** — the first-run experience, tutorial, and
  welcome hub reached feature-complete beta coverage.
- **Interactive release workflow** — release automation gained a manifest-driven
  handoff for agent-assisted release verification.

### Fixed

- **Tutorial command drift** — tutorial steps were realigned with the shipped
  Rust CLI.
- **Install flow polish** — installer next steps and Homebrew publishing were
  made more reliable.

## [0.3.1-beta] — Docs Cutover & Onboarding Fixes

### Added

- **Docs domain cutover** — `docs.eddacraft.ai` now served via a dedicated proxy
  with shared-secret middleware and a Nordic terminal-themed landing page.

### Fixed

- **Welcome screen** — first-user onboarding flows restored after regressions in
  0.3.0-beta.
- **Auth error messages** — raw HTTP errors replaced with user-friendly messages
  in device-code and login flows.
- **TUI version display** — shell footer now shows the correct version string.
- **Release pipeline** — ARM64 Windows target removed from cargo-dist (upstream
  dependency not yet available).

### Security

- **OAuth state hardening** — replay protection via issued-at timestamp with
  600-second expiry; nonce cookie cleared on all callback exit paths.
- **Docs proxy** — upstream auth redirect blocking and response header stripping
  for shared secrets.
- **CI credential scoping** — Azure credentials passed as composite action
  inputs instead of job-level environment variables.
- **Feature flag validation** — strict input validation on snapshot loading and
  explicit reason codes for unimplemented operators.

## [0.3.0-beta] — Rust CLI & Native Engine

### Changed

- **Native Rust binary** — anvil is now distributed as a single static binary
  with no Node.js runtime required. The Node.js package (`@eddacraft/anvil-cli`)
  is deprecated and will not receive further updates. See
  [The Switch to Rust](./rust-rewrite.md) for background on the Rust cutover and
  the small amount of path-cleanup guidance needed if you still have the old npm
  CLI installed.
- **Installation** — `curl -fsSL https://install.eddacraft.ai | sh` (macOS /
  Linux) or `irm https://install.eddacraft.ai/windows | iex` (Windows). Also
  available via Homebrew: `brew install eddacraft/tap/anvil`. Built-in
  self-updater via `anvil update`.
- **Platform support** — builds for x86_64 and aarch64 on macOS, Linux, and
  Windows (6 targets via cargo-dist).
- **Ratatui TUI** — all 10 interactive surfaces (welcome, tutorial, watch,
  wizard, status, doctor, init, audit, browser, gate) rebuilt using Ratatui with
  the eddacraft Terminal Standard design system.
- **Structured exit codes** — `0` (pass), `1` (general error), `2` (gate
  failure), `3` (auth required), `4` (config error).
- **Docs gating** — the `/anvil` documentation is now gated behind GitHub OAuth
  for beta users. Public docs (APS, Kindling, edda-stack) remain open.

### Added

- **Welcome screen & onboarding** — first-run detection with an interactive
  onboarding experience covering tutorial paths, live watch demo, and hook
  installer guidance. `anvil welcome` anytime.
- **Kernel engine** — native core engine with file watching, incremental
  tree-sitter parsing, and a real-time semantic dependency graph (petgraph).
  Supports foreground watch, embedded one-shot checks, and a dual-run harness
  for engine comparison.
- **Rust check ports** — secret scan, anti-pattern detection, command safety,
  architecture parity tests, and benchmarks all ported to Rust.
- **Beta authentication** — passwordless device-code and OTP flows with secure
  credential storage, session refresh with theft detection, and
  `anvil auth login` / `anvil admin approve` commands.
- **New commands** — `anvil new` (template browser), `anvil wizard` (interactive
  setup), `anvil audit` (security findings scan), `anvil drift` (snapshot,
  compare, report, list), `anvil validate` (APS plan validation), and
  `anvil gate-config` (gate thresholds). `--json` output mode across all
  commands.
- **MCP config generation** — library functions for generating MCP server
  configuration for Claude Code, Cursor, Windsurf, and VS Code.
- **Kernel benchmarks** — criterion benchmarks and a stress test harness for
  watcher, parser, graph, and policy evaluation, wired into CI.

### Performance

- 5–10x faster scanning on typical projects.
- 80% less memory in watch mode (~30–50MB vs ~400MB for a 5,000-file monorepo).
- Cold start under 10ms (vs 200–400ms with Node.js).
- Tree-sitter parse throughput ~15,000 files/second.
- Parallel file walks via rayon; watch mode excludes ignored directories at the
  OS level.

### Security

- Device-code and OTP authentication hardened with theft detection on session
  refresh.
- Atomic credential file writes with restrictive permissions at creation time.
- Log inputs sanitised to prevent log injection.
- GitHub Action expression injection sanitised; all GitHub Actions pinned to
  commit SHAs.
- Dependency patches: fast-xml-parser (CVE-2026-33036), `@hono/node-server`
  (CVE-2026-39406), axios, picomatch, undici, yauzl, rustls-webpki, and others.

## [0.2.1-beta] - 2026-03-26

### Added

- **Project memory** — anvil now tracks patterns and decisions in your codebase
  via the Edda memory system and Ember proposal engine.
- **Security hardening** — input validation and subprocess execution
  improvements across the platform.
- **Dependency patches** — minimatch, axios, svgo, tar, and others.

## [0.2.0-beta] - 2026-03-14

### Added

- **Licence verification** — offline JWT verification replaces auth-check;
  background refresh, file store with resolution order, `whoami` licence details
- **Edda Stack memory system** — Ember candidate proposals, Edda canonical
  memories, observation-to-proposal mapping, provenance tracking, evolution
  service, and full integration contracts (`@eddacraft/edda-stack`)
- **Forge & Temper pipeline** — pre-commit code review via cross-model
  negotiation (Forge hook + reviewer agent), structured finding/response
  protocol with round cap, deferred finding auto-filing as GitHub issues, Temper
  self-healing GitHub Actions workflow with 2-cycle cap
- **Rust kernel spike** — Phase 0 validated: tree-sitter parsing, notify-rs
  watcher, petgraph dependency graph, Cargo workspace with CI
- **Rust check ports** — secret scan, anti-pattern detection, and command safety
  ported to Rust for 10-40x speedup
- **Ratatui TUI** — shared component library for native terminal UI, 10 Ink
  components ported to Ratatui
- **Security CI pipeline** — Semgrep SAST, dependency audit, TruffleHog secret
  scan, licence compliance, OSSF Scorecard on every PR
- **Interactive release command** — `anvil release` for guided CLI releases
- **APS nested index loading** — depth-limited recursive plan loading
- **Tutorial continuation** — continue to another tutorial path from completion
- **JSON output** — `--json` flag on `hooks status` and `plan create`

### Changed

- Monorepo CI split: lightweight dev-branch checks, full release gate on PRs
  targeting `main` (cross-platform macOS + Windows)
- Concurrency groups on all CI workflows to prevent redundant runs
- BMAD adapter supports v6 YAML document format
- APS state uses atomic locking with `O_EXCL` lock files

### Fixed

- 292 bug fixes across CLI, API, APS state management, adapters, security hooks,
  and documentation
- Pulumi provider download failures from GitHub API rate limits (GITHUB_TOKEN
  now passed to plugin downloader)
- Watch mode signal handler reliability
- CLI error handling and edge case robustness

### Security

- Automated security scanning on every PR (Semgrep, dependency audit,
  TruffleHog, licence compliance)
- Pre-commit security hook hardened (subshell variable loss, heredoc injection)
- OSSF Scorecard pinned to v2.4.3

## [0.1.2-beta] - 2026-02-22

### Fixed

- CLI error handling improvements
- Watch mode signal handler reliability

## [0.1.1] - 2026-02-21

### Added

- Interactive tutorial system
- Doctor diagnostics command
- Explain command for rule details
- Watch mode profiles (source, plans, all)

### Fixed

- Init wizard project detection improvements

## [0.1.0] - 2026-02-21

### Added

- Core validation engine with architecture boundary checks
- Anti-pattern detection (AP-001 through AP-007)
- Secret detection with pattern matching and entropy analysis
- Watch mode for real-time validation
- GitHub Action for CI integration
- VS Code extension for in-editor diagnostics
- TUI (terminal UI) for interactive commands
- MCP server configuration generation
- OPA/Rego custom policy support
- Drift detection with snapshot comparison
- Evidence system for audit trails
- Beta access and authentication system

### Security

- All evidence is cryptographically signed
- Secrets never appear in logs or output

---

## Versioning

anvil follows [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking changes to config or CLI
- **MINOR** — new features, backward compatible
- **PATCH** — bug fixes

## Upgrading

See [Upgrade Notes](/anvil/releases/upgrade-notes) for migration guides.

---

**See also:** [Upgrade notes](/anvil/releases/upgrade-notes),
[The Switch to Rust](/anvil/releases/rust-rewrite)
