//! `anvil start` — activation entrypoint (orchestrator wrapper).

use std::path::Path;

use anyhow::{Context, bail};
use clap::Args;

use crate::GlobalArgs;
use crate::activation;
use crate::activation::agent_registry::{AgentClientId, InstallScope};
use crate::activation::detect_agents::RealDetectionEnv;
use crate::activation::orchestrator::{
    ActivationStep, ActivationStepEvent, ActivationStepLifecycle, InstallOutcome, StartRenderMode,
};
use crate::commands::mcp_installer;
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
    /// Accepted no-op: the Activation TUI is now the default whenever the
    /// session is genuinely interactive.
    ///
    /// Retained so scripts and muscle memory from the opt-in rollout keep
    /// working. Pass `--no-tui` (or set `ANVIL_NO_TUI=1`) to force the plain
    /// path; `--verify`, `--json`, `--watch`, CI, and piped output stay plain
    /// regardless.
    #[arg(long, hide = true)]
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
    /// Non-interactive: wire every supported MCP client even when that client
    /// is not detected on this host. Interactive `anvil start` already *offers*
    /// every registry client (unticked by default); this flag only changes the
    /// headless auto-install path so it writes configs for undetected clients
    /// too. Equivalent to setting `ANVIL_ALL_MCP_CLIENTS` to any non-empty
    /// value (presence-based, like `--no-mcp`). Existing anvil entries are
    /// always managed regardless of this flag.
    #[arg(long = "all-mcp-clients")]
    pub all_mcp_clients: bool,
    /// Explicitly configure one or more MCP clients from the full registry.
    /// Repeat the option to select multiple clients.
    #[arg(long = "mcp-client", value_enum)]
    pub mcp_client: Vec<AgentClientId>,
    /// Scope for clients selected with --mcp-client (and first-wave install).
    /// Global is the default; project scope uses each client's documented
    /// repository path. Clients that only support project scope (for example
    /// VS Code and Zed) appear as project offers on interactive start.
    #[arg(long = "mcp-scope", value_enum, default_value_t = InstallScope::Global)]
    pub mcp_scope: InstallScope,
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
    let render_mode = start_render_mode(args, global, read_only);
    let mut tui_log_lines = Vec::new();

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

    // CIB-224: `--no-mcp` / `ANVIL_NO_MCP` cannot be combined with an explicit
    // client selection — silent ignore left operators unsure whether install
    // was skipped by design. Fail fast with a one-line recovery.
    reject_no_mcp_with_client_selection(args)?;

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
        if matches!(render_mode, StartRenderMode::Tui) {
            // The explicit project-identity offer owns this write.
        } else if project_writes_gated {
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
        if !project_writes_gated && !matches!(render_mode, StartRenderMode::Tui) {
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
    let mut tui_session = None;
    let mut live_surface = None;
    if matches!(render_mode, StartRenderMode::Tui) {
        let mut session = crate::tui::TuiSession::enter()?;
        let surface = anvil_tui::surfaces::activation::ActivationSurface::live(
            project_writes_gated,
            matches!(
                daemon_capability,
                Some(anvil_intercept::ensure::StartCapability::MaySpawn)
            ),
        );
        session.draw_surface(&surface)?;
        tui_session = Some(session);
        live_surface = Some(surface);
    }
    // The spawn path bound-waits up to ~12s for a fresh daemon to bind. Name
    // the action on stderr before blocking so an interactive `anvil start`
    // does not read as a silent hang. stderr keeps the stdout / `--json`
    // single-document contracts intact (read-only modes never reach here).
    if matches!(
        daemon_capability,
        Some(anvil_intercept::ensure::StartCapability::MaySpawn)
    ) {
        let line = "anvil: ensuring the per-user save-time daemon is running…";
        if matches!(render_mode, StartRenderMode::Tui) {
            tui_log_lines.push(line.to_string());
        }
        if !matches!(render_mode, StartRenderMode::Tui) {
            eprintln!("{line}");
        }
    }
    let daemon_outcome = daemon_capability.map(crate::commands::intercept::ensure_save_time_daemon);

    let mcp_policy = mcp_install_policy(args);
    let force_all_mcp_clients = force_all_mcp_clients(args);
    let mut activation_run = None;
    let (mut diagnostic, mut install_report) = if read_only {
        (
            activation::verify(root),
            activation::orchestrator::InstallReport::default(),
        )
    } else {
        // Both Install and Skip route through the same entry point; the policy
        // and render mode are the only things that differ.
        let outcome = if matches!(render_mode, StartRenderMode::Tui) {
            let session = tui_session
                .as_mut()
                .context("activation TUI session was not initialised")?;
            let surface = live_surface
                .as_mut()
                .context("activation live surface was not initialised")?;
            let mut observed_events = Vec::new();
            let mut observer = |event: &ActivationStepEvent| {
                observed_events.push(event.clone());
                let mut live_lines = tui_log_lines.clone();
                live_lines.extend(observed_events.iter().map(ActivationStepEvent::render_line));
                surface
                    .update_live_progress(live_lines, activation_progress_steps(&observed_events));
                session.draw_surface(surface)
            };
            activation::orchestrator::run_with_mcp_policy_and_mode_observing(
                root,
                global,
                orchestrator_mcp_install_policy(args, render_mode),
                force_all_mcp_clients,
                &args.mcp_client,
                render_mode,
                args.new_identity,
                &mut observer,
            )?
        } else {
            activation::orchestrator::run_with_mcp_policy_and_mode(
                root,
                global,
                orchestrator_mcp_install_policy(args, render_mode),
                force_all_mcp_clients,
                &args.mcp_client,
                render_mode,
                args.new_identity,
            )?
        };
        activation_run = Some(outcome.run);
        (outcome.diagnostic, outcome.install_report)
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
    let (agent_inventory, agents_cached) = run_agent_detection(
        root,
        read_only || project_writes_gated || matches!(render_mode, StartRenderMode::Tui),
    );
    if !read_only && !start_mcp_opt_out(args) {
        let first_wave_lines = install_first_wave_mcp_clients(args, render_mode)?;
        for line in &first_wave_lines {
            eprintln!("{line}");
        }
        reconcile_plain_mcp_diagnostic(
            root,
            render_mode,
            args.mcp_scope,
            !first_wave_lines.is_empty(),
            &mut diagnostic,
        );
    }

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

    // CIB-183: a repeat `anvil start` whose evidence says the repo was
    // already activated (config pre-existing, no fresh MCP writes, no
    // errors) collapses the plain success output to the protection state,
    // the daemon/save-time-driver posture, and exactly one next step —
    // instead of reprinting the full first-run recipe. Scoped to the plain
    // mutating path: `--verify` / `--json` are byte-stable machine
    // contracts, `--watch` hands off to the watcher, `--format` /
    // `--new-identity` perform first-run-shaped writes, and the TUI keeps
    // its typed verdict (which derives its next step from the same
    // arbiter — see `arbitrated_next_step`).
    let repeat_collapsed = !read_only
        && !args.watch
        && args.format.is_none()
        && !args.new_identity
        && matches!(render_mode, StartRenderMode::Plain)
        && is_repeat_success(
            activation_run.as_ref(),
            &diagnostic,
            &install_report,
            daemon_outcome.as_ref(),
        );

    if global.json {
        let json = serde_json::to_string_pretty(&activation::render_json(&diagnostic))?;
        println!("{json}");
    } else {
        let human_output = if repeat_collapsed {
            // CIB-190: one bounded local value receipt, pre-rendered here
            // in the command layer (where the wall clock lives) and
            // threaded through the CIB-183 seam. `None` on any miss —
            // absent, stale, or zero evidence, a slow or failing read —
            // and the collapsed output renders exactly as before.
            let value_line = compute_repeat_value_line(root);
            render_repeat_start_output(&diagnostic, daemon_outcome.as_ref(), value_line.as_deref())
        } else {
            render_start_human_output(
                root,
                read_only,
                &diagnostic,
                &install_report,
                daemon_outcome.as_ref(),
                mcp_policy,
                &agent_inventory,
                agents_cached,
            )
        };
        if matches!(render_mode, StartRenderMode::Tui) {
            let consent_plan = activation::orchestrator::build_tui_consent_plan(
                root,
                mcp_policy,
                project_writes_gated,
                args.format.map(StartFormat::config_format),
                args.new_identity,
                activation::orchestrator::RegistryMcpSelection {
                    scope: args.mcp_scope,
                    explicit_clients: &args.mcp_client,
                },
            );
            let mut progress_steps = Vec::new();
            if let Some(run) = &activation_run {
                progress_steps = activation_progress_steps(run.events());
                tui_log_lines.extend(run.events().iter().map(ActivationStepEvent::render_line));
                tui_log_lines.extend(run.log_lines().iter().cloned());
            }
            let phase = if consent_plan.offers().is_empty() {
                anvil_tui::surfaces::activation::ActivationPhase::Verdict
            } else {
                anvil_tui::surfaces::activation::ActivationPhase::Consent
            };
            if matches!(
                phase,
                anvil_tui::surfaces::activation::ActivationPhase::Consent
            ) {
                prepare_consent_progress_steps(&mut progress_steps);
            }
            let verdict_model = activation_verdict_model(
                &diagnostic,
                &install_report,
                consent_plan.settled_mcp(),
                None,
            );
            let tier_evidence = activation_tier_evidence(
                &diagnostic,
                &install_report,
                activation_run.as_ref(),
                matches!(
                    phase,
                    anvil_tui::surfaces::activation::ActivationPhase::Consent
                ),
            );
            // ACTTUI-014 hand-off: do not leave Working progress chrome under Verdict.
            let progress_for_ui = if matches!(
                phase,
                anvil_tui::surfaces::activation::ActivationPhase::Verdict
            ) {
                Vec::new()
            } else {
                progress_steps.clone()
            };
            let mut state =
                anvil_tui::surfaces::activation::ActivationSurface::from_typed_with_progress(
                    human_output,
                    verdict_model,
                    tier_evidence,
                    project_writes_gated,
                    tui_log_lines.clone(),
                    progress_for_ui,
                    false,
                    phase,
                );
            // ACTTUI-016: attach Prove for any surface that may reach Verdict
            // (including after empty Consent apply).
            state = state.with_prove(activation_prove_runner(&diagnostic, root));
            let mut tui_session = tui_session
                .take()
                .context("activation TUI session was not initialised")?;
            if let Some(applied) = run_activation_surface_with(
                state,
                &consent_plan,
                project_writes_gated,
                |mut surface| {
                    let _ = tui_session.run_surface(&mut surface)?;
                    Ok(surface)
                },
            )? {
                for path in &applied.written_workflows {
                    tui_log_lines.push(format!(
                        "installed GitHub Actions workflow {}",
                        path.strip_prefix(root).unwrap_or(path).display(),
                    ));
                }
                for line in &applied.first_wave_mcp_lines {
                    tui_log_lines.push(line.clone());
                }
                for error in &applied.first_wave_mcp_errors {
                    tui_log_lines.push(error.clone());
                }
                if let Some(error) = &applied.workflow_error {
                    tracing::warn!(
                        error = %error,
                        "activation TUI: failed to install selected GitHub Actions workflows",
                    );
                    tui_log_lines.push(format!(
                        "could not install selected GitHub Actions workflows ({error}); continuing"
                    ));
                }
                ensure_tui_load_bearing_actions_succeeded(&applied)?;
                let hooks_active =
                    install_report.hooks_active || applied.install_report.hooks_active;
                install_report = applied.install_report.clone();
                install_report.hooks_active = hooks_active;
                diagnostic = activation::verify(root);
                if let Some(error) = install_report.aggregated_failure() {
                    diagnostic.last_error = Some(format!("MCP install failed: {error}"));
                }
                let post_consent_output = render_start_human_output(
                    root,
                    read_only,
                    &diagnostic,
                    &install_report,
                    daemon_outcome.as_ref(),
                    mcp_policy,
                    &agent_inventory,
                    agents_cached,
                );
                let mut verdict = activation_post_consent_surface(
                    post_consent_output,
                    &diagnostic,
                    &install_report,
                    activation_run.as_ref(),
                    &applied,
                    project_writes_gated,
                    tui_log_lines,
                    false,
                    root,
                    consent_plan.settled_mcp(),
                );
                let _ = tui_session.run_surface(&mut verdict)?;
            }
            tui_session.leave()?;
        } else {
            print!("{human_output}");
        }
        // MLP2-051g — verbose tier-evidence on stderr. Additive: the
        // stdout block above is byte-identical with or without
        // `--why`, so scripted consumers of `anvil start --verify`
        // (the originating use-case for the flag) are unaffected. On
        // the opt-in TUI path routes the same evidence into the in-surface
        // ACTTUI-006 LogPanel instead of leaking a post-exit stderr block.
        if args.why && !matches!(render_mode, StartRenderMode::Tui) {
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

    write_warmup_cache_if_mutating(
        root,
        read_only || project_writes_gated || matches!(render_mode, StartRenderMode::Tui),
        global.verbose,
    );

    // FLEET-003: `start` is the sole remote-beacon emission point. The
    // disclosure is resolved first and only a real terminal can persist
    // `notice_shown`; the detached worker then rechecks every hard off before
    // its bounded request. Read-only/JSON probes never disclose or emit.
    if !read_only {
        crate::telemetry::print_first_run_disclosure(start_is_interactive());
        crate::telemetry::spawn_start_beacon();
    }

    // LAUNCH-011: hand off to the kernel watcher OR print the
    // appropriate skip reason. Each non-spawn variant carries its
    // own copy so the user sees a state-specific explanation, not
    // a generic "watch declined" line.
    match watch_decision {
        WatchDecision::NotRequested => {
            // UJ-001: plain endings name the single next step; JSON and
            // read-only (--verify) surfaces stay byte-identical. CIB-166:
            // when the diagnostic block printed a `next:` repair hint, that
            // hint owns the ending and no closing line prints. CIB-183: the
            // collapsed repeat-success body already carries its single
            // arbitrated next step, so it owns the ending outright.
            if !global.json
                && !read_only
                && !repeat_collapsed
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

fn run_activation_surface_with(
    mut state: anvil_tui::surfaces::activation::ActivationSurface,
    consent_plan: &activation::orchestrator::TuiConsentPlan,
    project_writes_gated: bool,
    runner: impl FnOnce(
        anvil_tui::surfaces::activation::ActivationSurface,
    ) -> anyhow::Result<anvil_tui::surfaces::activation::ActivationSurface>,
) -> anyhow::Result<Option<activation::orchestrator::TuiConsentApplyOutcome>> {
    if !consent_plan.offers().is_empty() {
        state = state.with_consent(activation_consent_state(
            consent_plan.offers(),
            project_writes_gated,
        ));
    }
    let state = runner(state)?;
    Ok(state
        .consent()
        .filter(|consent| consent.submitted())
        .map(|consent| consent_plan.apply(consent.selected_ids())))
}

/// Whether the project's configured checks include `secret-detection`.
///
/// Mirrors planless `anvil check` resolution via
/// [`crate::commands::gate::read_anvilrc_checks`]: no config (or empty `checks`)
/// defaults to the planless set that includes secret-detection; an explicit
/// non-empty list enables Prove only when that list includes the check.
fn secret_detection_enabled_in_project(root: &Path) -> bool {
    match crate::commands::gate::read_anvilrc_checks(root) {
        Ok(None) => true,
        Ok(Some(checks)) => checks.iter().any(|name| {
            crate::commands::check_catalog::canonical_check_name(name) == Some("secret-detection")
                || name == "secret-detection"
        }),
        // Parse/IO failure: fail closed on the check-config gate so we never
        // claim the project's configured pipeline ran when we cannot read it.
        Err(_) => false,
    }
}

/// ACTTUI-016: run the ADTRUST-006 secret fixture through the real secret-detection
/// engine and return toast copy. Claims check-pipeline proof only — never MCP
/// pre-write live status.
fn run_activation_prove(all_languages_unsupported: bool, secret_detection_enabled: bool) -> String {
    use anvil_checks::secret::{SecretCheckConfig, scan_content};

    if all_languages_unsupported {
        return "Prove unavailable: no supported languages in this repo, so secret-detection would report nothing. This does not claim MCP pre-write is live."
            .to_string();
    }
    if !secret_detection_enabled {
        return "Prove unavailable: secret-detection is not enabled in this project config. This does not claim MCP pre-write is live."
            .to_string();
    }

    // Same fixture bytes as the plain-path ADTRUST-006 recipe (and the unit test
    // that pins AWS Key detection). In-memory scan avoids a durable repo write.
    let content = r#"const KEY = "AKIAQRSTUVWXYZ123456";"#;
    let findings = scan_content(
        content,
        ".anvil-smoke-test.ts",
        &SecretCheckConfig::default(),
    );
    if findings.is_empty() {
        "Prove: no secret-detection finding on the fixture — the check pipeline did not catch the known secret shape. This does not claim MCP pre-write is live."
            .to_string()
    } else {
        format!(
            "Prove: secret-detection caught {} finding(s) on the fixture (check pipeline only — not MCP pre-write).",
            findings.len()
        )
    }
}

/// ACTTUI-021: honest refuse for MCP pre-write prove (distinct from check pipeline).
fn mcp_pre_write_prove_note(mcp_live: bool) -> &'static str {
    if mcp_live {
        "MCP pre-write: live (see Layers). Check-pipeline Prove does not re-test the editor."
    } else {
        "MCP pre-write prove unavailable: no live client yet — restart the editor/agent or run `anvil status` (not claimed by check-pipeline Prove)."
    }
}

fn activation_prove_runner(
    diagnostic: &activation::ActivationDiagnostic,
    root: &Path,
) -> anvil_tui::surfaces::activation::ProveRunner {
    let all_unsupported = diagnostic.all_languages_unsupported;
    let secret_enabled = secret_detection_enabled_in_project(root);
    let mcp_live = diagnostic.mcp_pre_write_live();
    std::sync::Arc::new(move || {
        let mut toast = run_activation_prove(all_unsupported, secret_enabled);
        // Always append MCP honesty line (ACTTUI-021 refuse/acknowledge path).
        toast.push(' ');
        toast.push_str(mcp_pre_write_prove_note(mcp_live));
        toast
    })
}

/// Whether this run first established protection for the project, gating the
/// JOURNEY-008 celebration banner.
///
/// Keyed on `project_applied` (the LAUNCH-010 baseline write actually
/// happened), **not** `selected_ids` (the operator merely ticked the box). The
/// baseline write is skipped when there are no analysable files to sample, yet
/// `ProtectionState::Protecting` depends only on reaching live validation — so
/// keying off the tick would re-fire the banner on every activation of such a
/// project. The verdict view additionally gates on `protecting`, so a partial
/// apply that wrote the baseline without reaching protecting shows no banner.
fn baseline_written_this_run(applied: &activation::orchestrator::TuiConsentApplyOutcome) -> bool {
    applied
        .project_applied
        .contains(&ActivationStep::BaselineSample)
}

#[allow(clippy::too_many_arguments)]
fn activation_post_consent_surface(
    human_output: String,
    diagnostic: &activation::ActivationDiagnostic,
    install_report: &activation::orchestrator::InstallReport,
    run: Option<&activation::orchestrator::ActivationRun>,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
    project_writes_gated: bool,
    log_lines: Vec<String>,
    daemon_spinner: bool,
    root: &Path,
    settled_mcp: &[String],
) -> anvil_tui::surfaces::activation::ActivationSurface {
    use anvil_tui::surfaces::activation::{ActivationPhase, ActivationSurface};

    // JOURNEY-008: celebrate only when this run first established protection.
    let first_success = baseline_written_this_run(applied);

    // Verdict phase: drop Working progress rows (ACTTUI-014 hand-off).
    ActivationSurface::from_typed_with_progress(
        human_output,
        activation_verdict_model(diagnostic, install_report, settled_mcp, Some(applied))
            .with_first_success(first_success),
        activation_post_consent_evidence(diagnostic, install_report, run, applied),
        project_writes_gated,
        log_lines,
        Vec::new(),
        daemon_spinner,
        ActivationPhase::Verdict,
    )
    .with_prove(activation_prove_runner(diagnostic, root))
}

#[cfg(test)]
fn activation_post_consent_progress_steps(
    mut steps: Vec<anvil_tui::surfaces::activation::ActivationProgressStep>,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
) -> Vec<anvil_tui::surfaces::activation::ActivationProgressStep> {
    use anvil_tui::surfaces::activation::ActivationProgressStatus;

    for step in &mut steps {
        match step.id.as_str() {
            "workflow-consent" if step.status == ActivationProgressStatus::Pending => {
                let selected = applied
                    .selected_ids
                    .iter()
                    .filter(|id| id.starts_with("workflow:"))
                    .count();
                finish_consent_progress(
                    step,
                    selected,
                    applied.written_workflows.len(),
                    applied.workflow_error.as_deref(),
                    "workflow",
                );
            }
            "mcp-consent" if step.status == ActivationProgressStatus::Pending => {
                let failure = applied.install_report.aggregated_failure();
                let selected = applied
                    .selected_ids
                    .iter()
                    .filter(|id| id.starts_with("mcp:"))
                    .count();
                let installed = applied
                    .install_report
                    .per_client
                    .values()
                    .filter(|outcome| matches!(outcome, InstallOutcome::Installed { .. }))
                    .count();
                finish_consent_progress(step, selected, installed, failure.as_deref(), "MCP");
            }
            "init-config"
                if step.status == ActivationProgressStatus::Pending
                    || applied.selected_ids.contains("project:init-config") =>
            {
                finish_project_progress(
                    step,
                    applied,
                    "project:init-config",
                    ActivationStep::InitConfig,
                );
            }
            "project-identity"
                if step.status == ActivationProgressStatus::Pending
                    || applied.selected_ids.contains("project:identity") =>
            {
                finish_project_progress(
                    step,
                    applied,
                    "project:identity",
                    ActivationStep::ProjectIdentity,
                );
            }
            "witness-attributes"
                if step.status == ActivationProgressStatus::Pending
                    || applied.selected_ids.contains("project:witness-attributes") =>
            {
                finish_project_progress(
                    step,
                    applied,
                    "project:witness-attributes",
                    ActivationStep::WitnessAttributes,
                );
            }
            "git-hooks"
                if step.status == ActivationProgressStatus::Pending
                    || applied.selected_ids.contains("project:git-hooks") =>
            {
                finish_project_progress(
                    step,
                    applied,
                    "project:git-hooks",
                    ActivationStep::GitHooks,
                );
            }
            "baseline-sample"
                if step.status == ActivationProgressStatus::Pending
                    || applied.selected_ids.contains("project:baseline") =>
            {
                finish_project_progress(
                    step,
                    applied,
                    "project:baseline",
                    ActivationStep::BaselineSample,
                );
            }
            "final-probe" => {
                step.status = ActivationProgressStatus::Passed;
                step.message = Some("re-probed after consent".to_string());
            }
            "verdict" => {
                step.status = ActivationProgressStatus::Passed;
                step.message = Some("post-consent verdict".to_string());
            }
            _ => {}
        }
    }
    steps
}

fn prepare_consent_progress_steps(
    steps: &mut [anvil_tui::surfaces::activation::ActivationProgressStep],
) {
    use anvil_tui::surfaces::activation::ActivationProgressStatus;

    for step in steps {
        if matches!(step.id.as_str(), "final-probe" | "verdict") {
            step.status = ActivationProgressStatus::Pending;
            step.message = Some("awaiting consent outcome".to_string());
        }
    }
}

fn ensure_tui_load_bearing_actions_succeeded(
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
) -> anyhow::Result<()> {
    if let Some(error) = applied.project_errors.get(&ActivationStep::InitConfig) {
        anyhow::bail!("init step of `anvil start` failed: {error}");
    }
    if let Some(error) = applied.first_wave_mcp_errors.first() {
        anyhow::bail!("{error}");
    }
    Ok(())
}

#[cfg(test)]
fn finish_consent_progress(
    step: &mut anvil_tui::surfaces::activation::ActivationProgressStep,
    selected: usize,
    applied: usize,
    error: Option<&str>,
    label: &str,
) {
    use anvil_tui::surfaces::activation::ActivationProgressStatus;
    if let Some(error) = error {
        step.status = ActivationProgressStatus::Failed;
        step.message = Some(format!("{label} apply failed: {error}"));
    } else if selected == 0 {
        step.status = ActivationProgressStatus::Skipped;
        step.message = Some("not selected".to_string());
    } else {
        step.status = ActivationProgressStatus::Passed;
        step.message = Some(format!("{selected} selected; {applied} applied"));
    }
}

#[cfg(test)]
fn finish_project_progress(
    step: &mut anvil_tui::surfaces::activation::ActivationProgressStep,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
    offer_id: &str,
    activation_step: ActivationStep,
) {
    use anvil_tui::surfaces::activation::ActivationProgressStatus;
    if !applied.selected_ids.contains(offer_id) {
        step.status = ActivationProgressStatus::Skipped;
        step.message = Some("not selected".to_string());
    } else if let Some(error) = applied.project_errors.get(&activation_step) {
        step.status = ActivationProgressStatus::Failed;
        step.message = Some(error.clone());
    } else if applied.project_applied.contains(&activation_step) {
        step.status = ActivationProgressStatus::Passed;
        step.message = Some("selected write applied".to_string());
    } else if let Some(reason) = applied.project_skipped.get(&activation_step) {
        step.status = ActivationProgressStatus::Skipped;
        step.message = Some(reason.clone());
    }
}

fn activation_consent_state(
    offers: &[activation::orchestrator::TuiConsentOffer],
    project_writes_gated: bool,
) -> anvil_tui::surfaces::activation::ConsentState {
    use activation::orchestrator::TuiConsentOfferKind;
    use anvil_tui::surfaces::activation::{ConsentItem, ConsentKind, ConsentState};

    let items = offers
        .iter()
        .map(|offer| {
            let kind = match offer.kind {
                TuiConsentOfferKind::Mcp => ConsentKind::Mcp,
                TuiConsentOfferKind::Workflow => ConsentKind::Workflow,
                TuiConsentOfferKind::Project => ConsentKind::Project,
                TuiConsentOfferKind::Hooks => ConsentKind::Hooks,
            };
            let mut item = ConsentItem::new(
                offer.id.clone(),
                offer.label.clone(),
                offer.description.clone(),
                kind,
            )
            // CIB-245: the blurb is authored beside the offer, never here, so
            // the plain and TUI paths cannot drift.
            .blurb(offer.blurb.clone());
            if offer.repo_scoped {
                item = item.repo_scoped();
            }
            if let Some(reason) = &offer.unsafe_drift {
                item = item.unsafe_drift(reason.clone());
            }
            item
        })
        .collect();
    ConsentState::new(items, project_writes_gated)
}

/// CIB-244: whether a dual-era install outcome is a **this-run** result the
/// operator asked for, or merely the standing state of a client they did not
/// select. Only the former belongs in the Install list; the rest collapse to a
/// single "unchanged" line so the section reads as "what you installed".
fn is_this_run_install_outcome(outcome: &InstallOutcome) -> bool {
    use activation::orchestrator::SkipReason;
    matches!(
        outcome,
        InstallOutcome::Installed { .. }
            | InstallOutcome::Failed { .. }
            // Refusing to overwrite a foreign entry is a this-run decision the
            // operator must see; the other skips are "nothing happened".
            | InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(_),
            }
    )
}

/// CIB-244: build the verdict Install rows from what this run actually did.
///
/// `applied` is the post-consent apply outcome and carries the registry
/// (`AgentClientId`) clients the operator selected — Grok, Codex, `OpenCode`
/// and friends — which the dual-era `install_report.per_client` map cannot
/// express.
fn activation_install_rows(
    install_report: &activation::orchestrator::InstallReport,
    settled_mcp: &[String],
    applied: Option<&activation::orchestrator::TuiConsentApplyOutcome>,
) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut unchanged = 0usize;
    for (client, outcome) in &install_report.per_client {
        if is_undetected_editor(outcome) {
            continue;
        }
        if is_this_run_install_outcome(outcome) {
            rows.push(format!(
                "{}: {}",
                client.display_name(),
                install_outcome_label(outcome),
            ));
        } else {
            unchanged += 1;
        }
    }
    // Registry clients chosen in consent: first-class rows, named by client.
    if let Some(applied) = applied {
        for row in &applied.registry_installs {
            rows.push(format!("{}: {}", row.display_name, row.label()));
        }
    }
    rows.extend(settled_mcp.iter().cloned());
    if unchanged > 0 {
        rows.push(format!(
            "{unchanged} other detected {} unchanged (see Evidence)",
            if unchanged == 1 { "client" } else { "clients" },
        ));
    }
    if rows.is_empty() {
        rows.push("no MCP or project install actions this run".to_string());
    }
    rows
}

fn activation_verdict_model(
    diagnostic: &activation::ActivationDiagnostic,
    install_report: &activation::orchestrator::InstallReport,
    settled_mcp: &[String],
    applied: Option<&activation::orchestrator::TuiConsentApplyOutcome>,
) -> anvil_tui::surfaces::activation::VerdictModel {
    use anvil_tui::surfaces::activation::{VerdictModel, VerdictSection};

    let state = diagnostic.protection_state();
    let activation_rows = vec![
        format!("state: {}", state.label()),
        // CIB-183: the TUI verdict shares the plain renderers' headline
        // arbitration (incl. the DLIFE-006 daemon-unreachable override)
        // and next-step arbiter — no duplicated copy on either row.
        activation::headline_for_diagnostic(diagnostic).to_string(),
        arbitrated_next_step(diagnostic),
    ];
    let mut layer_rows = vec![format!("config: {}", diagnostic.config.label())];
    layer_rows.extend(diagnostic.mcp.iter().map(|(client, probe)| {
        format!(
            "{} MCP: {} ({})",
            client.display_name(),
            probe.tier.label(),
            probe.transport.label(),
        )
    }));
    // CIB-244: the live MCP probe is still dual-era, so Layers says which
    // clients it covers rather than implying the Install list is unprobed
    // or that unlisted clients failed.
    if !diagnostic.mcp.is_empty() {
        layer_rows.push(format!(
            "MCP probe coverage: {} only — other clients report under Install",
            diagnostic
                .mcp
                .keys()
                .map(|client| client.display_name())
                .collect::<Vec<_>>()
                .join(" and "),
        ));
    }
    layer_rows.push(format!("watch: {}", diagnostic.watch.label()));
    // ACTTUI-019: shared subordinate facts (same strings as `anvil status`).
    let posture = activation::SharedPostureFacts::from_diagnostic(diagnostic);
    layer_rows.extend(posture.fact_lines());
    layer_rows.push(format!(
        "commit/push hooks: {}",
        if install_report.hooks_active {
            "active"
        } else {
            "not installed this run"
        },
    ));

    // ACTTUI-020 / CIB-244: this-run outcomes (dual-era + registry clients)
    // first, then settled (no-write) rows, then one collapsed line for
    // detected clients this run left alone.
    let install_rows = activation_install_rows(install_report, settled_mcp, applied);

    let mut language_rows: Vec<String> = diagnostic
        .language_profile
        .entries
        .iter()
        .map(|entry| {
            format!(
                "{} ({} {}): {} — inventory only; Prove is global, not per-language",
                entry.name,
                entry.files_seen,
                if entry.files_seen == 1 {
                    "file"
                } else {
                    "files"
                },
                entry.coverage_tier.label(),
            )
        })
        .collect();
    if diagnostic.language_profile.unclassified_files_seen > 0 {
        language_rows.push(format!(
            "{} unclassified {}",
            diagnostic.language_profile.unclassified_files_seen,
            if diagnostic.language_profile.unclassified_files_seen == 1 {
                "file"
            } else {
                "files"
            },
        ));
    } else if language_rows.is_empty() && diagnostic.all_languages_unsupported {
        language_rows.push("all detected languages are unsupported in this release".to_string());
    }

    let mut config_rows = vec![format!("project config: {}", diagnostic.config.label())];
    config_rows.push(baseline_label(diagnostic));
    if let Some(error) = &diagnostic.last_error {
        config_rows.push(format!("last error: {error}"));
    }

    VerdictModel::new(
        state.label(),
        format!("Activation state: {}", state.label()),
        vec![
            VerdictSection::new("activation", "Activation", activation_rows),
            VerdictSection::new("layers", "Layers", layer_rows),
            VerdictSection::new("install", "Install", install_rows),
            VerdictSection::new("languages", "Languages", language_rows),
            VerdictSection::new("config", "Config", config_rows),
        ],
    )
}

fn activation_tier_evidence(
    diagnostic: &activation::ActivationDiagnostic,
    install_report: &activation::orchestrator::InstallReport,
    run: Option<&activation::orchestrator::ActivationRun>,
    consent_pending: bool,
) -> Vec<eddacraft_tui::prelude::LogEntry> {
    let mut rows = Vec::new();
    if consent_pending {
        append_pre_consent_lifecycle_evidence(&mut rows, run);
    } else {
        append_lifecycle_evidence(&mut rows, run);
    }
    append_diagnostic_evidence(&mut rows, diagnostic);
    append_install_evidence(&mut rows, install_report);
    rows
}

fn append_pre_consent_lifecycle_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    run: Option<&activation::orchestrator::ActivationRun>,
) {
    if let Some(run) = run {
        for event in run.events().iter().filter(|event| {
            !matches!(
                event.step,
                ActivationStep::FinalProbe | ActivationStep::Verdict
            )
        }) {
            append_lifecycle_event(rows, event);
        }
    }
}

fn activation_post_consent_evidence(
    diagnostic: &activation::ActivationDiagnostic,
    install_report: &activation::orchestrator::InstallReport,
    run: Option<&activation::orchestrator::ActivationRun>,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
) -> Vec<eddacraft_tui::prelude::LogEntry> {
    use eddacraft_tui::prelude::LogLevel;

    let mut rows = Vec::new();
    if let Some(run) = run {
        for event in run.events().iter().filter(|event| {
            event.lifecycle != ActivationStepLifecycle::Deferred
                && !matches!(
                    event.step,
                    ActivationStep::FinalProbe | ActivationStep::Verdict
                )
        }) {
            append_lifecycle_event(&mut rows, event);
        }
    }

    append_project_consent_evidence(&mut rows, run, applied);
    append_workflow_consent_evidence(&mut rows, run, applied);
    append_mcp_consent_evidence(&mut rows, run, applied);
    push_evidence(
        &mut rows,
        LogLevel::Info,
        "completed — re-probed after consent",
        "lifecycle/final-probe",
    );
    append_diagnostic_evidence(&mut rows, diagnostic);
    append_install_evidence(&mut rows, install_report);
    push_evidence(
        &mut rows,
        LogLevel::Info,
        "completed — post-consent verdict",
        "lifecycle/verdict",
    );
    rows
}

fn append_project_consent_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    run: Option<&activation::orchestrator::ActivationRun>,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
) {
    for (step, offer_id) in [
        (ActivationStep::InitConfig, "project:init-config"),
        (ActivationStep::ProjectIdentity, "project:identity"),
        (
            ActivationStep::WitnessAttributes,
            "project:witness-attributes",
        ),
        (ActivationStep::GitHooks, "project:git-hooks"),
        (ActivationStep::BaselineSample, "project:baseline"),
    ] {
        if consent_step_was_deferred(run, step) || applied.selected_ids.contains(offer_id) {
            append_project_apply_evidence(rows, applied, step, offer_id);
        }
    }
}

fn append_workflow_consent_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    run: Option<&activation::orchestrator::ActivationRun>,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
) {
    use eddacraft_tui::prelude::LogLevel;
    if consent_step_was_deferred(run, ActivationStep::WorkflowConsent) {
        let selected = applied
            .selected_ids
            .iter()
            .filter(|id| id.starts_with("workflow:"))
            .count();
        push_evidence(
            rows,
            if applied.workflow_error.is_some() {
                LogLevel::Error
            } else if selected == 0 {
                LogLevel::Debug
            } else {
                LogLevel::Info
            },
            applied.workflow_error.as_ref().map_or_else(
                || {
                    if selected == 0 {
                        "not selected".to_string()
                    } else {
                        format!(
                            "{selected} selected; {} workflow write(s)",
                            applied.written_workflows.len(),
                        )
                    }
                },
                |error| format!("workflow apply failed: {error}"),
            ),
            "consent/workflow",
        );
    }
}

fn append_mcp_consent_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    run: Option<&activation::orchestrator::ActivationRun>,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
) {
    use eddacraft_tui::prelude::LogLevel;
    if consent_step_was_deferred(run, ActivationStep::McpConsent) {
        let mcp_failure = applied.install_report.aggregated_failure();
        let selected = applied
            .selected_ids
            .iter()
            .filter(|id| id.starts_with("mcp:"))
            .count();
        let installed = applied
            .install_report
            .per_client
            .values()
            .filter(|outcome| matches!(outcome, InstallOutcome::Installed { .. }))
            .count();
        push_evidence(
            rows,
            if mcp_failure.is_some() {
                LogLevel::Error
            } else if selected == 0 {
                LogLevel::Debug
            } else {
                LogLevel::Info
            },
            mcp_failure.map_or_else(
                || {
                    if selected == 0 {
                        "not selected".to_string()
                    } else {
                        format!("{selected} selected; {installed} MCP write(s)")
                    }
                },
                |error| format!("MCP apply failed: {error}"),
            ),
            "consent/mcp",
        );
    }
}

