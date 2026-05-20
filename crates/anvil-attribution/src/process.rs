//! Linux `/proc/<pid>/stat` introspection.
//!
//! The kernel's [`proc(5)`] page describes `/proc/<pid>/stat` as a
//! single space-separated line of 52+ fields. The second field
//! (`comm`) is the executable's basename wrapped in parens and can
//! itself contain spaces and parens. The canonical parsing trick is to
//! scan for the LAST `)` in the line and split everything after that
//! by ASCII whitespace; this avoids any string-escape edge case in the
//! comm field. We mirror that approach here.
//!
//! [`proc(5)`]: https://man7.org/linux/man-pages/man5/proc.5.html
//!
//! ## Why we parse manually rather than depend on `procfs`
//!
//! `procfs` would pull in a heavy transitive tree (chrono, byteorder,
//! flate2, …) for two integer fields. The kernel's format is stable
//! enough that two careful field accesses by position are robust
//! through any kernel version that still names the file
//! `/proc/<pid>/stat`.
//!
//! ## Other platforms
//!
//! macOS exposes the equivalent through `sysctl kern.proc.pid.<pid>`
//! and Windows through `GetProcessTimes`. v1 is Linux-only — callers
//! get [`io::ErrorKind::Unsupported`] on other platforms so they can
//! degrade attribution gracefully (the trust-model already permits a
//! missing `pid_starttime` to downgrade a session to worktree-level
//! fence per ADR-038 noise discipline).

#[cfg(target_os = "linux")]
use std::fs;
use std::io;

use thiserror::Error;

/// Conservative fallback for `clock(3)` `CLK_TCK` if `sysconf(3)`
/// fails or reports a non-positive value. The kernel default has
/// been 100 for two decades, but `CONFIG_HZ_250` / `CONFIG_HZ_1000`
/// build configurations exist (notably on some Debian / Arch kernels
/// and on real-time-tuned hosts), so the runtime path queries
/// `_SC_CLK_TCK` first and only falls back to this constant when the
/// syscall fails.
#[cfg(target_os = "linux")]
const FALLBACK_CLK_TCK: u64 = 100;

#[cfg(target_os = "linux")]
fn clk_tck() -> u64 {
    // `nix::unistd::sysconf` is the safe wrapper around `sysconf(3)`;
    // workspace policy forbids the raw `unsafe { libc::sysconf(...) }`
    // call. `_SC_CLK_TCK` reports the kernel's `USER_HZ` configuration
    // (commonly 100, 250, or 1000 on Linux). On any failure path
    // (Err, Ok(None), or non-positive value) we fall back to 100 —
    // the kernel default for two decades — so attribution still
    // works, just with the historical units.
    match nix::unistd::sysconf(nix::unistd::SysconfVar::CLK_TCK) {
        Ok(Some(raw)) if raw > 0 => u64::try_from(raw).unwrap_or(FALLBACK_CLK_TCK),
        _ => FALLBACK_CLK_TCK,
    }
}

/// Errors from [`pid_starttime`] / [`parent_pid`]. Wraps `io::Error`
/// so callers see file-not-found vs. malformed-content separately.
#[derive(Debug, Error)]
pub enum ProcessInfoError {
    /// `/proc/<pid>/stat` (or `/proc/stat` for boot time) could not be
    /// read — typically because the process no longer exists.
    #[error("could not read {path}: {source}")]
    Io {
        /// Which procfs entry failed to read.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// The stat file was readable but did not match the expected
    /// layout — either fewer fields than required after the comm
    /// closing paren, or a field that failed to parse as the expected
    /// integer type.
    #[error("could not parse {path}: {reason}")]
    Malformed {
        /// Which procfs entry failed to parse.
        path: String,
        /// Free-form reason text.
        reason: String,
    },
}

/// Linux-only: process start time in Unix seconds since epoch.
///
/// On non-Linux platforms this returns
/// [`ProcessInfoError::Io`] wrapping
/// [`io::ErrorKind::Unsupported`]; callers should treat that as
/// "attribution unavailable" rather than as a fatal error.
pub fn pid_starttime(pid: u32) -> Result<u64, ProcessInfoError> {
    #[cfg(target_os = "linux")]
    {
        pid_starttime_linux(pid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        Err(ProcessInfoError::Io {
            path: format!("/proc/{}/stat", pid),
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "pid_starttime is Linux-only in v1",
            ),
        })
    }
}

/// Linux-only: parent process id for `pid`, or `None` if the parent
/// field is `0` (only init itself).
pub fn parent_pid(pid: u32) -> Result<Option<u32>, ProcessInfoError> {
    #[cfg(target_os = "linux")]
    {
        parent_pid_linux(pid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        Err(ProcessInfoError::Io {
            path: format!("/proc/{}/stat", pid),
            source: io::Error::new(io::ErrorKind::Unsupported, "parent_pid is Linux-only in v1"),
        })
    }
}

#[cfg(target_os = "linux")]
fn pid_starttime_linux(pid: u32) -> Result<u64, ProcessInfoError> {
    let path = format!("/proc/{pid}/stat");
    let raw = fs::read_to_string(&path).map_err(|source| ProcessInfoError::Io {
        path: path.clone(),
        source,
    })?;

    let fields = stat_fields_after_comm(&raw, &path)?;

    // Field index after comm:
    //   0=state, 1=ppid, 2=pgrp, 3=session, 4=tty_nr, 5=tpgid,
    //   6=flags, 7=minflt, 8=cminflt, 9=majflt, 10=cmajflt,
    //   11=utime, 12=stime, 13=cutime, 14=cstime, 15=priority,
    //   16=nice, 17=num_threads, 18=itrealvalue,
    //   19=starttime (in clock ticks since boot)
    let starttime_ticks: u64 = fields
        .get(19)
        .copied()
        .ok_or_else(|| ProcessInfoError::Malformed {
            path: path.clone(),
            reason: "expected at least 20 fields after comm".into(),
        })?
        .parse()
        .map_err(|e| ProcessInfoError::Malformed {
            path: path.clone(),
            reason: format!("starttime field not an integer: {e}"),
        })?;

    let boot_time = read_boot_time_seconds()?;
    Ok(boot_time + starttime_ticks / clk_tck())
}

#[cfg(target_os = "linux")]
fn parent_pid_linux(pid: u32) -> Result<Option<u32>, ProcessInfoError> {
    let path = format!("/proc/{pid}/stat");
    let raw = fs::read_to_string(&path).map_err(|source| ProcessInfoError::Io {
        path: path.clone(),
        source,
    })?;

    let fields = stat_fields_after_comm(&raw, &path)?;

    let parent: u32 = fields
        .get(1)
        .copied()
        .ok_or_else(|| ProcessInfoError::Malformed {
            path: path.clone(),
            reason: "expected at least 2 fields after comm".into(),
        })?
        .parse()
        .map_err(|e| ProcessInfoError::Malformed {
            path: path.clone(),
            reason: format!("ppid field not an integer: {e}"),
        })?;

    Ok(if parent == 0 { None } else { Some(parent) })
}

#[cfg(target_os = "linux")]
fn stat_fields_after_comm<'a>(raw: &'a str, path: &str) -> Result<Vec<&'a str>, ProcessInfoError> {
    let close_paren = raw.rfind(')').ok_or_else(|| ProcessInfoError::Malformed {
        path: path.into(),
        reason: "missing closing paren after comm field".into(),
    })?;

