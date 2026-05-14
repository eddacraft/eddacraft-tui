//! `anvil start` — activation entrypoint (LAUNCH-006 / LAUNCH-009 / LAUNCH-011).
//!
//! Thin wrapper over `activation::orchestrator`. The orchestration logic
//! lives in the activation module so LAUNCH-009 / LAUNCH-010 / LAUNCH-011
//! can extend the diagnostic probes without touching this command.
//!
//! Behavioural promotion: previously `anvil start` was a clap alias for
//! `anvil welcome` (the menu / tutorial surface). It now drives the
//! activation flow that ends in one literal `ProtectionState`. `anvil
//! welcome` is unchanged and remains the documented menu surface.
//!
//! `anvil start` writes MCP config entries for Cursor and Claude Code to
//! the user's home directory (`~/.cursor/mcp.json`, `~/.claude.json`).
//! Pass `--verify` to skip writes and run a read-only probe instead.
//!
//! ## Watch fallback (LAUNCH-011)
//!
//! When MCP pre-write validation is not live AND the repo is in a
//! state where save-time fallback would actually produce coverage,
//! `anvil start --watch` runs the activation orchestrator and then
//! hands off to the kernel watcher inline, scoped to the current
//! repo. The diagnostic block is rendered first (carrying the
//! explicit "MCP pre-write validation is not attached" note from
//! `activation::render_human` and a synthesised `state: watching`
//! literal so the printed state matches the protection layer about
//! to take over); the watcher then takes over the foreground until
//! Ctrl-C.
//!
//! Honesty contract:
//!
//! - `--watch` synthesises `WatchTier::Running` in the pre-handoff
//!   diagnostic ONLY when the spawn is going to happen this run, so
//!   the printed `state:` literal matches the protection layer about
//!   to enter. The synthesis never claims a tier stronger than the
//!   `protection_state` mapping would already permit at
//!   `WatchTier::Running` (`Watching` or `ReadyRestartRequired` —
//!   never `Protecting`).
//! - When MCP IS at `LiveValidation`, `--watch` is a no-op: pre-write
//!   validation already covers the save path, so spawning the watcher
//!   would only generate redundant fallback noise. The user sees the
//!   explicit "redundant" message; no watcher is spawned.
//! - When the repo is in a state where watch fallback would NOT
//!   produce useful coverage (config invalid / absent, `last_error`
//!   set, all detected languages out of scope), `--watch` skips the
//!   spawn with a state-specific explanation instead of running a
//!   watcher that would generate noise without findings. The user
//!   sees the diagnostic + the skip reason; the actionable next step
//!   (fix config, run init, name the language gap) is the same as
//!   the bare `anvil start` repair hint.
//! - `--watch --verify` is rejected — read-only probes do not spawn
//!   processes, and reporting `state: watching` synthetically would
//!   over-claim.
//! - `--watch --json` is rejected — JSON consumers expect a single
//!   parseable document, but the watcher streams its own event lines.

use std::path::Path;

use anyhow::bail;
use clap::Args;

use crate::GlobalArgs;
use crate::activation;
use crate::activation::orchestrator::InstallOutcome;
use crate::commands::watch as watch_cmd;
use crate::config_summary::render_rule_mode_summary;

#[derive(Debug, Args)]
pub struct StartArgs {
    /// Run a non-mutating activation probe — skip init, first-scan, and
    /// the MCP install step. Forwards to the same backend as `anvil
    /// status --verify` (LAUNCH-012).
    #[arg(long)]
    pub verify: bool,
    /// After activation, run the save-time watch fallback when MCP
    /// cannot pre-write attach. Streams kernel watch events on stdout
    /// until Ctrl-C. Honest fallback only — never claimed equivalent
    /// to MCP pre-write validation. LAUNCH-011.
    #[arg(long)]
    pub watch: bool,
}

