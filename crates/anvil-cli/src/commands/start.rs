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

use anyhow::{Context, bail};
use clap::Args;

use crate::GlobalArgs;
use crate::activation;
use crate::activation::orchestrator::InstallOutcome;
use crate::commands::watch as watch_cmd;
use crate::config_summary::render_rule_mode_summary;
use crate::warmup_cache::write_watch_warmup_cache;

#[derive(Debug, Args)]
// MLP2-051g — `--why` pushed the bool count past 3. clap-derive arg
// structs are flat by construction (no state-machine refactor is
// possible without breaking the derive macro's contract), so the
// lint's recommended remediation does not apply here.
#[allow(clippy::struct_excessive_bools)]
pub struct StartArgs {
    /// Run a non-mutating activation probe — skip init, first-scan,
    /// and the MCP install step. Produces the same output as
    /// `anvil status --verify`.
    #[arg(long)]
    pub verify: bool,
    /// After activation, run the save-time watch fallback when MCP
    /// cannot pre-write attach. Streams watch events on stdout until
    /// Ctrl-C. An honest fallback — never equivalent to MCP pre-write
    /// validation.
    #[arg(long)]
    pub watch: bool,
    /// Open the opt-in Activation TUI when the session is genuinely interactive.
    ///
    /// First-release rollout flag for ADR-103 / ACTTUI-001. Machine and
    /// non-interactive contracts still win: `--verify`, `--json`, `--watch`,
    /// `--no-tui`, CI, and piped output stay on the plain path.
    #[arg(long)]
    pub tui: bool,
    /// Pick a config file format for first-run activation. When set,
    /// the orchestrator writes `.anvil.<ext>` (yaml / yml / json /
    /// toml) instead of the legacy `.anvilrc`. Incompatible with
    /// `--verify` / `--json` (read-only).
    #[arg(long, value_enum)]
    pub format: Option<StartFormat>,
    /// Mint a fresh project UUID and record the previous one as
    /// `forked_from`. Use after cloning a repo whose
    /// `anvil/project-id` was inherited from the parent. Incompatible
    /// with `--verify` (read-only).
    #[arg(long = "new-identity")]
    pub new_identity: bool,
    /// Print per-tier activation evidence to stderr alongside the
    /// normal verdict on stdout. Most useful when `--verify` stalls
    /// at `ready_restart_required` and you need to see which tier is
    /// the missing piece. Stdout is byte-identical with or without
    /// this flag — scripted consumers of `anvil start --verify` are
    /// unaffected.
    #[arg(long)]
    pub why: bool,
    /// Skip auto-starting the per-user save-time daemon. In an interactive
    /// terminal `anvil start` auto-starts the daemon so save-time validation
    /// is daemon-backed; pass `--no-daemon` (or set `ANVIL_NO_DAEMON=1`) to
    /// suppress that and rely on the scoped fallback. A daemon already
    /// running is still reused; only the auto-start is suppressed. No-op
    /// under `--verify` / `--json` (those read-only probes never start a
    /// daemon). Non-interactive contexts (CI, hooks, piped output) already
    /// fall back automatically.
    #[arg(long = "no-daemon")]
    pub no_daemon: bool,
    /// Skip MCP config installation. The daemon-backed activation spine still
    /// runs; this is for corporate environments where editor MCP integration is
    /// blocked or deliberately disabled. Equivalent to setting `ANVIL_NO_MCP` to
    /// any non-empty value — note that, like `ANVIL_NO_DAEMON`, this is presence-
    /// based: `ANVIL_NO_MCP=0` (or `false`) still ENABLES the opt-out; only
    /// leaving it unset or empty keeps MCP install on.
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
    /// Wire the anvil MCP entry for every supported editor client (Cursor
    /// and Claude Code), even ones not detected on this host. By default
    /// `anvil start` only writes an MCP config for editors it actually
    /// detects (binary on PATH or pre-existing editor state), so it never
    /// creates `~/.cursor/mcp.json` for an editor you do not use. Use this
    /// to pre-wire both editors anyway. Equivalent to setting
    /// `ANVIL_ALL_MCP_CLIENTS` to any non-empty value (presence-based,
    /// like `--no-mcp`). Existing anvil entries are always managed
    /// regardless of this flag.
    #[arg(long = "all-mcp-clients")]
    pub all_mcp_clients: bool,
}

/// MLP2-039 — the format set chosen at adoption time. Maps onto
/// [`anvil_config::ConfigFormat`] when threading into the orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lower")]
pub enum StartFormat {
    Yaml,
    Yml,
    Json,
    Toml,
}

impl StartFormat {
    pub(crate) fn config_format(self) -> anvil_config::ConfigFormat {
        match self {
            Self::Yaml => anvil_config::ConfigFormat::Yaml,
            Self::Yml => anvil_config::ConfigFormat::Yml,
            Self::Json => anvil_config::ConfigFormat::Json,
            Self::Toml => anvil_config::ConfigFormat::Toml,
        }
    }
}

