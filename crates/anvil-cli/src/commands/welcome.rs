use std::io::IsTerminal;

use anvil_tui::surfaces::fix_request::FixRequest;
use anvil_tui::surfaces::welcome::{QuickStartOption, WelcomeState};
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
        print_plain_welcome();
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
                    Some(results) => {
                        let mut tutorial_state =
                            anvil_tui::surfaces::tutorial::TutorialState::new();
                        tutorial_state.set_scan_results(results);
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
    println!("{WELCOME_NEXT_STEP}");

    Ok(())
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
            "Anvil configuration detected \u{2014} skipping setup.",
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
            Ok(gitignore_updated) => {
                let summary = onboarding::InitCompleteSummary {
                    config_path: ".anvilrc".to_string(),
                    plans_dir: format!("{}/", config.planning_dir),
                    cache_dir: ".anvil/cache/".to_string(),
                    gitignore_updated,
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

    crate::tui::draw_loading(
        terminal,
        "Discovery",
        "Scanning project for findings\u{2026}",
        theme,
    )?;

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
            }
        }
    };
    discovery.set_results(results);

    let exit = crate::tui::run_surface_in(terminal, &mut discovery, theme)?;

    if exit == SurfaceExit::Quit && !discovery.wants_continue {
        return Ok(None);
    }

    Ok(discovery.results)
}

/// Scan the current project for real secret and antipattern findings.
const SCAN_MAX_FILES: usize = 500;
const SCAN_MAX_FILE_SIZE: u64 = 512 * 1024; // 512 KB

// SCAN-003 — first-run rayon pool cap.
//
// The default rayon global pool is sized to `num_cpus::get()`. On 16-core
// dev boxes the welcome-screen scan can pin every core, fighting the TUI
// render thread and any background LSP / indexer for CPU. Cap the
// pool at `min(num_cpus::get(), DEFAULT_FIRST_RUN_THREAD_CAP)` so the
// terminal stays responsive while the scan completes.
//
// Env var contract — `ANVIL_SCAN_THREADS`
// ---------------------------------------
// Operators (and the future RTAI debounced-scan surface) may override the
// cap via `ANVIL_SCAN_THREADS=<positive integer>`. This is the canonical
// env-var name for the scan-pool cap and is shared with the upcoming
// real-time AI validation (RTAI) first-run UX work — see
// `plans/modules/realtime-ai-validation.aps.md`. Locking in the name now,
// before RTAI ships, prevents the user-visible split-knob problem the
// spec calls out.
//
// `ANVIL_RAYON_THREADS` was the alternative considered. We picked
// `ANVIL_SCAN_THREADS` because:
//   - It scopes to scanning (the actual concern) rather than naming an
//     internal dependency (rayon) that may get swapped out.
//   - It composes cleanly with the existing `ANVIL_SCAN_ALL` toggle the
//     welcome screen already honours.
//   - "rayon threads" leaks an implementation detail that operators
//     should not need to know.
//
// Invalid / non-positive values fall back to the cap; this is a hint, not
// a hard contract — we never want a malformed env var to abort the scan.
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
    // Dependency lockfiles carry high-entropy integrity hashes that are secret-
    // scan false positives, and hold nothing else worth flagging (GH #2584).
    if anvil_checks::filter::is_lockfile(path) {
        return None;
    }
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
    // SCAN-004 skip count. `.env*` files are intentionally excluded from this
    // allowlist (GH #2584): a gitignored `.env` is the user's local secret
    // store, not a leak. A committed `.env` is still scanned by Phase 1b.
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

    // SCAN-004: candidates the gitignore-blind walk saw but the scanned set
    // (`seen` = Phase 1b's gitignore-respecting walk + the allowlist) never
    // included were dropped by `.gitignore`.
    //
    // This is a filtered membership count over `all_candidates`, NOT a
    // cardinality difference, which is what makes it robust to the two walks
    // having different shapes (Phase 1a prunes dirs via `filter_entry`; Phase
    // 1b prunes per-file in `candidate_path`). A file in `seen` but not in
    // `all_candidates` is simply not iterated — it was scanned, so it is
    // correctly not a "skip". A file in `all_candidates` but not in `seen`
    // made it past Phase 1a's prune, so its directory was not pruned; Phase 1b
    // (no dir prune) therefore reaches it too, meaning the only reason it is
    // absent from `seen` is gitignore. Hence every counted file is genuinely
    // gitignore-excluded.
    //
    // Suppressed to 0 when the scan was truncated by the file cap (Phase 1b
    // stopped early for an unrelated reason, so unscanned candidates beyond the
    // cap would wrongly look gitignore-dropped) or when `scan_all` disabled
    // gitignore entirely (`all_candidates` is empty then). `truncated` is set
    // only by Phase 1b hitting `SCAN_MAX_FILES`; Phase 1a is uncapped by
    // design, so there is no Phase 1a truncation to account for.
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
    }
}

