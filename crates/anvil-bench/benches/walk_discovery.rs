//! SCAN-005 spike benchmark: sequential `ignore::WalkBuilder` vs
//! `ignore::WalkParallel` for the discovery *walk* phase.
//!
//! Discovery already scans files in parallel (rayon, SCAN-001). The open
//! question SCAN-005 asks is whether parallelising the *walk itself* — the
//! directory traversal plus the per-entry `metadata()` stat that
//! `candidate_path` performs — buys ≥20% wall time. This bench isolates that
//! phase: it builds a synthetic repo once, then collects scan candidates two
//! ways using the identical candidate predicate. The variables are the walk
//! strategy and its collection mechanism — sequential `Walk` + in-order
//! collect vs `WalkParallel` + concurrent `mpsc` collect. The channel
//! send/recv is part of the parallel strategy's real cost and is deliberately
//! measured (a production refactor would pay it too), not factored out.
//!
//! The predicate is reconstructed from the public `anvil_checks::filter`
//! helpers because `welcome::candidate_path` is private; it mirrors that
//! function (file type, `ScanFilter::includes`, binary skip, 512 KB size cap).

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use anvil_checks::filter::{BUILD_ARTEFACT_DIRS, ScanFilter, is_binary_path};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ignore::{WalkBuilder, WalkState};
use tempfile::TempDir;

/// Default large-repo proxy. The SCAN-005 spec calls for a 30k-file synthetic;
/// 20k keeps one-time setup quick while staying well past the point where walk
/// cost is measurable. Override with `ANVIL_BENCH_WALK_FILES`.
const DEFAULT_CORPUS_FILES: usize = 20_000;
const SUBDIRS: usize = 100;
/// Mirrors `welcome::SCAN_MAX_FILE_SIZE`.
const SCAN_MAX_FILE_SIZE: u64 = 512 * 1024;

/// Where the benchmark walks. Either a real on-disk repo (when
/// `ANVIL_BENCH_WALK_ROOT` is set) or a synthetic tree built for the run.
/// The `TempDir` (if any) is held to keep the synthetic tree alive.
struct Corpus {
    root: PathBuf,
    label: String,
    _tmp: Option<TempDir>,
}

