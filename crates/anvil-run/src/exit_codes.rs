//! Stable exit codes for `anvil-run` (INTL-008).
//!
//! Shell wrappers and CI harnesses switch on these to distinguish
//! "the wrapped command failed" from "the launcher itself refused to
//! start the command, and here is why". Adding a new code is
//! additive; renumbering an existing one is a breaking change for
//! every wrapper that compares against it.
//!
//! ```text
//!   0      child exited 0
//!   1..127 child exited with that status (forwarded)
//!  128+N   child terminated by signal N (Unix convention)
//!  64      USAGE         — bad CLI input
//!  69      UNAVAILABLE   — daemon could not be reached / handshake failed
//!  73      CANT_CREATE   — spawn or registration failed in a recoverable way
//!  75      TEMP_FAIL     — fence is active, retry later
//!  78      CONFIG        — required env / configuration missing
//! ```
//!
//! The 64/69/73/75/78 values are the BSD `sysexits.h` codes — common
//! enough that operators recognise them, and most real tools never
//! exit with values in that range. They DO overlap with forwarded
//! child statuses in principle: a wrapped tool exiting 69 is
//! indistinguishable from "daemon unavailable" looking at the exit
//! code alone. A wrapper that must tell the two apart looks for the
//! launcher's *refusal banner* on stderr specifically: the child
//! inherits stderr too (`spawn::build_command` sets `Stdio::inherit`),
//! so stderr content as such is not launcher-owned — but the banner
//! is distinctive, and a refusal is emitted before any child spawns,
//! so when the launcher refuses, no child output is interleaved with
//! it. Pure exit-code switching is good enough for the common case;
//! matching the banner covers the rest.
//!
//! (A `$ANVIL_RUN_REFUSED` env signal was once documented here, but
//! the wrapper could only set it by inspecting the exit code — which
//! cannot distinguish a launcher refusal from a forwarded child
//! status in the overlapping `64..=78` range (these fall within the
//! `1..127` forwarded-child band in the table above), so it would
//! carry the same ambiguity it claimed to resolve. Matching the
//! launcher's refusal banner on stderr is the practical
//! disambiguator. See GH #1707 / Council C-027.)

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
