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

## 0.9.3-beta — 7 August 2026 — Honesty and Windows path

### Fixed

- **Windows PowerShell install runs again on clean machines.** The dual-install
  guard (WinGet/Scoop vs cargo-dist) was injected in a way that exited before
  the installer body ran, so `irm … | iex` could do nothing on a machine with no
  package-manager anvil. The guard now runs after cargo-dist's parameters and
  only refuses a true dual install; clean installs complete as normal.

- **Public installers no longer show private GitHub issue numbers.** Earlier
  Windows installer banners and help text embedded internal tracker ids. Anyone
  who downloaded the asset could see those numbers even though the private
  issues stay inaccessible. Ship artefacts now use neutral product wording only
  — no private issue-number branding in installer copy.

- **`anvil update --check` and install method work for cargo-dist installs.**
  Official installer receipts live under the `eddacraft-anvil` app name. anvil
  loads that layout (with a legacy `anvil` fallback), reports the install method
  honestly, and can check GitHub for updates instead of misclassifying the
  install as a plain `cargo install`.

- **Install and welcome no longer promise save-time from bare `anvil start`.**
  The post-install banner and welcome next-step copy described start as daily
  save-time protection. A bare start activates and reports state; it does not
  attach save-time to the worktree on its own. Banner and welcome now describe
  activation honestly, lead newcomers at `anvil welcome` where appropriate, and
  name save-time only where a path actually requests it (for example `--watch`).

- **Project-scoped `anvil start` can install MCP.** Interactive start with
  `--mcp-scope project` no longer claims "MCP installation disabled" solely
  because the scope is project. Consent can offer and install project-scoped
  clients like the headless path already could.

- **Start MCP copy states what was observed, not who wrote it.** Meanings that
  used to say the editor "has seen" anvil or that "anvil has written" the entry
  now say an MCP entry is present (and whether restart is still needed). The
  same honesty applies to `start --verify`, which is a non-mutating probe.

- **No false "log in again" nag for people who already have a session.**
  Ordinary beta or pro credentials are not treated as edicts. Only an explicit
  edict forces the edict re-verify path, so a transient network check no longer
  demands `anvil auth login` when you are already authenticated.

- **Workspace register no longer claims success when nothing stuck.** If
  registration does not leave a durable entry that `workspace list` can see,
  anvil says so instead of printing `Registered …` with exit 0.

- **`workspace list --json` returns JSON.** Machine-readable list output is
  honoured instead of falling back to plain text, so scripts can parse workspace
  membership reliably.

- **`anvil status` no longer creates project cache as a side effect.** Pure
  status leaves never-activated trees clean; what's-new markers write only when
  `.anvil/cache` already exists.

- **`anvil start` result screen shows one help bar and a full next step.** The
  activation verdict no longer draws a second key legend that disagreed with the
  shell bar (arrows vs `j/k`). The `next:` guidance line is promoted out of the
  tree and wraps at typical console widths so the whole next step stays
  readable.

- **Welcome autoplay failures stay in the TUI.** Auth or demo failures during
  autoplay surface recovery inside the interface instead of dumping you to a
  bare shell with little context.

