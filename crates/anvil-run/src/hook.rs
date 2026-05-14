//! INTL-007: hook side-channel registration.
//!
//! Some agents do not start through `anvil-run` — Claude Code's
//! `PreToolUse` hook, for example, fires inside an already-running
//! agent process. For those flows the launcher exposes a
//! `hook register` subcommand that performs a degraded session
//! registration:
//!
//! - The calling PID is sent as-is; the launcher does not place
//!   the process in a fresh process group (it cannot — the agent
//!   is already running).
//! - The daemon caps the enforcement action for hook-registered
//!   sessions at fence-only, because the caller's PID belongs to
//!   the agent itself rather than a controlled wrapper PGID.
//! - The launcher reports `hook_registered: true` on the wire so
//!   the daemon can apply the cap at registration time.

use std::path::PathBuf;

use anvil_intercept_proto::SessionId;
use serde_json::Value;

use crate::cli::HookRegisterArgs;
use crate::context::{ContextError, LaunchContext};
use crate::ipc;
use crate::session::{REGISTER_METHOD, new_session_id};

/// Outcome of a successful hook registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRegistration {
    pub session_id: SessionId,
    pub worktree: PathBuf,
    pub pid: u32,
}

/// Typed failure modes for [`run_register`]. The caller maps each
/// variant to the right launcher exit code so an invalid `--cwd`
/// does not get reported as "daemon unavailable".
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    /// The `--pid 0` was explicitly supplied. Pre-existing planning
    /// contract documents `--pid 0` as invalid; reject loudly rather
    /// than silently falling back to the parent PID.
    #[error("--pid must be a positive integer (got 0)")]
    InvalidPid,
    /// The launcher could not determine the parent PID and the
    /// caller did not supply `--pid` explicitly.
    #[error("could not determine parent PID for hook registration; pass --pid")]
    ParentPidUnavailable,
    /// `--cwd` or context resolution failed before any daemon I/O.
    #[error("hook register: bad launch context: {0}")]
    BadContext(#[from] ContextError),
    /// Daemon IPC failed.
    #[error("hook register: daemon failure: {0}")]
    Daemon(#[source] anyhow::Error),
}

/// Run the hook subcommand. The function is small on purpose —
/// `main` calls it directly and exits on its result.
pub fn run_register(args: &HookRegisterArgs) -> Result<HookRegistration, HookError> {
    let pid = match args.pid {
        Some(0) => return Err(HookError::InvalidPid),
        Some(p) => p,
        None => parent_pid().ok_or(HookError::ParentPidUnavailable)?,
    };
    let ctx = LaunchContext::resolve(args.cwd.clone(), None)?;
    let session_id = new_session_id();
    let pgid = current_pgid_for(pid);
    let params = build_hook_register_params(&session_id, &ctx.worktree, &args.tool, pid, pgid);
    let _: Value = ipc::request(
        REGISTER_METHOD,
        params,
        &format!("anvil-run-hook-register-{}", session_id.as_str()),
    )
    .map_err(HookError::Daemon)?;
    Ok(HookRegistration {
        session_id,
        worktree: ctx.worktree,
        pid,
    })
}

/// Build the wire params for a hook-mode registration. The shape
/// extends the standard `session.register` with a
/// `hook_registered: true` flag so the daemon can downgrade
/// enforcement at registration time. `pgid` is `None` when the
/// launcher cannot read it (e.g. cross-namespace caller); the daemon
/// treats absence as "no controlled PGID — apply fence-only cap".
#[must_use]
pub fn build_hook_register_params(
    session_id: &SessionId,
    worktree: &std::path::Path,
    tool: &str,
    pid: u32,
    pgid: Option<i32>,
) -> Value {
    let mut params = serde_json::Map::new();
    params.insert(
        "session_id".into(),
        Value::String(session_id.as_str().to_owned()),
    );
    params.insert(
        "worktree".into(),
        Value::String(worktree.to_string_lossy().into_owned()),
    );
    params.insert("driver_id".into(), Value::String(tool.into()));
    params.insert("claimed_agent_id".into(), Value::String(tool.into()));
    params.insert("pid".into(), Value::Number(serde_json::Number::from(pid)));
    match pgid {
        Some(g) => {
            params.insert("pgid".into(), Value::Number(serde_json::Number::from(g)));
        }
        None => {
            params.insert("pgid".into(), Value::Null);
        }
    }
    params.insert("hook_registered".into(), Value::Bool(true));
    Value::Object(params)
}

#[cfg(unix)]
fn parent_pid() -> Option<u32> {
    use nix::unistd::getppid;
    u32::try_from(getppid().as_raw()).ok()
}

#[cfg(windows)]
fn parent_pid() -> Option<u32> {
    // Windows does not expose a portable getppid() in std. The
    // pre-MLP-014 contract is that the hook caller passes `--pid`
    // explicitly on Windows.
    None
}

#[cfg(unix)]
fn current_pgid_for(pid: u32) -> Option<i32> {
    use nix::unistd::{Pid, getpgid};
    let raw = i32::try_from(pid).ok()?;
    getpgid(Some(Pid::from_raw(raw))).ok().map(|p| p.as_raw())
}

#[cfg(windows)]
fn current_pgid_for(_pid: u32) -> Option<i32> {
    // Windows has no equivalent to a Unix process group; the daemon
    // applies the fence-only enforcement cap on hook registrations
    // unconditionally on Windows.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn hook_register_params_carry_the_downgrade_flag() {
        let params = build_hook_register_params(
            &SessionId::new("sess_h"),
            Path::new("/tmp/wt"),
            "claude-code",
            4242,
            Some(4242),
        );
        assert_eq!(params["session_id"], "sess_h");
        assert_eq!(params["worktree"], "/tmp/wt");
        assert_eq!(params["driver_id"], "claude-code");
        assert_eq!(params["pid"], 4242);
        assert_eq!(params["pgid"], 4242);
        assert_eq!(
            params["hook_registered"], true,
            "the daemon needs this flag to cap enforcement at fence-only",
        );
    }

    #[test]
    fn hook_register_params_record_unknown_pgid_as_null() {
        // Hook-mode on Windows (and unusual /proc states on Unix) may
        // not have a PGID to report. The daemon distinguishes
        // "no PGID known" (null) from a real numeric PGID so the
        // fence-only enforcement cap is applied unconditionally.
        let params = build_hook_register_params(
            &SessionId::new("sess_h"),
            Path::new("/tmp/wt"),
            "claude-code",
            4242,
            None,
        );
        assert!(params["pgid"].is_null(), "pgid must be null when unknown");
    }

    #[test]
    fn hook_register_params_identify_tool_as_both_driver_and_agent() {
        // Hook-mode does not get a separate claimed_agent_id — the
        // tool name doubles for it. Pin this so a future refactor
        // does not silently break the daemon's per-driver policy.
        let params = build_hook_register_params(
            &SessionId::new("sess_h"),
            Path::new("/tmp/wt"),
            "claude-code",
            1,
            Some(1),
        );
        assert_eq!(params["driver_id"], params["claimed_agent_id"]);
    }

    /// Pre-existing contract: `--pid 0` is invalid. The launcher
    /// must reject it rather than silently falling back to the
    /// parent PID, which could register the wrong process.
    #[test]
    fn explicit_pid_zero_is_rejected() {
        let args = HookRegisterArgs {
            tool: "claude-code".into(),
            cwd: None,
            pid: Some(0),
        };
        let err = run_register(&args).expect_err("pid 0 must error");
        assert!(matches!(err, HookError::InvalidPid));
    }
}