pub fn run(args: &StartArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let root = Path::new(".");

    // `--json` implies read-only: init writes its own JSON record to
    // stdout in JSON mode, which would produce two concatenated JSON
    // documents and break parseable consumers. Match `anvil status
    // --verify --json` (LAUNCH-012) — the activation diagnostic is the
    // entire JSON output. Users who want side-effects under JSON
    // should call `anvil init --json` and `anvil start --json`
    // separately.
    let read_only = args.verify || global.json;

    if args.watch {
        // LAUNCH-011: the watch fallback path performs work (spawns
        // the kernel watcher) and streams events on stdout. Both
        // properties are incompatible with the read-only / single-
        // document contracts, so we reject the combination explicitly
        // rather than silently degrading. The user is offered the two
        // honest paths (`--verify` for a read-only probe; `--watch`
        // for the live fallback).
        if args.verify {
            bail!(
                "`--watch` and `--verify` are mutually exclusive — `--verify` is read-only and cannot spawn the watcher. Run `anvil start --watch` to enter watch fallback or `anvil start --verify` to probe state."
            );
        }
        if global.json {
            bail!(
                "`--watch` and `--json` are mutually exclusive — the watcher streams event lines on stdout, breaking the single JSON document contract. Run `anvil start --watch` without `--json`, or `anvil start --json` for a read-only diagnostic."
            );
        }
    }

    let (mut diagnostic, install_report) = if read_only {
        (
            activation::verify(root),
            activation::orchestrator::InstallReport::default(),
        )
    } else {
        activation::orchestrator::run(root, global)?
    };

    // LAUNCH-011: the watch spawn shares the SUPPRESSION axes of the
    // diagnostic's `WatchTier::Offered` gate (config valid + no
    // `last_error` + supported languages) plus an additional
    // `LiveValidation` redundancy check. It deliberately differs from
    // the offer gate on the MCP-tier axis: the offer is suppressed at
    // `RestartRequired+` (the user should restart, not switch to
    // watch), but `--watch` with `RestartRequired` still spawns the
    // watcher as honest belt-and-braces — the user has explicitly
    // asked to layer save-time fallback on top of the restart-pending
    // state. Compute the spawn decision once and reuse it for both
    // the synthesis and the hand-off branch so the two cannot drift.
    let watch_decision = if args.watch {
        WatchDecision::for_diagnostic(&diagnostic)
    } else {
        WatchDecision::NotRequested
    };

    // When the spawn is going to happen this run, synthesise that
    // final state in the diagnostic BEFORE rendering so the printed
    // `state:` line matches the protection layer the user is moments
    // away from running. The synthesis is bounded by the spawn
    // decision: any path that does NOT spawn (`NoOpRedundant`,
    // `SkipConfigInvalid`, `SkipConfigAbsent`, `SkipError`,
    // `SkipNoCoverage`, `NotRequested`) leaves the diagnostic at the
    // orchestrator's reported tier so the user sees the same state
    // they would see without `--watch`, plus the skip reason below.
    if matches!(watch_decision, WatchDecision::Spawn) {
        diagnostic.watch = activation::diagnostic::WatchTier::Running;
    }

    if global.json {
        let json = serde_json::to_string_pretty(&activation::render_json(&diagnostic))?;
        println!("{json}");
    } else {
        print!(
            "{}",
            activation::render_human_with_install(&diagnostic, &install_report)
        );
        print!("{}", render_rule_mode_summary(root));
    }

    // If any client's install step actually failed, propagate as a
    // non-zero exit so CI hooks (`anvil start && next-step`) don't
    // silently advance past a broken activation. The diagnostic +
    // install report already carry the human/JSON detail; this just
    // wires the exit-code contract to match.
    //
    // Skip in --verify (read-only, install never ran) and --json
    // (programmatic consumers should parse `state` and `last_error`
    // from the JSON document; the `last_error` field carries every
    // failure, aggregated).
    if !read_only
        && let Some(err) = install_report.per_client.values().find_map(|o| match o {
            InstallOutcome::Failed { error } => Some(error.as_str()),
            _ => None,
        })
    {
        bail!("MCP install failed: {err}");
    }

    // LAUNCH-011: hand off to the kernel watcher OR print the
    // appropriate skip reason. Each non-spawn variant carries its
    // own copy so the user sees a state-specific explanation, not
    // a generic "watch declined" line.
    match watch_decision {
        WatchDecision::NotRequested => Ok(()),
        WatchDecision::Spawn => {
            // Print the explicit watch hand-off marker so subprocess
            // consumers (and humans reading the stream) see the
            // moment activation stops and the kernel watcher takes
            // over. The language is conservative on purpose — never
            // "fully protected", never "MCP attached".
            println!(
                "  watch: starting save-time fallback — MCP pre-write validation is not attached; this layer validates files after they are saved."
            );
            let watch_args = watch_cmd::WatchArgs::fallback_for_repo();
            watch_cmd::run(&watch_args, global)
        }
        WatchDecision::NoOpRedundant => {
            println!(
                "  watch: skipped — MCP pre-write validation is live; save-time fallback is redundant."
            );
            Ok(())
        }
        WatchDecision::SkipConfigInvalid => {
            println!(
                "  watch: skipped — `.anvilrc` is invalid; fix the config error first, then re-run `anvil start --watch`."
            );
            Ok(())
        }
        WatchDecision::SkipConfigAbsent => {
            println!(
                "  watch: skipped — no `.anvilrc` to honour; run `anvil init` first, then re-run `anvil start --watch` for save-time fallback."
            );
            Ok(())
        }
        WatchDecision::SkipError => {
            println!(
                "  watch: skipped — activation error must be cleared before save-time fallback can run; see `last_error` above."
            );
            Ok(())
        }
        WatchDecision::SkipNoCoverage => {
            println!(
                "  watch: skipped — repo languages are out of scope for the current release; the watcher would not produce findings."
            );
            Ok(())
        }
    }
}

