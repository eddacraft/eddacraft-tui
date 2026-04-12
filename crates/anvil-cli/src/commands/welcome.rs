use std::io::IsTerminal;

use anvil_tui::surfaces::welcome::{QuickStartOption, WelcomeState};
use eddacraft_tui::theme::EddaCraftTheme;

use crate::GlobalArgs;
use crate::services::first_run::{
    create_first_run_marker, first_run_marker_path, is_first_run, should_skip_welcome,
};
use crate::tui::SurfaceExit;

#[derive(Debug, clap::Args)]
pub struct WelcomeArgs {}

pub fn run(_args: &WelcomeArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let marker_path = first_run_marker_path()?;
    let first_run = is_first_run(&marker_path);

    if global.verbose {
        eprintln!("[welcome] marker_path={}", marker_path.display());
        eprintln!("[welcome] is_first_run={first_run}");
        eprintln!("[welcome] ANVIL_SKIP_WELCOME={}", should_skip_welcome());
    }

    // Env-var bypass: create marker silently and exit.
    if should_skip_welcome() {
        if let Err(err) = create_first_run_marker(&marker_path) {
            eprintln!(
                "[welcome] warning: failed to create first-run marker at {}: {err}",
                marker_path.display()
            );
        }
        return Ok(());
    }

    if global.no_tui || !std::io::stdout().is_terminal() {
        print_plain_welcome();
        create_first_run_marker(&marker_path)?;
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
                        let exit = run_tutorial_with_fix(&mut terminal, &theme, &mut tutorial_state)?;
                        if exit == SurfaceExit::Quit {
                            Ok(())
                        } else {
                            run_welcome_hub(&mut terminal, &theme)
                        }
                    }
                    None => run_welcome_hub(&mut terminal, &theme),
                }
            }
            Ok(OnboardingOutcome::Skip) => run_welcome_hub(&mut terminal, &theme),
            Err(e) => Err(e),
        }
    } else {
        run_welcome_hub(&mut terminal, &theme)
    };

    // Always teardown terminal, even on error.
    let teardown_result = crate::tui::teardown_terminal(&mut terminal);

    // Write marker before propagating errors — a TUI crash should not
    // prevent the marker from being written, otherwise the user is stuck
    // in an onboarding loop on every launch.
    if let Err(err) = create_first_run_marker(&marker_path) {
        eprintln!(
            "[welcome] warning: failed to create first-run marker at {}: {err}",
            marker_path.display()
        );
    }

    // Prefer the app error over the teardown error.
    result.and(teardown_result)?;

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

    // Check for stale tutorial progress from a previous install and offer reset.
    if let Ok(progress_path) = crate::commands::tutorial::progress_file_path() {
        if progress_path.exists() {
            crate::tui::draw_loading(
                terminal,
                "Setup",
                "Previous tutorial progress found \u{2014} resetting for fresh install.",
                theme,
            )?;
            let _ = std::fs::remove_file(&progress_path);
            std::thread::sleep(std::time::Duration::from_millis(600));
        }
    }

    let mut onboarding = OnboardingWelcomeState::new();
    let exit = crate::tui::run_surface_in(terminal, &mut onboarding, theme)?;

    if exit == SurfaceExit::Quit && onboarding.chosen.is_none() {
        return Ok(OnboardingOutcome::Quit);
    }

    match onboarding.chosen {
        Some(OnboardingChoice::GuidedSetup) => {
            run_guided_init(terminal, theme)?;
            Ok(OnboardingOutcome::Configured)
        }
        Some(OnboardingChoice::SkipToTutorial) => Ok(OnboardingOutcome::Tutorial),
        Some(OnboardingChoice::SkipEntirely) | None => Ok(OnboardingOutcome::Skip),
    }
}

