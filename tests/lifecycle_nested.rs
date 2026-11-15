//! Nested terminal guards share the process's terminal (GH #6557).
//!
//! Raw mode, the alternate screen and mouse capture belong to the process, not
//! to one guard. Dropping or leaving an inner guard must leave those modes on
//! while an outer guard still holds them; only the last holder restores them.
//!
//! The test binary re-runs itself under `script(1)` so the child has a real
//! terminal: `TerminalGuard::enter()` needs one for raw mode, and the escapes
//! the guards write are captured from the pseudo-terminal. Markers printed
//! between steps split that output so each step's escapes can be checked on
//! their own. Without `script` the test reports that it skipped and passes.
#![cfg(all(unix, feature = "lifecycle"))]

use std::io::Write;
use std::process::Command;

use crossterm::terminal;
use eddacraft_tui::lifecycle::TerminalGuard;

const CHILD: &str = "EDDACRAFT_TUI_NESTED_GUARD_CHILD";

/// Leaves the alternate screen (`LeaveAlternateScreen`).
const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";
/// Turns normal mouse tracking off; the first escape `DisableMouseCapture` writes.
#[cfg(feature = "flow")]
const DISABLE_MOUSE: &str = "\x1b[?1000l";

fn mark(name: &str) {
    let mut stdout = std::io::stdout();
    write!(stdout, "<{name}>\r\n").expect("marker");
    stdout.flush().expect("flush marker");
}

fn mark_raw(name: &str) {
    let raw = terminal::is_raw_mode_enabled().expect("read raw mode");
    mark(&format!("{name}:raw={raw}"));
}

/// The child half: nests guards when the parent launched it under `script`.
#[test]
fn nested_guard_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }

    let outer = TerminalGuard::enter().expect("outer guard");
    let inner = TerminalGuard::enter().expect("inner guard");
    mark_raw("both-held");
    drop(inner);
    mark_raw("inner-dropped");

    let inner = TerminalGuard::enter().expect("second inner guard");
    inner.leave().expect("inner leave");
    mark_raw("inner-left");

    #[cfg(feature = "flow")]
    {
        use eddacraft_tui::flow::{FlowSession, MouseCaptureGuard};

        let outer_mouse = FlowSession::enter();
        let inner_mouse = MouseCaptureGuard::enable();
        mark("mouse-held");
        drop(inner_mouse);
        mark("mouse-inner-dropped");
        drop(outer_mouse);
        mark("mouse-outer-dropped");
    }

    outer.leave().expect("outer leave");
    mark_raw("outer-left");
}

/// Runs the child under `script`, returning its output, or `None` when
/// `script` is not installed.
fn run_under_pty() -> Option<String> {
    let exe = std::env::current_exe().expect("test binary");
    // Single-quoted for `sh -c`, so a target path with spaces still runs.
    let exe = format!("'{}'", exe.display().to_string().replace('\'', r"'\''"));
    let child = format!("{exe} --exact nested_guard_child --nocapture --test-threads=1");
    let mut command = Command::new("script");
    if cfg!(target_os = "macos") {
        command.args(["-q", "/dev/null", "/bin/sh", "-c", &child]);
    } else {
        command.args(["-q", "-e", "-c", &child, "/dev/null"]);
    }
    command.env(CHILD, "1").env("TERM", "xterm-256color");
    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => panic!("spawn script: {error}"),
    };
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "child failed under script: {text:?}"
    );
    Some(text)
}

/// The output between the end of marker `from` and the start of marker `to`.
fn between<'a>(output: &'a str, from: &str, to: &str) -> &'a str {
    let start = output
        .find(from)
        .unwrap_or_else(|| panic!("marker {from} missing\n{output:?}"))
        + from.len();
    let end = output[start..]
        .find(to)
        .unwrap_or_else(|| panic!("marker {to} missing after {from}\n{output:?}"))
        + start;
    &output[start..end]
}

#[test]
fn an_inner_guard_does_not_tear_down_the_outer_session() {
    let Some(output) = run_under_pty() else {
        eprintln!("script(1) not found; skipped the pseudo-terminal run");
        return;
    };

    assert!(
        output.contains("<both-held:raw=true>"),
        "raw mode should be on while both guards are held\n{output:?}"
    );
    assert!(
        output.contains("<inner-dropped:raw=true>"),
        "dropping the inner guard disabled raw mode under the outer guard\n{output:?}"
    );
    assert!(
        !between(&output, "<both-held:raw=true>", "<inner-dropped").contains(LEAVE_ALT_SCREEN),
        "dropping the inner guard left the alternate screen under the outer guard\n{output:?}"
    );
    assert!(
        output.contains("<inner-left:raw=true>"),
        "leaving the inner guard disabled raw mode under the outer guard\n{output:?}"
    );
    assert!(
        !between(&output, "<inner-dropped:raw=true>", "<inner-left").contains(LEAVE_ALT_SCREEN),
        "leaving the inner guard left the alternate screen under the outer guard\n{output:?}"
    );

    #[cfg(feature = "flow")]
    {
        assert!(
            !between(&output, "<mouse-held>", "<mouse-inner-dropped>").contains(DISABLE_MOUSE),
            "dropping the inner mouse guard turned capture off under the outer session\n{output:?}"
        );
        assert!(
            between(&output, "<mouse-inner-dropped>", "<mouse-outer-dropped>")
                .contains(DISABLE_MOUSE),
            "the last mouse guard did not turn capture off\n{output:?}"
        );
    }

    // The last guard still restores everything.
    assert!(
        output.contains("<outer-left:raw=false>"),
        "the last guard did not disable raw mode\n{output:?}"
    );
    assert!(
        output
            .rsplit_once("<inner-left:raw=true>")
            .is_some_and(|(_, tail)| tail.contains(LEAVE_ALT_SCREEN)),
        "the last guard did not leave the alternate screen\n{output:?}"
    );
}
