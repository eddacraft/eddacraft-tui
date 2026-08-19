use super::CommandOutput;

/// Execute a fixed tutorial command from the supplied canonical session root.
///
/// Direct commands beginning with the fixed token `anvil` bypass the shell and
/// PATH entirely: the running executable is invoked with structured arguments.
/// The policy-directory step is a structured filesystem operation. Remaining
/// non-`anvil` commands are read-only and keep shell support with an explicit
/// current directory.
///
/// # Safety contract
///
/// Command strings must come from a fixed allow-list (tutorial step
/// definitions in `paths.rs`), never from user input. The function is
/// `pub(crate)` to limit the blast radius. Fixed `anvil` arguments use the
/// current tutorial's simple whitespace-token grammar; unsupported quoting or
/// shell syntax fails closed.
///
/// WELCOME-013 adds a file watcher that re-runs verification on
/// filesystem changes, avoiding the need for async command execution
/// in most interactive steps. Commands that do run are expected to
/// complete quickly (sub-second); the watcher handles the
/// edit-then-verify cycle without blocking.
pub(crate) fn execute_command_in(command: &str, root: &std::path::Path) -> CommandOutput {
    if is_structured_filesystem_command(command) {
        return create_policy_directory(root);
    }
    let process = process_for_command(command, root, std::env::current_exe);
    command_output(process.and_then(|mut process| process.output()))
}

pub(super) fn policy_directory_command() -> &'static str {
    if cfg!(windows) {
        r"mkdir .anvil\policies"
    } else {
        "mkdir -p .anvil/policies"
    }
}

pub(super) fn is_structured_filesystem_command(command: &str) -> bool {
    command == policy_directory_command()
}

fn create_policy_directory(root: &std::path::Path) -> CommandOutput {
    let result = super::resolve_working_path(root, std::path::Path::new(".anvil/policies"))
        .and_then(std::fs::create_dir_all);
    match result {
        Ok(()) => CommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            exit_code: Some(0),
        },
        Err(error) => CommandOutput {
            stdout: String::new(),
            stderr: format!("failed to create tutorial directory .anvil/policies: {error}"),
            success: false,
            exit_code: None,
        },
    }
}

/// Test-compatible fallback for states built without a bound workspace.
///
/// Production tutorial entry points bind a canonical root before the surface
/// runs. Keeping this wrapper lets isolated state tests exercise fixed commands
/// without manufacturing CLI launch context; execution still receives an
/// explicit resolved current directory and direct `anvil` commands still
/// bypass PATH.
pub(crate) fn execute_command(command: &str) -> CommandOutput {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => return command_output(Err(error)),
    };
    execute_command_in(command, &root)
}

fn process_for_command(
    command: &str,
    root: &std::path::Path,
    current_exe: impl FnOnce() -> std::io::Result<std::path::PathBuf>,
) -> std::io::Result<std::process::Command> {
    if is_structured_filesystem_command(command) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "policy directory creation must use the structured filesystem operation",
        ));
    }
    let root = super::canonicalize_working_path(root)?;
    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "tutorial working root is not a directory",
        ));
    }

    let mut process = if let Some(args) = direct_anvil_args(command)? {
        let mut process = std::process::Command::new(current_exe()?);
        process.args(args);
        process
    } else if cfg!(windows) {
        let mut process = std::process::Command::new("cmd");
        process.arg("/C").arg(command);
        process
    } else {
        let mut process = std::process::Command::new("sh");
        process.arg("-c").arg(command);
        process
    };
    process.current_dir(root);
    Ok(process)
}

fn direct_anvil_args(command: &str) -> std::io::Result<Option<Vec<&str>>> {
    let mut words = command.split_ascii_whitespace();
    let Some(program) = words.next() else {
        return Ok(None);
    };
    if program != "anvil" {
        return Ok(None);
    }
    let args = words.collect::<Vec<_>>();
    if args.iter().any(|arg| {
        arg.is_empty()
            || !arg.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':' | b'\\')
            })
    }) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "fixed anvil command contains unsupported argument syntax",
        ));
    }
    Ok(Some(args))
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

