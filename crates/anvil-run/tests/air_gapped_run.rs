//! #1705 (Council C-011): air-gap coverage for the `anvil-run`
//! launcher (INTL-001..-009).
//!
//! The shared air-gap harness exercises `anvil` subcommands from
//! `anvil-cli`'s test suite, but `anvil-run` ships as a separate
//! binary. `CARGO_BIN_EXE_anvil-run` is only defined for *this*
//! crate's integration tests, so the launcher's air-gap case lives
//! here rather than in `crates/anvil-cli/tests/air_gapped.rs`. The
//! harness *script* is shared and resolved by walking up from
//! `CARGO_MANIFEST_DIR`.
//!
//! `anvil-run --dry-run` resolves the launch context and runs the
//! daemon preflight — a Unix-domain socket connection, not a network
//! call — before printing the resolved plan and exiting. Under the
//! network-blocked namespace it must exit cleanly: `0` when a daemon
//! answers, or a launcher refusal code (`EXIT_DAEMON_UNAVAILABLE`)
//! when none is running. It must never hang on a network resolver or
//! be killed by signal.

#![cfg(target_os = "linux")]

use std::path::PathBuf;
use std::process::Command;

const ANVIL_RUN_BIN: &str = env!("CARGO_BIN_EXE_anvil-run");
const SKIP_EXIT_CODE: i32 = 77;

// KEEP-IN-SYNC: `harness_path` + `run_air_gapped` mirror the helpers in
// `crates/anvil-cli/tests/air_gapped.rs`. If the harness protocol
// changes (skip code, env hygiene, path walk), update both sites.

/// Resolve the workspace-root path to the network-blocked harness.
///
/// `CARGO_MANIFEST_DIR` points at `crates/anvil-run/`; the harness
/// lives at `<repo-root>/tools/test-harness/network-blocked/run.sh`,
/// so we go up two levels — matching the resolver in
/// `crates/anvil-cli/tests/air_gapped.rs`.
fn harness_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/anvil-run -> crates
    p.pop(); // crates -> repo root
    p.push("tools");
    p.push("test-harness");
    p.push("network-blocked");
    p.push("run.sh");
    p
}

/// Run `anvil-run` with `args` through the air-gapped harness. Returns
/// `None` if the harness chose to skip (exit 77, e.g. a kernel that
/// forbids unprivileged user namespaces); otherwise the `Output`.
fn run_air_gapped(args: &[&str]) -> Option<std::process::Output> {
    let harness = harness_path();
    assert!(
        harness.exists(),
        "harness script not found at {}; expected to be checked in",
        harness.display(),
    );
    let mut cmd = Command::new(&harness);
    cmd.arg(ANVIL_RUN_BIN);
    cmd.args(args);
    // Close stdin explicitly (mirrors the anvil-cli helper) so no
    // command under test can hang the harness waiting on input.
    cmd.stdin(std::process::Stdio::null());
    // Match the anvil-cli air-gap suite's hygiene env so welcome /
    // dev-only flows cannot reach for state. Neither is a
    // network-touching path; they exist to keep the test bounded.
    cmd.env("ANVIL_DEV", "1");
    cmd.env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("failed to spawn harness");
    if out.status.code() == Some(SKIP_EXIT_CODE) {
        eprintln!(
            "air-gapped harness skipped:\nstderr={}",
            String::from_utf8_lossy(&out.stderr),
        );
        return None;
    }
    Some(out)
}

#[test]
fn anvil_run_dry_run_exits_cleanly_with_no_network() {
    // `--dry-run` resolves context + queries the daemon, then prints
    // the plan without spawning the child. Whether or not a daemon is
    // reachable, it must exit (not hang, not die by signal) with no
    // network access.
    let Some(out) = run_air_gapped(&["--dry-run", "--tool", "codex", "--", "echo", "air-gap"])
    else {
        return;
    };
    assert!(
        out.status.code().is_some(),
        "anvil-run --dry-run was killed by signal under air-gap: {:?}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
}
