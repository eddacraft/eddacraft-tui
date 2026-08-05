use std::io::IsTerminal;

use anvil_tui::surface::Surface;
use anvil_tui::surfaces::fix_request::FixRequest;
use anvil_tui::surfaces::welcome::{QuickStartOption, WelcomeState};
use anyhow::Context;
use eddacraft_tui::theme::EddaCraftTheme;

use crate::GlobalArgs;
use crate::services::first_run::{
    create_first_run_marker, delete_first_run_marker, first_run_marker_path, is_first_run,
    should_skip_welcome,
};
use crate::services::interactive_fix::{FixOutcome, apply_fix_request};
use crate::tui::SurfaceExit;

/// Draw a loading message and wait for `duration`, processing resize events
/// so the terminal doesn't appear frozen.
fn timed_loading(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    surface_name: &str,
    message: &str,
    theme: &EddaCraftTheme,
    duration: std::time::Duration,
) -> anyhow::Result<()> {
    crate::tui::draw_loading(terminal, surface_name, message, theme)?;
    let deadline = std::time::Instant::now() + duration;
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let poll_time = remaining.min(std::time::Duration::from_millis(50));
        if crossterm::event::poll(poll_time)? {
            match crossterm::event::read()? {
                crossterm::event::Event::Resize(_, _) => {
                    crate::tui::draw_loading(terminal, surface_name, message, theme)?;
                }
                crossterm::event::Event::Key(key)
                    if key.kind == crossterm::event::KeyEventKind::Press
                        && (key.code == crossterm::event::KeyCode::Char('q')
                            || key.code == crossterm::event::KeyCode::Esc) =>
                {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Build the tutorial state for an entry point that ran discovery: threads
/// the scan results through and wires the WOW-004 read-only completion
/// re-scan (same scanner as discovery; a failed re-scan means no delta).
fn tutorial_state_with_scan(
    results: anvil_tui::surfaces::tutorial::discovery::ScanResults,
) -> anvil_tui::surfaces::tutorial::TutorialState {
    let mut state = anvil_tui::surfaces::tutorial::TutorialState::new();
    state.set_scan_results(results);
    state.set_completion_rescan(|| scan_project().ok());
    // CIB-248: the autoplay demo runs its check in-process, so it never
    // re-enters the licence-gated `anvil check` CLI with a sandbox HOME that
    // hides the user's credentials.
    state.set_autoplay_runner(crate::commands::tutorial::autoplay::in_process_check_runner());
    state
}

/// WOW-005: how the first-win surface handed control back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstWinFlow {
    /// Proceed to the tutorial path picker (declined, applied-and-continued,
    /// clean result acknowledged, or no reroute was offered).
    Continue,
    /// The user quit the whole welcome flow.
    Quit,
}

/// WOW-005: build the first-win surface state for the discovery results, or
/// `None` when the flow should land on the path picker exactly as before.
///
/// - Candidates come from `first_win_candidates` in discovery render order
///   (the stable severity-descending sort the user just saw); the first
///   candidate whose deterministic fix preview is computable is offered.
///   A transiently unpreviewable candidate (e.g. unreadable file) falls back
///   to the next one instead of dropping the whole first win.
/// - Showcase-substituted results from a scan that actually ran mean the
///   repository is clean — state that honestly instead of presenting example
///   findings as a local win (CIB-170). A showcase fallback with zero files
///   scanned is a scan *failure*, not a clean result, so no claim is made.
/// - Everything else (nothing actionable or previewable, skipped scan)
///   lands on the picker unchanged.
fn build_first_win_state(
    results: &anvil_tui::surfaces::tutorial::discovery::ScanResults,
    project_writes_gated: bool,
    preview: impl Fn(&FixRequest) -> Option<anvil_tui::surfaces::tutorial::first_win::FixPreview>,
) -> Option<anvil_tui::surfaces::tutorial::first_win::FirstWinState> {
    use anvil_tui::surfaces::tutorial::first_win::{FirstWinState, first_win_candidates};

    let candidates = first_win_candidates(results);
    for finding in candidates {
        let Some(request) = finding.fix_request() else {
            continue;
        };
        if let Some(preview) = preview(&request) {
            return Some(FirstWinState::offer(
                finding.clone(),
                preview,
                project_writes_gated,
            ));
        }
    }
    if results.is_showcase && results.files_scanned > 0 {
        return Some(FirstWinState::clean(results.files_scanned));
    }
    None
}

/// WOW-005: fold a consented apply's outcome back into the reroute state.
///
/// Applied prunes the fixed finding from the session scan results so the
/// WOW-003 picker counts and the WOW-004 completion baseline reflect
/// post-fix reality; Refused/Failed leave the results untouched and surface
/// the reason honestly. Split out of the surface loop so the outcome
/// bookkeeping is directly testable.
fn handle_apply_outcome(
    outcome: FixOutcome,
    request: &FixRequest,
    results: &mut anvil_tui::surfaces::tutorial::discovery::ScanResults,
    state: &mut anvil_tui::surfaces::tutorial::first_win::FirstWinState,
) {
    match outcome {
        FixOutcome::Applied { summary } => {
            remove_fixed_finding(results, request);
            state.mark_outcome(true, summary);
        }
        FixOutcome::Refused { reason } | FixOutcome::Failed { reason } => {
            state.mark_outcome(false, reason);
        }
    }
}

/// WOW-005: run the first-win reroute between discovery and the tutorial
/// path picker. Applies the fix only after explicit consent through the
/// shared ACTTUI chrome (unticked default; CIB-165) and prunes the applied
/// finding from `results` so the picker counts (WOW-003) and the WOW-004
/// completion baseline reflect post-fix reality — the tutorial-time fix
/// never touches the activation finding-baseline written by `anvil start`
/// (CIB-127), so it cannot confuse baseline state.
fn run_first_win_reroute(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
    results: &mut anvil_tui::surfaces::tutorial::discovery::ScanResults,
) -> anyhow::Result<FirstWinFlow> {
    use crate::services::interactive_fix::{apply_previewed_fix_request, preview_fix_request};

    // The scan roots findings at the process cwd (`scan_project`), so the
    // preview/apply containment guard uses the same root.
    let root = std::env::current_dir()?;
    let project_writes_gated = crate::install_root::project_writes_gated();
    let Some(mut state) = build_first_win_state(results, project_writes_gated, |request| {
        preview_fix_request(request, &root)
    }) else {
        return Ok(FirstWinFlow::Continue);
    };

    loop {
        crate::tui::run_surface_in(terminal, &mut state, theme)?;

        if let Some(request) = state.take_pending_apply() {
            // The apply refuses — writing nothing — unless the on-disk line
            // still matches the previewed text exactly (TOCTOU guard), so the
            // diff the user consented to is the only change that can land.
            let outcome = match state.offer.as_ref() {
                Some(offer) => apply_previewed_fix_request(&request, &offer.preview.before, &root),
                None => FixOutcome::Failed {
                    reason: "No previewed fix to apply".to_string(),
                },
            };
            handle_apply_outcome(outcome, &request, results, &mut state);
            continue;
        }

        if state.wants_continue || state.declined {
            return Ok(FirstWinFlow::Continue);
        }
        return Ok(FirstWinFlow::Quit);
    }
}

fn remove_fixed_finding(
    results: &mut anvil_tui::surfaces::tutorial::discovery::ScanResults,
    request: &FixRequest,
) {
    if let FixRequest::AntiPatternWarning {
        file,
        line,
        warning_id,
    } = request
    {
        results.findings.retain(|finding| {
            finding.file != *file
                || finding.line != Some(*line)
                || finding.warning_id.as_deref() != Some(warning_id.as_str())
        });
    }
}

#[derive(Debug, clap::Args)]
pub struct WelcomeArgs {
    /// Reset onboarding state and re-run the first-time experience.
    #[arg(long)]
    pub reset: bool,
}

pub fn run(args: &WelcomeArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let marker_path = first_run_marker_path()?;

    // DISTRIB-006 (ADR-060): the first-run marker lives at `<root>/.anvil/first-run`
    // (project-local). Under a gated ANVIL_HOME the candidate must not write or
    // delete it — the menu/onboarding still renders (read), but it leaves the
    // real project's first-run state untouched. Config seeded by onboarding is
    // separately gated in `init::run_in`.
    let project_writes_gated = crate::install_root::project_writes_gated();

    if args.reset && !project_writes_gated {
        delete_first_run_marker(&marker_path)?;
        // Also clear tutorial progress so the tutorial starts fresh.
        if let Ok(progress_path) = crate::commands::tutorial::progress_file_path() {
            match std::fs::remove_file(&progress_path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    eprintln!(
                        "[welcome] warning: failed to remove tutorial progress at {}: {err}",
                        progress_path.display()
                    );
                }
            }
        }
    }

    let first_run = is_first_run(&marker_path);

    if global.verbose {
        eprintln!("[welcome] marker_path={}", marker_path.display());
        eprintln!("[welcome] is_first_run={first_run}");
        eprintln!("[welcome] ANVIL_SKIP_WELCOME={}", should_skip_welcome());
    }

    // Env-var bypass: create marker silently and exit.
    if should_skip_welcome() {
        if !project_writes_gated && let Err(err) = create_first_run_marker(&marker_path) {
            eprintln!(
                "[welcome] warning: failed to create first-run marker at {}: {err}",
                marker_path.display()
            );
        }
        return Ok(());
    }

    if global.no_tui || !std::io::stdout().is_terminal() {
        print_plain_welcome(start_prompts_sign_in());
        // INSIGHTS-005: the nudge rides the plain closing output too.
        print_welcome_insights_hint(project_writes_gated);
        // Telemetry disclosure rides the plain closing output too. On this
        // branch stdout may be piped: the notice prints, but notice-shown is
        // persisted only when a human could actually see it (stdout is a
        // TTY), so a non-TTY first run can never unlock the beacon.
        crate::telemetry::print_first_run_disclosure(std::io::stdout().is_terminal());
        if !project_writes_gated {
            create_first_run_marker(&marker_path)?;
        }
        return Ok(());
    }

    let mut terminal = crate::tui::setup_terminal()?;
    let theme = EddaCraftTheme;

    let result = if first_run {
        match run_onboarding(&mut terminal, &theme) {
            Ok(OnboardingOutcome::Quit) => Ok(()),
            Ok(OnboardingOutcome::Tutorial | OnboardingOutcome::Configured) => {
                match run_discovery(&mut terminal, &theme)? {
                    Some(mut results) => {
                        // WOW-005: land on the highest-value actionable real
                        // finding first; declining falls through to the
                        // tutorial path picker exactly as before.
                        if run_first_win_reroute(&mut terminal, &theme, &mut results)?
                            == FirstWinFlow::Quit
                        {
                            Ok(())
                        } else {
                            let mut tutorial_state = tutorial_state_with_scan(results);
                            let exit = run_tutorial_with_fix(
                                &mut terminal,
                                &theme,
                                &mut tutorial_state,
                                global.verbose,
                            )?;
                            if exit == SurfaceExit::Quit {
                                Ok(())
                            } else {
                                run_welcome_hub(&mut terminal, &theme, global.verbose)
                            }
                        }
                    }
                    None => run_welcome_hub(&mut terminal, &theme, global.verbose),
                }
            }
            Ok(OnboardingOutcome::Skip) => run_welcome_hub(&mut terminal, &theme, global.verbose),
            Err(e) => Err(e),
        }
    } else {
        run_welcome_hub(&mut terminal, &theme, global.verbose)
    };

    // Always teardown terminal, even on error.
    let teardown_result = crate::tui::teardown_terminal(&mut terminal);

    // Write marker on first run only — don't clobber an existing marker's
    // creation timestamp on subsequent launches.
    if first_run
        && !project_writes_gated
        && let Err(err) = create_first_run_marker(&marker_path)
    {
        eprintln!(
            "[welcome] warning: failed to create first-run marker at {}: {err}",
            marker_path.display()
        );
    }

    // Prefer the app error over the teardown error.
    result.and(teardown_result)?;

    // UJ-001: printed after terminal teardown so the next step lands in
    // scrollback once the TUI session ends.
    println!("{}", welcome_next_step(start_prompts_sign_in()));
    // INSIGHTS-005: the first-week nudge rides the closing output too.
    print_welcome_insights_hint(project_writes_gated);
    // The telemetry disclosure must strictly precede any first beacon; the
    // TUI path just ran on a real terminal, so the notice both prints and
    // is recorded as shown.
    crate::telemetry::print_first_run_disclosure(true);

    Ok(())
}

/// INSIGHTS-005: render the first-week insights nudge to the `anvil welcome`
/// closing output if it is due. Reuses the INSIGHTS-004 contract verbatim
/// (14-day cohort window, once per week, suppressed after an `anvil insights`
/// run, shared `.anvil/insights-hint.json` state — no new state file or
/// rate-limit bucket), so the nudge stays at-most-once-per-week across
/// `status`, `watch`, and `welcome`. The gate is threaded straight into the
/// canonical `first_week_insights_hint`, which returns `None` — with no read
/// and no write — under a gated project root (DISTRIB-006 / ADR-060), so no
/// surface can regress by forgetting the guard (CIB-133).
///
/// Resolving the hint consumes the once-per-week marker, so this is only ever
/// called on a surface that actually prints it (both `welcome` closing paths
/// do).
fn print_welcome_insights_hint(project_writes_gated: bool) {
    let root =
        crate::util::workspace_root().unwrap_or_else(|_| std::path::Path::new(".").to_path_buf());
    if let Some(hint) = crate::insights::first_week_hint::first_week_insights_hint(
        &root,
        chrono::Utc::now(),
        project_writes_gated,
    ) {
        println!();
        println!("{hint}");
    }
}

/// Outcome of the first-run onboarding flow, used by the caller to decide
/// what to show next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardingOutcome {
    /// User completed guided setup or config already existed — proceed normally.
    Configured,
    /// User chose to skip to tutorial — start tutorial immediately.
    Tutorial,
    /// User chose to skip entirely — go to welcome hub.
    Skip,
    /// User pressed quit — exit the entire program.
    Quit,
}

fn run_onboarding(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<OnboardingOutcome> {
    use anvil_tui::surfaces::onboarding::{OnboardingChoice, OnboardingWelcomeState};

    // Check for stale tutorial progress from a previous install and reset.
    if let Ok(progress_path) = crate::commands::tutorial::progress_file_path()
        && progress_path.exists()
    {
        timed_loading(
            terminal,
            "Setup",
            "Previous tutorial progress found \u{2014} resetting for fresh install.",
            theme,
            std::time::Duration::from_millis(600),
        )?;
        if let Err(e) = std::fs::remove_file(&progress_path) {
            eprintln!("[welcome] warning: could not remove tutorial progress: {e}");
        }
    }

    let mut onboarding = OnboardingWelcomeState::new();
    let exit = crate::tui::run_surface_in(terminal, &mut onboarding, theme)?;

    if exit == SurfaceExit::Quit && onboarding.chosen.is_none() {
        return Ok(OnboardingOutcome::Quit);
    }

    match onboarding.chosen {
        Some(OnboardingChoice::GuidedSetup) => {
            if run_guided_init(terminal, theme)? {
                Ok(OnboardingOutcome::Quit)
            } else {
                Ok(OnboardingOutcome::Configured)
            }
        }
        Some(OnboardingChoice::SkipToTutorial) => Ok(OnboardingOutcome::Tutorial),
        Some(OnboardingChoice::SkipEntirely) | None => Ok(OnboardingOutcome::Skip),
    }
}

/// Returns `true` if the user quit out of the landing screen and the caller
/// should stop the onboarding flow.
fn run_guided_init(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<bool> {
    use anvil_tui::surfaces::onboarding;

    // Skip init if config already exists.
    if onboarding::config_exists() {
        timed_loading(
            terminal,
            "Init",
            "anvil configuration detected \u{2014} skipping setup.",
            theme,
            std::time::Duration::from_millis(200),
        )?;
        return Ok(false);
    }

    let checks = crate::commands::defaults::default_available_checks();
    let mut init_state = anvil_tui::surfaces::init::InitState::new(checks);
    let exit = crate::tui::run_surface_in(terminal, &mut init_state, theme)?;

    if exit == SurfaceExit::Quit && !init_state.confirmed {
        // User quit the init wizard — don't show error, just fall through.
        return Ok(false);
    }

    if init_state.confirmed {
        let checks: Vec<String> = if init_state.config.checks.is_empty() {
            crate::commands::defaults::default_check_names()
        } else {
            init_state.config.checks
        };

        let init_root = if init_state.config.directory == "." {
            std::path::PathBuf::from(".")
        } else {
            std::path::PathBuf::from(&init_state.config.directory)
        };
        if let Err(e) = std::fs::create_dir_all(&init_root) {
            eprintln!("[welcome] warning: failed to create directory: {e}");
        }

        let mut config = crate::commands::init::AnvilConfig::default();
        config.format = crate::commands::init::format_label(init_state.config.format);
        config.checks.clone_from(&checks);

        match crate::commands::init::generate_config(&config, &init_root) {
            Ok(generated) => {
                // Name the file actually written (always `.anvilrc`) rather than
                // a hardcoded literal, so the landing summary can never drift
                // from what `generate_config` created (CIB-171).
                let config_path = generated
                    .config_path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or(crate::commands::init::CONFIG_FILE_NAME)
                    .to_string();
                let summary = onboarding::InitCompleteSummary {
                    config_path,
                    plans_dir: format!("{}/", config.planning_dir),
                    cache_dir: ".anvil/cache/".to_string(),
                    gitignore_updated: generated.gitignore_updated,
                    checks_enabled: checks,
                };
                let mut landing = onboarding::InitCompleteState::new(summary);
                crate::tui::run_surface_in(terminal, &mut landing, theme)?;
                // InitCompleteState::should_quit returns true for both Enter
                // (wants_continue) and q/Esc; wants_continue is the authoritative
                // signal for whether to proceed past the landing screen.
                if !landing.wants_continue {
                    return Ok(true);
                }
            }
            Err(e) => {
                timed_loading(
                    terminal,
                    "Init",
                    &format!("Warning: could not save config: {e}"),
                    theme,
                    std::time::Duration::from_millis(400),
                )?;
            }
        }
    }

    Ok(false)
}

fn run_discovery(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<Option<anvil_tui::surfaces::tutorial::discovery::ScanResults>> {
    use anvil_tui::surfaces::tutorial::discovery::{DiscoveryState, ScanResults};
    use anvil_tui::surfaces::tutorial::showcase;

    let mut discovery = DiscoveryState::new();
    discovery.tick();
    draw_discovery_scanning(terminal, theme, &discovery)?;

    let results = match scan_project() {
        Ok(results) if results.findings.is_empty() => {
            // Clean project — show showcase examples so user sees capabilities.
            let findings = showcase::showcase_findings();
            ScanResults {
                findings,
                files_scanned: results.files_scanned,
                duration_ms: results.duration_ms,
                truncated: false,
                // Keep the real scan's gitignore provenance even when we
                // swap in showcase findings — the skipped files were still
                // skipped, and hiding that would defeat SCAN-004.
                files_skipped_by_ignore: results.files_skipped_by_ignore,
                // CIB-170: mark the substituted findings as showcase examples
                // so discovery renders the "Example findings" banner/badge and
                // the user cannot mistake the demo secret for a real leak.
                is_showcase: true,
            }
        }
        Ok(results) => results,
        Err(e) => {
            // Scan failed — surface the error and fall back to showcase mode.
            eprintln!(
                "Warning: failed to scan project for discovery findings: {e}. Falling back to showcase examples."
            );
            let findings = showcase::showcase_findings();
            ScanResults {
                findings,
                files_scanned: 0,
                duration_ms: 0,
                truncated: false,
                files_skipped_by_ignore: 0,
                // CIB-170: scan-failure fallback is also showcase data.
                is_showcase: true,
            }
        }
    };
    discovery.set_results(results);

    let exit = crate::tui::run_surface_in(terminal, &mut discovery, theme)?;

    Ok(discovery_outcome(
        exit,
        discovery.wants_continue,
        discovery.results,
    ))
}

fn draw_discovery_scanning(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
    discovery: &anvil_tui::surfaces::tutorial::discovery::DiscoveryState,
) -> anyhow::Result<()> {
    terminal.draw(|frame| {
        let content = anvil_tui::shell::render_shell(
            frame,
            frame.area(),
            discovery.surface_name(),
            discovery.help_text(),
            theme,
        );
        discovery.render(frame, content, theme);
    })?;
    Ok(())
}

/// Classify how the discovery surface exited into the caller's result.
///
/// `Esc` (`SurfaceExit::Back`) on the results/continue screen backs out to the
/// caller rather than advancing into the tutorial — treating `Back` as
/// "continue" was the CIB-171 navigation trap. Quitting without an explicit
/// Enter-to-continue also backs out; only an explicit continue carries the
/// scan results forward.
fn discovery_outcome(
    exit: SurfaceExit,
    wants_continue: bool,
    results: Option<anvil_tui::surfaces::tutorial::discovery::ScanResults>,
) -> Option<anvil_tui::surfaces::tutorial::discovery::ScanResults> {
    match exit {
        SurfaceExit::Back => None,
        SurfaceExit::Quit if !wants_continue => None,
        SurfaceExit::Quit => results,
    }
}

/// Scan the current project for real secret and antipattern findings.
const SCAN_MAX_FILES: usize = 500;
const SCAN_MAX_FILE_SIZE: u64 = 512 * 1024; // 512 KB

// SCAN-003: first-run pool size = ANVIL_SCAN_THREADS (positive, clamped to cpus)
// or else min(cpus, DEFAULT_FIRST_RUN_THREAD_CAP). Keeps the TUI responsive.
const ANVIL_SCAN_THREADS_ENV: &str = "ANVIL_SCAN_THREADS";
const DEFAULT_FIRST_RUN_THREAD_CAP: usize = 4;

/// SCAN-003: resolve the desired thread count for the first-run scan.
///
/// Precedence:
///   1. `ANVIL_SCAN_THREADS` (positive integer) — honoured verbatim,
///      still clamped at `num_cpus::get()` so an over-large value cannot
///      schedule more workers than the host has cores.
///   2. Otherwise: `min(num_cpus::get(), DEFAULT_FIRST_RUN_THREAD_CAP)`.
///
/// Returns `None` when the input arguments cannot produce a positive
/// thread count (which would mean the caller should fall back to the
/// global pool — currently unreachable, but kept explicit for callers).
fn resolve_first_run_thread_count(env_value: Option<&str>, available_cpus: usize) -> Option<usize> {
    let cpus = available_cpus.max(1);
    let from_env = env_value
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .map(|n| n.min(cpus));
    let resolved = from_env.unwrap_or_else(|| cpus.min(DEFAULT_FIRST_RUN_THREAD_CAP));
    (resolved > 0).then_some(resolved)
}

/// SCAN-003: resolve the first-run thread budget from `ANVIL_SCAN_THREADS`
/// (default `min(num_cpus, DEFAULT_FIRST_RUN_THREAD_CAP)`). Shared by the
/// Phase 1a parallel walker and the Phase 2 rayon pool so both honour the same
/// "don't fight the TUI for cores" cap from one place.
fn first_run_thread_count() -> Option<usize> {
    resolve_first_run_thread_count(
        std::env::var(ANVIL_SCAN_THREADS_ENV).ok().as_deref(),
        num_cpus::get(),
    )
}

/// SCAN-003: build a scoped rayon pool sized for the first-run scan.
///
/// Returns the pool when construction succeeds; on failure the caller
/// should fall back to the global rayon pool — failure here is a
/// best-effort fallback, never fatal to the scan.
fn build_first_run_pool() -> Option<rayon::ThreadPool> {
    let threads = first_run_thread_count()?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(|idx| format!("anvil-scan-{idx}"))
        .build()
        .ok()
}

/// Decide whether a single `ignore::DirEntry` should be scanned. Used by both
/// the gitignore-aware primary walker and the always-scan allowlist walker so
/// filtering stays consistent between the two passes.
fn candidate_path(
    entry: &ignore::DirEntry,
    cwd: &std::path::Path,
    filter: &anvil_checks::filter::ScanFilter,
) -> Option<(std::path::PathBuf, String)> {
    let ft = entry.file_type()?;
    if !ft.is_file() {
        return None;
    }
    let path = entry.path();
    if !filter.includes(path) {
        return None;
    }
    if anvil_checks::filter::is_binary_path(path) {
        return None;
    }
    // Lockfiles are NOT excluded here: `scan_one` runs them through the
    // restricted URL-credential-only secret scan (GH #2584). Excluding them
    // would also drop the chance to flag a credential committed to a `resolved`
    // URL.
    // `.env*` files are the designated place to keep local secrets, so
    // reporting their contents as findings is noise the user cannot action
    // (GH #2584). They are almost always gitignored — and frequently by a
    // *global* gitignore — so a gitignore-respecting walk cannot reliably tell
    // a committed `.env` from a local one; exempt them unconditionally.
    if anvil_checks::surface::env::is_env_file(path) {
        return None;
    }
    // Size check is repeated at read-time (scan_one) because the file can
    // grow between Phase 1 and Phase 2. This pre-check is an early-exit so
    // the rayon pool doesn't even see huge files.
    if let Ok(meta) = entry.metadata()
        && meta.len() > SCAN_MAX_FILE_SIZE
    {
        return None;
    }
    let rel_path = path
        .strip_prefix(cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    Some((path.to_path_buf(), rel_path))
}

/// SCAN-006: collect the gitignore-blind Phase 1a candidates in parallel.
///
/// Phase 1a is **uncapped** — it must see the whole tree to build the
/// `all_candidates` set the SCAN-004 skip count subtracts from — so it is the
/// dominant walk cost and the one that benefits most from parallelism
/// (especially on large / cold-cache repos). It is also the *safe* walk to
/// parallelise: its output feeds an order-independent `HashSet` plus the
/// `ALWAYS_SCAN_FILENAMES` allowlist filter, and final user-visible ordering is
/// imposed later by `findings.sort_by`, so concurrency introduces no
/// determinism hazard. (The capped Phase 1b walk stays sequential — see
/// `scan_project_at` — because its early-break at `SCAN_MAX_FILES` and
/// deterministic truncation depend on an ordered single-threaded walk.)
///
/// `candidate_path` (incl. its `metadata()` stat) runs concurrently across the
/// walker's threads; the `mpsc` channel collects results with low contention.
/// Threads are capped with the SCAN-003 first-run budget (`ANVIL_SCAN_THREADS`,
/// default `min(num_cpus, 4)`) so the walk does not fight the TUI for cores.
fn collect_blind_candidates_parallel(
    cwd: &std::path::Path,
    filter: &anvil_checks::filter::ScanFilter,
) -> Vec<(std::path::PathBuf, String)> {
    let mut builder = ignore::WalkBuilder::new(cwd);
    builder
        .follow_links(false)
        .standard_filters(false)
        .hidden(false);
    if let Some(threads) = first_run_thread_count() {
        builder.threads(threads);
    }

    let filter_for_prune = filter.clone();
    builder.filter_entry(move |entry| {
        // Only prune directories — always descend into files so the filename
        // allowlist can match them.
        if entry.file_type().is_none_or(|ft| !ft.is_dir()) {
            return true;
        }
        filter_for_prune.includes(entry.path())
    });

    // Safety/lifetime note: `build_parallel().run()` is synchronous — it joins
    // every worker thread before returning — so the `&Path` (`cwd`) and
    // `&ScanFilter` (`filter`) borrows captured by the per-thread closure stay
    // valid for the whole walk despite the `Send` boundary. Do NOT switch to an
    // async / detached walk variant without giving these owned copies first.
    let (tx, rx) = std::sync::mpsc::channel();
    builder.build_parallel().run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            if let Ok(entry) = result
                && let Some(candidate) = candidate_path(&entry, cwd, filter)
            {
                let _ = tx.send(candidate);
            }
            ignore::WalkState::Continue
        })
    });
    drop(tx);
    rx.iter().collect()
}

/// Read and scan one file. Returns `None` when the file cannot be read or has
/// grown past the size cap since the Phase 1 metadata check (TOCTOU guard).
fn scan_one(
    path: &std::path::Path,
    rel_path: &str,
    secret_config: &anvil_checks::secret::types::SecretCheckConfig,
) -> Option<Vec<anvil_tui::surfaces::tutorial::discovery::Finding>> {
    use anvil_tui::surfaces::tutorial::discovery::{Finding, FindingSeverity, FindingSource};

    // TOCTOU re-check: the size verified in Phase 1 could have grown.
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > SCAN_MAX_FILE_SIZE {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;

    let mut local: Vec<Finding> = Vec::new();

    let secret_hits =
        anvil_checks::secret::scanner::scan_content(&content, rel_path, secret_config);
    for hit in &secret_hits {
        local.push(Finding {
            file: hit.file.clone(),
            line: Some(hit.line),
            severity: FindingSeverity::Error,
            source: FindingSource::Secret,
            title: format!("Secret detected: {}", hit.pattern_name),
            message: hit.redacted_line.clone(),
            suggestion: "Move the value to an environment variable or secrets manager.".to_string(),
            warning_id: None,
        });
    }

    // Lockfiles get the secret scan (URL-credential-only, handled inside
    // `scan_content`) but no antipattern pass — generated dependency metadata
    // has no meaningful antipatterns, and scanning it is pure noise (GH #2584).
    if anvil_checks::filter::is_lockfile(path) {
        return Some(local);
    }

    let ap_result = anvil_checks::antipattern::scanner::scan_file(rel_path, &content, None);
    for warning in &ap_result.warnings {
        if warning.suppressed.is_some() {
            continue;
        }
        local.push(Finding {
            file: warning.location.file.clone(),
            line: Some(warning.location.line),
            severity: match warning.severity {
                anvil_checks::antipattern::types::WarningSeverity::Error => FindingSeverity::Error,
                anvil_checks::antipattern::types::WarningSeverity::Warning => {
                    FindingSeverity::Warning
                }
                anvil_checks::antipattern::types::WarningSeverity::Info => FindingSeverity::Info,
            },
            source: FindingSource::AntiPattern,
            title: warning.title.clone(),
            message: warning.message.clone(),
            suggestion: warning.suggestion.clone(),
            warning_id: Some(warning.id.clone()),
        });
    }

    Some(local)
}

fn scan_project() -> anyhow::Result<anvil_tui::surfaces::tutorial::discovery::ScanResults> {
    let cwd = std::env::current_dir()?;
    // `ANVIL_SCAN_ALL` bypasses gitignore (see `scan_project_at`). Parsed here
    // so the discovery logic stays a pure function of `(root, scan_all)` and
    // can be tested against a temp tree without mutating the process env or
    // cwd.
    let scan_all = std::env::var("ANVIL_SCAN_ALL")
        .is_ok_and(|v| !matches!(v.trim().to_ascii_lowercase().as_str(), "" | "0" | "false"));
    Ok(scan_project_at(&cwd, scan_all))
}

/// Discover and scan candidate files under `cwd`. Split out from
/// `scan_project` so the gitignore-skip accounting (SCAN-004) is testable
/// against a temp tree without touching the process cwd or environment.
/// Infallible: read/permission errors are folded into per-file skips rather
/// than aborting the scan, so the only fallible step (`current_dir`) stays in
/// the `scan_project` wrapper.
#[allow(clippy::too_many_lines)]
fn scan_project_at(
    cwd: &std::path::Path,
    scan_all: bool,
) -> anvil_tui::surfaces::tutorial::discovery::ScanResults {
    use anvil_checks::filter::{BUILD_ARTEFACT_DIRS, ScanFilter, is_always_scan_filename};
    use anvil_tui::surfaces::tutorial::discovery::{Finding, ScanResults};
    use rayon::prelude::*;
    use std::collections::HashSet;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::atomic::{AtomicUsize, Ordering};

    let start = std::time::Instant::now();
    let secret_config = anvil_checks::secret::types::SecretCheckConfig::default();

    // Correctness excludes (from `ScanFilter::default_excludes()`) + the
    // opt-in build-artefact set. `default_with` composes the canonical
    // defaults with our extras so the welcome-flow filter cannot drift from
    // `ScanFilter::default_excludes()` when that set changes.
    let filter = ScanFilter::default_with(
        BUILD_ARTEFACT_DIRS
            .iter()
            .map(|d| format!("{d}/"))
            .collect(),
    );

    let mut candidates: Vec<(std::path::PathBuf, String)> = Vec::new();
    let mut seen: HashSet<std::path::PathBuf> = HashSet::new();
    let mut truncated = false;

    // SCAN-004: every scan candidate the gitignore-blind Phase 1a walk can
    // see. Phase 1b's gitignore-respecting set (`seen`) is later subtracted
    // from this to count files `.gitignore` excluded. Only populated when
    // `!scan_all` (Phase 1a runs only then); when `scan_all` is set nothing is
    // gitignore-excluded, so an empty set yields a count of 0.
    let mut all_candidates: HashSet<std::path::PathBuf> = HashSet::new();

    // Phase 1a: always-scan allowlist pass. Bypasses standard gitignore
    // filters so leaked credential files (`id_rsa`, `credentials.json`, …) in
    // gitignored locations are still caught — precisely the class of secret the
    // first-run scan exists to flag — and builds `all_candidates` for the
    // SCAN-004 skip count. `.env*` files are intentionally excluded from the
    // first-run scan entirely (GH #2584) — `candidate_path` drops them, so
    // neither phase scans them. A secret committed to a tracked `.env` is
    // caught by `anvil gate`/`audit` and the save-time intercept, not here.
    //
    // SCAN-006: the walk runs in parallel (`collect_blind_candidates_parallel`)
    // because it is uncapped and order-free; the sequential post-pass below
    // just folds its results into the order-independent sets. `filter_entry`
    // (inside the helper) prunes build-artefact dirs so the gitignore-blind
    // walk can't dominate runtime by descending into node_modules/target/...
    // Skipped when `scan_all` is set because Phase 1b then covers everything
    // (including `.env`) via the same walker.
    if !scan_all {
        for candidate in collect_blind_candidates_parallel(cwd, &filter) {
            // Record every gitignore-blind candidate so Phase 1b's
            // gitignore-respecting set can be subtracted from it (SCAN-004).
            // `candidate_path` (the *same* predicate, incl. the size stat, as
            // the scan set) is applied inside the parallel helper, so the only
            // axis that differs between the two sets is gitignore.
            all_candidates.insert(candidate.0.clone());
            // No cap on the allowlist pass: ALWAYS_SCAN_FILENAMES is a small,
            // rare-filename set of credential files force-scanned even when
            // gitignored — exactly the class of leak the first-run scan exists
            // to flag.
            if is_always_scan_filename(&candidate.0) && seen.insert(candidate.0.clone()) {
                candidates.push(candidate);
            }
        }
    }

    // Phase 1b: general walk. Honours gitignore (unless `scan_all`) and
    // stops at SCAN_MAX_FILES. `hidden(false)` is deliberate — dotfile
    // directories (`.config`, `.secrets`, ...) must be descended into so
    // secret-bearing dotfiles can be discovered; gitignore still prunes
    // anything listed there.
    let walker = ignore::WalkBuilder::new(cwd)
        .follow_links(false)
        .standard_filters(!scan_all)
        .hidden(false)
        .build();

    let mut iter = walker.filter_map(Result::ok);
    for entry in iter.by_ref() {
        if let Some(candidate) = candidate_path(&entry, cwd, &filter) {
            if seen.insert(candidate.0.clone()) {
                candidates.push(candidate);
            }
            if candidates.len() >= SCAN_MAX_FILES {
                // Decide truncation honestly: did the walker actually have
                // more matching entries beyond the cap?
                truncated = iter
                    .by_ref()
                    .any(|e| candidate_path(&e, cwd, &filter).is_some());
                break;
            }
        }
    }

    // Phase 2: read + scan files in parallel. Each worker runs inside
    // catch_unwind so a panic in one scanner doesn't propagate through the
    // rayon collect and tear down the TUI terminal state.
    //
    // SCAN-003: the parallel collect runs inside a scoped rayon pool capped
    // at `min(num_cpus, DEFAULT_FIRST_RUN_THREAD_CAP)` (override via
    // `ANVIL_SCAN_THREADS`). Falling back to the global pool when the
    // builder fails preserves the legacy behaviour rather than aborting
    // discovery.
    let panics = AtomicUsize::new(0);
    let read_failures = AtomicUsize::new(0);

    let scan_closure = || -> Vec<Vec<Finding>> {
        candidates
            .par_iter()
            .map(|(path, rel_path)| {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    scan_one(path, rel_path, &secret_config)
                }));
                match result {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        read_failures.fetch_add(1, Ordering::Relaxed);
                        Vec::new()
                    }
                    Err(_) => {
                        panics.fetch_add(1, Ordering::Relaxed);
                        Vec::new()
                    }
                }
            })
            .collect()
    };

    let all_findings: Vec<Vec<Finding>> = if let Some(pool) = build_first_run_pool() {
        pool.install(scan_closure)
    } else {
        scan_closure()
    };

    // files_scanned counts files that were successfully read + scanned, not
    // raw candidates: panics and read/TOCTOU failures drop out.
    let files_scanned = candidates
        .len()
        .saturating_sub(panics.load(Ordering::Relaxed))
        .saturating_sub(read_failures.load(Ordering::Relaxed));

    let mut findings: Vec<Finding> = all_findings.into_iter().flatten().collect();

    // Surface custom-pattern compile errors once at the discovery boundary so
    // misconfigured user regexes don't silently produce zero hits across every
    // scanned file. This compile is for error reporting only — scan_one still
    // performs its own per-file compilation. Eliminating that redundancy
    // requires threading pre-compiled patterns into scan_one/scan_content and
    // is tracked separately (EAMIG follow-on to C-002).
    let (_, pattern_errors) =
        anvil_checks::secret::patterns::compile_custom_patterns(&secret_config.custom_patterns);
    for err in &pattern_errors {
        findings.push(Finding {
            file: ".anvilrc".to_string(),
            line: None,
            severity: anvil_tui::surfaces::tutorial::discovery::FindingSeverity::Warning,
            source: anvil_tui::surfaces::tutorial::discovery::FindingSource::Secret,
            title: "Custom secret pattern failed to compile".to_string(),
            message: err.clone(),
            suggestion: "Fix or remove the offending pattern in your secret-scan configuration."
                .to_string(),
            warning_id: None,
        });
    }

    // Deterministic ordering: severity desc, then file asc, line asc, title
    // asc. Without this the rayon collect order leaks thread scheduling into
    // user-visible output and snapshot tests.
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.title.cmp(&b.title))
    });

    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis() as u64;

    // SCAN-004: gitignore-skipped count = candidates in the blind walk not in
    // `seen` (filtered membership, not set-size delta). Zero when truncated or
    // when scan_all disabled gitignore.
    let files_skipped_by_ignore = if truncated || scan_all {
        0
    } else {
        all_candidates.iter().filter(|p| !seen.contains(*p)).count()
    };

    ScanResults {
        findings,
        files_scanned,
        duration_ms,
        truncated,
        files_skipped_by_ignore,
        is_showcase: false,
    }
}

