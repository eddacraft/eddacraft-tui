# Activation Orchestrator — As-Built

| Type     | Authority | Owner  | Status | Freshness                                                                                                                                                                       |
| -------- | --------- | ------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | LAUNCH | Live   | Last reviewed 2026-07-14 (targeted delta review: MCPX first-wave registry and optional client configuration); prior activation-spine review 2026-07-02 against main `d1fded280` |

| Upstream                                                                           | Downstream                                                                                   |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `crates/anvil-cli`, `crates/anvil-kernel`, `crates/anvil-checks`, ADR-001, ADR-092 | anvil start / status / doctor / tutorial CLI surfaces, MCP install step, activation TUI path |

> **Status:** Live (beta) **Last reviewed:** 2026-07-14 (targeted delta review:
> MCPX first-wave registry and optional client configuration); prior targeted
> review 2026-07-02 against main `d1fded280`; full review 2026-05-07 against
> `v0.6.0-beta` slate (HEAD `8bbe65b9`) **Module:**
> `crates/anvil-cli/src/activation/` **Module owner (APS):** LAUNCH
> (`launch-flow-readiness.aps.md`, 18/18 complete) **Used by:** `anvil start`,
> `anvil status --verify`, `anvil tutorial` (ProtectionLoop default),
> `anvil doctor` (state-vocabulary alignment)

## Overview

`anvil start` is the canonical first-minute surface for `v0.6.0-beta`: install →
`cd repo` → `anvil start`. The command is a thin wrapper over the activation
orchestrator (`crates/anvil-cli/src/activation/orchestrator/mod.rs:70`); the
orchestrator composes only read-safe / idempotent primitives, then renders one
`ActivationDiagnostic` ending in a single literal `state:` word from a fixed
six-element vocabulary.

Note the gate posture: `anvil start` is licence-gated; the ungated first-touch
demo surface is `anvil welcome` (ADR-080).

The honesty contract is the load-bearing invariant: surfaces never claim
pre-write protection without evidence, and the printed `state:` literal is the
only allowed vocabulary for activation outcomes. ADR-092 defines the activation
spine as daemon ensure, worktree registration, hooks where allowed, and
save-time validation, with MCP as an optional L0 upgrade rather than the sole
gate. The same `ProtectionState` enum
(`crates/anvil-cli/src/activation/state.rs:15`) is consumed by
`anvil status --verify`, `anvil doctor`, the protection-loop tutorial path, and
JSON consumers — there is one renderer, surfaces cannot drift.

The first-wave agent registry supplements this diagnostic. `anvil start` may
offer strongly detected MCP clients, and explicit `--mcp-client` selections can
install their documented config shapes. Those writes are reported separately;
they do not promote the Cursor/Claude-specific diagnostic to `protecting` or
claim that another client completed a live handshake.

DSV-051 adds the operator runbook for the headless save-time driver:
[`docs/runbooks/save-time-background-driver.md`](../runbooks/save-time-background-driver.md).
Activation copy points at `anvil intercept status` when daemon-backed state
needs inspection, and only claims daemon-backed save-time validation is armed
when the diagnostic proves `save_time_driver_attached`.

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
                │  4. register worktree with daemon    │── ▶ intercept daemon
                │  5. install_for_clients              │── ▶ ~/.cursor/mcp.json
                │  6. re-verify                        │── ▶ ~/.claude.json
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

