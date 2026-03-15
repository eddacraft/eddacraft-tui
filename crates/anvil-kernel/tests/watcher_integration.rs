use std::fs;
use std::time::Duration;

use anvil_kernel::watcher::{WatcherConfig, start_watcher};

#[test]
fn detects_file_creation() {
    let dir = tempfile::tempdir().unwrap();
    let config = WatcherConfig {
        root: dir.path().to_path_buf(),
        debounce_window: Duration::from_millis(10),
        max_pending: 100,
        tick_interval: Duration::from_millis(5),
    };

    let (_watcher, rx) = start_watcher(&config).unwrap();

    // Give the watcher time to start
    std::thread::sleep(Duration::from_millis(50));

    // Create a file
    fs::write(dir.path().join("test.rs"), "fn main() {}").unwrap();

    // Wait for the batch
    let batch = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!batch.changes.is_empty());
    assert!(batch.changes.iter().any(|c| c.path.ends_with("test.rs")));
}
