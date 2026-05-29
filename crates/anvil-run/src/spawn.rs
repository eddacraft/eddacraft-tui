//! INTL-004: launch the wrapped command in its own process group
//! (Unix) or under a deterministic Job Object name (Windows), and
//! inject the `ANVIL_TASK_ID` / `ANVIL_AGENT_TAG` env vars before
//! exec.
//!
//! ## Process-group strategy
//!
//! - **Unix:** Set `setpgid(child, child)` via
//!   [`std::os::unix::process::CommandExt::process_group`]. The
//!   daemon then targets `kill(-pgid, signum)` to interrupt the
//!   whole agent process tree.
//! - **Windows:** A Job Object is created by the daemon-side
//!   `INTD-006` logic; the launcher only supplies a deterministic
//!   name derived from the session id so the daemon can `OpenJobObject`
//!   independently. Per the INTL contract, the launcher MUST NOT
//!   hand the daemon a raw HANDLE because handles are not
//!   cross-process.
//!
//! ## Env injection
//!
//! `ANVIL_TASK_ID` and `ANVIL_AGENT_TAG` are written onto the child
//! environment before the OS exec runs. The values are advisory
//! (see [`anvil_intercept_proto::session`]) so the daemon never
//! treats them as proof; the witness chain is the authentication
//! backstop.

use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use anvil_attribution::env::set_attribution_env;
use anvil_intercept_proto::SessionId;
use anvil_intercept_proto::session::AgentTag;
use anyhow::{Context, Result};
use serde_json::Value;

use crate::ipc;

/// Outcome of the spawn: the live child plus the metadata the
/// daemon needs to take ownership of the process group / Job Object.
pub struct SpawnedChild {
    /// The OS process handle. `wait()` blocks until exit.
    pub child: std::process::Child,
    /// PID reported by the OS at spawn time.
    pub pid: u32,
    /// Unix process-group id (equals PID, since the launcher sets
    /// the child as the leader). `None` on Windows.
    pub pgid: Option<i32>,
    /// Windows Job Object name. `None` on Unix.
    pub job_object_name: Option<String>,
}

/// Configure a [`Command`] for an Anvil-managed launch.
///
/// `cwd` and the wrapped command/args are caller-supplied; everything
/// else (env, process-group flag, Job Object naming) is owned here
/// so callers cannot accidentally bypass the contract.
pub fn build_command(
    program: &str,
    program_args: &[String],
    cwd: &Path,
    session_id: &SessionId,
    agent_tag: &AgentTag,
) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(program_args);
    cmd.current_dir(cwd);
    apply_env_propagation(&mut cmd, session_id, agent_tag);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    apply_process_group(&mut cmd);
    cmd
}

/// Spawn `cmd` and report the captured metadata.
pub fn spawn(mut cmd: Command, session_id: &SessionId) -> Result<SpawnedChild> {
    let child = cmd.spawn().context("failed to spawn the wrapped command")?;
    let child_pid = child.id();
    let group_id = pgid_for(child_pid);
    let job_object_name = if cfg!(windows) {
        Some(job_object_name_for(session_id))
    } else {
        None
    };
    Ok(SpawnedChild {
        child,
        pid: child_pid,
        pgid: group_id,
        job_object_name,
    })
}

/// Tell the daemon about the new child. The launcher reports
/// (`pid`, `pgid`, `pid_starttime`, `job_object_name`) so the daemon
/// can target signals (Unix) or open the `JobObject` (Windows).
///
/// The `pid_starttime` reported here MUST be the child's, not the
/// launcher's — that is what MLP-014 stores against the registration
/// for PID-reuse defence. We capture it from
/// [`anvil_attribution::process::pid_starttime`] after spawn so the
/// daemon's stored value can match the running child.
pub fn report_to_daemon(session_id: &SessionId, spawned: &SpawnedChild) -> Result<()> {
    let child_starttime = crate::session::pid_starttime_or_fallback(spawned.pid);
    let params = serde_json::json!({
        "session_id": session_id.as_str(),
        "pid": spawned.pid,
        "pgid": spawned.pgid,
        "pid_starttime": child_starttime,
        "job_object_name": spawned.job_object_name,
    });
    let _: Value = ipc::request(
        "session.report_process",
        &params,
        &format!("anvil-run-report-{}", session_id.as_str()),
    )
    .or_else(|err| {
        // The daemon may not yet implement this method (it lands
        // with INTD's MLP-014-aligned work). Treat method-not-found
        // as a soft warning rather than a launch refusal — the
        // launcher's other registration steps already gave the
        // daemon the pid/pgid via `session.register` + heartbeat.
        if err.to_string().contains("Method not found") {
            Ok(Value::Null)
        } else {
            Err(err)
        }
    })?;
    Ok(())
}

/// Block until the child exits and translate the [`ExitStatus`]
/// into a launcher exit code.
pub fn wait_for_child(mut child: std::process::Child) -> Result<ExitStatus> {
    child
        .wait()
        .context("failed to wait for the wrapped command")
}

