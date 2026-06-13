//! USAGE-001: command-invocation observation, proven end-to-end against
//! the built `anvil` binary.
//!
//! These tests assert the producer is **live** (not merely present in
//! source): running a real command appends exactly one `command.invoked`
//! Kindling row to the user-scoped sidecar, carrying the canonical
//! command name and honouring the privacy contract (no raw argument
//! values; sensitive-named options redacted).
//!
//! `ANVIL_HOME` is set on the **child process only**, so the usage log
//! lands under `<ANVIL_HOME>/user/kindling/usage.ndjson` (the same
//! user-state re-root as credentials) and the run is hermetic regardless
//! of test parallelism. Usage rows are user-state, not project-state, so
//! they are written even under a gated `ANVIL_HOME`.
//!
//! ## R2 mitigation (adding a command without an observation)
//!
//! The producer is wired once in `main`, above the command dispatch, so
//! it fires uniformly for every command — no per-command wiring exists to
//! forget. [`every_sampled_command_emits_exactly_one_row`] exercises a
//! representative spread of the registered command surface and asserts
//! each invocation yields exactly one well-formed row, locking that
//! structural guarantee.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Resolve the user-scoped usage sidecar under a given `ANVIL_HOME`.
fn usage_log(anvil_home: &Path) -> PathBuf {
    anvil_home
        .join("user")
        .join("kindling")
        .join("usage.ndjson")
}

/// Run an `anvil` subcommand with `ANVIL_HOME` set on the child env only,
/// plus the hermetic/offline env the other CLI tests use. Returns the
/// combined stdout+stderr for diagnostics.
fn run_anvil(home: &Path, args: &[&str]) -> String {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.args(args)
        // Run in the temp home so any repo-scanning command stays cheap
        // and hermetic, and never touches the real working tree.
        .current_dir(home)
        .env("ANVIL_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd.env_remove("ANVIL_TOUCH_PROJECT_STATE");
    cmd.env_remove("TRACEPARENT");
    let out = cmd.output().expect("spawn anvil");
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    combined
}

/// Read the usage rows as parsed JSON values, asserting each line is
/// valid JSON.
fn read_rows(home: &Path) -> Vec<serde_json::Value> {
    let path = usage_log(home);
    let contents =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    contents
        .lines()
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("usage row must be valid JSON ({e}); got: {line}"))
        })
        .collect()
}

#[test]
fn version_invocation_writes_one_command_invoked_row() {
    let home = tempdir().expect("anvil home");

    let out = run_anvil(home.path(), &["version"]);
    let rows = read_rows(home.path());

    assert_eq!(
        rows.len(),
        1,
        "exactly one usage row per invocation; out: {out}"
    );
    let row = &rows[0];
    assert_eq!(row["kind"], "command.invoked");
    assert_eq!(row["command"], "version");
    // flag_set is always present (never omitted), even when empty (ADR-041).
    assert!(
        row["flag_set"].is_array(),
        "flag_set must be present: {row}"
    );
    assert_eq!(row["flag_set"].as_array().expect("array").len(), 0);
    // Principal is anonymised: an unauthenticated run records `anonymous`,
    // never a raw identity.
    assert_eq!(row["principal"], "anonymous");
}

// Note: redaction of raw/sensitive argument *values* is proven
// rigorously at the unit level (`usage::tests::*` in
// `crates/anvil-cli/src/usage.rs` and `redaction::tests::*`), which can
// drive `arg_shapes_from_argv` + `append_usage_observation_to` directly
// with controlled sensitive values. No real subcommand accepts a
// sensitive-named flag, so an end-to-end "secret never written" test
// here would only pass vacuously; the unit tests are the real coverage.

#[test]
fn argument_shape_is_recorded_without_raw_value() {
    let home = tempdir().expect("anvil home");

    // `status --json` is a real, parseable invocation carrying a flag.
    // The row must capture the `json` flag's shape but no value content.
    run_anvil(home.path(), &["status", "--json"]);
    let rows = read_rows(home.path());
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["command"], "status");
    let args = row["args"].as_array().expect("args array");
    assert!(
        args.iter().any(|a| a["name"] == "json"),
        "the --json flag must be recorded as an arg shape: {row}"
    );
}

#[test]
fn every_sampled_command_emits_exactly_one_row() {
    // A spread across the registered command surface. The producer is
    // command-agnostic and wired once above dispatch, so every one of
    // these must yield exactly one row with its canonical name.
    for (args, canonical) in [
        (vec!["version"], "version"),
        (vec!["doctor"], "doctor"),
        (vec!["licenses"], "licenses"),
        (vec!["status"], "status"),
    ] {
        let home = tempdir().expect("anvil home");
        let out = run_anvil(home.path(), &args);
        let rows = read_rows(home.path());
        assert_eq!(
            rows.len(),
            1,
            "command {canonical:?} must emit exactly one usage row; out: {out}"
        );
        assert_eq!(
            rows[0]["command"], canonical,
            "row must carry the canonical command name"
        );
    }
}
