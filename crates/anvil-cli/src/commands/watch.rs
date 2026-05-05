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

    /// Glob patterns to watch (comma-separated, e.g. "src/**/*.ts,lib/**/*.ts").
    /// Empty = watch everything that passes the built-in denylist.
    #[arg(long)]
    patterns: Option<String>,

    /// Glob patterns to exclude (comma-separated, e.g. "vendor/**,**/*.test.ts").
    /// Bare directory names like "vendor" only match the directory itself —
    /// use "vendor/**" to exclude its contents.
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

// FileFilter owns the hardcoded internal denylist (node_modules, .git, …).
// User --patterns / --exclude are glob filters applied separately by the
// kernel's WatchPatternFilter — they no longer extend FileFilter.

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

/// The internal `FileFilter` owns the hardcoded denylist and the
/// parseable-extension gate. When the user has supplied their own scoping
/// criterion (e.g. `--patterns '**/*.rs'`), the parseable gate must yield
/// — otherwise events for non-JS files are dropped before the user's
/// pattern matcher ever sees them.
fn build_filter(user_supplied_patterns: bool) -> anvil_kernel::watcher::filter::FileFilter {
    anvil_kernel::watcher::filter::FileFilter::default()
        .with_respect_extensions(!user_supplied_patterns)
}

/// `--exclude` switched from "directory names" to glob patterns in
/// LAUNCH-001. A user who previously ran `--exclude vendor` will now
/// find their vendor tree silently watched, because the bare name
/// matches only a path equal to "vendor". Detect that shape at parse
/// time and warn with the corrected form.
///
/// Routes through stderr in `--json` mode so the JSON-lines event stream
/// on stdout stays parseable; otherwise stdout, alongside the rest of the
/// watch surface.
fn warn_on_bare_exclude_patterns(patterns: &[String], json_mode: bool) {
    for pattern in patterns {
        if is_likely_bare_directory_name(pattern) {
            // ASCII-only so it renders cleanly on Windows terminals that
            // are not configured for full Unicode (cmd.exe with a legacy
            // code page, log capture pipelines, dumb TERM environments).
            let line = format!(
                "[warn] --exclude {pattern} matches only a path named exactly \"{pattern}\"; \
                 to exclude its contents use --exclude {pattern}/**"
            );
            if json_mode {
                eprintln!("{line}");
            } else {
                println!("{line}");
            }
        }
    }
}

/// A pattern is a "likely bare directory name" if it contains no glob
/// metacharacters and no path separator — i.e. exactly the shape that
/// the previous denylist-based `--exclude` accepted. Empty strings and
/// patterns that look like glob expressions are not warned on.
fn is_likely_bare_directory_name(pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    !pattern.contains(['/', '\\', '*', '?', '[', '{', '!'])
}