    let tail = &raw[close_paren + 1..];
    Ok(tail.split_ascii_whitespace().collect())
}

#[cfg(target_os = "linux")]
fn read_boot_time_seconds() -> Result<u64, ProcessInfoError> {
    let path = "/proc/stat";
    let raw = fs::read_to_string(path).map_err(|source| ProcessInfoError::Io {
        path: path.into(),
        source,
    })?;

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("btime ") {
            return rest
                .trim()
                .parse::<u64>()
                .map_err(|e| ProcessInfoError::Malformed {
                    path: path.into(),
                    reason: format!("btime not an integer: {e}"),
                });
        }
    }

    Err(ProcessInfoError::Malformed {
        path: path.into(),
        reason: "no 'btime' line found".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn self_starttime_is_within_sensible_window() {
        let pid = std::process::id();
        let starttime = pid_starttime(pid).expect("read self starttime");

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_secs();

        // Process started no later than "now" (defensive: allow 60s
        // clock skew either way for VMs / CI weirdness).
        assert!(
            starttime <= now_secs + 60,
            "starttime {starttime} should be <= now {now_secs} + 60"
        );

        // Process started after the unix epoch + some sanity floor
        // (year 2000). If this fails, /proc/stat returned garbage.
        assert!(
            starttime > 946_684_800,
            "starttime {starttime} should be after 2000-01-01"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn self_parent_pid_is_some_for_non_init_processes() {
        let pid = std::process::id();
        let parent = parent_pid(pid).expect("read self parent pid");
        assert!(
            parent.is_some(),
            "non-init test process should have a parent pid, got None"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn nonexistent_pid_returns_io_error() {
        // PID 1 will exist; pick a pid that's vanishingly unlikely
        // to be active (4_000_000_000 is above typical
        // /proc/sys/kernel/pid_max). If the test ever flakes because
        // the box really has a process with that pid, accept it as a
        // false negative — the surface contract is io-error or ok.
        let err = pid_starttime(4_000_000_000).unwrap_err();
        match err {
            ProcessInfoError::Io { .. } => {}
            ProcessInfoError::Malformed { .. } => {
                panic!("expected Io error for nonexistent pid, got Malformed")
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn init_starttime_is_earliest() {
        // PID 1 starts before any other PID on a Linux system, so its
        // starttime must be <= self's starttime. This guards against
        // a regression where we accidentally read a per-pid time
        // instead of an absolute time.
        let self_pid = std::process::id();
        let self_t = pid_starttime(self_pid).expect("self starttime");
        let init_t = pid_starttime(1).expect("init starttime");
        assert!(
            init_t <= self_t,
            "init starttime {init_t} should be <= self starttime {self_t}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn stat_fields_after_comm_handles_paren_in_comm() {
        // Synthesise a stat-style line where comm contains a closing
        // paren. The last ')' marks the real boundary.
        let raw = "1234 (weird ) name) S 99 100 0 0 0";
        let fields = stat_fields_after_comm(raw, "synthetic").expect("split");
        assert_eq!(fields[0], "S", "state field");
        assert_eq!(fields[1], "99", "ppid field");
    }
}
