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
                        let sub_exit = crate::tui::run_surface_in(
                            &mut terminal,
                            &mut tutorial_state,
                            &theme,
                        )?;
                        if sub_exit == SurfaceExit::Quit {
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
        // TODO: Write config based on init_state.config.
        // Discovery scan runs after init returns (wired in run()).
        crate::tui::draw_loading(
            terminal,
            "Init",
            "Setup complete. Proceeding to welcome\u{2026}",
            theme,
        )?;
        std::thread::sleep(std::time::Duration::from_millis(200));
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

    // TODO(WELCOME-013): Replace with real project scan using anvil-checks scanners
    // and kernel run_embedded(). For now, use showcase findings so the user sees
    // what Anvil is capable of detecting.
    let findings = showcase::showcase_findings();
    let results = ScanResults {
        findings,
        files_scanned: 0,
        duration_ms: 0,
    };
    discovery.set_results(results);

    let exit = crate::tui::run_surface_in(terminal, &mut discovery, theme)?;

    if exit == SurfaceExit::Quit && !discovery.wants_continue {
        return Ok(None);
    }

    Ok(discovery.results)
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
                welcome.status_message = Some(
                    "Watch mode requires a kernel watcher channel. Run \u{2018}anvil watch\u{2019} from the command line."
                        .to_string(),
                );
                welcome.should_quit = false;
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
                let sub_exit = crate::tui::run_surface_in(terminal, &mut doctor_state, theme)?;
                if sub_exit == SurfaceExit::Quit {
                    break;
                }
                welcome.should_quit = false;
                welcome.chosen = None;
            }
            Some(QuickStartOption::RunTutorial) => {
                welcome.status_message = None;
                match run_discovery(terminal, theme)? {
                    Some(results) => {
                        let mut tutorial_state =
                            anvil_tui::surfaces::tutorial::TutorialState::new();
                        tutorial_state.set_scan_results(results);
                        let sub_exit =
                            crate::tui::run_surface_in(terminal, &mut tutorial_state, theme)?;
                        if sub_exit == SurfaceExit::Quit {
                            break;
                        }
                    }
                    None => {
                        // User quit during discovery — return to welcome hub.
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
