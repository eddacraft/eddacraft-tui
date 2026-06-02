use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anvil_tui::surfaces::tutorial::{
    STATIC_MODE_WATCHER_UNAVAILABLE, TutorialPath, TutorialPhase, TutorialState,
};
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
    /// In-progress session that can be resumed after an interruption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    in_progress: Option<InProgressSession>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InProgressSession {
    path: String,
    current_step: usize,
    steps_completed: Vec<bool>,
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
    let mut state = TutorialState::new();

    // Populate completed paths so the selector shows checkmarks.
    let completed: Vec<TutorialPath> = progress
        .completed_paths
        .iter()
        .filter_map(|s| TutorialPath::from_label(s))
        .collect();
    state.set_completed_paths(completed);

    // Resume an interrupted session if one exists. Unrecognised path
    // labels (e.g. after a rename) are silently dropped — the next save
    // overwrites the stale entry.
    if let Some(ref session) = progress.in_progress
        && let Some(path) = TutorialPath::from_label(&session.path)
    {
        state.resume_path(path, session.current_step, &session.steps_completed);
    }

    // Start a file watcher for live verification on watched steps
    // (WELCOME-013). Falls back to keyboard-only mode if the watcher
    // cannot start (e.g. inotify limit reached). The in-TUI notice
    // surfaces the specific cause so users aren't left wondering why
    // file saves stop retriggering checks.
    let (file_rx, _watcher_handle) = if let Ok((rx, handle)) = try_start_watcher() {
        (Some(rx), Some(handle))
    } else {
        state.enable_static_mode_with_reason(STATIC_MODE_WATCHER_UNAVAILABLE);
        (None, None)
    };

    let mut state = crate::tui::run_tutorial(state, file_rx.as_ref())?;

    // Handle watch demo requests (WELCOME-014). The tutorial exits when a
    // watch_demo step is activated. We launch the demo, then resume.
    while state.wants_watch_demo {
        state.wants_watch_demo = false;
        if run_watch_demo_for_tutorial().is_ok() {
            state.advance_step();
        }
        state = crate::tui::run_tutorial(state, file_rx.as_ref())?;
    }

    save_progress_from_state(&progress_path, &progress, &state)?;

    Ok(())
}

/// Launch the watch mode demo (WELCOME-014). Starts the kernel engine
/// watcher, runs the demo surface, and returns when the user exits.
fn run_watch_demo_for_tutorial() -> anyhow::Result<()> {
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
        warmup_paths: Vec::new(),
    };

    let (event_tx, event_rx) = std::sync::mpsc::channel();

    let handle = anvil_kernel::watch::run_watch(&watch_config, event_tx)
        .context("starting engine watcher for demo")?;

    let data = anvil_tui::surfaces::watch::WatchData {
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
    };

    let state = anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState::new(data);
    let demo_result = crate::tui::run_watch_demo(state, &event_rx);

    // Always stop the watcher, regardless of whether the demo succeeded.
    handle.stop().context("stopping demo watcher")?;
    demo_result
}

/// Try to start a file watcher on the current working directory.
/// Returns the change-batch receiver and watcher handle on success.
/// The caller holds the handle to keep the watcher alive; dropping it stops watching.
fn try_start_watcher() -> anyhow::Result<(
    std::sync::mpsc::Receiver<anvil_kernel::watcher::events::ChangeBatch>,
    anvil_kernel::watcher::WatcherHandle,
)> {
    let root = std::env::current_dir().context("cannot determine working directory")?;
    let config = anvil_kernel::watcher::WatcherConfig {
        root,
        debounce_window: std::time::Duration::from_millis(200),
        ..Default::default()
    };
    let (handle, rx, _diag) = anvil_kernel::watcher::start_watcher(&config, None)?;
    Ok((rx, handle))
}

