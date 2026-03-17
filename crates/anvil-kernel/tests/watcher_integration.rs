use std::fs;
use std::time::Duration;

use anvil_kernel::watcher::filter::FileFilter;
use anvil_kernel::watcher::{WatcherConfig, start_watcher};

#[test]
fn detects_parseable_file_creation() {
    let dir = tempfile::tempdir().unwrap();
    let config = WatcherConfig {
        root: dir.path().to_path_buf(),
        debounce_window: Duration::from_millis(10),
        max_pending: 100,
        tick_interval: Duration::from_millis(5),
        filter: None, // uses default filter
    };

    let (_watcher, rx) = start_watcher(&config).unwrap();

    // Give the watcher time to start
    std::thread::sleep(Duration::from_millis(50));

    // Create a .ts file (parseable — should pass filter)
    fs::write(dir.path().join("test.ts"), "const x = 1;").unwrap();

    // Wait for the batch
    let batch = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!batch.changes.is_empty());
    assert!(batch.changes.iter().any(|c| c.path.ends_with("test.ts")));
}

#[test]
fn filters_out_non_parseable_files() {
    let dir = tempfile::tempdir().unwrap();
    let config = WatcherConfig {
        root: dir.path().to_path_buf(),
        debounce_window: Duration::from_millis(10),
        max_pending: 100,
        tick_interval: Duration::from_millis(5),
        filter: Some(FileFilter::default()),
    };

    let (_watcher, rx) = start_watcher(&config).unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Create a .md file (not parseable — should be filtered out)
    fs::write(dir.path().join("README.md"), "# Hello").unwrap();

    // Then create a .ts file (parseable — should pass)
    std::thread::sleep(Duration::from_millis(20));
    fs::write(dir.path().join("index.ts"), "export {};").unwrap();

    // We should only get the .ts file
    let batch = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(batch.changes.iter().all(|c| !c.path.ends_with("README.md")));
    assert!(batch.changes.iter().any(|c| c.path.ends_with("index.ts")));
}