/// Print the active include/exclude scope so a viewer can see the LAUNCH-001
/// glob filter is doing something. Silent in JSON mode (where structured
/// telemetry is the canonical channel) and TUI mode (rendered separately).
fn print_active_scope(include: &[String], exclude: &[String], global: &GlobalArgs) {
    if global.json {
        return;
    }
    let in_tui = !global.no_tui && std::io::stdout().is_terminal();
    if in_tui {
        return;
    }
    // ASCII-only so it renders cleanly on Windows terminals without full
    // Unicode support; the watch banner is the first thing a piped or
    // recorded session captures and emoji mojibake at that exact moment
    // is the kind of papercut a hype-builder demo can't afford.
    if include.is_empty() {
        println!("[watching] everything (denylist still applies)");
    } else {
        println!("[watching] {}", include.join(", "));
    }
    if !exclude.is_empty() {
        println!("[excluding] {}", exclude.join(", "));
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
///
/// `tui_parent` (LAUNCH-002): when true, the parent is in TUI mode. The child
/// receives `--no-tui` regardless of the parent's `--no-tui` flag (otherwise
/// two Ratatui sessions would fight over the same alternate-screen), and
/// stdout/stderr are routed to `Stdio::null()` so child writes cannot
/// corrupt the parent's render. We deliberately use `null()` not `piped()`:
/// piped pipes that nobody reads will block the child once the OS pipe
/// buffer fills (~64 KiB on Linux), which would deadlock long-running gates.
fn build_action_command(
    exe: &std::path::Path,
    action: &str,
    workspace_root: &std::path::Path,
    json: bool,
    no_tui: bool,
    tui_parent: bool,
) -> std::process::Command {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg(action);
    if json {
        cmd.arg("--json");
    }
    if no_tui || tui_parent {
        cmd.arg("--no-tui");
    }
    cmd.current_dir(workspace_root);
    if tui_parent {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
    } else if json {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::inherit());
    } else {
        cmd.stdout(std::process::Stdio::inherit());
        cmd.stderr(std::process::Stdio::inherit());
    }
    cmd
}

/// Action dispatcher (LAUNCH-002).
///
/// Owns the worker thread, the in-flight `Child`, the rerun atomics, and a
/// cancellation flag. `Drop` cancels, kills any in-flight child, and joins
/// the worker — fixing a pre-existing leak where the previous fire-and-forget
/// `thread::spawn` worker held child stdio descriptors and rerun atomics
/// across the parent's exit.
///
/// Both watch.rs branches (TUI and non-TUI) use this. In TUI mode, a
/// `SyncSender<ActionResultLine>` is provided and child stdio is discarded
/// (the parent owns the alt-screen). In non-TUI mode, the sender is `None`
/// and child stdio is inherited (bit-for-bit identical to the previous
/// behaviour).
pub(crate) struct ActionDispatcher(std::sync::Arc<DispatcherInner>);

/// Bundle the TUI-side action plumbing so signatures don't grow two
/// `Option<…>` parameters in lockstep.
pub(crate) struct WatchActionLink<'a> {
    pub action_rx: &'a std::sync::mpsc::Receiver<anvil_tui::surfaces::watch::ActionResultLine>,
    pub dispatcher: &'a ActionDispatcher,
}

struct DispatcherInner {
    action: String,
    workspace_root: PathBuf,
    json: bool,
    no_tui_arg: bool,
    /// Parent is in TUI mode → force `--no-tui` on the child and discard
    /// child stdio. See `build_action_command` for the rationale.
    tui_parent: bool,
    sender: Option<std::sync::mpsc::SyncSender<anvil_tui::surfaces::watch::ActionResultLine>>,
    running: AtomicBool,
    pending: AtomicBool,
    cancel: AtomicBool,
    /// In-flight child process. Held in a mutex so `shutdown()` can kill
    /// it from another thread while the worker is polling `try_wait()`.
    in_flight: std::sync::Mutex<Option<std::process::Child>>,
    worker: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    /// Test-only override for `current_exe()`. None in production.
    exe_override: Option<PathBuf>,
}

impl ActionDispatcher {
    fn new(
        action: String,
        workspace_root: PathBuf,
        json: bool,
        no_tui_arg: bool,
        tui_parent: bool,
        sender: Option<std::sync::mpsc::SyncSender<anvil_tui::surfaces::watch::ActionResultLine>>,
    ) -> Self {
        Self::new_with_exe(
            action,
            workspace_root,
            json,
            no_tui_arg,
            tui_parent,
            sender,
            None,
        )
    }

    fn new_with_exe(
        action: String,
        workspace_root: PathBuf,
        json: bool,
        no_tui_arg: bool,
        tui_parent: bool,
        sender: Option<std::sync::mpsc::SyncSender<anvil_tui::surfaces::watch::ActionResultLine>>,
        exe_override: Option<PathBuf>,
    ) -> Self {
        Self(std::sync::Arc::new(DispatcherInner {
            action,
            workspace_root,
            json,
            no_tui_arg,
            tui_parent,
            sender,
            running: AtomicBool::new(false),
            pending: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            in_flight: std::sync::Mutex::new(None),
            worker: std::sync::Mutex::new(None),
            exe_override,
        }))
    }