- **File paths read the same way everywhere.** `check`, the pre-commit gate,
  `audit`, and `skill install` each had their own idea of how to print a
  location, so one run could show you `src/app.py`, `/.env`, and
  `\\?\C:\project\...` for the same kind of finding. Paths inside your project
  now always print relative to it, anything outside prints as an ordinary
  absolute path, and Windows paths no longer arrive with the `\\?\` prefix or
  mixed slashes.

- **No more line `0`.** `audit` reported findings about a whole file — a
  committed `.env`, say — as `.env:0`, which read like a line number counting
  from zero. Whole-file findings now show just the filename, and `--format json`
  reports `"line": null` for them. Every line number anvil prints is 1-based.

- **Secrets found outside your project keep their real path.** A secret detected
  outside the workspace could be reported with its leading `/` removed, naming a
  file that was not the one scanned.

  On Windows only, because the paths in SARIF output change, code-scanning
  alerts for secret findings are fingerprinted afresh: expect existing alerts to
  close and reappear once after upgrading.

- **Init and doctor recognise `.anvil.yaml` (and the other supported names).**
  Discovery already accepted alternate config basenames; init's already-exists
  error and doctor's probe now match, so a YAML-named config is not treated as
  missing.

### Changed

- **Activation Install shows the assistants you chose.** The verdict Install
  block lists this-run outcomes for every client you selected in consent, not a
  parade of Cursor/Claude Code "not selected" / "already up to date" rows. Other
  detected clients you left alone collapse to one summary line.

- **Consent is grouped with plain-language "what is this".** Project, hooks,
  workflows, and MCP clients are separate steps with short blurbs on why anvil
  wants each write and what happens if you skip it. Selections persist across
  steps; submit is still explicit.

- **Start value receipt names its evidence scope.** When save-time or witness
  lines appear on a repeat start, they say whether the evidence is machine-wide
  or for this repository, so two machines or two clones are not mistaken for one
  picture.

- **`audit`, `gate`, and `check` say what they actually cover.** Chain and
  secret summaries disclose coverage and that the secret domain is not the same
  as a full `check` of every file type. Green `gate` / `check` results name the
  secret and antipattern file-type domains so a pass is not read as full-tree
  coverage. The same scope statement is carried through TUI, SARIF, and
  plain/JSON audit output.

- **Init first-scan copy matches the sample.** A clean first scan names the
  anti-pattern sample rather than sounding like a whole-project all-clear. When
  nothing matches the scan's language coverage, the hint says so instead of
  implying an empty repository failed.

- **Init summary names `.gitignore` when it changes ignore entries.** The plain
  and TUI success summaries list every path touched; the gitignore line is
  omitted when entries were already present.

- **`status` and `insights` name next steps and domains.** Protection:warming
  points at a next step (or refuses that label when it cannot). Zero counts in
  insights say which domain was counted so an empty score is not read as "all
  clear everywhere".

- **Open admission is disclosed, not flipped.** Where admission is intentionally
  open, surfaces say so instead of implying a closed default you never set.

- **Start flag and non-git honesty.** Combining `--no-mcp` with explicit MCP
  client flags fails loudly. `--format` warns when config already exists and the
  flag will not rewrite it. Non-git init vs worktree registration messages no
  longer contradict each other.

- **After upgrade, recovery paths are clearer.** When the daemon or MCP binary
  looks older than the CLI you just installed, status/update copy points at the
  usual recovery steps for that install method.

- **Gate copy: "blocking" means threshold, not severity=warning.** Banner
  wording no longer reads as if only low-severity "warnings" can block; it is
  findings that meet the block threshold.

- **Pre-commit gate labels pre-existing tree debt.** When the full-tree gate
  blocks on already-committed problems (for example a checked-in `.env`) while
  your staged change is clean, those findings are labeled as pre-existing so the
  current commit is not blamed for the whole yard. Full-tree scanning is
  unchanged.

- **Config-mode hooks report honestly in doctor and status.** Config-mode-only
  hooks no longer silent-Pass doctor or claim L3/L4 on. Fire is labeled
  unverified (or impossible on older Git), and file-mode remains the green
  default path.

- **Watch save-time verdicts are scoped and timed clearly.** Daemon-backed
  save-time checks state their family scope; partial or stale evidence dominates
  a clean snapshot. `--no-daemon` is a hard no-contact path. The TUI watch
  dashboard shows relative ages for queue and history so operators outside UTC
  do not misread recent work as hours stale (JSON timestamps stay absolute).

- **Architecture and CI completion copy in the TUI stays within verified
  writes.** Architecture summary names the verified config write and on-demand
  check/gate commands without claiming editor or commit-review wiring. CI
  summary no longer claims the pipeline is live after guidance-only steps.

### Docs

- **Public CLI docs match current flags and auth exit code 3.** Reference copy
  no longer implies only Claude Code and Cursor; skill install docs cover
  multi-client `--client` and moving files outside a skills directory.
- **Tutorial and antipattern-scan scope.** Non-interactive tutorial refusal
  exits non-zero with accurate copy. Antipattern-scan naming is clarified
  against the built-in rule catalogue. Tutorial autoplay keeps Escape and
  command effects inside the path picker's workspace safety chain.

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
