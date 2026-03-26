use std::io::IsTerminal;
use std::path::PathBuf;

use anvil_tui::surfaces::tutorial::{TutorialPath, TutorialPhase, TutorialState};
use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;

#[derive(Debug, clap::Args)]
pub struct TutorialArgs {
    /// Reset tutorial progress
    #[arg(long)]
    reset: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct TutorialProgress {
    completed_paths: Vec<String>,
}

pub fn run(args: &TutorialArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let progress_path = progress_file_path()?;

    if args.reset {
        return reset_progress(&progress_path);
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        println!("Tutorial requires an interactive terminal. Run without --no-tui.");
        return Ok(());
    }

    let progress = load_progress(&progress_path);
    let state = TutorialState::new();

    let state = crate::tui::run_surface(state)?;

    save_progress_from_state(&progress_path, &progress, &state)?;

    Ok(())
}

fn progress_file_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".anvil").join("tutorial-progress.json"))
}

fn reset_progress(path: &PathBuf) -> anyhow::Result<()> {
    if path.exists() {
        std::fs::remove_file(path).context("failed to remove progress file")?;
    }
    println!("Tutorial progress reset.");
    Ok(())
}

fn load_progress(path: &PathBuf) -> TutorialProgress {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_progress_from_state(
    path: &PathBuf,
    existing: &TutorialProgress,
    state: &TutorialState,
) -> anyhow::Result<()> {
    let mut completed: Vec<String> = existing.completed_paths.clone();

    // Check if the current path was fully completed in this session
    if state.phase == TutorialPhase::Complete
        && let Some(path_enum) = state.chosen_path
    {
        let label = path_enum.label().to_string();
        if !completed.contains(&label) {
            completed.push(label);
        }
    }

    // Also check paths where all steps are completed (user might have
    // completed a path, returned to path select, then quit)
    for tutorial_path in &state.paths {
        let label = tutorial_path.label().to_string();
        if !completed.contains(&label) && is_path_completed(state, *tutorial_path) {
            completed.push(label);
        }
    }

    let progress = TutorialProgress {
        completed_paths: completed,
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("failed to create progress directory")?;
    }

    let json = serde_json::to_string_pretty(&progress)?;
    std::fs::write(path, json).context("failed to write progress file")?;

    Ok(())
}

fn is_path_completed(state: &TutorialState, path: TutorialPath) -> bool {
    if state.chosen_path != Some(path) {
        return false;
    }
    !state.steps.is_empty() && state.steps.iter().all(|s| s.completed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eddacraft_tui::keyboard::Action;

    #[test]
    fn progress_file_under_home() {
        let path = progress_file_path().unwrap();
        assert!(path.to_string_lossy().contains(".anvil"));
        assert!(path.to_string_lossy().ends_with("tutorial-progress.json"));
    }

    #[test]
    fn load_missing_progress_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent-anvil-test/progress.json");
        let progress = load_progress(&path);
        assert!(progress.completed_paths.is_empty());
    }

    #[test]
    fn save_and_load_progress() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");

        let existing = TutorialProgress::default();
        let mut state = TutorialState::new();

        // Simulate completing the Policy path
        state.handle_key(Action::Select); // choose Policy
        let step_count = state.steps.len();
        for _ in 0..step_count {
            state.handle_key(Action::Select);
        }

        save_progress_from_state(&path, &existing, &state).unwrap();

        let loaded = load_progress(&path);
        assert!(loaded.completed_paths.contains(&"Policy".to_string()));
    }

    #[test]
    fn reset_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");
        std::fs::write(&path, "{}").unwrap();
        assert!(path.exists());

        reset_progress(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn reset_nonexistent_file_succeeds() {
        let path = PathBuf::from("/tmp/nonexistent-anvil-test/progress.json");
        reset_progress(&path).unwrap();
    }

    #[test]
    fn is_path_completed_when_all_steps_done() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        let step_count = state.steps.len();
        for _ in 0..step_count {
            state.handle_key(Action::Select);
        }
        // After completing all steps, phase is Complete
        assert_eq!(state.phase, TutorialPhase::Complete);
        assert!(is_path_completed(&state, TutorialPath::Policy));
    }

    #[test]
    fn is_path_completed_when_partially_done() {
        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        state.handle_key(Action::Select); // complete first step only
        assert!(!is_path_completed(&state, TutorialPath::Policy));
    }

    #[test]
    fn existing_progress_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");

        let existing = TutorialProgress {
            completed_paths: vec!["Architecture".to_string()],
        };

        let mut state = TutorialState::new();
        state.handle_key(Action::Select); // choose Policy
        let step_count = state.steps.len();
        for _ in 0..step_count {
            state.handle_key(Action::Select);
        }

        save_progress_from_state(&path, &existing, &state).unwrap();

        let loaded = load_progress(&path);
        assert!(loaded.completed_paths.contains(&"Architecture".to_string()));
        assert!(loaded.completed_paths.contains(&"Policy".to_string()));
    }
}
