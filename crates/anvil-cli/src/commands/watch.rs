use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct WatchArgs {
    /// File or directory to scope the watcher (when a file is given, its
    /// parent directory is watched; other files there may also trigger events)
    #[arg(long, short = 'f')]
    file: Option<String>,

    /// Action to run on change: gate, check
    #[arg(long, short)]
    action: Option<String>,

    /// Watch planning documents
    #[arg(long)]
    plans: bool,

    /// Watch source files
    #[arg(long)]
    source: bool,

    /// Watch everything
    #[arg(long)]
    all: bool,

    /// Glob patterns to watch (comma-separated)
    #[arg(long)]
    patterns: Option<String>,

    /// Directory names to exclude (comma-separated, e.g. "vendor,tmp")
    #[arg(long)]
    exclude: Option<String>,

    /// Debounce interval in milliseconds
    #[arg(long)]
    debounce: Option<u64>,
}

const DEFAULT_WATCH_PATTERNS: &[&str] = &[
    "**/*.md",
    "**/*.aps.md",
    "**/prd.*",
    "**/plan.*",
    "**/spec.*",
];

const SOURCE_PATTERNS: &[&str] = &[
    "src/**/*.ts",
    "src/**/*.tsx",
    "lib/**/*.ts",
    "crates/**/*.rs",
];

// Default excludes are handled by FileFilter::default_patterns() in the kernel.
// CLI --exclude adds to those defaults via build_filter().

#[derive(Debug, Serialize)]
struct WatchEvent {
    timestamp: String,
    event_type: String,
    detail: String,
}

/// Normalise a path by canonicalising the longest existing ancestor, then
/// re-appending the remaining suffix. This resolves `..` traversal even when
/// the full path doesn't exist on disk.
fn normalise_path_via_ancestors(path: &std::path::Path) -> PathBuf {
    let mut ancestors: Vec<&std::path::Path> = path.ancestors().collect();
    ancestors.reverse(); // root first

    for ancestor in &ancestors {
        if let Ok(canon) = ancestor.canonicalize()
            && let Ok(suffix) = path.strip_prefix(ancestor)
        {
            // Re-append the remaining components and clean up any remaining ..
            let mut result = canon;
            for component in suffix.components() {
                match component {
                    std::path::Component::ParentDir => {
                        result.pop();
                    }
                    std::path::Component::Normal(c) => {
                        result.push(c);
                    }
                    _ => {}
                }
            }
            return result;
        }
    }
    // Absolute fallback: just return the original
    path.to_path_buf()
}

/// Resolve the effective watch root: if `--file` is given, scope to that path.
/// Returns an error if the resolved path escapes the workspace boundary.
fn resolve_watch_root(workspace_root: &std::path::Path, file_arg: Option<&str>) -> Result<PathBuf> {
    match file_arg {
        Some(f) => {
            let p = std::path::Path::new(f);
            let abs = if p.is_absolute() {
                p.to_path_buf()
            } else {
                workspace_root.join(p)
            };
            // Canonicalise to resolve .. traversal. For non-existent paths,
            // canonicalise the longest existing ancestor then re-append the rest.
            let resolved = abs
                .canonicalize()
                .unwrap_or_else(|_| normalise_path_via_ancestors(&abs));

            // Validate the resolved path is within the workspace
            let canon_ws = workspace_root
                .canonicalize()
                .unwrap_or_else(|_| workspace_root.to_path_buf());
            if !resolved.starts_with(&canon_ws) {
                bail!(
                    "Watch path '{}' escapes workspace root '{}'",
                    resolved.display(),
                    canon_ws.display()
                );
            }

            if resolved.is_dir() {
                Ok(resolved)
            } else {
                // Single file — watch its parent directory
                Ok(resolved
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .map_or_else(|| workspace_root.to_path_buf(), PathBuf::from))
            }
        }
        None => Ok(workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.to_path_buf())),
    }
}