#[allow(clippy::too_many_lines)]
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

    // DISTRIB-006 (ADR-060): a gated ANVIL_HOME (non-default, no
    // `--touch-project-state`) suppresses the same durable per-project writes as
    // read-only mode — the identity mint, the `--format` pre-write, the
    // detected-agents cache, and the warmup cache — so a candidate never persists
    // state the production binary reads. The MCP install and daemon-exercise
    // paths still run (they target the candidate's own home, not project state);
    // the orchestrator emits the single read-only-posture note.
    let project_writes_gated = crate::install_root::project_writes_gated();

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

    // MLP2-033 — `--new-identity` mints a fresh `project_uuid` BEFORE
    // the orchestrator's idempotent `ensure_project_id` runs. The
    // orchestrator step is non-fatal and idempotent on whatever it
    // finds, so pre-minting + handing off is the smallest correct
    // wiring. Read-only / `--verify` rejects the flag explicitly —
    // the mint mutates `anvil/project-id`, which a read-only probe
    // must never do.
    if args.new_identity {
        if read_only {
            bail!(
                "`--new-identity` is incompatible with `--verify` / `--json` (read-only). Drop the read-only flag, or run `anvil baseline --new-identity` for a non-orchestrator surface."
            );
        }
        // DISTRIB-006 (ADR-060): minting a fresh identity overwrites
        // `anvil/project-id`, durable state prod reads — skip it under a gated
        // ANVIL_HOME and tell the operator the flag was a no-op (the orchestrator
        // below also emits the general read-only-posture note).
        if project_writes_gated {
            eprintln!(
                "anvil: --new-identity ignored under a gated ANVIL_HOME — pass \
                 --touch-project-state to rotate the project UUID"
            );
        } else if let Err(e) =
            activation::identity::mint_new_identity(root, env!("CARGO_PKG_VERSION"))
        {
            // Same non-fatal posture as the orchestrator's identity
            // step — surface the failure so the operator sees it,
            // but let the rest of activation proceed (the
            // orchestrator's own `ensure_project_id` will pick up
            // whatever state was left on disk, even if mint failed
            // mid-write).
            tracing::warn!(
                error = %e,
                "start: --new-identity mint failed; orchestrator will fall back to ensure_project_id",
            );
            eprintln!("anvil: --new-identity could not mint a fresh project_uuid ({e})");
        }
    }

    // MLP2-039 — when `--format` is set and no project config exists yet,
    // write `.anvil.<ext>` BEFORE the orchestrator runs so that its init
    // step (which currently writes `.anvilrc`) is suppressed by the
    // already-present config.
    if let Some(format) = args.format {
        // CIB-051: the pre-write creates `.anvil.<ext>`, durable project
        // state a read-only probe must never touch. Reject the
        // combination explicitly — mirroring the `--watch` /
        // `--new-identity` bails above — rather than silently dropping
        // the flag.
        if read_only {
            bail!(
                "`--format` is incompatible with `--verify` / `--json` (read-only) — it writes `.anvil.<ext>` on first run. Drop the read-only flag to adopt the config."
            );
        }
        // DISTRIB-006 (ADR-060): a gated ANVIL_HOME suppresses the durable
        // per-project write; the orchestrator emits the read-only-posture
        // note.
        if !project_writes_gated {
            pre_write_anvil_config(root, format)?;
        }
    }

    // DLIFE-003 (ADR-082): in an interactive session `anvil start` is the
    // activation moment where taking daemon lifecycle responsibility is
    // least surprising, so it auto-starts the per-user save-time daemon —
    // no prompt, because activation IS the consent. Headless / CI / hook /
    // piped contexts fall back deterministically (`NoStart{NonInteractive}`)
    // rather than leave a surprise background daemon behind (ADR-082 §4).
    // `--no-daemon` / `ANVIL_NO_DAEMON` opt out explicitly; the read-only
    // probes (`--verify` / `--json`) never start a daemon
    // (`daemon_capability_for_start` returns `None`). The ensure runs BEFORE
    // the diagnostic so that when a daemon is already live (the common
    // re-run case) the diagnostic's own attestation probe reflects it; a
    // freshly *started* daemon has not yet admitted this worktree, so it can
    // never promote the protection state on its own — the daemon line
    // reports the lifecycle action only and the scoped fallback is preserved
    // on every non-started path.
    let daemon_capability = daemon_capability_for_start(
        start_daemon_opt_out(args),
        read_only,
        start_is_interactive(),
    );
    // The spawn path bound-waits up to ~12s for a fresh daemon to bind. Name
    // the action on stderr before blocking so an interactive `anvil start`
    // does not read as a silent hang. stderr keeps the stdout / `--json`
    // single-document contracts intact (read-only modes never reach here).
    if matches!(
        daemon_capability,
        Some(anvil_intercept::ensure::StartCapability::MaySpawn)
    ) {
        eprintln!("anvil: ensuring the per-user save-time daemon is running…");
    }
    let daemon_outcome = daemon_capability.map(crate::commands::intercept::ensure_save_time_daemon);

    let mcp_policy = mcp_install_policy(args);
    let (mut diagnostic, install_report) = if read_only {
        (
            activation::verify(root),
            activation::orchestrator::InstallReport::default(),
        )
    } else {
        // Both Install and Skip route through the same entry point; the policy
        // is the only thing that differs (Council cleanup — the previous match
        // hid that the Install arm was itself just `run_with_mcp_policy(.., Install)`).
        activation::orchestrator::run_with_mcp_policy(
            root,
            global,
            mcp_policy,
            args.all_mcp_clients,
        )?
    };

    // ADOPT-003 CLI wiring — auto-detect installed AI tools and
    // cache the result for `anvil-run`. Cache write is skipped in
    // read-only modes (`--verify`, `--json`); the in-memory
    // inventory is still computed so the human summary line can
    // describe what was visible without mutating disk. A cache
    // write failure is non-fatal — the diagnostic + install report
    // are the load-bearing surfaces, and degraded behaviour
    // (anvil-run not seeing the cache) is preferable to aborting
    // `anvil start` over an advisory follow-up cache. The
    // `agents_cached` flag annotates the summary line so the
    // user can distinguish "detected and cached" from "detected
    // (probe only)" / "detected (cache not written)".
    let (agent_inventory, agents_cached) =
        run_agent_detection(root, read_only || project_writes_gated);

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
        let human_output = render_start_human_output(
            root,
            read_only,
            &diagnostic,
            &install_report,
            daemon_outcome.as_ref(),
            mcp_policy,
            &agent_inventory,
            agents_cached,
        );
        if activation_tui_eligible(args, global, read_only) {
            let state = anvil_tui::surfaces::activation::ActivationSurface::from_verdict(
                human_output,
                project_writes_gated,
            );
            let _state = crate::tui::run_surface(state)?;
        } else {
            print!("{human_output}");
        }
        // MLP2-051g — verbose tier-evidence on stderr. Additive: the
        // stdout block above is byte-identical with or without
        // `--why`, so scripted consumers of `anvil start --verify`
        // (the originating use-case for the flag) are unaffected. On
        // the opt-in TUI path this prints after the surface exits; ACTTUI-006
        // moves the same evidence into an in-surface LogPanel.
        if args.why {
            eprint!("{}", activation::render_human_verbose(&diagnostic));
        }
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

    write_warmup_cache_if_mutating(root, read_only || project_writes_gated, global.verbose);

    // LAUNCH-011: hand off to the kernel watcher OR print the
    // appropriate skip reason. Each non-spawn variant carries its
    // own copy so the user sees a state-specific explanation, not
    // a generic "watch declined" line.
    match watch_decision {
        WatchDecision::NotRequested => {
            // UJ-001: plain endings name the single next step; JSON and
            // read-only (--verify) surfaces stay byte-identical. CIB-166:
            // when the diagnostic block printed a `next:` repair hint, that
            // hint owns the ending and no closing line prints.
            if !global.json
                && !read_only
                && let Some(line) = ending_next_step_line(&diagnostic)
            {
                println!("{line}");
            }
            Ok(())
        }
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
                "  watch: skipped — project config is invalid; fix the config error first, then re-run `anvil start --watch`."
            );
            Ok(())
        }
        WatchDecision::SkipConfigAbsent => {
            println!(
                "  watch: skipped — no project config found; run `anvil start --format yaml` (or `anvil init`) to adopt anvil, then re-run `anvil start --watch` for save-time fallback."
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

/// MLP2-039 — write `.anvil.<ext>` for the chosen format if no project
/// config already exists. Idempotent — running `anvil start --format yaml`
/// twice on a fresh repo writes the file once, second run is a no-op.
///
/// Returns `Ok(())` even on read-only-friendly bail conditions (the
/// project already has a config) so that the orchestrator continues to
/// run its MCP install + diagnostic probe.
fn pre_write_anvil_config(root: &Path, format: StartFormat) -> anyhow::Result<()> {
    let cfg_format = format.config_format();
    let target = root.join(format!(".anvil.{}", cfg_format.extension()));
    if target.exists() {
        return Ok(());
    }
    // If `.anvilrc` or any OTHER `.anvil.<ext>` already exists, do not
    // double-write — the operator should run `anvil migrate` to convert,
    // not `anvil start --format` to add a second config alongside the
    // first.
    if root.join(".anvilrc").exists() {
        return Ok(());
    }
    if let Some(existing) = anvil_config::discover(root, ".anvil")
        .with_context(|| format!("scanning {} for .anvil.<ext>", root.display()))?
    {
        // A different-format `.anvil.<ext>` is present. Leave it; the
        // orchestrator's normal flow will probe it as the active config.
        tracing::debug!(
            existing = %existing.path.display(),
            requested = ?format,
            "anvil start --format: skipping pre-write; existing .anvil.<ext> present"
        );
        return Ok(());
    }

    // Build the default config value. Mirrors `init::AnvilConfig::default()`
    // shape (schema_version / planning_dir / format / checks) so a project
    // adopted via `--format` reads identically to one adopted via the
    // legacy `.anvilrc` path. Keys are emitted in `schemaVersion` /
    // `planningDir` camelCase across all formats so MLP2-041's
    // `InitConfigView::from_value` reads them without snake-case fallback.
    let value = default_anvil_config_value(format);
    let serialised = serialise_to_format(&value, cfg_format)
        .with_context(|| format!("serialising default config as {}", cfg_format.extension()))?;
    crate::util::atomic_write(&target, serialised.as_bytes())
        .with_context(|| format!("writing {}", target.display()))?;
    Ok(())
}

fn default_anvil_config_value(format: StartFormat) -> serde_json::Value {
    // Hard-coded mirror of `commands::init::AnvilConfig::default()` to
    // avoid leaking the private struct through a new pub surface for a
    // single use site. Update in lock-step if init's defaults change.
    //
    // Council MAJOR (wave 1G review) — the embedded `format` field must
    // match the file extension we are writing, not be hard-coded to
    // `"yaml"`. `init::generate_config_with_force` consumes this field
    // to dispatch its serialiser, so an inconsistent value would
    // silently misroute the writer the moment a consumer migrates to
    // `InitConfigView::from_value`.
    serde_json::json!({
        "schemaVersion": "1.0.0",
        "planningDir": "plans",
        "format": format.config_format().extension(),
        "checks": crate::commands::defaults::default_check_names(),
    })
}

fn serialise_to_format(
    value: &serde_json::Value,
    format: anvil_config::ConfigFormat,
) -> anyhow::Result<String> {
    use anvil_config::ConfigFormat;
    match format {
        ConfigFormat::Yaml | ConfigFormat::Yml => {
            serde_yaml::to_string(value).context("yaml serialisation failed")
        }
        ConfigFormat::Json => {
            let mut s = serde_json::to_string_pretty(value).context("json serialisation failed")?;
            s.push('\n');
            Ok(s)
        }
        ConfigFormat::Toml => toml::to_string_pretty(value).context("toml serialisation failed"),
    }
}

fn write_warmup_cache_if_mutating(root: &Path, read_only: bool, verbose: bool) {
    if read_only {
        return;
    }
    if let Err(error) = write_watch_warmup_cache(root)
        && verbose
    {
        eprintln!("[watch] warm-up cache not written: {error:#}");
    }
}

/// ADOPT-003 — run AI-tool detection and (when not read-only) cache
/// the inventory to `.anvil/cache/detected-agents.json`. Returns
/// `(inventory, cached)`. `cached` is `true` when the cache file
/// reflects the returned inventory; the human renderer uses it to
/// annotate the summary line under `--verify` so users do not
/// mistake a read-only probe for one that updated the cache.
///
/// Council MAJOR fix — detection result is returned unconditionally
/// (in both read-only and mutating modes, and even when the cache
/// write fails). A write failure is surfaced as a non-fatal stderr
/// warning so users without `--verbose` are not blind to it; the
/// summary line still names the live detection.
fn run_agent_detection(
    root: &Path,
    read_only: bool,
) -> (activation::detect_agents::AgentInventory, bool) {
    use activation::detect_agents::{RealDetectionEnv, detect_all, detect_and_cache};
    let env = RealDetectionEnv;
    if read_only {
        return (detect_all(&env), false);
    }
    let outcome = detect_and_cache(root, &env);
    if let Some(error) = outcome.write_error {
        // Surface unconditionally — a silent write failure would
        // misalign the in-memory summary printed below from what
        // `anvil-run` later reads (or fails to read).
        eprintln!("anvil: detected-agents cache not written: {error:#}");
        return (outcome.inventory, false);
    }
    (outcome.inventory, true)
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

// ---------------------------------------------------------------------------
// DLIFE-003: daemon lifecycle wiring for `anvil start`
// ---------------------------------------------------------------------------

/// Decide whether `anvil start` should ensure the per-user save-time
/// daemon this run, and with what capability. Pure so every branch is
/// unit-tested; the caller supplies `opt_out` / `read_only` /
/// `interactive` from the flags, env, and TTY (the ensure primitive's
/// contract: the capability is decided by the caller, not sniffed).
///
/// Precedence (ADR-082):
/// 1. Read-only modes (`--verify` / `--json`) → `None`: those probes are
///    non-mutating and must never start a daemon (module constraint).
/// 2. Explicit opt-out (`--no-daemon` / `ANVIL_NO_DAEMON`) →
///    `NoSpawn(OptOut)`. A daemon already running is still reused; only
///    the *auto-start* is suppressed.
/// 3. Non-interactive context (CI / hook / piped, not a TTY) →
///    `NoSpawn(NonInteractive)`: deterministic fallback, never a surprise
///    background daemon in automation (ADR-082 §4, mirroring `anvil watch`).
/// 4. Interactive session → `MaySpawn`: the activation moment auto-starts
///    with no prompt (settled tiered posture). The already-live and
///    platform-unsupported cases are decided inside the ensure primitive.
fn daemon_capability_for_start(
    opt_out: bool,
    read_only: bool,
    interactive: bool,
) -> Option<anvil_intercept::ensure::StartCapability> {
    use anvil_intercept::ensure::{NoStartReason, StartCapability};
    if read_only {
        return None;
    }
    Some(if opt_out {
        StartCapability::NoSpawn(NoStartReason::OptOut)
    } else if !interactive {
        StartCapability::NoSpawn(NoStartReason::NonInteractive)
    } else {
        StartCapability::MaySpawn
    })
}

/// Whether the operator explicitly opted out of daemon auto-start, via
/// the `--no-daemon` flag or a non-empty `ANVIL_NO_DAEMON` env var (the
/// scriptable/CI-friendly form, set `ANVIL_NO_DAEMON=1`). A daemon that
/// is already running is still reused — this only suppresses spawning a
#[allow(
    clippy::too_many_arguments,
    reason = "start output is composed from the orchestrator's already-separated result objects; this helper preserves the existing plain ordering"
)]
fn render_start_human_output(
    root: &Path,
    read_only: bool,
    diagnostic: &activation::ActivationDiagnostic,
    install_report: &activation::orchestrator::InstallReport,
    daemon_outcome: Option<&anvil_intercept::ensure::EnsureOutcome>,
    mcp_policy: activation::orchestrator::McpInstallPolicy,
    agent_inventory: &activation::detect_agents::AgentInventory,
    agents_cached: bool,
) -> String {
    use std::fmt::Write as _;

    let mut out = activation::render_human_with_install(diagnostic, install_report);
    out.push_str(&render_rule_mode_summary(root));

    // DLIFE-003: report the daemon lifecycle action taken this run (started /
    // reused / opted-out / unsupported / failed). The line is additive and
    // honest — it reports the action, never a protection claim. Absent under
    // read-only modes (`daemon_outcome` is `None`), keeping `--verify`
    // byte-stable.
    if let Some(outcome) = daemon_outcome {
        out.push_str(&render_daemon_lifecycle_line(outcome));
    }

    // ACTMO-016 (ADR-094 decision 4): if cwd is not a registerable Git worktree,
    // the daemon was ensured but nothing was registered — an honest state
    // distinct from `protecting`.
    if !read_only && let Err(reason) = crate::registration::registerable_worktree(root) {
        let _ = writeln!(
            out,
            "  worktree: no worktree registered ({reason}). \
             Run from inside a worktree, or `anvil workspace register <path>`."
        );
    }

    if !read_only && matches!(mcp_policy, activation::orchestrator::McpInstallPolicy::Skip) {
        out.push_str(
            "  install: skipped — MCP config installation disabled (`--no-mcp` / `ANVIL_NO_MCP`)\n",
        );
    }

    // ADTRUST-006: first-run claim summary + verification recipe.
    if !read_only
        && !matches!(
            diagnostic.protection_state(),
            activation::state::ProtectionState::Error
        )
    {
        out.push_str(&render_first_run_recipe(
            diagnostic,
            install_report.hooks_active,
        ));
    }

    // ADOPT-003 — print the auto-detected AI tool summary after the diagnostic
    // block. Suppressed when nothing was detected.
    let summary = activation::detect_agents::render_inventory_summary(agent_inventory);
    if !summary.is_empty() {
        if agents_cached {
            let _ = writeln!(out, "  {summary}");
        } else {
            let _ = writeln!(out, "  {summary} (not cached)");
        }
    }

    out
}

/// Whether the opt-in activation TUI rollout flag is active.
fn activation_tui_requested(args: &StartArgs) -> bool {
    args.tui || std::env::var_os("ANVIL_ACTIVATION_TUI").is_some_and(|value| !value.is_empty())
}

/// Whether this invocation may enter the activation TUI.
///
/// ADR-103 requires the TUI to be additive on the genuinely interactive path
/// only: read-only, JSON, watch fallback, `--no-tui`, CI, and piped output stay
/// on the deterministic plain/machine contracts.
fn activation_tui_eligible(args: &StartArgs, global: &GlobalArgs, read_only: bool) -> bool {
    use std::io::IsTerminal as _;

    activation_tui_requested(args)
        && !read_only
        && !args.watch
        && !global.no_tui
        && !global.json
        && !crate::is_non_interactive_env()
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::io::stderr().is_terminal()
}

/// new one.
fn start_daemon_opt_out(args: &StartArgs) -> bool {
    args.no_daemon || std::env::var_os("ANVIL_NO_DAEMON").is_some_and(|value| !value.is_empty())
}

fn start_mcp_opt_out(args: &StartArgs) -> bool {
    args.no_mcp || std::env::var_os("ANVIL_NO_MCP").is_some_and(|value| !value.is_empty())
}

fn mcp_install_policy(args: &StartArgs) -> activation::orchestrator::McpInstallPolicy {
    if start_mcp_opt_out(args) {
        activation::orchestrator::McpInstallPolicy::Skip
    } else {
        activation::orchestrator::McpInstallPolicy::Install
    }
}

/// Whether `anvil start` has an interactive consent surface for
/// auto-starting the daemon. False in the contexts that must never grow
/// a surprise background daemon: CI / commit hooks / explicit
/// `ANVIL_NO_PROMPT` (via [`crate::is_non_interactive_env`]) or a
/// non-terminal stdout (piped / captured / nohup).
fn start_is_interactive() -> bool {
    use std::io::IsTerminal as _;
    !crate::is_non_interactive_env() && std::io::stdout().is_terminal()
}

/// Render the one-line daemon lifecycle outcome for `anvil start`.
///
/// Honesty contract (module risk): the line reports the lifecycle
/// ACTION only — it never claims protection is active. The protection
/// `state:` line is owned by the activation diagnostic and is not
/// influenced by a freshly started daemon, which has not yet attested
/// this worktree. Every non-started path names the scoped fallback so a
/// user is never left thinking save-time validation silently vanished.
fn render_daemon_lifecycle_line(outcome: &anvil_intercept::ensure::EnsureOutcome) -> String {
    use anvil_intercept::ensure::{EnsureOutcome, NoStartReason};
    let body = match outcome {
        EnsureOutcome::Started => "started the per-user save-time daemon; \
             it attests this worktree once your editor's MCP client connects."
            .to_owned(),
        EnsureOutcome::Reused => {
            "reusing the per-user save-time daemon already running.".to_owned()
        }
        EnsureOutcome::NoStart {
            reason: NoStartReason::OptOut,
        } => "not started (--no-daemon); save-time validation uses the scoped fallback.".to_owned(),
        EnsureOutcome::NoStart {
            reason: NoStartReason::NonInteractive,
        } => "not auto-started (non-interactive: CI, hook, or piped output); \
             run `anvil start` in a terminal to auto-start it, or save-time \
             validation uses the scoped fallback."
            .to_owned(),
        EnsureOutcome::NoStart {
            reason: NoStartReason::PlatformUnsupported,
        } => "background start is not yet available on this platform; \
             save-time validation uses the scoped fallback."
            .to_owned(),
        EnsureOutcome::Failed { recovery } => format!(
            "could not start the daemon — {recovery} \
             Save-time validation uses the scoped fallback until then."
        ),
    };
    format!("  daemon: {body}\n")
}

// ---------------------------------------------------------------------------
// ADTRUST-006: First-run claim summary + verification recipe
// ---------------------------------------------------------------------------
//
// When `anvil start` lands, print a short summary the user can verify
// themselves. Names the current claim state (reusing the closed-set
// vocabulary from `activation::state::ProtectionState`), lists the
// active layers in one line each, and ends with a recipe pointing at
// a real shipping check so reproducing the steps actually produces
// the documented signal (`secret-detection` is the load-bearing
// example because it is the broadest single check in the v0.6.x
// release surface).
//
// The recipe text is intentionally a `const &str` so the contract
// surface and the test fixture stay in lock — a future copy change
// breaks the pinned test before it can ship.

/// Pinned recipe copy referenced by both the user-facing block and
/// the contract test. `RECIPE_CHECK_NAME` names the check the recipe
/// triggers; the test pins both so a typo cannot drift the surface
/// away from a check that actually ships.
const RECIPE_CHECK_NAME: &str = "secret-detection";

const RECIPE_LINE_WRITE: &str =
    "    1. echo 'const KEY = \"AKIAEXAMPLE1234567\";' >> .anvil-smoke-test.ts";
const RECIPE_LINE_EXPECT: &str =
    "    2. expect: `anvil check .anvil-smoke-test.ts` reports a secret-detection finding";

// CIB-172: step 3 removes the throwaway smoke-test file. `rm` is a standard
// utility under `sh` on Unix but is *not* a cmd.exe builtin (`'rm' is not
// recognized`), so Windows must use `del`. Both variants are named and
// compiled on every host so each is directly testable regardless of the build
// target — mirroring the tutorial's `create_policy_directory_command`
// (`crates/anvil-tui/src/surfaces/tutorial/paths.rs`).
const RECIPE_CLEANUP_UNIX: &str = "    3. rm .anvil-smoke-test.ts when done";
const RECIPE_CLEANUP_WINDOWS: &str = "    3. del .anvil-smoke-test.ts when done";

/// The platform-appropriate cleanup step (step 3) for the smoke recipe.
fn recipe_cleanup_line() -> &'static str {
    if cfg!(windows) {
        RECIPE_CLEANUP_WINDOWS
    } else {
        RECIPE_CLEANUP_UNIX
    }
}