/// Try to start a file watcher for the tutorial. Returns the receiver and
/// handle on success, or `None` if the watcher cannot be started (e.g.
/// inotify limit reached, no project directory).
/// The tutorial's live-verification watcher: change receiver plus its handle.
type TutorialWatcher = (
    std::sync::mpsc::Receiver<anvil_kernel::watcher::events::ChangeBatch>,
    anvil_kernel::watcher::WatcherHandle,
);

fn bind_welcome_tutorial_workspace(
    tutorial_state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    workspace_root: &std::path::Path,
) -> anyhow::Result<()> {
    tutorial_state
        .bind_working_root(workspace_root)
        .context("binding welcome tutorial workspace")
}

fn try_start_tutorial_watcher(workspace_root: &std::path::Path) -> Option<TutorialWatcher> {
    let config = anvil_kernel::watcher::WatcherConfig {
        root: workspace_root.to_path_buf(),
        debounce_window: std::time::Duration::from_millis(300),
        filter: Some(anvil_kernel::watcher::filter::FileFilter::default()),
        ..Default::default()
    };
    match anvil_kernel::watcher::start_watcher(&config, None) {
        Ok((handle, rx, _diag)) => Some((rx, handle)),
        Err(_) => None,
    }
}

/// Run the tutorial surface in a loop, handling 'f' fix requests and watch
/// demo launches. When a file watcher is available (WELCOME-013), file
/// changes trigger automatic re-verification on watched steps. When the
/// user triggers a watch demo step (WELCOME-014), the watch demo surface
/// is launched and the tutorial resumes afterward.
/// Start (or restart) the tutorial file watcher, falling back to static mode.
///
/// The tutorial re-establishes its watcher whenever an autoplay sandbox is torn
/// down — on ordinary teardown and, since CIB-248, on demo failure recovery.
/// Without a watcher the tutorial degrades to informational steps rather than
/// silently failing live verification.
fn restart_tutorial_watcher(
    tutorial_state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    workspace_root: &std::path::Path,
) -> Option<TutorialWatcher> {
    let watcher = try_start_tutorial_watcher(workspace_root);
    if watcher.is_none() {
        tutorial_state.enable_static_mode_with_reason(
            anvil_tui::surfaces::tutorial::STATIC_MODE_WATCHER_UNAVAILABLE,
        );
    }
    watcher
}