/// Try to start a file watcher for the tutorial. Returns the receiver and
/// handle on success, or `None` if the watcher cannot be started (e.g.
/// inotify limit reached, no project directory).
fn try_start_tutorial_watcher() -> Option<(
    std::sync::mpsc::Receiver<anvil_kernel::watcher::events::ChangeBatch>,
    anvil_kernel::watcher::WatcherHandle,
)> {
    let root = crate::util::workspace_root().ok()?;
    let config = anvil_kernel::watcher::WatcherConfig {
        root,
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
fn run_tutorial_with_fix(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
    tutorial_state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    verbose: bool,
) -> anyhow::Result<SurfaceExit> {
    // Try to start a file watcher for live verification (WELCOME-013).
    // If unavailable, the tutorial enters static mode (all steps become
    // informational press-enter-to-continue, commands are not executed).
    let mut watcher = try_start_tutorial_watcher();

    if watcher.is_none() {
        tutorial_state.enable_static_mode_with_reason(
            anvil_tui::surfaces::tutorial::STATIC_MODE_WATCHER_UNAVAILABLE,
        );
    }

    loop {
        let file_rx = watcher.as_ref().map(|(rx, _)| rx);

        // Use the tutorial-specific loop that drains file-change events
        // and checks for wants_watch_demo exit.
        crate::tui::run_tutorial_in(terminal, tutorial_state, file_rx, theme)?;

        // Reset transient exit flags to prevent stale state from causing
        // immediate re-exit on the next loop iteration.
        tutorial_state.wants_back = false;

        if tutorial_state.wants_watch_demo {
            tutorial_state.wants_watch_demo = false;

            // Drop the tutorial watcher before launching the watch demo
            // to avoid two concurrent watchers over the same root, which
            // would double inotify descriptor usage (C-001).
            drop(watcher.take());

            // Treat failures as best-effort so the welcome/tutorial flow
            // resumes instead of aborting. Routes through format_user_error
            // so wrapped notify::Error paths only leak when --verbose is
            // explicit (see #1017).
            if let Err(err) = run_watch_demo_from_tutorial(terminal, theme) {
                eprintln!(
                    "Watch demo unavailable: {}",
                    crate::util::format_user_error(&err, verbose)
                );
            }

            // Advance past the watch demo step and resume.
            tutorial_state.advance_step();

            // Restart the tutorial watcher for remaining steps.
            watcher = try_start_tutorial_watcher();
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
) -> anyhow::Result<()> {
    use anvil_tui::surfaces::watch::{WatchData, WatchStats, WatchStatus};

    crate::tui::draw_loading(terminal, "Watch Demo", "Starting watch mode\u{2026}", theme)?;

    let Ok(workspace_root) = crate::util::workspace_root() else {
        timed_loading(
            terminal,
            "Watch Demo",
            "Could not determine project root \u{2014} skipping demo.",
            theme,
            std::time::Duration::from_secs(1),
        )?;
        return Ok(());
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

    let Ok(handle) = anvil_kernel::watch::run_watch(&watch_config, event_tx) else {
        timed_loading(
            terminal,
            "Watch Demo",
            "File watcher unavailable \u{2014} skipping demo.",
            theme,
            std::time::Duration::from_secs(1),
        )?;
        return Ok(());
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
    };

    let state = anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState::new(data);
    crate::tui::run_watch_demo_in(terminal, state, &event_rx, theme)?;

    if let Err(error) = handle.stop() {
        eprintln!("Failed to stop watch demo watcher: {error}");
    }
    Ok(())
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
                let mut gate_state = anvil_tui::surfaces::gate::GateState::new(data);
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
                let mut audit_state = anvil_tui::surfaces::audit::AuditState::new(data);
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
                let mut doctor_state = anvil_tui::surfaces::doctor::DoctorState::new(checks);
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
                    let mut tutorial_state = anvil_tui::surfaces::tutorial::TutorialState::new();
                    tutorial_state.set_scan_results(results);
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
                        if let Some(results) = run_discovery(terminal, theme)? {
                            let mut tutorial_state =
                                anvil_tui::surfaces::tutorial::TutorialState::new();
                            tutorial_state.set_scan_results(results);
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

    // Skip spawning external processes during tests
    if cfg!(test) {
        return format!("Visit: {url}");
    }

    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
    } else {
        std::process::Command::new("xdg-open")
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
    };
    match result {
        Ok(output) if output.status.success() => format!("Opened {url} in your browser"),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = stderr.lines().next().unwrap_or("unknown error");
            format!("Could not open browser: {reason}  |  Visit: {url}")
        }
        Err(e) => format!("Could not open browser: {e}  |  Visit: {url}"),
    }
}

/// UJ-001: the single next-step line every welcome exit prints — the
/// discovery path hands the user to the daily-value path.
const WELCOME_NEXT_STEP: &str = "  Next: run `anvil start` for daily save-time protection.";

fn print_plain_welcome() {
    print!("{}", plain_welcome_message());
}

fn plain_welcome_message() -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "  Welcome to Anvil");
    let _ = writeln!(out, "  Structural governance for AI-assisted development");
    let _ = writeln!(out);
    let _ = writeln!(out, "  Available commands:");
    let _ = writeln!(out, "    anvil tutorial   Interactive tutorial");
    let _ = writeln!(out, "    anvil audit      Run project audit");
    let _ = writeln!(out, "    anvil doctor     Diagnose your environment");
    let _ = writeln!(out, "    anvil status     Show project status");
    let _ = writeln!(out);
    let _ = writeln!(out, "  Visit: https://docs.eddacraft.ai");
    let _ = writeln!(out);
    let _ = writeln!(out, "{WELCOME_NEXT_STEP}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // UJ-001: every welcome exit carries the user to the daily-value path.
    #[test]
    fn welcome_next_step_names_anvil_start() {
        assert!(
            WELCOME_NEXT_STEP.contains("anvil start"),
            "the welcome next step is `anvil start`, got: {WELCOME_NEXT_STEP}",
        );
    }

    #[test]
    fn plain_welcome_carries_the_next_step_line() {
        let msg = plain_welcome_message();
        assert!(
            msg.contains(WELCOME_NEXT_STEP),
            "the plain welcome surface must end with the next-step line:\n{msg}",
        );
    }

    #[test]
    fn open_docs_message_does_not_panic() {
        let msg = open_docs_message();
        assert!(!msg.is_empty());
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
            // the test writes — not the ambient environment's.
            let _ = Command::new("git")
                .args(["config", "core.excludesfile", "/dev/null"])
                .current_dir(dir)
                .status();
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
