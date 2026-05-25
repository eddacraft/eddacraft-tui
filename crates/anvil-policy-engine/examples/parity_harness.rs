//! POLENG-008 regorus parity harness.
//!
//! Times regorus rule evaluation to compare against the Go OPA reference
//! (`scripts/bench-vs-go-opa.sh`). It measures `Engine::eval_rule` on a
//! prepared engine with input bound once — the same scope `opa bench` reports
//! as `rego_query_eval_ns` — so the two numbers compare like for like and the
//! result isolates *engine* eval speed (ADR-040 D-1), not facade/serde
//! overhead (which the criterion bench in `benches/parity.rs` covers).
//!
//! Usage: `parity_harness <policy.rego> <input.json> <rule-path> [iterations]`
//! Prints one JSON line: `{"engine":"regorus","n":N,"p50":..,"p95":..,"p99":..,"mean":..}`.

use std::time::Instant;

use regorus::Engine;

/// Nearest-rank percentile over ascending samples. Index arithmetic loses
/// precision, which is irrelevant for a benchmark percentile.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (p * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: parity_harness <policy.rego> <input.json> <rule-path> [iterations]");
        std::process::exit(2);
    }
    let policy = std::fs::read_to_string(&args[1]).expect("read policy");
    let input_json = std::fs::read_to_string(&args[2]).expect("read input");
    let rule = args[3].clone();
    let iters: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(50_000);

    let mut engine = Engine::new();
    engine
        .add_policy("policy.rego".to_string(), policy)
        .expect("add_policy");
    engine.set_input_json(&input_json).expect("set_input_json");

    // Warm up: the first eval triggers compile/prepare once; exclude it.
    for _ in 0..2000 {
        engine.eval_rule(rule.clone()).expect("eval_rule");
    }

    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        let _ = engine.eval_rule(rule.clone()).expect("eval_rule");
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort_unstable();

    let mean = samples.iter().sum::<u128>() / samples.len() as u128;
    println!(
        "{{\"engine\":\"regorus\",\"n\":{},\"p50\":{},\"p95\":{},\"p99\":{},\"mean\":{}}}",
        iters,
        percentile(&samples, 0.50),
        percentile(&samples, 0.95),
        percentile(&samples, 0.99),
        mean
    );
}
