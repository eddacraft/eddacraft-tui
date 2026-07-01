//! CIB-125 / MLP2-005 "No divergence": `append_chained` must linearise across
//! **separate processes**, not just threads.
//!
//! The `tests/concurrency.rs` sibling proves the flock serialises raw `append`s
//! across threads. It does not exercise the load-bearing MLP2-005 property: that
//! `append_chained` derives `(seq, prev_line_hash)` and writes **atomically under
//! one flock hold**, so two *processes* racing on the same chain (a daemon vs an
//! embedded CLI fallback, or concurrent worktree hooks) produce ONE linear,
//! `verify_chain_dag`-Healthy chain — never a fork.
//!
//! This is a **custom-harness** test (`harness = false` in `Cargo.toml`): the test
//! binary re-execs *itself* via `current_exe()` as N worker processes (selected by
//! the [`ROOT_ENV`] environment variable), each appending against the same witness
//! root. The parent then verifies the merged chain.
//!
//! **Regression guard:** if `append_chained` were reverted to read the chain head
//! *outside* the lock (the pre-MLP2-005 TOCTOU), two processes would read the same
//! tip and write two records with the same `(seq, prev)` — a fork — and the
//! `verify_chain_dag` below would fail with a `ChainBreak`, failing this test.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, fs, process::Command, thread};

use anvil_witness::{
    GenesisAnchor, RolloverPolicy, WitnessLine, WitnessWriter, verify_chain_dag, witness_paths,
};

/// Set on a worker re-exec; its value is the shared witness root. Absent ⇒ parent.
const ROOT_ENV: &str = "ANVIL_WITNESS_XPROC_ROOT";
/// Per-worker tag, so each record carries a unique `commit_sha` (`{tag}-{i}`).
const TAG_ENV: &str = "ANVIL_WITNESS_XPROC_TAG";

const WORKERS: usize = 6;
const APPENDS_PER_WORKER: usize = 5;
const UUID: &str = "01997e4a-1b2c-7345-8901-abcdef123456";
const TS: &str = "2026-07-01T00:00:00Z";

fn main() {
    match env::var(ROOT_ENV) {
        Ok(root) => {
            let tag = env::var(TAG_ENV).unwrap_or_default();
            worker(Path::new(&root), &tag);
        }
        Err(_) => parent(),
    }
}

fn genesis_seed() -> WitnessLine {
    WitnessLine::genesis(
        &GenesisAnchor::Fresh,
        UUID,
        "active",
        TS,
        "pre-commit",
        None,
    )
}

fn record(seq: u64, prev: String, commit: &str) -> WitnessLine {
    WitnessLine {
        seq,
        scope: "active".to_string(),
        kind: "witness".to_string(),
        prev_line_hash: prev,
        project_uuid: UUID.to_string(),
        commit_sha: Some(commit.to_string()),
        parent_commits: Vec::new(),
        prev_line_hashes: Vec::new(),
        agent_tag: None,
        rules_sha: None,
        cutoff_commit: None,
        ts: TS.to_string(),
        validation_at: "pre-commit".to_string(),
    }
}

/// One worker process: wait for the parent's go-signal (so all workers contend at
/// once), then append `APPENDS_PER_WORKER` records via `append_chained`.
fn worker(root: &Path, tag: &str) {
    let go = root.join(".xproc-go");
    let deadline = Instant::now() + Duration::from_secs(15);
    while !go.exists() {
        if Instant::now() >= deadline {
            eprintln!("worker {tag}: go-signal never appeared");
            std::process::exit(2);
        }
        thread::sleep(Duration::from_millis(2));
    }

    let writer = WitnessWriter::open(root, "active", RolloverPolicy::default())
        .expect("worker: open writer");
    for i in 0..APPENDS_PER_WORKER {
        let commit = format!("{tag}-{i}");
        writer
            .append_chained(genesis_seed, |seq, prev| record(seq, prev, &commit))
            .unwrap_or_else(|e| panic!("worker {tag}: append_chained failed: {e}"));
    }
}

/// The parent process: spawn N workers, release them together, then verify the
/// merged chain is one linear, Healthy sequence with every record present once.
fn parent() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let exe = env::current_exe().expect("current_exe");

    let mut children: Vec<_> = (0..WORKERS)
        .map(|w| {
            Command::new(&exe)
                .env(ROOT_ENV, &root)
                .env(TAG_ENV, format!("w{w}"))
                .spawn()
                .expect("spawn worker")
        })
        .collect();

    // Release all workers simultaneously to force real lock contention.
    fs::write(root.join(".xproc-go"), b"go").expect("write go-signal");

    for (w, child) in children.iter_mut().enumerate() {
        let status = child.wait().expect("wait worker");
        assert!(status.success(), "worker w{w} exited with {status}");
    }

    // The N processes, each appending K times via `append_chained`, must yield ONE
    // linear, verifiable chain: genesis + N*K records. A fork (two records sharing
    // a `(seq, prev)`) would surface here as a `ChainBreak` from `verify_chain_dag`.
    let expected_records = WORKERS * APPENDS_PER_WORKER;
    let paths = witness_paths(&root);
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    let dag = verify_chain_dag(&refs)
        .expect("cross-process chain must verify Healthy (a fork would ChainBreak here)");
    assert_eq!(
        dag.line_count,
        1 + expected_records as u64,
        "expected genesis + {expected_records} records in one chain, got {}",
        dag.line_count,
    );
    assert_eq!(dag.merge_count, 0, "the chain must be strictly linear");

    // Stronger than the count: every worker's record survived exactly once (no
    // lost or duplicated append), and no unexpected record appeared.
    let mut seen: HashSet<String> = HashSet::new();
    for p in &paths {
        for raw in fs::read_to_string(p).unwrap_or_default().lines() {
            if raw.is_empty() {
                continue;
            }
            let line: WitnessLine = serde_json::from_str(raw).expect("clean line on disk");
            if let Some(sha) = line.commit_sha {
                assert!(
                    seen.insert(sha.clone()),
                    "record {sha} appears more than once"
                );
            }
        }
    }
    let expected: HashSet<String> = (0..WORKERS)
        .flat_map(|w| (0..APPENDS_PER_WORKER).map(move |i| format!("w{w}-{i}")))
        .collect();
    assert_eq!(
        seen, expected,
        "every worker's records must survive exactly once",
    );

    println!(
        "cross-process linearisation OK: {WORKERS} processes x {APPENDS_PER_WORKER} appends \
         -> {} lines, one Healthy chain",
        dag.line_count,
    );
}
