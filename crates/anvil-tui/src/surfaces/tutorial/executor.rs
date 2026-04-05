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
/// # TODO(WELCOME-013)
///
/// Move to channel-based async execution so long-running commands
/// (e.g. `anvil architecture compile`) don't freeze the TUI. Add a
/// timeout (30s) and a spinner during execution.
pub(crate) fn execute_command(command: &str) -> CommandOutput {
    let result = if cfg!(windows) {
        std::process::Command::new("cmd")
            .arg("/C")
            .arg(command)
            .output()
    } else {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
    };

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
        assert!(output.stdout.trim() == "HELLO");
    }
}