/// Build a `FileFilter` from CLI patterns and exclude args.
fn build_filter(exclude: &[String]) -> anvil_kernel::watcher::filter::FileFilter {
    if exclude.is_empty() {
        anvil_kernel::watcher::filter::FileFilter::default()
    } else {
        let mut patterns = anvil_kernel::watcher::filter::FileFilter::default_patterns();
        for ex in exclude {
            let cleaned = ex.trim_end_matches('/').trim_end_matches("/**");
            let cleaned = std::path::Path::new(cleaned)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(cleaned);
            if !patterns.iter().any(|p| p == cleaned) {
                patterns.push(cleaned.to_string());
            }
        }
        anvil_kernel::watcher::filter::FileFilter::new(patterns)
    }
}

/// Validate the `--action` argument.
fn validate_action(action: Option<&str>) -> Result<Option<&str>> {
    match action {
        Some("gate" | "check") | None => Ok(action),
        Some(other) => bail!("Unsupported action: {other}. Supported: gate, check"),
    }
}

/// Build the Command for action dispatch (extracted for testability).
fn build_action_command(
    exe: &std::path::Path,
    action: &str,
    workspace_root: &std::path::Path,
    json: bool,
    no_tui: bool,
) -> std::process::Command {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg(action);
    if json {
        cmd.arg("--json");
    }
    if no_tui {
        cmd.arg("--no-tui");
    }
    cmd.current_dir(workspace_root);
    if json {
        cmd.stdout(std::process::Stdio::null());
    } else {
        cmd.stdout(std::process::Stdio::inherit());
    }
    cmd.stderr(std::process::Stdio::inherit());
    cmd
}