/// What `--watch` should do based on the orchestrator's diagnostic.
///
/// Splitting the decision into a single enum keeps the synthesis
/// branch (`Spawn` → set `WatchTier::Running` before render) and the
/// post-render branch (each variant prints its skip copy or hands
/// off) reading from the same source of truth — they cannot drift
/// silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchDecision {
    /// `--watch` was not passed — neither synthesis nor skip copy.
    NotRequested,
    /// Spawn the kernel watcher inline. Synthesis sets
    /// `WatchTier::Running`; the post-render branch prints the
    /// hand-off marker and enters `watch_cmd::run`.
    Spawn,
    /// MCP is at `LiveValidation` — pre-write covers the save path,
    /// so a watcher would be redundant noise.
    NoOpRedundant,
    /// `.anvilrc` did not parse. The user must fix config first.
    SkipConfigInvalid,
    /// No `.anvilrc` on disk. The user must run `anvil init` first.
    SkipConfigAbsent,
    /// `last_error` is set — activation aborted somewhere upstream.
    SkipError,
    /// All detected languages are out of scope for the current
    /// release; the watcher would not produce findings.
    SkipNoCoverage,
}

impl WatchDecision {
    /// Decide what `--watch` should do given the orchestrator's
    /// diagnostic. The order mirrors the priority that
    /// [`activation::ActivationDiagnostic::protection_state`] uses
    /// so the spawn decision can never produce a message that
    /// contradicts the rendered state:
    ///
    /// 1. `last_error` — if activation aborted upstream, the
    ///    diagnostic state is `Error`; advertising "MCP live,
    ///    fallback redundant" alongside would lie about the state.
    /// 2. `ConfigStatus::Invalid` / `Absent` — the diagnostic state
    ///    is `Error` / `NeedsAction` respectively; the user's
    ///    actionable next step is fixing config or running init.
    /// 3. `mcp_pre_write_live` — only after errors and config are
    ///    cleared can we trust a live MCP claim; the diagnostic
    ///    state is `Protecting`, so spawning a watcher would be
    ///    redundant noise.
    /// 4. `all_languages_unsupported` — the diagnostic state is
    ///    `Unsupported`; watch would produce no findings on
    ///    out-of-scope files.
    /// 5. otherwise → spawn.
    fn for_diagnostic(d: &activation::ActivationDiagnostic) -> Self {
        if d.last_error.is_some() {
            return Self::SkipError;
        }
        match d.config {
            activation::diagnostic::ConfigStatus::Invalid => return Self::SkipConfigInvalid,
            activation::diagnostic::ConfigStatus::Absent => return Self::SkipConfigAbsent,
            activation::diagnostic::ConfigStatus::Valid => {}
        }
        if d.mcp_pre_write_live() {
            return Self::NoOpRedundant;
        }
        if d.all_languages_unsupported {
            return Self::SkipNoCoverage;
        }
        Self::Spawn
    }
}
