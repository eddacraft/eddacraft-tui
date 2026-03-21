use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::events::{ChangeBatch, ChangeKind, FileChange};

/// Coalesces rapid file changes within a configurable window.
///
/// Files changed multiple times within the debounce window are
/// collapsed into a single change. Backpressure is applied by
/// bounding the pending change map to `max_pending` entries --
/// if exceeded, the oldest batch is flushed immediately.
pub struct Debouncer {
    window: Duration,
    max_pending: usize,
    pending: HashMap<PathBuf, (ChangeKind, Instant)>,
}

impl Debouncer {
    pub fn new(window: Duration, max_pending: usize) -> Self {
        Self {
            window,
            max_pending,
            pending: HashMap::new(),
        }
    }

    /// Record a file change. Returns a batch if the debounce window
    /// has elapsed for any pending changes or backpressure triggers.
    pub fn record(&mut self, change: FileChange) -> Option<ChangeBatch> {
        let now = Instant::now();
        self.pending.insert(change.path, (change.kind, now));

        if self.pending.len() > self.max_pending {
            return Some(self.flush(now));
        }
        None
    }

    /// Check for changes whose debounce window has elapsed.
    /// Call this on a timer tick (e.g. every 10-50ms).
    pub fn tick(&mut self) -> Option<ChangeBatch> {
        let now = Instant::now();
        let ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (_, ts))| now.duration_since(*ts) >= self.window)
            .map(|(p, _)| p.clone())
            .collect();

        if ready.is_empty() {
            return None;
        }

        let changes: Vec<FileChange> = ready
            .into_iter()
            .filter_map(|p| {
                self.pending
                    .remove(&p)
                    .map(|(kind, _)| FileChange { path: p, kind })
            })
            .collect();

        Some(ChangeBatch {
            changes,
            received_at: now,
        })
    }

    /// Flush all pending changes immediately.
    pub fn flush(&mut self, now: Instant) -> ChangeBatch {
        let changes: Vec<FileChange> = self
            .pending
            .drain()
            .map(|(path, (kind, _))| FileChange { path, kind })
            .collect();
        ChangeBatch {
            changes,
            received_at: now,
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_rapid_changes_to_same_file() {
        let mut d = Debouncer::new(Duration::from_millis(50), 100);

        let path = PathBuf::from("src/main.rs");
        // Two rapid changes to the same file
        assert!(
            d.record(FileChange {
                path: path.clone(),
                kind: ChangeKind::Modified,
            })
            .is_none()
        );
        assert!(
            d.record(FileChange {
                path: path.clone(),
                kind: ChangeKind::Modified,
            })
            .is_none()
        );

        // Only one pending entry
        assert_eq!(d.pending_count(), 1);
    }

    #[test]
    fn backpressure_flushes_when_max_exceeded() {
        let mut d = Debouncer::new(Duration::from_millis(50), 2);

        d.record(FileChange {
            path: PathBuf::from("a.rs"),
            kind: ChangeKind::Modified,
        });
        d.record(FileChange {
            path: PathBuf::from("b.rs"),
            kind: ChangeKind::Modified,
        });
        // Third change exceeds max_pending=2
        let batch = d.record(FileChange {
            path: PathBuf::from("c.rs"),
            kind: ChangeKind::Modified,
        });

        assert!(batch.is_some());
        let batch = batch.unwrap();
        assert_eq!(batch.changes.len(), 3);
        assert_eq!(d.pending_count(), 0);
    }

    #[test]
    fn tick_emits_after_window_elapses() {
        let mut d = Debouncer::new(Duration::from_millis(0), 100);

        d.record(FileChange {
            path: PathBuf::from("a.rs"),
            kind: ChangeKind::Created,
        });

        // Window is 0ms so tick should emit immediately
        let batch = d.tick();
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().changes.len(), 1);
        assert_eq!(d.pending_count(), 0);
    }

    #[test]
    fn tick_does_not_emit_within_window() {
        let mut d = Debouncer::new(Duration::from_secs(60), 100);

        d.record(FileChange {
            path: PathBuf::from("a.rs"),
            kind: ChangeKind::Modified,
        });

        // Window is 60s, tick should not emit
        let batch = d.tick();
        assert!(batch.is_none());
        assert_eq!(d.pending_count(), 1);
    }
}
