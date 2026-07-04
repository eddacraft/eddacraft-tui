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
//! Runs at 16 writers by default and 80 writers as a stress variant.
//! MLP2-015 promoted the 80-writer variant out of `#[ignore]` after a
//! local flake-budget review (10/10 green @ ~10ms each); the test fits
//! well inside the standard cargo-test budget so it now runs in CI
//! alongside the 16-writer sanity test.

use anvil_witness::{GenesisAnchor, RolloverPolicy, WitnessLine, WitnessWriter};
use std::collections::HashSet;
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

fn line_for_thread(seq: u64, prev: &str, thread_id: usize) -> WitnessLine {
    WitnessLine {
        seq,
        scope: "active".to_string(),
        kind: "witness".to_string(),
        prev_line_hash: prev.to_string(),
        project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".to_string(),
        commit_sha: Some(format!("t{thread_id}-{seq}")),
        parent_commits: Vec::new(),
        prev_line_hashes: Vec::new(),
        agent_tag: None,
        rules_sha: None,
        cutoff_commit: None,
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

    // CLAWP-047: force simultaneous append contention. Without the
    // barrier, threads could be scheduled to run sequentially and the
    // serialisation lock would never actually be contended, so the test
    // could pass without exercising concurrent appends. All writers
    // build their line first, then release together on the barrier.
    let barrier = Arc::new(Barrier::new(n_writers));
    let mut handles = Vec::with_capacity(n_writers);
    for thread_id in 0..n_writers {
        let writer = writer.clone();
        let prev = prev.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            // Each writer uses the same prev hash (the GENESIS
            // anchor) because we're testing serialisation under
            // load, not chain semantics — see the module doc.
            // Real callers chain from the running tip.
            let line = line_for_thread(thread_id as u64 + 1, &prev, thread_id);
            barrier.wait();
            // Generous acquire bound: this test asserts serialisation
            // correctness, not lock latency. With 80 fsync-ing writers
            // released on one barrier, the tail writer on a 2-core shared
            // CI runner legitimately waits past the default 5 s
            // (observed: `LockTimeout(5s)` reds on the cross-compile smoke
            // legs), so give contention a bound only a genuine wedge
            // would exceed.
            writer
                .append_with_lock_timeout(&line, std::time::Duration::from_mins(2))
                .unwrap();
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // CLAWP-046: assert every writer's record survived exactly once, not
    // just that the line *count* matches. Collect each line's commit_sha
    // and require the on-disk set to equal the expected per-thread set
    // (`t{thread_id}-{seq}`) — a lost write masked by a duplicate, or a
    // corrupted identity, would keep the count at N but fail this.
    let on_disk = std::fs::read_to_string(writer.active_path()).unwrap();
    let mut seen: HashSet<String> = HashSet::new();
    let mut parsed = 0;
    for raw in on_disk.lines() {
        if raw.is_empty() {
            continue;
        }
        let line: WitnessLine = serde_json::from_str(raw).expect("interleaved bytes in chain");
        seen.insert(
            line.commit_sha
                .expect("each witness line carries a commit_sha"),
        );
        parsed += 1;
    }
    assert_eq!(
        parsed, n_writers,
        "expected {n_writers} clean lines on disk, got {parsed}"
    );
    let expected: HashSet<String> = (0..n_writers)
        .map(|thread_id| format!("t{thread_id}-{}", thread_id + 1))
        .collect();
    assert_eq!(
        seen, expected,
        "every writer's record must survive exactly once (no lost or duplicated appends)"
    );
}

#[test]
fn sixteen_writers_no_interleaving() {
    run_concurrent(16);
}

#[test]
fn eighty_writers_no_interleaving() {
    run_concurrent(80);
}