/// Run the specified action when a file change is detected.
/// Uses inherited stdio for real-time output streaming (C-007).
fn dispatch_action(action: &str, workspace_root: &std::path::Path, json: bool, no_tui: bool) {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("\u{2717} Cannot resolve current executable: {e}");
            return;
        }
    };
    let mut cmd = build_action_command(&exe, action, workspace_root, json, no_tui);

    match cmd.spawn().and_then(|mut child| child.wait()) {
        Ok(status) => {
            if !status.success() {
                eprintln!(
                    "\u{26a0} Action '{action}' exited with code {}",
                    status.code().unwrap_or(-1)
                );
            }
        }
        Err(e) => {
            eprintln!("\u{2717} Failed to run action '{action}': {e}");
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn run(args: &WatchArgs, global: &GlobalArgs) -> Result<()> {
    let workspace_root = crate::util::workspace_root()?;
    let action = validate_action(args.action.as_deref())?;

    // Reject --action in TUI mode (action dispatch requires non-interactive output)
    if action.is_some() && !(global.json || !std::io::stdout().is_terminal() || global.no_tui) {
        bail!(
            "--action requires --no-tui or --json mode (TUI action dispatch is not yet supported)"
        );
    }

    // Resolve watch root — if --file is given, scope to that path
    let watch_root = resolve_watch_root(&workspace_root, args.file.as_deref())?;

    // Build include patterns — passed to kernel WatchConfig
    let patterns: Vec<String> = if let Some(ref p) = args.patterns {
        p.split(',').map(|s| s.trim().to_string()).collect()
    } else if args.all || (args.source && args.plans) {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .chain(SOURCE_PATTERNS.iter())
            .map(ToString::to_string)
            .collect()
    } else if args.source {
        SOURCE_PATTERNS.iter().map(ToString::to_string).collect()
    } else {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect()
    };

    // Build exclude patterns and create file filter
    let exclude: Vec<String> = args.exclude.as_ref().map_or_else(Vec::new, |s| {
        s.split(',').map(|s| s.trim().to_string()).collect()
    });
    let filter = build_filter(&exclude);

    let arch_config_path = workspace_root.join(".anvil").join("architecture.yaml");
    let arch_config = if arch_config_path.exists() {
        Some(arch_config_path)
    } else {
        None
    };

    let watcher_config = anvil_kernel::watcher::WatcherConfig {
        root: watch_root.clone(),
        debounce_window: std::time::Duration::from_millis(args.debounce.unwrap_or(300)),
        filter: Some(filter),
        ..Default::default()
    };

    let watch_config = anvil_kernel::watch::WatchConfig {
        root: watch_root.clone(),
        architecture_config: arch_config.clone(),
        watcher: watcher_config,
        include_patterns: patterns,
        exclude_patterns: exclude.clone(),
    };

    let (event_tx, event_rx) = mpsc::channel();

    let handle = anvil_kernel::watch::run_watch(&watch_config, event_tx)
        .context("starting kernel watcher")?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_flag = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        shutdown_flag.store(true, Ordering::SeqCst);
    })
    .context("setting Ctrl-C handler")?;

    if global.json || !std::io::stdout().is_terminal() || global.no_tui {
        let mut snapshot_count: u64 = 0;
        let action_running = Arc::new(AtomicBool::new(false));
        let action_pending = Arc::new(AtomicBool::new(false));

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            match event_rx.recv_timeout(std::time::Duration::from_millis(250)) {
                Ok(event) => {
                    if global.json {
                        let watch_event = WatchEvent {
                            timestamp: event.timestamp.clone(),
                            event_type: format!("{:?}", event.event_type),
                            detail: format!("{:?}", event.payload),
                        };
                        println!("{}", serde_json::to_string(&watch_event)?);
                    } else {
                        print_event_plain(&event);
                    }

                    // Dispatch action on snapshot events (skip initial scan, guard concurrency)
                    if let Some(action) = action
                        && matches!(event.event_type, anvil_kernel_types::EventType::Snapshot)
                    {
                        snapshot_count += 1;
                        if snapshot_count > 1 {
                            if action_running.swap(true, Ordering::SeqCst) {
                                // Action already running — mark pending rerun
                                action_pending.store(true, Ordering::SeqCst);
                            } else {
                                let flag = Arc::clone(&action_running);
                                let pending = Arc::clone(&action_pending);
                                let act = action.to_string();
                                let ws = workspace_root.clone();
                                let g_json = global.json;
                                let g_no_tui = global.no_tui;
                                std::thread::spawn(move || {
                                    // Run action, then loop while pending reruns exist
                                    loop {
                                        dispatch_action(&act, &ws, g_json, g_no_tui);
                                        if !pending.swap(false, Ordering::SeqCst) {
                                            break;
                                        }
                                    }
                                    flag.store(false, Ordering::SeqCst);
                                });
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    } else {
        let state =
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
        crate::tui::run_watch(state, &event_rx, Some(&shutdown))?;
    }

    handle.stop().context("stopping watcher")?;
    Ok(())
}

fn print_event_plain(event: &anvil_kernel_types::EngineEvent) {
    use anvil_kernel_types::{EventPayload, EventType};

    let prefix = match event.event_type {
        EventType::Progress => "\u{25b6}",
        EventType::Snapshot => "\u{1f4f8}",
        EventType::Violation => "\u{26a0}",
        EventType::Error => "\u{2717}",
    };

    match &event.payload {
        EventPayload::Progress {
            phase,
            current,
            total,
        } => {
            println!("{prefix} {phase}: {current}/{total}");
        }
        EventPayload::Snapshot {
            node_count,
            edge_count,
            files_watched,
        } => {
            println!(
                "{prefix} Snapshot: {node_count} nodes, {edge_count} edges, {files_watched} files"
            );
        }
        EventPayload::Violation {
            policy_id,
            file,
            message,
            ..
        } => {
            println!("{prefix} [{policy_id}] {file}: {message}");
        }
        EventPayload::Error(err) => {
            eprintln!("{prefix} Error: {}", err.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: WatchArgs,
    }

    #[test]
    fn args_parses_empty() {
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_source() {
        let w = Wrapper::try_parse_from(["test", "--source"]).unwrap();
        assert!(w.inner.source);
    }

    #[test]
    fn args_parses_all() {
        let w = Wrapper::try_parse_from(["test", "--all"]).unwrap();
        assert!(w.inner.all);
    }

    #[test]
    fn args_parses_patterns() {
        let w = Wrapper::try_parse_from(["test", "--patterns", "**/*.ts,**/*.tsx"]).unwrap();
        assert!(w.inner.patterns.is_some());
    }

    #[test]
    fn args_parses_file_and_action() {
        let w = Wrapper::try_parse_from([
            "test",
            "--file",
            "src/",
            "--action",
            "gate",
            "--exclude",
            "vendor/",
        ])
        .unwrap();
        assert_eq!(w.inner.file.as_deref(), Some("src/"));
        assert_eq!(w.inner.action.as_deref(), Some("gate"));
        assert_eq!(w.inner.exclude.as_deref(), Some("vendor/"));
    }

    #[test]
    fn validate_action_accepts_gate_and_check() {
        assert!(validate_action(Some("gate")).is_ok());
        assert!(validate_action(Some("check")).is_ok());
        assert!(validate_action(None).is_ok());
    }

    #[test]
    fn validate_action_rejects_unknown() {
        assert!(validate_action(Some("deploy")).is_err());
    }

    #[test]
    fn resolve_watch_root_uses_workspace_for_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(
            resolve_watch_root(tmp.path(), None).unwrap(),
            tmp.path()
                .canonicalize()
                .unwrap_or_else(|_| tmp.path().to_path_buf())
        );
    }

    #[test]
    fn resolve_watch_root_joins_relative() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let result = resolve_watch_root(tmp.path(), Some("src")).unwrap();
        assert_eq!(result, src_dir.canonicalize().unwrap());
    }

    #[test]
    fn resolve_watch_root_file_uses_parent() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        let result = resolve_watch_root(tmp.path(), Some("main.rs")).unwrap();
        assert_eq!(result, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn resolve_watch_root_rejects_path_traversal() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_watch_root(tmp.path(), Some("../../etc"));
        assert!(result.is_err());
    }

    #[test]
    fn build_filter_includes_extra_excludes() {
        let filter = build_filter(&["vendor".to_string(), "tmp".to_string()]);
        assert!(filter.should_ignore(std::path::Path::new("vendor/lib.ts")));
        assert!(filter.should_ignore(std::path::Path::new("tmp/scratch.ts")));
        // Default excludes still work
        assert!(filter.should_ignore(std::path::Path::new("node_modules/x.ts")));
    }

    #[test]
    fn build_filter_dedup_existing_default() {
        let filter = build_filter(&["node_modules".to_string()]);
        // Should still work — no duplicate panic or double-entry
        assert!(filter.should_ignore(std::path::Path::new("node_modules/x.ts")));
    }

    #[test]
    fn resolve_watch_root_rejects_nonexistent_traversal() {
        // Even when the target doesn't exist on disk, the ancestor-based
        // canonicalisation should still catch .. traversal
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_watch_root(tmp.path(), Some("../nonexistent-dir"));
        assert!(result.is_err());
    }

    #[test]
    fn normalise_path_via_ancestors_success_non_existent_target() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        std::fs::create_dir_all(base.join("a/b")).unwrap();
        // Path traverses up from a/b then into non-existent "c"
        let input = base.join("a/b/../../c");
        let result = normalise_path_via_ancestors(&input);
        let expected = base.join("c");
        assert_eq!(result, expected);
    }

    #[test]
    fn build_action_command_sets_correct_args() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        let cmd = build_action_command(&exe, "gate", &ws, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(args, vec![std::ffi::OsStr::new("gate")]);

        let cmd = build_action_command(&exe, "check", &ws, true, true);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(
            args,
            vec![
                std::ffi::OsStr::new("check"),
                std::ffi::OsStr::new("--json"),
                std::ffi::OsStr::new("--no-tui"),
            ]
        );
    }

    #[test]
    fn build_action_command_sets_cwd() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/my/project");
        let cmd = build_action_command(&exe, "gate", &ws, false, false);
        assert_eq!(
            cmd.get_current_dir(),
            Some(std::path::Path::new("/my/project"))
        );
    }

    #[test]
    fn build_filter_sanitises_deep_paths() {
        let filter = build_filter(&["apps/foo/vendor".to_string()]);
        assert!(filter.should_ignore(std::path::Path::new("vendor/lib.ts")));
        // Full deep path also matches because FileFilter checks each component
        assert!(filter.should_ignore(std::path::Path::new("apps/foo/vendor/lib.ts")));
    }

    // The concurrency guard (action_running/action_pending AtomicBool pair) is
    // tested indirectly via the watch integration flow. Direct unit testing is
    // impractical because the guard is coupled to thread spawning and the
    // dispatch loop in run(). The previous test here only exercised
    // AtomicBool::swap semantics in isolation, which is tautological.

    // --- normalise_path_via_ancestors ---

    #[test]
    fn normalise_path_resolves_dotdot_traversal() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let sub = base.join("a").join("b");
        std::fs::create_dir_all(&sub).unwrap();

        // a/b/../../c should resolve to <tmp>/c
        let input = sub.join("..").join("..").join("c");
        let result = normalise_path_via_ancestors(&input);
        assert_eq!(result, base.join("c"));
    }

    #[test]
    fn normalise_path_handles_absolute_existing_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let file = base.join("exists.txt");
        std::fs::write(&file, "").unwrap();

        let result = normalise_path_via_ancestors(&file);
        assert_eq!(result, file.canonicalize().unwrap());
    }

    #[test]
    fn normalise_path_handles_relative_components() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let sub = base.join("deep");
        std::fs::create_dir(&sub).unwrap();

        // deep/../shallow should resolve to <tmp>/shallow
        let input = sub.join("..").join("shallow");
        let result = normalise_path_via_ancestors(&input);
        assert_eq!(result, base.join("shallow"));
    }

    // --- WatchEvent serialisation ---

    #[test]
    fn watch_event_serialises_to_json() {
        let event = WatchEvent {
            timestamp: "2026-04-01T00:00:00Z".to_string(),
            event_type: "Snapshot".to_string(),
            detail: "10 nodes".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["timestamp"], "2026-04-01T00:00:00Z");
        assert_eq!(parsed["event_type"], "Snapshot");
        assert_eq!(parsed["detail"], "10 nodes");
    }

    #[test]
    fn watch_event_uses_snake_case_keys() {
        let event = WatchEvent {
            timestamp: "t".to_string(),
            event_type: "Progress".to_string(),
            detail: "d".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_type\""));
        assert!(!json.contains("\"eventType\""));
    }

    // --- build_filter with empty excludes ---

    #[test]
    fn build_filter_empty_excludes_returns_default() {
        let filter = build_filter(&[]);
        // Default filter should still ignore standard dirs like node_modules
        assert!(filter.should_ignore(std::path::Path::new("node_modules/x.ts")));
        // But not arbitrary dirs
        assert!(!filter.should_ignore(std::path::Path::new("src/main.rs")));
    }

    // --- Pattern selection logic ---

    fn collect_patterns(args: &[&str]) -> Vec<String> {
        let w = Wrapper::try_parse_from(args).unwrap();
        if let Some(ref p) = w.inner.patterns {
            p.split(',').map(|s| s.trim().to_string()).collect()
        } else if w.inner.all || (w.inner.source && w.inner.plans) {
            DEFAULT_WATCH_PATTERNS
                .iter()
                .chain(SOURCE_PATTERNS.iter())
                .map(ToString::to_string)
                .collect()
        } else if w.inner.source {
            SOURCE_PATTERNS.iter().map(ToString::to_string).collect()
        } else {
            DEFAULT_WATCH_PATTERNS
                .iter()
                .map(ToString::to_string)
                .collect()
        }
    }

    #[test]
    fn pattern_selection_source_picks_source_patterns() {
        let patterns = collect_patterns(&["test", "--source"]);
        let expected: Vec<String> = SOURCE_PATTERNS.iter().map(ToString::to_string).collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_all_picks_both() {
        let patterns = collect_patterns(&["test", "--all"]);
        let expected: Vec<String> = DEFAULT_WATCH_PATTERNS
            .iter()
            .chain(SOURCE_PATTERNS.iter())
            .map(ToString::to_string)
            .collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_source_and_plans_picks_both() {
        let patterns = collect_patterns(&["test", "--source", "--plans"]);
        let expected: Vec<String> = DEFAULT_WATCH_PATTERNS
            .iter()
            .chain(SOURCE_PATTERNS.iter())
            .map(ToString::to_string)
            .collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_default_picks_default_watch_patterns() {
        let patterns = collect_patterns(&["test"]);
        let expected: Vec<String> = DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_plans_alone_picks_default() {
        let patterns = collect_patterns(&["test", "--plans"]);
        let expected: Vec<String> = DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(patterns, expected);
    }
}