/// Derive a deterministic Job Object name from the session id.
/// Cross-process Windows naming requires a stable string both ends
/// can compute independently; deriving from the session id keeps the
/// launcher and the daemon in sync without an extra round-trip.
#[must_use]
pub fn job_object_name_for(session_id: &SessionId) -> String {
    format!("anvil-intercept-{}", session_id.as_str())
}

#[cfg(unix)]
fn pgid_for(pid: u32) -> Option<i32> {
    // The launcher sets the child as its own pgid leader via
    // `Command::process_group(0)`, so the pgid is the pid. The
    // function is split out so tests can pin the invariant.
    i32::try_from(pid).ok()
}

#[cfg(windows)]
fn pgid_for(_pid: u32) -> Option<i32> {
    None
}

#[cfg(unix)]
fn apply_process_group(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    // `process_group(0)` becomes `setpgid(child, child)` on the
    // child side — the child is the leader of its own process
    // group, so the daemon can address `kill(-pgid, signal)`.
    cmd.process_group(0);
}

#[cfg(windows)]
fn apply_process_group(_cmd: &mut Command) {
    // Windows uses a Job Object created by the daemon side; the
    // launcher just emits a deterministic name. See INTD-006.
}

#[cfg(not(any(unix, windows)))]
fn apply_process_group(_cmd: &mut Command) {}

fn apply_env_propagation(cmd: &mut Command, session_id: &SessionId, agent_tag: &AgentTag) {
    // Delegate to the shared encoder so `ANVIL_AGENT_TAG` decoding is
    // identical to the daemon-side attribution path (avoids a silent
    // empty-string fallback if the encoder ever errors).
    set_attribution_env(cmd, agent_tag, session_id.as_str());
}

#[cfg(test)]
mod tests {
    use super::*;
    // `PathBuf` and `dummy_tag` are consumed only by the `#[cfg(unix)]` tests
    // below (Unix-first shell integration per INTL-006). Without matching cfg
    // gates they are unused on Windows, which trips `-D warnings` in the
    // `Cross (x86_64-pc-windows-msvc)` CI job.
    #[cfg(unix)]
    use std::path::PathBuf;

    #[cfg(unix)]
    fn dummy_tag() -> AgentTag {
        AgentTag::new("anvil-run", "claude-1", 1_700_000_000)
    }

    #[test]
    fn job_object_name_is_derived_deterministically_from_session_id() {
        let id = SessionId::new("sess_abc");
        let a = job_object_name_for(&id);
        let b = job_object_name_for(&id);
        assert_eq!(a, b);
        assert!(a.starts_with("anvil-intercept-"));
        assert!(a.ends_with("sess_abc"));
    }

    #[cfg(unix)]
    #[test]
    fn build_command_sets_anvil_env_vars_for_the_child() {
        // Unix-only: hard-codes `/usr/bin/env` and `/`. A Windows
        // version of this test would need `where /R %PATH% env` or
        // similar; INTL-006 keeps the shell integration Unix-first.
        use anvil_intercept_proto::session::{ANVIL_AGENT_TAG_ENV, ANVIL_TASK_ID_ENV};

        let session_id = SessionId::new("sess_test");
        let tag = dummy_tag();
        let cmd = build_command("/usr/bin/env", &[], &PathBuf::from("/"), &session_id, &tag);
        // `cmd` is configured but not spawned here. Spawn separately
        // so the test does not leak inheritance from this process.
        let mut cmd = cmd;
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());
        let output = cmd.output().expect("env runs");
        let out = String::from_utf8_lossy(&output.stdout);
        assert!(
            out.contains(&format!("{ANVIL_TASK_ID_ENV}=sess_test")),
            "ANVIL_TASK_ID must be set; got:\n{out}",
        );
        assert!(
            out.contains(ANVIL_AGENT_TAG_ENV),
            "ANVIL_AGENT_TAG must be set; got:\n{out}",
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_pgid_equals_pid_when_child_is_its_own_group_leader() {
        // Smoke test the wiring: spawn a no-op child, then read
        // its pgid via `getpgid` and confirm it matches the pid.
        use nix::unistd::{Pid, getpgid};

        let session_id = SessionId::new("sess_pgrp_test");
        let tag = dummy_tag();
        let mut cmd = build_command("/usr/bin/true", &[], &PathBuf::from("/"), &session_id, &tag);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        let mut child = cmd.spawn().expect("spawn");
        let child_pid = Pid::from_raw(i32::try_from(child.id()).unwrap());
        // Best-effort: the child may have already exited before we
        // query, in which case `getpgid` returns ESRCH. Either
        // outcome — pgid == pid OR ESRCH — proves the launcher
        // did its job; only "pgid == launcher's own pgid" would
        // be a regression.
        match getpgid(Some(child_pid)) {
            Ok(pgid) => assert_eq!(
                pgid.as_raw(),
                child_pid.as_raw(),
                "child should be its own pgid leader",
            ),
            Err(nix::errno::Errno::ESRCH) => {
                // Race with child exit — acceptable; the relevant
                // invariant (own pgid) is also exercised by other
                // spawns in the wider test suite.
            }
            Err(e) => panic!("unexpected getpgid error: {e}"),
        }
        let _ = child.wait();
    }
}