fn run_guided_init(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    use anvil_tui::surfaces::onboarding;

    // Skip init if config already exists.
    if onboarding::config_exists() {
        crate::tui::draw_loading(
            terminal,
            "Init",
            "Anvil configuration detected \u{2014} skipping setup.",
            theme,
        )?;
        std::thread::sleep(std::time::Duration::from_millis(200));
        return Ok(());
    }

    let checks = onboarding::default_available_checks();
    let mut init_state = anvil_tui::surfaces::init::InitState::new(checks);
    let exit = crate::tui::run_surface_in(terminal, &mut init_state, theme)?;

    if exit == SurfaceExit::Quit && !init_state.confirmed {
        // User quit the init wizard — don't show error, just fall through.
        return Ok(());
    }

    if init_state.confirmed {
        let checks: Vec<String> = if init_state.config.checks.is_empty() {
            vec!["secret-scan".to_string(), "anti-pattern".to_string()]
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

        let config = crate::commands::init::AnvilConfig {
            schema_version: "1.0.0".to_string(),
            planning_dir: "plans".to_string(),
            format: crate::commands::init::format_label(init_state.config.format),
            checks,
        };

        match crate::commands::init::generate_config(&config, &init_root) {
            Ok(()) => {
                crate::tui::draw_loading(
                    terminal,
                    "Init",
                    "Config saved to .anvilrc. Proceeding to scan\u{2026}",
                    theme,
                )?;
            }
            Err(e) => {
                crate::tui::draw_loading(
                    terminal,
                    "Init",
                    &format!("Warning: could not save config: {e}"),
                    theme,
                )?;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }

    Ok(())
}

fn run_discovery(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<Option<anvil_tui::surfaces::tutorial::discovery::ScanResults>> {
    use anvil_tui::surfaces::tutorial::discovery::{DiscoveryState, ScanResults};
    use anvil_tui::surfaces::tutorial::showcase;

    let mut discovery = DiscoveryState::new();

    let results = match scan_project() {
        Ok(results) if results.findings.is_empty() => {
            // Clean project — show showcase examples so user sees capabilities.
            let findings = showcase::showcase_findings();
            ScanResults {
                findings,
                files_scanned: results.files_scanned,
                duration_ms: results.duration_ms,
            }
        }
        Ok(results) => results,
        Err(_) => {
            // Scan failed — fall back to showcase mode.
            let findings = showcase::showcase_findings();
            ScanResults {
                findings,
                files_scanned: 0,
                duration_ms: 0,
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
fn scan_project() -> anyhow::Result<anvil_tui::surfaces::tutorial::discovery::ScanResults> {
    use anvil_checks::filter::ScanFilter;
    use anvil_tui::surfaces::tutorial::discovery::{
        Finding, FindingSeverity, FindingSource, ScanResults,
    };

    let start = std::time::Instant::now();
    let filter = ScanFilter::default_excludes();
    let cwd = std::env::current_dir()?;
    let secret_config = anvil_checks::secret::types::SecretCheckConfig::default();

    let mut findings = Vec::new();
    let mut files_scanned: usize = 0;
    const MAX_FILES: usize = 500;

    for entry in walkdir::WalkDir::new(&cwd)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !filter.includes(path) {
            continue;
        }
        // Skip binary / non-text files by extension.
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(
            ext,
            "png" | "jpg" | "jpeg" | "gif" | "ico" | "woff" | "woff2" | "ttf"
                | "otf" | "eot" | "pdf" | "zip" | "gz" | "tar" | "exe"
                | "dll" | "so" | "dylib" | "wasm" | "o" | "a"
        ) {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };

        let rel_path = path
            .strip_prefix(&cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Secret scan
        let secret_hits =
            anvil_checks::secret::scanner::scan_content(&content, &rel_path, &secret_config);
        for hit in &secret_hits {
            findings.push(Finding {
                file: hit.file.clone(),
                line: Some(hit.line),
                severity: FindingSeverity::Error,
                source: FindingSource::Secret,
                title: format!("Secret detected: {}", hit.pattern_name),
                message: hit.redacted_line.clone(),
                suggestion: "Move the value to an environment variable or secrets manager."
                    .to_string(),
            });
        }

        // Antipattern scan
        let ap_result =
            anvil_checks::antipattern::scanner::scan_file(&rel_path, &content, None);
        for warning in &ap_result.warnings {
            if warning.suppressed.is_some() {
                continue;
            }
            findings.push(Finding {
                file: warning.location.file.clone(),
                line: Some(warning.location.line),
                severity: match warning.severity {
                    anvil_checks::antipattern::types::WarningSeverity::Error => {
                        FindingSeverity::Error
                    }
                    anvil_checks::antipattern::types::WarningSeverity::Warning => {
                        FindingSeverity::Warning
                    }
                    anvil_checks::antipattern::types::WarningSeverity::Info => {
                        FindingSeverity::Info
                    }
                },
                source: FindingSource::AntiPattern,
                title: warning.title.clone(),
                message: warning.message.clone(),
                suggestion: warning.suggestion.clone(),
            });
        }

        files_scanned += 1;
        if files_scanned >= MAX_FILES {
            break;
        }
    }

    // Sort by severity descending (Error first).
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ScanResults {
        findings,
        files_scanned,
        duration_ms,
    })
}

/// Run the tutorial surface in a loop, handling 'f' fix requests.
/// When the user presses 'f' in the tutorial, we exit the surface, create a
/// FixState for the top finding, run it, then resume the tutorial.
fn run_tutorial_with_fix(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
    tutorial_state: &mut anvil_tui::surfaces::tutorial::TutorialState,
) -> anyhow::Result<SurfaceExit> {
    loop {
        let exit = crate::tui::run_surface_in(terminal, tutorial_state, theme)?;

        if tutorial_state.wants_fix {
            tutorial_state.wants_fix = false;

            // Get the top finding from domain_findings.
            let finding = tutorial_state
                .domain_findings
                .as_ref()
                .and_then(|d| d.top_findings(1).into_iter().next().cloned());

            if let Some(finding) = finding {
                let mut fix_state =
                    anvil_tui::surfaces::tutorial::fix::FixState::new(finding.clone());

                // Load file context around the finding for display.
                if let Ok(content) = std::fs::read_to_string(&finding.file) {
                    let all_lines: Vec<String> =
                        content.lines().map(|l| l.to_string()).collect();
                    let target = finding.line.unwrap_or(1).saturating_sub(1);
                    let start = target.saturating_sub(5);
                    let end = (target + 6).min(all_lines.len());
                    let context: Vec<String> = all_lines[start..end].to_vec();
                    fix_state.set_context(context, start + 1);
                }

                let fix_exit = crate::tui::run_surface_in(terminal, &mut fix_state, theme)?;
                if fix_exit == SurfaceExit::Quit {
                    return Ok(SurfaceExit::Quit);
                }
            }
            // Resume the tutorial — loop back to run_surface_in.
            continue;
        }

        return Ok(exit);
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
        ..Default::default()
    };

    let watch_config = anvil_kernel::watch::WatchConfig {
        root: workspace_root,
        architecture_config: None,
        watcher: watcher_config,
        include_patterns: vec!["**/*".to_string()],
        exclude_patterns: Vec::new(),
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
        });

    let exit = crate::tui::run_watch_in(terminal, &mut state, &event_rx)?;

    handle.stop().context("stopping watcher")?;
    Ok(exit)
}

fn run_welcome_hub(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    theme: &EddaCraftTheme,
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
                crate::tui::draw_loading(
                    terminal,
                    "Watch",
                    "Starting file watcher...",
                    theme,
                )?;
                match start_watch_from_hub(terminal, theme) {
                    Ok(SurfaceExit::Quit) => break,
                    Ok(SurfaceExit::Back) => {}
                    Err(e) => {
                        welcome.status_message =
                            Some(format!("Watch mode failed: {e}"));
                    }
                }
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::ViewDocs) => {
                welcome.status_message = Some(open_docs_message());
                welcome.should_quit = false;
            }
            Some(QuickStartOption::RunAudit) => {
                crate::tui::draw_loading(terminal, "Audit", "Scanning project...", theme)?;
                let data = crate::commands::audit::collect_audit_data();
                let mut audit_state = anvil_tui::surfaces::audit::AuditState::new(data);
                let sub_exit = crate::tui::run_surface_in(terminal, &mut audit_state, theme)?;
                if sub_exit == SurfaceExit::Quit {
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
                    let sub_exit =
                        crate::tui::run_surface_in(terminal, &mut doctor_state, theme)?;
                    if doctor_state.wants_fix {
                        if let Some(idx) = doctor_state.fix_index {
                            crate::commands::doctor::apply_fix_at(
                                &mut doctor_state.checks,
                                idx,
                            );
                        }
                        doctor_state.wants_fix = false;
                        doctor_state.fix_index = None;
                        continue;
                    }
                    if sub_exit == SurfaceExit::Quit {
                        break;
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
                    // No file watcher available from the welcome hub, so run
                    // in static mode (watch/demo steps become informational).
                    tutorial_state.enable_static_mode();
                    let sub_exit =
                        crate::tui::run_surface_in(terminal, &mut tutorial_state, theme)?;
                    if sub_exit == SurfaceExit::Quit {
                        break;
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

fn print_plain_welcome() {
    println!();
    println!("  Welcome to Anvil");
    println!("  Structural governance for AI-assisted development");
    println!();
    println!("  Available commands:");
    println!("    anvil tutorial   Interactive tutorial");
    println!("    anvil audit      Run project audit");
    println!("    anvil doctor     Diagnose your environment");
    println!("    anvil status     Show project status");
    println!();
    println!("  Visit: https://docs.eddacraft.ai");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_docs_message_does_not_panic() {
        let msg = open_docs_message();
        assert!(!msg.is_empty());
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
