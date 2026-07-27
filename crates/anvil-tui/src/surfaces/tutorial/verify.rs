use super::CommandOutput;

/// Verification check to run after a tutorial step's command completes.
#[derive(Debug, Clone)]
pub enum Verify {
    /// Check that a file or directory exists at the given path.
    FileExists(String),
    /// Check the command's exit code matches the expected value.
    ExitCode(i32),
    /// Check that the command's stdout contains the given text.
    ///
    /// Uses substring matching (`str::contains`) rather than regex to avoid
    /// pulling in the `regex` crate. This is sufficient for tutorial
    /// verification where patterns are simple known strings.
    OutputContains(String),
}

/// Result of running a verification check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyResult {
    /// Verification passed.
    Pass,
    /// Verification failed with a contextual hint.
    Fail(String),
}

impl Verify {
    /// Run this verification check against captured command output.
    pub fn check(&self, output: &CommandOutput) -> VerifyResult {
        self.check_in_root(output, None)
    }

    pub(crate) fn check_in_root(
        &self,
        output: &CommandOutput,
        working_root: Option<&std::path::Path>,
    ) -> VerifyResult {
        match self {
            Verify::FileExists(path) => {
                let target = if let Some(root) = working_root {
                    match super::resolve_working_path(root, std::path::Path::new(path)) {
                        Ok(target) => target,
                        Err(error) => return VerifyResult::Fail(error.to_string()),
                    }
                } else {
                    std::path::PathBuf::from(path)
                };
                if target.exists() {
                    VerifyResult::Pass
                } else {
                    VerifyResult::Fail(format!("Expected file not found: {path}"))
                }
            }
            Verify::ExitCode(expected) => match output.exit_code {
                Some(code) if code == *expected => VerifyResult::Pass,
                Some(code) => {
                    VerifyResult::Fail(format!("Expected exit code {expected}, got {code}"))
                }
                None => VerifyResult::Fail(format!(
                    "Expected exit code {expected}, but process had no exit code"
                )),
            },
            Verify::OutputContains(pattern) => {
                if output.stdout.contains(pattern.as_str())
                    || output.stderr.contains(pattern.as_str())
                {
                    VerifyResult::Pass
                } else {
                    VerifyResult::Fail(format!("Output did not contain expected text: {pattern}"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_output(
        stdout: &str,
        stderr: &str,
        success: bool,
        exit_code: Option<i32>,
    ) -> CommandOutput {
        CommandOutput {
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            success,
            exit_code,
        }
    }

    // --- FileExists ---

    #[test]
    fn file_exists_pass() {
        let dir = std::env::temp_dir().join("anvil_verify_test_exists");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("marker.txt");
        std::fs::write(&file, "ok").unwrap();

        let verify = Verify::FileExists(file.to_string_lossy().to_string());
        let output = make_output("", "", true, Some(0));
        assert_eq!(verify.check(&output), VerifyResult::Pass);

        // Clean up.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_exists_fail() {
        let path = std::env::temp_dir().join(format!(
            "anvil_verify_nonexistent_{}_marker.txt",
            std::process::id()
        ));
        let path = path.to_string_lossy().to_string();
        let verify = Verify::FileExists(path.clone());
        let output = make_output("", "", true, Some(0));
        assert_eq!(
            verify.check(&output),
            VerifyResult::Fail(format!("Expected file not found: {path}")),
        );
    }

    #[test]
    fn file_exists_is_scoped_to_working_root() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join("marker.txt"), "ok").expect("fixture");
        let output = make_output("", "", true, Some(0));

        assert_eq!(
            Verify::FileExists("marker.txt".to_string()).check_in_root(&output, Some(root.path())),
            VerifyResult::Pass
        );
        assert!(matches!(
            Verify::FileExists("../escape.txt".to_string())
                .check_in_root(&output, Some(root.path())),
            VerifyResult::Fail(message) if message.contains("outside")
        ));
    }

    // --- ExitCode ---

    #[test]
    fn exit_code_pass() {
        let verify = Verify::ExitCode(0);
        let output = make_output("", "", true, Some(0));
        assert_eq!(verify.check(&output), VerifyResult::Pass);
    }

    #[test]
    fn exit_code_fail() {
        let verify = Verify::ExitCode(0);
        let output = make_output("", "", false, Some(1));
        assert_eq!(
            verify.check(&output),
            VerifyResult::Fail("Expected exit code 0, got 1".to_string()),
        );
    }

    #[test]
    fn exit_code_none_fails() {
        let verify = Verify::ExitCode(0);
        let output = make_output("", "", false, None);
        assert_eq!(
            verify.check(&output),
            VerifyResult::Fail("Expected exit code 0, but process had no exit code".to_string()),
        );
    }

    // --- OutputContains ---

    #[test]
    fn output_contains_pass() {
        let verify = Verify::OutputContains("status".to_string());
        let output = make_output(r#"{"status":"ok"}"#, "", true, Some(0));
        assert_eq!(verify.check(&output), VerifyResult::Pass);
    }

    #[test]
    fn output_contains_fail() {
        let verify = Verify::OutputContains("status".to_string());
        let output = make_output("nothing relevant here", "", true, Some(0));
        assert_eq!(
            verify.check(&output),
            VerifyResult::Fail("Output did not contain expected text: status".to_string()),
        );
    }

    #[test]
    fn output_contains_matches_stderr() {
        let verify = Verify::OutputContains("warning".to_string());
        let output = make_output("", "warning: deprecated", true, Some(0));
        assert_eq!(verify.check(&output), VerifyResult::Pass);
    }

    #[test]
    fn output_contains_against_empty() {
        let verify = Verify::OutputContains("status".to_string());
        let output = make_output("", "", true, Some(0));
        assert_eq!(
            verify.check(&output),
            VerifyResult::Fail("Output did not contain expected text: status".to_string()),
        );
    }
}
