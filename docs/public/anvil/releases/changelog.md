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

## 0.9.2-beta — 3 August 2026 — MCP 2.0 reconnect

### Fixed

- **Codex and similar assistants work with anvil again.** After the last MCP
  update, some clients that send normal progress metadata were rejected as if
  they spoke a newer protocol, so tool lists and tool calls failed. Those
  clients connect and call tools again; broken modern requests still fail
  clearly.

### Added

- **Prove it from `anvil start`.** On the result screen, press `t` for
  **Prove**. anvil runs a real secret check on a sample and tells you whether it
  caught the secret — a 30-second reality check, not a placeholder toast. If
  Prove cannot run here (no secret checks configured, unsupported languages), it
  says so plainly. It never pretends your editor's live save guard is working
  when only the check engine was tested.

### Changed

- **`anvil status` and `anvil start` tell the same story.** When one says
  "warming" while the assistant path is already live (or save-time is still
  catching up), status adds one plain line of meaning so you are not left
  reconciling two different truths.

- **Cleaner keyboard help on activation.** Consent and the result screen share
  one help bar — the key list no longer fights itself.

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
- `anvil mcp install --client <id>` configures twelve AI clients (Claude Code,
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

## Unreleased — next beta after 0.9.2-beta

Draft of customer-visible work already on the maintained branch. Version and
date land when the next tag ships. Focus: daily-path honesty and the Windows
install / self-update path.

### Fixed

- Windows PowerShell install completes on clean machines; dual-install guard
  only refuses a true WinGet/Scoop + cargo-dist clash.
- Public installers no longer embed private GitHub issue numbers.
- `anvil update --check` and install-method reporting work for cargo-dist
  installs under the `eddacraft-anvil` receipt layout.
- Install banner and welcome no longer promise daily save-time from bare
  `anvil start`.
- Project-scoped interactive start can install MCP clients.
- Start MCP copy states observed entry presence (and restart need), not
  authorship or "editor has seen".
- No false "log in again" nag for ordinary sessions on a transient network
  check.
- Workspace register refuses to claim success when nothing durable stuck;
  `workspace list --json` returns JSON.
- Pure `anvil status` no longer creates `.anvil/cache` as a side effect.
- Start result screen: one help bar and a full wrapping `next:` step.
- Welcome autoplay failures recover inside the TUI.
- File paths print consistently (project-relative inside the tree; ordinary
  absolute outside; no Windows `\\?\` / mixed-slash noise).
- Whole-file audit findings drop the misleading line `0`; out-of-workspace
  secret paths keep their real location (Windows SARIF fingerprint churn once).
- Init and doctor recognise `.anvil.yaml` and the other supported config names.

### Changed

- Activation Install lists the assistants you chose this run; consent steps use
  plain-language "what is this".
- Start value receipt names machine-wide vs repository evidence scope.
- `audit`, `gate`, and `check` disclose what they actually cover (including
  secret/antipattern file-type domains); audit scope is consistent across TUI,
  SARIF, and plain/JSON.
- Init first-scan and success summaries scope clean results and name
  `.gitignore` when ignore entries change.
- Status/insights name next steps and counted domains; open admission is
  disclosed.
- Start flag conflicts and non-git init/register messages are consistent;
  post-upgrade recovery copy points at the right install-method steps.
- Gate "blocking" means threshold; pre-commit labels pre-existing tree debt.
- Config-mode hooks report honestly in doctor/status.
- Watch save-time verdicts are scoped; TUI queue/history show relative ages.
- Architecture and CI TUI completion copy stay within verified writes.

### Docs

- Public CLI docs match multi-client MCP flags and auth exit code 3.
- Tutorial refusal and antipattern-scan naming stay accurate; tutorial autoplay
  keeps effects inside the path-picker workspace safety chain.

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

- Authentication-required action commands exit **`3`** for safer automation;
  read-only `anvil status` stays exit **`0`** with an informational envelope.
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
