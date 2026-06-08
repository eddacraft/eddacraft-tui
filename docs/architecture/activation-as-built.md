# Activation Orchestrator — As-Built

| Type     | Authority | Owner  | Status | Freshness                                                             |
| -------- | --------- | ------ | ------ | --------------------------------------------------------------------- |
| As-built | Derived   | LAUNCH | Live   | Last reviewed 2026-05-07 against `v0.6.0-beta` and `crates/anvil-cli` |

| Upstream                                                                  | Downstream                                                                                   |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `crates/anvil-cli`, `crates/anvil-kernel`, `crates/anvil-checks`, ADR-001 | anvil start / status / doctor / tutorial CLI surfaces, MCP install step, activation TUI path |

> **Status:** Live (beta) **Last reviewed:** 2026-05-07 against `v0.6.0-beta`
> slate (HEAD `8bbe65b9`) **Module:** `crates/anvil-cli/src/activation/`
> **Module owner (APS):** LAUNCH (`launch-flow-readiness.aps.md`, 18/18
> complete) **Used by:** `anvil start`, `anvil status --verify`,
> `anvil tutorial` (ProtectionLoop default), `anvil doctor` (state-vocabulary
> alignment)

## Overview

`anvil start` is the canonical first-minute surface for `v0.6.0-beta`: install →
`cd repo` → `anvil start`. The command is a thin wrapper over the activation
orchestrator (`crates/anvil-cli/src/activation/orchestrator/mod.rs:55`); the
orchestrator composes only read-safe / idempotent primitives, then renders one
`ActivationDiagnostic` ending in a single literal `state:` word from a fixed
six-element vocabulary.

The honesty contract is the load-bearing invariant: surfaces never claim
pre-write protection without evidence, and the printed `state:` literal is the
only allowed vocabulary for activation outcomes. The same `ProtectionState` enum
(`crates/anvil-cli/src/activation/state.rs:15`) is consumed by
`anvil status --verify`, `anvil doctor`, the protection-loop tutorial path, and
JSON consumers — there is one renderer, surfaces cannot drift.

## Architecture diagram

```text
                        ┌──────────────────────────────┐
   anvil start  ───────▶│  commands/start.rs           │
                        │  (StartArgs: --verify --watch)│
                        └──────────────┬───────────────┘
                                       │
                read-only? ───────────┐│
                                      ▼▼
                ┌──────────────────────────────────────┐
                │ activation::orchestrator::run        │ ◀── mutating path
                │  1. probe verify                     │     (commands/start.rs:119)
                │  2. init if .anvilrc absent          │
                │  3. baseline.json if absent          │
                │  4. install_for_clients              │── ▶ ~/.cursor/mcp.json
                │  5. re-verify                        │── ▶ ~/.claude.json
                └──────────────┬───────────────────────┘
                               │
                               ▼
   activation::verify  ──────▶ ┌──────────────────────────┐
   (read-only path,            │  ActivationDiagnostic    │
    --verify / --json)         │  • config: ConfigStatus  │
                               │  • mcp:    BTreeMap      │
                               │  • watch:  WatchTier     │
                               │  • baseline_summary      │
                               │  • language_profile      │
                               │  • last_error            │
                               └────────────┬─────────────┘
                                            │
                            global.json? ───┴───┐
                                                ▼
                ┌──────────────────────────┬────────────────────────┐
                │ render_human_with_install│ render_json            │
                │ (terminal block ending   │ (single JSON document) │
                │  in `state: <literal>`)  │                        │
                └──────────────────────────┴────────────────────────┘
                                            │
                            --watch?  ──────┘
                                            │
                                            ▼
                ┌────────────────────────────────────────┐
                │ WatchDecision::for_diagnostic          │
                │  Spawn → kernel watcher inline         │
                │  NoOpRedundant / SkipConfigInvalid /   │
                │  SkipConfigAbsent / SkipError /        │
                │  SkipNoCoverage / NotRequested         │
                └────────────────────────────────────────┘
```

## The protection state vocabulary

The vocabulary is fixed and exhaustive; all six variants are defined in
`crates/anvil-cli/src/activation/state.rs:15-45` and serialised via
`ProtectionState::label` (`state.rs:50-59`). Surfaces never invent ad-hoc
strings.