fn run_tutorial_with_fix(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
    tutorial_state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    verbose: bool,
) -> anyhow::Result<SurfaceExit> {
    use crate::commands::tutorial::autoplay::AutoplaySandbox;

    let workspace_root = crate::util::workspace_root()?;
    bind_welcome_tutorial_workspace(tutorial_state, &workspace_root)?;

    // Try to start a file watcher for live verification (WELCOME-013).
    // If unavailable, the tutorial enters static mode (all steps become
    // informational press-enter-to-continue, commands are not executed).
    let mut autoplay_sandbox: Option<AutoplaySandbox> = None;
    let mut watcher = restart_tutorial_watcher(tutorial_state, &workspace_root);

    loop {
        let file_rx = watcher.as_ref().map(|(rx, _)| rx);

        // Use the tutorial-specific loop that drains file-change events
        // and checks for wants_watch_demo exit.
        if let Err(error) = crate::tui::run_tutorial_in(terminal, tutorial_state, file_rx, theme) {
            tutorial_state.abort_autoplay_session();
            return Err(error);
        }

        // CIB-248: a failed demo step must not end the welcome session. Tear
        // the sandbox down, tell the user what happened, and hand them back to
        // the path picker still inside the TUI.
        if let Some(failure) = tutorial_state.take_autoplay_failure() {
            drop(autoplay_sandbox.take());
            tutorial_state.recover_from_autoplay_failure(format!(
                "The hands-free demo stopped: {failure}. Your repo was not touched — pick a path to continue."
            ));
            bind_welcome_tutorial_workspace(tutorial_state, &workspace_root)?;
            watcher = restart_tutorial_watcher(tutorial_state, &workspace_root);
            continue;
        }

        if tutorial_state.take_autoplay_teardown_requested() {
            drop(autoplay_sandbox.take());
            bind_welcome_tutorial_workspace(tutorial_state, &workspace_root)?;
            watcher = restart_tutorial_watcher(tutorial_state, &workspace_root);
            continue;
        }

        // Reset transient exit flags to prevent stale state from causing
        // immediate re-exit on the next loop iteration.
        tutorial_state.wants_back = false;

        if tutorial_state.wants_autoplay_setup {
            drop(watcher.take());
            autoplay_sandbox = Some(AutoplaySandbox::new()?);
            let sandbox = autoplay_sandbox.as_ref().expect("sandbox inserted above");
            tutorial_state.start_autoplay_in(sandbox.root())?;
            continue;
        }

        if tutorial_state.wants_watch_demo {
            tutorial_state.wants_watch_demo = false;
            let active_demo = crate::commands::tutorial::watch_demo_mode(tutorial_state)
                == crate::commands::tutorial::WatchDemoMode::Autoplay;

            // Drop the tutorial watcher before launching the watch demo
            // to avoid two concurrent watchers over the same root, which
            // would double inotify descriptor usage (C-001).
            drop(watcher.take());

            // Treat failures as best-effort so the welcome/tutorial flow
            // resumes instead of aborting. Routes through format_user_error
            // so wrapped notify::Error paths only leak when --verbose is
            // explicit (see #1017).
            match run_watch_demo_from_tutorial(
                terminal,
                theme,
                tutorial_state,
                autoplay_sandbox.as_ref(),
                &workspace_root,
            ) {
                Ok(anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::Continue) => {
                    tutorial_state.advance_step();
                }
                Ok(
                    anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::HandBack
                    | anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::CycleComplete,
                ) => {}
                Err(err) if active_demo => {
                    tutorial_state.abort_autoplay_session();
                    return Err(err);
                }
                Err(err) => eprintln!(
                    "Watch demo unavailable: {}",
                    crate::util::format_user_error(&err, verbose)
                ),
            }

            // Restart the tutorial watcher for remaining steps.
            watcher = if tutorial_state.autoplay_session_active() {
                None
            } else {
                try_start_tutorial_watcher(&workspace_root)
            };
            continue;
        }

        if let Some(request) = tutorial_state.pending_fix.take() {
            match apply_fix_request(&request, None) {
                FixOutcome::Applied { summary } => {
                    if let Some(results) = tutorial_state.scan_results.as_mut() {
                        remove_fixed_finding(results, &request);
                    }
                    if let Some(results) = tutorial_state.domain_findings.as_mut() {
                        remove_fixed_finding(results, &request);
                    }
                    tutorial_state.resuming_notice = Some(summary);
                }
                FixOutcome::Refused { reason } | FixOutcome::Failed { reason } => {
                    tutorial_state.resuming_notice = Some(reason);
                }
            }
            continue;
        }

        if tutorial_state.should_quit {
            return Ok(SurfaceExit::Quit);
        }
        return Ok(SurfaceExit::Back);
    }
}

