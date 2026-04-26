//! SCAN-001: parallel-rollout benchmark for `run_secret_check`.
//!
//! The welcome-screen rewrite delivered ~8.9× wall-time win on `entx`
//! by combining `ignore::WalkBuilder` with `rayon` per-file scanning.
//! SCAN-001 extends the same pattern to the rest of the scan-fanout
//! sites — including `run_secret_check`, which until this slice scanned
//! every file serially. This bench pins the speed-up so a regression
//! later (e.g. dropping `par_iter`, accidentally falling back to a
//! single-threaded rayon pool) is caught before it ships.
//!
//! The bench builds a deterministic in-memory corpus mirrored to a
//! `tempfile::TempDir`, then runs `run_secret_check` against it twice:
//!
//!   - `serial_baseline` — forces rayon to one worker thread.
//!   - `parallel_rollout` — uses the rolled-out pool (default rayon
//!     global pool — capped via `ANVIL_SCAN_THREADS` in the welcome
//!     surface, but unbounded here so the bench reports the scale of
//!     the win on multi-core boxes).
//!
//! The acceptance threshold for SCAN-001 is `parallel_rollout`
//! delivering >3× wall-time reduction over `serial_baseline` on this
//! corpus. Run via `cargo bench -p eddacraft-anvil-bench -- scan` and
//! capture the numbers in the commit body.

use std::time::Duration;

use anvil_checks::secret::{SecretCheckConfig, run_secret_check};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use tempfile::TempDir;

/// `entx`-class repo size — ~3 k files spread across nested directories.
const CORPUS_FILES: usize = 3000;

/// Build a synthetic repo on disk. Most files are clean source files;
/// a small handful seed real secret-pattern matches so the scanner
/// actually exercises the matcher hot path rather than short-circuiting
/// on a uniform corpus.
// Spread files across 30 subdirectories (100 each) so the walker
// does real directory traversal work.
const SUBDIRS: usize = 30;

fn build_repo() -> (TempDir, Vec<String>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();

    let mut paths: Vec<String> = Vec::with_capacity(CORPUS_FILES);

    for sub in 0..SUBDIRS {
        let subdir = root.join(format!("pkg{sub:02}"));
        std::fs::create_dir_all(&subdir).expect("subdir");
        let per = CORPUS_FILES / SUBDIRS;
        for i in 0..per {
            let path = subdir.join(format!("file_{i:04}.ts"));
            // Mix in plausible source content with a 1-in-50 secret seed
            // (the seed only appears in the file body, never in
            // filenames). Keeps the bench dominated by clean-file work
            // while still touching every code path.
            let content = if (sub * per + i).is_multiple_of(50) {
                "const apiKey = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\nexport function go() { return apiKey; }\n".to_string()
            } else {
                format!(
                    "export const value_{i} = {i};\nfunction helper_{i}() {{ return value_{i} * 2; }}\n"
                )
            };
            std::fs::write(&path, content).expect("write");
            paths.push(path.to_string_lossy().to_string());
        }
    }

    (dir, paths)
}

fn run_with_threads<F: FnOnce() + Send>(threads: usize, body: F) {
    // A scoped pool keeps the rayon override from leaking into other
    // benches that share the criterion process.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("rayon pool");
    pool.install(body);
}

fn bench_secret_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));
    group.throughput(Throughput::Elements(CORPUS_FILES as u64));

    let (_dir, paths) = build_repo();
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let config = SecretCheckConfig::default();

    group.bench_function("serial_baseline", |b| {
        b.iter(|| {
            run_with_threads(1, || {
                let _ = run_secret_check(&refs, &config, None);
            });
        });
    });

    group.bench_function("parallel_rollout", |b| {
        b.iter(|| {
            // Use min(8, available) — captures the realistic 4-8 thread
            // cap that the welcome flow imposes via SCAN-003 while
            // still leaving enough head-room to demonstrate the win.
            let threads = std::cmp::min(num_cpus_fallback(), 8);
            run_with_threads(threads, || {
                let _ = run_secret_check(&refs, &config, None);
            });
        });
    });

    group.finish();
}

/// Tiny `num_cpus` shim — anvil-bench doesn't pull in `num_cpus` and we
/// want to avoid adding it just for the bench.
fn num_cpus_fallback() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
        .max(1)
}

criterion_group!(benches, bench_secret_scan);
criterion_main!(benches);
