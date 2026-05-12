//! MLP-017: air-gapped operation guarantee scaffold.
//!
//! Confirms that the read-only / activation-adjacent commands ship
//! today survive being run with no network access. This is the v1
//! scaffold for the release-gate guarantee that ADR-036 §D-3 calls
//! out: `anvil start`, `anvil baseline`, `anvil intercept ensure`,
//! all `anvil hook` subcommands, and `anvil audit` MUST make zero
//! network calls in normal operation.
//!
//! The scaffold lives here and grows as each command lands:
//!
//! - **v1 (this PR):** the `anvil version --offline` subcommand
//!   succeeds under the network-blocked harness, and the read-only
//!   `anvil status --verify --json` path exits cleanly (any code;
//!   not killed by signal). These are the currently-shipping
//!   commands in the MLP/INTL slate that legitimately run without
//!   network access. They give the harness real exercise and make
//!   the rule visible to anyone adding new commands.
//! - **As MLP-002 → -008 land:** each new command (`anvil hook
//!   pre-commit`, `anvil baseline`, etc.) adds a `#[test]` here.
//!   Reviewers should reject hook PRs that don't extend this test.
//!
//! The harness lives at `tools/test-harness/network-blocked/run.sh`
//! and uses Linux `unshare -n -r` to strip the network namespace.
//! On platforms where the primitive isn't available the script
//! exits 77 (skip) and the test treats that as "platform skip" so
//! local macOS development isn't blocked.

#![cfg(target_os = "linux")]

use std::path::PathBuf;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const SKIP_EXIT_CODE: i32 = 77;

/// Resolve the workspace-root path to the network-blocked harness.
///
/// `CARGO_MANIFEST_DIR` points at `crates/anvil-cli/`; the harness
/// lives at `<repo-root>/tools/test-harness/network-blocked/run.sh`,
/// so we go up two levels.
fn harness_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/anvil-cli -> crates
    p.pop(); // crates -> repo root
    p.push("tools");
    p.push("test-harness");
    p.push("network-blocked");
    p.push("run.sh");
    p
}

/// Run `args` through the air-gapped harness. Returns `None` if the
/// harness chose to skip (exit 77); otherwise the `Output` from the
/// command under test.
fn run_air_gapped(args: &[&str]) -> Option<std::process::Output> {
    let harness = harness_path();
    assert!(
        harness.exists(),
        "harness script not found at {}; expected to be checked in",
        harness.display(),
    );
    let mut cmd = Command::new(&harness);
    cmd.arg(ANVIL_BIN);
    cmd.args(args);
    // Match the rest of the test suite's hygiene env. ANVIL_DEV
    // suppresses dev-only welcome flows; ANVIL_SKIP_WELCOME guards
    // against the welcome screen reaching for state. Neither of
    // these is a network-touching path on its own; they exist to
    // keep the test bounded.
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
fn anvil_version_offline_succeeds_with_no_network() {
    let Some(out) = run_air_gapped(&["--no-tui", "version", "--offline"]) else {
        return;
    };
    assert!(
        out.status.success(),
        "anvil version --offline failed under air-gap: stderr={}",
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("anvil "), "missing version line: {stdout}");
}

#[test]
fn anvil_status_verify_json_exits_cleanly_with_no_network() {
    // `anvil status --verify --json` is the read-only activation
    // probe (LAUNCH-012). It must not depend on the network in
    // normal operation — the diagnostic comes from local state.
    let Some(out) = run_air_gapped(&["--no-tui", "--json", "status", "--verify"]) else {
        return;
    };
    // We don't enforce success here: `status --verify` legitimately
    // returns a non-zero exit when activation is incomplete (e.g.
    // running outside a workspace). What we DO enforce is that the
    // process exits cleanly under the air-gap rather than hanging on
    // a network resolver. Cleanly = any exit status; not segfault,
    // not killed-by-signal. Test name reflects that contract.
    assert!(
        out.status.code().is_some(),
        "anvil status --verify --json was killed by signal under air-gap: {:?}",
        out.status,
    );
}

#[test]
fn harness_is_executable_and_checked_in() {
    // Defensive: if a reviewer accidentally drops the executable
    // bit on the harness, every subsequent test would silently
    // exit 126 (perms). Detect that early with an unambiguous
    // failure here.
    let harness = harness_path();
    assert!(
        harness.exists(),
        "harness missing at {}; MLP-017 scaffold has been damaged",
        harness.display(),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&harness).expect("harness metadata");
        let mode = meta.permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "harness at {} is not executable (mode={mode:o})",
            harness.display(),
        );
    }
}
