//! POLENG-008 criterion bench — regorus eval through the Anvil facade.
//!
//! Tracks the *realistic* per-evaluation cost Anvil pays (`Engine::eval`:
//! serialise input → set on engine → eval → extract) on the representative
//! fixtures, so regressions show up in `cargo bench`. The cross-engine parity
//! gate (regorus vs Go OPA) lives in `scripts/bench-vs-go-opa.sh`, which uses
//! the eval-only `examples/parity_harness.rs` for a like-for-like comparison.

use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

use anvil_policy_engine::{Engine, EngineConfig, PolicyInput};
use criterion::{Criterion, criterion_group, criterion_main};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/fixtures")
}

fn load_input() -> PolicyInput {
    let raw = fs::read_to_string(fixtures_dir().join("input.json")).expect("read input.json");
    serde_json::from_str(&raw).expect("parse input.json")
}

fn bench_policy(c: &mut Criterion, name: &str, file: &str, query: &str) {
    let source = fs::read_to_string(fixtures_dir().join(file)).expect("read policy");
    let input = load_input();
    let mut engine = Engine::new(EngineConfig::default()).expect("engine");
    engine
        .add_policy(file.to_string(), source)
        .expect("add_policy");

    c.bench_function(&format!("eval/{name}"), |b| {
        b.iter(|| {
            black_box(
                engine
                    .eval(black_box(&input), black_box(query))
                    .expect("eval"),
            )
        });
    });
}

fn benches(c: &mut Criterion) {
    bench_policy(
        c,
        "arch_boundary",
        "arch_boundary.rego",
        "data.arch.findings",
    );
    bench_policy(
        c,
        "baseline_filter",
        "baseline_filter.rego",
        "data.baseline_filter.findings",
    );
    bench_policy(c, "repo_scan", "repo_scan.rego", "data.repo.summary");
}

criterion_group!(parity, benches);
criterion_main!(parity);