| Literal                  | Meaning                                                                                  | What drives it                                                                                             | User's next action                                                             |
| ------------------------ | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `protecting`             | Pre-write `anvil_validate_write` evidence has been observed live in this repo.           | Highest MCP tier across clients is `LiveValidation` (`diagnostic.rs:223`).                                 | None — try the AI guardrail demo.                                              |
| `ready_restart_required` | MCP config is wired and the server starts; the editor has not yet attached.              | Highest MCP tier is `RestartRequired` or `RestartHandshakeVerified` (`diagnostic.rs:232-235, 248`).        | Restart Cursor / Claude Code; re-run `anvil start --verify`.                   |
| `watching`               | Daemon-backed activation or save-time fallback; pre-write attachment is not in evidence. | `DaemonAttestation::Enforced`, or `WatchTier::Running` with MCP below `RestartRequired` (`diagnostic.rs`). | MCP is optional; wire pre-write MCP if desired, otherwise accept the fallback. |
| `needs_action`           | No literal protection claim; user has actionable next steps.                             | Default branch when no stronger signal applies (`diagnostic.rs:263`).                                      | Read the `next:` repair hint below the diagnostic.                             |
| `unsupported`            | Repo languages are out of scope for this release.                                        | `all_languages_unsupported` and MCP below `RestartRequired` (`diagnostic.rs:257-259`).                     | Wait for the language pack, or scope anvil to a TS / JS subdirectory.          |
| `error`                  | Activation hit a hard error before any state could be established.                       | `last_error.is_some()` or `ConfigStatus::Invalid` (`diagnostic.rs:213-219`).                               | Read `last_error:` and the `next:` repair hint.                                |

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
orchestrator executes seven main ordered steps:

1. **Probe config (and run init if absent).** `verify_with_home` is called first
   (`orchestrator/mod.rs:169`); if `ConfigStatus::Absent`, the orchestrator
   invokes `commands::init::run_in` inline (`orchestrator/mod.rs:170-173`).
   `init::run_in` writes `.anvilrc` and runs the LAUNCH-004 post-init first-scan
   transparently (`commands/init.rs:57-93`). Structurally invalid config
   surfaces as `state: error` via `verify` — the orchestrator does not overwrite
   it.

2. **Install commit/push hook coverage (ACTMO-005).** After project setup,
   activation calls the silent hook installer from `commands::hooks`. It reuses
   the default `anvil hooks install` policy: prefer detected Husky, otherwise
   write Anvil-managed `pre-commit` and `pre-push` files under `.git/hooks/`.
   Existing unmanaged hooks are preserved by the non-force skip semantics.
   Hook-install failure is logged and rendered as a warning, never as an
   activation abort.

3. **Write activation baseline if absent (LAUNCH-010).** When
   `.anvil/baseline.json` is missing, the orchestrator calls
   `services::sample_analyser::run_baseline_scan` and writes the resulting
   fingerprint set via `baseline::write_baseline`
   (`orchestrator/mod.rs:270-281`). Failure to write is logged and swallowed —
   the baseline is a future-change-tracking aid, not a blocker. The schema lives
   in `activation/baseline.rs`; idempotency is enforced by the
   `!baseline::baseline_exists(root)` guard (`orchestrator/mod.rs:271`).

4. **Register the worktree with the intercept daemon (ACTMO-002, gated by
   ACTMO-016 / ADR-094).** After the local config and baseline are present, the
   orchestrator attempts MCP-independent session registration through
   `registration.rs` (`registration::register_worktree_with_daemon`,
   `registration.rs:56-90`). The registration uses a stable activation-owned
   session id derived from the canonical worktree path (canonicalised with
   `dunce`) and the `activation-spine` agent tag, so a live daemon can attest
   the current worktree without waiting for an MCP tool call.

   **Registerable-worktree gate (ACTMO-016 / ADR-094 decision 4).** The
   orchestrator only registers `cwd` when it is a real Git worktree
   (`orchestrator/mod.rs:283-315`). `registration::registerable_worktree`
   (`registration.rs:557-581`) classifies the directory via `git rev-parse`
   (`--is-bare-repository` / `--is-inside-git-dir` / `--show-toplevel`) and
   rejects bare repositories and the `.git` internal directory; ordinary,
   linked, and submodule worktrees are accepted and their canonical top level is
   what gets registered. Outside a registerable worktree — a bare repo, inside
   `.git`, or not a repo at all — `anvil start` stays honest: it does **not**
   seed a junk session keyed to e.g. `$HOME`, logs an info line, and leaves the
   daemon ensured by the caller (exit 0) while `start.rs` surfaces the "no
   worktree registered" guidance. Registration failure (daemon absent, fenced,
   cap-exceeded, or rejected) is logged and non-fatal; a same-worktree
   re-register heartbeats the existing owner (ADR-094 decision 3) rather than
   erroring, and the rendered diagnostic remains the source of truth.

