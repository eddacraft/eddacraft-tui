//! Panic-restore behaviour for [`TerminalGuard`] / the lifecycle panic hook
//! (TUIN-004).
//!
//! `TerminalGuard::enter()` installs a **process-global** panic hook (exactly
//! once, via an internal `OnceLock`) that calls `restore_terminal()` and then
//! chains to the previous hook. That global, install-once state is impossible to
//! exercise deterministically from multiple in-process `#[test]`s — and there is
//! no real TTY in CI, so the terminal side effects are not directly observable.
//!
//! So the load-bearing test runs the scenario in a **fresh subprocess** (fresh
//! panic hook + fresh `OnceLock`): it installs a sentinel "previous" hook, then
//! `TerminalGuard::enter()` (whose raw-mode call fails on the non-TTY child, but
//! still installs the lifecycle hook first), then panics. The parent asserts the
//! child unwound via panic *and* that the sentinel previous hook ran — proving
//! the lifecycle hook executed its body (`restore_terminal()` then chain) rather
//! than swallowing the panic or aborting.

#![cfg(feature = "lifecycle")]

use std::process::Command;

/// Env var that switches this test binary into the panicking child routine.
const CHILD_ENV: &str = "ANVIL_LIFECYCLE_PANIC_CHILD";
/// Marker the sentinel previous hook prints once it runs.
const PREV_HOOK_MARKER: &str = "SENTINEL_PREVIOUS_HOOK_RAN";
const PANIC_MESSAGE: &str = "intentional-panic-under-guard";

#[test]
fn panic_under_guard_restores_and_chains_to_previous_hook() {
    // Child leg: run the scenario and diverge (panic). Detected by env var.
    if std::env::var_os(CHILD_ENV).is_some() {
        run_child_scenario();
        unreachable!("child scenario must panic");
    }

    // Parent leg: re-exec this same test binary, filtered to this one test, with
    // the child env set so the `if` above takes the child branch.
    let exe = std::env::current_exe().expect("locate test binary");
    let output = Command::new(exe)
        .args([
            "panic_under_guard_restores_and_chains_to_previous_hook",
            "--exact",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .output()
        .expect("spawn child test process");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // The child must have unwound via panic, not exited cleanly.
    assert!(
        !output.status.success(),
        "child must fail via panic; status={:?}\n--- output ---\n{combined}",
        output.status,
    );
    // The lifecycle hook must have chained to our sentinel previous hook (it runs
    // `restore_terminal()` immediately before this), proving it neither swallowed
    // the panic nor aborted the process.
    assert!(
        combined.contains(PREV_HOOK_MARKER),
        "sentinel previous hook did not run — lifecycle hook failed to chain\n--- output ---\n{combined}",
    );
    // And the original panic payload must still surface.
    assert!(
        combined.contains(PANIC_MESSAGE),
        "panic payload missing from child output\n--- output ---\n{combined}",
    );
}

fn run_child_scenario() {
    // 1. Install a sentinel hook that the lifecycle hook will chain to.
    std::panic::set_hook(Box::new(|info| {
        // Printed only after the lifecycle hook's `restore_terminal()` has run.
        println!("{PREV_HOOK_MARKER}");
        eprintln!("panic: {info}");
    }));

    // 2. `enter()` installs the lifecycle panic hook on top of our sentinel. Its
    //    `enable_raw_mode()` fails on the non-TTY child stdout, so it returns
    //    `Err` — but the panic hook is installed *before* that call, which is the
    //    behaviour under test. We deliberately ignore the result.
    let _ = eddacraft_tui::lifecycle::TerminalGuard::enter();

    // 3. Panic. The chain is: lifecycle hook (restore_terminal) -> sentinel.
    panic!("{PANIC_MESSAGE}");
}

/// `restore_terminal()` is a best-effort, idempotent no-op off a real terminal:
/// safe to call any number of times without panicking or erroring out. This runs
/// in-process (no global-hook state involved).
#[test]
fn restore_terminal_is_safe_and_idempotent_without_a_tty() {
    eddacraft_tui::lifecycle::restore_terminal();
    eddacraft_tui::lifecycle::restore_terminal();
}