/// Name of the worker thread the autoplay check runs on.
///
/// The stable name keeps the worker identifiable in diagnostics. Panic-hook
/// suppression is deliberately keyed to [`is_autoplay_panic_contained`], not
/// this reusable name.
pub const AUTOPLAY_WORKER_THREAD: &str = "anvil-autoplay-check";

std::thread_local! {
    static AUTOPLAY_PANIC_CONTAINED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the current thread is inside the autoplay runner's panic-catching
/// boundary.
///
/// The process-wide host panic hook uses this signal to avoid restoring or
/// printing over the live TUI only when the panic is guaranteed to be caught
/// and converted into a failed demo step. A worker thread name alone does not
/// establish that invariant.
pub fn is_autoplay_panic_contained() -> bool {
    AUTOPLAY_PANIC_CONTAINED
        .try_with(std::cell::Cell::get)
        .unwrap_or(false)
}

struct AutoplayPanicContainment {
    previous: bool,
}

impl AutoplayPanicContainment {
    fn enter() -> Self {
        let previous = AUTOPLAY_PANIC_CONTAINED.with(|contained| contained.replace(true));
        Self { previous }
    }
}

impl Drop for AutoplayPanicContainment {
    fn drop(&mut self) {
        AUTOPLAY_PANIC_CONTAINED.with(|contained| contained.set(self.previous));
    }
}

/// Run an autoplay check inside the panic boundary recognised by the host hook.
pub fn catch_autoplay_panic<T>(runner: impl FnOnce() -> T) -> std::thread::Result<T> {
    let _containment = AutoplayPanicContainment::enter();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(runner))
}

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
            .name(AUTOPLAY_WORKER_THREAD.to_string())
            .spawn(move || {
                // CIB-249 teardown: a panic in the runner must not unwind out
                // of this thread. The host installs a process-wide panic hook
                // that restores the terminal, and the TUI is still running, so
                // an escaping panic would tear the screen down mid-session.
                // The subprocess era contained panics for free (child process,
                // piped stderr); catching here restores that, reporting the
                // panic as an ordinary failed step through the existing
                // recovery path.
                let output = catch_autoplay_panic(|| runner(&target)).unwrap_or_else(|payload| {
                    CommandOutput {
                        stdout: String::new(),
                        stderr: format!("autoplay check panicked: {}", panic_message(&*payload)),
                        success: false,
                        exit_code: None,
                    }
                });
                // A disconnected receiver just means the tutorial moved on.
                let _ = sender.send(output);
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

    /// Abandon the result. **Does not stop the work.**
    ///
    /// Dropping the receiver does not cancel the worker thread: the check runs
    /// to completion, its `send` then fails, and only at that point does the
    /// thread exit. The 30-second limit in [`Self::is_finished`] is the same —
    /// it bounds how long the tutorial *waits*, not how long the check runs.
    /// The subprocess implementation could kill its child; this one cannot.
    ///
    /// That is tolerable only because the demo check reads one pinned sandbox
    /// file. If the sandbox `TempDir` is dropped first, the runner simply finds
    /// nothing to read. A runner doing heavier work would need a real
    /// cancellation signal.
    pub(crate) fn cancel(self) {
        drop(self.receiver);
    }
}

/// Best-effort human text for a caught panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

/// Command named by the CIB-349 sign-in bridge. Tutorial copy and the
/// footer must mention this *before* any licence-gated command so a
/// first-time unsigned-in walk does not dead-end on an auth wall.
pub(super) const AUTH_LOGIN_COMMAND: &str = "anvil auth login";

/// Conservative fallback used when the CLI has not injected
/// [`crate::feature_flags::tutorial_command_needs_licence_gate`]. Covers
/// the CIB-349 class (`policy` / `gate` / `architecture`) plus other
/// tutorial commands that would hit the licence wall. `start --verify`
/// and `status --verify` stay free.
pub(super) fn command_needs_licence_gate_fallback(command: &str) -> bool {
    let mut words = command.split_ascii_whitespace();
    let Some(program) = words.next() else {
        return false;
    };
    if program != "anvil" {
        return false;
    }
    let Some(sub) = words.next() else {
        return false;
    };
    if matches!(sub, "start" | "status")
        && command
            .split_ascii_whitespace()
            .any(|word| word == "--verify")
    {
        return false;
    }
    matches!(
        sub,
        "policy"
            | "gate"
            | "architecture"
            | "check"
            | "drift"
            | "status"
            | "start"
            | "init"
            | "watch"
            | "audit"
    )
}

fn instruction_offers_run(instruction: &str) -> bool {
    let lower = instruction.to_ascii_lowercase();
    lower.contains("run:") || lower.contains("re-run anvil") || lower.contains("run anvil")
}

fn instruction_mentions_gated_command(
    instruction: &str,
    command_is_gated: &impl Fn(&str) -> bool,
) -> bool {
    let tokens: Vec<&str> = instruction.split_ascii_whitespace().collect();
    for (index, token) in tokens.iter().enumerate() {
        let cleaned = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
        if cleaned != "anvil" {
            continue;
        }
        let Some(next) = tokens.get(index + 1) else {
            continue;
        };
        let sub = next.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
        let mut candidate = format!("anvil {sub}");
        if let Some(flag) = tokens.get(index + 2) {
            let flag = flag.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
            if flag == "--verify" {
                candidate.push_str(" --verify");
            }
        }
        if command_is_gated(&candidate) {
            return true;
        }
    }
    false
}

/// Rewrite a gated command step so Enter cannot run it. The original
/// command stays in the instruction as the follow-up after sign-in.
pub(super) fn bridge_command_step(step: &mut super::TutorialStep, cmd: &str) {
    step.description = format!(
        "This check is licence-gated. Sign in before running `{cmd}` so the walk does not stop on an auth wall."
    );
    step.instruction = format!("Run: {AUTH_LOGIN_COMMAND}, then `{cmd}`.");
    step.command = None;
    step.effect = None;
    step.verify = None;
    step.verify_hint = None;
    step.sign_in_bridge = true;
}

fn bridge_informational_step(step: &mut super::TutorialStep) {
    if !step.instruction.contains(AUTH_LOGIN_COMMAND) {
        step.instruction = format!(
            "Sign in first: {AUTH_LOGIN_COMMAND}. Then {}",
            step.instruction
        );
    }
    step.sign_in_bridge = true;
}

/// Convert gated command steps (and informational "run now" copy that
/// names a gated command) into a sign-in bridge. Call only when the
/// current session would hit the licence wall.
pub(super) fn apply_sign_in_bridge(
    steps: &mut [super::TutorialStep],
    command_is_gated: impl Fn(&str) -> bool,
) {
    for step in steps {
        if let Some(cmd) = step.command.clone() {
            if command_is_gated(&cmd) {
                bridge_command_step(step, &cmd);
            }
        } else if instruction_offers_run(&step.instruction)
            && instruction_mentions_gated_command(&step.instruction, &command_is_gated)
        {
            bridge_informational_step(step);
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

#[cfg(test)]
mod cross_platform_tests {
    use super::*;

    #[test]
    fn fixed_anvil_command_is_structured_from_current_executable_in_bound_root() {
        let root = tempfile::tempdir().expect("workspace");
        let fake_path_binary = root
            .path()
            .join(if cfg!(windows) { "anvil.exe" } else { "anvil" });
        std::fs::write(&fake_path_binary, "marker").expect("fake PATH binary");
        let current_exe = std::env::current_exe().expect("current executable");

        let process = process_for_command("anvil status --json", root.path(), || {
            Ok(current_exe.clone())
        })
        .expect("fixed anvil command");

        assert_eq!(process.get_program(), current_exe.as_os_str());
        assert_ne!(process.get_program(), fake_path_binary.as_os_str());
        assert_eq!(
            process.get_current_dir(),
            Some(
                dunce::canonicalize(root.path())
                    .expect("canonical workspace")
                    .as_path()
            )
        );
        assert_eq!(
            process.get_args().collect::<Vec<_>>(),
            ["status", "--json"].map(std::ffi::OsStr::new)
        );
    }

    #[test]
    fn policy_directory_command_uses_structured_filesystem_execution() {
        let root = tempfile::tempdir().expect("workspace");

        let output = execute_command_in(policy_directory_command(), root.path());

        assert!(output.success, "stderr: {}", output.stderr);
        assert!(root.path().join(".anvil/policies").is_dir());
        assert!(
            process_for_command(
                policy_directory_command(),
                root.path(),
                std::env::current_exe
            )
            .is_err(),
            "the structured directory operation must never build sh/cmd"
        );
    }

    /// CIB-261: Policy-checks re-run must not hard-stop when `.anvil/policies`
    /// already exists. The step still *displays* platform-native `mkdir` text
    /// (Windows without `-p`), but execution is `create_dir_all` under the
    /// structured filesystem path — so a second pass is a no-op success.
    #[test]
    fn policy_directory_creation_is_idempotent_when_directory_already_exists() {
        let root = tempfile::tempdir().expect("workspace");
        let command = policy_directory_command();

        let first = execute_command_in(command, root.path());
        assert!(first.success, "first create stderr: {}", first.stderr);
        assert_eq!(first.exit_code, Some(0));
        assert!(root.path().join(".anvil/policies").is_dir());

        let second = execute_command_in(command, root.path());
        assert!(
            second.success,
            "re-run must not fail when the directory already exists; stderr: {}",
            second.stderr
        );
        assert_eq!(second.exit_code, Some(0));
        assert!(root.path().join(".anvil/policies").is_dir());
    }

    /// CIB-349: the fallback classifier matches the welcome/tutorial
    /// commands that would hit the licence wall, and leaves the free
    /// `--verify` probes and non-anvil operations alone.
    #[test]
    fn fallback_licence_gate_matches_cib_349_commands() {
        assert!(command_needs_licence_gate_fallback("anvil policy test"));
        assert!(command_needs_licence_gate_fallback("anvil gate"));
        assert!(command_needs_licence_gate_fallback(
            "anvil architecture validate"
        ));
        assert!(command_needs_licence_gate_fallback("anvil status --json"));
        assert!(!command_needs_licence_gate_fallback("anvil start --verify"));
        assert!(!command_needs_licence_gate_fallback(
            "anvil status --verify"
        ));
        assert!(!command_needs_licence_gate_fallback(
            "anvil gctx egress status"
        ));
        assert!(!command_needs_licence_gate_fallback(
            "mkdir -p .anvil/policies"
        ));
    }

    #[test]
    fn apply_sign_in_bridge_rewrites_gated_command_steps() {
        let mut steps = vec![super::super::TutorialStep {
            title: "Test the Policy".to_string(),
            description: "Before enforcing a policy, confirm anvil can exercise it.".to_string(),
            instruction: "Run: anvil policy test to execute your Rego tests.".to_string(),
            command: Some("anvil policy test".to_string()),
            effect: Some(super::super::CommandEffect::ReadOnly),
            verify: Some(super::super::verify::Verify::ExitCode(0)),
            verify_hint: Some("failed".to_string()),
            ..super::super::TutorialStep::default()
        }];
        apply_sign_in_bridge(&mut steps, command_needs_licence_gate_fallback);
        let step = &steps[0];
        assert!(step.sign_in_bridge);
        assert!(step.command.is_none(), "must not stay a runnable check");
        assert!(step.effect.is_none());
        assert!(step.verify.is_none());
        assert!(
            step.instruction.contains(AUTH_LOGIN_COMMAND),
            "bridge must name auth login before the gated command: {}",
            step.instruction
        );
        assert!(
            step.instruction.contains("anvil policy test"),
            "bridge must still name the deferred command: {}",
            step.instruction
        );
    }

    #[test]
    fn apply_sign_in_bridge_rewrites_informational_run_now_copy() {
        let mut steps = vec![super::super::TutorialStep {
            title: "See the Policy Fire".to_string(),
            description: "Add a TODO comment.".to_string(),
            instruction: "Run: anvil gate to evaluate policies against the codebase.".to_string(),
            ..super::super::TutorialStep::default()
        }];
        apply_sign_in_bridge(&mut steps, command_needs_licence_gate_fallback);
        let step = &steps[0];
        assert!(step.sign_in_bridge);
        assert!(
            step.instruction.contains(AUTH_LOGIN_COMMAND),
            "informational run-now copy must name login first: {}",
            step.instruction
        );
        assert!(
            step.instruction.to_ascii_lowercase().contains("anvil gate"),
            "deferred gate command must still be named: {}",
            step.instruction
        );
    }

    #[test]
    fn apply_sign_in_bridge_leaves_verify_probes_runnable() {
        let mut steps = vec![super::super::TutorialStep {
            title: "Verify".to_string(),
            description: "Read-only probe.".to_string(),
            instruction: "Run: anvil start --verify".to_string(),
            command: Some("anvil start --verify".to_string()),
            effect: Some(super::super::CommandEffect::ReadOnly),
            ..super::super::TutorialStep::default()
        }];
        apply_sign_in_bridge(&mut steps, command_needs_licence_gate_fallback);
        assert!(!steps[0].sign_in_bridge);
        assert_eq!(steps[0].command.as_deref(), Some("anvil start --verify"));
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
    fn shell_command_runs_from_bound_root() {
        let root = tempfile::tempdir().expect("workspace");

        let output = execute_command_in("pwd", root.path());

        assert!(output.success, "stderr: {}", output.stderr);
        assert_eq!(
            dunce::canonicalize(std::path::Path::new(output.stdout.trim())).expect("reported cwd"),
            dunce::canonicalize(root.path()).expect("workspace root")
        );
    }

    #[test]
    fn policy_directory_creation_rejects_symlinked_parent() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        symlink(outside.path(), root.path().join(".anvil")).expect("symlink policy parent");

        let output = execute_command_in("mkdir -p .anvil/policies", root.path());

        assert!(!output.success, "symlinked policy parent must fail closed");
        assert!(
            output.stderr.contains(".anvil/policies"),
            "the error must identify the safe relative target: {}",
            output.stderr
        );
        assert!(
            !outside.path().join("policies").exists(),
            "directory creation must not escape the bound workspace"
        );
    }

    #[test]
    fn fixed_anvil_command_ignores_fake_path_binary() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().expect("workspace");
        let marker = root.path().join("fake-selected");
        let fake = root.path().join("anvil");
        std::fs::write(
            &fake,
            format!("#!/bin/sh\nprintf selected > '{}'\n", marker.display()),
        )
        .expect("fake anvil");
        let mut permissions = std::fs::metadata(&fake).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake, permissions).expect("executable");

        let mut process = process_for_command(
            "anvil cib250-never-select-path-binary",
            root.path(),
            std::env::current_exe,
        )
        .expect("fixed anvil command");
        process.env("PATH", root.path());
        let _ = process.output().expect("run current executable");

        assert!(!marker.exists(), "PATH-resolved fake anvil was executed");
    }

    #[test]
    fn fixed_anvil_command_rejects_shell_syntax() {
        let root = tempfile::tempdir().expect("workspace");
        assert!(
            process_for_command(
                "anvil check src/app.ts && touch escaped",
                root.path(),
                std::env::current_exe,
            )
            .is_err()
        );
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
            AutoplayCommand::spawn("anvil check src/app.ts", root.path(), Some(&runner))
                .expect("spawn");
        while !command.is_finished().expect("poll") {
            std::thread::yield_now();
        }
        let output = command.finish();

        assert!(output.success);
        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.contains("AP-003"));

        // `resolve_working_path` canonicalises the target (macOS: `/var` →
        // `/private/var`), and the runner must never see a path outside root.
        let expected_target = dunce::canonicalize(root.path())
            .expect("canonicalize root")
            .join("src/app.ts");
        let observed = seen.lock().expect("read target").clone().expect("invoked");
        assert_eq!(observed, expected_target);
    }

    fn unreachable_runner() -> AutoplayRunner {
        std::sync::Arc::new(|_: &std::path::Path| unreachable!("runner must not be reached"))
    }

    /// CIB-248 / CIB-249 teardown: a panicking runner must surface as a failed
    /// step, not as an unwind off the worker thread.
    ///
    /// An escaping panic would reach the host's process-wide hook, which
    /// restores the terminal while the TUI is still drawing; the subprocess
    /// implementation this replaced contained panics inside the child. The
    /// caught panic becomes a `CommandOutput` failure, which the existing
    /// `take_autoplay_failure` → `recover_from_autoplay_failure` path already
    /// handles inside the TUI.
    ///
    /// The default panic hook still prints "thread ... panicked" to the test
    /// harness's stderr; that is expected noise here. In the real binary, the
    /// host hook suppresses it only while the explicit catch boundary is active.
    #[test]
    fn panicking_runner_becomes_a_failed_step_instead_of_unwinding() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src/app.ts"), "fixture").expect("fixture");

        let runner: AutoplayRunner = std::sync::Arc::new(|_: &std::path::Path| {
            assert!(
                is_autoplay_panic_contained(),
                "the catch boundary must be active before the runner can panic"
            );
            panic!("demo runner exploded");
        });

        let mut command =
            AutoplayCommand::spawn("anvil check src/app.ts", root.path(), Some(&runner))
                .expect("spawn");
        while !command.is_finished().expect("poll") {
            std::thread::yield_now();
        }
        let output = command.finish();

        assert!(!output.success, "a panic must not report success");
        assert_eq!(output.exit_code, None);
        assert!(
            output.stderr.contains("panicked") && output.stderr.contains("demo runner exploded"),
            "the panic payload must reach the failure message: {}",
            output.stderr
        );
    }

    /// The worker keeps its stable diagnostic name, but CIB-269 ensures the
    /// host panic hook no longer treats the name itself as containment.
    #[test]
    fn autoplay_worker_keeps_its_diagnostic_thread_name() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src/app.ts"), "fixture").expect("fixture");

        let seen: std::sync::Arc<std::sync::Mutex<Option<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let recorder = std::sync::Arc::clone(&seen);
        let runner: AutoplayRunner = std::sync::Arc::new(move |_: &std::path::Path| {
            *recorder.lock().expect("record name") =
                std::thread::current().name().map(ToString::to_string);
            CommandOutput {
                stdout: String::new(),
                stderr: String::new(),
                success: true,
                exit_code: Some(0),
            }
        });

        let mut command =
            AutoplayCommand::spawn("anvil check src/app.ts", root.path(), Some(&runner))
                .expect("spawn");
        while !command.is_finished().expect("poll") {
            std::thread::yield_now();
        }
        let _ = command.finish();

        assert_eq!(
            seen.lock().expect("read name").as_deref(),
            Some(AUTOPLAY_WORKER_THREAD)
        );
    }

    #[test]
    fn autoplay_panic_containment_is_thread_local_and_scoped() {
        assert!(!is_autoplay_panic_contained());

        let result = catch_autoplay_panic(|| {
            assert!(is_autoplay_panic_contained());
            std::thread::spawn(|| assert!(!is_autoplay_panic_contained()))
                .join()
                .expect("other thread probe");
            panic!("containment scope probe");
        });

        assert!(result.is_err(), "the scope probe should be caught");
        assert!(
            !is_autoplay_panic_contained(),
            "containment must clear after catch_unwind returns"
        );
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
            AutoplayCommand::spawn(
                "anvil check linked/app.ts",
                root.path(),
                Some(&unreachable_runner())
            )
            .is_err()
        );
    }
}