    /// Trigger a dispatch (or mark a pending rerun if one is in flight).
    /// Called from the watch loop on each post-initial Snapshot event.
    pub(crate) fn on_snapshot(&self) {
        if self.0.running.swap(true, Ordering::SeqCst) {
            self.0.pending.store(true, Ordering::SeqCst);
            return;
        }
        let inner = std::sync::Arc::clone(&self.0);
        let handle = std::thread::spawn(move || {
            loop {
                inner.run_one_action();
                if inner.cancel.load(Ordering::SeqCst)
                    || !inner.pending.swap(false, Ordering::SeqCst)
                {
                    break;
                }
            }
            inner.running.store(false, Ordering::SeqCst);
        });
        // Replace any prior (already-completed) handle. The previous handle
        // is dropped without joining; that's safe because `running` was just
        // false, which means the previous worker had already returned.
        let mut slot = self.0.worker.lock().expect("dispatcher worker mutex");
        let _ = slot.replace(handle);
    }

    /// Cancel any in-flight action and join the worker. Idempotent.
    fn shutdown(&self) {
        self.0.cancel.store(true, Ordering::SeqCst);
        // Don't try to coalesce more reruns after a cancel — the worker
        // checks `cancel` before re-iterating.
        self.0.pending.store(false, Ordering::SeqCst);
        if let Ok(mut slot) = self.0.in_flight.lock()
            && let Some(mut child) = slot.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        let handle_opt = self.0.worker.lock().ok().and_then(|mut g| g.take());
        if let Some(handle) = handle_opt {
            let _ = handle.join();
        }
    }
}

impl Drop for ActionDispatcher {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl DispatcherInner {
    fn run_one_action(&self) {
        let exe = if let Some(p) = self.exe_override.as_ref() {
            p.clone()
        } else {
            match std::env::current_exe() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[error] Cannot resolve current executable: {e}");
                    return;
                }
            }
        };
        let mut cmd = build_action_command(
            &exe,
            &self.action,
            &self.workspace_root,
            self.json,
            self.no_tui_arg,
            self.tui_parent,
        );

        let start = std::time::Instant::now();
        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[error] Failed to run action '{}': {e}", self.action);
                self.maybe_send(None, start.elapsed());
                return;
            }
        };

        // Park the child so shutdown() can kill it from another thread.
        if let Ok(mut slot) = self.in_flight.lock() {
            *slot = Some(child);
        }

        // Poll for completion or cancellation. 50 ms keeps the cancel
        // latency tight without busy-spinning on a long-running gate.
        let exit_code = self.wait_for_completion();
        if !self.tui_parent && exit_code.is_some_and(|c| c != 0) {
            // Preserve the existing user-facing warning on the non-TUI
            // path. In TUI mode the result surfaces via the footer line
            // instead, so no stderr write.
            eprintln!(
                "[warn] Action '{}' exited with code {}",
                self.action,
                exit_code.unwrap_or(-1)
            );
        }
        self.maybe_send(exit_code, start.elapsed());
    }

    fn wait_for_completion(&self) -> Option<i32> {
        loop {
            // Briefly take the slot. If shutdown took the child, we're done.
            let mut slot = self.in_flight.lock().ok()?;
            let child = slot.as_mut()?;
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code();
                    slot.take();
                    return code;
                }
                Ok(None) => {
                    drop(slot);
                    if self.cancel.load(Ordering::SeqCst) {
                        if let Ok(mut g) = self.in_flight.lock()
                            && let Some(mut child) = g.take()
                        {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => {
                    slot.take();
                    return None;
                }
            }
        }
    }

    fn maybe_send(&self, exit_code: Option<i32>, elapsed: std::time::Duration) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };
        let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let line = anvil_tui::surfaces::watch::ActionResultLine {
            action: self.action.clone(),
            exit_code,
            duration_ms,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        };
        // SyncSender::send blocks if the buffer is full — that's the
        // intended back-pressure when the TUI is not draining.
        // try_send + drop would lose the most recent result, which is
        // exactly the value we want to surface; block instead.
        let _ = sender.send(line);
    }
}