5. **Install MCP entries for Cursor and Claude Code by default (LAUNCH-009 part
   2 / ACTMO-004).** Unless `anvil start --no-mcp` is passed or `ANVIL_NO_MCP`
   is non-empty, the orchestrator resolves `current_exe()`, builds an
   `AnvilEntry::local_stdio` (`mcp_client.rs:91-101`), then calls
   `install::install_for_clients` (`orchestrator/mod.rs`). The install step is
   idempotent (`UpToDate` → skip), refuses to overwrite `UnsafeDrift`, and
   offers `NotPresent` / `SafeDrift` candidates unticked (CIB-184). With MCP
   install skipped, daemon-backed worktree registration still runs and the human
   output prints an explicit skipped-install line. See
   [MCP install (LAUNCH-009)](#mcp-install-launch-009) below.

6. **Re-probe.** A second `verify_with_home` call (`orchestrator/mod.rs:346`)
   absorbs the install side-effects and daemon attestation so the rendered
   diagnostic carries the post-install MCP tier and the spine state. If the
   daemon attests the registered worktree but no MCP client can be promoted to
   `LiveValidation`, ACTMO-003 maps the diagnostic to `state: watching` rather
   than looping on `ready_restart_required`. Any aggregated install failure is
   folded into `diagnostic.last_error` so `protection_state()` collapses to
   `Error` for JSON consumers (`orchestrator/mod.rs:351-353`).

7. **Render.** The CLI renders via
   `activation::render_human_with_install(&diagnostic, &install_report)`
   (`commands/start.rs:158`) or `activation::render_json` under JSON mode
   (`commands/start.rs:153`). The block ends in a single `state: <literal>` line
   plus a per-client `install:` summary (`render.rs:202-237`). The plain ending
   also prints a single UJ-001 next-step line (`start_next_step_line`,
   `commands/start.rs:632-638`): at `LiveValidation` it points to `anvil status`
   (watch would be redundant); when daemon attestation has armed the worktree it
   points to `anvil intercept status`; otherwise it names `anvil watch`. The
   line is suppressed under `--json` and `--verify` so those surfaces stay
   byte-identical (`commands/start.rs:350-354`).

After rendering, if any client install reported `Failed`, the CLI propagates a
non-zero exit so `anvil start && next-step` shell pipelines do not silently
advance (`commands/start.rs:172-179`).

**First-run marker invariant.** `anvil start` does NOT touch `.anvil/first-run`;
that marker is owned exclusively by `anvil welcome`
(`orchestrator/mod.rs:31-33`). The two surfaces never fight for first-run state.

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

## Save-time daemon routing (DSV-021)

Distinct from the LAUNCH-011 in-process hand-off above, `anvil watch` (action
`check`) is a thin client of the resident save-time daemon. The
`ANVIL_WATCH_DAEMON` environment variable selects the posture
(`commands/watch_save_time.rs:101-114`):

| Value                        | Mode                | Behaviour                                                                                                                                                                                                                 |
| ---------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| unset / unrecognised         | `DefaultOnWhenLive` | Route only when a live daemon answers an initial `workspace_status` probe; otherwise behave exactly as the pre-DSV subprocess-only path, no WARN. Presence guard: `build_save_time_client` (`commands/watch.rs:595-617`). |
| `0` / `false` / `off` / `no` | `Disabled`          | Subprocess-only; the daemon is never queried.                                                                                                                                                                             |
| `1` / `true` / `on` / `yes`  | `ForcedOn`          | Route even when no daemon answers; absence folds to the scoped fallback.                                                                                                                                                  |

There is no auto-start — watch never spawns the daemon. A daemon verdict
(`validate_paths`) skips the per-save subprocess entirely (ADR-061 §3). A
daemon-absent start or a mid-session death folds to a `check` scoped to exactly
the changed paths (never `--all`) and surfaces
`workspace_assurance: unavailable{daemon-absent}` — never a truncated `clean`
(`watch_save_time.rs:139-236`; routed at `watch.rs:847-891`). The first fallback
of a disconnect WARNs once and is latched until reconnect
(`watch_save_time.rs:212-223`).

The surface teaches its own recovery (UJ-006): `anvil watch --help` carries a
"Save-time daemon:" block explaining `ANVIL_WATCH_DAEMON`
(`commands/watch.rs:16`), and the daemon-absent fallback prints an ASCII
advisory naming `anvil start` (`fallback_advisory_line`, `watch.rs:431-438`,
emitted at `watch.rs:882`; the JSON channel uses the structured `tracing::warn!`
at `watch.rs:872-876` instead).

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
- **Claude Code permission sidecar (ACTMO-007)** — after a Claude MCP install,
  activation also merges `mcp__anvil__*` into the sibling
  `.claude/settings.json` `permissions.allow` array. Existing allow/deny rules
  are preserved, the rule is idempotent, and an already up-to-date
  `.claude.json` still repairs a missing allow rule.

**Drift policy** (`activation/orchestrator/install.rs:30-41`):

| `DriftClass`  | Interactive      | Non-interactive | Notes                                                               |
| ------------- | ---------------- | --------------- | ------------------------------------------------------------------- |
| `NotPresent`  | offered unticked | auto-install    | fresh write, no merge needed                                        |
| `SafeDrift`   | offered unticked | auto-install    | rewrite over a recognised anvil-shaped entry (likely version drift) |
| `UpToDate`    | not shown        | skip            | nothing to do                                                       |
| `UnsafeDrift` | not shown        | skip with note  | foreign tool / unknown shape — never overwrite                      |

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
  through the link). The Claude settings sidecar uses the same guard before
  writing `.claude/settings.json`.
