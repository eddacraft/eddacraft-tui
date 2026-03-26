pub mod debounce;
pub mod events;
pub mod filter;

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

/// Walk directories under `root`, adding a non-recursive watch for each
/// directory that isn't in the filter's ignore list. This avoids registering
/// inotify watches on `node_modules`, `.git`, `target`, etc.
fn watch_directories(
    watcher: &mut RecommendedWatcher,
    root: &std::path::Path,
    filter: &FileFilter,
) -> Result<(), WatcherError> {
    watcher.watch(root, RecursiveMode::NonRecursive)?;

    let walker = walkdir::WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() {
                return false;
            }
            !filter.should_ignore(e.path())
        });

    for entry in walker.flatten() {
        // flatten() skips permission errors, broken symlinks, etc.
        // The filter already excludes node_modules/target/.git so most
        // walk errors are from edge cases that don't affect monitoring.
        watcher.watch(entry.path(), RecursiveMode::NonRecursive)?;
    }

    Ok(())
}

/// Starts watching the given directory and sends `ChangeBatch` events
/// to the returned receiver. Runs until the watcher handle is dropped.
/// Handle returned by `start_watcher` — keeps the OS watcher alive.
/// Drop to stop watching.
pub struct WatcherHandle {
    _watcher: Arc<Mutex<RecommendedWatcher>>,
}

pub fn start_watcher(
    config: &WatcherConfig,
) -> Result<(WatcherHandle, mpsc::Receiver<ChangeBatch>), WatcherError> {
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
    watch_directories(&mut watcher, &config.root, &filter)?;

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
                        // files created inside them are picked up.
                        if kind == ChangeKind::Created
                            && path.is_dir()
                            && !thread_filter.should_ignore(&path)
                            && let Ok(mut w) = watcher_for_thread.lock()
                        {
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

    Ok((handle, batch_rx))
}
