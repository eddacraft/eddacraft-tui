use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anvil_tui::surfaces::welcome::{QuickStartOption, WelcomeState};
use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;

#[derive(Debug, clap::Args)]
pub struct WelcomeArgs {}

#[derive(Debug, Serialize, Deserialize)]
struct FirstRunMarker {
    created_at: String,
    version: String,
}

pub fn run(_args: &WelcomeArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let marker_path = first_run_marker_path();

    if global.no_tui || !std::io::stdout().is_terminal() {
        print_plain_welcome();
        create_first_run_marker(&marker_path)?;
        return Ok(());
    }

    let state = WelcomeState::new();
    let state = crate::tui::run_surface(state)?;

    create_first_run_marker(&marker_path)?;
    launch_chosen_action(state.chosen)?;

    Ok(())
}

fn first_run_marker_path() -> PathBuf {
    PathBuf::from(".anvil").join("first-run")
}

fn create_first_run_marker(path: &PathBuf) -> anyhow::Result<()> {
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
        version: "0.1.0".to_string(),
    };

    let json = serde_json::to_string_pretty(&marker)?;
    std::fs::write(path, json).context("failed to write first-run marker")?;

    Ok(())
}

fn launch_chosen_action(chosen: Option<QuickStartOption>) -> anyhow::Result<()> {
    let args: &[&str] = match chosen {
        Some(QuickStartOption::RunTutorial) => &["tutorial"],
        Some(QuickStartOption::RunAudit) => &["audit"],
        Some(QuickStartOption::RunDoctor) => &["doctor"],
        // Not yet implemented — uncomment as each command ships:
        // Some(QuickStartOption::RunGate) => &["gate"],
        // Some(QuickStartOption::StartWatch) => &["watch"],
        Some(QuickStartOption::ViewDocs) => {
            let url = "https://docs.eddacraft.ai";
            let opened = if cfg!(target_os = "macos") {
                std::process::Command::new("open").arg(url).status().ok()
            } else {
                std::process::Command::new("xdg-open")
                    .arg(url)
                    .status()
                    .ok()
            };
            match opened {
                Some(s) if s.success() => {
                    println!("Opened {url} in your browser");
                }
                _ => {
                    println!("Visit: {url}");
                }
            }
            return Ok(());
        }
        None => return Ok(()),
    };

    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let status = std::process::Command::new(&exe)
        .args(args)
        .status()
        .with_context(|| format!("failed to launch anvil {}", args[0]))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

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
    fn first_run_marker_path_is_correct() {
        let path = first_run_marker_path();
        assert_eq!(path, PathBuf::from(".anvil/first-run"));
    }

    #[test]
    fn create_marker_writes_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first-run");

        create_first_run_marker(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let marker: FirstRunMarker = serde_json::from_str(&content).unwrap();
        assert_eq!(marker.version, "0.1.0");
        assert!(!marker.created_at.is_empty());
    }

    #[test]
    fn launch_chosen_action_docs_does_not_panic() {
        // ViewDocs just prints a URL, doesn't exec — safe to test.
        launch_chosen_action(Some(QuickStartOption::ViewDocs)).unwrap();
        launch_chosen_action(None).unwrap();
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
