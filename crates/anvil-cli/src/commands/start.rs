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
    /// Pick a config file format for first-run activation. When set,
    /// the orchestrator writes `.anvil.<ext>` (yaml / yml / json /
    /// toml) instead of the legacy `.anvilrc`.
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
        // ANVIL_HOME (the orchestrator below emits the read-only-posture note).
        if !project_writes_gated
            && let Err(e) = activation::identity::mint_new_identity(root, env!("CARGO_PKG_VERSION"))
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
    // already-present config. Read-only / `--verify` skips the pre-write
    // (it would mutate state) and falls through to the diagnostic.
    if !read_only
        && !project_writes_gated
        && let Some(format) = args.format
    {
        pre_write_anvil_config(root, format)?;
    }

    let (mut diagnostic, install_report) = if read_only {
        (
            activation::verify(root),
            activation::orchestrator::InstallReport::default(),
        )
    } else {
        activation::orchestrator::run(root, global)?
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
        print!(
            "{}",
            activation::render_human_with_install(&diagnostic, &install_report)
        );
        print!("{}", render_rule_mode_summary(root));
        // MLP2-051g — verbose tier-evidence on stderr. Additive: the
        // stdout block above is byte-identical with or without
        // `--why`, so scripted consumers of `anvil start --verify`
        // (the originating use-case for the flag) are unaffected.
        if args.why {
            eprint!("{}", activation::render_human_verbose(&diagnostic));
        }
        // ADTRUST-006: first-run claim summary + verification recipe.
        // Only emit when activation actually ran (`read_only` is the
        // verify path — that surface already names the state) and
        // when the install side at least reached a renderable
        // protection layer (skip on hard `Error` so the recipe does
        // not race ahead of the cause line).
        if !read_only
            && !matches!(
                diagnostic.protection_state(),
                activation::state::ProtectionState::Error
            )
        {
            print!("{}", render_first_run_recipe(&diagnostic));
        }
        // ADOPT-003 — print the auto-detected AI tool summary after
        // the diagnostic block. Suppressed when nothing was
        // detected (empty render keeps the start output uncluttered
        // for users with no AI tooling installed). When the cache
        // was not written (read-only probe or write failure), the
        // line carries an explicit qualifier so users do not
        // mistake the summary for a successful cache update.
        let summary = activation::detect_agents::render_inventory_summary(&agent_inventory);
        if !summary.is_empty() {
            if agents_cached {
                println!("  {summary}");
            } else {
                println!("  {summary} (not cached)");
            }
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
                "  watch: skipped — project config is invalid; fix the config error first, then re-run `anvil start --watch`."
            );
            Ok(())
        }
        WatchDecision::SkipConfigAbsent => {
            println!(
                "  watch: skipped — no project config found; run `anvil start --format yaml` (or `anvil init`) to adopt Anvil, then re-run `anvil start --watch` for save-time fallback."
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

const RECIPE_LINES: &[&str] = &[
    "    1. echo 'const KEY = \"AKIAEXAMPLE1234567\";' >> .anvil-smoke-test.ts",
    "    2. expect: `anvil status` reports a secret-detection finding in the baseline summary",
    "    3. rm .anvil-smoke-test.ts when done",
];

fn render_first_run_recipe(diag: &activation::ActivationDiagnostic) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    out.push_str("\nverify:\n");
    let _ = writeln!(out, "  state: {}", diag.protection_state().label());
    out.push_str("  active layers:\n");
    if diag.mcp_pre_write_wired_or_live() {
        out.push_str("    - L0 mcp pre-write\n");
    }
    if matches!(diag.watch, activation::diagnostic::WatchTier::Running) {
        out.push_str("    - L2 save-time watch\n");
    }
    // L3/L4 hooks land via `anvil init`; the `anvil start` flow does
    // not install them in v1, so name the deterministic backbone
    // without claiming it is wired. Hook installation status is
    // surfaced separately by `anvil status`.
    out.push_str("    - L3/L4 commit + push hooks (via `anvil init`)\n");
    let _ = writeln!(
        out,
        "  recipe (try this now — triggers `{RECIPE_CHECK_NAME}`):"
    );
    for line in RECIPE_LINES {
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

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
        }
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
        let rendered = render_first_run_recipe(&diag);

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
        for line in RECIPE_LINES {
            assert!(
                rendered.contains(line),
                "recipe missing pinned line: {line:?}\nfull render:\n{rendered}",
            );
        }
    }

    /// Recipe enumerates the layers honestly: a `Protecting` diagnostic
    /// includes the L0 line; a bare `NeedsAction` diagnostic does not.
    #[test]
    fn first_run_recipe_layer_lines_reflect_diagnostic() {
        let protecting = render_first_run_recipe(&synth_diagnostic(
            activation::state::ProtectionState::Protecting,
        ));
        assert!(
            protecting.contains("L0 mcp pre-write"),
            "protecting render must name the active L0 line: {protecting}"
        );

        let needs_action = render_first_run_recipe(&synth_diagnostic(
            activation::state::ProtectionState::NeedsAction,
        ));
        assert!(
            !needs_action.contains("L0 mcp pre-write"),
            "needs_action render must NOT claim L0 is live: {needs_action}"
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