/// The full first-run smoke recipe, with the cleanup step selected for the
/// host platform. Steps 1 and 2 are platform-neutral; only step 3 branches.
fn recipe_lines() -> [&'static str; 3] {
    [RECIPE_LINE_WRITE, RECIPE_LINE_EXPECT, recipe_cleanup_line()]
}

/// CIB-166: one next-step arbiter per ending. The diagnostic block's `next:`
/// repair hint (rendered by `activation::render_human`) and the closing
/// `Next:` line used to compute their next steps independently, so a single
/// first-run printout could tell the user to start the intercept daemon and
/// then close with "run `anvil watch`". When the diagnostic printed a repair
/// hint, that hint owns the ending; the closing line prints only when there
/// is nothing to repair.
fn ending_next_step_line(diag: &activation::ActivationDiagnostic) -> Option<&'static str> {
    if activation::has_repair_hint(diag) {
        None
    } else {
        Some(start_next_step_line(diag))
    }
}

/// UJ-001: the single next-step line for a plain `anvil start` ending. Honest
/// about redundancy: when MCP pre-write is live, watch would be a no-op (the
/// `NoOpRedundant` axis), so the next step is the status surface instead.
///
/// CIB-166: gated by [`ending_next_step_line`], and `repair_hint` returns a
/// hint for every state except `Protecting`, so only the live-MCP arm is
/// reachable from `run()` today. The other arms are kept deliberately as the
/// honest fallback should `repair_hint` ever stop being total — a wrong
/// static "protection is live" claim is worse than a redundant branch — and
/// their copy stays pinned by the direct unit tests below.
fn start_next_step_line(diag: &activation::ActivationDiagnostic) -> &'static str {
    // CIB-164: on an all-languages-unsupported repo the closing line must not
    // recommend `anvil watch` — the watcher would produce no findings on
    // out-of-scope files, contradicting the `unsupported` verdict two lines up.
    // This arm takes precedence over the watch fallback but sits below the live
    // MCP / daemon claims (an attested worktree is honestly protected regardless
    // of the smoke-test language coverage).
    if diag.mcp_pre_write_live() {
        "  Next: MCP pre-write protection is live; run `anvil status` to see posture any time."
    } else if diag.daemon_attestation.attests_worktree() && diag.save_time_driver_attached {
        "  Next: daemon-backed save-time validation is armed; run `anvil intercept status` to inspect the daemon."
    } else if diag.daemon_attestation.attests_worktree() {
        "  Next: save-time driver is not attached; run `anvil intercept status` to inspect the daemon."
    } else if diag.all_languages_unsupported {
        "  Next: no supported languages detected here yet — anvil has nothing to validate in this repo, so there is no save-time step to run."
    } else {
        "  Next: run `anvil watch` to validate files as you save."
    }
}

