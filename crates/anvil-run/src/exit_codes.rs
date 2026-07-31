//! Stable CLI/hook process exit codes and outcome mapping.

/// Bad CLI usage (e.g. missing `--tool` or `--`).
pub const EXIT_USAGE: i32 = 64;

/// Daemon socket / pipe could not be reached, or the handshake
/// failed. Distinct from `EXIT_FENCED` so shells can decide whether
/// to suggest `anvil intercept start` versus `anvil intercept unblock`.
pub const EXIT_DAEMON_UNAVAILABLE: i32 = 69;

/// Spawn / registration failed for a reason that is not "fenced"
/// and not "daemon down" — e.g. the command name does not exist on
/// `$PATH`, or the daemon returned an unexpected JSON-RPC error.
pub const EXIT_SPAWN_FAILED: i32 = 73;

/// Worktree is currently fenced. The launcher refuses to start the
/// command; the wrapped UX prints the unblock instructions on
/// stderr (INTL-008).
pub const EXIT_FENCED: i32 = 75;

/// Required configuration is missing — e.g. `--tool` resolves to no
/// driver id or `--worktree` does not exist.
pub const EXIT_BAD_CONFIG: i32 = 78;

/// Translate a child's [`std::process::ExitStatus`] into a launcher
/// exit code. On Unix, signals map to `128 + signo`; on Windows the
/// raw exit code is forwarded modulo 256 so the shell parent sees
/// the same byte the OS exposed.
#[must_use]
pub fn forward_child_status(status: std::process::ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(code) = status.code() {
            code
        } else if let Some(signal) = status.signal() {
            128 + signal
        } else {
            // No signal and no code: defensive default. The Unix
            // process model guarantees one or the other so this
            // branch is effectively unreachable, but returning a
            // sentinel keeps the function total.
            128
        }
    }
    #[cfg(windows)]
    {
        status.code().unwrap_or(1)
    }
    #[cfg(not(any(unix, windows)))]
    {
        status.code().unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codes must not collide. A second test that catches the case
    /// where two constants get the same literal — easy to drift in
    /// a one-line edit.
    #[test]
    fn exit_codes_are_distinct() {
        let codes = [
            EXIT_USAGE,
            EXIT_DAEMON_UNAVAILABLE,
            EXIT_SPAWN_FAILED,
            EXIT_FENCED,
            EXIT_BAD_CONFIG,
        ];
        let set: std::collections::HashSet<i32> = codes.iter().copied().collect();
        assert_eq!(set.len(), codes.len(), "exit code collision in {codes:?}");
    }

    /// The codes must stay outside the child-status range so shell
    /// wrappers can distinguish "child failed with N" from
    /// "launcher refused with N". The BSD codes happily clear 63.
    #[test]
    fn launcher_codes_are_above_typical_child_range() {
        for code in [
            EXIT_USAGE,
            EXIT_DAEMON_UNAVAILABLE,
            EXIT_SPAWN_FAILED,
            EXIT_FENCED,
            EXIT_BAD_CONFIG,
        ] {
            assert!(
                code >= 64,
                "launcher exit codes must clear the BSD sysexits floor (64); got {code}",
            );
        }
    }
}
