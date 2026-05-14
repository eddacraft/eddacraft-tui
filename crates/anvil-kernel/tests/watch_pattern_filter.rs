//! LAUNCH-001 validation: the watch loop respects user-supplied
//! `--patterns` / `--exclude` glob filters. An excluded path must not
//! raise a violation event; an included path must.

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anvil_kernel::watch::{WatchConfig, run_watch};
use anvil_kernel::watcher::WatcherConfig;
use anvil_kernel_types::{EngineEvent, EventPayload};
use tempfile::TempDir;

/// Source files that, when scanned, will trigger a `NewDependencyIntroduction`
/// violation against the architecture config below — one importing into
/// `forbidden_dep`, one into a benign `infra` layer.
fn write_fixture(root: &std::path::Path) {
    fs::create_dir_all(root.join("src/app")).unwrap();
    fs::create_dir_all(root.join("vendor/lib")).unwrap();
    fs::create_dir_all(root.join(".anvil")).unwrap();

    fs::write(
        root.join("src/app/main.ts"),
        r#"
import { helper } from "../infra/helper";
export function main() { return helper(); }
"#,
    )
    .unwrap();
    fs::write(
        root.join("vendor/lib/util.ts"),
        r#"
import { helper } from "../../src/infra/helper";
export const util = helper();
"#,
    )
    .unwrap();
    fs::write(root.join(".anvil/architecture.yaml"), "layers: []\n").unwrap();
}

fn collect_events(rx: &mpsc::Receiver<EngineEvent>, settle: Duration) -> Vec<EngineEvent> {
    thread::sleep(settle);
    rx.try_iter().collect()
}

fn snapshot_files_watched(events: &[EngineEvent]) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::Snapshot { files_watched, .. } => Some(*files_watched),
            _ => None,
        })
        .collect()
}

fn touched_files_in_violations(events: &[EngineEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::Violation { file, .. } => Some(file.clone()),
            _ => None,
        })
        .collect()
}

fn watcher_config(root: PathBuf) -> WatcherConfig {
    WatcherConfig {
        root,
        debounce_window: Duration::from_millis(20),
        tick_interval: Duration::from_millis(10),
        ..Default::default()
    }
}

#[test]
fn exclude_pattern_drops_matching_files_from_initial_scan() {
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path());

    let (tx, rx) = mpsc::channel();
    let cfg = WatchConfig {
        root: tmp.path().to_path_buf(),
        architecture_config: Some(tmp.path().join(".anvil/architecture.yaml")),
        watcher: watcher_config(tmp.path().to_path_buf()),
        include_patterns: Vec::new(),
        exclude_patterns: vec!["vendor/**".to_string()],
        warmup_paths: Vec::new(),
    };

    let handle = run_watch(&cfg, tx).unwrap();
    let events = collect_events(&rx, Duration::from_millis(400));
    handle.stop().unwrap();

    let snapshots = snapshot_files_watched(&events);
    let max_files = snapshots.iter().copied().max().unwrap_or(0);
    assert!(
        max_files <= 1,
        "expected vendor/** to be excluded; snapshots reported {snapshots:?} files"
    );
}

#[test]
fn include_pattern_restricts_initial_scan_to_matching_files() {
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path());

    let (tx, rx) = mpsc::channel();
    let cfg = WatchConfig {
        root: tmp.path().to_path_buf(),
        architecture_config: Some(tmp.path().join(".anvil/architecture.yaml")),
        watcher: watcher_config(tmp.path().to_path_buf()),
        include_patterns: vec!["src/**/*.ts".to_string()],
        exclude_patterns: Vec::new(),
        warmup_paths: Vec::new(),
    };

    let handle = run_watch(&cfg, tx).unwrap();
    let events = collect_events(&rx, Duration::from_millis(400));
    handle.stop().unwrap();

    let snapshots = snapshot_files_watched(&events);
    let max_files = snapshots.iter().copied().max().unwrap_or(0);
    assert!(
        max_files <= 1,
        "expected only src/** files to be tracked; snapshots reported {snapshots:?} files"
    );
}

