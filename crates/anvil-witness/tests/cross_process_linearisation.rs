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
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

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
    if let Ok(root) = env::var(ROOT_ENV) {
        let tag = env::var(TAG_ENV).unwrap_or_default();
        worker(Path::new(&root), &tag);
        return;
    }
    // `harness = false` still owes nextest/libtest the list protocol:
    // `--list --format terse` must print `<name>: test` and run NOTHING
    // (nextest parses that output to build the test list; a second
    // `--list --ignored` pass must print nothing since this test is never
    // ignored). Without this arm, `cargo nextest list` executed the whole
    // 6-process linearisation at list time and its summary line broke list
    // parsing — the release-gate Test job is the only CI context that runs
    // nextest over this binary, so the gap stayed latent from CIB-125 until
    // the v0.9.0-beta cut.
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--list") {
        if !args.iter().any(|a| a == "--ignored") {
            println!("cross_process_linearisation: test");
        }
        return;
    }
    parent();
}

/// The `commit_sha` tag for worker `w`'s `i`-th record — matches the worker's
/// `format!("{tag}-{i}")` where `tag == "w{w}"`. Built with `push_str` rather than
/// `format!` so `w`/`i` are used via plain method calls: `CodeQL`'s unused-variable
/// query does not recognise a variable used only inside a format-macro capture.
fn commit_tag(w: usize, i: usize) -> String {
    let mut s = String::from("w");
    s.push_str(&w.to_string());
    s.push('-');
    s.push_str(&i.to_string());
    s
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

/// One worker process: announce readiness, wait for the parent's go-signal (so all
/// workers contend at once — the parent only releases once every worker is here),
/// then append `APPENDS_PER_WORKER` records via `append_chained`.
fn worker(root: &Path, tag: &str) {
    // Announce we've reached the barrier so the parent can release all workers
    // together (a symmetric barrier — otherwise a slow-to-spawn worker could start
    // after the go-signal and never overlap, weakening the contention this proves).
    fs::write(root.join(format!(".xproc-ready-{tag}")), b"1").expect("write ready-signal");

    let go = root.join(".xproc-go");
    let deadline = Instant::now() + Duration::from_secs(30);
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
                // Capture stderr so a worker's panic is legible (prefixed by tag on
                // failure) rather than interleaved across six processes' shared fd.
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn worker")
        })
        .collect();

    // Wait until every worker has reached the barrier, THEN release them together —
    // this guarantees real simultaneous contention even on a slow/loaded runner (F2).
    let ready_deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let ready = (0..WORKERS)
            .filter(|w| root.join(format!(".xproc-ready-w{w}")).exists())
            .count();
        if ready == WORKERS {
            break;
        }
        assert!(
            Instant::now() < ready_deadline,
            "only {ready}/{WORKERS} workers reached the barrier before the deadline",
        );
        thread::sleep(Duration::from_millis(2));
    }
    fs::write(root.join(".xproc-go"), b"go").expect("write go-signal");

    reap_workers(&mut children);
    verify_linear_chain(&root);
}

/// Reap every worker with a BOUNDED wait: a flock deadlock — the failure mode this
/// very test probes — would otherwise hang the parent on a blocking `wait()` until
/// the CI job's 45-60min limit, with no diagnostic. Poll with `try_wait`; on the
/// deadline, kill all stragglers (so none writes into the `TempDir` the caller then
/// drops) and fail fast. On a worker failure, surface its captured stderr, prefixed.
fn reap_workers(children: &mut [std::process::Child]) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut statuses: Vec<(usize, std::process::ExitStatus)> = Vec::new();
    loop {
        for (w, child) in children.iter_mut().enumerate() {
            if statuses.iter().any(|(sw, _)| *sw == w) {
                continue;
            }
            if let Some(status) = child.try_wait().expect("try_wait worker") {
                statuses.push((w, status));
            }
        }
        if statuses.len() == children.len() {
            break;
        }
        if Instant::now() >= deadline {
            for child in children.iter_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            let mut finished: Vec<usize> = statuses.iter().map(|(w, _)| *w).collect();
            finished.sort_unstable();
            panic!(
                "workers did not all finish within the deadline (possible flock deadlock); \
                 finished={finished:?}, killed the rest",
            );
        }
        thread::sleep(Duration::from_millis(5));
    }

    for (w, status) in &statuses {
        if !status.success() {
            if let Some(mut err) = children[*w].stderr.take() {
                let mut buf = String::new();
                let _ = err.read_to_string(&mut buf);
                for line in buf.lines() {
                    eprintln!("[worker w{w}] {line}");
                }
            }
            panic!("worker w{w} exited with {status}");
        }
    }
}

/// Assert the merged chain is ONE linear, Healthy sequence: genesis + N*K records,
/// each worker's record present exactly once. A fork (two records sharing a
/// `(seq, prev)`) surfaces as a `ChainBreak` from `verify_chain_dag`.
fn verify_linear_chain(root: &Path) {
    let expected_records = WORKERS * APPENDS_PER_WORKER;
    let paths = witness_paths(root);
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

    // Stronger than the count: every worker's record survived exactly once (no lost
    // or duplicated append), and no unexpected record appeared.
    let mut seen: HashSet<String> = HashSet::new();
    for p in &paths {
        // Fail loudly on a read error rather than defaulting to "" — a swallowed IO
        // error would surface later as a confusing missing-record assertion.
        let content = fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("failed to read witness file {}: {e}", p.display()));
        for raw in content.lines() {
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
        .flat_map(|w| (0..APPENDS_PER_WORKER).map(move |i| commit_tag(w, i)))
        .collect();
    assert_eq!(
        seen, expected,
        "every worker's records must survive exactly once"
    );

    println!(
        "cross-process linearisation OK: {WORKERS} processes x {APPENDS_PER_WORKER} appends \
         -> {} lines, one Healthy chain",
        dag.line_count,
    );
}
