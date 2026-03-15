pub mod debounce;
pub mod events;
pub mod filter;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use self::debounce::Debouncer;
use self::events::{ChangeBatch, ChangeKind, FileChange};

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
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            debounce_window: Duration::from_millis(50),
            max_pending: 500,
            tick_interval: Duration::from_millis(20),
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

/// Starts watching the given directory and sends `ChangeBatch` events
/// to the returned receiver. Runs until the watcher handle is dropped.
pub fn start_watcher(
    config: &WatcherConfig,
) -> Result<(RecommendedWatcher, mpsc::Receiver<ChangeBatch>), WatcherError> {
    let (raw_tx, raw_rx) = mpsc::channel::<notify::Result<Event>>();
    let (batch_tx, batch_rx) = mpsc::channel::<ChangeBatch>();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = raw_tx.send(res);
        },
        notify::Config::default(),
    )?;

    watcher.watch(&config.root, RecursiveMode::Recursive)?;

    let debounce_window = config.debounce_window;
    let max_pending = config.max_pending;
    let tick_interval = config.tick_interval;

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
                        if let Some(batch) = debouncer.record(FileChange { path, kind }) {
                            if batch_tx.send(batch).is_err() {
                                return;
                            }
                        }
                    }
                }
                Ok(Err(_)) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Check debounce expiry
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }

            if let Some(batch) = debouncer.tick() {
                if batch_tx.send(batch).is_err() {
                    return;
                }
            }
        }
    });

    Ok((watcher, batch_rx))
}
