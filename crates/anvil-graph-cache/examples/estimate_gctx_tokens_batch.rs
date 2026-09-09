//! Batch oracle for the Python `gctx-simple-v1` port cross-check.
//!
//! Reads a JSON array of strings on stdin; writes a JSON envelope object:
//! `{ "max_bytes": N, "estimator": "...", "results": [ ... ] }` where each
//! result is `{ "ok": true, "tokens": N, "input_bytes": N, "estimator": "...", "capped": bool }`
//! or `{ "ok": false, "error": "input_too_large", "input_bytes": N, "max_bytes": N }`.
//!
//! Driven by `plans/evals/context-compiler/2026-09-08/crosscheck_gctx_tokens.py`.

use anvil_graph_cache::{
    MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES, TokenEstimateError, estimate_gctx_tokens,
};
use serde_json::{Value, json};
use std::io::{self, Read};

fn main() {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw).expect("read stdin");
    let inputs: Vec<String> =
        serde_json::from_str(&raw).expect("stdin must be a JSON array of strings");

    let results: Vec<Value> = inputs
        .iter()
        .map(|input| match estimate_gctx_tokens(input, None) {
            Ok(est) => json!({
                "ok": true,
                "tokens": est.tokens,
                "input_bytes": est.input_bytes,
                "estimator": est.estimator,
                "capped": est.capped,
            }),
            Err(TokenEstimateError::InputTooLarge {
                input_bytes,
                max_bytes,
            }) => json!({
                "ok": false,
                "error": "input_too_large",
                "input_bytes": input_bytes,
                "max_bytes": max_bytes,
            }),
        })
        .collect();

    // Surface the shared constant so the driver can assert it matches Python.
    let envelope = json!({
        "max_bytes": MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES,
        "estimator": anvil_graph_cache::GCTX_TOKEN_ESTIMATOR_VERSION,
        "results": results,
    });
    println!("{}", serde_json::to_string(&envelope).expect("serialize"));
}