fn render_first_run_recipe(diag: &activation::ActivationDiagnostic, hooks_active: bool) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    out.push_str("\nverify:\n");
    let _ = writeln!(out, "  state: {}", diag.protection_state().label());
    out.push_str("  active layers:\n");
    // CIB-164: L0 is only "active" once an MCP client is producing live
    // validation. At `RestartRequired`/`RestartHandshakeVerified` the entry is
    // wired but explicitly not attached — listing a bare "L0 mcp pre-write"
    // under "active layers" over-claims (the diagnostic block one section up
    // already tells the user to restart). Label the wired-only case as pending
    // so the layer line agrees with the state instead of contradicting it.
    if diag.mcp_pre_write_live() {
        out.push_str("    - L0 mcp pre-write\n");
    } else if diag.mcp_pre_write_wired_or_live() {
        out.push_str("    - L0 mcp pre-write (pending — restart required)\n");
    }
    if diag.daemon_attestation.attests_worktree() && diag.save_time_driver_attached {
        out.push_str("    - L2 daemon-backed save-time\n");
    } else if matches!(diag.watch, activation::diagnostic::WatchTier::Running) {
        out.push_str("    - L2 save-time watch\n");
    }
    // Only claim hook coverage when `anvil start` actually installs hooks —
    // i.e. inside a Git repo with project writes allowed. In a non-Git or
    // write-gated directory the hooks were never written, so listing them would
    // over-claim coverage (Copilot review).
    if hooks_active {
        out.push_str("    - L3/L4 commit + push hooks (via `anvil start`)\n");
    }
    // CIB-164: the `.ts` smoke recipe asserts a `secret-detection` finding on a
    // TypeScript file. On an all-languages-unsupported repo that file is out of
    // scope — running the recipe would produce no finding, contradicting the
    // `unsupported` verdict above. Suppress the recipe and say so honestly
    // rather than hand the user steps that cannot pass.
    if diag.all_languages_unsupported {
        out.push_str(
            "  recipe: none — no supported languages detected in this repo, so the \
             smoke test would report no finding.\n",
        );
        return out;
    }
    let _ = writeln!(
        out,
        "  recipe (try this now — triggers `{RECIPE_CHECK_NAME}`):"
    );
    for line in recipe_lines() {
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn activation_tui_requested_by_flag_or_env() {
        let mut args = start_args_default();
        assert!(!activation_tui_requested(&args));

        args.tui = true;
        assert!(activation_tui_requested(&args));

        args.tui = false;
        temp_env::with_var("ANVIL_ACTIVATION_TUI", Some("1"), || {
            assert!(activation_tui_requested(&args));
        });
    }

    #[test]
    fn activation_tui_empty_env_value_does_not_request_tui() {
        let args = start_args_default();
        temp_env::with_var("ANVIL_ACTIVATION_TUI", Some(""), || {
            assert!(!activation_tui_requested(&args));
        });
    }

    fn synth_diagnostic(
        state_seed: activation::state::ProtectionState,
    ) -> activation::ActivationDiagnostic {
        use activation::diagnostic::{
            ActivationDiagnostic, ConfigStatus, McpClientId, McpTier, WatchTier,
        };

        let mut mcp = BTreeMap::new();
        if matches!(state_seed, activation::state::ProtectionState::Protecting) {
            mcp.insert(McpClientId::ClaudeCode, McpTier::LiveValidation.into());
        }
        let watch = if matches!(state_seed, activation::state::ProtectionState::Watching) {
            WatchTier::Running
        } else {
            WatchTier::NotRequested
        };
        let config = if matches!(state_seed, activation::state::ProtectionState::NeedsAction) {
            ConfigStatus::Absent
        } else {
            ConfigStatus::Valid
        };
        ActivationDiagnostic {
            config,
            mcp,
            watch,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: activation::language_profile::RepoLanguageProfile::default(),
            daemon_attestation: activation::daemon_evidence::DaemonAttestation::NotProbed,
            save_time_driver_attached: false,
        }
    }

    fn daemon_attested_diagnostic() -> activation::ActivationDiagnostic {
        let mut diag = synth_diagnostic(activation::state::ProtectionState::NeedsAction);
        diag.config = activation::diagnostic::ConfigStatus::Valid;
        diag.daemon_attestation = activation::daemon_evidence::DaemonAttestation::Enforced;
        diag
    }

    fn restart_required_diagnostic() -> activation::ActivationDiagnostic {
        use activation::diagnostic::{McpClientId, McpTier};
        let mut diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        diag.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        diag
    }

    // CIB-166: one next-step arbiter per ending — the closing `Next:` line
    // defers to the diagnostic block's `next:` repair hint.

    #[test]
    fn ending_has_exactly_one_next_step_owner() {
        // The closing line prints iff the rendered diagnostic carries no
        // `next:` hint, so an ending never issues two competing instructions
        // (and never zero). The fixture set must reach every
        // `ProtectionState` variant — asserted below — per the CIB-166
        // validation note ("agreement across states").
        let unsupported_diagnostic = || {
            let mut diag = synth_diagnostic(activation::state::ProtectionState::Unsupported);
            diag.all_languages_unsupported = true;
            diag
        };
        let error_diagnostic = || {
            let mut diag = synth_diagnostic(activation::state::ProtectionState::Error);
            diag.last_error = Some("synthetic activation failure".into());
            diag
        };
        // CIB-164 caveat interplay: `ReadyRestartRequired` is reachable with
        // `all_languages_unsupported` (MCP secrets validation is
        // language-agnostic); the repair hint owns that ending too. The old
        // closing "no supported languages" note is intentionally dropped
        // there — the hint's restart guidance never recommends `anvil watch`,
        // so no contradiction remains.
        let restart_required_unsupported = || {
            let mut diag = restart_required_diagnostic();
            diag.all_languages_unsupported = true;
            diag
        };

        let fixtures = [
            synth_diagnostic(activation::state::ProtectionState::Protecting),
            synth_diagnostic(activation::state::ProtectionState::Watching),
            synth_diagnostic(activation::state::ProtectionState::NeedsAction),
            daemon_attested_diagnostic(),
            restart_required_diagnostic(),
            restart_required_unsupported(),
            unsupported_diagnostic(),
            error_diagnostic(),
        ];
        let mut states_seen = std::collections::BTreeSet::new();
        for diag in &fixtures {
            states_seen.insert(format!("{:?}", diag.protection_state()));
            let hint_printed = activation::render_human(diag).contains("\n  next: ");
            let closing = ending_next_step_line(diag);
            assert!(
                hint_printed != closing.is_some(),
                "exactly one surface owns the {:?} ending (diagnostic hint printed: \
                 {hint_printed}, closing line: {closing:?})",
                diag.protection_state(),
            );
        }
        assert_eq!(
            states_seen.len(),
            6,
            "sweep must cover every ProtectionState variant, saw: {states_seen:?}",
        );
    }

    #[test]
    fn closing_watch_line_defers_to_daemon_repair_hint() {
        // The reproduced CIB-166 contradiction: at `ready_restart_required`
        // with the daemon unreachable, the diagnostic says "start the
        // intercept daemon with `anvil intercept start --foreground`" and the
        // ending closed with "Next: run `anvil watch`".
        let mut diag = restart_required_diagnostic();
        diag.daemon_attestation = activation::daemon_evidence::DaemonAttestation::Unreachable;
        assert_eq!(
            diag.protection_state(),
            activation::state::ProtectionState::ReadyRestartRequired,
            "fixture must land on the reproduced state",
        );
        let rendered = activation::render_human(&diag);
        assert!(
            rendered.contains("anvil intercept start --foreground"),
            "diagnostic owns the ending with the daemon repair hint, got: {rendered}",
        );
        assert_eq!(
            ending_next_step_line(&diag),
            None,
            "closing line must defer to the diagnostic's repair hint",
        );
    }

    #[test]
    fn closing_line_owns_ending_when_nothing_to_repair() {
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        assert!(
            !activation::render_human(&diag).contains("\n  next: "),
            "protecting renders no repair hint",
        );
        let line = ending_next_step_line(&diag)
            .expect("with nothing to repair the closing line owns the ending");
        assert!(
            line.contains("anvil status"),
            "protecting ending points at the status surface, got: {line}",
        );
    }

    // UJ-001: a plain `anvil start` ending names the single next step.

    #[test]
    fn next_step_names_watch_when_mcp_not_live() {
        let diag = synth_diagnostic(activation::state::ProtectionState::Watching);
        let line = start_next_step_line(&diag);
        assert!(
            line.contains("anvil watch"),
            "without live MCP the next step is `anvil watch`, got: {line}",
        );
        assert!(
            line.starts_with("  Next:"),
            "a single next-step line, not a banner, got: {line}",
        );
    }

    #[test]
    fn next_step_names_watch_at_restart_required() {
        use activation::diagnostic::{McpClientId, McpTier};
        let mut diag = synth_diagnostic(activation::state::ProtectionState::Watching);
        diag.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        let line = start_next_step_line(&diag);
        assert!(
            line.contains("anvil watch"),
            "at restart_required MCP is wired but not live; claiming live would \
             contradict the restart instruction above, got: {line}",
        );
    }

    #[test]
    fn start_save_time_driver_copy_registered_without_driver_says_not_attached() {
        let diag = daemon_attested_diagnostic();
        let line = start_next_step_line(&diag);
        assert!(
            !line.contains("anvil watch"),
            "registered worktree should not fall back to foreground watch, got: {line}",
        );
        assert!(
            line.contains("save-time driver is not attached"),
            "registered without an attached driver should avoid armed copy, got: {line}",
        );
    }

    #[test]
    fn start_save_time_driver_copy_names_intercept_status_when_driver_attached() {
        let mut diag = daemon_attested_diagnostic();
        diag.save_time_driver_attached = true;
        let line = start_next_step_line(&diag);
        assert!(
            !line.contains("anvil watch"),
            "attached save-time driver is already validating saves; got: {line}",
        );
        assert!(
            line.contains("anvil intercept status"),
            "attached-driver next step should name the daemon status surface, got: {line}",
        );
    }

    #[test]
    fn next_step_names_status_when_mcp_live() {
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        let line = start_next_step_line(&diag);
        assert!(
            !line.contains("anvil watch"),
            "with live MCP pre-write, watch is redundant (NoOpRedundant axis), got: {line}",
        );
        assert!(
            line.contains("anvil status"),
            "with live MCP the next step is checking posture via `anvil status`, got: {line}",
        );
    }

    /// ADOPT-003 — `run_agent_detection` writes the cache when the
    /// command is mutating and reports `cached = true`. The path
    /// under `.anvil/cache` is the same one anvil-run reads from;
    /// if this test breaks the consumer surface breaks with it.
    #[test]
    fn run_agent_detection_writes_cache_in_mutating_mode() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (_inv, cached) = run_agent_detection(tmp.path(), /* read_only = */ false);
        let cache = tmp
            .path()
            .join(".anvil")
            .join("cache")
            .join("detected-agents.json");
        assert!(
            cache.is_file(),
            "mutating start must write the detected-agents cache"
        );
        assert!(cached, "successful mutating run must report cached=true");
    }

    /// ADOPT-003 — read-only modes (`--verify`, `--json`) must
    /// never mutate disk; the cache file stays absent and the
    /// caller is told `cached = false` so the rendered summary can
    /// be annotated. The in-memory inventory is still returned so
    /// the caller can render the summary without committing to a
    /// write.
    #[test]
    fn run_agent_detection_does_not_write_cache_in_read_only_mode() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (_inv, cached) = run_agent_detection(tmp.path(), /* read_only = */ true);
        let cache = tmp
            .path()
            .join(".anvil")
            .join("cache")
            .join("detected-agents.json");
        assert!(
            !cache.exists(),
            "read-only start must not touch the detected-agents cache"
        );
        assert!(
            !cached,
            "read-only probe must not claim the cache was updated"
        );
    }

    /// ADTRUST-006 validation: the first-run recipe names a real
    /// shipping check, embeds the canonical state vocabulary, and
    /// preserves the pinned recipe lines verbatim so the contract
    /// surface cannot drift.
    #[test]
    fn first_run_recipe_matches_fixture() {
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        let rendered = render_first_run_recipe(&diag, true);

        assert!(
            rendered.contains("verify:"),
            "recipe must lead with a verify header: {rendered}"
        );
        assert!(
            rendered.contains("state: protecting"),
            "recipe must name the closed-set state: {rendered}"
        );
        assert!(
            rendered.contains(RECIPE_CHECK_NAME),
            "recipe must reference a real shipping check ({RECIPE_CHECK_NAME}): {rendered}"
        );
        for line in recipe_lines() {
            assert!(
                rendered.contains(line),
                "recipe missing pinned line: {line:?}\nfull render:\n{rendered}",
            );
        }
        assert!(
            !rendered.contains("anvil status") && !rendered.contains("baseline summary"),
            "smoke recipe must use a real scanning surface, not stale status baseline evidence: {rendered}"
        );
    }

    /// Recipe enumerates the layers honestly: a `Protecting` diagnostic
    /// includes the L0 line; a bare `NeedsAction` diagnostic does not.
    #[test]
    fn first_run_recipe_layer_lines_reflect_diagnostic() {
        let protecting = render_first_run_recipe(
            &synth_diagnostic(activation::state::ProtectionState::Protecting),
            true,
        );
        assert!(
            protecting.contains("L0 mcp pre-write"),
            "protecting render must name the active L0 line: {protecting}"
        );

        let needs_action = render_first_run_recipe(
            &synth_diagnostic(activation::state::ProtectionState::NeedsAction),
            true,
        );
        assert!(
            !needs_action.contains("L0 mcp pre-write"),
            "needs_action render must NOT claim L0 is live: {needs_action}"
        );

        let daemon_backed = render_first_run_recipe(&daemon_attested_diagnostic(), true);
        assert!(
            !daemon_backed.contains("L2 daemon-backed save-time"),
            "daemon-attested render without an attached driver must not claim active save-time: {daemon_backed}"
        );
        let mut attached = daemon_attested_diagnostic();
        attached.save_time_driver_attached = true;
        let attached_daemon_backed = render_first_run_recipe(&attached, true);
        assert!(
            attached_daemon_backed.contains("L2 daemon-backed save-time"),
            "daemon-attested render with an attached driver must name the active save-time layer: {attached_daemon_backed}"
        );

        // Hook coverage is only claimed when hooks were actually installed
        // (Git repo + writes allowed). A non-Git / write-gated run must not
        // over-claim L3/L4 hook coverage (Copilot review).
        assert!(
            daemon_backed.contains("L3/L4 commit + push hooks"),
            "hooks_active render must name the hook layer: {daemon_backed}"
        );
        let no_hooks = render_first_run_recipe(&daemon_attested_diagnostic(), false);
        assert!(
            !no_hooks.contains("L3/L4 commit + push hooks"),
            "non-Git / gated render must NOT claim hook coverage: {no_hooks}"
        );
    }

    /// CIB-164 (L0 honesty): a wired-but-not-live MCP client
    /// (`RestartRequired`) is one restart from live, not attached. The
    /// `verify:` block must label it as pending rather than list a bare
    /// active "L0 mcp pre-write" line that contradicts the restart
    /// instruction the diagnostic block prints one section up.
    #[test]
    fn first_run_recipe_marks_wired_but_not_live_mcp_as_pending() {
        use activation::diagnostic::{McpClientId, McpTier};
        let mut diag = synth_diagnostic(activation::state::ProtectionState::NeedsAction);
        diag.config = activation::diagnostic::ConfigStatus::Valid;
        diag.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        let rendered = render_first_run_recipe(&diag, false);
        assert!(
            rendered.contains("L0 mcp pre-write (pending — restart required)"),
            "wired-but-not-live MCP must be labelled pending: {rendered}"
        );
        assert!(
            !rendered.contains("- L0 mcp pre-write\n"),
            "wired-only MCP must NOT print a bare active L0 line: {rendered}"
        );
    }

    /// CIB-164 (unsupported honesty): on an all-languages-unsupported repo
    /// the `.ts` smoke recipe would produce no finding, contradicting the
    /// `unsupported` verdict. The recipe lines must be suppressed and
    /// replaced with an honest "none" note.
    #[test]
    fn first_run_recipe_suppresses_smoke_recipe_when_unsupported() {
        let mut diag = synth_diagnostic(activation::state::ProtectionState::NeedsAction);
        diag.config = activation::diagnostic::ConfigStatus::Valid;
        diag.all_languages_unsupported = true;
        let rendered = render_first_run_recipe(&diag, false);
        assert!(
            rendered.contains("recipe: none"),
            "unsupported render must state the recipe is unavailable: {rendered}"
        );
        for line in recipe_lines() {
            assert!(
                !rendered.contains(line),
                "unsupported render must NOT emit the .ts smoke line {line:?}: {rendered}"
            );
        }
        assert!(
            !rendered.contains("try this now"),
            "unsupported render must not invite the smoke test: {rendered}"
        );
    }

    /// CIB-172: the cleanup step (step 3) is platform-branched because `rm`
    /// is not a cmd.exe builtin (`'rm' is not recognized`). Both variants are
    /// compiled and named, so each is directly testable on any host regardless
    /// of `cfg!(windows)` — mirroring `create_policy_directory_command`.
    #[test]
    fn first_run_recipe_cleanup_is_platform_branched() {
        assert!(
            RECIPE_CLEANUP_WINDOWS.contains("del .anvil-smoke-test.ts"),
            "Windows cleanup step must use `del`: {RECIPE_CLEANUP_WINDOWS:?}"
        );
        assert!(
            !RECIPE_CLEANUP_WINDOWS.contains("rm "),
            "Windows cleanup step must not use `rm` (not a cmd.exe builtin): \
             {RECIPE_CLEANUP_WINDOWS:?}"
        );
        assert!(
            RECIPE_CLEANUP_UNIX.contains("rm .anvil-smoke-test.ts"),
            "Unix cleanup step must use `rm`: {RECIPE_CLEANUP_UNIX:?}"
        );
        // The platform selector returns the host-appropriate variant.
        let selected = recipe_cleanup_line();
        if cfg!(windows) {
            assert_eq!(selected, RECIPE_CLEANUP_WINDOWS);
        } else {
            assert_eq!(selected, RECIPE_CLEANUP_UNIX);
        }
    }

    /// CIB-164 (unsupported honesty): the closing next-step line must stop
    /// recommending `anvil watch` when no supported language is present —
    /// the watcher would report nothing, contradicting the verdict.
    #[test]
    fn next_step_does_not_recommend_watch_when_all_languages_unsupported() {
        let mut diag = synth_diagnostic(activation::state::ProtectionState::NeedsAction);
        diag.all_languages_unsupported = true;
        let line = start_next_step_line(&diag);
        assert!(
            !line.contains("anvil watch"),
            "unsupported repo must not recommend `anvil watch`: {line}"
        );
        assert!(
            line.contains("no supported languages"),
            "unsupported next step must name the coverage gap honestly: {line}"
        );
    }

    // MLP2-039 pre-write tests.

    #[test]
    fn pre_write_yaml_creates_anvil_yaml_with_parseable_content() {
        let tmp = tempfile::TempDir::new().unwrap();
        pre_write_anvil_config(tmp.path(), StartFormat::Yaml).unwrap();

        let path = tmp.path().join(".anvil.yaml");
        assert!(path.is_file());

        // Round-trip through MLP-011's discover + parse_file.
        let discovered = anvil_config::discover(tmp.path(), ".anvil")
            .unwrap()
            .expect("discover must find .anvil.yaml after pre-write");
        assert!(discovered.path.ends_with(".anvil.yaml"));
        let value = anvil_config::parse_file(&discovered.path).unwrap();
        assert!(value.is_object());
        let view = crate::config_view::InitConfigView::from_value(&value)
            .expect("InitConfigView parses the pre-written defaults");
        assert_eq!(view.schema_version, "1.0.0");
        assert_eq!(view.planning_dir, "plans");
    }

    #[test]
    fn pre_write_json_creates_anvil_json_with_parseable_content() {
        let tmp = tempfile::TempDir::new().unwrap();
        pre_write_anvil_config(tmp.path(), StartFormat::Json).unwrap();

        let path = tmp.path().join(".anvil.json");
        assert!(path.is_file());

        let raw = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed["schemaVersion"], "1.0.0");
        // Council MAJOR — embedded `format` field must match the file
        // extension, not be hard-coded to `"yaml"`.
        assert_eq!(parsed["format"], "json");
    }

    #[test]
    fn pre_write_toml_creates_anvil_toml_with_parseable_content() {
        let tmp = tempfile::TempDir::new().unwrap();
        pre_write_anvil_config(tmp.path(), StartFormat::Toml).unwrap();

        let path = tmp.path().join(".anvil.toml");
        assert!(path.is_file());

        let raw = std::fs::read_to_string(&path).unwrap();
        // toml::to_string_pretty preserves camelCase keys from the JSON
        // Value; pinning the line proves the writer didn't accidentally
        // serialise the field name in a way that round-trips wrong.
        assert!(raw.contains("schemaVersion = \"1.0.0\""), "got:\n{raw}");
        // Council MAJOR — embedded `format` field must match the file
        // extension.
        assert!(raw.contains("format = \"toml\""), "got:\n{raw}");
    }

    #[test]
    fn pre_write_yaml_embeds_yaml_format_field() {
        // Companion of the json/toml tests above: yaml must also emit
        // `format: "yaml"` so the embedded field always matches the
        // chosen extension.
        let tmp = tempfile::TempDir::new().unwrap();
        pre_write_anvil_config(tmp.path(), StartFormat::Yaml).unwrap();
        let raw = std::fs::read_to_string(tmp.path().join(".anvil.yaml")).unwrap();
        assert!(
            raw.contains("format: yaml") || raw.contains("format: \"yaml\""),
            "got:\n{raw}"
        );
    }

    #[test]
    fn pre_write_is_idempotent_on_second_call() {
        let tmp = tempfile::TempDir::new().unwrap();
        pre_write_anvil_config(tmp.path(), StartFormat::Yaml).unwrap();
        let path = tmp.path().join(".anvil.yaml");
        let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1100));
        pre_write_anvil_config(tmp.path(), StartFormat::Yaml).unwrap();
        let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();

        assert_eq!(
            mtime_before, mtime_after,
            "pre-write must not rewrite an existing target file"
        );
    }

    #[test]
    fn pre_write_skips_when_legacy_anvilrc_already_present() {
        // The operator opted in to MLP-011 via `--format yaml`, but a
        // legacy `.anvilrc` is already on disk. Refusing to write avoids
        // a duplicate-config-files state; the operator should
        // `anvil migrate` instead.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), r#"{"checks":[]}"#).unwrap();
        pre_write_anvil_config(tmp.path(), StartFormat::Yaml).unwrap();
        assert!(
            !tmp.path().join(".anvil.yaml").exists(),
            "pre-write must not run when .anvilrc is present"
        );
    }

    #[test]
    fn pre_write_skips_when_other_format_anvil_ext_already_present() {
        // `.anvil.toml` exists; running `start --format yaml` must not
        // create a second `.anvil.yaml` — the existing file is the active
        // config.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvil.toml"), "checks = []\n").unwrap();
        pre_write_anvil_config(tmp.path(), StartFormat::Yaml).unwrap();
        assert!(!tmp.path().join(".anvil.yaml").exists());
        // The pre-existing file is untouched.
        let raw = std::fs::read_to_string(tmp.path().join(".anvil.toml")).unwrap();
        assert!(raw.contains("checks"));
    }

    // DLIFE-003 daemon lifecycle wiring tests.

    fn start_args_default() -> StartArgs {
        StartArgs {
            verify: false,
            watch: false,
            tui: false,
            format: None,
            new_identity: false,
            why: false,
            no_daemon: false,
            no_mcp: false,
            all_mcp_clients: false,
        }
    }

    /// `--verify` and `--json` are read-only probes: they must never
    /// start a daemon, so the capability resolver returns `None` and the
    /// `run` flow skips the ensure entirely — regardless of opt-out or
    /// interactivity.
    #[test]
    fn read_only_start_never_ensures_a_daemon() {
        for opt_out in [false, true] {
            for interactive in [false, true] {
                assert!(
                    daemon_capability_for_start(opt_out, /* read_only = */ true, interactive)
                        .is_none(),
                    "read-only start must not start a daemon (opt_out={opt_out}, interactive={interactive})",
                );
            }
        }
    }

    /// An interactive mutating `anvil start` auto-starts the daemon
    /// (ADR-082 tiered posture — no prompt at the activation moment).
    #[test]
    fn interactive_start_may_spawn_the_daemon() {
        use anvil_intercept::ensure::StartCapability;
        assert_eq!(
            daemon_capability_for_start(
                /* opt_out = */ false, /* read_only = */ false,
                /* interactive = */ true,
            ),
            Some(StartCapability::MaySpawn),
        );
    }

    /// Non-interactive contexts (CI / hook / piped) fall back
    /// deterministically rather than leave a surprise background daemon
    /// behind (ADR-082 §4 — owner-confirmed headless posture).
    #[test]
    fn non_interactive_start_falls_back_without_spawning() {
        use anvil_intercept::ensure::{NoStartReason, StartCapability};
        assert_eq!(
            daemon_capability_for_start(
                /* opt_out = */ false, /* read_only = */ false,
                /* interactive = */ false,
            ),
            Some(StartCapability::NoSpawn(NoStartReason::NonInteractive)),
        );
    }

    /// `--no-daemon` / `ANVIL_NO_DAEMON` is the explicit opt-out and wins
    /// over interactivity: the ensure still runs (to reuse a live daemon
    /// honestly) but is told not to spawn.
    #[test]
    fn opt_out_suppresses_spawn_even_when_interactive() {
        use anvil_intercept::ensure::{NoStartReason, StartCapability};
        assert_eq!(
            daemon_capability_for_start(
                /* opt_out = */ true, /* read_only = */ false,
                /* interactive = */ true,
            ),
            Some(StartCapability::NoSpawn(NoStartReason::OptOut)),
        );
    }

    /// The `--no-daemon` flag drives the opt-out predicate. (The
    /// `ANVIL_NO_DAEMON` env arm is exercised end-to-end in the
    /// integration suite to avoid racy process-global env mutation here.)
    #[test]
    fn no_daemon_flag_sets_opt_out() {
        assert!(!start_daemon_opt_out(&start_args_default()));
        let opted_out = StartArgs {
            no_daemon: true,
            ..start_args_default()
        };
        assert!(start_daemon_opt_out(&opted_out));
    }

    #[test]
    fn no_mcp_flag_sets_mcp_install_policy_to_skip() {
        assert_eq!(
            mcp_install_policy(&start_args_default()),
            activation::orchestrator::McpInstallPolicy::Install,
        );
        let opted_out = StartArgs {
            no_mcp: true,
            ..start_args_default()
        };
        assert!(start_mcp_opt_out(&opted_out));
        assert_eq!(
            mcp_install_policy(&opted_out),
            activation::orchestrator::McpInstallPolicy::Skip,
        );
    }

    /// Daemon absent → started. The line reports the action and never
    /// over-claims protection.
    #[test]
    fn lifecycle_line_for_started_is_honest() {
        use anvil_intercept::ensure::EnsureOutcome;
        let line = render_daemon_lifecycle_line(&EnsureOutcome::Started);
        assert!(line.starts_with("  daemon: "), "got: {line}");
        assert!(line.contains("started"), "got: {line}");
        assert!(
            !line.to_lowercase().contains("protect"),
            "the lifecycle line must not claim protection: {line}",
        );
    }

    /// Daemon live → reused; no second daemon, no protection claim.
    #[test]
    fn lifecycle_line_for_reused_names_the_running_daemon() {
        use anvil_intercept::ensure::EnsureOutcome;
        let line = render_daemon_lifecycle_line(&EnsureOutcome::Reused);
        assert!(line.contains("reusing"), "got: {line}");
        assert!(
            !line.to_lowercase().contains("protect"),
            "reuse must not claim protection: {line}",
        );
    }

    /// Ensure failure → the repair hint is surfaced verbatim and the
    /// scoped fallback is named so the user is not left stranded.
    #[test]
    fn lifecycle_line_for_failure_surfaces_recovery_hint() {
        use anvil_intercept::ensure::EnsureOutcome;
        let recovery = "the daemon did not become ready within 12s. See the daemon log at /run/x.";
        let line = render_daemon_lifecycle_line(&EnsureOutcome::Failed {
            recovery: recovery.to_owned(),
        });
        assert!(
            line.contains(recovery),
            "failure line must surface the recovery hint verbatim: {line}",
        );
        assert!(
            line.contains("scoped fallback"),
            "failure must name the preserved fallback: {line}",
        );
    }

    /// Opt-out and platform-unsupported render distinct, honest copy —
    /// a Windows user must not see the opt-out hint (DLIFE-002 typed
    /// `NoStartReason` contract carried through to the surface).
    #[test]
    fn lifecycle_line_distinguishes_opt_out_from_platform_unsupported() {
        use anvil_intercept::ensure::{EnsureOutcome, NoStartReason};
        let opt_out = render_daemon_lifecycle_line(&EnsureOutcome::NoStart {
            reason: NoStartReason::OptOut,
        });
        let unsupported = render_daemon_lifecycle_line(&EnsureOutcome::NoStart {
            reason: NoStartReason::PlatformUnsupported,
        });
        assert!(opt_out.contains("--no-daemon"), "got: {opt_out}");
        assert!(
            !unsupported.contains("--no-daemon"),
            "platform-unsupported must not blame the opt-out flag: {unsupported}",
        );
        assert!(unsupported.contains("platform"), "got: {unsupported}");
        // Both preserve the scoped fallback contract.
        assert!(opt_out.contains("scoped fallback"));
        assert!(unsupported.contains("scoped fallback"));
    }

    /// The non-interactive fallback line names *why* it did not auto-start
    /// (so a CI / hook user understands the deterministic behaviour) and
    /// still preserves the scoped fallback.
    #[test]
    fn lifecycle_line_for_non_interactive_explains_and_preserves_fallback() {
        use anvil_intercept::ensure::{EnsureOutcome, NoStartReason};
        let line = render_daemon_lifecycle_line(&EnsureOutcome::NoStart {
            reason: NoStartReason::NonInteractive,
        });
        assert!(line.contains("non-interactive"), "got: {line}");
        assert!(line.contains("scoped fallback"), "got: {line}");
        // It must not blame the opt-out flag — this is a context, not a
        // deliberate opt-out.
        assert!(!line.contains("--no-daemon"), "got: {line}");
    }

    #[test]
    fn start_format_maps_to_anvil_config_format() {
        // Pin the mapping so a future enum addition cannot silently
        // misroute (e.g., a `Yaml5` variant landing on the JSON writer).
        use anvil_config::ConfigFormat;
        assert_eq!(StartFormat::Yaml.config_format(), ConfigFormat::Yaml);
        assert_eq!(StartFormat::Yml.config_format(), ConfigFormat::Yml);
        assert_eq!(StartFormat::Json.config_format(), ConfigFormat::Json);
        assert_eq!(StartFormat::Toml.config_format(), ConfigFormat::Toml);
    }
}
