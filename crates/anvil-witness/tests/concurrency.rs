//! MLP-002 concurrency contract: many simultaneous writers must not
//! corrupt the active file. The chain hashes are caller-managed (the
//! writer's job is to serialise appends, not to chain them), so this
//! test asserts the weaker — and on-disk-observable — property:
//!
//! - After N parallel appends, the active file contains N complete
//!   lines and parses cleanly.
//! - No partial / interleaved bytes appear inside a line.
//!
//! This stops short of asserting hash-chain integrity under
//! concurrency because the production flow has a single chain head
//! (the hook lane) — concurrent hooks against the same worktree are
//! prevented by the hook framework itself. The flock here is the
//! belt to the framework's braces.
//!
//! Runs at 16 writers by default to keep CI fast. An 80-writer stress
//! variant is gated behind `#[ignore]` so it can be invoked on demand
//! with `cargo test --ignored`.

use anvil_witness::{GenesisAnchor, RolloverPolicy, WitnessLine, WitnessWriter};
use std::sync::Arc;
use tempfile::TempDir;

fn line_for_thread(seq: u64, prev: &str, thread_id: usize) -> WitnessLine {
    WitnessLine {
        seq,
        scope: "active".to_string(),
        kind: "witness".to_string(),
        prev_line_hash: prev.to_string(),
        project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
        commit_sha: Some(format!("t{thread_id}-{seq}")),
        agent_tag: None,
        rules_sha: None,
        ts: "2026-05-13T00:00:00Z".to_string(),
        validation_at: "pre-commit".to_string(),
    }
}

fn run_concurrent(n_writers: usize) {
    let dir = TempDir::new().unwrap();
    let writer = Arc::new(
        WitnessWriter::open(
            dir.path(),
            "active",
            RolloverPolicy::tight(100_000, 100_000_000),
        )
        .unwrap(),
    );
    let prev = GenesisAnchor::Fresh.anchor_string().to_string();

    let mut handles = Vec::with_capacity(n_writers);
    for thread_id in 0..n_writers {
        let writer = writer.clone();
        let prev = prev.clone();
        handles.push(std::thread::spawn(move || {
            // Each writer uses the same prev hash (the GENESIS
            // anchor) because we're testing serialisation under
            // load, not chain semantics — see the module doc.
            // Real callers chain from the running tip.
            let line = line_for_thread(thread_id as u64 + 1, &prev, thread_id);
            writer.append(&line).unwrap();
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // Every line should parse without interleaving.
    let on_disk = std::fs::read_to_string(writer.active_path()).unwrap();
    let mut parsed = 0;
    for raw in on_disk.lines() {
        if raw.is_empty() {
            continue;
        }
        let _: WitnessLine = serde_json::from_str(raw).expect("interleaved bytes in chain");
        parsed += 1;
    }
    assert_eq!(
        parsed, n_writers,
        "expected {n_writers} clean lines on disk, got {parsed}"
    );
}

#[test]
fn sixteen_writers_no_interleaving() {
    run_concurrent(16);
}

#[test]
#[ignore = "stress test — run with `cargo test --ignored` on demand"]
fn eighty_writers_no_interleaving() {
    run_concurrent(80);
}
