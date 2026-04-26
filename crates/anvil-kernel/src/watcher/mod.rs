pub mod debounce;
pub mod events;
pub mod filter;
pub mod pattern;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use self::debounce::Debouncer;
use self::events::{ChangeBatch, ChangeKind, FileChange};
use self::filter::FileFilter;

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Root directory to watch.
    pub root: PathBuf,
    /// Debounce window for coalescing rapid changes.
    pub debounce_window: Duration,
    /// Maximum pending changes before backpressure flush.
    pub max_pending: usize,
    /// Tick interval for checking debounce expiry.
    pub tick_interval: Duration,
    /// File filter for ignore patterns. If None, uses default patterns.
    pub filter: Option<FileFilter>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            debounce_window: Duration::from_millis(50),
            max_pending: 500,
            tick_interval: Duration::from_millis(20),
            filter: None,
        }
    }
}

/// Error type for watcher operations.
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("channel receive error: {0}")]
    Recv(#[from] mpsc::RecvTimeoutError),
}

/// Diagnostics returned from [`watch_directories`] so callers can surface
/// partial failure — e.g. when the OS-level watch limit
/// (`fs.inotify.max_user_watches` on Linux) is reached partway through
/// registration and only some directories end up being monitored.
#[derive(Debug, Default, Clone)]
pub struct WatchSetupDiagnostics {
    /// Number of directories that registered a watch successfully.
    pub registered: u64,
    /// Number of directories that failed to register. A non-zero value
    /// means file changes in those subtrees won't be observed.
    pub failed: u64,
    /// A sample of the error messages from failed registrations (first few).
    pub sample_errors: Vec<String>,
    /// Whether the root-level watch itself failed (catastrophic — no events
    /// will flow at all).
    pub root_failed: bool,
    /// True if any error looked like an OS-level limit being hit (e.g.
    /// inotify `max_user_watches`). Used to surface an actionable hint.
    pub limit_exhausted: bool,
}

/// Walk directories under `root`, adding a non-recursive watch for each
/// directory that isn't in the filter's ignore list. This avoids registering
/// inotify watches on `node_modules`, `.git`, `target`, etc.
///
/// Failures on individual directories are counted and returned rather than
/// propagated — partial coverage is better than no watcher at all, but the
/// caller must surface a diagnostic so the user knows why changes in some
/// directories are missing.
///
/// Testing note: the partial-failure paths (mid-walk `watcher.watch()`
/// failure incrementing `diag.failed`, `MaxFilesWatch` setting
/// `limit_exhausted`, and the "watch partially registered" error event
/// emitted by `run_watch`) are not covered by automated tests because
/// they require a failing `notify::Watcher::watch()` call, which in
/// practice only happens when `fs.inotify.max_user_watches` is exhausted
/// on the host. Adding a `#[cfg(test)]` trait-object seam just to fake
/// this was judged not worth the indirection; these paths are exercised
/// manually when the limit is actually hit.
fn watch_directories(
    watcher: &mut RecommendedWatcher,
    root: &std::path::Path,
    filter: &FileFilter,
) -> Result<WatchSetupDiagnostics, WatcherError> {
    const MAX_SAMPLE_ERRORS: usize = 3;
    let mut diag = WatchSetupDiagnostics::default();

    match watcher.watch(root, RecursiveMode::NonRecursive) {
        Ok(()) => diag.registered += 1,
        Err(e) => {
            // Root-level failure is catastrophic — no subtree can compensate.
            // Propagate so the caller can fail fast with a clear error.
            diag.root_failed = true;
            return Err(WatcherError::from(e));
        }
    }

    // SCAN-001: gitignore-aware directory walk for inotify registration
    // (same shape as the welcome-screen / audit / drift discovery
    // surfaces). Watch registration itself stays serial because
    // `notify::Watcher::watch` mutates internal kernel-watch state on
    // each call; parallelising it has no reliable wall-time win.
    let filter_for_walker = filter.clone();
    let walker = ignore::WalkBuilder::new(root)
        .standard_filters(false)
        .hidden(false)
        .follow_links(false)
        .filter_entry(move |e| {
            if !e.file_type().is_some_and(|ft| ft.is_dir()) {
                return false;
            }
            !filter_for_walker.should_ignore(e.path())
        })
        .build();

    for entry in walker.filter_map(Result::ok).filter(|e| e.depth() >= 1) {
        // flatten() skips permission errors, broken symlinks, etc.
        // The filter already excludes node_modules/target/.git so most
        // walk errors are from edge cases that don't affect monitoring.
        match watcher.watch(entry.path(), RecursiveMode::NonRecursive) {
            Ok(()) => diag.registered += 1,
            Err(e) => {
                diag.failed += 1;
                if matches!(e.kind, notify::ErrorKind::MaxFilesWatch) {
                    diag.limit_exhausted = true;
                }
                if diag.sample_errors.len() < MAX_SAMPLE_ERRORS {
                    diag.sample_errors
                        .push(format!("{}: {e}", entry.path().display()));
                }
            }
        }
    }

    Ok(diag)
}