| Literal                  | Meaning                                                                        | What drives it                                                                                      | User's next action                                                    |
| ------------------------ | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `protecting`             | Pre-write `anvil_validate_write` evidence has been observed live in this repo. | Highest MCP tier across clients is `LiveValidation` (`diagnostic.rs:223`).                          | None — try the AI guardrail demo.                                     |
| `ready_restart_required` | MCP config is wired and the server starts; the editor has not yet attached.    | Highest MCP tier is `RestartRequired` or `RestartHandshakeVerified` (`diagnostic.rs:232-235, 248`). | Restart Cursor / Claude Code; re-run `anvil start --verify`.          |
| `watching`               | Save-time fallback only; pre-write attachment is not in evidence.              | `WatchTier::Running` and MCP below `RestartRequired` (`diagnostic.rs:237-246`).                     | Wire pre-write MCP if possible; otherwise accept the fallback.        |
| `needs_action`           | No literal protection claim; user has actionable next steps.                   | Default branch when no stronger signal applies (`diagnostic.rs:263`).                               | Read the `next:` repair hint below the diagnostic.                    |
| `unsupported`            | Repo languages are out of scope for this release.                              | `all_languages_unsupported` and MCP below `RestartRequired` (`diagnostic.rs:257-259`).              | Wait for the language pack, or scope anvil to a TS / JS subdirectory. |
| `error`                  | Activation hit a hard error before any state could be established.             | `last_error.is_some()` or `ConfigStatus::Invalid` (`diagnostic.rs:213-219`).                        | Read `last_error:` and the `next:` repair hint.                       |

The state mapping lives in a single function,
`ActivationDiagnostic::protection_state` (`diagnostic.rs:211-264`); call sites
must go through it rather than computing state ad-hoc from the layered probes.

The vocabulary contract is test-pinned: `state.rs:96-117` proves the six labels
are unique and snake_case;
`crates/anvil-tui/src/surfaces/tutorial/mod.rs:881-908`
(`protection_loop_copy_uses_activation_state_vocabulary`) pins the tutorial copy
against drift.

## Lifecycle: `anvil start` (mutating path)

The mutating path runs when neither `--verify` nor `--json` is set. The
orchestrator's `run_with_home` (`orchestrator/mod.rs:68-140`) executes five
ordered steps:

1. **Probe config (and run init if absent).** `verify_with_home` is called first
   (`orchestrator/mod.rs:74`); if `ConfigStatus::Absent`, the orchestrator
   invokes `commands::init::run_in` inline (`orchestrator/mod.rs:75-78`).
   `init::run_in` writes `.anvilrc` and runs the LAUNCH-004 post-init first-scan
   transparently (`commands/init.rs:57-93`). Structurally invalid config
   surfaces as `state: error` via `verify` — the orchestrator does not overwrite
   it.

2. **Write activation baseline if absent (LAUNCH-010).** When
   `.anvil/baseline.json` is missing, the orchestrator calls
   `services::sample_analyser::run_baseline_scan` and writes the resulting
   fingerprint set via `baseline::write_baseline`
   (`orchestrator/mod.rs:91-101`). Failure to write is logged and swallowed —
   the baseline is a future-change-tracking aid, not a blocker. The schema lives
   in `activation/baseline.rs`; idempotency is enforced by the
   `!baseline::baseline_exists(root)` guard (`orchestrator/mod.rs:91`).