/// Launch the watch mode demo from within a tutorial (WELCOME-014).
/// Starts the kernel watcher, runs the demo surface with guided overlay,
/// and cleans up the watcher on exit.
fn run_watch_demo_from_tutorial(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
    tutorial_state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    sandbox: Option<&crate::commands::tutorial::autoplay::AutoplaySandbox>,
    workspace_root: &std::path::Path,
) -> anyhow::Result<anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome> {
    use anvil_tui::surfaces::watch::{WatchData, WatchStats, WatchStatus};

    crate::tui::draw_loading(terminal, "Watch Demo", "Starting watch mode\u{2026}", theme)?;

    let active_demo = crate::commands::tutorial::watch_demo_mode(tutorial_state)
        == crate::commands::tutorial::WatchDemoMode::Autoplay;
    let workspace_root =
        match crate::commands::tutorial::watch_demo_root(tutorial_state, sandbox, || {
            Ok(workspace_root.to_path_buf())
        }) {
            Ok(root) => root,
            Err(error) if active_demo => return Err(error),
            Err(_) => {
                timed_loading(
                    terminal,
                    "Watch Demo",
                    "Could not determine project root \u{2014} skipping demo.",
                    theme,
                    std::time::Duration::from_secs(1),
                )?;
                return Ok(anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::Continue);
            }
        };

    let watcher_config = anvil_kernel::watcher::WatcherConfig {
        root: workspace_root.clone(),
        debounce_window: std::time::Duration::from_millis(300),
        filter: Some(anvil_kernel::watcher::filter::FileFilter::default()),
        ..Default::default()
    };

    let watch_config = anvil_kernel::watch::WatchConfig {
        root: workspace_root,
        architecture_config: None,
        watcher: watcher_config,
        include_patterns: vec!["**/*".to_string()],
        exclude_patterns: Vec::new(),
        warmup_paths: Vec::new(),
    };

    let (event_tx, event_rx) = std::sync::mpsc::channel();

    let handle = match anvil_kernel::watch::run_watch(&watch_config, event_tx) {
        Ok(handle) => handle,
        Err(error) if active_demo => {
            return Err(error).context("starting autoplay watch demo");
        }
        Err(_) => {
            timed_loading(
                terminal,
                "Watch Demo",
                "File watcher unavailable \u{2014} skipping demo.",
                theme,
                std::time::Duration::from_secs(1),
            )?;
            return Ok(anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::Continue);
        }
    };

    let data = WatchData {
        status: WatchStatus::Idle,
        queue: std::collections::VecDeque::new(),
        history: Vec::new(),
        stats: WatchStats {
            total_runs: 0,
            pass_rate: 0.0,
            avg_duration_ms: 0,
            files_watched: 0,
        },
        warmup: None,
        last_action: None,
        update_hint: None,
        insights_hint: None,
        daemon_fallback_notice: None,
    };

    let state = anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState::new(data);
    let outcome = if active_demo {
        let sandbox = sandbox.context("active autoplay session has no sandbox")?;
        let mut edit = || sandbox.script_second_edit();
        crate::tui::run_watch_demo_autoplay_in(
            terminal,
            state,
            &event_rx,
            theme,
            tutorial_state,
            &mut edit,
        )
    } else {
        crate::tui::run_watch_demo_in(terminal, state, &event_rx, theme)
            .map(|()| anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::Continue)
    };

    let stop_result = handle.stop().context("stopping watch demo watcher");
    match outcome {
        Err(primary) => {
            if !active_demo && let Err(error) = stop_result {
                eprintln!("Failed to stop watch demo watcher: {error}");
            }
            Err(primary)
        }
        Ok(outcome) => {
            if active_demo {
                stop_result?;
            } else if let Err(error) = stop_result {
                eprintln!("Failed to stop watch demo watcher: {error}");
            }
            Ok(outcome)
        }
    }
}

