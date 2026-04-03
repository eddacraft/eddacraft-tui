use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anvil_tui::surfaces::welcome::{QuickStartOption, WelcomeState};
use anyhow::Context;
use eddacraft_tui::theme::EddaCraftTheme;
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;
use crate::tui::SurfaceExit;

#[derive(Debug, clap::Args)]
pub struct WelcomeArgs {}

#[derive(Debug, Serialize, Deserialize)]
struct FirstRunMarker {
    created_at: String,
    version: String,
}

pub fn run(_args: &WelcomeArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let marker_path = first_run_marker_path()?;

    if global.no_tui || !std::io::stdout().is_terminal() {
        print_plain_welcome();
        create_first_run_marker(&marker_path)?;
        return Ok(());
    }

    let mut terminal = crate::tui::setup_terminal()?;
    let theme = EddaCraftTheme;

    let result = run_welcome_hub(&mut terminal, &theme);

    // Always teardown terminal, even on error.
    // Teardown runs before propagating the hub error so the terminal
    // is always restored, but we preserve the hub error when both fail.
    let teardown_result = crate::tui::teardown_terminal(&mut terminal);
    result.or(teardown_result)?;
    create_first_run_marker(&marker_path)?;

    Ok(())
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
                let mut tutorial_state = anvil_tui::surfaces::tutorial::TutorialState::new();
                let sub_exit = crate::tui::run_surface_in(terminal, &mut tutorial_state, theme)?;
                if sub_exit == SurfaceExit::Quit {
                    break;
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

fn first_run_marker_path() -> anyhow::Result<PathBuf> {
    Ok(crate::util::workspace_root()?
        .join(".anvil")
        .join("first-run"))
}

fn create_first_run_marker(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("failed to create .anvil directory")?;
    }

    let marker = FirstRunMarker {
        created_at: format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let json = serde_json::to_string_pretty(&marker)?;
    crate::util::atomic_write(path, json.as_bytes()).context("failed to write first-run marker")?;

    Ok(())
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
    fn first_run_marker_path_is_anchored() {
        let path = first_run_marker_path().unwrap();
        assert!(
            path.is_absolute(),
            "marker path should be absolute, got: {path:?}"
        );
        assert!(path.ends_with(".anvil/first-run"));
    }

    #[test]
    fn create_marker_writes_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first-run");

        create_first_run_marker(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let marker: FirstRunMarker = serde_json::from_str(&content).unwrap();
        assert_eq!(marker.version, env!("CARGO_PKG_VERSION"));
        assert!(!marker.created_at.is_empty());
    }

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
}