3. **Install MCP entries for Cursor and Claude Code (LAUNCH-009 part 2).** The
   orchestrator resolves `current_exe()`, builds an `AnvilEntry::local_stdio`
   (`mcp_client.rs:91-101`), then calls `install::install_for_clients`
   (`orchestrator/mod.rs:108-124`). The install step is idempotent (`UpToDate` →
   skip), refuses to overwrite `UnsafeDrift`, and pre-selects `NotPresent` /
   `SafeDrift` candidates. See
   [MCP install (LAUNCH-009)](#mcp-install-launch-009) below.

4. **Re-probe.** A second `verify_with_home` call (`orchestrator/mod.rs:130`)
   absorbs the install side-effects so the rendered diagnostic carries the
   post-install MCP tier. Any aggregated install failure is folded into
   `diagnostic.last_error` so `protection_state()` collapses to `Error` for JSON
   consumers (`orchestrator/mod.rs:135-137`).

5. **Render.** The CLI renders via
   `activation::render_human_with_install(&diagnostic, &install_report)`
   (`commands/start.rs:158`) or `activation::render_json` under JSON mode
   (`commands/start.rs:153`). The block ends in a single `state: <literal>` line
   plus a per-client `install:` summary (`render.rs:202-237`).

After rendering, if any client install reported `Failed`, the CLI propagates a
non-zero exit so `anvil start && next-step` shell pipelines do not silently
advance (`commands/start.rs:172-179`).

**First-run marker invariant.** `anvil start` does NOT touch `.anvil/first-run`;
that marker is owned exclusively by `anvil welcome`
(`orchestrator/mod.rs:28-30`). The two surfaces never fight for first-run state.

## Read-only path (`anvil start --verify`)

`--verify` (or any invocation under `--json`) short-circuits to
`activation::verify` (`commands/start.rs:91, 113-117`). Same backend as
`anvil status --verify` (LAUNCH-012); idempotency is unit-pinned by
`idempotent_reverify_is_pure` (`diagnostic.rs:1280-1319`).

What `--verify` skips:

- `init` (no `.anvilrc` write).
- The first-scan baseline write.
- The MCP install step (no editor configs touched).

What `--verify` still does (the read-only probes inside `verify_with_home`,
`diagnostic.rs:333-431`):

- Probes `.anvilrc` parse status (JSON / TOML / YAML) via `probe_config_status`
  (`diagnostic.rs:525-565`).
- Reads each registered editor's MCP config via `mcp_client::probe_all`
  (`mcp_client.rs:636-649`); promotes `RestartRequired` clients to
  `RestartHandshakeVerified` after a 1s MCP `initialize` handshake against the
  installed entry (`diagnostic.rs:462-523`, `mcp_client.rs:890-966`).
- Reads `.anvil/baseline.json` if present (`diagnostic.rs:608-634`).
- Walks the working tree for the language profile via
  `language_profile::profile_repo` (`language_profile.rs:330-375`).
- Computes the `WatchTier::Offered` gate (`diagnostic.rs:405-419`).

`--json` implies `--verify` semantics so init's own JSON record cannot
concatenate with the activation diagnostic and break parseable consumers
(`commands/start.rs:91`). CI consumers parse the `state` field; they do not
inspect the exit code (`commands/start.rs:172-179` only fires under the mutating
path).

## Watch fallback (`anvil start --watch`)

`--watch` runs the activation orchestrator first, then optionally hands off to
the kernel watcher inline. The decision is captured in a single enum,
`WatchDecision` (`commands/start.rs:239-258`); the variant is computed once
(`commands/start.rs:133-137`) and consumed by both the diagnostic-synthesis
branch (`commands/start.rs:148-150`) and the post-render branch
(`commands/start.rs:185-229`), so the printed copy and the spawn behaviour
cannot drift.

| `WatchDecision`     | Trigger                                                                                      | Effect                                                                                                                      |
| ------------------- | -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `NotRequested`      | `--watch` not passed.                                                                        | No synthesis; orchestrator state rendered as-is.                                                                            |
| `Spawn`             | Config valid, no `last_error`, MCP below `RestartRequired`, at least one supported language. | Synthesise `WatchTier::Running` before render (`commands/start.rs:148-150`); print hand-off marker; enter `watch_cmd::run`. |
| `NoOpRedundant`     | MCP at `LiveValidation`.                                                                     | Print "skipped — MCP pre-write validation is live; save-time fallback is redundant." (`commands/start.rs:199-204`)          |
| `SkipConfigInvalid` | `ConfigStatus::Invalid`.                                                                     | "fix the config error first, then re-run …" (`commands/start.rs:205-210`)                                                   |
| `SkipConfigAbsent`  | `ConfigStatus::Absent`.                                                                      | "no `.anvilrc` to honour; run `anvil init` first …" (`commands/start.rs:211-216`)                                           |
| `SkipError`         | `last_error.is_some()`.                                                                      | "activation error must be cleared before save-time fallback can run …" (`commands/start.rs:217-222`)                        |
| `SkipNoCoverage`    | All detected languages unsupported.                                                          | "repo languages are out of scope for the current release …" (`commands/start.rs:223-228`)                                   |

The decision priority mirrors `protection_state`'s priority
(`commands/start.rs:282-298`): error → config absent/invalid → LiveValidation →
all-unsupported → spawn.

**Honesty contract** (declared in `commands/start.rs:29-54`):

- `--watch` synthesises `WatchTier::Running` _only_ when the spawn is going to
  happen. The synthesis never claims a tier stronger than the `protection_state`
  mapping permits at `WatchTier::Running` — `Watching` or
  `ReadyRestartRequired`, never `Protecting`.
- At `LiveValidation`, `--watch` is a no-op (pre-write covers the save path; the
  watcher would generate redundant noise).
- At `RestartRequired+`, `--watch` _does_ spawn — the user has explicitly asked
  to layer save-time fallback on top of the restart-pending state (the
  `protection_state` mapping demotes `Watching` to `ReadyRestartRequired` so the
  printed state still nudges toward restart; `diagnostic.rs:719-742`).

`--watch + --verify` and `--watch + --json` are rejected explicitly with a hint
(`commands/start.rs:101-110`): `--verify` is read-only and cannot spawn
processes; `--json` requires a single document on stdout but the watcher streams
event lines.

## Language profile (LAUNCH-015 / LAUNCH-016)

The single source of truth for "what languages does anvil claim coverage for in
this release?" is `LANGUAGE_REGISTRY` (`language_profile.rs:78-145`). Surfaces
never duplicate the registry inline.

| Language                     | Extensions                    | Coverage tier | Basis                                                   |
| ---------------------------- | ----------------------------- | ------------- | ------------------------------------------------------- |
| TypeScript                   | `.ts .tsx .mts .cts`          | supported     | antipattern + secret checks ship                        |
| JavaScript                   | `.js .jsx .mjs .cjs`          | supported     | antipattern + secret checks ship                        |
| Web (HTML/CSS)               | `.html .htm .css .scss .less` | supported     | antipattern checks ship                                 |
| SQL                          | `.sql`                        | partial       | structural governance pending SURFSQL Phase 1           |
| Markdown                     | `.md .mdx`                    | partial       | secret checks ship; structural governance pending MDGOV |
| Python                       | `.py .pyw`                    | unsupported   | PYLAN anchor not yet shipped                            |
| Rust                         | `.rs`                         | unsupported   | RSTLAN anchor not yet shipped                           |
| Go, Java/Kotlin, C/C++, Ruby | (see registry)                | unsupported   | no language pack scheduled in v0.5.x                    |

`profile_repo` (`language_profile.rs:330-375`) walks the working tree in a
single pass, classifying each file's extension against the registry. Vendored
and generated paths are filtered by an inline denylist
(`language_profile.rs:424-454`) drawn from
`anvil-checks::filter::DEFAULT_DIR_EXCLUDES` and `BUILD_ARTEFACT_DIRS`, plus
`.anvil` itself — without those excludes a SvelteKit / Angular repo could flip
its protection state away from `unsupported` because of generated TypeScript
under `.svelte-kit/` or `.angular/`.

`partition_for_language_specific_checks` (`language_profile.rs:284-320`) returns
`(scannable: Vec<&str>, LanguageSkipLedger)` for any candidate file list. Files
whose extension belongs to an `Unsupported` registry entry are dropped from the
scannable list and tallied in the ledger keyed by language name with
`reason: "unsupported"`. Cross-language checks (secrets, env-template) must NOT
use this partition — they run on all files. Callers should only invoke it before
language-specific antipattern checks.

**LAUNCH-016 acceptance status (per the APS module,
`plans/archive/modules/launch-flow-readiness.aps.md:355-385`):**

- (a) Default behaviour scans `.ts` and skips `.py` for language-specific
  checks: **met** by `AntipatternCheckConfig::default().extensions`; the
  partition helper is the visible-ledger contract for downstream adopters.
- (b) Secret scanner runs on both: **hand-off**. The post-init activation path
  does not invoke the secret scanner; the partition helper is left out of
  secret-scan call sites.
- (c) Run summary records the skip with language and count: **met** via
  `AnalysisOutcome.skipped_unsupported_languages` and the `repo_languages` array
  in `anvil status --verify --json`.
- (d) Explicit `extensions:` opt-in to scan unsupported languages: **hand-off**
  to a follow-up PR that wires user-config-aware filtering through
  `commands::check`, `commands::watch`, and `commands::audit`.

## MCP install (LAUNCH-009)

The install module (`activation/orchestrator/install.rs`) drives per-client
install for the two clients registered in v1: Cursor and Claude Code
(`mcp_client.rs:330-332`).

**Per-client config paths** (workspace-first then global):

- **Cursor** — `.cursor/mcp.json` (workspace) → `~/.cursor/mcp.json` (global)
  (`mcp_client/cursor.rs:33-156`).
- **Claude Code** — `.claude.json` (workspace) → `~/.claude.json` (global)
  (`mcp_client/claude_code.rs:35-145`). Editor configs that hold sensitive data
  are written with mode `0o600` on Unix
  (`activation/orchestrator/install.rs:42-48`).

**Drift policy** (`activation/orchestrator/install.rs:30-41`):

| `DriftClass`  | Interactive  | Non-interactive | Notes                                                               |
| ------------- | ------------ | --------------- | ------------------------------------------------------------------- |
| `NotPresent`  | pre-selected | auto-install    | fresh write, no merge needed                                        |
| `SafeDrift`   | pre-selected | auto-install    | rewrite over a recognised anvil-shaped entry (likely version drift) |
| `UpToDate`    | not shown    | skip            | nothing to do                                                       |
| `UnsafeDrift` | not shown    | skip with note  | foreign tool / unknown shape — never overwrite                      |

The drift classifier (`mcp_client.rs:457-508`) refuses to call any entry
"anvil's" unless the canonical args (`["mcp", "serve", "--stdio"]`) match AND
the command basename is `anvil` / `anvil.exe`. A foreign command such as
`/bin/bash` carrying our key + args is `UnsafeDrift`. Bare `"anvil"` (from
`anvil mcp-config` PATH-resolved) is treated as equivalent to a full
`current_exe()` path via `entries_equivalent` (`mcp_client.rs:537-573`) so those
installs do not falsely report as `RestartRequired` after an idempotent re-run.

**Atomicity and safety:**

- Writes go through `util::atomic_write` (rename a uniquely-named tempfile in
  the same directory).
- Symlink-parent guard (`activation/orchestrator/install.rs:446-456`): installs
  refuse to write when any parent of the target is a symlink (a symlinked
  `~/.cursor` or `~/.claude.json` parent would otherwise let `tempfile_in` write
  through the link).
- Configs that fail to parse are surfaced as `UnsafeDrift` with the parser
  reason; no overwrite, no silent install
  (`activation/orchestrator/install.rs:323-339, 659-677`).

`--verify` mode never enters this module — installs are skipped along with init
and the baseline write (`commands/start.rs:113-120`).

The interactive picker (`demand::MultiSelect`,
`activation/orchestrator/install.rs:500-531`) is gated behind a strict
TTY-and-not-CI check (`orchestrator/mod.rs:162-168`): stdin and stderr must both
be terminals, `--no-tui` and `--json` must not be set, and
`is_non_interactive_env` (the `CI=true` / `GIT_DIR` / `ANVIL_NO_PROMPT` gate)
must be false.

## Diagnostic schema

`ActivationDiagnostic` (`diagnostic.rs:172-204`) is the structured probe result.
The layered shape is deliberate so a surface can render "config valid, MCP
startable, restart still required" without collapsing distinct failures.

Fields:

- `config: ConfigStatus` — `Absent` / `Valid` / `Invalid`
  (`diagnostic.rs:127-134`).
- `mcp: BTreeMap<McpClientId, McpProbeResult>` — one row per registered client
  (Cursor + ClaudeCode in v1); `McpProbeResult` carries `tier: McpTier`
  (`diagnostic.rs:62-84`) and `transport: McpTransport`
  (`mcp_client.rs:115-129`). The tier ladder runs
  `NotDetected → ConfigAbsent → ConfigPresent → ServerStartable → RestartRequired → RestartHandshakeVerified → LiveValidation`.
- `watch: WatchTier` — `NotRequested` / `Offered` / `Running`
  (`diagnostic.rs:103-122`). `Offered` is informational and does not change
  `protection_state()`; `Running` is what the watcher synthesis sets.
- `baseline_present: bool` and `baseline_summary: Option<BaselineSummary>`
  (`diagnostic.rs:151-164`) — per-kind counts from `.anvil/baseline.json`.
- `last_error: Option<String>` — propagates MCP-probe and baseline-load failures
  plus any aggregated MCP install failure (`orchestrator/mod.rs:135-137`).
- `all_languages_unsupported: bool` and `language_profile: RepoLanguageProfile`
  (`language_profile.rs:158-211`) — drives the `Unsupported` state.

JSON output shape (`render.rs:294-351`) is stable contract. Keys are pinned at
`render.rs:528-555`:

```json
{
  "state": "ready_restart_required",
  "headline": "Ready, restart required …",
  "config": "valid",
  "mcp": [{"client": "cursor", "tier": "restart_required", "transport": "stdio"}, …],
  "watch": "not_requested",
  "baseline_present": true,
  "baseline": {"total": 4, "antipattern": 3, "secret": 1, "created_at": "…"},
  "last_error": null,
  "all_languages_unsupported": false,
  "repo_languages": [{"name": "TypeScript", "files_seen": 37, "coverage_tier": "supported", "basis": "…"}, …],
  "unclassified_files_seen": 0
}
```

Per-client install outcomes are intentionally **not** in the JSON contract —
`anvil start --json` short-circuits to a read-only probe so install never runs
in JSON mode (`render.rs:251-289`).

## Cross-cutting concerns

### Honesty contract

- Surfaces never claim pre-write protection without evidence; only
  `LiveValidation` produces `state: protecting` (`diagnostic.rs:223`).
- The six `ProtectionState` literals are the ONLY allowed vocabulary
  (`state.rs:9-13`).
- The "MCP pre-write validation is not attached" note in the human renderer is
  suppressed in five cases — `RestartRequired+`, `Error`, `Unsupported`,
  `Protecting`, and `NeedsAction + ConfigStatus::Absent` — so the note never
  orphans next to a state whose headline already carries the partial-state
  language (`render.rs:84-102`).
- Copy invariants are test-pinned:
  `protection_loop_copy_uses_activation_state_vocabulary`
  (`crates/anvil-tui/src/surfaces/tutorial/mod.rs:881-908`) asserts every state
  literal appears in the tutorial body; the companion test at `:910-955` asserts
  the tutorial never uses words like "protected" / "fully protected" — only
  `anvil start --verify` is allowed to produce that claim.

### Idempotency

Re-running `anvil start` is safe.

- `init` is gated by `ConfigStatus::Absent` (`orchestrator/mod.rs:74-78`);
  `orchestrator_skips_init_when_config_valid` (`orchestrator/mod.rs:213-240`)
  pins that the `.anvilrc` mtime does not change across re-runs.
- The activation baseline is written only when absent
  (`orchestrator/mod.rs:91`); pinned by
  `orchestrator_baseline_write_is_idempotent` (`orchestrator/mod.rs:358-395`).
- MCP install reports `skipped — already up to date` for `UpToDate` candidates
  (`render.rs:223-225`); pinned by `install_is_idempotent`
  (`activation/orchestrator/install.rs:773-800`).
- `verify` itself is pure — `idempotent_reverify_is_pure`
  (`diagnostic.rs:1280-1319`) snapshots every mtime in the work tree and asserts
  re-runs leave them all untouched.

### State alignment

`anvil start`, `anvil status --verify`, `anvil doctor`, and `anvil tutorial`
(ProtectionLoop default) all consume the same `ProtectionState` enum and share
the same renderers (`render::render_human` / `render::render_json`). The mapping
logic exists in exactly one place (`ActivationDiagnostic::protection_state`,
`diagnostic.rs:211-264`).

### Council-locked exclusions

What `anvil start` deliberately does **not** do
(`plans/specs/2026-05-04-launch-a1-execution.md:57-67`):

- No `.cursorrules` / `.clauderules` / global AI rule-file injection.
- No cloud login, team policy pull, or CI setup.
- No default git hook installation.
- No demo fixtures, challenge files, or guaranteed-catch prompt catalogues.
- No Windsurf, VS Code, Copilot CLI, or Codex CLI MCP install — Cursor + Claude
  Code only.
- No process auto-attach. anvil only knows what it wired itself; it does not
  "find the AI session running in this repo".
- No no-args TUI theatre — `anvil start` is the activation surface;
  `anvil welcome` remains the menu / tutorial surface.

## Tutorial integration (LAUNCH-014)

`TutorialPath::ProtectionLoop` is the value-first default path
(`crates/anvil-tui/src/surfaces/tutorial/paths.rs` consumed at
`crates/anvil-tui/src/surfaces/tutorial/mod.rs:264`). On a fresh tutorial state,
`ProtectionLoop` is index 0 and pre-selected (pinned by
`protection_loop_path_is_default_first_path`,
`crates/anvil-tui/src/surfaces/tutorial/mod.rs:872-877`).

The five-step walk is value-first (no fixture authoring); the final step points
the user at `anvil start --verify` — the only surface that produces a literal
`ProtectionState` (pinned at
`crates/anvil-tui/src/surfaces/tutorial/mod.rs:949-954`). Copy invariants are
test-pinned (`:881-955`) so the tutorial body must reference all five
user-actionable state literals (`protecting`, `ready_restart_required`,
`watching`, `needs_action`, `unsupported`) and must NOT contain `protected` /
`fully protected` / `pre-write` claims.

The CLI dispatcher at `crates/anvil-cli/src/commands/tutorial.rs:46-93` loads
`TutorialState::new()` and runs the surface; the file is the entry from the
binary, while the path catalogue lives in `anvil-tui`.

## `anvil version` (LAUNCH-013)

`anvil version` (`crates/anvil-cli/src/commands/version.rs:78-113`) is
install-method-aware. `InstallMethod` (`commands/version.rs:42-76`) covers
`Homebrew`, `Scoop`, `Winget`, `CargoDist` (the cargo-dist installer /
PowerShell installer detected via the install receipt), `CargoInstall`,
`DevBuild`, and `Unknown`. Each variant maps to a recommended upgrade command
via `upgrade_command_for` (`commands/version.rs:169-196`).

The latest-release lookup is HTTPS to
`api.github.com/repos/eddacraft/anvil/releases/latest` and is bounded by a
3-second timeout. `--offline` skips the network probe entirely
(`commands/version.rs:83-87`). Network failures are non-fatal — the local
version always prints.

JSON shape is locked (`commands/version.rs:117-130`): `current_version`,
`latest_version` (`Option<&str>` — null when the network probe was skipped or
failed), `update_available`, `install_method`, `upgrade_command`. Adding fields
is allowed; renames or removals are breaking.

## Known gaps

Items below are dated and load-bearing for the v0.6.0-beta release:

### G-01: LAUNCH-016 partial — `extensions:` user opt-in deferred (2026-05-07)

`partition_for_language_specific_checks` ships the contract; the `extensions:`
user-config opt-in (acceptance criterion (d)) for re-enabling
unsupported-language scanning is hand-off to a follow-up PR through
`commands::check`, `commands::watch`, and `commands::audit`
(`plans/archive/modules/launch-flow-readiness.aps.md:380-385`). The seam is in
place — downstream consumers compose the user-config decision before invoking
`partition_for_language_specific_checks`.

### G-02: Watch-liveness probing is unwired pending LAUNCH-011 (2026-05-07)

The protection-loop tutorial step 5 only enumerates what `--verify` actually
probes today — config, MCP entries on disk, baseline presence, language profile,
watch tier offer-gate. Live watch-process probing (which would produce
`WatchTier::Running` outside the `--watch` synthesis path) is not wired. The
`WatchTier::Running` state is reachable today only through `anvil start --watch`
synthesis (`commands/start.rs:148-150`).

### G-03: Windows daemon validation reports `not-wired` (2026-05-07)

`LocalDaemonValidationClient::validate_pre_write` is `#[cfg(unix)]`-gated. On
Windows the MCP path still works but the `correlation.daemonStatus` field
returned by `anvil_validate_write` is always `"not-wired"`. Cross-link to
`docs/architecture/intercept-as-built.md` when written (planned, in flight).
Currently surfaced in `docs/archive/runbooks/v0.6.0-beta-release-runbook.md`.

### G-04: Tutorial `--json` mode constraints (2026-05-07)

`anvil start --json` short-circuits to read-only verify so init's own JSON
output cannot concatenate; this is the documented single-document contract
(`commands/start.rs:86-91`, `render.rs:251-289`). `--watch + --json` is
explicitly rejected for the same reason (`commands/start.rs:106-110`). CI
consumers needing a side-effecting JSON flow must run `anvil init --json` and
`anvil start --json` separately.

## Source references

`crates/anvil-cli/src/activation/`:

- `mod.rs` — module surface; re-exports `ActivationDiagnostic`, `verify`,
  `render_human`, `render_human_with_install`, `render_json`.
- `state.rs` — `ProtectionState` enum + headlines (single vocabulary).
- `diagnostic.rs` — `ActivationDiagnostic`, `protection_state` mapping,
  `verify_with_home`, the watch-tier offer gate.
- `render.rs` — human and JSON renderers; per-client install summary;
  `repair_hint` for the `next:` line.
- `language_profile.rs` — `LANGUAGE_REGISTRY`, `profile_repo`,
  `partition_for_language_specific_checks`, `LanguageSkipLedger`.
- `baseline.rs` — `.anvil/baseline.json` schema (LAUNCH-010), atomic writer,
  `read_baseline` / `baseline_exists`.
- `mcp_client.rs` — `McpClient` trait, `AnvilEntry`, `DriftClass`, `probe_all`,
  `probe_startable` (1s MCP `initialize` handshake), `entries_equivalent`,
  `looks_like_anvil`.
- `mcp_client/cursor.rs` — Cursor impl (`.cursor/mcp.json`).
- `mcp_client/claude_code.rs` — Claude Code impl (`.claude.json`).
- `orchestrator/mod.rs` — composed `run` / `run_with_home`; init, baseline,
  install, re-verify, last-error fold.
- `orchestrator/install.rs` — drift policy, picker UX, atomic writes, symlink
  guard, idempotency.

`crates/anvil-cli/src/commands/`:

- `start.rs` — `anvil start` CLI; `WatchDecision` enum; `--watch` hand-off and
  skip copy; `--watch + --verify` / `--watch + --json` rejection.
- `welcome.rs` — `anvil welcome` (the menu / tutorial surface; sole owner of the
  `.anvil/first-run` marker).
- `init.rs` — `init::run_in` invoked by the orchestrator when `.anvilrc` is
  absent.
- `tutorial.rs` — CLI tutorial dispatcher; loads `TutorialState::new()`
  (`anvil-tui` ProtectionLoop is the default path).
- `version.rs` — `anvil version` install-method detection and upgrade-command
  mapping.

## Related docs

- `plans/archive/modules/launch-flow-readiness.aps.md` — APS module file
  (intent + acceptance for every LAUNCH-NNN task).
- `plans/specs/2026-05-04-launch-a1-execution.md` — Tier A1 execution plan with
  the council-locked hard constraints.
- `docs/public/anvil/guides/wow-start-demo.md` — public-side narrative this doc
  backs up.
- `docs/public/anvil/quickstart.md` — beta quickstart (full 10-minute install +
  activate path).
- `docs/architecture/intercept-as-built.md` — planned, in flight (will carry the
  daemon-side detail referenced in G-03).
- `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` — operator-facing
  caveats.
- `RELEASE-PLAN.md` — Tier A1 framing.