/// Start watch mode from the welcome hub. Sets up the kernel watcher,
/// runs the watch TUI surface, then stops the watcher on exit.
fn start_watch_from_hub(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    _theme: &EddaCraftTheme,
) -> anyhow::Result<SurfaceExit> {
    use anyhow::Context;

    let workspace_root = crate::util::workspace_root()?;

    let watcher_config = anvil_kernel::watcher::WatcherConfig {
        root: workspace_root.clone(),
        debounce_window: std::time::Duration::from_millis(300),
        filter: Some(anvil_kernel::watcher::filter::FileFilter::default()),
        ..Default::default()
    };

    let watch_config = anvil_kernel::watch::WatchConfig {
        root: workspace_root,
        architecture_config: None,
        watcher: watcher_config,
        include_patterns: vec!["**/*".to_string()],
        exclude_patterns: Vec::new(),
        warmup_paths: Vec::new(),
    };

    let (event_tx, event_rx) = std::sync::mpsc::channel();

    let handle = anvil_kernel::watch::run_watch(&watch_config, event_tx)
        .context("starting kernel watcher")?;

    let mut state =
        anvil_tui::surfaces::watch::WatchState::new(anvil_tui::surfaces::watch::WatchData {
            status: anvil_tui::surfaces::watch::WatchStatus::Idle,
            queue: std::collections::VecDeque::new(),
            history: Vec::new(),
            stats: anvil_tui::surfaces::watch::WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
            daemon_fallback_notice: None,
        });

    let exit = crate::tui::run_watch_in(terminal, &mut state, &event_rx);
    let stop_result = handle.stop().context("stopping watcher");

    stop_result?;
    exit
}

#[allow(clippy::too_many_lines)]
fn run_welcome_hub(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
    verbose: bool,
) -> anyhow::Result<()> {
    let mut welcome = WelcomeState::new();

    loop {
        let exit = crate::tui::run_surface_in(terminal, &mut welcome, theme)?;

        if exit == SurfaceExit::Quit && welcome.chosen.is_none() {
            break;
        }

        match welcome.chosen.take() {
            Some(QuickStartOption::RunGate) => {
                crate::tui::draw_loading(terminal, "Gate", "Running quality checks...", theme)?;
                let data = crate::commands::gate::collect_gate_data();
                let mut gate_state = anvil_tui::surfaces::gate::GateState::new(data).embedded();
                let sub_exit = crate::tui::run_surface_in(terminal, &mut gate_state, theme)?;
                if sub_exit == SurfaceExit::Quit {
                    break;
                }
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::StartWatch) => {
                crate::tui::draw_loading(terminal, "Watch", "Starting file watcher...", theme)?;
                match start_watch_from_hub(terminal, theme) {
                    Ok(SurfaceExit::Quit) => break,
                    Ok(SurfaceExit::Back) => {}
                    Err(e) => {
                        welcome.status_message = Some(format!("Watch mode failed: {e}"));
                    }
                }
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::ViewDocs) => {
                welcome.status_message = Some(open_docs_message());
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::RunAudit) => {
                crate::tui::draw_loading(terminal, "Audit", "Scanning project...", theme)?;
                let data = crate::commands::audit::collect_audit_data();
                let mut audit_state = anvil_tui::surfaces::audit::AuditState::new(data).embedded();
                loop {
                    let sub_exit = crate::tui::run_surface_in(terminal, &mut audit_state, theme)?;
                    if let Some(request) = audit_state.pending_fix.take() {
                        let selected = audit_state.selected_item;
                        if matches!(
                            apply_fix_request(&request, None),
                            FixOutcome::Applied { .. }
                        ) {
                            audit_state.data = crate::commands::audit::collect_audit_data();
                            audit_state.selected_item =
                                selected.min(audit_state.data.issues.len().saturating_sub(1));
                            audit_state.expanded = false;
                        }
                        continue;
                    }
                    if sub_exit == SurfaceExit::Quit {
                        break;
                    }
                    break;
                }
                if audit_state.should_quit {
                    break;
                }
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::RunDoctor) => {
                crate::tui::draw_loading(terminal, "Doctor", "Running diagnostics...", theme)?;
                let checks = crate::commands::doctor::collect_checks();
                let mut doctor_state =
                    anvil_tui::surfaces::doctor::DoctorState::new(checks).embedded();
                loop {
                    let _sub_exit = crate::tui::run_surface_in(terminal, &mut doctor_state, theme)?;
                    if let Some(request) = doctor_state.pending_fix.take() {
                        let selected = doctor_state.selected;
                        if matches!(
                            apply_fix_request(&request, Some(&mut doctor_state.checks)),
                            FixOutcome::Applied { .. }
                        ) {
                            let fresh = crate::commands::doctor::collect_checks();
                            doctor_state.checks = fresh;
                            doctor_state.selected =
                                selected.min(doctor_state.checks.len().saturating_sub(1));
                        }
                        continue;
                    }
                    break;
                }
                if doctor_state.should_quit {
                    break;
                }
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::RunTutorial) => {
                welcome.status_message = None;
                if let Some(results) = run_discovery(terminal, theme)? {
                    let mut tutorial_state = tutorial_state_with_scan(results);
                    let sub_exit =
                        run_tutorial_with_fix(terminal, theme, &mut tutorial_state, verbose)?;
                    if sub_exit == SurfaceExit::Quit {
                        break;
                    }
                }
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::RestartOnboarding) => {
                let marker_path = first_run_marker_path()?;
                delete_first_run_marker(&marker_path)?;
                match crate::commands::tutorial::progress_file_path() {
                    Ok(progress_path) => match std::fs::remove_file(&progress_path) {
                        Ok(()) => {}
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                        Err(err) => {
                            eprintln!(
                                "[welcome] warning: failed to remove tutorial progress at {}: {err}",
                                progress_path.display(),
                            );
                        }
                    },
                    Err(err) => {
                        eprintln!(
                            "[welcome] warning: failed to resolve tutorial progress path: {err}",
                        );
                    }
                }

                let onboarding_ok = match run_onboarding(terminal, theme) {
                    Ok(OnboardingOutcome::Quit) => break,
                    Ok(OnboardingOutcome::Tutorial | OnboardingOutcome::Configured) => {
                        if let Some(mut results) = run_discovery(terminal, theme)? {
                            // WOW-005: restarted onboarding replays the
                            // first-run experience, including the first-win
                            // reroute.
                            if run_first_win_reroute(terminal, theme, &mut results)?
                                == FirstWinFlow::Quit
                            {
                                break;
                            }
                            let mut tutorial_state = tutorial_state_with_scan(results);
                            let sub_exit = run_tutorial_with_fix(
                                terminal,
                                theme,
                                &mut tutorial_state,
                                verbose,
                            )?;
                            if sub_exit == SurfaceExit::Quit {
                                break;
                            }
                        }
                        true
                    }
                    Ok(OnboardingOutcome::Skip) => true,
                    Err(e) => {
                        welcome.status_message = Some(format!("Onboarding failed: {e}"));
                        false
                    }
                };

                // Only re-create marker after successful onboarding so users
                // can retry on failure.
                if onboarding_ok {
                    let marker_path = first_run_marker_path()?;
                    if let Err(err) = create_first_run_marker(&marker_path) {
                        eprintln!("[welcome] warning: failed to create first-run marker: {err}");
                    }
                }

                welcome.should_quit = false;
                welcome.chosen = None;
            }
            None => {
                break;
            }
        }
    }

    Ok(())
}

fn open_docs_message() -> String {
    let url = "https://docs.eddacraft.ai";

    // Under test the shared helper is a no-op, so keep the plain visit line
    // rather than claiming a browser opened.
    if cfg!(test) {
        return format!("Visit: {url}");
    }

    match crate::util::open_in_browser(url) {
        Ok(()) => format!("Opened {url} in your browser"),
        Err(reason) => format!("Could not open browser: {reason}  |  Visit: {url}"),
    }
}

/// UJ-001: the next-step line a welcome exit prints when the reader can
/// already activate — a plain `anvil start` will run for them.
const WELCOME_NEXT_STEP_ACTIVATE: &str =
    "  Next: run `anvil start` for daily save-time protection.";

/// The next-step line for a signed-out reader. `anvil welcome` is the ungated
/// demo surface (ADR-080), but `anvil start` is licence-gated — so pointing an
/// unauthenticated reader straight at `anvil start` dead-ends at the auth wall.
/// Name the sign-in bridge, and offer the free, no-sign-in `anvil start
/// --verify` probe so the closing copy always leaves the reader something that
/// actually runs. The `anvil start` mention is preserved so the golden-path
/// handoff (UJ-001) still reads as a single continuous journey.
const WELCOME_NEXT_STEP_SIGN_IN: &str = "  Next: sign in with `anvil auth login` (early access: https://eddacraft.ai), then run `anvil start` for daily save-time protection.\n  No sign-in yet? `anvil start --verify` shows your current protection state for free.";

/// Pick the honest next-step copy. `prompts_sign_in` is `true` when a plain
/// `anvil start` would stop at the auth wall for this reader right now (see
/// [`start_prompts_sign_in`]).
fn welcome_next_step(prompts_sign_in: bool) -> &'static str {
    if prompts_sign_in {
        WELCOME_NEXT_STEP_SIGN_IN
    } else {
        WELCOME_NEXT_STEP_ACTIVATE
    }
}

