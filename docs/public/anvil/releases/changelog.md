---
id: changelog
title: Changelog
description: Release history for anvil.
sidebar_position: 1
---

# Changelog

All notable changes to anvil are documented here.

## [Unreleased]

## [0.5.0] — 2026-05-01 — AI Guardrails & Mid-Edit Validation

### Added

- **Git config-mode hooks** — `anvil hooks install --config` and
  `anvil hooks uninstall --config` manage Git 2.54 native `hook.<event>.command`
  entries; file-mode hooks remain the default and Husky stays as the contributor
  bootstrap.
- **Hook setup visibility** — `anvil hooks status`, `anvil doctor`, onboarding,
  and tutorial now recognise config-mode hooks, `core.hooksPath`, third-party
  hook managers, and duplicate file/config execution risk.
- **AI guardrail profile** — `anvil gate --profile ai` runs the AI-focused check
  set with strict configuration handling and a stable JSON envelope for agent
  and MCP consumers.
- **AI-001 reasoning rule** — flags source comments that justify code with
  authority, social proof, or deflection instead of technical reasoning; honours
  `// @anvil-ignore AI-001` and emits at info severity.
- **`.env` and `.envrc` secret scanning** — `.env`, `.env.*`, and `.envrc` files
  are parsed as key/value files so leaked values report the variable name and
  source line (`SURFENV-001`).
- **`anvil mcp-config`** — Rust CLI command generates, verifies, and writes
  Claude Code, Cursor, Windsurf, and VS Code MCP server configuration with
  stdio/http transports, `--write`, `--verify`, workspace overrides, path-safety
  prompts, and atomic writes.
- **API SQL migration runner** — Anvil API deploys now have a first-party SQL
  migration runner with dry-run support and drift detection.

### Fixed

- **Doctor outside git repos** — missing git repositories now surface as a
  structured warning through the doctor JSON contract rather than failing the
  whole run.
- **First-run guidance** — init and onboarding copy now points unauthenticated
  users at `anvil auth login` where required, and inotify capacity warnings are
  clearer about what to change.
- **Release publishers** — Scoop and WinGet publishing paths now fail earlier on
  token or fork problems, and the cargo-dist installer is pinned by SHA256 in
  the release workflow.
- **API deploy stability** — CORS preflight caching, Vercel API routing, and the
  `svix`/`uuid` runtime override were tightened after post-release deploy
  failures.

### Improved

- **Scan performance and safety** — repository scans now use the shared parallel
  walk pattern across more CLI surfaces, skip oversized lines before regex
  evaluation, and cap first-run scan threads via `ANVIL_SCAN_THREADS` (default
  `min(num_cpus, 4)`).
- **AI workflow docs** — the AI guardrail profile, MCP and editor setup path,
  and beta tester guide now describe the current Rust CLI behaviour.
- **Git hook docs** — public docs now explain file-mode versus config-mode
  hooks, coexistence warnings, and the current decision to keep Husky as the
  contributor bootstrap.
- **Beta validation scenarios** — public tester scenarios were refreshed around
  the current onboarding, hooks, AI guardrail, and MCP flows.

## [0.4.0-beta] — 2026-04-25 — First-Run Polish & Native Scanner

### Changed (breaking)

- **`anvil watch --exclude` now uses glob patterns** — pass
  `--exclude 'vendor/**'` to skip a directory tree; bare directory names now
  warn so existing scripts surface the change instead of silently watching the
  wrong paths.
- **JSON output now carries notifications** — `anvil doctor --json` returns
  `{ "checks": [...], "notifications": [...], "schema_version": "2.0.0" }`, and
  `check`, `gate`, and `audit` include `notifications[]` alongside their
  existing payloads.

### Added

- **Native Rust scanner** — the Rust engine is now the authoritative scanner,
  with registry-backed rules, parallel scanning, rule provenance on findings,
  and fixture coverage for every shipped rule.
- **First-run scan after `anvil init`** — new projects get immediate findings,
  counts, and `file:line` pointers instead of being sent to another command.
- **Watch filtering** — `anvil watch --patterns` and `--exclude` now drive the
  watch loop, with a startup banner showing the active include/exclude scope.
- **`anvil check --artifact`** — scan generated files, build outputs, and other
  opaque artefacts outside the normal source-file filter.
- **`anvil licenses`** — prints bundled third-party attributions from
  `ACKNOWLEDGEMENTS.md`.
- **Scoop distribution** — Scoop joins WinGet and the existing installers, with
  README install instructions covering every supported package manager.
- **Per-operator admin keys** — admin operations can now use individually
  provisioned operator credentials instead of a single shared admin key.

### Fixed

- **`anvil watch` reliability** — partial setup failures no longer abort the
  loop, per-change panics are isolated, Ctrl-C exits cleanly, and error chains
  no longer leak the current working directory.
- **`anvil doctor` remediation** — checks now show concrete next actions, and
  `--fix` writes a valid default `.anvilrc` without running `git init` in unsafe
  directories.
- **`anvil init` robustness** — post-init git-history sampling now times out
  instead of hanging on slow filesystems or stalled remotes; tight inotify
  limits are reported up front with a fix hint.
- **Config-driven gate checks** — `.anvilrc` check selection now uses the same
  canonical names as the gate runner.
- **Non-interactive mode** — empty `ANVIL_NO_PROMPT` and `NONINTERACTIVE` values
  now correctly opt out of prompts.
- **Admin CLI and auth flows** — route coverage, timestamp validation, JSON
  output hygiene, EOF handling, and migration-send safety were tightened across
  the beta access surfaces.
- **Tutorial and TUI papercuts** — tutorial exit codes, `husky` handling, ASCII
  fallback, narrow-terminal titles, discovery scrolling, and `.anvilrc`
  detection races were corrected.

### Improved

- **Onboarding language** — init, welcome, tutorial, and watch now use the same
  defaults and explain scan truncation or watcher failures where the user can
  act on them.
- **Public docs** — release pages, install docs, the quality model, and the
  `.anvil` pattern reference were refreshed for the native scanner release.
- **Release preflight** — `scripts/release.sh` now runs Rust and TypeScript fmt,
  lint, typecheck, and tests as one bundled release gate.

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