fn append_project_apply_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    applied: &activation::orchestrator::TuiConsentApplyOutcome,
    step: ActivationStep,
    offer_id: &str,
) {
    use eddacraft_tui::prelude::LogLevel;
    let source = format!("consent/{}", step.label());
    if !applied.selected_ids.contains(offer_id) {
        push_evidence(rows, LogLevel::Debug, "not selected", source);
    } else if let Some(error) = applied.project_errors.get(&step) {
        push_evidence(rows, LogLevel::Error, error.clone(), source);
    } else if let Some(reason) = applied.project_skipped.get(&step) {
        push_evidence(rows, LogLevel::Debug, reason.clone(), source);
    } else {
        push_evidence(rows, LogLevel::Info, "selected write applied", source);
    }
}

fn consent_step_was_deferred(
    run: Option<&activation::orchestrator::ActivationRun>,
    step: ActivationStep,
) -> bool {
    run.is_none_or(|run| {
        run.events()
            .iter()
            .any(|event| event.step == step && event.lifecycle == ActivationStepLifecycle::Deferred)
    })
}

fn append_lifecycle_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    run: Option<&activation::orchestrator::ActivationRun>,
) {
    use eddacraft_tui::prelude::LogLevel;

    if let Some(run) = run {
        for event in run.events() {
            append_lifecycle_event(rows, event);
        }
        for line in run.log_lines() {
            push_evidence(rows, LogLevel::Info, line.clone(), "orchestrator");
        }
    }
}

