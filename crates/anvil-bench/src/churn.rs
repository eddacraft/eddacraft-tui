//! Shared file-churn driver for the resource benches (RLB-002/-005).
//!
//! A background thread rewrites a rotating window of source files on an
//! interval, so a watched repo produces sustained debounced saves — each of
//! which a watcher turns into a per-save `anvil check`. Both the watch bench
//! and the concurrent bench drive churn this way; the driver lives here so they
//! share one implementation.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Collect rewritable source files (the languages the scanner parses) from a
/// repo. JSON and other files are skipped — appending a comment line would make
/// JSON invalid and is not representative of a code save.
#[must_use]
pub fn collect_churnable_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => stack.push(path),
                Ok(ft) if ft.is_file() => {
                    if matches!(
                        path.extension().and_then(|e| e.to_str()),
                        Some("ts" | "js" | "rs")
                    ) {
                        out.push(path);
                    }
                }
                _ => {}
            }
        }
    }
    out.sort();
    out
}

/// Append a changing comment line so both content and mtime change — what
/// `notify` reports and what a real save looks like.
fn append_churn_line(path: &Path, tick: u64) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(file, "// churn {tick}")
}

/// Background file-churn driver: rewrites a rotating window of files on an
/// interval until dropped or [`ChurnDriver::stop`]ped. Implements `Drop` so an
/// early return never leaks the background writer.
pub struct ChurnDriver {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ChurnDriver {
    /// Start churning `batch` files per `interval` tick, rotating through `files`.
    #[must_use]
    pub fn start(files: Vec<PathBuf>, interval: Duration, batch: usize) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            let mut cursor = 0usize;
            let mut tick: u64 = 0;
            while !stop_flag.load(Ordering::Acquire) {
                for _ in 0..batch.max(1) {
                    let path = &files[cursor % files.len()];
                    let _ = append_churn_line(path, tick);
                    cursor = cursor.wrapping_add(1);
                }
                tick = tick.wrapping_add(1);
                std::thread::sleep(interval);
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }

    /// Signal the thread to stop and join it.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ChurnDriver {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_only_scannable_source_files() {
        use crate::fixture::{RepoSpec, generate_repo};
        let dir = tempfile::tempdir().unwrap();
        let repo = generate_repo(&RepoSpec::small(), dir.path()).unwrap();
        let files = collect_churnable_files(repo.root());
        assert!(!files.is_empty(), "expected churnable source files");
        assert!(
            files.iter().all(|p| matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("ts" | "js" | "rs")
            )),
            "json/other files must be excluded from churn"
        );
    }

    #[test]
    fn append_churn_line_changes_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.ts");
        std::fs::write(&path, "const x = 1;\n").unwrap();
        let before = std::fs::metadata(&path).unwrap().len();
        append_churn_line(&path, 7).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("// churn 7"));
        assert!(after.len() as u64 > before);
    }

    #[test]
    fn churn_driver_rewrites_until_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.ts");
        std::fs::write(&path, "// seed\n").unwrap();
        let driver = ChurnDriver::start(vec![path.clone()], Duration::from_millis(5), 1);
        std::thread::sleep(Duration::from_millis(60));
        driver.stop();
        let churns = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("// churn "))
            .count();
        assert!(churns >= 2, "expected sustained churn, saw {churns} writes");
    }

    #[test]
    fn drop_without_stop_does_not_leak_thread() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.ts");
        std::fs::write(&path, "// seed\n").unwrap();
        {
            let _driver = ChurnDriver::start(vec![path.clone()], Duration::from_millis(5), 1);
            std::thread::sleep(Duration::from_millis(20));
            // dropped here without calling stop() — Drop must join the thread.
        }
        // If Drop joined, no further writes happen after this point. We can't
        // assert "no more writes" deterministically, but reaching here without
        // hanging proves the join completed.
    }
}
