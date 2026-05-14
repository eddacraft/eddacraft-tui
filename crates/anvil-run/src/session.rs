//! INTL-003: session identity + registration plumbing.
//!
//! The launcher mints a `SessionId` for each spawn, queries the
//! daemon to register the session keyed by `(WorktreeKey, AgentTag)`,
//! and threads the daemon-returned `AgentTag` back through to spawn
//! (INTL-004) so the child env carries the right `ANVIL_AGENT_TAG`.
//!
//! Until MLP-014 lands the daemon's `mint_agent_tag` path, the
//! daemon's `session.register` response only echoes the launcher's
//! request payload. This module handles both cases: if the daemon
//! returns an `agent_tag` object, we honour it; otherwise we
//! synthesise a launcher-local `AgentTag` from the same triple the
//! launcher sent. The trust model documented on `AgentTag` is
//! identical in either case — env propagation is advisory.

use std::path::Path;

use anvil_intercept_proto::SessionId;
use anvil_intercept_proto::session::AgentTag;
use anyhow::{Context, Result};
use serde_json::Value;

use crate::ipc;

/// Registration outcome for INTL-003.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub session_id: SessionId,
    pub agent_tag: AgentTag,
}

/// JSON-RPC method names for session.register / session.unregister.
/// The daemon accepts both `session.register` and the legacy
/// `register-session`; we use the namespaced form so a future driver
/// hash check can include the method name verbatim.
pub const REGISTER_METHOD: &str = "session.register";
pub const UNREGISTER_METHOD: &str = "session.unregister";
pub const HEARTBEAT_METHOD: &str = "heartbeat";

/// Generate a new opaque session id. Uses UUID v7 for the same
/// time-ordered property MLP-001 picked for `project_uuid` — useful
/// when an operator sorts the daemon's `list-sessions` output.
#[must_use]
pub fn new_session_id() -> SessionId {
    SessionId::new(format!("sess_{}", uuid::Uuid::now_v7().simple()))
}