fn append_lifecycle_event(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    event: &ActivationStepEvent,
) {
    use eddacraft_tui::prelude::LogLevel;

    let level = match event.lifecycle {
        ActivationStepLifecycle::Failed => LogLevel::Error,
        ActivationStepLifecycle::Deferred | ActivationStepLifecycle::Skipped => LogLevel::Debug,
        ActivationStepLifecycle::Started | ActivationStepLifecycle::Completed => LogLevel::Info,
    };
    let message = event.detail.as_ref().map_or_else(
        || event.lifecycle.label().to_string(),
        |detail| format!("{} — {detail}", event.lifecycle.label()),
    );
    push_evidence(
        rows,
        level,
        message,
        format!("lifecycle/{}", event.step.label()),
    );
}

fn append_diagnostic_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    diagnostic: &activation::ActivationDiagnostic,
) {
    use eddacraft_tui::prelude::LogLevel;

    let state = diagnostic.protection_state();
    let state_level = match state {
        activation::state::ProtectionState::Error => LogLevel::Error,
        activation::state::ProtectionState::NeedsAction
        | activation::state::ProtectionState::ReadyRestartRequired => LogLevel::Warn,
        activation::state::ProtectionState::Unsupported => LogLevel::Debug,
        activation::state::ProtectionState::Protecting
        | activation::state::ProtectionState::Watching => LogLevel::Info,
    };
    push_evidence(
        rows,
        state_level,
        format!("state: {}", state.label()),
        "activation",
    );
    push_evidence(
        rows,
        match diagnostic.config {
            activation::diagnostic::ConfigStatus::Valid => LogLevel::Info,
            activation::diagnostic::ConfigStatus::Absent => LogLevel::Warn,
            activation::diagnostic::ConfigStatus::Invalid => LogLevel::Error,
        },
        format!("config: {}", diagnostic.config.label()),
        "config",
    );
    for (client, probe) in &diagnostic.mcp {
        push_evidence(
            rows,
            mcp_tier_level(probe.tier),
            format!(
                "tier: {}; transport: {}",
                probe.tier.label(),
                probe.transport.label(),
            ),
            format!("mcp/{}", client.label()),
        );
    }
    push_evidence(
        rows,
        LogLevel::Info,
        format!("watch: {}", diagnostic.watch.label()),
        "watch",
    );
    push_evidence(
        rows,
        daemon_attestation_level(diagnostic.daemon_attestation),
        daemon_attestation_label(diagnostic.daemon_attestation),
        "daemon",
    );
    push_evidence(rows, LogLevel::Info, baseline_label(diagnostic), "baseline");
    for entry in &diagnostic.language_profile.entries {
        push_evidence(
            rows,
            match entry.coverage_tier {
                activation::CoverageTier::Supported => LogLevel::Info,
                activation::CoverageTier::Partial => LogLevel::Warn,
                activation::CoverageTier::Unsupported => LogLevel::Debug,
            },
            format!(
                "{}: {} file(s); {} — {}",
                entry.name,
                entry.files_seen,
                entry.coverage_tier.label(),
                entry.basis,
            ),
            "languages",
        );
    }
    if let Some(error) = &diagnostic.last_error {
        push_evidence(rows, LogLevel::Error, error.clone(), "activation/error");
    }
}

fn append_install_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    install_report: &activation::orchestrator::InstallReport,
) {
    for (client, outcome) in &install_report.per_client {
        if is_undetected_editor(outcome) {
            continue;
        }
        push_evidence(
            rows,
            install_outcome_level(outcome),
            install_outcome_label(outcome),
            format!("install/{}", client.label()),
        );
    }
}

fn push_evidence(
    rows: &mut Vec<eddacraft_tui::prelude::LogEntry>,
    level: eddacraft_tui::prelude::LogLevel,
    message: impl Into<String>,
    source: impl Into<String>,
) {
    let index = rows.len();
    rows.push(eddacraft_tui::prelude::LogEntry::new(
        format!("activation-{index:03}"),
        format!("{index:02}"),
        level,
        message,
        source,
    ));
}

fn mcp_tier_level(tier: activation::diagnostic::McpTier) -> eddacraft_tui::prelude::LogLevel {
    use activation::diagnostic::McpTier;
    use eddacraft_tui::prelude::LogLevel;
    match tier {
        McpTier::LiveValidation | McpTier::ServerStartable => LogLevel::Info,
        McpTier::RestartRequired | McpTier::RestartHandshakeVerified => LogLevel::Warn,
        McpTier::NotDetected | McpTier::ConfigAbsent | McpTier::ConfigPresent => LogLevel::Debug,
    }
}

fn daemon_attestation_label(
    attestation: activation::daemon_evidence::DaemonAttestation,
) -> &'static str {
    use activation::daemon_evidence::DaemonAttestation;
    match attestation {
        DaemonAttestation::NotProbed => "not probed",
        DaemonAttestation::Unreachable => "unreachable",
        DaemonAttestation::Unenforced => "worktree not enforced",
        DaemonAttestation::StaleHeartbeat => "stale heartbeat",
        DaemonAttestation::AllSurfacesQuarantined => "all surfaces quarantined",
        DaemonAttestation::Warming => "warming",
        DaemonAttestation::NoParticipatingSurface => "no participating surface",
        DaemonAttestation::Enforced => "worktree enforced",
        DaemonAttestation::Promoted => "live validation promoted",
    }
}

fn daemon_attestation_level(
    attestation: activation::daemon_evidence::DaemonAttestation,
) -> eddacraft_tui::prelude::LogLevel {
    use activation::daemon_evidence::DaemonAttestation;
    use eddacraft_tui::prelude::LogLevel;
    match attestation {
        DaemonAttestation::Unreachable
        | DaemonAttestation::Unenforced
        | DaemonAttestation::StaleHeartbeat
        | DaemonAttestation::AllSurfacesQuarantined
        | DaemonAttestation::NoParticipatingSurface => LogLevel::Warn,
        DaemonAttestation::NotProbed => LogLevel::Debug,
        DaemonAttestation::Warming | DaemonAttestation::Enforced | DaemonAttestation::Promoted => {
            LogLevel::Info
        }
    }
}

fn baseline_label(diagnostic: &activation::ActivationDiagnostic) -> String {
    match (&diagnostic.baseline_summary, diagnostic.baseline_present) {
        (Some(summary), _) => format!(
            "baseline: present ({} total; {} antipattern; {} secret-shaped)",
            summary.total, summary.antipattern, summary.secret,
        ),
        (None, true) => "baseline: present (summary unavailable)".to_string(),
        (None, false) => "baseline: absent".to_string(),
    }
}

fn is_undetected_editor(outcome: &InstallOutcome) -> bool {
    matches!(
        outcome,
        InstallOutcome::Skipped {
            reason: activation::orchestrator::SkipReason::EditorNotDetected,
        }
    )
}

fn install_outcome_label(outcome: &InstallOutcome) -> String {
    use activation::mcp_client::DriftClass;
    use activation::orchestrator::SkipReason;
    match outcome {
        InstallOutcome::Installed { path, drift } => {
            let kind = match drift {
                DriftClass::NotPresent => "fresh",
                DriftClass::UpToDate => "rewrote up-to-date entry",
                DriftClass::SafeDrift { .. } => "rewrote drifted entry",
                DriftClass::UnsafeDrift { .. } => "rewrote unsafe entry",
            };
            format!("installed at {} ({kind})", path.display())
        }
        InstallOutcome::Skipped {
            reason: SkipReason::UserDeselected,
        } => "skipped — not selected".to_string(),
        InstallOutcome::Skipped {
            reason: SkipReason::ConsentDeferredToTui,
        } => "skipped — consent deferred to activation TUI".to_string(),
        InstallOutcome::Skipped {
            reason: SkipReason::EditorNotDetected,
        } => "skipped — editor not detected".to_string(),
        InstallOutcome::Skipped {
            reason: SkipReason::UnsafeDrift(reason),
        } => format!("skipped — refused to overwrite ({reason})"),
        InstallOutcome::Skipped {
            reason: SkipReason::AlreadyUpToDate,
        } => "skipped — already up to date".to_string(),
        InstallOutcome::Failed { error } => format!("failed — {error}"),
    }
}