pub(crate) fn progress_file_path() -> anyhow::Result<PathBuf> {
    let home = crate::util::user_home_dir().context("could not determine home directory")?;
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
    path: &Path,
    existing: &TutorialProgress,
    state: &TutorialState,
) -> anyhow::Result<()> {
    // Normalise any legacy path labels in the existing file to their current
    // canonical form so a re-completion after a label rename does not write a
    // duplicate entry (e.g. "Policy" alongside "Policy checks").
    let mut completed: Vec<String> = Vec::with_capacity(existing.completed_paths.len());
    for entry in &existing.completed_paths {
        let canonical = TutorialPath::from_label(entry)
            .map_or_else(|| entry.clone(), |p| p.label().to_string());
        if !completed.contains(&canonical) {
            completed.push(canonical);
        }
    }

    // Check if the current path was fully completed in this session.
    if state.phase == TutorialPhase::Complete
        && let Some(path_enum) = state.chosen_path
    {
        let label = path_enum.label().to_string();
        if !completed.contains(&label) {
            completed.push(label);
        }
    }

    // Also check paths where all steps are completed (user might have
    // completed a path, returned to path select, then quit).
    for tutorial_path in &state.paths {
        let label = tutorial_path.label().to_string();
        if !completed.contains(&label) && is_path_completed(state, *tutorial_path) {
            completed.push(label);
        }
    }

    // Persist in-progress state if the user quit mid-path so the next
    // launch can resume from where they left off. Don't save in-progress
    // for a path that's already in completed_paths (redo scenario).
    let in_progress = if state.phase == TutorialPhase::Running
        && let Some(path_enum) = state.chosen_path
        && !state.steps.is_empty()
        && !completed.contains(&path_enum.label().to_string())
    {
        Some(InProgressSession {
            path: path_enum.label().to_string(),
            current_step: state.current_step,
            steps_completed: state.steps.iter().map(|s| s.completed).collect(),
        })
    } else {
        None
    };

    let progress = TutorialProgress {
        completed_paths: completed,
        in_progress,
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("failed to create progress directory")?;
    }

    let json = serde_json::to_string_pretty(&progress)?;
    crate::util::atomic_write(path, json.as_bytes()).context("failed to write progress file")?;

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
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.json");
        let progress = load_progress(&path);
        assert!(progress.completed_paths.is_empty());
    }

    /// Helper: load a path's steps and mark them all completed, transitioning
    /// to the Complete phase. This avoids executing real commands in tests.
    fn complete_path(state: &mut TutorialState, path: TutorialPath) {
        state.load_steps(path);
        for step in &mut state.steps {
            step.completed = true;
        }
        state.phase = TutorialPhase::Complete;
    }

    #[test]
    fn save_and_load_progress() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");

        let existing = TutorialProgress::default();
        let mut state = TutorialState::new();
        complete_path(&mut state, TutorialPath::Policy);

        save_progress_from_state(&path, &existing, &state).unwrap();

        let loaded = load_progress(&path);
        assert!(
            loaded
                .completed_paths
                .contains(&"Policy checks".to_string())
        );
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
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.json");
        reset_progress(&path).unwrap();
    }

    #[test]
    fn is_path_completed_when_all_steps_done() {
        let mut state = TutorialState::new();
        complete_path(&mut state, TutorialPath::Policy);
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

        // Seed the existing file with a legacy label to exercise the
        // on-save normalisation: older builds wrote "Architecture" before
        // labels were reframed to "Boundary findings".
        let existing = TutorialProgress {
            completed_paths: vec!["Architecture".to_string()],
            in_progress: None,
        };

        let mut state = TutorialState::new();
        complete_path(&mut state, TutorialPath::Policy);

        save_progress_from_state(&path, &existing, &state).unwrap();

        let loaded = load_progress(&path);
        // Legacy "Architecture" is migrated to the current canonical label.
        assert!(
            loaded
                .completed_paths
                .contains(&"Boundary findings".to_string())
        );
        assert!(
            loaded
                .completed_paths
                .contains(&"Policy checks".to_string())
        );
        // No duplicate legacy entry should remain in the saved file.
        assert!(!loaded.completed_paths.contains(&"Architecture".to_string()));
    }

    // --- In-progress persistence tests ---

    #[test]
    fn quit_mid_path_saves_in_progress() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");

        let mut state = TutorialState::new();
        state.load_steps(TutorialPath::Drift);
        // Complete step 0 then quit mid-path.
        state.steps[0].completed = true;
        state.current_step = 1;
        // Phase is still Running — user pressed q.

        save_progress_from_state(&path, &TutorialProgress::default(), &state).unwrap();

        let loaded = load_progress(&path);
        let session = loaded.in_progress.expect("should have in_progress");
        assert_eq!(session.path, "Configuration drift");
        assert_eq!(session.current_step, 1);
        assert_eq!(session.steps_completed.len(), state.steps.len());
        assert!(session.steps_completed[0]);
        assert!(!session.steps_completed[1]);
    }

    #[test]
    fn completed_path_clears_in_progress() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");

        let mut state = TutorialState::new();
        complete_path(&mut state, TutorialPath::Policy);
        // Phase is Complete — should not save in_progress.

        save_progress_from_state(&path, &TutorialProgress::default(), &state).unwrap();

        let loaded = load_progress(&path);
        assert!(loaded.in_progress.is_none());
        assert!(
            loaded
                .completed_paths
                .contains(&"Policy checks".to_string())
        );
    }

    #[test]
    fn load_progress_with_in_progress_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");

        let json = r#"{
            "completed_paths": ["Policy"],
            "in_progress": {
                "path": "Drift",
                "current_step": 2,
                "steps_completed": [true, true, false]
            }
        }"#;
        std::fs::write(&path, json).unwrap();

        let loaded = load_progress(&path);
        assert!(loaded.completed_paths.contains(&"Policy".to_string()));
        let session = loaded.in_progress.expect("should parse in_progress");
        assert_eq!(session.path, "Drift");
        assert_eq!(session.current_step, 2);
        assert_eq!(session.steps_completed, vec![true, true, false]);
    }

    #[test]
    fn load_progress_without_in_progress_field() {
        // Backwards-compat: old progress files lack in_progress.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tutorial-progress.json");
        std::fs::write(&path, r#"{"completed_paths":["CI Integration"]}"#).unwrap();

        let loaded = load_progress(&path);
        assert!(
            loaded
                .completed_paths
                .contains(&"CI Integration".to_string())
        );
        assert!(loaded.in_progress.is_none());
    }
}