/// Whether a plain `anvil start` would stop at the licence/auth wall for this
/// reader right now.
///
/// Local-only — never makes a network call, because `anvil welcome` is the
/// deliberately ungated demo surface (ADR-080): its whole value is instant
/// local findings with zero auth interaction, and a live auth exchange is the
/// exact friction (`Refresh token is invalid or revoked`) that ADR-080 pulled
/// out of the first interaction. This predicate only picks honest next-step
/// *copy*, so it must not itself perform the auth round-trip welcome defers.
///
/// It therefore *approximates* the pre-dispatch gate in `main::check_auth`
/// within that no-network constraint: the wall applies only when the licence
/// gate is *enforcing* (not dev-bypassed / disabled) AND no valid local
/// credential is present.
///
/// One deliberate divergence: `check_auth` can *silently refresh* an expired
/// credential that still carries a refresh token (a network exchange), so such
/// a reader is not actually prompted. Welcome cannot replicate that refresh
/// without the forbidden network call, and the refresh can still fail
/// permanently — so it conservatively treats *any* expired credential (and any
/// resolution/read error) as "would prompt". That errs toward the sign-in
/// bridge rather than over-promising activation it cannot verify it can
/// deliver; a signed-in-but-refreshable reader may see a bridge they did not
/// strictly need, which is the safe direction (the bridge still names
/// `anvil start` and the free `--verify` probe). Revisit this heuristic when
/// `cli.licence-gate` / `CLI_GATED_COMMANDS` are reworked for the free/paid
/// tier split — the gated set it reads is expected to change there.
fn start_prompts_sign_in() -> bool {
    let gate = crate::feature_flags::resolve_cli_licence_gate();
    if !matches!(
        crate::feature_flags::local_auth_precheck(&gate),
        crate::feature_flags::LocalAuthPrecheck::Enforce
    ) {
        // Dev bypass or a disabled gate → `anvil start` runs without a local
        // credential check, so no sign-in bridge is needed.
        return false;
    }
    match crate::auth::credentials::load() {
        // A present, unexpired credential clears the wall. An expired one is
        // conservatively treated as "would prompt" even with a refresh token:
        // `check_auth` would attempt a silent network refresh we cannot perform
        // here, and that refresh can fail permanently — so we err toward the
        // sign-in bridge rather than assuming a refresh that may not land.
        Ok(Some(creds)) => crate::auth::credentials::is_expired(&creds),
        // No credential on disk, or the store could not be read.
        _ => true,
    }
}

fn print_plain_welcome(prompts_sign_in: bool) {
    print!("{}", plain_welcome_message(prompts_sign_in));
}

