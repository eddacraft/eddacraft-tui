use std::fs;
use std::time::Duration;

use anvil_kernel::watcher::filter::FileFilter;
use anvil_kernel::watcher::{
    WatchSetupDiagnostics, WatcherConfig, failure_guidance, start_watcher,
};
use notify::{Error as NotifyError, ErrorKind as NotifyErrorKind};

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

    let (_watcher, rx, _diag) = start_watcher(&config, None).unwrap();

    // Give the watcher time to start; cross-compiled macOS runners can
    // take longer to register the temp directory with the OS watcher.
    std::thread::sleep(Duration::from_millis(250));

    // Create a .ts file (parseable — should pass filter)
    fs::write(dir.path().join("test.ts"), "const x = 1;").unwrap();

    // Wait for the batch
    let batch = rx.recv_timeout(Duration::from_secs(10)).unwrap();
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

    let (_watcher, rx, _diag) = start_watcher(&config, None).unwrap();

    // Match the conservative warm-up used by detects_parseable_file_creation;
    // a 50 ms budget can expire before the OS registers the watch on a loaded
    // runner, causing the recv_timeout below to return a stale/empty batch.
    std::thread::sleep(Duration::from_millis(250));

    // Create a .md file (not parseable — should be filtered out)
    fs::write(dir.path().join("README.md"), "# Hello").unwrap();

    // Then create a .ts file (parseable — should pass)
    std::thread::sleep(Duration::from_millis(20));
    fs::write(dir.path().join("index.ts"), "export {};").unwrap();

    // We should only get the .ts file. Use the same generous 10 s receive
    // window as detects_parseable_file_creation: a loaded CI runner can take
    // well over 2 s to emit the debounced batch, and a short window here is
    // the next-most-likely flake after the warm-up sleep was aligned.
    let batch = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(batch.changes.iter().all(|c| !c.path.ends_with("README.md")));
    assert!(batch.changes.iter().any(|c| c.path.ends_with("index.ts")));
}

#[test]
fn setup_diagnostics_report_registered_directories() {
    let dir = tempfile::tempdir().unwrap();
    // Create a nested directory so we know watch_directories walks past root.
    fs::create_dir_all(dir.path().join("src/inner")).unwrap();

    let config = WatcherConfig {
        root: dir.path().to_path_buf(),
        debounce_window: Duration::from_millis(10),
        max_pending: 100,
        tick_interval: Duration::from_millis(5),
        filter: None,
    };

    let (_watcher, _rx, diag) = start_watcher(&config, None).unwrap();

    // root + src + src/inner = at least 3 successful registrations on a clean host.
    assert!(
        diag.registered >= 3,
        "expected >=3 registered dirs, got {}",
        diag.registered
    );
    assert_eq!(
        diag.failed, 0,
        "no failures expected on a fresh tempdir, got {} (samples: {:?})",
        diag.failed, diag.sample_errors
    );
    assert!(!diag.root_failed);
    assert!(!diag.limit_exhausted);
    assert!(diag.sample_errors.is_empty());
}

#[test]
fn setup_diagnostics_default_is_all_zero() {
    let d = WatchSetupDiagnostics::default();
    assert_eq!(d.registered, 0);
    assert_eq!(d.failed, 0);
    assert!(!d.root_failed);
    assert!(!d.limit_exhausted);
    assert!(d.sample_errors.is_empty());
}

// CIB-175: watcher-start failures should render a human line that names the
// likely cause and a concrete next step, per platform, rather than a raw
// `notify` chain. The assertions here synthesise `notify::Error` values and
// check the copy names a cause + next step, with cfg-gated platform wording.

#[test]
fn failure_guidance_max_files_watch_names_limit_and_next_step() {
    let err = NotifyError::new(NotifyErrorKind::MaxFilesWatch);
    let msg = failure_guidance(&err);
    let lower = msg.to_lowercase();
    // Names the cause.
    assert!(
        lower.contains("limit"),
        "should name the OS watch-limit cause: {msg}"
    );
    // Names a next step (reduce scope or close processes).
    assert!(
        msg.contains("--exclude") || lower.contains("close"),
        "should give an actionable next step: {msg}"
    );

    #[cfg(target_os = "linux")]
    assert!(
        msg.contains("max_user_watches"),
        "Linux copy should name the inotify sysctl: {msg}"
    );
    #[cfg(not(target_os = "linux"))]
    assert!(
        !msg.contains("max_user_watches"),
        "non-Linux copy should stay generic (no inotify sysctl wording): {msg}"
    );
}

#[test]
fn failure_guidance_io_names_permission_or_fd_and_retry() {
    let err = NotifyError::io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "permission denied",
    ));
    let msg = failure_guidance(&err);
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("permission") || lower.contains("descriptor"),
        "I/O copy should name a permission/fd cause: {msg}"
    );
    assert!(
        lower.contains("retry"),
        "I/O copy should give a next step: {msg}"
    );
}

#[test]
fn failure_guidance_path_not_found_suggests_retry() {
    let err = NotifyError::path_not_found();
    let msg = failure_guidance(&err);
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("exist"),
        "path-not-found copy should name the missing-path cause: {msg}"
    );
    assert!(
        lower.contains("retry"),
        "path-not-found copy should give a next step: {msg}"
    );
}

#[test]
fn failure_guidance_generic_falls_back_to_retry_and_report() {
    let err = NotifyError::generic("opaque internal failure");
    let msg = failure_guidance(&err);
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("retry"),
        "generic copy should suggest retry: {msg}"
    );
    assert!(
        lower.contains("report"),
        "generic copy should suggest reporting: {msg}"
    );
}
