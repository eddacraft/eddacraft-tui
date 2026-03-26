---
id: changelog
title: Changelog
description: Release history for Anvil.
sidebar_position: 1
---

# Changelog

All notable changes to Anvil are documented here.

## [0.2.1-beta] - 2026-03-26

### Added

- **Project memory** — Anvil now tracks patterns and decisions in your codebase. New commands: `anvil edda`, `anvil ember`, `anvil stack`.
- **Security hardening** — input validation and subprocess execution improvements across the platform.
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
  watcher, petgraph dependency graph, Cargo workspace with CI (`crates/spike/`)
- **Rust check ports** — secret scan, anti-pattern detection, and command safety
  ported to Rust (`crates/anvil-checks/`, RENG-001–003, RENG-005)
- **Ratatui TUI** — shared component library (`crates/eddacraft-tui/`), 10 Ink
  components ported to Ratatui (PORT-001, PORT-002)
- **Security CI pipeline** — Semgrep SAST, dependency audit, TruffleHog secret
  scan, licence compliance, OSSF Scorecard on every PR
- **Interactive release command** — `anvil release` for guided CLI releases
- **APS nested index loading** — depth-limited recursive plan loading (CRB-011)
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

Anvil follows [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking changes to config or CLI
- **MINOR** — new features, backward compatible
- **PATCH** — bug fixes

## Upgrading

See [Upgrade Notes](/anvil/releases/upgrade-notes) for migration guides.