fn plain_welcome_message(prompts_sign_in: bool) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  Welcome to anvil");
    let _ = writeln!(out, "  Structural governance for AI-assisted development");
    let _ = writeln!(out);
    let _ = writeln!(out, "  Available commands:");
    if prompts_sign_in {
        // Signed out: lead with what runs without sign-in, and mark the
        // licence-gated commands so the reader is never sent into the wall.
        let _ = writeln!(
            out,
            "    anvil tutorial        Interactive tutorial (no sign-in)"
        );
        let _ = writeln!(
            out,
            "    anvil doctor          Diagnose your environment (no sign-in)"
        );
        let _ = writeln!(
            out,
            "    anvil start --verify  Show your protection state (no sign-in)"
        );
        let _ = writeln!(
            out,
            "    anvil audit           Run project audit (needs sign-in)"
        );
        let _ = writeln!(
            out,
            "    anvil status          Show project status (needs sign-in)"
        );
    } else {
        let _ = writeln!(out, "    anvil tutorial   Interactive tutorial");
        let _ = writeln!(out, "    anvil audit      Run project audit");
        let _ = writeln!(out, "    anvil doctor     Diagnose your environment");
        let _ = writeln!(out, "    anvil status     Show project status");
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "  Visit: https://docs.eddacraft.ai");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", welcome_next_step(prompts_sign_in));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_step(state: &mut anvil_tui::surfaces::tutorial::TutorialState, target: &str) {
        state.load_steps(anvil_tui::surfaces::tutorial::TutorialPath::Policy);
        state.steps[0] = anvil_tui::surfaces::tutorial::TutorialStep {
            edit_target: Some(target.to_string()),
            seed_template: Some("rooted".to_string()),
            ..Default::default()
        };
        state.open_step_editor();
        state.save_step_editor().expect("save rooted editor");
    }

    #[test]
    fn welcome_tutorial_state_is_bound_to_resolved_workspace() {
        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace
            .path()
            .canonicalize()
            .expect("canonical workspace");
        let mut state = tutorial_state_with_scan(
            anvil_tui::surfaces::tutorial::discovery::ScanResults::default(),
        );

        bind_welcome_tutorial_workspace(&mut state, &root).expect("bind welcome tutorial");
        write_test_step(&mut state, "welcome-root-marker.txt");

        assert!(root.join("welcome-root-marker.txt").exists());
    }

    #[test]
    fn welcome_autoplay_teardown_rebinds_original_workspace() {
        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace
            .path()
            .canonicalize()
            .expect("canonical workspace");
        let sandbox = tempfile::tempdir().expect("sandbox");
        let mut state = tutorial_state_with_scan(
            anvil_tui::surfaces::tutorial::discovery::ScanResults::default(),
        );
        bind_welcome_tutorial_workspace(&mut state, &root).expect("initial bind");
        state
            .start_autoplay_in(sandbox.path())
            .expect("autoplay bind");
        state.abort_autoplay_session();

        bind_welcome_tutorial_workspace(&mut state, &root).expect("teardown rebind");
        write_test_step(&mut state, "welcome-rebound-marker.txt");

        assert!(root.join("welcome-rebound-marker.txt").exists());
        assert!(!sandbox.path().join("welcome-rebound-marker.txt").exists());
    }

    // CIB-171: Esc on the discovery results/continue screen backs out to the
    // hub caller rather than advancing into the tutorial.
    #[test]
    fn discovery_esc_backs_out_instead_of_continuing() {
        use anvil_tui::surfaces::tutorial::discovery::ScanResults;
        let results = || Some(ScanResults::default());

        // Esc (Back) must not carry results forward — it returns None so the
        // hub loop returns to the menu.
        assert!(discovery_outcome(SurfaceExit::Back, false, results()).is_none());

        // Quit without an explicit continue also backs out.
        assert!(discovery_outcome(SurfaceExit::Quit, false, results()).is_none());

        // Only an explicit Enter-to-continue advances into the tutorial.
        assert!(discovery_outcome(SurfaceExit::Quit, true, results()).is_some());
    }

    // ── WOW-005: first-win reroute routing ────────────────────────────────

    mod first_win_routing {
        use anvil_tui::surfaces::tutorial::discovery::{
            Finding, FindingSeverity, FindingSource, ScanResults,
        };
        use anvil_tui::surfaces::tutorial::first_win::{
            FIRST_WIN_CONSENT_ID, FirstWinPhase, FixPreview,
        };

        use super::super::build_first_win_state;

        fn actionable_finding() -> Finding {
            Finding {
                file: "src/app.ts".to_string(),
                line: Some(3),
                severity: FindingSeverity::Warning,
                source: FindingSource::AntiPattern,
                title: "Avoid `any`".to_string(),
                message: "message".to_string(),
                suggestion: "suggestion".to_string(),
                warning_id: Some("AP-003".to_string()),
            }
        }

        fn results(findings: Vec<Finding>, is_showcase: bool, files_scanned: usize) -> ScanResults {
            ScanResults {
                findings,
                files_scanned,
                duration_ms: 5,
                truncated: false,
                files_skipped_by_ignore: 0,
                is_showcase,
            }
        }

        // Matches `build_first_win_state`'s preview closure signature, hence
        // the `Option` return.
        #[allow(clippy::unnecessary_wraps)]
        fn some_preview(
            _request: &anvil_tui::surfaces::fix_request::FixRequest,
        ) -> Option<FixPreview> {
            Some(FixPreview {
                line: 3,
                before: "const value: any = source;".to_string(),
                after: "const value: unknown = source;".to_string(),
            })
        }

        #[test]
        fn actionable_finding_with_preview_offers_the_first_win() {
            let r = results(vec![actionable_finding()], false, 10);
            let state = build_first_win_state(&r, false, some_preview).expect("offer state");
            assert!(matches!(state.phase, FirstWinPhase::Offer));
            let offer = state.offer.as_ref().expect("offer");
            assert_eq!(offer.finding.file, "src/app.ts");
            // CIB-165: nothing pre-selected.
            assert!(!offer.consent.is_selected(FIRST_WIN_CONSENT_ID));
        }

        #[test]
        fn unavailable_preview_lands_on_the_picker_unchanged() {
            // If the diff cannot be computed, no offer is shown — the flow
            // must never promise a fix it cannot preview first.
            let r = results(vec![actionable_finding()], false, 10);
            assert!(build_first_win_state(&r, false, |_| None).is_none());
        }

        #[test]
        fn clean_scan_states_an_honest_clean_result() {
            // Clean repo: discovery substituted showcase examples for a real
            // scan of 42 files. The reroute states the clean result and never
            // offers example findings as a local win (CIB-170).
            let r = results(vec![actionable_finding()], true, 42);
            let state = build_first_win_state(&r, false, some_preview).expect("clean state");
            assert!(matches!(
                state.phase,
                FirstWinPhase::Clean { files_scanned: 42 }
            ));
            assert!(state.offer.is_none());
        }

        #[test]
        fn scan_failure_fallback_makes_no_clean_claim() {
            // Showcase substitution with zero files scanned means the scan
            // failed — claiming "clean" would be dishonest, so no reroute.
            let r = results(vec![actionable_finding()], true, 0);
            assert!(build_first_win_state(&r, false, some_preview).is_none());
        }

        #[test]
        fn nothing_actionable_lands_on_the_picker_unchanged() {
            let mut finding = actionable_finding();
            finding.warning_id = None; // no deterministic fix
            let r = results(vec![finding], false, 10);
            assert!(build_first_win_state(&r, false, some_preview).is_none());
        }

        #[test]
        fn skipped_scan_lands_on_the_picker_unchanged() {
            // 's' during scanning yields empty, non-showcase results with
            // zero files scanned: no findings, no clean claim.
            let r = results(vec![], false, 0);
            assert!(build_first_win_state(&r, false, some_preview).is_none());
        }

        #[test]
        fn unpreviewable_top_candidate_falls_back_to_the_next() {
            // A transiently unreadable top candidate must not drop the whole
            // first win — the next actionable row in discovery order is
            // offered instead.
            let mut second = actionable_finding();
            second.file = "src/second.ts".to_string();
            let r = results(vec![actionable_finding(), second], false, 10);
            let state = build_first_win_state(&r, false, |request| match request {
                anvil_tui::surfaces::fix_request::FixRequest::AntiPatternWarning {
                    file, ..
                } if file == "src/second.ts" => some_preview(request),
                _ => None,
            })
            .expect("fallback offer");
            assert_eq!(
                state.offer.as_ref().expect("offer").finding.file,
                "src/second.ts"
            );
        }

        // ── handle_apply_outcome bookkeeping ─────────────────────────────

        use super::super::handle_apply_outcome;
        use crate::services::interactive_fix::FixOutcome;
        use anvil_tui::surfaces::tutorial::first_win::FirstWinState;

        fn offer_state_for(finding: Finding) -> FirstWinState {
            FirstWinState::offer(
                finding,
                FixPreview {
                    line: 3,
                    before: "const value: any = source;".to_string(),
                    after: "const value: unknown = source;".to_string(),
                },
                false,
            )
        }

        #[test]
        fn applied_outcome_prunes_exactly_the_matching_finding() {
            let fixed = actionable_finding();
            let mut untouched = actionable_finding();
            untouched.line = Some(9); // same file, different line — must survive
            let mut r = results(vec![fixed.clone(), untouched], false, 10);
            let mut state = offer_state_for(fixed.clone());
            let request = fixed.fix_request().expect("request");

            handle_apply_outcome(
                FixOutcome::Applied {
                    summary: "Applied fix in src/app.ts:3".to_string(),
                },
                &request,
                &mut r,
                &mut state,
            );

            // WOW-003/WOW-004 count integrity: only the fixed finding is gone.
            assert_eq!(r.findings.len(), 1);
            assert_eq!(r.findings[0].line, Some(9));
            assert!(matches!(
                state.phase,
                FirstWinPhase::Done { applied: true, .. }
            ));
        }

        #[test]
        fn refused_and_failed_outcomes_leave_results_untouched() {
            let finding = actionable_finding();
            let request = finding.fix_request().expect("request");
            for outcome in [
                FixOutcome::Refused {
                    reason: "changed since the preview".to_string(),
                },
                FixOutcome::Failed {
                    reason: "failed to write".to_string(),
                },
            ] {
                let mut r = results(vec![finding.clone()], false, 10);
                let mut state = offer_state_for(finding.clone());
                handle_apply_outcome(outcome, &request, &mut r, &mut state);
                assert_eq!(r.findings.len(), 1, "results must be untouched");
                match &state.phase {
                    FirstWinPhase::Done { applied, message } => {
                        assert!(!applied);
                        assert!(!message.is_empty());
                    }
                    other => panic!("expected Done, got {other:?}"),
                }
            }
        }

        // ── Real registry guidance renders in full ────────────────────────

        #[test]
        fn offer_renders_full_real_ap003_guidance() {
            // Council repro: the shipped AP-003 suggestion is ~1.5k chars
            // across ~30 authored lines; the offer must show it in full with
            // the authored line structure intact (no fixed-height clipping,
            // no newline-collapsed run-on words).
            use anvil_tui::surface::Surface as _;
            use ratatui::Terminal;
            use ratatui::backend::TestBackend;

            let pattern = anvil_checks::antipattern::patterns::get_pattern("AP-003")
                .expect("AP-003 pattern in the compiled registry");
            let mut finding = actionable_finding();
            finding.message = pattern.explanation.clone();
            finding.suggestion = pattern.suggestion.clone();
            let state = offer_state_for(finding);

            let backend = TestBackend::new(110, 72);
            let mut terminal = Terminal::new(backend).unwrap();
            let theme = eddacraft_tui::theme::EddaCraftTheme;
            terminal
                .draw(|frame| state.render(frame, frame.area(), &theme))
                .unwrap();
            let buf = terminal.backend().buffer();
            let area = buf.area;
            let mut out = String::new();
            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    out.push_str(buf[(x, y)].symbol());
                }
                out.push('\n');
            }

            // Every authored guidance line is visible verbatim on its own
            // row (registry lines are pre-wrapped well under the test width).
            for line in pattern.suggestion.lines() {
                let line = line.trim_end();
                if line.is_empty() {
                    continue;
                }
                assert!(
                    out.contains(line),
                    "authored guidance line must be visible: {line:?}\nrendered:\n{out}"
                );
            }
            // The reproduced newline-collapse mangle must not occur.
            assert!(!out.contains("shortcutfor"), "rendered:\n{out}");
            // The consent chrome is still on screen below the guidance.
            assert!(out.contains("[ ] Apply this fix to"), "rendered:\n{out}");
        }
    }

    // UJ-001: every welcome exit — in either variant — carries the user
    // toward the daily-value `anvil start` path.
    #[test]
    fn welcome_next_step_always_names_anvil_start() {
        for prompts_sign_in in [false, true] {
            let line = welcome_next_step(prompts_sign_in);
            assert!(
                line.contains("anvil start"),
                "the welcome next step must name `anvil start` (prompts_sign_in={prompts_sign_in}), got: {line}",
            );
        }
    }

    // Signed-in reader: the copy points straight at activation and does not
    // nag about sign-in.
    #[test]
    fn welcome_next_step_activate_variant_is_direct() {
        let line = welcome_next_step(false);
        assert!(line.contains("anvil start"), "got: {line}");
        assert!(
            !line.contains("auth login"),
            "an activatable reader must not be told to sign in: {line}",
        );
        assert!(
            !line.contains("--verify"),
            "the direct variant does not need the free-probe fallback: {line}",
        );
    }

    // Signed-out reader: the copy bridges to sign-in AND offers the free,
    // no-sign-in `anvil start --verify` probe so the closing line always
    // leaves the reader a command that actually runs.
    #[test]
    fn welcome_next_step_sign_in_variant_bridges_auth_and_offers_verify() {
        let line = welcome_next_step(true);
        assert!(
            line.contains("anvil auth login"),
            "the sign-in variant must name the login bridge: {line}",
        );
        assert!(
            line.contains("early access"),
            "the sign-in variant must name early access: {line}",
        );
        assert!(
            line.contains("anvil start --verify"),
            "the sign-in variant must offer the free read-only probe: {line}",
        );
    }

    #[test]
    fn plain_welcome_carries_the_matching_next_step_line() {
        for prompts_sign_in in [false, true] {
            let msg = plain_welcome_message(prompts_sign_in);
            assert!(
                msg.contains(welcome_next_step(prompts_sign_in)),
                "the plain welcome surface must end with its next-step line (prompts_sign_in={prompts_sign_in}):\n{msg}",
            );
        }
    }

    // Signed-out plain welcome must not advertise a licence-gated command
    // without flagging it, and must surface the free `--verify` probe.
    #[test]
    fn plain_welcome_sign_in_variant_flags_gated_commands_and_offers_verify() {
        let msg = plain_welcome_message(true);
        assert!(
            msg.contains("anvil start --verify"),
            "signed-out plain welcome must list the free probe:\n{msg}",
        );
        assert!(
            msg.contains("needs sign-in"),
            "signed-out plain welcome must flag the licence-gated commands:\n{msg}",
        );
        // The gated commands (`audit`, `status`) must be flagged, not offered
        // bare as if they were free.
        for gated in ["anvil audit", "anvil status"] {
            let line = msg
                .lines()
                .find(|l| l.contains(gated))
                .unwrap_or_else(|| panic!("{gated} must appear in the list:\n{msg}"));
            assert!(
                line.contains("needs sign-in"),
                "`{gated}` must be flagged as gated in the signed-out list: {line}",
            );
        }
    }

    // Signed-in plain welcome keeps the compact, unannotated list.
    #[test]
    fn plain_welcome_activate_variant_lists_commands_unannotated() {
        let msg = plain_welcome_message(false);
        assert!(msg.contains("anvil audit"), "got:\n{msg}");
        assert!(
            !msg.contains("needs sign-in"),
            "the activatable list must not carry sign-in annotations:\n{msg}",
        );
        assert!(
            !msg.contains("--verify"),
            "the activatable list does not need the free-probe fallback:\n{msg}",
        );
    }

    // The gate mirror: a dev-bypassed session (`ANVIL_DEV=1`) resolves the
    // licence gate to a Skip, so a plain `anvil start` would NOT wall — the
    // predicate must report no sign-in prompt regardless of credential state.
    #[test]
    fn start_prompts_sign_in_is_false_under_dev_bypass() {
        temp_env::with_var("ANVIL_DEV", Some("1"), || {
            assert!(
                !start_prompts_sign_in(),
                "a dev-bypassed session clears the wall, so no sign-in bridge is shown",
            );
        });
    }

    #[test]
    fn open_docs_message_does_not_panic() {
        let msg = open_docs_message();
        assert!(!msg.is_empty());
    }

    // ── INSIGHTS-005: first-week nudge on the welcome surface ───────
    //
    // Grouped under `welcome::insights_hint::*` so the surface tests are
    // greppable (`cargo test -p eddacraft-anvil welcome::insights_hint`).
    mod insights_hint {
        use crate::insights::first_week_hint::first_week_insights_hint;
        use chrono::{Duration, Utc};
        use tempfile::TempDir;

        /// A throwaway repo whose `anvil/project-id` puts the user inside the
        /// 14-day first-week cohort window.
        fn repo_in_first_week() -> (TempDir, std::path::PathBuf) {
            let dir = TempDir::with_prefix("anvil-welcome-insights-").unwrap();
            let root = dir.path().to_path_buf();
            let anvil = root.join("anvil");
            std::fs::create_dir_all(&anvil).unwrap();
            let created =
                (Utc::now() - Duration::days(3)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            std::fs::write(
                anvil.join("project-id"),
                format!(
                    "project_uuid: 01999999-0000-0000-0000-000000000005\ncreated_at: {created}\n"
                ),
            )
            .unwrap();
            (dir, root)
        }

        #[test]
        fn shows_within_window_when_not_gated() {
            let (_tmp, root) = repo_in_first_week();
            let hint = first_week_insights_hint(&root, Utc::now(), false);
            assert!(
                hint.as_deref()
                    .is_some_and(|h| h.contains("run `anvil insights`")),
                "the welcome surface must carry the first-week nudge in window, got {hint:?}"
            );
        }

        #[test]
        fn suppressed_when_project_writes_gated() {
            let (_tmp, root) = repo_in_first_week();
            // Under a gated project root the candidate must neither emit the
            // nudge nor write the project's hint state (DISTRIB-006 / ADR-060).
            assert!(first_week_insights_hint(&root, Utc::now(), true).is_none());
            assert!(
                !root.join(".anvil/insights-hint.json").exists(),
                "gated welcome must not write the project hint state"
            );
        }

        #[test]
        fn shares_rate_limit_with_status_and_watch() {
            let (_tmp, root) = repo_in_first_week();
            // The welcome surface emits once...
            assert!(first_week_insights_hint(&root, Utc::now(), false).is_some());
            // ...and that emission consumes the shared once-per-week marker, so
            // the same underlying hint behind `status`/`watch` is now suppressed.
            let via_status = first_week_insights_hint(&root, Utc::now(), false);
            assert!(
                via_status.is_none(),
                "the welcome emission must share the per-week rate limit with status/watch"
            );
        }

        #[test]
        fn welcome_is_suppressed_after_status_or_watch_emits() {
            let (_tmp, root) = repo_in_first_week();
            // The reverse direction: status/watch emits first via the shared
            // mechanism, so the welcome surface is then suppressed for the week.
            assert!(first_week_insights_hint(&root, Utc::now(), false).is_some());
            assert!(first_week_insights_hint(&root, Utc::now(), false).is_none());
        }
    }

    // ── SCAN-003: first-run rayon pool tests ────────────────────────
    //
    // Grouped under `welcome::pool::*` so the steps-file validate
    // command (`cargo test -p eddacraft-anvil welcome::pool`) hits
    // them precisely.

    mod pool {
        use super::super::{
            DEFAULT_FIRST_RUN_THREAD_CAP, build_first_run_pool, resolve_first_run_thread_count,
        };

        #[test]
        fn defaults_to_min_of_cpus_and_cap_when_env_unset() {
            let resolved = resolve_first_run_thread_count(None, 16).unwrap();
            assert_eq!(resolved, DEFAULT_FIRST_RUN_THREAD_CAP);
        }

        #[test]
        fn caps_to_cpu_count_when_cpus_below_default() {
            let resolved = resolve_first_run_thread_count(None, 2).unwrap();
            assert_eq!(resolved, 2);
        }

        #[test]
        fn env_override_is_honoured_within_cpu_bound() {
            // 6 < 16 cores → return 6 verbatim.
            let resolved = resolve_first_run_thread_count(Some("6"), 16).unwrap();
            assert_eq!(resolved, 6);
        }

        #[test]
        fn env_override_clamps_to_available_cpus() {
            // Asking for 32 on a 4-core box must not over-schedule.
            let resolved = resolve_first_run_thread_count(Some("32"), 4).unwrap();
            assert_eq!(resolved, 4);
        }

        #[test]
        fn invalid_env_value_falls_back_to_default() {
            let resolved = resolve_first_run_thread_count(Some("not-a-number"), 8).unwrap();
            assert_eq!(resolved, DEFAULT_FIRST_RUN_THREAD_CAP);
        }

        #[test]
        fn zero_env_value_falls_back_to_default() {
            // `0` is a malformed override (you can't run 0 threads); fall
            // back to the default cap rather than refusing to scan.
            let resolved = resolve_first_run_thread_count(Some("0"), 8).unwrap();
            assert_eq!(resolved, DEFAULT_FIRST_RUN_THREAD_CAP);
        }

        #[test]
        fn whitespace_env_value_is_trimmed() {
            let resolved = resolve_first_run_thread_count(Some("  3  "), 8).unwrap();
            assert_eq!(resolved, 3);
        }

        #[test]
        fn pool_builder_honours_env_override() {
            // Drive `build_first_run_pool` end-to-end with a fixed env
            // override so we exercise the same code path scan_project
            // calls. `temp_env::with_var` keeps the env mutation scoped
            // to this test.
            temp_env::with_var("ANVIL_SCAN_THREADS", Some("2"), || {
                let pool = build_first_run_pool().expect("scoped pool builds");
                assert_eq!(pool.current_num_threads(), 2);
            });
        }
    }

    // ── SCAN-004: gitignore-skip provenance ─────────────────────────
    //
    // Exercises the real two-phase walk in `scan_project_at` against a temp
    // git repo so the gitignore-skip count reflects production behaviour, not
    // a stubbed set difference. Grouped so
    // `cargo test -p eddacraft-anvil welcome::tests::gitignore_skip` hits them.
    mod gitignore_skip {
        use super::super::scan_project_at;
        use std::fs;
        use std::process::Command;

        /// `git init` so the `ignore` crate honours `.gitignore` (it only
        /// applies git ignore rules inside a git repo). No commit is made —
        /// presence of `.git` plus the `.gitignore` file is enough.
        fn git_init(dir: &std::path::Path) {
            let ok = Command::new("git")
                .args(["init", "-q"])
                .current_dir(dir)
                .status()
                .is_ok_and(|s| s.success());
            assert!(ok, "git init failed in test fixture");
        }

        #[test]
        fn counts_candidate_files_dropped_by_gitignore() {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            git_init(root);

            // A tracked source file (scanned) and a gitignored one (dropped).
            // Both are scan candidates by extension; only gitignore differs,
            // so exactly one file is "skipped by ignore".
            fs::create_dir(root.join("src")).unwrap();
            fs::write(root.join("src/app.rs"), "fn main() {}\n").unwrap();
            fs::create_dir(root.join("ignored")).unwrap();
            fs::write(root.join("ignored/leaked.rs"), "let x = 1;\n").unwrap();
            fs::write(root.join(".gitignore"), "ignored/\n").unwrap();

            let results = scan_project_at(root, false);
            // Guards the fixture's assumption that `.rs` is a scan candidate:
            // if the filter ever stops treating `.rs` as eligible, the tracked
            // file is not scanned and the skip count drops to 0 — this assert
            // then fails loudly rather than the test silently passing wrong.
            assert!(
                results.files_scanned >= 1,
                "tracked src/app.rs must be a scan candidate for this test to be meaningful"
            );
            assert_eq!(
                results.files_skipped_by_ignore, 1,
                "the one gitignored .rs file should be counted as skipped"
            );
        }

        #[test]
        fn scan_all_disables_skip_count() {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            git_init(root);
            fs::create_dir(root.join("ignored")).unwrap();
            fs::write(root.join("ignored/leaked.rs"), "let x = 1;\n").unwrap();
            fs::write(root.join(".gitignore"), "ignored/\n").unwrap();

            // scan_all bypasses gitignore, so nothing is attributable to it.
            let results = scan_project_at(root, true);
            assert_eq!(results.files_skipped_by_ignore, 0);
        }

        #[test]
        fn clean_tree_yields_zero() {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            git_init(root);
            fs::write(root.join("app.rs"), "fn main() {}\n").unwrap();

            let results = scan_project_at(root, false);
            assert_eq!(results.files_skipped_by_ignore, 0);
        }
    }

    // ── SCAN-006: parallel Phase 1a walk invariants ─────────────────
    mod discovery_parallel {
        use super::super::{SCAN_MAX_FILES, scan_project_at};
        use anvil_tui::surfaces::tutorial::discovery::FindingSeverity;
        use std::fs;

        // The parallel Phase 1a walk must not leak thread-scheduling order into
        // user-visible output. `scan_project_at` imposes a final deterministic
        // sort, so repeated runs over the same tree must yield identical
        // findings in identical order.
        #[test]
        fn findings_order_is_deterministic_across_runs() {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            let token = format!("ghp_{}", "a".repeat(36));
            for i in 0..12 {
                let dir = root.join(format!("pkg{i:02}"));
                fs::create_dir_all(&dir).unwrap();
                fs::write(dir.join("conf.rs"), format!("let k = \"{token}\";\n")).unwrap();
            }
            // Project to (severity-rank, file, line, title). The rank encodes
            // the real sort's primary key (severity *descending* — Error first),
            // so a plain ascending sort of this tuple equals the documented
            // canonical order regardless of any severity mix in the fixture.
            let run = |r: &std::path::Path| {
                scan_project_at(r, false)
                    .findings
                    .iter()
                    .map(|f| {
                        let rank = match f.severity {
                            FindingSeverity::Error => 0u8,
                            FindingSeverity::Warning => 1,
                            FindingSeverity::Info => 2,
                        };
                        (rank, f.file.clone(), f.line, f.title.clone())
                    })
                    .collect::<Vec<_>>()
            };
            let first = run(root);
            assert!(!first.is_empty(), "fixture should produce secret findings");
            // Canonical-order invariant: the output is already in its own sorted
            // order, so determinism does not hinge on walk scheduling luck.
            let mut sorted = first.clone();
            sorted.sort();
            assert_eq!(
                first, sorted,
                "findings must be emitted in canonical sorted order"
            );
            // And repeated runs must agree (several runs to give the parallel
            // walk room to schedule differently).
            for _ in 0..5 {
                assert_eq!(
                    run(root),
                    first,
                    "parallel walk must produce a deterministic finding order across runs"
                );
            }
        }

        // Phase 1b stays sequential precisely to preserve the SCAN_MAX_FILES
        // early-break + honest truncation flag. More than the cap of candidate
        // files must set `truncated` and bound the scanned set.
        #[test]
        fn over_cap_candidate_set_is_truncated() {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            let n = SCAN_MAX_FILES + 25;
            for i in 0..n {
                fs::write(root.join(format!("f{i:04}.rs")), "fn x() {}\n").unwrap();
            }
            let results = scan_project_at(root, false);
            assert!(
                results.truncated,
                "more than SCAN_MAX_FILES candidates must truncate"
            );
            assert!(
                results.files_scanned <= SCAN_MAX_FILES,
                "scanned set must not exceed the cap: {}",
                results.files_scanned
            );
            // Truncation suppresses the SCAN-004 skip count (a capped walk can't
            // honestly attribute unscanned candidates to gitignore).
            assert_eq!(
                results.files_skipped_by_ignore, 0,
                "truncated scan must suppress the gitignore-skip count"
            );
        }
    }

    // ── GH #2584: dotenv + lockfile noise reduction ─────────────────
    //
    // Drives the real two-phase walk in `scan_project_at` against a temp git
    // repo so the noise-reduction behaviour reflects production, not a stub.
    mod noise_reduction {
        use super::super::scan_project_at;
        use std::fs;
        use std::process::Command;

        fn git_init(dir: &std::path::Path) {
            let ok = Command::new("git")
                .args(["init", "-q"])
                .current_dir(dir)
                .status()
                .is_ok_and(|s| s.success());
            assert!(ok, "git init failed in test fixture");
            // Neutralise any developer/CI global gitignore (which commonly
            // lists `.env`) so this fixture's gitignore state is exactly what
            // the test writes — not the ambient environment's. Point
            // `core.excludesfile` at an empty file inside the repo rather than
            // `/dev/null`, which does not exist on Windows.
            let empty = dir.join(".empty-global-excludes");
            fs::write(&empty, "").expect("write empty excludes file");
            let configured = Command::new("git")
                .args(["config", "core.excludesfile"])
                .arg(&empty)
                .current_dir(dir)
                .status()
                .is_ok_and(|s| s.success());
            assert!(configured, "git config core.excludesfile failed");
        }

        fn github_token() -> String {
            // A high-confidence GitHub token pattern — flagged regardless of the
            // keyword allowlist (issue #1800).
            format!("ghp_{}", "a".repeat(36))
        }

        fn has_secret_finding_for(
            results: &anvil_tui::surfaces::tutorial::discovery::ScanResults,
            file_substr: &str,
        ) -> bool {
            use anvil_tui::surfaces::tutorial::discovery::FindingSource;
            results
                .findings
                .iter()
                .any(|f| f.source == FindingSource::Secret && f.file.contains(file_substr))
        }

        #[test]
        fn gitignored_dotenv_local_is_not_scanned() {
            // A gitignored `.env.local` is the user's local secret store; its
            // contents must not be reported back as high-severity findings.
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            git_init(root);
            fs::write(
                root.join(".env.local"),
                format!("GITHUB_TOKEN={}\n", github_token()),
            )
            .unwrap();
            fs::write(root.join(".gitignore"), ".env.local\n").unwrap();

            let results = scan_project_at(root, false);
            assert!(
                !has_secret_finding_for(&results, ".env.local"),
                "a gitignored .env.local must not be force-scanned (GH #2584):\n{:#?}",
                results.findings,
            );
        }

        #[test]
        fn dotenv_is_exempt_even_when_not_gitignored() {
            // `.env` files are the designated local secret store, so they are
            // exempt from secret scanning regardless of gitignore state — a
            // gitignore-respecting walk can't reliably tell a committed `.env`
            // from a local one (global gitignores commonly hide `.env`), so the
            // exemption is unconditional rather than gitignore-dependent.
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            git_init(root); // also points core.excludesfile at /dev/null
            fs::write(
                root.join(".env"),
                format!("GITHUB_TOKEN={}\n", github_token()),
            )
            .unwrap();
            // No .gitignore — the .env is a tracked, non-ignored file here.

            let results = scan_project_at(root, false);
            assert!(
                !has_secret_finding_for(&results, ".env"),
                "a .env file must not be secret-scanned (GH #2584):\n{:#?}",
                results.findings,
            );
        }

        #[test]
        fn package_lock_integrity_hashes_are_not_flagged() {
            // npm lockfile integrity hashes are high-entropy by construction;
            // they must not surface as entropy/secret findings.
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            let lock = r#"{
  "name": "demo",
  "lockfileVersion": 3,
  "packages": {
    "node_modules/left-pad": {
      "version": "1.3.0",
      "resolved": "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz",
      "integrity": "sha512-XI5MPzVNApjAyhQzphX8BkmKsKUxD4LdyK24iZeQGinBN9yTQT3bFlCBy/aVx2HrNcqQGsdot8ghrjyrvMCoEA=="
    }
  }
}
"#;
            fs::write(root.join("package-lock.json"), lock).unwrap();

            let results = scan_project_at(root, false);
            assert!(
                !has_secret_finding_for(&results, "package-lock.json"),
                "lockfile integrity hashes must not be flagged (GH #2584):\n{:#?}",
                results.findings,
            );
        }

        #[test]
        fn package_lock_url_credential_is_flagged() {
            // A credential embedded in a lockfile `resolved` URL is a real
            // secret and must still surface, even though integrity hashes are
            // ignored.
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            let lock = "{\n  \"integrity\": \"sha512-XI5MPzVNApjAyhQzphX8BkmKsKUxD4LdyK24iZeQGinB\",\n  \"resolved\": \"https://deployer:s3cr3tT0ken@npm.private.example/x/-/x-1.0.0.tgz\"\n}\n";
            fs::write(root.join("package-lock.json"), lock).unwrap();

            let results = scan_project_at(root, false);
            assert!(
                has_secret_finding_for(&results, "package-lock.json"),
                "a credential in a lockfile resolved URL must be flagged:\n{:#?}",
                results.findings,
            );
            assert!(
                !results
                    .findings
                    .iter()
                    .any(|f| f.message.contains("s3cr3tT0ken")),
                "the credential must be redacted in the finding",
            );
        }
    }

    #[test]
    fn welcome_state_quits_on_chosen() {
        use anvil_tui::surface::Surface;

        let mut state = WelcomeState::new();
        assert!(!Surface::should_quit(&state));
        state.chosen = Some(QuickStartOption::RunAudit);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn onboarding_outcome_variants_exist() {
        // Verify all four variants are distinct.
        let outcomes = [
            OnboardingOutcome::Configured,
            OnboardingOutcome::Tutorial,
            OnboardingOutcome::Skip,
            OnboardingOutcome::Quit,
        ];
        for (i, a) in outcomes.iter().enumerate() {
            for (j, b) in outcomes.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