/// Handle returned by [`start_watcher`] — keeps the OS watcher alive.
/// The `Arc<Mutex>` is shared with the event-processing thread so it can
/// register newly created directories at runtime. Drop to stop watching.
pub struct WatcherHandle {
    _watcher: Arc<Mutex<RecommendedWatcher>>,
}

/// Starts watching the given directory and sends [`ChangeBatch`] events
/// to the returned receiver. Runs until the [`WatcherHandle`] is dropped.
///
/// The returned [`WatchSetupDiagnostics`] records how many directories were
/// successfully watched vs. failed. A non-zero `failed` count means file
/// changes in those subtrees will not generate events — most commonly caused
/// by hitting the OS-level watch limit (`fs.inotify.max_user_watches` on
/// Linux). Callers should surface this to the user so they can raise the
/// limit or close other watch-heavy processes.
pub fn start_watcher(
    config: &WatcherConfig,
) -> Result<
    (
        WatcherHandle,
        mpsc::Receiver<ChangeBatch>,
        WatchSetupDiagnostics,
    ),
    WatcherError,
> {
    let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<Event>>();
    let (batch_tx, batch_rx) = mpsc::channel::<ChangeBatch>();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = raw_tx.send(res);
        },
        notify::Config::default(),
    )?;

    // Watch directories selectively — skip ignored dirs (`node_modules`,
    // `.git`, `target`, etc.) at the OS level to avoid exhausting inotify
    // limits.
    let filter = config.filter.clone().unwrap_or_default();
    let diagnostics = watch_directories(&mut watcher, &config.root, &filter)?;

    // Wrap watcher so the processing thread can register new directories.
    let watcher_arc = Arc::new(Mutex::new(watcher));
    let watcher_for_thread = Arc::clone(&watcher_arc);

    let debounce_window = config.debounce_window;
    let max_pending = config.max_pending;
    let tick_interval = config.tick_interval;
    let thread_filter = filter.clone();

    std::thread::spawn(move || {
        let mut debouncer = Debouncer::new(debounce_window, max_pending);

        loop {
            match raw_rx.recv_timeout(tick_interval) {
                Ok(Ok(event)) => {
                    for path in event.paths {
                        let kind = match event.kind {
                            EventKind::Create(_) => ChangeKind::Created,
                            EventKind::Modify(_) => ChangeKind::Modified,
                            EventKind::Remove(_) => ChangeKind::Removed,
                            _ => continue,
                        };

                        // Register newly created directories for watching so
                        // files created inside them are picked up. Uses
                        // symlink_metadata to avoid following symlinks that
                        // could point outside the project root.
                        if kind == ChangeKind::Created
                            && path.symlink_metadata().is_ok_and(|m| m.is_dir())
                            && !thread_filter.should_ignore(&path)
                            && let Ok(mut w) = watcher_for_thread.lock()
                        {
                            // Silently discard watch errors — the directory may
                            // have been deleted between the Create event and now
                            // (race with rapid create/delete cycles). This is
                            // benign; the next create will retry.
                            let _ = w.watch(&path, RecursiveMode::NonRecursive);
                        }

                        // Skip files that don't pass the filter.
                        // Removed files always pass — we need to track deletions.
                        if kind != ChangeKind::Removed && !thread_filter.should_process(&path) {
                            continue;
                        }

                        if let Some(batch) = debouncer.record(FileChange { path, kind })
                            && batch_tx.send(batch).is_err()
                        {
                            return;
                        }
                    }
                }
                Ok(Err(_)) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check debounce expiry
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }

            if let Some(batch) = debouncer.tick()
                && batch_tx.send(batch).is_err()
            {
                return;
            }
        }
    });

    let handle = WatcherHandle {
        _watcher: watcher_arc,
    };

    Ok((handle, batch_rx, diagnostics))
}