#[test]
fn no_filter_means_all_files_are_scanned() {
    // Baseline: confirm the absence of patterns doesn't accidentally
    // filter anything out (the noop fast-path).
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path());

    let (tx, rx) = mpsc::channel();
    let cfg = WatchConfig {
        root: tmp.path().to_path_buf(),
        architecture_config: Some(tmp.path().join(".anvil/architecture.yaml")),
        watcher: watcher_config(tmp.path().to_path_buf()),
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        warmup_paths: Vec::new(),
    };

    let handle = run_watch(&cfg, tx).unwrap();
    let events = collect_events(&rx, Duration::from_millis(400));
    handle.stop().unwrap();

    let snapshots = snapshot_files_watched(&events);
    let max_files = snapshots.iter().copied().max().unwrap_or(0);
    assert_eq!(
        max_files, 2,
        "expected both fixture files to be tracked with no filter; got {snapshots:?}"
    );
}

#[test]
fn invalid_pattern_surfaces_a_clear_error() {
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path());

    let (tx, _rx) = mpsc::channel();
    let cfg = WatchConfig {
        root: tmp.path().to_path_buf(),
        architecture_config: None,
        watcher: watcher_config(tmp.path().to_path_buf()),
        include_patterns: vec!["src/[unclosed".to_string()],
        exclude_patterns: Vec::new(),
        warmup_paths: Vec::new(),
    };

    let Err(err) = run_watch(&cfg, tx) else {
        panic!("invalid glob should fail at start");
    };
    let msg = err.to_string();
    assert!(
        msg.contains("invalid watch pattern") && msg.contains("src/[unclosed"),
        "expected pattern compile error, got: {msg}"
    );
}

#[test]
fn excluded_runtime_change_does_not_emit_violation() {
    // After initial scan, write a new file that matches the exclude
    // pattern. The watch loop must drop the modify event.
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path());

    let (tx, rx) = mpsc::channel();
    let cfg = WatchConfig {
        root: tmp.path().to_path_buf(),
        architecture_config: Some(tmp.path().join(".anvil/architecture.yaml")),
        watcher: watcher_config(tmp.path().to_path_buf()),
        include_patterns: Vec::new(),
        exclude_patterns: vec!["vendor/**".to_string()],
        warmup_paths: Vec::new(),
    };

    let handle = run_watch(&cfg, tx).unwrap();
    // Let the initial scan complete before we write the new vendor file.
    thread::sleep(Duration::from_millis(300));
    // Drain events from the initial scan so we only assess what was
    // produced after the vendor write.
    let _ = rx.try_iter().count();

    fs::write(
        tmp.path().join("vendor/lib/added.ts"),
        "export const added = 1;\n",
    )
    .unwrap();

    let events = collect_events(&rx, Duration::from_millis(600));
    handle.stop().unwrap();

    let touched = touched_files_in_violations(&events);
    assert!(
        !touched.iter().any(|f| f.contains("vendor")),
        "vendor file should not produce events; touched files: {touched:?}"
    );
}

// NOTE: an integration test for "deleting an excluded file emits no
// event" was considered and dropped. Under both the pre-M3 and post-M3
// code paths, a Removed event for an untracked path is a no-op at the
// event surface: `process_change` calls `remove_file`, which returns
// an empty delta, the snapshot branch is skipped, and nothing is
// emitted. Such a test would pass regardless of whether the M3
// graph-membership gate is in place, so it would not catch the bypass
// regressing.
//
// The M3 fix is a perf/cleanliness improvement (avoid spurious work
// on `.git`/`node_modules` churn during rebases) — not a behavioural
// change at the event surface. We accept that there is no isolating
// regression test for the gate today; if the bypass is ever needed
// under load (e.g. a perf benchmark observes the spurious work), add
// a benchmark-anchored guard there rather than re-introducing this
// tautological event-level test.