fn install_outcome_level(outcome: &InstallOutcome) -> eddacraft_tui::prelude::LogLevel {
    use activation::orchestrator::SkipReason;
    use eddacraft_tui::prelude::LogLevel;
    match outcome {
        InstallOutcome::Installed { .. } => LogLevel::Info,
        InstallOutcome::Failed { .. } => LogLevel::Error,
        InstallOutcome::Skipped {
            reason: SkipReason::UnsafeDrift(_),
        } => LogLevel::Warn,
        InstallOutcome::Skipped { .. } => LogLevel::Debug,
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
    pre_write_anvil_config_format(root, format.config_format())
}

/// Path of an existing project config (`.anvilrc` or any `.anvil.<ext>`), if
/// present. Used by CIB-225 to name the path that caused `--format` to be
/// ignored.
fn existing_project_config_path(root: &Path) -> anyhow::Result<Option<std::path::PathBuf>> {
    let legacy = root.join(".anvilrc");
    // Prefer try_exists so permission/IO errors do not silently look like
    // "absent" and allow a second config write (CIB-225 review).
    match legacy.try_exists() {
        Ok(true) => return Ok(Some(legacy)),
        Ok(false) => {}
        Err(err) => {
            return Err(anyhow::Error::new(err).context(format!(
                "checking for existing project config at {}",
                legacy.display()
            )));
        }
    }
    if let Some(existing) = anvil_config::discover(root, ".anvil")
        .with_context(|| format!("scanning {} for .anvil.<ext>", root.display()))?
    {
        return Ok(Some(existing.path));
    }
    Ok(None)
}

/// CIB-225: one-line stderr warning when `--format` is ignored because a
/// project config already exists. Pure so unit tests pin the wording without
/// capturing process stderr.
fn format_ignored_existing_config_warning(existing: &Path) -> String {
    format!(
        "anvil: --format ignored — project config already exists at {}",
        existing.display()
    )
}

/// CIB-223 soft path: one coherent next-step line when cwd is not a
/// registerable Git worktree. Durable init/config may still run; protection
/// cannot attach until the directory is a worktree (or registered).
fn non_registerable_worktree_line(reason: &crate::registration::NotRegisterable) -> String {
    use crate::registration::NotRegisterable;
    match reason {
        NotRegisterable::NotAWorktree(_) => {
            "  worktree: project config may be written here; run `git init` (then re-run `anvil start`) or `anvil workspace register <path>` before protection can attach.\n"
                .to_string()
        }
        NotRegisterable::BareRepository => {
            "  worktree: project config may be written here; bare repositories have no working tree — check out or run from a worktree before protection can attach.\n"
                .to_string()
        }
        NotRegisterable::InsideGitDir => {
            "  worktree: project config may be written here; run from the repository working tree (not inside `.git`) before protection can attach.\n"
                .to_string()
        }
    }
}

pub(crate) fn pre_write_anvil_config_format(
    root: &Path,
    format: anvil_config::ConfigFormat,
) -> anyhow::Result<()> {
    let target = root.join(format!(".anvil.{}", format.extension()));
    if target.exists() {
        // Same format already on disk — idempotent no-op (flag already applied).
        return Ok(());
    }
    // If `.anvilrc` or any OTHER `.anvil.<ext>` already exists, do not
    // double-write — the operator should run `anvil migrate` to convert,
    // not `anvil start --format` to add a second config alongside the
    // first. CIB-225: surface one stderr warning naming the existing path
    // so `--format` is never a silent no-op when another config wins.
    if let Some(existing) = existing_project_config_path(root)? {
        let warning = format_ignored_existing_config_warning(&existing);
        eprintln!("{warning}");
        tracing::debug!(
            existing = %existing.display(),
            requested = ?format,
            "anvil start --format: skipping pre-write; existing project config present"
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
    let serialised = serialise_to_format(&value, format)
        .with_context(|| format!("serialising default config as {}", format.extension()))?;
    crate::util::atomic_write(&target, serialised.as_bytes())
        .with_context(|| format!("writing {}", target.display()))?;
    Ok(())
}

fn default_anvil_config_value(format: anvil_config::ConfigFormat) -> serde_json::Value {
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
        "format": format.extension(),
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

fn activation_progress_steps(
    events: &[ActivationStepEvent],
) -> Vec<anvil_tui::surfaces::activation::ActivationProgressStep> {
    const ORDER: [ActivationStep; 11] = [
        ActivationStep::InitialProbe,
        ActivationStep::InitConfig,
        ActivationStep::ProjectIdentity,
        ActivationStep::WitnessAttributes,
        ActivationStep::GitHooks,
        ActivationStep::BaselineSample,
        ActivationStep::WorktreeRegistration,
        ActivationStep::WorkflowConsent,
        ActivationStep::McpConsent,
        ActivationStep::FinalProbe,
        ActivationStep::Verdict,
    ];

    ORDER
        .into_iter()
        .filter_map(|step| progress_step_from_events(step, events))
        .collect()
}

fn progress_step_from_events(
    step: ActivationStep,
    events: &[ActivationStepEvent],
) -> Option<anvil_tui::surfaces::activation::ActivationProgressStep> {
    use anvil_tui::surfaces::activation::{ActivationProgressStatus, ActivationProgressStep};

    let step_events: Vec<&ActivationStepEvent> =
        events.iter().filter(|event| event.step == step).collect();
    if step_events.is_empty() {
        return None;
    }
    let status = if step_events
        .iter()
        .any(|event| event.lifecycle == ActivationStepLifecycle::Failed)
    {
        ActivationProgressStatus::Failed
    } else if step_events
        .iter()
        .any(|event| event.lifecycle == ActivationStepLifecycle::Started)
        && !step_events.iter().any(|event| {
            matches!(
                event.lifecycle,
                ActivationStepLifecycle::Completed
                    | ActivationStepLifecycle::Deferred
                    | ActivationStepLifecycle::Skipped
            )
        })
    {
        ActivationProgressStatus::Running
    } else if step_events
        .iter()
        .any(|event| event.lifecycle == ActivationStepLifecycle::Completed)
    {
        ActivationProgressStatus::Passed
    } else if step_events
        .iter()
        .any(|event| event.lifecycle == ActivationStepLifecycle::Deferred)
    {
        ActivationProgressStatus::Pending
    } else if step_events
        .iter()
        .any(|event| event.lifecycle == ActivationStepLifecycle::Skipped)
    {
        ActivationProgressStatus::Skipped
    } else {
        ActivationProgressStatus::Pending
    };

    let mut row = ActivationProgressStep::new(step.label(), progress_label(step), status);
    if let Some(detail) = step_events
        .iter()
        .rev()
        .find_map(|event| event.detail.as_deref())
    {
        row = row.with_message(detail.to_string());
    }
    Some(row)
}

fn progress_label(step: ActivationStep) -> &'static str {
    match step {
        ActivationStep::InitialProbe => "Initial probe",
        ActivationStep::InitConfig => "Project config",
        ActivationStep::ProjectIdentity => "Project identity",
        ActivationStep::WitnessAttributes => "Witness attributes",
        ActivationStep::GitHooks => "Git hooks",
        ActivationStep::BaselineSample => "Baseline sample",
        ActivationStep::WorktreeRegistration => "Worktree registration",
        ActivationStep::WorkflowConsent => "Workflow consent",
        ActivationStep::McpConsent => "MCP consent",
        ActivationStep::FinalProbe => "Final probe",
        ActivationStep::Verdict => "Verdict",
    }
}

/// Compose the existing plain `anvil start` output into one string so the
/// opt-in TUI can render the same verdict text without changing the plain path.
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

    // ACTMO-016 (ADR-094 decision 4) + CIB-223 soft path: if cwd is not a
    // registerable Git worktree, the daemon was ensured but nothing was
    // registered. Config init is still allowed; frame one coherent next step
    // (not success-then-contradiction).
    if !read_only && let Err(reason) = crate::registration::registerable_worktree(root) {
        out.push_str(&non_registerable_worktree_line(&reason));
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

/// Whether the caller explicitly forced the plain path through the activation
/// TUI-specific environment escape hatch.
fn activation_tui_env_opt_out() -> bool {
    std::env::var_os("ANVIL_NO_TUI").is_some_and(|value| !value.is_empty())
}

/// Whether the invocation itself permits the activation TUI, ignoring the
/// terminal probe.
///
/// Split from [`activation_tui_eligible`] so the decision is testable without a
/// PTY: this half is pure argument and environment policy, the other half asks
/// the OS whether the three stdio handles are really a terminal.
fn activation_tui_allowed(args: &StartArgs, global: &GlobalArgs, read_only: bool) -> bool {
    !read_only
        && !args.watch
        && !global.no_tui
        && !activation_tui_env_opt_out()
        && !global.json
        && !crate::is_non_interactive_env()
}

/// Whether this invocation may enter the activation TUI.
///
/// ADR-103 makes the TUI the default on the genuinely interactive path, so no
/// opt-in is consulted. The trust boundary is unchanged: read-only, JSON, the
/// watch fallback, `--no-tui` / `ANVIL_NO_TUI`, CI, and piped output all stay on
/// the deterministic plain/machine contracts.
fn activation_tui_eligible(args: &StartArgs, global: &GlobalArgs, read_only: bool) -> bool {
    use std::io::IsTerminal as _;

    activation_tui_allowed(args, global, read_only)
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::io::stderr().is_terminal()
}

fn start_render_mode(args: &StartArgs, global: &GlobalArgs, read_only: bool) -> StartRenderMode {
    if activation_tui_eligible(args, global, read_only) {
        StartRenderMode::Tui
    } else {
        StartRenderMode::Plain
    }
}

/// Whether the operator explicitly opted out of daemon auto-start, via the
/// `--no-daemon` flag or a non-empty `ANVIL_NO_DAEMON` env var (the
/// scriptable/CI-friendly form, set `ANVIL_NO_DAEMON=1`). A daemon that is
/// already running is still reused — this only suppresses spawning a new one.
fn start_daemon_opt_out(args: &StartArgs) -> bool {
    args.no_daemon || std::env::var_os("ANVIL_NO_DAEMON").is_some_and(|value| !value.is_empty())
}

fn start_mcp_opt_out(args: &StartArgs) -> bool {
    args.no_mcp || std::env::var_os("ANVIL_NO_MCP").is_some_and(|value| !value.is_empty())
}

/// CIB-224: `--no-mcp` / `ANVIL_NO_MCP` is mutually exclusive with explicit
/// MCP client selection (`--mcp-client`, `--all-mcp-clients`,
/// `ANVIL_ALL_MCP_CLIENTS`). Fail with a one-line recovery rather than
/// silently ignoring the selection.
fn reject_no_mcp_with_client_selection(args: &StartArgs) -> anyhow::Result<()> {
    if !start_mcp_opt_out(args) {
        return Ok(());
    }
    if !args.mcp_client.is_empty() {
        bail!(
            "`--no-mcp` / `ANVIL_NO_MCP` and `--mcp-client` are mutually exclusive — drop `--no-mcp` or unset `ANVIL_NO_MCP` to install the selected client(s), or drop `--mcp-client` to skip MCP install."
        );
    }
    if force_all_mcp_clients(args) {
        bail!(
            "`--no-mcp` / `ANVIL_NO_MCP` and `--all-mcp-clients` / `ANVIL_ALL_MCP_CLIENTS` are mutually exclusive — drop `--no-mcp` or unset `ANVIL_NO_MCP` to install all clients, or drop the all-clients selection to skip MCP install."
        );
    }
    Ok(())
}

fn mcp_install_policy(args: &StartArgs) -> activation::orchestrator::McpInstallPolicy {
    if start_mcp_opt_out(args) {
        activation::orchestrator::McpInstallPolicy::Skip
    } else {
        activation::orchestrator::McpInstallPolicy::Install
    }
}

/// The pre-registry Claude/Cursor installer always chooses an existing config
/// or its global fallback. Project-scoped activation therefore routes every
/// client through the shared scope-aware installer instead.
///
/// Plain start still uses this so the legacy global installer is not invoked
/// under `--mcp-scope project` (first-wave / registry install runs separately).
/// Interactive TUI must **not** use this `Skip` path: that policy surfaces
/// "MCP installation disabled" and never defers to the consent picker
/// (CIB-220). Use [`orchestrator_mcp_install_policy`] at the orchestrator call
/// sites.
fn legacy_mcp_install_policy(args: &StartArgs) -> activation::orchestrator::McpInstallPolicy {
    if args.mcp_scope == InstallScope::Project {
        activation::orchestrator::McpInstallPolicy::Skip
    } else {
        mcp_install_policy(args)
    }
}

/// MCP policy handed to the activation orchestrator for this render mode.
///
/// - **TUI:** always the real opt-out policy ([`mcp_install_policy`]). Project
///   scope must `Install` so `McpConsent` defers to the interactive picker
///   rather than claiming installation is disabled (CIB-220).
/// - **Plain:** keep the legacy project-scope `Skip` so the pre-registry
///   global installer is not invoked; plain project installs go through
///   [`install_first_wave_mcp_clients`].
fn orchestrator_mcp_install_policy(
    args: &StartArgs,
    render_mode: StartRenderMode,
) -> activation::orchestrator::McpInstallPolicy {
    if matches!(render_mode, StartRenderMode::Tui) {
        mcp_install_policy(args)
    } else {
        legacy_mcp_install_policy(args)
    }
}

fn reconcile_plain_mcp_diagnostic(
    root: &Path,
    render_mode: StartRenderMode,
    scope: InstallScope,
    install_attempted: bool,
    diagnostic: &mut activation::ActivationDiagnostic,
) {
    if matches!(render_mode, StartRenderMode::Plain)
        && scope == InstallScope::Project
        && install_attempted
    {
        *diagnostic = activation::verify(root);
    }
}

fn install_first_wave_mcp_clients(
    args: &StartArgs,
    render_mode: StartRenderMode,
) -> anyhow::Result<Vec<String>> {
    let home = crate::util::user_home_dir();
    let project = std::env::current_dir().context("resolving project directory")?;
    let command = std::env::current_exe().context("resolving anvil executable")?;
    install_first_wave_mcp_clients_at(args, render_mode, home.as_deref(), &project, &command)
}

fn install_first_wave_mcp_clients_at(
    args: &StartArgs,
    render_mode: StartRenderMode,
    home: Option<&Path>,
    project: &Path,
    command: &Path,
) -> anyhow::Result<Vec<String>> {
    if matches!(render_mode, StartRenderMode::Tui) {
        return Ok(Vec::new());
    }
    let explicit = !args.mcp_client.is_empty();
    let force_all = force_all_mcp_clients(args);
    let env = RealDetectionEnv;
    let clients = if explicit {
        args.mcp_client.clone()
    } else if force_all {
        AgentClientId::all()
            .iter()
            .filter(|entry| entry.supports_mcp(args.mcp_scope))
            .map(|entry| entry.id)
            .collect()
    } else {
        AgentClientId::all()
            .iter()
            .filter(|entry| {
                entry.supports_mcp(args.mcp_scope)
                    && entry.detected_for_mcp(&env, args.mcp_scope, project)
                    && !(args.mcp_scope == InstallScope::Global
                        && matches!(entry.id, AgentClientId::ClaudeCode | AgentClientId::Cursor))
            })
            .map(|entry| entry.id)
            .collect()
    };

    if clients.is_empty() {
        return Ok(Vec::new());
    }
    let root = match args.mcp_scope {
        InstallScope::Global => home.context("could not determine home directory")?,
        InstallScope::Project => project,
    };
    let command = command
        .to_str()
        .context("anvil executable path is not valid UTF-8")?;
    let mut lines = Vec::new();
    for client in clients {
        if args.mcp_scope == InstallScope::Global
            && matches!(client, AgentClientId::ClaudeCode | AgentClientId::Cursor)
        {
            continue;
        }
        if !client.entry().supports_mcp(args.mcp_scope) {
            if explicit {
                bail!(
                    "{} does not support {}-scope MCP installation",
                    client.entry().display_name,
                    args.mcp_scope.label()
                );
            }
            continue;
        }
        match mcp_installer::install(client, args.mcp_scope, root, command, false, false) {
            Ok(report) => lines.push(format!(
                "anvil: {} MCP config {} at {}; restart guidance: {}",
                client.entry().display_name,
                if report.wrote {
                    "installed"
                } else {
                    "already configured"
                },
                report.path.display(),
                report.reload_hint
            )),
            Err(error) if !explicit => lines.push(format!(
                "anvil: skipped {} MCP config: {error:#}",
                client.entry().display_name
            )),
            Err(error) => return Err(error),
        }
    }
    Ok(lines)
}

fn force_all_mcp_clients(args: &StartArgs) -> bool {
    args.all_mcp_clients
        || std::env::var_os("ANVIL_ALL_MCP_CLIENTS").is_some_and(|value| !value.is_empty())
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
    "    1. echo 'const KEY = \"AKIAQRSTUVWXYZ123456\";' >> .anvil-smoke-test.ts";
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

/// CIB-183: the single arbitrated next-step row, shared verbatim by the
/// collapsed repeat renderer and the TUI verdict model. Reuses the
/// CIB-162/CIB-166 arbitration: when the diagnostic carries a repair hint
/// that hint owns the ending; otherwise the UJ-001 closing line does.
/// Returned without leading indentation so each surface applies its own.
fn arbitrated_next_step(diag: &activation::ActivationDiagnostic) -> String {
    match activation::repair_hint_for(diag) {
        Some(hint) => format!("next: {hint}"),
        None => start_next_step_line(diag).trim_start().to_string(),
    }
}

/// CIB-183: honest, evidence-based detection of a repeat `anvil start`
/// that ended in success (or with the single clear restart step). Every
/// axis is derived from what this run actually observed and did — never
/// a timestamp guess:
///
/// 1. The init step recorded that the project config already existed
///    before this run started
///    ([`activation::orchestrator::ActivationRun::config_present_before_run`]).
/// 2. The MCP install step made no fresh writes and hit no failures —
///    every per-client outcome is `AlreadyUpToDate` or an undetected
///    editor. Fresh installs, refused unsafe drift, picker deselections,
///    consent deferrals, and failures all keep the rich output. An EMPTY
///    per-client map (MCP install disabled via `--no-mcp` /
///    `ANVIL_NO_MCP`) counts as settled by design: nothing MCP-related
///    happened this run, so there is nothing to report richly.
/// 3. Nothing errored: no `last_error` on the diagnostic and the daemon
///    ensure did not fail (a failed ensure carries recovery copy that
///    belongs in the rich block).
/// 4. The final state is `Protecting`, `Watching`, or
///    `ReadyRestartRequired` — protection is live/armed, or the one next
///    action (restart / start the daemon) is already clear. `NeedsAction`,
///    `Unsupported`, and `Error` are repair or coverage-gap states and
///    keep the richer recipe.
///
/// The project-spine ensures (identity, hooks, witness attributes) are
/// idempotent self-healing writes on every run, so they intentionally do
/// not participate in the evidence.
fn is_repeat_success(
    run: Option<&activation::orchestrator::ActivationRun>,
    diagnostic: &activation::ActivationDiagnostic,
    install_report: &activation::orchestrator::InstallReport,
    daemon_outcome: Option<&anvil_intercept::ensure::EnsureOutcome>,
) -> bool {
    use activation::orchestrator::SkipReason;
    use activation::state::ProtectionState;

    let Some(run) = run else {
        return false;
    };
    if !run.config_present_before_run() {
        return false;
    }
    let install_settled = install_report.per_client.values().all(|outcome| {
        matches!(
            outcome,
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate | SkipReason::EditorNotDetected,
            }
        )
    });
    if !install_settled {
        return false;
    }
    if diagnostic.last_error.is_some()
        || matches!(
            daemon_outcome,
            Some(anvil_intercept::ensure::EnsureOutcome::Failed { .. })
        )
    {
        return false;
    }
    matches!(
        diagnostic.protection_state(),
        ProtectionState::Protecting
            | ProtectionState::Watching
            | ProtectionState::ReadyRestartRequired
    )
}

/// CIB-183: collapsed output for a repeat `anvil start` success. Renders
/// exactly (a) the protection state + headline, (b) the daemon and
/// save-time-driver posture, and (c) one arbitrated next step — never the
/// first-run recipe, install block, or language breakdown the user has
/// already seen. Deterministic: same diagnostic in, same bytes out.
///
/// `extra_line` is the CIB-190 seam — one optional, already-rendered local
/// value line inserted between the posture lines and the next step. The
/// caller ([`run`] via [`compute_repeat_value_line`]) owns computing it;
/// this renderer stays a pure function of its arguments.
fn render_repeat_start_output(
    diagnostic: &activation::ActivationDiagnostic,
    daemon_outcome: Option<&anvil_intercept::ensure::EnsureOutcome>,
    extra_line: Option<&str>,
) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    out.push_str("ACTIVATION\n");
    let _ = writeln!(out, "  state: {}", diagnostic.protection_state().label());
    let _ = writeln!(out, "  {}", activation::headline_for_diagnostic(diagnostic));
    if let Some(outcome) = daemon_outcome {
        out.push_str(&render_daemon_lifecycle_line(outcome));
    }
    let _ = writeln!(
        out,
        "  save-time driver: {}",
        if diagnostic.save_time_driver_attached {
            "attached"
        } else {
            "not attached"
        },
    );
    if let Some(line) = extra_line {
        let _ = writeln!(out, "  {line}");
    }
    let _ = writeln!(out, "  {}", arbitrated_next_step(diagnostic));
    out
}

// ── CIB-190: repeat-start local value receipt ────────────────────────────

/// Wall-clock cap on the value-receipt aggregate read.
///
/// The receipt is a nicety, never a gate: the read runs on a helper
/// thread and the line is skipped silently when this budget is exhausted
/// (or the read errors), so a huge witness chain or a wedged filesystem
/// can never stretch a repeat `anvil start`. Deliberately well inside the
/// 500 ms interactive daemon-probe budget
/// (`daemon_evidence::ACTIVATION_DAEMON_QUERY_TIMEOUT`) so the receipt
/// cannot dominate repeat-start latency even in the worst case.
const REPEAT_VALUE_RECEIPT_BUDGET: std::time::Duration = std::time::Duration::from_millis(150);

/// Recency horizon for the value receipt, in days.
///
/// The cumulative aggregate is deliberately wall-clock-free, so it cannot
/// by itself distinguish "recorded yesterday" from "recorded last year" —
/// judging that a stream's evidence is stale therefore requires the one
/// wall-clock comparison this feature makes, and it happens here in the
/// command layer: a stream whose own window end is older than this
/// horizon — or dated in the future, which we equally cannot vouch for —
/// is treated as absent (the line simply does not render). The
/// deterministic renderer (`render_repeat_start_output`) never sees a
/// clock — it receives the pre-rendered line or nothing.
const REPEAT_VALUE_STALE_AFTER_DAYS: i64 = 30;

/// Compute the optional local value receipt for the collapsed
/// repeat-start output. Command layer only: resolves the user-scoped
/// usage sidecar, reads the cumulative aggregate inside
/// [`REPEAT_VALUE_RECEIPT_BUDGET`], and applies the staleness horizon.
/// Every failure mode — unresolvable sidecar path, read error, budget
/// overrun, no fresh non-zero evidence — is `None`: the receipt may
/// never delay activation or fail it.
fn compute_repeat_value_line(root: &Path) -> Option<String> {
    let sidecar = crate::usage::default_usage_log_path().ok()?;
    let value =
        read_cumulative_value_within(root.to_path_buf(), sidecar, REPEAT_VALUE_RECEIPT_BUDGET)?;
    repeat_value_line(&value, chrono::Utc::now())
}

/// Read the cumulative aggregate on a helper thread, abandoning the
/// result when `budget` elapses first. The witness chain is append-only
/// and unbounded, so the read is time-boxed rather than trusted to be
/// cheap; an abandoned reader finishes (or stays blocked) on a detached
/// thread whose send simply finds no receiver.
fn read_cumulative_value_within(
    root: std::path::PathBuf,
    sidecar: std::path::PathBuf,
    budget: std::time::Duration,
) -> Option<crate::insights::cumulative::CumulativeValue> {
    let (tx, rx) = std::sync::mpsc::channel();
    // ≤15 bytes so the name survives Linux's TASK_COMM_LEN truncation.
    std::thread::Builder::new()
        .name("anvil-value".to_string())
        .spawn(move || {
            let _ = tx.send(crate::insights::cumulative::cumulative_value(
                &root, &sidecar,
            ));
        })
        .ok()?;
    rx.recv_timeout(budget).ok()?.ok()
}

/// The one bounded local value line, or `None`.
///
/// Pure function of the aggregate and the caller-supplied `now` — the
/// only clock use is the staleness comparison documented on
/// [`REPEAT_VALUE_STALE_AFTER_DAYS`]. A line renders only when the
/// chosen stream has evidence (per its honest-empty guard), its own
/// window end is inside the horizon (neither stale nor future-dated),
/// and the chosen count is non-zero;
/// anything else is omitted — never rendered as "0 events". The copy
/// carries counts, the stream's own window words (day-precision dates),
/// and a plain-language evidence scope ("on this machine" for save-time,
/// "for this repository" for witness — CIB-222). No paths or repository
/// names leak — structural, since [`CumulativeValue`] carries none.
///
/// Source priority: save-time protection (risky writes flagged, then
/// saves checked) over witness events — the save-time figures are the
/// direct "what has Anvil done" claim; the witness stream is the
/// fallback heartbeat. A stale or fence-only save-time window falls
/// through to the witness arm rather than suppressing the receipt.
fn repeat_value_line(
    value: &crate::insights::cumulative::CumulativeValue,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<String> {
    use crate::insights::scorecard::date_part;

    // Two-sided freshness (council-db2646a1 major 1): the age must be
    // non-negative AND inside the horizon (inclusive at exactly 30
    // days). A future-dated window end — clock skew, or a stray
    // future-dated row — is evidence we cannot vouch for, so it is
    // omitted rather than treated as trivially fresh.
    let fresh = |window_end: Option<&str>| {
        window_end
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
            .is_some_and(|ts| {
                let age = now.signed_duration_since(ts.with_timezone(&chrono::Utc));
                age >= chrono::Duration::zero()
                    && age <= chrono::Duration::days(REPEAT_VALUE_STALE_AFTER_DAYS)
            })
    };

    let save = &value.save_time;
    if save.has_evidence() && fresh(save.window_end.as_deref()) {
        let (Some(start), Some(end)) = (save.window_start.as_deref(), save.window_end.as_deref())
        else {
            unreachable!("has_evidence guarantees both bounds");
        };
        let window = format!("({} to {})", date_part(start), date_part(end));
        if save.risky_writes_flagged > 0 {
            return Some(format!(
                "value: {} flagged at save time on this machine {window}",
                count_noun(save.risky_writes_flagged, "risky write", "risky writes"),
            ));
        }
        if save.evaluations_observed > 0 {
            return Some(format!(
                "value: {} checked on this machine {window}",
                count_noun(save.evaluations_observed, "save", "saves"),
            ));
        }
        // Fence-only evidence has no user-meaningful single count; fall
        // through to the witness stream.
    }

    if value.witness_has_evidence()
        && fresh(value.witness_last_event.as_deref())
        && value.witness_events_last_30_days > 0
    {
        let Some(last) = value.witness_last_event.as_deref() else {
            unreachable!("witness_has_evidence guarantees the bound");
        };
        return Some(format!(
            "value: {} recorded for this repository in the 30 days to {}",
            count_noun(
                value.witness_events_last_30_days,
                "witness event",
                "witness events",
            ),
            date_part(last),
        ));
    }
    None
}

/// `"1 risky write"` / `"3 risky writes"` — honest grammar for the one
/// count the receipt carries.
fn count_noun(n: u64, singular: &str, plural: &str) -> String {
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {plural}")
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
    use eddacraft_tui::keyboard::Action;
    use eddacraft_tui::surface::Surface;
    use std::collections::BTreeMap;

    #[test]
    fn baseline_written_this_run_true_when_applied() {
        use activation::orchestrator::TuiConsentApplyOutcome;
        use std::collections::BTreeSet;

        // Ticked AND actually written this run -> first success.
        let applied = TuiConsentApplyOutcome {
            selected_ids: BTreeSet::from(["project:baseline".to_string()]),
            project_applied: BTreeSet::from([ActivationStep::BaselineSample]),
            ..Default::default()
        };
        assert!(baseline_written_this_run(&applied));
    }

    #[test]
    fn baseline_written_this_run_false_when_ticked_but_skipped() {
        use activation::orchestrator::TuiConsentApplyOutcome;
        use std::collections::BTreeSet;

        // JOURNEY-008 regression: the operator ticked the baseline box but the
        // write was skipped (e.g. no analysable files). `selected_ids` still
        // carries the tick, yet no baseline was written — this must NOT count
        // as a first success, or the celebration banner re-fires on every
        // activation of such a project. Key off `project_applied`, not the tick.
        let mut project_skipped = BTreeMap::new();
        project_skipped.insert(
            ActivationStep::BaselineSample,
            "no analysable files for baseline".to_string(),
        );
        let applied = TuiConsentApplyOutcome {
            selected_ids: BTreeSet::from(["project:baseline".to_string()]),
            project_applied: BTreeSet::new(),
            project_skipped,
            ..Default::default()
        };
        assert!(
            !baseline_written_this_run(&applied),
            "ticked-but-skipped baseline must not count as first success"
        );
    }

    fn test_tui_consent_plan(root: &Path, home: &Path) -> activation::orchestrator::TuiConsentPlan {
        activation::orchestrator::build_tui_consent_plan_with_home(
            root,
            Some(home),
            activation::orchestrator::McpInstallPolicy::Install,
            &activation::mcp_client::all_client_ids(),
            Some(activation::mcp_client::AnvilEntry::local_stdio(
                std::path::PathBuf::from("/usr/local/bin/anvil"),
            )),
            false,
        )
    }

    fn test_activation_surface() -> anvil_tui::surfaces::activation::ActivationSurface {
        anvil_tui::surfaces::activation::ActivationSurface::from_verdict(
            "ACTIVATION\n  state: ready_restart_required\n",
            false,
        )
    }

    fn global_args_default() -> GlobalArgs {
        GlobalArgs {
            json: false,
            no_tui: false,
            verbose: false,
            anvil_home: None,
            touch_project_state: false,
        }
    }

    /// Run `body` with every non-interactive signal and TUI env hatch cleared,
    /// plus any extra variables the caller pins. Models the genuine
    /// interactive shell the TUI default targets.
    fn with_interactive_env<R>(extra: &[(&str, Option<&str>)], body: impl FnOnce() -> R) -> R {
        let mut vars: Vec<(String, Option<String>)> = [
            "CI",
            "ANVIL_NO_PROMPT",
            "NONINTERACTIVE",
            "GIT_DIR",
            "GIT_INDEX_FILE",
            "ANVIL_NO_TUI",
            "ANVIL_ACTIVATION_TUI",
        ]
        .into_iter()
        .map(|key| (key.to_string(), None))
        .collect();
        for (key, value) in extra {
            let value = value.map(ToString::to_string);
            match vars.iter_mut().find(|(existing, _)| existing == key) {
                Some(slot) => slot.1 = value,
                None => vars.push(((*key).to_string(), value)),
            }
        }
        temp_env::with_vars(vars, body)
    }

    #[test]
    fn activation_tui_is_the_interactive_default_without_any_opt_in() {
        // ACTTUI-013 (ADR-103 Release 2): a genuine interactive session enters
        // the TUI with no flag and no env var set. Release 1 required
        // `--tui` / `ANVIL_ACTIVATION_TUI=1` here; that gate is gone.
        with_interactive_env(&[], || {
            assert!(activation_tui_allowed(
                &start_args_default(),
                &global_args_default(),
                false
            ));
        });
    }

    #[test]
    fn retired_tui_opt_in_aliases_are_inert() {
        // `--tui` and `ANVIL_ACTIVATION_TUI=1` are accepted no-op aliases: they
        // must not change the decision in either direction, and they must not
        // override an explicit opt-out.
        let mut args = start_args_default();
        args.tui = true;

        with_interactive_env(&[("ANVIL_ACTIVATION_TUI", Some("1"))], || {
            assert!(activation_tui_allowed(&args, &global_args_default(), false));

            let opted_out = GlobalArgs {
                no_tui: true,
                ..global_args_default()
            };
            assert!(
                !activation_tui_allowed(&args, &opted_out, false),
                "the retired opt-in must not resurrect the TUI past --no-tui"
            );
        });

        with_interactive_env(
            &[
                ("ANVIL_ACTIVATION_TUI", Some("1")),
                ("ANVIL_NO_TUI", Some("1")),
            ],
            || {
                assert!(
                    !activation_tui_allowed(&args, &global_args_default(), false),
                    "the retired opt-in must not resurrect the TUI past ANVIL_NO_TUI"
                );
            },
        );
    }

    #[test]
    fn activation_tui_allowed_keeps_every_plain_path_contract() {
        with_interactive_env(&[], || {
            let args = start_args_default();
            let global = global_args_default();

            assert!(
                !activation_tui_allowed(&args, &global, true),
                "read-only (--verify / --json) stays on the plain contract"
            );
            assert!(
                !activation_tui_allowed(
                    &StartArgs {
                        watch: true,
                        ..start_args_default()
                    },
                    &global,
                    false
                ),
                "the watch fallback streams events on stdout"
            );
            assert!(
                !activation_tui_allowed(
                    &args,
                    &GlobalArgs {
                        no_tui: true,
                        ..global_args_default()
                    },
                    false
                ),
                "--no-tui is the permanent escape hatch"
            );
            assert!(
                !activation_tui_allowed(
                    &args,
                    &GlobalArgs {
                        json: true,
                        ..global_args_default()
                    },
                    false
                ),
                "--json is a machine contract"
            );
        });

        with_interactive_env(&[("ANVIL_NO_TUI", Some("1"))], || {
            assert!(!activation_tui_allowed(
                &start_args_default(),
                &global_args_default(),
                false
            ));
        });

        with_interactive_env(&[("CI", Some("true"))], || {
            assert!(
                !activation_tui_allowed(&start_args_default(), &global_args_default(), false),
                "CI is non-interactive"
            );
        });
    }

    #[test]
    fn production_tui_boundary_applies_a_ticked_mcp_offer() {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let plan = test_tui_consent_plan(root.path(), home.path());

        let applied =
            run_activation_surface_with(test_activation_surface(), &plan, false, |mut surface| {
                // CIB-245: consent is stepped by section, so reaching an MCP
                // row means walking to the MCP section first (`l`/Right) and
                // then down inside it — both through the production key map.
                for _ in 0..surface.consent().unwrap().steps().len() {
                    if surface.consent().unwrap().current_step()
                        == Some(anvil_tui::surfaces::activation::ConsentKind::Mcp)
                    {
                        break;
                    }
                    surface.handle_key(Action::Right);
                }
                let cursor_index = surface
                    .consent()
                    .unwrap()
                    .step_items()
                    .iter()
                    .position(|item| item.id == "mcp:cursor")
                    .unwrap();
                for _ in 0..cursor_index {
                    surface.handle_key(Action::Down);
                }
                surface.handle_key(Action::Toggle);
                surface.handle_key(Action::Character('a'));
                Ok(surface)
            })
            .unwrap();

        assert!(applied.is_some());
        assert!(home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
    }

    /// CIB-245: stepping across sections and ticking in more than one keeps
    /// every selection, so an operator cannot lose a choice by navigating.
    #[test]
    fn production_tui_boundary_retains_selections_across_consent_sections() {
        use anvil_tui::surfaces::activation::ConsentKind;

        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let plan = test_tui_consent_plan(root.path(), home.path());

        // The first (cursor) row of each section, when it is selectable — the
        // exact set the walk below ticks.
        let reference = activation_consent_state(plan.offers(), false);
        let expected: std::collections::BTreeSet<String> = reference
            .steps()
            .iter()
            .filter_map(|kind| {
                reference
                    .items
                    .iter()
                    .find(|item| item.kind == *kind)
                    .filter(|item| item.selectable())
                    .map(|item| item.id.clone())
            })
            .collect();
        assert!(
            reference.steps().len() > 1 && reference.steps().contains(&ConsentKind::Mcp),
            "fixture must span more than one consent section: {:?}",
            reference.steps(),
        );

        let applied =
            run_activation_surface_with(test_activation_surface(), &plan, false, |mut surface| {
                let sections = surface.consent().unwrap().steps().len();
                // Tick the cursor row of every section, stepping forward.
                for _ in 0..sections {
                    surface.handle_key(Action::Toggle);
                    surface.handle_key(Action::Right);
                }
                // Step back through every section; earlier ticks must survive.
                for _ in 0..sections {
                    surface.handle_key(Action::Left);
                }
                surface.handle_key(Action::Character('a'));
                Ok(surface)
            })
            .unwrap()
            .expect("consent applied");

        assert_eq!(
            applied.selected_ids, expected,
            "one row ticked per section survives the round trip",
        );
    }

    #[test]
    fn production_tui_boundary_empty_apply_writes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let plan = test_tui_consent_plan(root.path(), home.path());

        let applied =
            run_activation_surface_with(test_activation_surface(), &plan, false, |mut surface| {
                surface.handle_key(Action::Character('a'));
                Ok(surface)
            })
            .unwrap();

        assert!(applied.is_some());
        assert!(!root.path().join(".github").exists());
        assert!(!home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
    }

    #[test]
    fn production_tui_boundary_quit_cancels_without_writes() {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let plan = test_tui_consent_plan(root.path(), home.path());

        let applied =
            run_activation_surface_with(test_activation_surface(), &plan, false, |mut surface| {
                surface.handle_key(Action::Toggle);
                surface.handle_key(Action::Quit);
                Ok(surface)
            })
            .unwrap();

        assert!(applied.is_none());
        assert!(!root.path().join(".github").exists());
        assert!(!home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
    }

    #[test]
    fn post_consent_surface_opens_typed_verdict_with_applied_report() {
        use activation::diagnostic::McpClientId;
        use activation::mcp_client::DriftClass;
        use anvil_tui::surfaces::activation::ActivationPhase;

        let diagnostic = daemon_attested_diagnostic();
        let mut report = activation::orchestrator::InstallReport::default();
        report.per_client.insert(
            McpClientId::Cursor,
            InstallOutcome::Installed {
                path: "/tmp/.cursor/mcp.json".into(),
                drift: DriftClass::NotPresent,
            },
        );
        let applied = activation::orchestrator::TuiConsentApplyOutcome {
            install_report: report.clone(),
            written_workflows: Vec::new(),
            workflow_error: None,
            selected_ids: std::collections::BTreeSet::from(["mcp:cursor".to_string()]),
            ..Default::default()
        };

        let surface = activation_post_consent_surface(
            "plain copy intentionally unrelated".to_string(),
            &diagnostic,
            &report,
            None,
            &applied,
            false,
            Vec::new(),
            false,
            Path::new("."),
            &[],
        );

        assert_eq!(surface.phase(), ActivationPhase::Verdict);
        assert!(
            surface
                .verdict_view()
                .model()
                .sections
                .iter()
                .any(|section| {
                    section.id == "install"
                        && section
                            .rows
                            .iter()
                            .any(|row| row.contains("Cursor: installed at"))
                })
        );
        assert!(surface.tier_evidence_entries().iter().any(|entry| {
            entry.source == "install/cursor" && entry.message.contains("installed at")
        }));
        assert!(surface.tier_evidence_entries().iter().any(|entry| {
            entry.source == "consent/mcp" && entry.message.contains("1 selected")
        }));
    }

    #[test]
    fn post_consent_progress_replaces_deferred_statuses() {
        use anvil_tui::surfaces::activation::{ActivationProgressStatus, ActivationProgressStep};

        let steps = vec![
            ActivationProgressStep::new(
                "workflow-consent",
                "Workflow consent",
                ActivationProgressStatus::Pending,
            )
            .with_message("awaiting operator choice"),
            ActivationProgressStep::new(
                "mcp-consent",
                "MCP consent",
                ActivationProgressStatus::Pending,
            )
            .with_message("awaiting operator choice"),
        ];
        let applied = activation::orchestrator::TuiConsentApplyOutcome {
            install_report: activation::orchestrator::InstallReport::default(),
            written_workflows: Vec::new(),
            workflow_error: None,
            selected_ids: std::collections::BTreeSet::from([
                "workflow:github-actions".to_string(),
                "mcp:cursor".to_string(),
            ]),
            ..Default::default()
        };

        let steps = activation_post_consent_progress_steps(steps, &applied);

        assert!(steps.iter().all(|step| {
            step.status == ActivationProgressStatus::Passed
                && step
                    .message
                    .as_deref()
                    .is_some_and(|message| message.contains("selected"))
        }));
    }

    #[test]
    fn post_consent_progress_marks_empty_apply_as_not_selected() {
        use anvil_tui::surfaces::activation::{ActivationProgressStatus, ActivationProgressStep};

        let steps = vec![
            ActivationProgressStep::new(
                "mcp-consent",
                "MCP consent",
                ActivationProgressStatus::Pending,
            )
            .with_message("awaiting operator choice"),
        ];

        let steps = activation_post_consent_progress_steps(
            steps,
            &activation::orchestrator::TuiConsentApplyOutcome::default(),
        );

        assert_eq!(steps[0].status, ActivationProgressStatus::Skipped);
        assert_eq!(steps[0].message.as_deref(), Some("not selected"));
    }

    #[test]
    fn consent_progress_holds_final_probe_and_verdict_until_apply() {
        use anvil_tui::surfaces::activation::{ActivationProgressStatus, ActivationProgressStep};

        let mut steps = vec![
            ActivationProgressStep::new(
                "final-probe",
                "Final probe",
                ActivationProgressStatus::Passed,
            ),
            ActivationProgressStep::new("verdict", "Verdict", ActivationProgressStatus::Passed),
        ];
        prepare_consent_progress_steps(&mut steps);
        assert!(steps.iter().all(|step| {
            step.status == ActivationProgressStatus::Pending
                && step.message.as_deref() == Some("awaiting consent outcome")
        }));

        let steps = activation_post_consent_progress_steps(
            steps,
            &activation::orchestrator::TuiConsentApplyOutcome::default(),
        );
        assert!(
            steps
                .iter()
                .all(|step| step.status == ActivationProgressStatus::Passed)
        );
    }

    #[test]
    fn tui_config_apply_failure_preserves_plain_path_exit_contract() {
        let mut applied = activation::orchestrator::TuiConsentApplyOutcome::default();
        applied
            .project_errors
            .insert(ActivationStep::InitConfig, "permission denied".to_string());

        let error = ensure_tui_load_bearing_actions_succeeded(&applied).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("init step of `anvil start` failed")
        );
    }

    #[test]
    fn post_consent_progress_preserves_non_deferred_skips() {
        use anvil_tui::surfaces::activation::{ActivationProgressStatus, ActivationProgressStep};

        let steps = vec![
            ActivationProgressStep::new(
                "mcp-consent",
                "MCP consent",
                ActivationProgressStatus::Skipped,
            )
            .with_message("MCP installation disabled"),
        ];
        let applied = activation::orchestrator::TuiConsentApplyOutcome {
            install_report: activation::orchestrator::InstallReport::default(),
            written_workflows: Vec::new(),
            workflow_error: None,
            ..Default::default()
        };

        let steps = activation_post_consent_progress_steps(steps, &applied);

        assert_eq!(steps[0].status, ActivationProgressStatus::Skipped);
        assert_eq!(
            steps[0].message.as_deref(),
            Some("MCP installation disabled")
        );
    }

    #[test]
    fn post_consent_evidence_preserves_non_deferred_consent_events() {
        use activation::orchestrator::{
            ActivationRun, ActivationStep, ActivationStepEvent, ActivationStepLifecycle,
        };

        let run = ActivationRun::from_events(vec![ActivationStepEvent {
            step: ActivationStep::McpConsent,
            lifecycle: ActivationStepLifecycle::Skipped,
            detail: Some("MCP installation disabled".to_string()),
        }]);
        let diagnostic = daemon_attested_diagnostic();
        let report = activation::orchestrator::InstallReport::default();
        let applied = activation::orchestrator::TuiConsentApplyOutcome {
            install_report: report.clone(),
            written_workflows: Vec::new(),
            workflow_error: None,
            ..Default::default()
        };

        let entries = activation_post_consent_evidence(&diagnostic, &report, Some(&run), &applied);

        assert!(entries.iter().any(|entry| {
            entry.source == "lifecycle/mcp-consent"
                && entry.message.contains("MCP installation disabled")
        }));
        assert!(!entries.iter().any(|entry| entry.source == "consent/mcp"));
    }

    #[test]
    fn activation_tui_env_opt_out_treats_empty_value_as_unset() {
        // C-017: `ANVIL_NO_TUI=` (empty) behaves exactly like the sibling
        // `ANVIL_NO_DAEMON` / `ANVIL_NO_MCP` hatches — empty means unset, only
        // a non-empty value opts out.
        temp_env::with_var("ANVIL_NO_TUI", None::<&str>, || {
            assert!(!activation_tui_env_opt_out());
        });
        temp_env::with_var("ANVIL_NO_TUI", Some("1"), || {
            assert!(activation_tui_env_opt_out());
        });
        temp_env::with_var("ANVIL_NO_TUI", Some(""), || {
            assert!(!activation_tui_env_opt_out());
        });
    }

    #[test]
    fn no_tui_empty_value_semantics_match_sibling_env_hatches() {
        let args = start_args_default();
        temp_env::with_vars(
            [("ANVIL_NO_DAEMON", Some("")), ("ANVIL_NO_MCP", Some(""))],
            || {
                assert!(!start_daemon_opt_out(&args));
                assert!(!start_mcp_opt_out(&args));
            },
        );
        temp_env::with_vars(
            [("ANVIL_NO_DAEMON", Some("1")), ("ANVIL_NO_MCP", Some("1"))],
            || {
                assert!(start_daemon_opt_out(&args));
                assert!(start_mcp_opt_out(&args));
            },
        );
    }

    #[test]
    fn activation_progress_steps_mark_deferred_tui_consent_as_pending() {
        use activation::orchestrator::{
            ActivationStep, ActivationStepEvent, ActivationStepLifecycle,
        };
        use anvil_tui::surfaces::activation::ActivationProgressStatus;

        let events = [
            ActivationStepEvent {
                step: ActivationStep::InitialProbe,
                lifecycle: ActivationStepLifecycle::Started,
                detail: None,
            },
            ActivationStepEvent {
                step: ActivationStep::InitialProbe,
                lifecycle: ActivationStepLifecycle::Completed,
                detail: None,
            },
            ActivationStepEvent {
                step: ActivationStep::McpConsent,
                lifecycle: ActivationStepLifecycle::Deferred,
                detail: Some("awaiting operator approval".to_string()),
            },
        ];
        let steps = activation_progress_steps(&events);
        assert_eq!(steps[0].label, "Initial probe");
        assert_eq!(steps[0].status, ActivationProgressStatus::Passed);
        let mcp = steps.iter().find(|step| step.id == "mcp-consent").unwrap();
        assert_eq!(mcp.status, ActivationProgressStatus::Pending);
        assert_eq!(mcp.message.as_deref(), Some("awaiting operator approval"));
    }

    #[test]
    fn activation_progress_steps_mark_failed_lifecycle_as_failed_not_running() {
        use activation::orchestrator::{
            ActivationStep, ActivationStepEvent, ActivationStepLifecycle,
        };
        use anvil_tui::surfaces::activation::ActivationProgressStatus;

        let events = [
            ActivationStepEvent {
                step: ActivationStep::GitHooks,
                lifecycle: ActivationStepLifecycle::Started,
                detail: None,
            },
            ActivationStepEvent {
                step: ActivationStep::GitHooks,
                lifecycle: ActivationStepLifecycle::Failed,
                detail: Some("could not install git hooks".to_string()),
            },
        ];
        let steps = activation_progress_steps(&events);
        let hooks = steps.iter().find(|step| step.id == "git-hooks").unwrap();
        assert_eq!(hooks.status, ActivationProgressStatus::Failed);
        assert_eq!(
            hooks.message.as_deref(),
            Some("could not install git hooks")
        );
    }

    #[test]
    fn activation_progress_steps_mark_non_consent_skips_as_skipped() {
        use activation::orchestrator::{
            ActivationStep, ActivationStepEvent, ActivationStepLifecycle,
        };
        use anvil_tui::surfaces::activation::ActivationProgressStatus;

        let events = [ActivationStepEvent {
            step: ActivationStep::WorktreeRegistration,
            lifecycle: ActivationStepLifecycle::Skipped,
            detail: Some("not a registerable worktree".to_string()),
        }];
        let steps = activation_progress_steps(&events);
        assert_eq!(steps[0].label, "Worktree registration");
        assert_eq!(steps[0].status, ActivationProgressStatus::Skipped);
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

    #[test]
    fn typed_tui_verdict_ignores_plain_copy_shape() {
        use anvil_tui::surfaces::activation::{ActivationPhase, ActivationSurface};

        let diagnostic = daemon_attested_diagnostic();
        let report = activation::orchestrator::InstallReport::default();
        let model = activation_verdict_model(&diagnostic, &report, &[], None);
        let surface = ActivationSurface::from_typed_with_progress(
            "copy changed completely; state: error",
            model.clone(),
            Vec::new(),
            false,
            Vec::new(),
            Vec::new(),
            false,
            ActivationPhase::Verdict,
        );

        assert_eq!(surface.verdict_view().model(), &model);
        assert_eq!(surface.verdict_view().model().state_label, "watching");
        // ACTTUI-019: layers carry SharedPostureFacts strings (same as status).
        assert!(
            surface
                .verdict_view()
                .model()
                .sections
                .iter()
                .any(|section| {
                    section.id == "layers"
                        && section
                            .rows
                            .iter()
                            .any(|row| row == "daemon: attesting worktree")
                })
        );
    }

    #[test]
    fn typed_tui_evidence_classifies_diagnostic_and_install_records() {
        use activation::diagnostic::{McpClientId, McpTier};
        use activation::mcp_client::DriftClass;
        use eddacraft_tui::prelude::LogLevel;

        let mut diagnostic = synth_diagnostic(activation::state::ProtectionState::Protecting);
        diagnostic.config = activation::diagnostic::ConfigStatus::Invalid;
        diagnostic.mcp.insert(
            McpClientId::Cursor,
            McpTier::RestartHandshakeVerified.into(),
        );
        diagnostic.daemon_attestation =
            activation::daemon_evidence::DaemonAttestation::StaleHeartbeat;
        let mut report = activation::orchestrator::InstallReport::default();
        report.per_client.insert(
            McpClientId::ClaudeCode,
            InstallOutcome::Installed {
                path: "/tmp/.claude.json".into(),
                drift: DriftClass::NotPresent,
            },
        );
        report.per_client.insert(
            McpClientId::Cursor,
            InstallOutcome::Skipped {
                reason: activation::orchestrator::SkipReason::UnsafeDrift(
                    "foreign command".to_string(),
                ),
            },
        );

        let entries = activation_tier_evidence(&diagnostic, &report, None, false);

        assert!(entries.iter().any(|entry| {
            entry.source == "config"
                && entry.level == LogLevel::Error
                && entry.message == "config: invalid"
        }));
        assert!(entries.iter().any(|entry| {
            entry.source == "mcp/cursor"
                && entry.level == LogLevel::Warn
                && entry.message.contains("restart_handshake_verified")
        }));
        assert!(entries.iter().any(|entry| {
            entry.source == "daemon"
                && entry.level == LogLevel::Warn
                && entry.message == "stale heartbeat"
        }));
        assert!(entries.iter().any(|entry| {
            entry.source == "install/claude-code"
                && entry.level == LogLevel::Info
                && entry.message.contains("installed at")
        }));
        assert!(entries.iter().any(|entry| {
            entry.source == "install/cursor"
                && entry.level == LogLevel::Warn
                && entry.message.contains("refused to overwrite")
        }));
    }

    #[test]
    fn consent_evidence_omits_terminal_events_until_post_consent_reprobe() {
        use activation::orchestrator::{
            ActivationRun, ActivationStep, ActivationStepEvent, ActivationStepLifecycle,
        };

        let run = ActivationRun::from_events(vec![
            ActivationStepEvent {
                step: ActivationStep::FinalProbe,
                lifecycle: ActivationStepLifecycle::Completed,
                detail: None,
            },
            ActivationStepEvent {
                step: ActivationStep::Verdict,
                lifecycle: ActivationStepLifecycle::Completed,
                detail: None,
            },
        ]);
        let diagnostic = daemon_attested_diagnostic();
        let report = activation::orchestrator::InstallReport::default();

        let before = activation_tier_evidence(&diagnostic, &report, Some(&run), true);
        assert!(!before.iter().any(|entry| {
            matches!(
                entry.source.as_str(),
                "lifecycle/final-probe" | "lifecycle/verdict"
            )
        }));

        let after = activation_post_consent_evidence(
            &diagnostic,
            &report,
            Some(&run),
            &activation::orchestrator::TuiConsentApplyOutcome::default(),
        );
        assert_eq!(
            after
                .iter()
                .filter(|entry| entry.source == "lifecycle/final-probe")
                .count(),
            1
        );
        assert_eq!(
            after
                .iter()
                .filter(|entry| entry.source == "lifecycle/verdict")
                .count(),
            1
        );
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

    // CIB-183: quiet repeat-success output — collapse detection and the
    // collapsed renderer.

    fn repeat_activation_run() -> activation::orchestrator::ActivationRun {
        use activation::orchestrator::{
            ActivationRun, ActivationStep, ActivationStepEvent, ActivationStepLifecycle,
            INIT_CONFIG_ALREADY_PRESENT_DETAIL,
        };
        ActivationRun::from_events(vec![ActivationStepEvent {
            step: ActivationStep::InitConfig,
            lifecycle: ActivationStepLifecycle::Skipped,
            detail: Some(INIT_CONFIG_ALREADY_PRESENT_DETAIL.to_string()),
        }])
    }

    fn first_run_activation_run() -> activation::orchestrator::ActivationRun {
        use activation::orchestrator::{
            ActivationRun, ActivationStep, ActivationStepEvent, ActivationStepLifecycle,
        };
        ActivationRun::from_events(vec![ActivationStepEvent {
            step: ActivationStep::InitConfig,
            lifecycle: ActivationStepLifecycle::Completed,
            detail: None,
        }])
    }

    fn up_to_date_install_report() -> activation::orchestrator::InstallReport {
        use activation::diagnostic::McpClientId;
        use activation::orchestrator::SkipReason;
        let mut report = activation::orchestrator::InstallReport::default();
        for client in [McpClientId::Cursor, McpClientId::ClaudeCode] {
            report.per_client.insert(
                client,
                InstallOutcome::Skipped {
                    reason: SkipReason::AlreadyUpToDate,
                },
            );
        }
        report
    }

    #[test]
    fn repeat_success_detected_from_run_evidence() {
        // Repeat evidence is the recorded lifecycle event (config existed
        // before this run) plus a settled install report — never a
        // timestamp guess.
        let run = repeat_activation_run();
        let report = up_to_date_install_report();
        for diag in [
            synth_diagnostic(activation::state::ProtectionState::Protecting),
            synth_diagnostic(activation::state::ProtectionState::Watching),
            restart_required_diagnostic(),
        ] {
            assert!(
                is_repeat_success(Some(&run), &diag, &report, None),
                "settled repeat evidence must collapse {:?}",
                diag.protection_state(),
            );
        }
    }

    #[test]
    fn first_run_is_never_a_repeat() {
        // A run whose init step wrote the config this run keeps the rich
        // first-run recipe, even when everything else looks settled.
        let run = first_run_activation_run();
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        assert!(!is_repeat_success(
            Some(&run),
            &diag,
            &up_to_date_install_report(),
            None
        ));
        // No recorded run at all (read-only paths) can never collapse.
        assert!(!is_repeat_success(
            None,
            &diag,
            &up_to_date_install_report(),
            None
        ));
    }

    #[test]
    fn fresh_installs_and_failures_keep_the_rich_output() {
        use activation::diagnostic::McpClientId;
        use activation::mcp_client::DriftClass;

        let run = repeat_activation_run();
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);

        let mut installed = up_to_date_install_report();
        installed.per_client.insert(
            McpClientId::Cursor,
            InstallOutcome::Installed {
                path: "/tmp/.cursor/mcp.json".into(),
                drift: DriftClass::NotPresent,
            },
        );
        assert!(
            !is_repeat_success(Some(&run), &diag, &installed, None),
            "a fresh MCP write this run is not a quiet repeat",
        );

        let mut failed = up_to_date_install_report();
        failed.per_client.insert(
            McpClientId::Cursor,
            InstallOutcome::Failed {
                error: "synthetic".to_string(),
            },
        );
        assert!(
            !is_repeat_success(Some(&run), &diag, &failed, None),
            "an install failure keeps the rich diagnostic",
        );

        let daemon_failed = anvil_intercept::ensure::EnsureOutcome::Failed {
            recovery: "run `anvil intercept start --foreground`.".to_string(),
        };
        assert!(
            !is_repeat_success(
                Some(&run),
                &diag,
                &up_to_date_install_report(),
                Some(&daemon_failed)
            ),
            "a failed daemon ensure keeps the rich diagnostic",
        );
    }

    #[test]
    fn repair_and_coverage_gap_states_keep_the_rich_output() {
        let run = repeat_activation_run();
        let report = up_to_date_install_report();

        let needs_action = synth_diagnostic(activation::state::ProtectionState::NeedsAction);
        assert!(!is_repeat_success(Some(&run), &needs_action, &report, None));

        let mut unsupported = synth_diagnostic(activation::state::ProtectionState::Unsupported);
        unsupported.all_languages_unsupported = true;
        assert!(!is_repeat_success(Some(&run), &unsupported, &report, None));

        let mut errored = synth_diagnostic(activation::state::ProtectionState::Protecting);
        errored.last_error = Some("synthetic activation failure".to_string());
        assert!(
            !is_repeat_success(Some(&run), &errored, &report, None),
            "repair states keep actionable detail; the recovery action stays primary",
        );
    }

    #[test]
    fn no_mcp_empty_install_report_still_collapses() {
        // `--no-mcp` / `ANVIL_NO_MCP` produces an EMPTY per-client map
        // (`McpInstallPolicy::Skip` returns `InstallReport::default()`), and
        // `Iterator::all` over zero entries is vacuously true. That collapse
        // is intended — nothing MCP-related happened this run — and this
        // test pins it so a future per-client skip record for the disabled
        // policy cannot silently flip `--no-mcp` repeats to rich output.
        let run = repeat_activation_run();
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        let empty = activation::orchestrator::InstallReport::default();
        assert!(is_repeat_success(Some(&run), &diag, &empty, None));
    }

    #[test]
    fn invalid_config_never_collapses() {
        // A corrupted `.anvilrc` records the same already-present init skip
        // as a healthy repeat, so the collapse guarantee for invalid config
        // must hold at `is_repeat_success` itself (today it exits via the
        // `Error` protection state) — this pins it against reorderings in
        // `protection_state()` or new `ConfigStatus` variants.
        use activation::diagnostic::ConfigStatus;
        let run = repeat_activation_run();
        let mut diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        diag.config = ConfigStatus::Invalid;
        assert!(!is_repeat_success(
            Some(&run),
            &diag,
            &up_to_date_install_report(),
            None
        ));
    }

    /// CIB-244: the operator selects a registry client (Codex) and deselects
    /// Cursor. Install must name Codex; the Cursor `not selected` row must not
    /// headline the section.
    #[test]
    fn install_section_names_selected_registry_clients_not_deselected_dual_era() {
        use activation::orchestrator::{
            RegistryInstallRow, RegistryInstallStatus, SkipReason, TuiConsentApplyOutcome,
        };

        let mut report = activation::orchestrator::InstallReport::default();
        report.per_client.insert(
            activation::diagnostic::McpClientId::Cursor,
            InstallOutcome::Skipped {
                reason: SkipReason::UserDeselected,
            },
        );
        report.per_client.insert(
            activation::diagnostic::McpClientId::ClaudeCode,
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate,
            },
        );
        let applied = TuiConsentApplyOutcome {
            registry_installs: vec![RegistryInstallRow {
                display_name: "Codex".to_string(),
                status: RegistryInstallStatus::Installed {
                    path: std::path::PathBuf::from("/home/dev/.codex/config.toml"),
                },
            }],
            ..TuiConsentApplyOutcome::default()
        };

        let rows = activation_install_rows(&report, &[], Some(&applied));

        assert_eq!(
            rows.first().map(String::as_str),
            Some("Codex: MCP installed at /home/dev/.codex/config.toml"),
            "the client the operator chose must lead the Install list: {rows:?}",
        );
        assert!(
            !rows.iter().any(|row| row.contains("not selected")),
            "deselected dual-era clients must not appear as Install rows: {rows:?}",
        );
        assert!(
            !rows.iter().any(|row| row.contains("already up to date")),
            "untouched dual-era clients must not appear as Install rows: {rows:?}",
        );
        assert!(
            rows.iter()
                .any(|row| row == "2 other detected clients unchanged (see Evidence)"),
            "unselected detected clients collapse to one honest line: {rows:?}",
        );
    }

    /// CIB-244: this-run outcomes stay first-class — an install and an
    /// unsafe-drift refusal are both decisions the operator must see.
    #[test]
    fn install_section_keeps_this_run_dual_era_outcomes() {
        use activation::diagnostic::McpClientId;
        use activation::mcp_client::DriftClass;
        use activation::orchestrator::SkipReason;

        let mut report = activation::orchestrator::InstallReport::default();
        report.per_client.insert(
            McpClientId::Cursor,
            InstallOutcome::Installed {
                path: std::path::PathBuf::from("/home/dev/.cursor/mcp.json"),
                drift: DriftClass::NotPresent,
            },
        );
        report.per_client.insert(
            McpClientId::ClaudeCode,
            InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift("foreign mcpServers.anvil entry".to_string()),
            },
        );

        let rows = activation_install_rows(&report, &[], None);

        assert!(
            rows.iter().any(|row| row.contains("installed at")),
            "{rows:?}",
        );
        assert!(
            rows.iter().any(|row| row.contains("refused to overwrite")),
            "{rows:?}",
        );
        assert!(
            !rows.iter().any(|row| row.contains("unchanged")),
            "nothing was left alone, so no collapsed line: {rows:?}",
        );
    }

    /// CIB-244: a registry install that failed or was gated is reported as such
    /// under Install, not buried in the evidence log.
    #[test]
    fn install_section_reports_registry_failures_and_skips() {
        use activation::orchestrator::{
            RegistryInstallRow, RegistryInstallStatus, TuiConsentApplyOutcome,
        };

        let applied = TuiConsentApplyOutcome {
            registry_installs: vec![
                RegistryInstallRow {
                    display_name: "OpenCode".to_string(),
                    status: RegistryInstallStatus::Failed {
                        error: "permission denied".to_string(),
                    },
                },
                RegistryInstallRow {
                    display_name: "Zed".to_string(),
                    status: RegistryInstallStatus::Skipped {
                        reason: "project writes are gated for this ANVIL_HOME".to_string(),
                    },
                },
            ],
            ..TuiConsentApplyOutcome::default()
        };

        let rows = activation_install_rows(
            &activation::orchestrator::InstallReport::default(),
            &[],
            Some(&applied),
        );

        assert!(
            rows.iter()
                .any(|row| row == "OpenCode: MCP install failed: permission denied"),
            "{rows:?}",
        );
        assert!(
            rows.iter()
                .any(|row| row.starts_with("Zed: MCP install skipped:")),
            "{rows:?}",
        );
    }

    /// CIB-244: with nothing selected and nothing settled, the section still
    /// refuses to imply work happened.
    #[test]
    fn install_section_is_honest_when_nothing_happened() {
        let rows = activation_install_rows(
            &activation::orchestrator::InstallReport::default(),
            &[],
            None,
        );
        assert_eq!(rows, ["no MCP or project install actions this run"]);
    }

    /// CIB-244: Layers scopes its dual-era MCP probe rather than implying the
    /// clients named under Install were probed and found missing.
    #[test]
    fn layers_scopes_the_dual_era_mcp_probe() {
        let diagnostic = synth_diagnostic(activation::state::ProtectionState::Protecting);
        assert!(
            !diagnostic.mcp.is_empty(),
            "fixture must carry MCP probe rows for this assertion to mean anything",
        );
        let model = activation_verdict_model(
            &diagnostic,
            &activation::orchestrator::InstallReport::default(),
            &[],
            None,
        );
        let layers = &model
            .sections
            .iter()
            .find(|section| section.id == "layers")
            .expect("layers section")
            .rows;
        assert!(
            layers.iter().any(|row| {
                row.starts_with("MCP probe coverage")
                    && row.ends_with("other clients report under Install")
            }),
            "{layers:?}",
        );
    }

    #[test]
    fn tui_verdict_headline_and_next_step_render_the_arbitrated_copy() {
        // Cross-surface literal pin: at ready_restart_required with the
        // daemon unreachable, the TUI verdict must carry the DLIFE-006
        // daemon-unreachable headline override and the repair-hint next
        // step — the same copy the collapsed plain body renders — not the
        // generic restart headline.
        let mut diag = restart_required_diagnostic();
        diag.daemon_attestation = activation::daemon_evidence::DaemonAttestation::Unreachable;
        let model = activation_verdict_model(&diag, &up_to_date_install_report(), &[], None);
        let rows = &model
            .sections
            .iter()
            .find(|s| s.id == "activation")
            .expect("activation section")
            .rows;
        assert!(
            rows.iter()
                .any(|row| row.starts_with("Daemon not reachable")),
            "TUI headline must use the daemon-unreachable override: {rows:?}",
        );
        assert!(
            rows.iter()
                .any(|row| row.starts_with("next: no intercept daemon is answering")),
            "TUI next step must be the arbitrated repair hint: {rows:?}",
        );
    }

    #[test]
    fn collapsed_repeat_protecting_output_is_state_posture_and_one_next_step() {
        // Snapshot: the collapsed repeat `protecting` bytes. Deterministic —
        // same diagnostic in, same bytes out.
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        let rendered = render_repeat_start_output(
            &diag,
            Some(&anvil_intercept::ensure::EnsureOutcome::Reused),
            None,
        );
        assert_eq!(
            rendered,
            "ACTIVATION\n\
             \x20 state: protecting\n\
             \x20 Protecting — pre-write validation is live in this repo.\n\
             \x20 daemon: reusing the per-user save-time daemon already running.\n\
             \x20 save-time driver: not attached\n\
             \x20 Next: MCP pre-write protection is live; run `anvil status` to see posture any time.\n",
        );
        // The first-run blocks must be gone.
        for banned in ["verify:", "active layers", "recipe", "install:", "mcp:"] {
            assert!(
                !rendered.contains(banned),
                "collapsed output must not reprint `{banned}`: {rendered}",
            );
        }
    }

    #[test]
    fn collapsed_repeat_repair_hint_owns_the_single_next_step() {
        // At ready_restart_required with the daemon unreachable, the one
        // next step is the CIB-162/166 repair hint — the recovery action
        // stays primary and no competing `Next:` line renders.
        let mut diag = restart_required_diagnostic();
        diag.daemon_attestation = activation::daemon_evidence::DaemonAttestation::Unreachable;
        let rendered = render_repeat_start_output(&diag, None, None);
        assert!(
            rendered.contains("next: no intercept daemon is answering"),
            "repair hint must own the collapsed ending: {rendered}",
        );
        assert!(
            !rendered.contains("Next:"),
            "no competing closing line may render: {rendered}",
        );
        assert_eq!(
            rendered.matches("next:").count(),
            1,
            "exactly one next step: {rendered}",
        );
        // The DLIFE-006 daemon-unreachable headline override is reused.
        assert!(
            rendered.contains("Daemon not reachable"),
            "collapsed headline must reuse headline_for_diagnostic: {rendered}",
        );
    }

    #[test]
    fn collapsed_renderer_reserves_the_cib190_extra_line_slot() {
        // CIB-190 seam: one optional, pre-rendered local value line slots
        // between the posture lines and the single next step. This item
        // only reserves the seam — the value line itself is CIB-190's.
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        let rendered = render_repeat_start_output(&diag, None, Some("local value placeholder"));
        let lines: Vec<&str> = rendered.lines().collect();
        let slot = lines
            .iter()
            .position(|line| *line == "  local value placeholder")
            .expect("extra line renders with the standard indent");
        assert!(
            lines[slot - 1].starts_with("  save-time driver:"),
            "extra line sits after the posture lines: {rendered}",
        );
        assert!(
            lines[slot + 1].starts_with("  Next:"),
            "extra line sits before the single next step: {rendered}",
        );
        // Omitting it removes exactly that one line.
        let without = render_repeat_start_output(&diag, None, None);
        assert_eq!(
            without.lines().count(),
            lines.len() - 1,
            "empty slot renders nothing extra",
        );
    }

    // ── CIB-190: the repeat-start local value receipt ──

    fn empty_receipt_value() -> crate::insights::cumulative::CumulativeValue {
        crate::insights::cumulative::CumulativeValue {
            since: None,
            as_of: None,
            witness_first_event: None,
            witness_last_event: None,
            witness_events_total: 0,
            witness_events_last_30_days: 0,
            witness_events_last_90_days: 0,
            save_time: crate::insights::cumulative::SaveTimeCounts {
                window_start: None,
                window_end: None,
                evaluations_observed: 0,
                risky_writes_flagged: 0,
                writes_blocked: 0,
                secret_findings_caught: 0,
                fences_engaged: 0,
            },
        }
    }

    fn receipt_now(ts: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn value_receipt_renders_fresh_save_time_evidence() {
        // Healthy evidence: the save-time stream is the primary claim.
        let now = receipt_now("2026-07-11T00:00:00Z");
        let mut value = empty_receipt_value();
        value.save_time.window_start = Some("2026-07-05T10:00:00Z".to_string());
        value.save_time.window_end = Some("2026-07-08T12:00:00Z".to_string());
        value.save_time.evaluations_observed = 3;
        value.save_time.risky_writes_flagged = 2;
        assert_eq!(
            repeat_value_line(&value, now).as_deref(),
            Some(
                "value: 2 risky writes flagged at save time on this machine (2026-07-05 to 2026-07-08)"
            ),
        );
        // Nothing flagged → the honest fallback claim is saves checked.
        value.save_time.risky_writes_flagged = 0;
        assert_eq!(
            repeat_value_line(&value, now).as_deref(),
            Some("value: 3 saves checked on this machine (2026-07-05 to 2026-07-08)"),
        );
        // Singular grammar.
        value.save_time.risky_writes_flagged = 1;
        assert_eq!(
            repeat_value_line(&value, now).as_deref(),
            Some(
                "value: 1 risky write flagged at save time on this machine (2026-07-05 to 2026-07-08)"
            ),
        );
    }

    #[test]
    fn value_receipt_falls_back_to_fresh_witness_events() {
        // Witness-only evidence renders the windowed count, labelled by
        // the witness stream's OWN anchor — never a wall-clock date.
        let now = receipt_now("2026-07-11T00:00:00Z");
        let mut value = empty_receipt_value();
        value.witness_first_event = Some("2026-01-05T08:00:00Z".to_string());
        value.witness_last_event = Some("2026-07-01T10:00:00Z".to_string());
        value.witness_events_total = 12;
        value.witness_events_last_30_days = 4;
        value.witness_events_last_90_days = 9;
        let expected =
            "value: 4 witness events recorded for this repository in the 30 days to 2026-07-01";
        assert_eq!(repeat_value_line(&value, now).as_deref(), Some(expected));
        // A stale save-time window must not suppress the fresh witness
        // arm — staleness falls through, it never vetoes the receipt.
        value.save_time.window_start = Some("2026-04-01T00:00:00Z".to_string());
        value.save_time.window_end = Some("2026-04-02T00:00:00Z".to_string());
        value.save_time.evaluations_observed = 7;
        value.save_time.risky_writes_flagged = 7;
        assert_eq!(repeat_value_line(&value, now).as_deref(), Some(expected));
    }

    #[test]
    fn value_receipt_omits_absent_and_zero_evidence() {
        let now = receipt_now("2026-07-11T00:00:00Z");
        // No evidence anywhere → no line (never "0 events").
        assert_eq!(repeat_value_line(&empty_receipt_value(), now), None);

        // Fence-only save-time evidence has no chosen count; with no
        // witness evidence the receipt is omitted, not zero-filled.
        let mut fences_only = empty_receipt_value();
        fences_only.save_time.window_start = Some("2026-07-05T10:00:00Z".to_string());
        fences_only.save_time.window_end = Some("2026-07-08T12:00:00Z".to_string());
        fences_only.save_time.fences_engaged = 2;
        assert_eq!(repeat_value_line(&fences_only, now), None);

        // Witness bounds present but a zero windowed count → omitted.
        let mut zero_witness = empty_receipt_value();
        zero_witness.witness_first_event = Some("2026-06-01T00:00:00Z".to_string());
        zero_witness.witness_last_event = Some("2026-07-01T00:00:00Z".to_string());
        zero_witness.witness_events_total = 5;
        zero_witness.witness_events_last_30_days = 0;
        assert_eq!(repeat_value_line(&zero_witness, now), None);
    }

    #[test]
    fn value_receipt_omits_stale_and_unparseable_evidence() {
        // Both streams' own window ends sit outside the 30-day horizon
        // relative to command time → the receipt is omitted entirely.
        let mut value = empty_receipt_value();
        value.witness_first_event = Some("2026-01-05T08:00:00Z".to_string());
        value.witness_last_event = Some("2026-03-01T10:00:00Z".to_string());
        value.witness_events_total = 12;
        value.witness_events_last_30_days = 4;
        value.witness_events_last_90_days = 9;
        value.save_time.window_start = Some("2026-02-01T00:00:00Z".to_string());
        value.save_time.window_end = Some("2026-02-20T00:00:00Z".to_string());
        value.save_time.evaluations_observed = 9;
        value.save_time.risky_writes_flagged = 3;
        let stale_now = receipt_now("2026-07-11T00:00:00Z");
        assert_eq!(repeat_value_line(&value, stale_now), None);
        // The same aggregate read close to its own evidence is fresh —
        // the horizon is relative to command time, not absolute.
        assert!(repeat_value_line(&value, receipt_now("2026-03-01T12:00:00Z")).is_some());
        // An unparseable window end is ambiguous evidence: treated as
        // absent, never guessed at.
        value.save_time.window_end = Some("not-a-timestamp".to_string());
        assert_eq!(repeat_value_line(&value, stale_now), None);
    }

    #[test]
    fn value_receipt_omits_future_dated_and_pins_the_horizon_boundary() {
        // Council-db2646a1 major 1: freshness is two-sided. A
        // future-dated window end (clock skew, or a stray future-dated
        // row) is evidence we cannot vouch for → omitted, never treated
        // as trivially fresh.
        let now = receipt_now("2026-07-11T00:00:00Z");
        let mut value = empty_receipt_value();
        value.save_time.window_start = Some("2099-01-01T00:00:00Z".to_string());
        value.save_time.window_end = Some("2099-01-02T00:00:00Z".to_string());
        value.save_time.evaluations_observed = 5;
        value.save_time.risky_writes_flagged = 2;
        assert_eq!(repeat_value_line(&value, now), None);

        // The horizon is inclusive: an age of exactly 30 days renders …
        value.save_time.window_start = Some("2026-06-01T00:00:00Z".to_string());
        value.save_time.window_end = Some("2026-06-11T00:00:00Z".to_string());
        assert_eq!(
            repeat_value_line(&value, now).as_deref(),
            Some(
                "value: 2 risky writes flagged at save time on this machine (2026-06-01 to 2026-06-11)"
            ),
        );
        // … and one second past it is omitted.
        assert_eq!(
            repeat_value_line(&value, receipt_now("2026-07-11T00:00:01Z")),
            None,
        );
    }

    #[test]
    fn value_receipt_is_confined_to_the_collapsed_repeat_path() {
        // The receipt enters through `render_repeat_start_output`'s
        // extra-line slot only, and `run` computes it only when the
        // CIB-183 collapse fires — so every repair path (where the
        // recovery action must stay primary) fails `is_repeat_success`
        // and can never carry the line. Pin that gate from the
        // receipt's perspective.
        let run = repeat_activation_run();
        let report = up_to_date_install_report();

        let needs_action = synth_diagnostic(activation::state::ProtectionState::NeedsAction);
        assert!(!is_repeat_success(Some(&run), &needs_action, &report, None));

        let mut errored = synth_diagnostic(activation::state::ProtectionState::Protecting);
        errored.last_error = Some("synthetic activation failure".to_string());
        assert!(!is_repeat_success(Some(&run), &errored, &report, None));

        let healthy = synth_diagnostic(activation::state::ProtectionState::Protecting);
        let daemon_failed = anvil_intercept::ensure::EnsureOutcome::Failed {
            recovery: "run `anvil intercept start --foreground`.".to_string(),
        };
        assert!(!is_repeat_success(
            Some(&run),
            &healthy,
            &report,
            Some(&daemon_failed)
        ));
    }

    #[test]
    fn collapsed_output_carries_the_value_receipt_line() {
        // End-to-end through the CIB-183 seam: the receipt renders once,
        // in the reserved slot, with the standard indent.
        let diag = synth_diagnostic(activation::state::ProtectionState::Protecting);
        let mut value = empty_receipt_value();
        value.save_time.window_start = Some("2026-07-05T10:00:00Z".to_string());
        value.save_time.window_end = Some("2026-07-08T12:00:00Z".to_string());
        value.save_time.evaluations_observed = 3;
        value.save_time.risky_writes_flagged = 2;
        let line = repeat_value_line(&value, receipt_now("2026-07-11T00:00:00Z")).unwrap();
        let rendered = render_repeat_start_output(&diag, None, Some(&line));
        assert!(
            rendered.contains(
                "\n  value: 2 risky writes flagged at save time on this machine (2026-07-05 to 2026-07-08)\n"
            ),
            "{rendered}",
        );
        assert_eq!(rendered.matches("value:").count(), 1, "{rendered}");
    }

    /// Write a marker-seeded witness chain (one event at `ts`) under
    /// `root`, for the receipt redaction fixtures.
    fn write_marker_witness(root: &std::path::Path, ts: &str) {
        use anvil_witness::{GenesisAnchor, WitnessLine};
        let witness_dir = root.join("anvil/witness");
        std::fs::create_dir_all(&witness_dir).unwrap();
        let witness = WitnessLine {
            seq: 1,
            scope: "active".to_string(),
            kind: "witness".to_string(),
            prev_line_hash: GenesisAnchor::Fresh.anchor_string().to_string(),
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
            commit_sha: Some(format!("{:040x}", 1)),
            parent_commits: Vec::new(),
            prev_line_hashes: Vec::new(),
            agent_tag: Some("marker-agent".to_string()),
            rules_sha: None,
            cutoff_commit: None,
            ts: ts.to_string(),
            validation_at: "pre-commit".to_string(),
        };
        std::fs::write(
            witness_dir.join("active.ndjson"),
            witness.to_ndjson_line().unwrap(),
        )
        .unwrap();
    }

    /// One marker-seeded flagged save-time sidecar row at `ts` — every
    /// free-text field carries "marker".
    fn marker_gate_row(ts: &str) -> String {
        concat!(
            r#"{"kind":"gate_evaluated","session_id":"marker-sess","#,
            r#""timestamp":"2026-07-08T12:00:00Z","gate_eval_id":"marker-eval","#,
            r#""gate_id":"save-time","#,
            r#""inputs":{"file_count":1,"#,
            r#""changed_files":["/home/markeruser/marker-repo/src/marker.rs"],"#,
            r#""baseline_hash":"marker-hash"},"#,
            r#""outcome":"fail","rules_evaluated":["path-deny"],"#,
            r#""rules_violated":["path-deny"],"enforcement":"warning","#,
            r#""duration_ms":12,"partial":false,"principal":"marker@example.com"}"#,
            "\n",
        )
        .replace("2026-07-08T12:00:00Z", ts)
    }

    /// The receipt copy must carry counts and window words only.
    fn assert_receipt_redacted(line: &str) {
        let lowered = line.to_lowercase();
        assert!(!lowered.contains("marker"), "{line}");
        assert!(!line.contains('/'), "no path fragments: {line}");
        assert!(!line.contains('@'), "no principals: {line}");
    }

    #[test]
    fn value_receipt_redacts_to_counts_and_window_words() {
        // Marker-seeded sources: every free-text field carries "marker",
        // so the exact-copy assertion plus one substring check proves
        // nothing but counts and window words reach the line.
        let tmp = tempfile::TempDir::new().unwrap();
        write_marker_witness(tmp.path(), "2026-07-01T10:00:00Z");
        let sidecar = tmp.path().join("kindling/usage.ndjson");
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::write(&sidecar, marker_gate_row("2026-07-08T12:00:00Z")).unwrap();

        let value = crate::insights::cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();
        let line = repeat_value_line(&value, receipt_now("2026-07-11T00:00:00Z"))
            .expect("fresh flagged evidence renders");
        assert_eq!(
            line,
            "value: 1 risky write flagged at save time on this machine (2026-07-08 to 2026-07-08)",
        );
        assert_receipt_redacted(&line);
    }

    #[test]
    fn value_receipt_witness_arm_redacts_to_counts_and_window_words() {
        // Council-db2646a1 minor 3: the primary redaction fixture
        // renders the save-time arm, so pin the WITNESS arm marker-free
        // too — stale save-time evidence falls through and the witness
        // line is the asserted render.
        let tmp = tempfile::TempDir::new().unwrap();
        write_marker_witness(tmp.path(), "2026-07-01T10:00:00Z");
        let sidecar = tmp.path().join("kindling/usage.ndjson");
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::write(&sidecar, marker_gate_row("2026-01-08T12:00:00Z")).unwrap();

        let value = crate::insights::cumulative::cumulative_value(tmp.path(), &sidecar).unwrap();
        assert!(
            value.save_time.has_evidence(),
            "fixture must be stale save-time evidence, not absent",
        );
        let line = repeat_value_line(&value, receipt_now("2026-07-11T00:00:00Z"))
            .expect("fresh witness evidence renders");
        assert_eq!(
            line,
            "value: 1 witness event recorded for this repository in the 30 days to 2026-07-01",
        );
        assert_receipt_redacted(&line);
    }

    #[test]
    fn value_receipt_skips_silently_on_read_error() {
        // A sidecar path that opens but cannot be read line-wise (a
        // directory) is a read error: the receipt is skipped, never a
        // panic and never a failed activation.
        let tmp = tempfile::TempDir::new().unwrap();
        let sidecar_dir = tmp.path().join("kindling/usage.ndjson");
        std::fs::create_dir_all(&sidecar_dir).unwrap();
        assert!(
            read_cumulative_value_within(
                tmp.path().to_path_buf(),
                sidecar_dir,
                std::time::Duration::from_secs(5),
            )
            .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn value_receipt_skips_when_the_read_exceeds_the_budget() {
        // A FIFO with no writer blocks `open(2)` indefinitely — a
        // stand-in for any pathologically slow witness chain. The
        // receipt must come back `None` within the budget instead of
        // hanging `anvil start`; the abandoned reader stays blocked on
        // its detached thread until process exit (the documented
        // time-box contract).
        let tmp = tempfile::TempDir::new().unwrap();
        let witness_dir = tmp.path().join("anvil/witness");
        std::fs::create_dir_all(&witness_dir).unwrap();
        nix::unistd::mkfifo(
            &witness_dir.join("active.ndjson"),
            nix::sys::stat::Mode::from_bits_truncate(0o600),
        )
        .expect("mkfifo the synthetic witness segment");

        let started = std::time::Instant::now();
        assert!(
            read_cumulative_value_within(
                tmp.path().to_path_buf(),
                tmp.path().join("kindling/usage.ndjson"),
                std::time::Duration::from_millis(50),
            )
            .is_none()
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "the time-box must bound the caller, not the reader",
        );
    }

    #[test]
    fn tui_verdict_next_step_matches_the_plain_arbiter() {
        // CIB-183: the TUI verdict derives its next step from the same
        // arbiter as the plain path — byte-identical copy, no duplication.
        for diag in [
            synth_diagnostic(activation::state::ProtectionState::Protecting),
            restart_required_diagnostic(),
            daemon_attested_diagnostic(),
        ] {
            let model = activation_verdict_model(
                &diag,
                &activation::orchestrator::InstallReport::default(),
                &[],
                None,
            );
            let activation_section = model
                .sections
                .iter()
                .find(|section| section.id == "activation")
                .expect("verdict model has an activation section");
            assert!(
                activation_section
                    .rows
                    .contains(&arbitrated_next_step(&diag)),
                "TUI verdict must carry the arbitrated next step for {:?}: {:?}",
                diag.protection_state(),
                activation_section.rows,
            );
        }
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

    /// #3221: the smoke-recipe AKIA string must match the `AKIA[0-9A-Z]{16}`
    /// detector — a 15-char suffix was a silent false negative.
    #[test]
    fn first_run_recipe_smoke_string_triggers_secret_detection() {
        use anvil_checks::secret::{SecretCheckConfig, scan_content};

        let content = r#"const KEY = "AKIAQRSTUVWXYZ123456";"#;
        let findings = scan_content(
            content,
            ".anvil-smoke-test.ts",
            &SecretCheckConfig::default(),
        );
        assert!(
            findings.iter().any(|f| f.pattern_name == "AWS Key"),
            "recipe smoke string must trigger secret-detection, got: {findings:?}",
        );
    }

    #[test]
    fn activation_prove_reports_check_pipeline_only() {
        let toast = run_activation_prove(false, true);
        assert!(
            toast.contains("secret-detection caught"),
            "prove must report a real finding: {toast}"
        );
        assert!(
            toast.contains("check pipeline only"),
            "prove must not over-claim MCP: {toast}"
        );
        assert!(!toast.contains("contract-hardening"));
        assert!(toast.contains("not MCP pre-write") || toast.contains("does not claim MCP"));

        let blocked = run_activation_prove(true, true);
        assert!(
            blocked.contains("Prove unavailable") && blocked.contains("no supported languages"),
            "unsupported repos must gate prove: {blocked}"
        );

        let disabled = run_activation_prove(false, false);
        assert!(
            disabled.contains("secret-detection is not enabled"),
            "disabled check must gate prove: {disabled}"
        );
    }

    #[test]
    fn secret_detection_enabled_honours_anvilrc_checks() {
        let tmp = tempfile::tempdir().unwrap();
        // No config → planless default includes secret-detection.
        assert!(secret_detection_enabled_in_project(tmp.path()));

        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks:\n  - antipattern-scan\n",
        )
        .unwrap();
        assert!(
            !secret_detection_enabled_in_project(tmp.path()),
            "explicit list without secret-detection must disable Prove"
        );

        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks:\n  - secret-detection\n  - antipattern-scan\n",
        )
        .unwrap();
        assert!(secret_detection_enabled_in_project(tmp.path()));
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

    /// CIB-225: when `.anvilrc` already exists, `--format toml` must not
    /// rewrite/convert and must report the existing path as the reason the
    /// flag was ignored.
    #[test]
    fn pre_write_format_ignored_when_legacy_anvilrc_present_names_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let legacy = tmp.path().join(".anvilrc");
        std::fs::write(&legacy, r#"{"checks":[]}"#).unwrap();

        let existing = existing_project_config_path(tmp.path())
            .unwrap()
            .expect("legacy .anvilrc must be discovered");
        assert_eq!(existing, legacy);

        pre_write_anvil_config(tmp.path(), StartFormat::Toml).unwrap();
        assert!(
            !tmp.path().join(".anvil.toml").exists(),
            "pre-write must not convert .anvilrc when --format is set"
        );
        let warning = format_ignored_existing_config_warning(&existing);
        assert!(
            warning.contains("--format ignored"),
            "warning must name the ignored flag: {warning}"
        );
        assert!(
            warning.contains(existing.to_string_lossy().as_ref()),
            "warning must name the existing config path: {warning}"
        );
    }

    /// CIB-225: other-format `.anvil.<ext>` also causes `--format` to be
    /// ignored with the existing path named (no second config written).
    #[test]
    fn pre_write_format_ignored_when_other_anvil_ext_present_names_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let existing_path = tmp.path().join(".anvil.yaml");
        std::fs::write(&existing_path, "checks: []\n").unwrap();

        let existing = existing_project_config_path(tmp.path())
            .unwrap()
            .expect("existing .anvil.yaml must be discovered");
        assert_eq!(existing, existing_path);

        pre_write_anvil_config(tmp.path(), StartFormat::Toml).unwrap();
        assert!(!tmp.path().join(".anvil.toml").exists());
        let warning = format_ignored_existing_config_warning(&existing);
        assert!(
            warning.contains(existing.to_string_lossy().as_ref()),
            "warning must name the existing .anvil.yaml path: {warning}"
        );
    }

    /// CIB-224: `--no-mcp` + `--mcp-client` fails with a one-line recovery.
    #[test]
    fn no_mcp_with_mcp_client_is_rejected() {
        let mut args = start_args_default();
        args.no_mcp = true;
        args.mcp_client = vec![AgentClientId::Codex];
        let err = reject_no_mcp_with_client_selection(&args).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("mutually exclusive"),
            "must name mutual exclusion: {msg}"
        );
        assert!(
            msg.contains("--mcp-client") && msg.contains("--no-mcp"),
            "recovery must name both flags: {msg}"
        );
        assert!(
            msg.contains("ANVIL_NO_MCP") && msg.contains("unset"),
            "recovery must cover env opt-out: {msg}"
        );
    }

    /// CIB-224: `--no-mcp` + `--all-mcp-clients` fails with recovery.
    #[test]
    fn no_mcp_with_all_mcp_clients_is_rejected() {
        let mut args = start_args_default();
        args.no_mcp = true;
        args.all_mcp_clients = true;
        let err = reject_no_mcp_with_client_selection(&args).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("mutually exclusive"),
            "must name mutual exclusion: {msg}"
        );
        assert!(
            msg.contains("--all-mcp-clients") || msg.contains("ANVIL_ALL_MCP_CLIENTS"),
            "recovery must name all-clients selection: {msg}"
        );
        assert!(
            msg.contains("ANVIL_NO_MCP") && msg.contains("unset"),
            "recovery must cover env opt-out: {msg}"
        );
    }

    /// CIB-224: env forms of the conflict also reject.
    #[test]
    fn no_mcp_env_with_all_mcp_clients_env_is_rejected() {
        let args = start_args_default();
        temp_env::with_vars(
            [
                ("ANVIL_NO_MCP", Some("1")),
                ("ANVIL_ALL_MCP_CLIENTS", Some("1")),
            ],
            || {
                let err = reject_no_mcp_with_client_selection(&args).unwrap_err();
                let msg = format!("{err:#}");
                assert!(
                    msg.contains("mutually exclusive"),
                    "env forms must also conflict: {msg}"
                );
            },
        );
    }

    /// CIB-224: `--no-mcp` alone remains valid.
    #[test]
    fn no_mcp_alone_is_allowed() {
        let mut args = start_args_default();
        args.no_mcp = true;
        reject_no_mcp_with_client_selection(&args).expect("--no-mcp alone must be allowed");
    }

    /// CIB-223 soft path: non-git cwd gets one coherent next-step line —
    /// config may be written; git init / register before protection attaches.
    /// No jarring "no worktree registered" success-then-contradiction framing.
    #[test]
    fn non_git_cwd_worktree_message_is_coherent_soft_path() {
        let line = non_registerable_worktree_line(
            &crate::registration::NotRegisterable::NotAWorktree("not a git repository".to_string()),
        );
        assert!(
            line.contains("project config may be written"),
            "soft path must allow durable init framing: {line}"
        );
        assert!(
            line.contains("git init"),
            "soft path must name `git init` as recovery: {line}"
        );
        assert!(
            line.contains("anvil workspace register") || line.contains("register"),
            "soft path must name registration recovery: {line}"
        );
        assert!(
            line.contains("before protection can attach"),
            "soft path must state protection requires git/worktree: {line}"
        );
        assert!(
            !line.contains("no worktree registered"),
            "must not use the old jarring phrasing: {line}"
        );

        // Full human output on a non-git tempdir carries the same contract.
        let tmp = tempfile::TempDir::new().unwrap();
        let diag = synth_diagnostic(activation::state::ProtectionState::NeedsAction);
        let out = render_start_human_output(
            tmp.path(),
            false,
            &diag,
            &activation::orchestrator::InstallReport::default(),
            None,
            activation::orchestrator::McpInstallPolicy::Install,
            &activation::detect_agents::AgentInventory::default(),
            false,
        );
        assert!(
            out.contains("project config may be written"),
            "render_start_human_output must include soft-path framing for non-git cwd:
{out}"
        );
        assert!(
            out.contains("git init"),
            "render_start_human_output must name git init for non-git cwd:
{out}"
        );
        assert!(
            !out.contains("no worktree registered"),
            "render_start_human_output must not use the old jarring phrasing:
{out}"
        );
    }

    /// CIB-223: bare / inside-git-dir reasons stay coherent (config ok; protect later).
    #[test]
    fn non_registerable_bare_and_git_dir_messages_stay_soft() {
        let bare =
            non_registerable_worktree_line(&crate::registration::NotRegisterable::BareRepository);
        assert!(bare.contains("project config may be written"), "{bare}");
        assert!(bare.contains("before protection can attach"), "{bare}");
        assert!(!bare.contains("no worktree registered"), "{bare}");

        let inside =
            non_registerable_worktree_line(&crate::registration::NotRegisterable::InsideGitDir);
        assert!(inside.contains("project config may be written"), "{inside}");
        assert!(inside.contains("before protection can attach"), "{inside}");
        assert!(!inside.contains("no worktree registered"), "{inside}");
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
            mcp_client: Vec::new(),
            mcp_scope: InstallScope::Global,
        }
    }

    #[test]
    fn tui_mode_never_runs_the_supplemental_plain_mcp_installer() {
        let mut args = start_args_default();
        args.all_mcp_clients = true;
        args.mcp_client = vec![AgentClientId::Codex];

        let lines = install_first_wave_mcp_clients(&args, StartRenderMode::Tui).unwrap();

        assert!(lines.is_empty());
    }

    #[test]
    fn plain_project_scope_installs_legacy_clients_only_under_project_root() {
        let project = tempfile::TempDir::new().unwrap();
        let home = tempfile::TempDir::new().unwrap();
        std::fs::write(project.path().join(".anvilrc"), r#"{"checks":[]}"#).unwrap();
        let mut diagnostic =
            activation::diagnostic::verify_with_home(project.path(), Some(home.path()));
        assert_eq!(
            diagnostic.protection_state(),
            activation::state::ProtectionState::NeedsAction,
        );
        let mut args = start_args_default();
        args.mcp_scope = InstallScope::Project;
        args.mcp_client = vec![AgentClientId::ClaudeCode, AgentClientId::Cursor];

        let command = std::env::current_exe().unwrap();
        let lines = install_first_wave_mcp_clients_at(
            &args,
            StartRenderMode::Plain,
            Some(home.path()),
            project.path(),
            &command,
        )
        .unwrap();

        assert_eq!(lines.len(), 2);
        assert!(project.path().join(".mcp.json").exists());
        assert!(project.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
        assert!(!home.path().join(".cursor/mcp.json").exists());

        reconcile_plain_mcp_diagnostic(
            project.path(),
            StartRenderMode::Plain,
            InstallScope::Project,
            !lines.is_empty(),
            &mut diagnostic,
        );
        assert_eq!(
            diagnostic.protection_state(),
            activation::state::ProtectionState::ReadyRestartRequired,
        );
    }

    #[test]
    fn global_first_wave_reconciliation_preserves_a_legacy_install_error() {
        let project = tempfile::TempDir::new().unwrap();
        let mut diagnostic = activation::verify(project.path());
        diagnostic.last_error = Some("legacy MCP install failed".to_string());

        reconcile_plain_mcp_diagnostic(
            project.path(),
            StartRenderMode::Plain,
            InstallScope::Global,
            true,
            &mut diagnostic,
        );

        assert_eq!(
            diagnostic.last_error.as_deref(),
            Some("legacy MCP install failed")
        );
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

    #[test]
    fn project_scope_disables_the_global_fallback_legacy_installer() {
        let args = StartArgs {
            mcp_scope: InstallScope::Project,
            ..start_args_default()
        };

        assert_eq!(
            legacy_mcp_install_policy(&args),
            activation::orchestrator::McpInstallPolicy::Skip,
        );
        assert_eq!(
            mcp_install_policy(&args),
            activation::orchestrator::McpInstallPolicy::Install,
        );
    }

    /// CIB-220: interactive project-scope start must Install (defer to the
    /// MCP picker), never Skip with "MCP installation disabled".
    #[test]
    fn tui_project_scope_orchestrator_policy_installs_not_disabled() {
        let args = StartArgs {
            mcp_scope: InstallScope::Project,
            ..start_args_default()
        };
        assert_eq!(
            orchestrator_mcp_install_policy(&args, StartRenderMode::Tui),
            activation::orchestrator::McpInstallPolicy::Install,
        );
        // --no-mcp still skips for both modes.
        let opted_out = StartArgs {
            no_mcp: true,
            mcp_scope: InstallScope::Project,
            ..start_args_default()
        };
        assert_eq!(
            orchestrator_mcp_install_policy(&opted_out, StartRenderMode::Tui),
            activation::orchestrator::McpInstallPolicy::Skip,
        );
        // Plain project scope keeps the legacy Skip so first-wave owns install.
        assert_eq!(
            orchestrator_mcp_install_policy(&args, StartRenderMode::Plain),
            activation::orchestrator::McpInstallPolicy::Skip,
        );
    }

    #[test]
    fn environment_all_clients_opt_in_matches_the_flag() {
        let args = start_args_default();
        temp_env::with_var("ANVIL_ALL_MCP_CLIENTS", Some("1"), || {
            assert!(force_all_mcp_clients(&args));
        });
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
