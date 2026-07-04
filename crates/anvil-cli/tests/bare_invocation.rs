//! CIB-177: bare `anvil` (no subcommand) must lead with a first-run pointer.
//!
//! Plain `anvil` fails clap's required-subcommand parse and renders the full
//! long help at exit 2. A first-time user should meet a short orientation
//! naming `anvil welcome` (tour) and `anvil start` (activate) before the wall
//! of commands — without changing the exit-code contract or subcommand parsing.

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

#[test]
fn bare_anvil_leads_with_first_run_pointer_and_still_exits_2() {
    let out = Command::new(ANVIL_BIN)
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .output()
        .expect("failed to invoke anvil");

    // Unchanged parsing contract: bare invocation is still a required-subcommand
    // failure at exit 2 (EXIT_GATE_FAIL), rendered to stderr.
    assert_eq!(
        out.status.code(),
        Some(2),
        "bare `anvil` must still exit 2; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);

    let welcome = stderr
        .find("anvil welcome")
        .unwrap_or_else(|| panic!("first-run pointer names `anvil welcome`:\n{stderr}"));
    let start = stderr
        .find("anvil start")
        .unwrap_or_else(|| panic!("first-run pointer names `anvil start`:\n{stderr}"));
    let commands = stderr
        .find("Commands:")
        .unwrap_or_else(|| panic!("help lists the commands:\n{stderr}"));

    assert!(
        welcome < commands && start < commands,
        "first-run pointer must lead before the command list:\n{stderr}"
    );

    // The exit-code contract block stays in place (after_help preserved).
    assert!(
        stderr.contains("EXIT CODES:"),
        "exit-codes footer must be preserved:\n{stderr}"
    );
}