#[allow(clippy::too_many_lines)]
pub fn run(args: &WatchArgs, global: &GlobalArgs) -> Result<()> {
    let workspace_root = crate::util::workspace_root()?;
    let action = validate_action(args.action.as_deref())?;

    // LAUNCH-002: --action is now allowed in TUI mode. The dispatcher forces
    // --no-tui on the child and discards child stdio so two Ratatui sessions
    // can't fight over the same alternate-screen.

    // Resolve watch root — if --file is given, scope to that path
    let watch_root = resolve_watch_root(&workspace_root, args.file.as_deref())?;

    // Build include patterns passed to the kernel's WatchPatternFilter.
    //
    // The defaults assume the user wants the broadest reasonable scope
    // unless they opt into a narrower one:
    //   no flags            → empty (let the FileFilter denylist define scope)
    //   --all               → empty (same — FileFilter is the only gate)
    //   --plans only        → DEFAULT_WATCH_PATTERNS (planning docs)
    //   --source only       → SOURCE_PATTERNS (parseable sources)
    //   --plans + --source  → both
    //   --patterns "..."    → use those verbatim
    //
    // --all is checked first so it stays "watch everything" even when
    // combined with the narrower flags — without this, `--all --plans`
    // would silently scope to planning docs.
    //
    // Previously the no-flag and bare --plans cases both sent
    // DEFAULT_WATCH_PATTERNS, which silently restricted `anvil watch`
    // to planning docs and dropped every source-file event before it
    // ever reached the policy engine.
    let patterns: Vec<String> = if args.all {
        Vec::new()
    } else if let Some(ref p) = args.patterns {
        p.split(',').map(|s| s.trim().to_string()).collect()
    } else if args.source && args.plans {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .chain(SOURCE_PATTERNS.iter())
            .map(ToString::to_string)
            .collect()
    } else if args.source {
        SOURCE_PATTERNS.iter().map(ToString::to_string).collect()
    } else if args.plans {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect()
    } else {
        Vec::new()
    };

    // Exclude globs are applied by the kernel's WatchPatternFilter — they
    // no longer extend the internal FileFilter denylist. The internal
    // denylist (node_modules, .git, target, …) stays in place via
    // build_filter(); user-supplied excludes are passed through as
    // WatchConfig.exclude_patterns below.
    let exclude: Vec<String> = args.exclude.as_ref().map_or_else(Vec::new, |s| {
        s.split(',').map(|s| s.trim().to_string()).collect()
    });
    warn_on_bare_exclude_patterns(&exclude, global.json);
    // When the user has supplied an explicit scoped pattern (--patterns,
    // --source, --plans), the FileFilter must not additionally enforce
    // its hardcoded ts/js extension gate — that would silently drop events
    // for file types the user explicitly asked us to watch.
    //
    // `--all` is deliberately *not* in this set: it widens scope to "watch
    // everything that passes the denylist", but the kernel's parser still
    // only handles TS/JS today, so forwarding non-JS files to it produces
    // UnsupportedLanguage errors and noisy snapshots. Keep the extension
    // gate enabled for `--all` until the kernel supports more languages.
    let user_supplied_patterns = args.patterns.is_some() || args.source || args.plans;
    let filter = build_filter(user_supplied_patterns);

    print_active_scope(&patterns, &exclude, global);

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

    let non_tui = global.json || !std::io::stdout().is_terminal() || global.no_tui;

    // LAUNCH-002: in TUI mode, the dispatcher emits ActionResultLine records
    // through a sync_channel(1) into the watch loop. The bound is intentional
    // back-pressure: if the TUI hasn't drained the most recent result, the
    // worker blocks on `send` until it does, naturally rate-limiting reruns.
    let (action_tx, action_rx) = if action.is_some() && !non_tui {
        let (tx, rx) = mpsc::sync_channel::<anvil_tui::surfaces::watch::ActionResultLine>(1);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let dispatcher = action.map(|act| {
        ActionDispatcher::new(
            act.to_string(),
            workspace_root.clone(),
            global.json,
            global.no_tui,
            !non_tui,
            action_tx,
        )
    });

    if non_tui {
        let mut snapshot_count: u64 = 0;

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

                    // Dispatch action on snapshot events (skip initial scan).
                    // Concurrency / rerun guarding lives in ActionDispatcher.
                    if let Some(d) = dispatcher.as_ref()
                        && matches!(event.event_type, anvil_kernel_types::EventType::Snapshot)
                    {
                        snapshot_count += 1;
                        if snapshot_count > 1 {
                            d.on_snapshot();
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
                last_action: None,
            });
        let link = action_rx
            .as_ref()
            .zip(dispatcher.as_ref())
            .map(|(rx, d)| WatchActionLink {
                action_rx: rx,
                dispatcher: d,
            });
        crate::tui::run_watch(state, &event_rx, link.as_ref(), Some(&shutdown))?;
    }

    // Dispatcher Drop cancels any in-flight action and joins the worker —
    // closes the pre-existing Ctrl-C leak.
    drop(dispatcher);
    handle.stop().context("stopping watcher")?;
    Ok(())
}

fn print_event_plain(event: &anvil_kernel_types::EngineEvent) {
    use anvil_kernel_types::{EventPayload, EventType};

    // ASCII-only labels so per-event watch output renders cleanly on
    // Windows terminals and CI log captures that lack full Unicode. The
    // banner and bare-exclude warning were previously fixed; this is the
    // hot path during a demo and was missed in that round.
    let prefix = match event.event_type {
        EventType::Progress => "[progress]",
        EventType::Snapshot => "[snapshot]",
        EventType::Violation => "[violation]",
        EventType::Error => "[error]",
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

    // The previous tests that exercised --exclude extending the internal
    // FileFilter denylist were removed in LAUNCH-001: --exclude is now a
    // user-glob path, applied by the kernel's WatchPatternFilter, and no
    // longer touches the internal denylist. Coverage moved to
    // crates/anvil-kernel/src/watcher/pattern.rs (unit) and
    // crates/anvil-kernel/tests/watch_pattern_filter.rs (integration).

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

        let cmd = build_action_command(&exe, "gate", &ws, false, false, false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert_eq!(args, vec![std::ffi::OsStr::new("gate")]);

        let cmd = build_action_command(&exe, "check", &ws, true, true, false);
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
        let cmd = build_action_command(&exe, "gate", &ws, false, false, false);
        assert_eq!(
            cmd.get_current_dir(),
            Some(std::path::Path::new("/my/project"))
        );
    }

    // --- LAUNCH-002: --no-tui propagation in TUI parent context ---

    #[test]
    fn tui_parent_forces_no_tui_on_child_even_without_parent_flag() {
        // The foot-gun the original guard was hiding: with the parent in TUI
        // mode and no `--no-tui` flag set, a naive guard-drop would let the
        // child enter its own Ratatui alt-screen and fight the parent.
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        let cmd = build_action_command(
            &exe, "gate", &ws, false, /* no_tui */ false, /* tui_parent */ true,
        );
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(
            args.iter().any(|a| *a == std::ffi::OsStr::new("--no-tui")),
            "child must receive --no-tui when parent is in TUI mode, got {args:?}"
        );
    }

    #[test]
    fn tui_parent_does_not_duplicate_no_tui_when_parent_flag_also_set() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        let cmd = build_action_command(
            &exe, "gate", &ws, false, /* no_tui */ true, /* tui_parent */ true,
        );
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        let count = args
            .iter()
            .filter(|a| **a == std::ffi::OsStr::new("--no-tui"))
            .count();
        assert_eq!(
            count, 1,
            "--no-tui should appear exactly once, got {args:?}"
        );
    }

    #[test]
    fn non_tui_parent_does_not_force_no_tui() {
        let exe = PathBuf::from("/usr/bin/anvil");
        let ws = PathBuf::from("/project");

        let cmd = build_action_command(
            &exe, "gate", &ws, false, /* no_tui */ false, /* tui_parent */ false,
        );
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(
            !args.iter().any(|a| *a == std::ffi::OsStr::new("--no-tui")),
            "non-TUI parent without explicit --no-tui must not force it on child, got {args:?}"
        );
    }

    // --- LAUNCH-002: ActionDispatcher shutdown ---

    /// Spawns a real `/bin/sleep 30` child via the dispatcher's exe override,
    /// then calls `shutdown()` and asserts the worker joins promptly. Closes
    /// the pre-existing leak where Ctrl-C orphaned the dispatch worker.
    /// Unix-only because the test depends on `/bin/sleep`; Windows lacks an
    /// equivalent at a stable path.
    #[cfg(unix)]
    #[test]
    fn shutdown_kills_in_flight_child_and_joins_worker() {
        let dispatcher = ActionDispatcher::new_with_exe(
            "30".to_string(), // sleep 30 seconds
            PathBuf::from("/tmp"),
            false,
            false,
            false,
            None,
            Some(PathBuf::from("/bin/sleep")),
        );

        dispatcher.on_snapshot();

        // Wait briefly for the worker to spawn /bin/sleep and park it in
        // the in_flight slot. 250 ms is generous; the child usually appears
        // within a few ms.
        let parked = std::time::Instant::now();
        loop {
            if dispatcher
                .0
                .in_flight
                .lock()
                .ok()
                .is_some_and(|g| g.is_some())
            {
                break;
            }
            assert!(
                parked.elapsed() <= std::time::Duration::from_millis(500),
                "child did not park in in_flight slot within 500 ms"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let shutdown_started = std::time::Instant::now();
        dispatcher.shutdown();
        let shutdown_took = shutdown_started.elapsed();

        // /bin/sleep 30 would not have exited naturally. If shutdown
        // returned, the child was killed and the worker joined. The kill
        // path includes a 50 ms poll grace; allow up to 1 s for slow CI.
        assert!(
            shutdown_took < std::time::Duration::from_secs(1),
            "shutdown took {shutdown_took:?}; expected < 1 s — \
             worker did not join promptly, child may have leaked"
        );

        // No child remains in the slot.
        assert!(
            dispatcher
                .0
                .in_flight
                .lock()
                .ok()
                .is_some_and(|g| g.is_none()),
            "in_flight slot should be empty after shutdown"
        );

        // Idempotent: a second shutdown is a no-op.
        dispatcher.shutdown();
    }

    /// `Drop` must call `shutdown` so a panic or early-return path doesn't
    /// leak the worker.
    #[cfg(unix)]
    #[test]
    fn drop_invokes_shutdown() {
        let parked = {
            let dispatcher = ActionDispatcher::new_with_exe(
                "30".to_string(),
                PathBuf::from("/tmp"),
                false,
                false,
                false,
                None,
                Some(PathBuf::from("/bin/sleep")),
            );
            dispatcher.on_snapshot();

            // Wait for the child to park.
            let waited = std::time::Instant::now();
            loop {
                if dispatcher
                    .0
                    .in_flight
                    .lock()
                    .ok()
                    .is_some_and(|g| g.is_some())
                {
                    break;
                }
                assert!(
                    waited.elapsed() <= std::time::Duration::from_millis(500),
                    "child did not park within 500 ms"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            std::time::Instant::now()
            // dispatcher dropped here; Drop -> shutdown -> kill+join
        };

        assert!(
            parked.elapsed() < std::time::Duration::from_secs(1),
            "Drop should kill child and join worker within 1 s"
        );
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

    // --- bare-exclude warning heuristic (M4) ---

    #[test]
    fn bare_directory_name_is_detected() {
        assert!(is_likely_bare_directory_name("vendor"));
        assert!(is_likely_bare_directory_name("tmp"));
        assert!(is_likely_bare_directory_name("node_modules"));
    }

    #[test]
    fn glob_patterns_are_not_treated_as_bare_names() {
        assert!(!is_likely_bare_directory_name("vendor/**"));
        assert!(!is_likely_bare_directory_name("**/*.test.ts"));
        assert!(!is_likely_bare_directory_name("src/foo"));
        assert!(!is_likely_bare_directory_name("*.log"));
        assert!(!is_likely_bare_directory_name("file?.ts"));
        assert!(!is_likely_bare_directory_name("[abc]/lib"));
        assert!(!is_likely_bare_directory_name("{a,b}/lib"));
        assert!(!is_likely_bare_directory_name("!skip"));
    }

    #[test]
    fn empty_string_does_not_trigger_bare_warning() {
        assert!(!is_likely_bare_directory_name(""));
    }

    // --- build_filter ---

    #[test]
    fn build_filter_returns_default_denylist() {
        let filter = build_filter(false);
        // Default filter still ignores standard dirs like node_modules
        assert!(filter.should_ignore(std::path::Path::new("node_modules/x.ts")));
        // But not arbitrary dirs
        assert!(!filter.should_ignore(std::path::Path::new("src/main.rs")));
    }

    #[test]
    fn build_filter_with_user_patterns_bypasses_extension_gate() {
        // The demo-killer regression: --patterns '**/*.rs' must not be
        // dropped by the FileFilter's hardcoded ts/js list before the
        // user's WatchPatternFilter ever sees the event.
        let filter = build_filter(true);
        assert!(filter.should_process(std::path::Path::new("src/main.rs")));
        assert!(filter.should_process(std::path::Path::new("lib.py")));
        // Denylist still applies.
        assert!(!filter.should_process(std::path::Path::new("node_modules/foo.rs")));
    }

    // --- Pattern selection logic ---
    //
    // The helper mirrors the include-pattern computation in `run()`.
    // Keep them in sync — a test-local duplicate that drifts from the
    // production logic was the gap that let the M2 default-pattern bug
    // ship in the original LAUNCH-001 commit.

    fn collect_patterns(args: &[&str]) -> Vec<String> {
        let w = Wrapper::try_parse_from(args).unwrap();
        if w.inner.all {
            Vec::new()
        } else if let Some(ref p) = w.inner.patterns {
            p.split(',').map(|s| s.trim().to_string()).collect()
        } else if w.inner.source && w.inner.plans {
            DEFAULT_WATCH_PATTERNS
                .iter()
                .chain(SOURCE_PATTERNS.iter())
                .map(ToString::to_string)
                .collect()
        } else if w.inner.source {
            SOURCE_PATTERNS.iter().map(ToString::to_string).collect()
        } else if w.inner.plans {
            DEFAULT_WATCH_PATTERNS
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            Vec::new()
        }
    }

    #[test]
    fn pattern_selection_source_picks_source_patterns() {
        let patterns = collect_patterns(&["test", "--source"]);
        let expected: Vec<String> = SOURCE_PATTERNS.iter().map(ToString::to_string).collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_all_returns_empty_for_broadest_scope() {
        // --all delegates scope to the FileFilter denylist; the kernel
        // pattern filter is intentionally noop.
        let patterns = collect_patterns(&["test", "--all"]);
        assert!(
            patterns.is_empty(),
            "--all should send empty include_patterns, got {patterns:?}"
        );
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
    fn pattern_selection_default_returns_empty_for_broadest_scope() {
        // No flags = let the FileFilter denylist define scope; do not
        // silently restrict to plan files (the M2 regression).
        let patterns = collect_patterns(&["test"]);
        assert!(
            patterns.is_empty(),
            "no flags should send empty include_patterns, got {patterns:?}"
        );
    }

    #[test]
    fn pattern_selection_plans_alone_picks_plan_patterns() {
        // Bare --plans is now opt-in narrowing, not the default.
        let patterns = collect_patterns(&["test", "--plans"]);
        let expected: Vec<String> = DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(patterns, expected);
    }

    #[test]
    fn pattern_selection_all_overrides_narrower_flags() {
        // --all is "watch everything" — combining it with --plans,
        // --source, or --patterns must not silently narrow scope.
        for combo in [
            vec!["test", "--all", "--plans"],
            vec!["test", "--all", "--source"],
            vec!["test", "--all", "--plans", "--source"],
            vec!["test", "--all", "--patterns", "src/**/*.ts"],
        ] {
            let patterns = collect_patterns(&combo);
            assert!(
                patterns.is_empty(),
                "{combo:?} should keep --all's broad scope, got {patterns:?}"
            );
        }
    }
}
