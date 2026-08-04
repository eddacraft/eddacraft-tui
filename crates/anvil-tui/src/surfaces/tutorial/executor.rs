use super::CommandOutput;

/// Execute a shell command and capture its output.
///
/// The command string is passed to the platform shell (`sh -c` on Unix,
/// `cmd /C` on Windows) so shell syntax works without extra parsing.
/// Commands are expected to complete quickly — this call blocks the
/// calling thread.
///
/// # Safety contract
///
/// Command strings must come from a fixed allow-list (tutorial step
/// definitions in `paths.rs`), never from user input. The function is
/// `pub(crate)` to limit the blast radius.
///
/// WELCOME-013 adds a file watcher that re-runs verification on
/// filesystem changes, avoiding the need for async command execution
/// in most interactive steps. Commands that do run are expected to
/// complete quickly (sub-second); the watcher handles the
/// edit-then-verify cycle without blocking.
pub(crate) fn execute_command(command: &str) -> CommandOutput {
    let mut process = if cfg!(windows) {
        let mut process = std::process::Command::new("cmd");
        process.arg("/C").arg(command);
        process
    } else {
        let mut process = std::process::Command::new("sh");
        process.arg("-c").arg(command);
        process
    };
    command_output(process.output())
}

/// Runs one autoplay demo check **in-process** and returns what the user sees.
///
/// CIB-248: autoplay used to re-enter the gated `anvil check` CLI as a child
/// process with `env_clear()` and a sandbox `HOME`/`ANVIL_HOME`. Real
/// credentials live under the host config dir, so the child never saw them and
/// every demo step failed `Authentication required` — including straight after
/// a successful `anvil auth login`, because the sandbox env is what hid the
/// credentials, not the absence of a session.
///
/// ADR-080 already settles the shape of the fix: `welcome` is ungated and the
/// hub runs gate / audit / doctor "in-process", so the demo runs the check the
/// same way. No CLI dispatch happens, so the licence gate is never consulted
/// and no externally-settable bypass is introduced.
///
/// `anvil-tui` deliberately depends only on `eddacraft-tui` and
/// `anvil-kernel-types`, so the runner is **injected** by `anvil-cli` (which
/// already owns the check crates) rather than called directly from here.
pub type AutoplayRunner =
    std::sync::Arc<dyn Fn(&std::path::Path) -> CommandOutput + Send + Sync + 'static>;

/// Validate an autoplay command and resolve its target inside the sandbox.
///
/// The allow-list is unchanged from the subprocess era: exactly
/// `anvil check <relative-target>`, resolved through
/// [`super::resolve_working_path`] so symlink and `..` escapes are rejected.
/// Keeping this gate matters even without a child process — the runner is
/// handed a path, and that path must stay inside the sandbox.
fn autoplay_target(command: &str, root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let words = command.split_ascii_whitespace().collect::<Vec<_>>();
    let ["anvil", "check", target] = words.as_slice() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "autoplay command must be exactly: anvil check <relative-target>",
        ));
    };
    super::resolve_working_path(root, std::path::Path::new(target))
}

/// An autoplay check running on a worker thread.
///
/// The demo check is fast (one pinned fixture), but it still runs off the UI
/// thread so a slow filesystem cannot freeze the tutorial mid-frame. The
/// polling shape (`is_finished` → `finish`) is unchanged from the subprocess
/// implementation, so the caller in `tutorial::mod` keeps its existing
/// non-blocking tick.
pub(crate) struct AutoplayCommand {
    receiver: std::sync::mpsc::Receiver<CommandOutput>,
    output: Option<CommandOutput>,
    started_at: std::time::Instant,
    timed_out: bool,
}

