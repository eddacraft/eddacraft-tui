//! Shared process-spawn helpers for the resource benches (RLB-002..005).
//!
//! Every resource bench drives a real long-running `anvil` subprocess (watch,
//! the intercept daemon, the MCP server) and measures its process tree. They
//! all need the same two things: locate the built `anvil` binary, and manage a
//! child that must stay alive across the measurement window (and be killed on
//! drop). This module is that shared surface so the four benches don't each
//! re-implement it.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Put a child in its **own process group** before spawning, so the whole group
/// (the process *and* every descendant it spawns — e.g. watch's per-save
/// `anvil check`) can be killed together by [`ManagedChild::shutdown`]. Without
/// this, killing only the direct child reparents its grandchildren to init,
/// where they linger and leak resources (CPU, inotify handles). No-op on
/// non-Unix.
pub fn in_new_process_group(cmd: &mut Command) -> &mut Command {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd
}

/// Locate the `anvil` binary to drive. Priority:
/// 1. `ANVIL_BENCH_ANVIL_BIN` (absolute, or resolved against cwd / the
///    workspace root) — how CI and a quiet-box run point at a release build;
/// 2. `target/debug/anvil` under the workspace, then `target/release/anvil`.
pub fn resolve_anvil_binary() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("ANVIL_BENCH_ANVIL_BIN") {
        return resolve_configured_anvil_binary(PathBuf::from(path));
    }

    let candidate = workspace_target_anvil("debug");
    if candidate.exists() {
        return Ok(candidate);
    }

    let candidate = workspace_target_anvil("release");
    if candidate.exists() {
        return Ok(candidate);
    }

    Err("set ANVIL_BENCH_ANVIL_BIN or build target/debug/anvil first".into())
}

fn resolve_configured_anvil_binary(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    let from_cwd = std::env::current_dir()?.join(&path);
    if from_cwd.exists() {
        return Ok(from_cwd);
    }

    if let Some(path) = resolve_against_cargo_target_dir(&path) {
        return Ok(path);
    }

    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path))
}

fn workspace_target_anvil(profile: &str) -> PathBuf {
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR") {
        let candidate = PathBuf::from(target_dir).join(profile).join("anvil");
        if candidate.exists() {
            return candidate;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target")
        .join(profile)
        .join("anvil")
}

fn resolve_against_cargo_target_dir(path: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    if components.next()?.as_os_str() != "target" {
        return None;
    }
    let profile = components.next()?.as_os_str();
    let binary = components.next()?.as_os_str();
    if binary != "anvil" || components.next().is_some() {
        return None;
    }
    Some(
        PathBuf::from(std::env::var_os("CARGO_TARGET_DIR")?)
            .join(profile)
            .join(binary),
    )
}

/// A spawned child that is killed and reaped on drop, with a liveness check so
/// a process that dies during startup or mid-window is reported as an error
/// rather than silently measured as a frozen zombie (a false "0% pass").
pub struct ManagedChild {
    child: Option<Child>,
    label: String,
}

impl ManagedChild {
    /// Wrap an already-spawned child. `label` names it in error messages.
    #[must_use]
    pub fn new(child: Child, label: impl Into<String>) -> Self {
        Self {
            child: Some(child),
            label: label.into(),
        }
    }

    /// The child pid (for `/proc` sampling).
    #[must_use]
    pub fn id(&self) -> u32 {
        self.child.as_ref().expect("child is live").id()
    }

    /// Error (with `context`) if the child has already exited.
    pub fn ensure_running(&mut self, context: &str) -> Result<()> {
        let label = self.label.clone();
        let child = self.child.as_mut().ok_or("child already reaped")?;
        match child.try_wait()? {
            Some(status) => Err(format!("{label} exited {status} {context}").into()),
            None => Ok(()),
        }
    }

    /// Kill and reap the child. On Unix, if the child was spawned via
    /// [`in_new_process_group`], the whole process group is signalled so
    /// grandchildren (e.g. a per-save `anvil check`) die with it; the direct
    /// kill is always issued too as a fallback. Idempotent.
    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            {
                use nix::sys::signal::{Signal, killpg};
                use nix::unistd::Pid;
                // Group id == leader pid because the child was placed in its own
                // group. Guard the i32 conversion so a (impossible on Linux)
                // out-of-range pid can never become 0 and signal our own group.
                if let Ok(pgid) = i32::try_from(child.id()) {
                    // Best-effort: ESRCH (already gone) is fine.
                    let _ = killpg(Pid::from_raw(pgid), Signal::SIGKILL);
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_binary_path_is_resolved_before_child_changes_dir() {
        temp_env::with_var("CARGO_TARGET_DIR", None::<&str>, || {
            let path =
                resolve_configured_anvil_binary(PathBuf::from("target/debug/anvil")).unwrap();
            assert!(path.is_absolute());
            assert!(path.ends_with("target/debug/anvil"));
        });
    }

    #[test]
    fn target_relative_configured_path_honours_cargo_target_dir() {
        let dir = tempfile::tempdir().unwrap();
        temp_env::with_var("CARGO_TARGET_DIR", Some(dir.path()), || {
            let path = resolve_against_cargo_target_dir(Path::new("target/release/anvil")).unwrap();
            assert_eq!(path, dir.path().join("release").join("anvil"));
        });
    }

    #[test]
    fn absolute_configured_path_is_returned_verbatim() {
        let abs = if cfg!(windows) {
            PathBuf::from("C:/anvil/anvil.exe")
        } else {
            PathBuf::from("/opt/anvil/anvil")
        };
        assert_eq!(resolve_configured_anvil_binary(abs.clone()).unwrap(), abs);
    }

    #[test]
    fn ensure_running_detects_exited_child() {
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("spawn `true`");
        // Reap deterministically before the liveness check: `Child` caches the
        // exit status, so `try_wait` inside `ensure_running` sees it without a
        // timing assumption. A fixed sleep raced child startup on loaded
        // Windows CI runners (Cross matrix red, 2026-06-07).
        child.wait().expect("`true` exits");
        let mut managed = ManagedChild::new(child, "true-probe");
        let err = managed
            .ensure_running("after probe")
            .expect_err("a process that ran `true` should be reported as exited");
        assert!(format!("{err}").contains("true-probe"));
    }

    #[test]
    fn ensure_running_passes_for_live_child() {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn `sleep`");
        let mut managed = ManagedChild::new(child, "sleep-probe");
        managed
            .ensure_running("while alive")
            .expect("sleep is alive");
        managed.shutdown();
    }
}