- Configs that fail to parse are surfaced as `UnsafeDrift` with the parser
  reason; no overwrite, no silent install
  (`activation/orchestrator/install.rs:323-339, 659-677`).

`--verify` mode never enters this module — installs are skipped along with init
and the baseline write (`commands/start.rs:113-120`).

The interactive picker (`demand::MultiSelect`,
`activation/orchestrator/install.rs:500-531`) is gated behind a strict
TTY-and-not-CI check (`orchestrator/mod.rs:629-635`): stdin and stderr must both
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
  plus any aggregated MCP install failure (`orchestrator/mod.rs:351-353`).
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

- `init` is gated by `ConfigStatus::Absent` (`orchestrator/mod.rs:169-173`);
  `orchestrator_skips_init_when_config_valid` (`orchestrator/mod.rs:1120-1146`)
  pins that the `.anvilrc` mtime does not change across re-runs.
- The activation baseline is written only when absent
  (`orchestrator/mod.rs:271`); pinned by
  `orchestrator_baseline_write_is_idempotent` (`orchestrator/mod.rs:1295-1331`).
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

**Save-time posture (UJ-005).** `anvil status` always states the save-time
posture. `gather_save_time` / `classify_save_time`
(`commands/status.rs:668-714`) map the `ANVIL_WATCH_DAEMON` mode ×
daemon-presence matrix onto three postures (`status.rs:642-652`): `Assurance`
(the daemon answered, or `ForcedOn` absence folded to
`unavailable{daemon-absent}`), `Off` (`DefaultOnWhenLive` with no live daemon —
an explicit off-state line naming `anvil start`, not omission), and `Hidden`
(the `=0` opt-out; no line at all). `--json` stays additive — `save_time` is
emitted only for `Assurance` (`status.rs:654-664`).

