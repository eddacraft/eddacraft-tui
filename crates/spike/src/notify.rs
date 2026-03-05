use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{EventKind, RecursiveMode, Watcher};

fn main() {
    println!("=== KERN-002: notify-rs file detection latency spike ===\n");

    let tmp_dir = std::env::temp_dir().join("anvil-spike-notify");
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).expect("failed to clean temp dir");
    }
    std::fs::create_dir_all(&tmp_dir).expect("failed to create temp dir");

    println!("  Watch directory: {}\n", tmp_dir.display());

    benchmark_detection_latency(&tmp_dir, 100);
    benchmark_burst_detection(&tmp_dir, 50);

    std::fs::remove_dir_all(&tmp_dir).ok();
    println!("\n=== Spike complete ===");
}

fn benchmark_detection_latency(dir: &Path, iterations: usize) {
    println!("--- Single-file detection latency ({iterations} events) ---\n");

    let (tx, rx) = mpsc::channel();
    let mut watcher =
        notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result {
                if is_relevant(event.kind) {
                    let _ = tx.send(Instant::now());
                }
            }
        })
        .expect("failed to create watcher");

    watcher
        .watch(dir, RecursiveMode::Recursive)
        .expect("failed to watch directory");

    std::thread::sleep(Duration::from_millis(100));

    let test_file = dir.join("test-latency.txt");
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let write_time = Instant::now();
        let mut f = std::fs::File::create(&test_file).expect("failed to create file");
        f.write_all(format!("iteration {i}\n").as_bytes())
            .expect("failed to write");
        f.sync_all().expect("failed to sync");
        drop(f);

        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(detection_time) => {
                let latency = detection_time.duration_since(write_time);
                latencies.push(latency);
            }
            Err(_) => {
                eprintln!("  ⚠ Timeout on iteration {i} — no event received within 2s");
            }
        }

        while rx.try_recv().is_ok() {}
        std::thread::sleep(Duration::from_millis(50));
    }

    if latencies.is_empty() {
        println!("  ✗ FAIL — no events detected");
        return;
    }

    latencies.sort();
    let count = latencies.len();
    let median = latencies[count / 2];
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let p99_idx = ((count as f64) * 0.99) as usize;
    let p99 = latencies[p99_idx.min(count - 1)];
    #[allow(clippy::cast_possible_truncation)]
    let mean: Duration = latencies.iter().sum::<Duration>() / count as u32;
    let min = latencies[0];
    let max = latencies[count - 1];

    println!("  Events detected: {count}/{iterations}");
    println!("  min={min:.1?}, mean={mean:.1?}, median={median:.1?}, p99={p99:.1?}, max={max:.1?}");
    println!("  Target: p99 < 20ms");

    if p99 < Duration::from_millis(20) {
        println!("  ✓ PASS");
    } else {
        println!("  ✗ FAIL — p99 exceeds 20ms");
    }
}

fn benchmark_burst_detection(dir: &Path, file_count: usize) {
    println!("\n--- Burst detection ({file_count} files written rapidly) ---\n");

    let (tx, rx) = mpsc::channel();
    let mut watcher =
        notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result {
                if is_relevant(event.kind) {
                    let _ = tx.send(Instant::now());
                }
            }
        })
        .expect("failed to create watcher");

    watcher
        .watch(dir, RecursiveMode::Recursive)
        .expect("failed to watch directory");

    std::thread::sleep(Duration::from_millis(100));

    let burst_start = Instant::now();
    for i in 0..file_count {
        let path = dir.join(format!("burst-{i}.txt"));
        std::fs::write(&path, format!("burst file {i}\n")).expect("failed to write burst file");
    }
    let write_elapsed = burst_start.elapsed();

    std::thread::sleep(Duration::from_millis(500));

    let mut event_count = 0;
    while rx.try_recv().is_ok() {
        event_count += 1;
    }

    let total_elapsed = burst_start.elapsed();

    println!("  Files written: {file_count} in {write_elapsed:.1?}");
    println!("  Events received: {event_count}");
    println!("  Total time to settle: {total_elapsed:.1?}");

    if event_count > 0 {
        println!("  ✓ PASS — watcher detected burst events");
    } else {
        println!("  ✗ FAIL — no burst events detected");
    }
}

fn is_relevant(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}
