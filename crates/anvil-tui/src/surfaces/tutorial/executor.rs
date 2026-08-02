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

fn autoplay_process(
    command: &str,
    root: &std::path::Path,
) -> std::io::Result<std::process::Command> {
    let words = command.split_ascii_whitespace().collect::<Vec<_>>();
    let ["anvil", "check", target] = words.as_slice() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "autoplay command must be exactly: anvil check <relative-target>",
        ));
    };
    let target = super::resolve_working_path(root, std::path::Path::new(target))?;
    let mut process = std::process::Command::new(std::env::current_exe()?);
    process
        .arg("check")
        .arg(target)
        .current_dir(root)
        .env_clear();
    for (name, path) in [
        ("ANVIL_HOME", ".anvil-home"),
        ("HOME", ".home"),
        ("XDG_CONFIG_HOME", ".config"),
        ("XDG_RUNTIME_DIR", ".runtime"),
        ("TMPDIR", ".tmp"),
        ("TEMP", ".tmp"),
        ("TMP", ".tmp"),
        ("USERPROFILE", ".home"),
        ("APPDATA", ".config"),
        ("LOCALAPPDATA", ".local-share"),
    ] {
        process.env(name, root.join(path));
    }
    Ok(process)
}

pub(crate) struct AutoplayCommand {
    child: Option<std::process::Child>,
    started_at: std::time::Instant,
    timed_out: bool,
}

impl AutoplayCommand {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    pub(crate) fn spawn(command: &str, root: &std::path::Path) -> std::io::Result<Self> {
        let mut process = autoplay_process(command, root)?;
        process
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        Ok(Self {
            child: Some(process.spawn()?),
            started_at: std::time::Instant::now(),
            timed_out: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn successful_for_test() -> Self {
        let mut process = std::process::Command::new(std::env::current_exe().expect("test binary"));
        process
            .arg("--help")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        Self {
            child: Some(process.spawn().expect("successful test child")),
            started_at: std::time::Instant::now(),
            timed_out: false,
        }
    }

    pub(crate) fn is_finished(&mut self) -> std::io::Result<bool> {
        let child = self.child.as_mut().expect("autoplay child is present");
        if child.try_wait()?.is_some() {
            return Ok(true);
        }
        if self.started_at.elapsed() >= Self::TIMEOUT {
            self.timed_out = true;
            child.kill()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn finish(mut self) -> CommandOutput {
        let timed_out = self.timed_out;
        let child = self.child.take().expect("autoplay child is present");
        let mut output = command_output(child.wait_with_output());
        if timed_out {
            output.success = false;
            output.stderr = "autoplay command exceeded its 30 second limit".to_string();
        }
        output
    }

    pub(crate) fn cancel(mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for AutoplayCommand {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
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

    #[test]
    fn autoplay_process_is_structured_and_environment_is_allowlisted() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src/app.ts"), "fixture").expect("fixture");
        let process = autoplay_process("anvil check src/app.ts", root.path()).expect("process");

        // `resolve_working_path` canonicalises the target (macOS: `/var` →
        // `/private/var`); current_dir stays the path we passed in.
        let expected_target = root
            .path()
            .canonicalize()
            .expect("canonicalize root")
            .join("src/app.ts");
        assert_eq!(process.get_current_dir(), Some(root.path()));
        assert_eq!(process.get_program(), std::env::current_exe().unwrap());
        assert_eq!(
            process.get_args().collect::<Vec<_>>(),
            [std::ffi::OsStr::new("check"), expected_target.as_os_str()]
        );
        let env: std::collections::HashMap<_, _> = process.get_envs().collect();
        for value in env.values().flatten() {
            assert!(std::path::Path::new(value).starts_with(root.path()));
        }
        assert!(!env.keys().any(|name| *name == std::ffi::OsStr::new("PATH")));
        assert!(
            !env.keys()
                .any(|name| *name == std::ffi::OsStr::new("ANVIL_REGISTRY_PATH"))
        );
    }

    #[test]
    fn autoplay_process_rejects_shell_and_alternate_binary_commands() {
        let root = tempfile::tempdir().expect("tempdir");
        assert!(autoplay_process("cat src/app.ts", root.path()).is_err());
        assert!(autoplay_process("anvil-check src/app.ts", root.path()).is_err());
        assert!(autoplay_process("anvil check", root.path()).is_err());
        assert!(autoplay_process("anvil check src/app.ts --fix", root.path()).is_err());
        assert!(autoplay_process("anvil check src/app.ts && touch escaped", root.path()).is_err());
        assert!(autoplay_process("anvil check ../outside.ts", root.path()).is_err());
        assert!(autoplay_process("anvil check /tmp/outside.ts", root.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn autoplay_process_rejects_symlink_escape_target() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        symlink(outside.path(), root.path().join("linked")).expect("symlink");

        assert!(autoplay_process("anvil check linked/app.ts", root.path()).is_err());
    }
}