fn corpus_files() -> usize {
    // Treat non-positive / unparseable values as invalid and fall back to the
    // default, so the corpus is never empty (which would trip the `seq > 0`
    // sanity check) — e.g. `ANVIL_BENCH_WALK_FILES=0`.
    std::env::var("ANVIL_BENCH_WALK_FILES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_CORPUS_FILES)
}

/// Resolve the corpus. Env knobs (all optional; defaults keep the committed
/// bench portable + CI-runnable):
/// - `ANVIL_BENCH_WALK_ROOT=<dir>`  — walk a real repo instead of building one.
/// - `ANVIL_BENCH_WALK_FILES=<n>`   — synthetic file count (default 20k).
/// - `ANVIL_BENCH_WALK_DIR=<dir>`   — build the synthetic tree here (e.g. a
///   real-disk path) instead of the system temp dir, which is often tmpfs/RAM
///   and hides all IO cost.
fn build_corpus() -> Corpus {
    if let Ok(root) = std::env::var("ANVIL_BENCH_WALK_ROOT") {
        return Corpus {
            root: PathBuf::from(&root),
            label: format!("real:{root}"),
            _tmp: None,
        };
    }

    let n = corpus_files();
    let mut builder = tempfile::Builder::new();
    builder.prefix("scan005-walk-");
    let tmp = match std::env::var("ANVIL_BENCH_WALK_DIR") {
        Ok(dir) => builder.tempdir_in(dir),
        Err(_) => builder.tempdir(),
    }
    .expect("tempdir");

    let root = tmp.path().to_path_buf();
    // Spread exactly `n` files across at most `SUBDIRS` subdirs, distributing
    // the remainder so none are dropped and small corpora (n < SUBDIRS) still
    // produce files. `n >= 1` is guaranteed by `corpus_files`.
    let subdirs = SUBDIRS.min(n);
    let mut file_idx = 0usize;
    for sub in 0..subdirs {
        let here = n / subdirs + usize::from(sub < n % subdirs);
        let subdir = root.join(format!("pkg{sub:03}")).join("src");
        std::fs::create_dir_all(&subdir).expect("subdir");
        for _ in 0..here {
            // Alternate extensions both pass the scan filter and the
            // binary check, matching a typical TS/Rust monorepo.
            let ext = if file_idx.is_multiple_of(2) {
                "ts"
            } else {
                "rs"
            };
            let path = subdir.join(format!("file_{file_idx:05}.{ext}"));
            std::fs::write(&path, b"export const x = 1;\n").expect("write");
            file_idx += 1;
        }
    }
    Corpus {
        root,
        label: format!("synthetic:{n}"),
        _tmp: Some(tmp),
    }
}

/// Canonical scan filter, identical to `welcome::scan_project_at`.
fn build_filter() -> ScanFilter {
    ScanFilter::default_with(
        BUILD_ARTEFACT_DIRS
            .iter()
            .map(|d| format!("{d}/"))
            .collect(),
    )
}

/// Candidate predicate reconstructed from `welcome::candidate_path`. Returns
/// the path when the entry is a scannable file. Performs the same `metadata()`
/// stat the real discovery does — that stat is the part most likely to gain
/// from parallelism, so it must be inside the measured region.
///
/// SYNC: keep in step with `welcome::candidate_path`
/// (`crates/anvil-cli/src/commands/welcome.rs`). The steps below mirror it
/// exactly — file type, `ScanFilter::includes`, binary skip, 512 KB size cap.
/// Note `candidate_path` does NOT consult `is_always_scan_filename`; that
/// helper gates the separate Phase 1a allowlist loop, not candidacy.
fn candidate(entry: &ignore::DirEntry, filter: &ScanFilter) -> Option<PathBuf> {
    let ft = entry.file_type()?;
    if !ft.is_file() {
        return None;
    }
    let path = entry.path();
    if !filter.includes(path) {
        return None;
    }
    if is_binary_path(path) {
        return None;
    }
    if let Ok(meta) = entry.metadata()
        && meta.len() > SCAN_MAX_FILE_SIZE
    {
        return None;
    }
    Some(path.to_path_buf())
}

/// Walker config matching the production Phase 1b general walk. `standard_filters(true)`
/// fixes the gitignore-on path — the default discovery case. The `scan_all`
/// (gitignore-off, `standard_filters(false)`) variant is out of scope for this
/// spike; the measured speedup is therefore only validated for gitignore-on.
fn walker(root: &std::path::Path) -> WalkBuilder {
    let mut b = WalkBuilder::new(root);
    b.follow_links(false).standard_filters(true).hidden(false);
    b
}

/// Current approach: single-threaded walk, collect candidates.
fn collect_sequential(root: &std::path::Path, filter: &ScanFilter) -> Vec<PathBuf> {
    walker(root)
        .build()
        .filter_map(Result::ok)
        .filter_map(|entry| candidate(&entry, filter))
        .collect()
}

/// Alternative: `WalkParallel`, candidates streamed back over an mpsc channel
/// (low contention vs a shared `Mutex<Vec>`), collected after the walk drains.
fn collect_parallel(root: &std::path::Path, filter: &ScanFilter) -> Vec<PathBuf> {
    let (tx, rx) = mpsc::channel();
    walker(root).build_parallel().run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            if let Ok(entry) = result
                && let Some(path) = candidate(&entry, filter)
            {
                let _ = tx.send(path);
            }
            WalkState::Continue
        })
    });
    drop(tx);
    rx.iter().collect()
}

fn bench_walk_discovery(c: &mut Criterion) {
    let corpus = build_corpus();
    let root = corpus.root.as_path();
    let filter = build_filter();

    // Sanity: both strategies must find the same candidate set, otherwise the
    // comparison is meaningless.
    let seq = collect_sequential(root, &filter).len();
    let par = collect_parallel(root, &filter).len();
    assert_eq!(
        seq, par,
        "sequential and parallel walks disagree on candidate count"
    );
    assert!(
        seq > 0,
        "corpus produced no candidates — filter mismatch or empty root?"
    );
    eprintln!("[walk_discovery] corpus={} candidates={seq}", corpus.label);

    let mut group = c.benchmark_group("walk_discovery");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(20));
    group.throughput(Throughput::Elements(seq as u64));

    group.bench_function("sequential_walkbuilder", |b| {
        b.iter(|| std::hint::black_box(collect_sequential(root, &filter).len()));
    });

    group.bench_function("parallel_walkparallel", |b| {
        b.iter(|| std::hint::black_box(collect_parallel(root, &filter).len()));
    });

    group.finish();
}

criterion_group!(benches, bench_walk_discovery);
criterion_main!(benches);