impl AutoplayCommand {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    /// Validate the command first, then require a runner.
    ///
    /// Ordering matters: a malformed or escaping command is rejected as such
    /// whether or not `anvil-cli` supplied a runner, so the allow-list message
    /// never gets masked by a wiring error.
    pub(crate) fn spawn(
        command: &str,
        root: &std::path::Path,
        runner: Option<&AutoplayRunner>,
    ) -> std::io::Result<Self> {
        let target = autoplay_target(command, root)?;
        let runner = runner.ok_or_else(|| {
            std::io::Error::other("autoplay demo is unavailable: no check runner was supplied")
        })?;
        let runner = std::sync::Arc::clone(runner);
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("anvil-autoplay-check".to_string())
            .spawn(move || {
                // A disconnected receiver just means the tutorial moved on.
                let _ = sender.send(runner(&target));
            })?;
        Ok(Self {
            receiver,
            output: None,
            started_at: std::time::Instant::now(),
            timed_out: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn successful_for_test() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        sender
            .send(CommandOutput {
                stdout: "AP-003 explicit any".to_string(),
                stderr: String::new(),
                success: true,
                exit_code: Some(0),
            })
            .expect("test output");
        Self {
            receiver,
            output: None,
            started_at: std::time::Instant::now(),
            timed_out: false,
        }
    }

    pub(crate) fn is_finished(&mut self) -> std::io::Result<bool> {
        if self.output.is_some() || self.timed_out {
            return Ok(true);
        }
        match self.receiver.try_recv() {
            Ok(output) => {
                self.output = Some(output);
                Ok(true)
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(std::io::Error::other(
                "autoplay check ended without reporting a result",
            )),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                if self.started_at.elapsed() >= Self::TIMEOUT {
                    self.timed_out = true;
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    pub(crate) fn finish(mut self) -> CommandOutput {
        if self.timed_out {
            return CommandOutput {
                stdout: String::new(),
                stderr: "autoplay command exceeded its 30 second limit".to_string(),
                success: false,
                exit_code: None,
            };
        }
        self.output
            .take()
            .or_else(|| self.receiver.recv().ok())
            .unwrap_or_else(|| CommandOutput {
                stdout: String::new(),
                stderr: "autoplay check ended without reporting a result".to_string(),
                success: false,
                exit_code: None,
            })
    }

    /// Abandon the result. The worker thread is not killable, but it only
    /// reads one pinned sandbox file and exits, so dropping the receiver is
    /// enough — the send simply fails and the thread ends.
    pub(crate) fn cancel(self) {}
}

fn command_output(result: std::io::Result<std::process::Output>) -> CommandOutput {
    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let success = output.status.success();
            let exit_code = output.status.code();
            CommandOutput {
                stdout,
                stderr,
                success,
                exit_code,
            }
        }
        Err(err) => CommandOutput {
            stdout: String::new(),
            stderr: format!("failed to spawn process: {err}"),
            success: false,
            exit_code: None,
        },
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn successful_command_returns_success() {
        let output = execute_command("echo hello");
        assert!(output.success);
        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.contains("hello"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn failing_command_returns_failure() {
        let output = execute_command("exit 1");
        assert!(!output.success);
        assert_eq!(output.exit_code, Some(1));
    }

    #[test]
    fn stderr_is_captured() {
        let output = execute_command("echo error_output >&2");
        assert!(output.stderr.contains("error_output"));
    }

    #[test]
    fn nonzero_exit_code_is_captured() {
        let output = execute_command("exit 42");
        assert!(!output.success);
        assert_eq!(output.exit_code, Some(42));
    }

    #[test]
    fn stdout_and_stderr_both_captured() {
        let output = execute_command("echo out; echo err >&2");
        assert!(output.stdout.contains("out"));
        assert!(output.stderr.contains("err"));
    }

    #[test]
    fn shell_builtins_work() {
        let output = execute_command("true");
        assert!(output.success);
        assert_eq!(output.exit_code, Some(0));
    }

    #[test]
    fn command_with_pipe_works() {
        let output = execute_command("echo hello | tr a-z A-Z");
        assert!(output.success);
        assert_eq!(output.stdout.trim(), "HELLO");
    }

    /// CIB-248: the demo check must reach the runner in-process. The runner is
    /// handed the resolved sandbox path and its output is what the tutorial
    /// shows — no `anvil check` child process, so the licence gate that used to
    /// fail every demo step with `Authentication required` is never consulted.
    #[test]
    fn autoplay_runs_in_process_against_the_resolved_sandbox_target() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src/app.ts"), "fixture").expect("fixture");

        let seen: std::sync::Arc<std::sync::Mutex<Option<std::path::PathBuf>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let recorder = std::sync::Arc::clone(&seen);
        let runner: AutoplayRunner = std::sync::Arc::new(move |target: &std::path::Path| {
            *recorder.lock().expect("record target") = Some(target.to_path_buf());
            CommandOutput {
                stdout: "AP-003 explicit any".to_string(),
                stderr: String::new(),
                success: true,
                exit_code: Some(0),
            }
        });

        let mut command =
            AutoplayCommand::spawn("anvil check src/app.ts", root.path(), Some(&runner)).expect("spawn");
        while !command.is_finished().expect("poll") {
            std::thread::yield_now();
        }
        let output = command.finish();

        assert!(output.success);
        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.contains("AP-003"));

        // `resolve_working_path` canonicalises the target (macOS: `/var` →
        // `/private/var`), and the runner must never see a path outside root.
        let expected_target = root
            .path()
            .canonicalize()
            .expect("canonicalize root")
            .join("src/app.ts");
        let observed = seen.lock().expect("read target").clone().expect("invoked");
        assert_eq!(observed, expected_target);
    }

    fn unreachable_runner() -> AutoplayRunner {
        std::sync::Arc::new(|_: &std::path::Path| unreachable!("runner must not be reached"))
    }

    #[test]
    fn autoplay_rejects_shell_and_alternate_binary_commands() {
        let root = tempfile::tempdir().expect("tempdir");
        let runner = unreachable_runner();
        for command in [
            "cat src/app.ts",
            "anvil-check src/app.ts",
            "anvil check",
            "anvil check src/app.ts --fix",
            "anvil check src/app.ts && touch escaped",
            "anvil check ../outside.ts",
            "anvil check /tmp/outside.ts",
        ] {
            assert!(
                AutoplayCommand::spawn(command, root.path(), Some(&runner)).is_err(),
                "expected rejection: {command}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn autoplay_rejects_symlink_escape_target() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        symlink(outside.path(), root.path().join("linked")).expect("symlink");

        assert!(
            AutoplayCommand::spawn("anvil check linked/app.ts", root.path(), Some(&unreachable_runner()))
                .is_err()
        );
    }
}
