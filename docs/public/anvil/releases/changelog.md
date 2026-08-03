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

The daily minute and assistant wiring. After setup, plain `anvil` is the
on-switch; `anvil start` opens consent-first activation on a real terminal by
default; install into the assistants you already use. Browser dashboard remains
flag-gated and is not a claim of this release.

### Added

- Plain `anvil` (no subcommand) is the daily on-switch after activation —
  healthy daemon, this worktree, and already-connected assistants, without
  reinstall prompts. Use `anvil start` for first-time setup and reconfigure.
- Optional anonymous usage telemetry (opt-out). Inspect with `anvil telemetry`;
  turn off with `anvil telemetry off`, `ANVIL_TELEMETRY=off`, or
  `DO_NOT_TRACK=1`. See [anonymous usage telemetry](../operations/telemetry.md).
- `anvil mcp install --client` configures twelve AI clients (Claude Code,
  Cursor, Codex, OpenCode, Gemini CLI, Antigravity, OpenClaw, VS Code, Copilot
  CLI, Grok, Warp, Zed), with `--verify` and `--dry-run`. Interactive start
  lists them with nothing ticked until you choose; scripts can use
  `--all-mcp-clients` / `ANVIL_ALL_MCP_CLIENTS`.
- Newer MCP protocol support without rewriting older client configs.
- `anvil skill install` ships the managed developer-functions skill;
  `anvil doctor` reports whether it is up to date or needs a reinstall.
- `anvil update` uses Homebrew, Scoop, or WinGet when that is how you installed,
  after explicit consent.
- `anvil lsp --stdio` exposes advisory graph context over Language Server
  Protocol for editors that prefer LSP.
- Anti-pattern scanning flags UI that is invisible until an entrance animation
  runs, and can skip generated trees via `linguist-generated` or
  `antipattern.exclude`.

### Changed

- On a real terminal, `anvil start` opens the consent-first activation screen by
  default; use `--no-tui` or `ANVIL_NO_TUI=1` for plain text.
- Warning-level anti-pattern findings no longer fail `anvil gate` unless you opt
  in with `--fail-on-warnings` or `ANVIL_FAIL_ON_WARNINGS`. Broken ciphers and
  JWT `none` still fail the gate.

### Fixed

- Remaining title-case product-name strings in user-facing copy are lowercase
  `anvil`.

## Unreleased — next beta after 0.9.1-beta

Draft of customer-visible work already on the maintained branch. Version and
date land when the next tag ships. Focus: keep assistants connected, and let you
prove protection from the start screen.

### Added

- On `anvil start`, press `t` for **Prove** — a real secret check on a sample,
  with an honest “can't prove here” when it cannot run. Never claims the
  editor's live save guard from that alone.

### Changed

- `anvil status` explains in one line when “warming” and a live assistant path
  would otherwise disagree with `anvil start`.
- Activation keyboard help is a single bar on consent and the result screen.

### Fixed

- Codex and similar assistants connect and call tools again after the last MCP
  update rejected normal progress metadata. Broken modern requests still fail
  clearly.

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
