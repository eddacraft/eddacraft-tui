//! Process-tree ancestor walk.
//!
//! When `ANVIL_AGENT_TAG` is absent from the current process's env,
//! the daemon walks upward through `getppid`-equivalent reads until it
//! finds a registered ancestor or runs out of parents. This module
//! ships the walk; the registry lookup itself is a caller-supplied
//! closure so this crate stays registry-agnostic.

use thiserror::Error;

use crate::process::{ProcessInfoError, parent_pid};

/// Default depth cap on the walk. Defends against malformed parent
/// chains and degenerate process trees; the real Linux kernel won't
/// produce cycles, but defending costs us one branch per step.
pub const DEFAULT_MAX_DEPTH: usize = 64;

/// Outcome of [`walk_ancestors`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkOutcome<T> {
    /// The visitor matched at the given pid; walk halted with the
    /// caller-supplied payload.
    Matched {
        /// PID at which the visitor returned a match.
        pid: u32,
        /// Caller-supplied payload from the matching visitor return.
        value: T,
    },

    /// Walk terminated by reaching init (PID 0 reported as parent,
    /// or PID 1 itself).
    ReachedRoot,

    /// Walk hit `max_depth` without matching. The deepest visited PID
    /// is reported so callers can log it.
    DepthExhausted {
        /// Last PID visited before the depth limit fired.
        deepest_pid: u32,
    },
}

/// Failures from [`walk_ancestors`]. Distinct from
/// [`WalkOutcome::ReachedRoot`] — those are normal terminations.
#[derive(Debug, Error)]
pub enum WalkError {
    /// Reading `/proc/<pid>/stat` for some ancestor failed. The walk
    /// halts at the first read error so callers see the exact PID
    /// that caused the abort.
    #[error("could not read process info for pid {pid}: {source}")]
    ProcessInfo {
        /// PID whose lookup failed.
        pid: u32,
        /// Underlying read error.
        #[source]
        source: ProcessInfoError,
    },
}

/// Walk from `start_pid` toward init, invoking `visit` at each step
/// until it returns `Some(value)`, the walk reaches init, or it
/// exceeds `max_depth`.
///
/// The starting PID is itself passed to `visit` first; that's the
/// behaviour callers want when the daemon is asked "is the current
/// process attributable?" — the answer should account for the current
/// process being a registered session, not only its ancestors.
pub fn walk_ancestors<F, T>(
    start_pid: u32,
    max_depth: usize,
    mut visit: F,
) -> Result<WalkOutcome<T>, WalkError>
where
    F: FnMut(u32) -> Option<T>,
{
    let mut pid = start_pid;
    // Track the deepest PID we actually invoked the visitor against;
    // `pid` itself is advanced to the *next* ancestor at the end of
    // each iteration, so reporting `pid` directly on exhaustion can
    // name an unvisited ancestor (e.g. `max_depth = 1` would report
    // the parent even though only `start_pid` ran). `last_visited`
    // is the contract-correct value for `DepthExhausted.deepest_pid`.
    let mut last_visited = start_pid;

    for _step in 0..max_depth {
        last_visited = pid;
        if let Some(value) = visit(pid) {
            return Ok(WalkOutcome::Matched { pid, value });
        }

        // PID 1 has itself as parent on Linux; rather than rely on
        // that quirk we treat PID 1 as the explicit walk terminator.
        if pid == 1 {
            return Ok(WalkOutcome::ReachedRoot);
        }

        match parent_pid(pid).map_err(|source| WalkError::ProcessInfo { pid, source })? {
            None => return Ok(WalkOutcome::ReachedRoot),
            Some(next) => {
                if next == pid {
                    // Defensive: a stat file reporting itself as its
                    // own parent would loop forever. Treat as root.
                    return Ok(WalkOutcome::ReachedRoot);
                }
                pid = next;
            }
        }
    }

    Ok(WalkOutcome::DepthExhausted {
        deepest_pid: last_visited,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_at_start_pid() {
        let result = walk_ancestors(42, DEFAULT_MAX_DEPTH, |pid| {
            if pid == 42 { Some("hit") } else { None }
        });
        assert!(
            matches!(
                result,
                Ok(WalkOutcome::Matched {
                    pid: 42,
                    value: "hit"
                })
            ),
            "got {result:?}"
        );
    }

    #[test]
    fn depth_zero_returns_exhausted_without_visiting() {
        let mut visited = Vec::new();
        let result = walk_ancestors(42, 0, |pid| {
            visited.push(pid);
            None::<()>
        });
        assert!(visited.is_empty(), "visitor should not run with depth 0");
        assert!(
            matches!(result, Ok(WalkOutcome::DepthExhausted { deepest_pid: 42 })),
            "got {result:?}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn walking_from_self_reaches_root_without_matching() {
        // Use a visitor that always returns None, so the walk must
        // climb all the way to init and report ReachedRoot.
        let result = walk_ancestors(std::process::id(), DEFAULT_MAX_DEPTH, |_pid| None::<()>)
            .expect("walk should succeed against live processes");
        assert!(
            matches!(result, WalkOutcome::ReachedRoot),
            "expected ReachedRoot, got {result:?}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn walking_from_self_visits_self_first() {
        let self_pid = std::process::id();
        let mut visited = Vec::new();
        let _ = walk_ancestors(self_pid, DEFAULT_MAX_DEPTH, |pid| {
            visited.push(pid);
            None::<()>
        });
        assert_eq!(visited.first().copied(), Some(self_pid));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn matching_at_self_short_circuits_walk() {
        let self_pid = std::process::id();
        let mut visited = Vec::new();
        let result = walk_ancestors(self_pid, DEFAULT_MAX_DEPTH, |pid| {
            visited.push(pid);
            (pid == self_pid).then_some("self")
        })
        .expect("walk");

        assert_eq!(visited.len(), 1, "should not climb past self");
        assert!(
            matches!(result, WalkOutcome::Matched { pid, value: "self" } if pid == self_pid),
            "got {result:?}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn nonexistent_pid_returns_process_info_error() {
        let err = walk_ancestors(4_000_000_000, DEFAULT_MAX_DEPTH, |_| None::<()>).unwrap_err();
        match err {
            WalkError::ProcessInfo { pid, .. } => {
                assert_eq!(pid, 4_000_000_000);
            }
        }
    }
}
