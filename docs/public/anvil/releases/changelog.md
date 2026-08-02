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

## 0.9.1-beta — 2 August 2026 — Daily Path Polish and MCP 2.0 support

Bare `anvil` daily ensure, default activation TUI, multi-client MCP install with
dual-era protocol support, managed skills, and gate honesty. Browser dashboard
remains flag-gated and is not a claim of this release.

### Added

- **Bare `anvil` is the daily ensure surface.** After a project is activated,
  running `anvil` with no subcommand ensures the save-time daemon, re-attests
  the worktree, and refreshes already-owned MCP entries — without reinstalling
  clients, re-offering declined picks, or rewriting activation. Never-activated
  worktrees exit non-zero and point at `anvil start` or `anvil welcome`. Use
  `anvil start` for first-time setup and reconfigure; use bare `anvil` for the
  daily on-switch. `anvil --json` emits a compact ensure document.

- **MCP install for the harnesses you actually use.**
  `anvil mcp install --client` accepts twelve clients — Claude Code, Cursor,
  Codex, OpenCode, Gemini CLI, Antigravity, OpenClaw, VS Code, Copilot CLI,
  Grok, Warp, and Zed. Each installer writes that client's documented config
  shape, supports `--verify` / `--dry-run`, and keeps unmanaged third-party
  entries intact. Interactive `anvil start` offers every supported client in the
  consent list (unticked by default — nothing is written until you select one);
  pass `--mcp-client <id>` (repeatable) or `--all-mcp-clients` /
  `ANVIL_ALL_MCP_CLIENTS` for scripted multi-client install, or `--no-mcp` to
  skip configuration.

- **Ratified MCP `2026-07-28` alongside sealed legacy protocol fixtures.** The
  stdio server accepts modern discovery and per-request protocol metadata while
  retaining all four supported initialise-era versions. Client configuration
  shapes are unchanged; modern and legacy stdio flows are regression-tested.

- **Managed agent skills.** `anvil skill install` ships the bundled
  `anvil-developer-functions` skill into supported clients (global or project
  scope), with `--verify` and `--dry-run` for safe checks. `anvil doctor` now
  reports managed-skill freshness (fresh, stale, dirty, unmanaged, absent, or
  broken) so an upgrade can be reconciled with a reinstall rather than leaving a
  silent mismatch.

- **Package-manager-aware `anvil update`.** When anvil was installed through
  Homebrew, Scoop, or WinGet, `anvil update` offers that manager's allowlisted
  upgrade command after explicit consent (`-y` for non-interactive scripts)
  instead of downloading a sidecar binary that would fight the package database.
  Direct and other installs keep the existing signed-artefact path.

- **Graph capabilities over LSP.** `anvil lsp --stdio` exposes advisory graph
  context to editors that speak Language Server Protocol — position-aware symbol
  resolution, references, plus `anvil/impactOfChange` and `anvil/affectedTests`
  extensions — so tools that prefer LSP over MCP can use the same resident graph
  without a second protocol stack.

- **Fragile-presentation check (FRAG-001).** Anti-pattern scanning flags UI
  content authored invisible (`opacity: 0`) and gated only on an entrance
  animation — a pattern that fails hard when motion is reduced or the animation
  never runs. Enabled by default; same opt-out path as other anti-pattern rules.

- **Declare generated files so scans stay quiet.** Anti-pattern scanning now
  honours GitHub's `linguist-generated` marker in `.gitattributes` and
  workspace-relative `antipattern.exclude` globs in project config, so
  machine-generated trees the auto-detector does not recognise can be skipped
  without baselining individual findings. Secret detection and other gate
  engines still see those files.

### Changed

- **Activation TUI is the default interactive path.** On a real terminal,
  `anvil start` now opens the consent-first activation TUI without `--tui` or
  `ANVIL_ACTIVATION_TUI=1`. Read-only, `--watch`, `--json`, `--no-tui`,
  `ANVIL_NO_TUI`, CI, and non-TTY sessions stay plain. `--tui` and
  `ANVIL_ACTIVATION_TUI` remain accepted no-ops so old scripts do not break. The
  first run that writes a protecting baseline also shows a once-per-project
  celebration banner above the honest status headline; healthy repeat runs stay
  quiet.

- **Warnings no longer block by default; must-block crypto still does.** The
  anti-pattern leg of `anvil gate` restores warnings-over-blocks:
  warning-severity findings no longer fail the gate unless you opt in with
  `--fail-on-warnings` or `ANVIL_FAIL_ON_WARNINGS`. Broken ciphers / ECB
  (WC-002) and JWT `none` (WC-003) are promoted to error severity so they block
  on their own merit. WC-001 (MD5/SHA-1) and the unsafe-rendering family stay
  warnings to avoid false blocks on legitimate non-crypto uses.

### Fixed

- **Brand casing in user-facing copy.** Remaining title-case product-name
  strings in insights, intercept-protected paths, and related surfaces are
  lowercase `anvil`, matching the product brand.

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
