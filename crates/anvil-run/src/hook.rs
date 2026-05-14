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
use anyhow::{Context, Result};
use serde_json::Value;

use crate::cli::HookRegisterArgs;
use crate::context::LaunchContext;
use crate::ipc;
use crate::session::{REGISTER_METHOD, new_session_id};

/// Outcome of a successful hook registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRegistration {
    pub session_id: SessionId,
    pub worktree: PathBuf,
    pub pid: u32,
}

/// Run the hook subcommand. The function is small on purpose —
/// `main` calls it directly and exits on its result.
pub fn run_register(args: &HookRegisterArgs) -> Result<HookRegistration> {
    let pid = match args.pid {
        Some(p) if p > 0 => p,
        Some(_) | None => parent_pid()?,
    };
    let ctx = LaunchContext::resolve(args.cwd.clone(), None)
        .context("hook register: resolving launch context")?;
    let session_id = new_session_id();
    let params = build_hook_register_params(&session_id, &ctx.worktree, &args.tool, pid);
    let _: Value = ipc::request(
        REGISTER_METHOD,
        params,
        &format!("anvil-run-hook-register-{}", session_id.as_str()),
    )?;
    Ok(HookRegistration {
        session_id,
        worktree: ctx.worktree,
        pid,
    })
}

/// Build the wire params for a hook-mode registration. The shape
/// extends the standard `session.register` with a
/// `hook_registered: true` flag so the daemon can downgrade
/// enforcement at registration time.
#[must_use]
pub fn build_hook_register_params(
    session_id: &SessionId,
    worktree: &std::path::Path,
    tool: &str,
    pid: u32,
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
    params.insert("hook_registered".into(), Value::Bool(true));
    Value::Object(params)
}

#[cfg(unix)]
fn parent_pid() -> Result<u32> {
    use nix::unistd::getppid;
    let ppid = getppid().as_raw();
    u32::try_from(ppid)
        .map_err(|_| anyhow::anyhow!("could not determine parent PID for hook registration"))
}

#[cfg(windows)]
fn parent_pid() -> Result<u32> {
    // Windows does not expose a portable getppid() in std. The
    // pre-MLP-014 contract is that the hook caller passes `--pid`
    // explicitly on Windows; we surface a clear error so the
    // operator knows they must.
    anyhow::bail!("on Windows, `anvil-run hook register` requires --pid to identify the caller");
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
        );
        assert_eq!(params["session_id"], "sess_h");
        assert_eq!(params["worktree"], "/tmp/wt");
        assert_eq!(params["driver_id"], "claude-code");
        assert_eq!(params["pid"], 4242);
        assert_eq!(
            params["hook_registered"], true,
            "the daemon needs this flag to cap enforcement at fence-only",
        );
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
        );
        assert_eq!(params["driver_id"], params["claimed_agent_id"]);
    }
}