### Council-locked exclusions

What `anvil start` deliberately does **not** do
(`plans/specs/2026-05-04-launch-a1-execution.md:57-67`):

- No `.cursorrules` / `.clauderules` / global AI rule-file injection.
- No cloud login, team policy pull, or CI setup.
- No `git config` mutation or unmanaged hook overwrite; managed hook
  installation is constrained by the `anvil hooks install` coexistence policy.
- No demo fixtures, challenge files, or guaranteed-catch prompt catalogues.
- No Windsurf, VS Code, Copilot CLI, or Codex CLI MCP install — Cursor + Claude
  Code only.
- No process auto-attach. anvil only knows what it wired itself; it does not
  "find the AI session running in this repo".
- No no-args TUI theatre — `anvil start` is the activation surface;
  `anvil welcome` remains the menu / tutorial surface. Per ADR-080 (UJ-004),
  `anvil welcome` is now the **ungated** beta demo surface — it runs without
  authentication so a new user sees real findings before the licence wall;
  `anvil start` and the rest of `CLI_GATED_COMMANDS`
  (`crates/anvil-cli/src/feature_flags.rs:46-65`) stay gated — the wall sits
  where ongoing value begins.

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

### G-02: Activation watch-tier liveness is unwired (2026-05-07, updated 2026-06-10)

Activation's `WatchTier::Running` is still reachable only through
`anvil start --watch` synthesis (`commands/start.rs:272-273`); the `--verify`
offer-gate is not wired to live process state. Since DSV-021 this gap is
distinct from save-time **daemon** liveness, which is now observable:
`anvil status` queries the daemon's `workspace_status` and renders real
assurance (`commands/status.rs:674`, `watch_save_time.rs:121-137`). The unwired
item is specifically activation's `WatchTier`, not save-time liveness generally.

### G-03: Windows daemon validation reports `not-wired` (2026-05-07)

`LocalDaemonValidationClient::validate_pre_write` is `#[cfg(unix)]`-gated. On
Windows the MCP path still works but the `correlation.daemonStatus` field
returned by `anvil_validate_write` is always `"not-wired"`. See
`docs/architecture/intercept-as-built.md` for the daemon-side detail. Also
surfaced in `docs/archive/runbooks/v0.6.0-beta-release-runbook.md`.

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
  skip copy; `--watch + --verify` / `--watch + --json` rejection; UJ-001
  next-step line (`start_next_step_line`).
- `watch.rs` — `anvil watch` CLI; daemon-routing seam (`build_save_time_client`
  presence guard, the routed dispatch, `fallback_advisory_line`); the "Save-time
  daemon:" `--help` block.
- `watch_save_time.rs` — DSV-021 routing model: `DaemonRoutingMode` /
  `daemon_routing_mode`, `WatchSaveTimeClient`, `SaveTimeDecision`,
  `daemon_absent_assurance`, `query_workspace_status`.
- `status.rs` — UJ-005 save-time posture (`gather_save_time` /
  `classify_save_time`, `SaveTimePosture`).
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
- `plans/decisions/080-ungate-welcome-demo-surface.md` — ADR-080:
  `anvil welcome` is the ungated beta demo surface; `anvil start` stays behind
  the licence gate.
- `docs/runbooks/anvil-no-mcp-activation.md` — operator procedure for
  `anvil start --no-mcp` / `ANVIL_NO_MCP=1` rollouts.
- `docs/public/anvil/guides/wow-start-demo.md` — public-side narrative this doc
  backs up.
- `docs/public/anvil/quickstart.md` — beta quickstart (full 10-minute install +
  activate path).
- `docs/architecture/intercept-as-built.md` — daemon-side intercept detail
  referenced in G-03.
- `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` — operator-facing
  caveats.
- `RELEASE-PLAN.md` — Tier A1 framing.