/// Read the kernel's reported process start time for `pid` as
/// **Unix seconds since epoch** (boot_time + ticks/CLK_TCK). The
/// canonical implementation lives in
/// [`anvil_attribution::process::pid_starttime`]; using it here
/// keeps the launcher's `AgentTag` units identical to the daemon's
/// process-attribution helper so PID-reuse comparisons compare like
/// for like.
///
/// On non-Linux hosts the upstream helper returns
/// `ProcessInfoError::Io` with `ErrorKind::Unsupported`; the launcher
/// degrades to a wall-clock-derived value so the env-propagation
/// path still works. The daemon authenticates via the witness chain,
/// so the wall-clock fallback only weakens PID-reuse defence on
/// macOS / Windows.
#[must_use]
pub fn pid_starttime_or_fallback(pid: u32) -> u64 {
    if let Ok(value) = anvil_attribution::process::pid_starttime(pid) {
        return value;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Inputs for [`register`]. Threaded into the daemon's
/// `session.register` JSON-RPC method as a single struct so callers
/// cannot drop a field by accident.
pub struct RegistrationRequest<'a> {
    pub session_id: &'a SessionId,
    pub worktree: &'a Path,
    pub cwd: &'a Path,
    pub driver_id: &'a str,
    pub claimed_agent_id: &'a str,
    /// Provisional `pid_starttime` (launcher's own at register time —
    /// the child is reported separately after spawn via
    /// `session.report_process`). MLP-014 uses the post-spawn value
    /// for PID-reuse defence; the launcher value is a hint only.
    pub pid_starttime: u64,
    pub tmux_pane: Option<&'a str>,
}

/// Issue the `session.register` JSON-RPC request. The launcher
/// supplies `(session_id, worktree, cwd, driver_id, claimed_agent_id,
/// pid_starttime, tmux_pane?)`; the daemon may either:
///
/// - Echo back an explicit `{"agent_tag": {...}}` object — MLP-014
///   minted-tag path — in which case the launcher honours it.
/// - Echo only `{"session_id":..., "worktree":...}` — the pre-MLP-014
///   daemon — in which case the launcher synthesises an `AgentTag`
///   from its own inputs.
pub fn register(req: &RegistrationRequest<'_>) -> Result<Registration> {
    let params = ipc::session_register_params(
        req.session_id.as_str(),
        req.worktree,
        req.cwd,
        req.driver_id,
        req.claimed_agent_id,
        req.pid_starttime,
        req.tmux_pane,
    );
    let response: Value = ipc::request(
        REGISTER_METHOD,
        params,
        &format!("anvil-run-register-{}", req.session_id.as_str()),
    )?;
    let agent_tag = interpret_register_response(
        &response,
        req.driver_id,
        req.claimed_agent_id,
        req.pid_starttime,
    )
    .context("interpreting session.register response")?;
    Ok(Registration {
        session_id: req.session_id.clone(),
        agent_tag,
    })
}

/// Pure helper: derive the `AgentTag` from a daemon response. If the
/// daemon returned an explicit `agent_tag` object that parses, use
/// it; otherwise synthesise from the inputs the launcher sent.
pub fn interpret_register_response(
    response: &Value,
    driver_id: &str,
    claimed_agent_id: &str,
    pid_starttime: u64,
) -> Result<AgentTag> {
    if let Some(tag_value) = response.get("agent_tag")
        && !tag_value.is_null()
    {
        let tag: AgentTag = serde_json::from_value(tag_value.clone())
            .context("daemon returned an agent_tag that did not match the proto shape")?;
        return Ok(tag);
    }
    Ok(AgentTag::new(driver_id, claimed_agent_id, pid_starttime))
}

/// Issue the `session.unregister` JSON-RPC request. Used by the
/// cleanup drop guard (INTL-005). Errors are surfaced to the caller
/// so the cleanup path can decide whether to log or swallow.
pub fn unregister(session_id: &SessionId) -> Result<()> {
    let params = serde_json::json!({"session_id": session_id.as_str()});
    let _: Value = ipc::request(
        UNREGISTER_METHOD,
        params,
        &format!("anvil-run-unregister-{}", session_id.as_str()),
    )?;
    Ok(())
}

/// Send a single heartbeat for `session_id`. Notifications do not
/// expect a response.
pub fn heartbeat(session_id: &SessionId) -> Result<()> {
    let params = serde_json::json!({"session_id": session_id.as_str()});
    ipc::notify(HEARTBEAT_METHOD, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_id_uses_a_stable_prefix() {
        let id = new_session_id();
        assert!(
            id.as_str().starts_with("sess_"),
            "session ids must be prefixed for log grepping: {}",
            id.as_str(),
        );
        // 5 byte prefix + 32 byte simple-UUID v7 = 37 chars.
        assert_eq!(id.as_str().len(), 5 + 32);
    }

    #[test]
    fn new_session_ids_are_unique_back_to_back() {
        let a = new_session_id();
        let b = new_session_id();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn pid_starttime_or_fallback_is_non_zero() {
        // /proc/self/stat is always non-zero on Linux; on other
        // OSes the wall-clock fallback is non-zero too unless the
        // system clock is at the Unix epoch (it isn't).
        let value = pid_starttime_or_fallback(std::process::id());
        assert!(value > 0, "pid_starttime must be positive, got {value}");
    }

    #[test]
    fn interpret_response_honours_explicit_agent_tag_from_daemon() {
        let response = serde_json::json!({
            "session_id": "sess_abc",
            "worktree": "/tmp/wt",
            "agent_tag": {
                "driver_id": "anvil-run",
                "claimed_agent_id": "minted-by-daemon",
                "pid_starttime": 12345,
            },
        });
        let tag = interpret_register_response(&response, "anvil-run", "ignored", 99).unwrap();
        assert_eq!(tag.claimed_agent_id, "minted-by-daemon");
        assert_eq!(tag.pid_starttime, 12345);
    }

    #[test]
    fn interpret_response_falls_back_when_daemon_omits_agent_tag() {
        // Pre-MLP-014 daemon: only echoes session_id/worktree.
        let response = serde_json::json!({
            "session_id": "sess_abc",
            "worktree": "/tmp/wt",
        });
        let tag =
            interpret_register_response(&response, "anvil-run", "claude-1", 1_700_000_000).unwrap();
        assert_eq!(tag.driver_id, "anvil-run");
        assert_eq!(tag.claimed_agent_id, "claude-1");
        assert_eq!(tag.pid_starttime, 1_700_000_000);
    }

    #[test]
    fn interpret_response_treats_null_agent_tag_as_absent() {
        let response = serde_json::json!({
            "session_id": "sess_abc",
            "agent_tag": null,
        });
        let tag = interpret_register_response(&response, "anvil-run", "claude-1", 42).unwrap();
        assert_eq!(tag.pid_starttime, 42);
    }

    #[test]
    fn interpret_response_errors_on_a_malformed_agent_tag() {
        let response = serde_json::json!({
            "agent_tag": "not-an-object",
        });
        let err = interpret_register_response(&response, "anvil-run", "x", 0).unwrap_err();
        assert!(err.to_string().contains("agent_tag"));
    }
}
