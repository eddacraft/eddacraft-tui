---
id: changelog
title: Current release notes
description:
  Current anvil beta changes and links to the complete release archive.
---

# Current release notes

This page summarises user-visible beta changes. Internal work items, source
paths, and implementation notes are deliberately excluded. For the full
version-by-version history and downloadable artefacts, use the
[GitHub release archive](https://github.com/eddacraft/anvil/releases).

## Unreleased — next beta after 0.9.0-beta

Draft summary of customer-visible work already on the maintained branch. The
version, date, and final scope are fixed when the next tag ships.

### Added

- Bare `anvil` (no subcommand) is the daily ensure surface after activation —
  daemon, worktree attestation, and already-owned MCP refresh without reinstall
  prompts. Use `anvil start` for first-time setup and reconfigure.
- `anvil mcp install --client` configures twelve AI clients, with `--verify` and
  `--dry-run`. Interactive `anvil start` offers every supported client in the
  consent list (unticked until you select one).
- The MCP stdio server supports ratified MCP `2026-07-28` discovery and keeps
  all four supported initialise-era versions. Client configuration shapes are
  unchanged; modern and legacy stdio flows are regression-tested.
- `anvil skill install` ships the managed `anvil-developer-functions` skill;
  `anvil doctor` reports managed-skill freshness.
- Package-manager-aware `anvil update` for Homebrew, Scoop, and WinGet installs.
- `anvil lsp --stdio` exposes advisory graph context over Language Server
  Protocol.
- Anti-pattern scanning honours `linguist-generated` markers and
  `antipattern.exclude` globs for generated trees.
- Fragile-presentation check for UI content hidden with `opacity: 0` and gated
  only on entrance animation.

### Changed

- On a real terminal, `anvil start` opens the consent-first activation surface
  by default; use `--no-tui` or `ANVIL_NO_TUI=1` for plain text.
- Warning-severity anti-pattern findings no longer fail `anvil gate` unless you
  opt in with `--fail-on-warnings` or `ANVIL_FAIL_ON_WARNINGS`. Broken ciphers /
  ECB and JWT `none` remain blocking errors.

## 0.9.0-beta — 12 July 2026

### Added

- `anvil welcome` provides an account-free first result on a user's own project.
- `anvil start` gives a quieter repeat-activation result and explicit protection
  state.
- `anvil insights --share` creates a reviewable local scorecard.
- Python joins the primary analysed language set.
- Read-only graph context is available to supported AI clients after explicit
  configuration.
- Infrastructure checks cover selected workflow, shell, container, and migration
  files.

### Changed

- Authentication-required action commands use a distinct exit code for safer
  automation.
- Interactive activation choices begin unselected; continuing without a choice
  writes nothing.
- Save-time activation manages the local daemon on supported platforms while
  read-only and non-interactive commands remain non-mutating.
- Graph persistence is enabled by default for faster warm starts and stores
  structural identity rather than source text.

### Fixed and hardened

- Secret scanning reduces low-value findings in common environment and lock
  files.
- Dashboard navigation and authentication output are more consistent.
- Workspace allowlists enforce only explicitly allowed roots.
- Credential, client configuration, and daemon state handling received
  additional platform hardening.

## Earlier beta releases

Earlier releases introduced the native Rust binary, local checks and gates,
architecture boundaries, save-time watching, Git hooks, machine-readable output,
and the public installer.

For artefact-level history and checksums, use
[GitHub Releases](https://github.com/eddacraft/anvil/releases).

## Upgrade

Read the [current upgrade guide](upgrade-notes.md) before changing a scripted or
team installation.
