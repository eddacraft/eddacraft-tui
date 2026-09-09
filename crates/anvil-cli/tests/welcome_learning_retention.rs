//! JREL-006/JREL-007: learning retention and selected-project ownership.

#[cfg(unix)]
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::time::{Duration, Instant};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn welcome_command(workspace: &Path, home: &Path) -> Command {
    let mut command = Command::new(ANVIL_BIN);
    for variable in [
        "CI",
        "ANVIL_HOME",
        "ANVIL_SKIP_WELCOME",
        "ANVIL_NO_TUI",
        "ANVIL_NO_PROMPT",
        "NONINTERACTIVE",
        "GIT_DIR",
        "GIT_INDEX_FILE",
    ] {
        command.env_remove(variable);
    }
    command
        .current_dir(workspace)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("ANVIL_DEV", "1")
        .env("ANVIL_NO_DAEMON", "1")
        .env("ANVIL_NO_MCP", "1");
    command
}

#[cfg(unix)]
struct PtyRun {
    status: std::process::ExitStatus,
    transcript: String,
}

#[cfg(unix)]
fn occurrence_count(bytes: &[u8], needle: &[u8]) -> usize {
    bytes
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[cfg(unix)]
fn run_welcome_script(workspace: &Path, home: &Path, actions: &[(&[u8], &[u8])]) -> PtyRun {
    run_welcome_script_with_anvil_home(workspace, home, None, actions)
}

#[cfg(unix)]
fn run_welcome_script_with_anvil_home(
    workspace: &Path,
    home: &Path,
    anvil_home: Option<&Path>,
    actions: &[(&[u8], &[u8])],
) -> PtyRun {
    let size = nix::pty::Winsize {
        ws_row: 30,
        ws_col: 120,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let pty = nix::pty::openpty(Some(&size), None).expect("open PTY");
    let mut master = std::fs::File::from(pty.master);
    let slave = std::fs::File::from(pty.slave);
    let stdin = slave.try_clone().expect("clone PTY stdin");
    let stdout = slave.try_clone().expect("clone PTY stdout");

    let mut command = welcome_command(workspace, home);
    if let Some(anvil_home) = anvil_home {
        command.env("ANVIL_HOME", anvil_home);
    }
    command
        .arg("welcome")
        .env("TERM", "xterm-256color")
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(slave.try_clone().expect("clone PTY stderr")));

    let mut child = command.spawn().expect("spawn welcome in PTY");
    drop(slave);
    nix::fcntl::fcntl(
        &master,
        nix::fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::O_NONBLOCK),
    )
    .expect("set PTY master non-blocking");

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut next_action = 0;
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        match master.read(&mut buffer) {
            Ok(0) => {}
            Ok(read) => bytes.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) if error.raw_os_error() == Some(libc::EIO) => {}
            Err(error) => panic!("read PTY output: {error}"),
        }

        if let Some((needle, input)) = actions.get(next_action)
            && occurrence_count(&bytes, b"\x1b[?1049h") >= 1
            && bytes.windows(needle.len()).any(|window| window == *needle)
        {
            master.write_all(input).expect("send scripted input");
            master.flush().expect("flush scripted input");
            next_action += 1;
        }

        if let Some(status) = child.try_wait().expect("poll welcome child") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill hung welcome child");
            child.wait().expect("reap hung welcome child");
            panic!(
                "`anvil welcome` did not complete after quit:\n{}",
                String::from_utf8_lossy(&bytes)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    while let Ok(read) = master.read(&mut buffer) {
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
    }

    PtyRun {
        status,
        transcript: String::from_utf8_lossy(&bytes).into_owned(),
    }
}

#[cfg(unix)]
fn run_welcome_and_quit(workspace: &Path, home: &Path) -> PtyRun {
    run_welcome_script(workspace, home, &[(b"esc/q quit", b"q")])
}

#[cfg(unix)]
#[test]
fn first_run_in_repo_b_preserves_repo_a_learning_when_user_quits_immediately() {
    let home = tempfile::tempdir().expect("isolated user home");
    let repo_a = tempfile::tempdir().expect("repo A");
    let repo_b = tempfile::tempdir().expect("repo B");
    std::fs::create_dir(repo_a.path().join(".git")).expect("repo A git marker");
    std::fs::create_dir(repo_b.path().join(".git")).expect("repo B git marker");

    let progress_path = home.path().join(".anvil/tutorial-progress.json");
    std::fs::create_dir_all(progress_path.parent().expect("progress parent"))
        .expect("create progress parent");
    let mut workspace_sessions = serde_json::Map::new();
    workspace_sessions.insert(
        repo_a.path().to_string_lossy().into_owned(),
        serde_json::json!({
            "path": "Policy checks",
            "current_step": 1,
            "steps_completed": [true, false]
        }),
    );
    let learned_in_repo_a = serde_json::to_vec_pretty(&serde_json::json!({
        "completed_paths": ["Architecture"],
        "workspace_sessions": workspace_sessions,
        "in_progress": {
            "path": "Policy",
            "current_step": 1,
            "steps_completed": [true, false]
        }
    }))
    .expect("serialise repo A learning");
    std::fs::write(&progress_path, &learned_in_repo_a).expect("seed repo A learning");

    let result = run_welcome_and_quit(repo_b.path(), home.path());

    assert!(
        result.status.success(),
        "welcome quit failed:\n{}",
        result.transcript
    );
    assert_eq!(
        std::fs::read(&progress_path).expect("repo A learning must remain"),
        learned_in_repo_a,
        "a missing project marker in repo B must not reset user-global learning"
    );
    assert!(
        !repo_b.path().join(".anvil/first-run").exists(),
        "quitting first-run setup must leave it deferred rather than completed"
    );
}

#[cfg(unix)]
#[test]
fn repo_b_learning_path_shows_repo_a_completion() {
    let home = tempfile::tempdir().expect("isolated user home");
    let repo_b = tempfile::tempdir().expect("repo B");
    std::fs::create_dir(repo_b.path().join(".git")).expect("repo B git marker");

    let progress_path = home.path().join(".anvil/tutorial-progress.json");
    std::fs::create_dir_all(progress_path.parent().expect("progress parent"))
        .expect("create progress parent");
    std::fs::write(&progress_path, br#"{"completed_paths":["Architecture"]}"#)
        .expect("seed repo A completion");

    let result = run_welcome_script(
        repo_b.path(),
        home.path(),
        &[(b"esc/q quit", b"\x1b[B\r"), (b"(redo)", b"q")],
    );

    assert!(
        result.status.success(),
        "welcome tutorial quit failed:\n{}",
        result.transcript
    );
    assert!(
        result.transcript.contains("(redo)"),
        "the welcome learning-path picker must render retained completion:\n{}",
        result.transcript
    );
    assert!(
        !repo_b.path().join(".anvil/first-run").exists(),
        "quitting the tutorial picker must leave first-run setup deferred"
    );
}

#[cfg(unix)]
#[test]
fn backing_out_of_learning_path_keeps_first_run_deferred() {
    let home = tempfile::tempdir().expect("isolated user home");
    let workspace = tempfile::tempdir().expect("new repository");
    std::fs::create_dir(workspace.path().join(".git")).expect("repository git marker");

    let progress_path = home.path().join(".anvil/tutorial-progress.json");
    std::fs::create_dir_all(progress_path.parent().expect("progress parent"))
        .expect("create progress parent");
    std::fs::write(&progress_path, br#"{"completed_paths":["Architecture"]}"#)
        .expect("seed completed learning path");

    let result = run_welcome_script(
        workspace.path(),
        home.path(),
        &[
            (b"esc/q quit", b"\x1b[B\r"),
            (b"(redo)", b"\x1b"),
            (b"Review gate decision", b"q"),
        ],
    );

    assert!(
        result.status.success(),
        "welcome back flow failed:\n{}",
        result.transcript
    );
    assert!(
        !workspace.path().join(".anvil/first-run").exists(),
        "backing out of a follow-on surface must leave first-run setup deferred"
    );
}

#[cfg(unix)]
#[test]
fn completing_learning_path_marks_first_run_complete() {
    let home = tempfile::tempdir().expect("isolated user home");
    let workspace = tempfile::tempdir().expect("new repository");
    std::fs::create_dir(workspace.path().join(".git")).expect("repository git marker");

    let result = run_welcome_script(
        workspace.path(),
        home.path(),
        &[
            (b"esc/q quit", b"\x1b[B\r"),
            // Select the five-step informational path, advance every step,
            // then quit directly from the completion screen.
            (b"anvil's protection loop", b"\r     q"),
        ],
    );

    assert!(
        result.status.success(),
        "welcome completion flow failed:\n{}",
        result.transcript
    );
    assert!(
        workspace.path().join(".anvil/first-run").is_file(),
        "completing a real learning path must create first-run completion evidence"
    );
}

#[cfg(unix)]
#[test]
fn guided_setup_keeps_config_scan_fix_tutorial_and_marker_on_the_selected_repo() {
    let home = tempfile::tempdir().expect("isolated user home");
    let parent = tempfile::tempdir().expect("repository parent");
    let repo_a = parent.path().join("repo-a");
    let repo_b = parent.path().join("repo-b");
    std::fs::create_dir_all(repo_a.join(".git")).expect("repo A git marker");
    std::fs::create_dir_all(repo_b.join(".git")).expect("repo B git marker");
    let source = "const selectedValue: any = source;\n";
    std::fs::write(repo_b.join("b-only.ts"), source).expect("repo B finding");

    let mut directory_input = repo_b.as_os_str().as_encoded_bytes().to_vec();
    directory_input.push(b'\r');
    let result = run_welcome_script(
        &repo_a,
        home.path(),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\r"),
            (b"YAML (.anvil.yaml) (default)", b"\r"),
            (b"Enter the project root directory:", &directory_input),
            (b"secret-detection", b"\r"),
            (b"Press enter to confirm", b"\r"),
            (b"anvil is ready", b"\r"),
            (b"b-only.ts", b"\r\r"),
            (b"Apply this fix to b-only.ts:1", b" a"),
            (b"Fix applied", b"\r"),
            (b"anvil's protection loop", b"\r     q"),
        ],
    );

    assert!(
        result.status.success(),
        "guided setup failed:\n{}",
        result.transcript
    );
    assert!(repo_b.join(".anvil.yaml").is_file(), "config must target B");
    assert!(
        repo_b.join(".anvil/first-run").is_file(),
        "completion marker must target B"
    );
    assert!(
        std::fs::read_to_string(repo_b.join("b-only.ts"))
            .expect("selected source")
            .contains(": unknown"),
        "the consented first-win fix must target B"
    );
    assert!(
        !repo_a.join(".anvil.yaml").exists() && !repo_a.join(".anvil").exists(),
        "launch repository A must remain unchanged"
    );
}

#[cfg(unix)]
#[test]
fn guided_setup_refuses_selected_project_writes_under_gated_anvil_home() {
    let home = tempfile::tempdir().expect("isolated user home");
    let install_root = tempfile::tempdir().expect("non-default install root");
    let parent = tempfile::tempdir().expect("repository parent");
    let repo_a = parent.path().join("repo-a");
    let repo_b = parent.path().join("repo-b");
    std::fs::create_dir_all(repo_a.join(".git")).expect("repo A git marker");
    std::fs::create_dir_all(repo_b.join(".git")).expect("repo B git marker");

    let mut directory_input = repo_b.as_os_str().as_encoded_bytes().to_vec();
    directory_input.push(b'\r');
    let result = run_welcome_script_with_anvil_home(
        &repo_a,
        home.path(),
        Some(install_root.path()),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\r"),
            (b"YAML (.anvil.yaml) (default)", b"\r"),
            (b"Enter the project root directory:", &directory_input),
            (b"secret-detection", b"\r"),
            (b"Press enter to confirm", b"\r"),
            (b"anvil is ready", b"q"),
        ],
    );

    assert!(
        !result.status.success(),
        "gated guided setup must fail closed:\n{}",
        result.transcript
    );
    assert!(
        result.transcript.contains("--touch-project-state"),
        "the refusal must explain how to grant project-write consent:\n{}",
        result.transcript
    );
    assert!(!repo_a.join(".anvil.yaml").exists());
    assert!(!repo_b.join(".anvil.yaml").exists());
    assert!(!repo_a.join(".anvil/first-run").exists());
    assert!(!repo_b.join(".anvil/first-run").exists());
}

#[cfg(unix)]
#[test]
fn preconfigured_launch_project_does_not_hide_a_different_selected_project() {
    let home = tempfile::tempdir().expect("isolated user home");
    let parent = tempfile::tempdir().expect("repository parent");
    let repo_a = parent.path().join("repo-a");
    let repo_b = parent.path().join("repo-b");
    std::fs::create_dir_all(repo_a.join(".git")).expect("repo A git marker");
    std::fs::create_dir_all(repo_b.join(".git")).expect("repo B git marker");
    let original_a = "version: 1\nchecks: []\n";
    std::fs::write(repo_a.join(".anvil.yaml"), original_a).expect("repo A config");

    let mut directory_input = repo_b.as_os_str().as_encoded_bytes().to_vec();
    directory_input.push(b'\r');
    let result = run_welcome_script(
        &repo_a,
        home.path(),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\r"),
            (b"YAML (.anvil.yaml) (default)", b"\r"),
            (b"Enter the project root directory:", &directory_input),
            (b"secret-detection", b"\r"),
            (b"Press enter to confirm", b"\r"),
            (b"anvil is ready", b"q"),
        ],
    );

    assert!(
        result.status.success(),
        "guided setup failed:\n{}",
        result.transcript
    );
    assert!(repo_b.join(".anvil.yaml").is_file(), "B must be configured");
    assert_eq!(
        std::fs::read_to_string(repo_a.join(".anvil.yaml")).expect("repo A config"),
        original_a,
        "A's existing configuration must remain unchanged"
    );
}

#[cfg(unix)]
#[test]
fn selected_project_config_detection_uses_b_and_preserves_its_config() {
    let home = tempfile::tempdir().expect("isolated user home");
    let parent = tempfile::tempdir().expect("repository parent");
    let repo_a = parent.path().join("repo-a");
    let repo_b = parent.path().join("repo-b");
    std::fs::create_dir_all(repo_a.join(".git")).expect("repo A git marker");
    std::fs::create_dir_all(repo_b.join(".git")).expect("repo B git marker");
    let original_b = "version: 1\nchecks: []\n";
    std::fs::write(repo_b.join(".anvil.yaml"), original_b).expect("repo B config");
    std::fs::write(repo_b.join("b-only.ts"), "const value: any = source;\n")
        .expect("repo B source");

    let mut directory_input = repo_b.as_os_str().as_encoded_bytes().to_vec();
    directory_input.push(b'\r');
    let result = run_welcome_script(
        &repo_a,
        home.path(),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\r"),
            (b"YAML (.anvil.yaml) (default)", b"\r"),
            (b"Enter the project root directory:", &directory_input),
            (b"secret-detection", b"\r"),
            (b"Press enter to confirm", b"\r"),
            (b"b-only.ts", b"q"),
            (b"Review gate decision", b"q"),
        ],
    );

    assert!(
        result.status.success(),
        "guided setup failed:\n{}",
        result.transcript
    );
    assert_eq!(
        std::fs::read_to_string(repo_b.join(".anvil.yaml")).expect("repo B config"),
        original_b,
        "B's existing configuration must not be overwritten"
    );
    assert!(!repo_a.join(".anvil.yaml").exists());
}

#[cfg(unix)]
#[test]
fn guided_setup_from_a_subdirectory_defaults_to_the_git_worktree_root() {
    let home = tempfile::tempdir().expect("isolated user home");
    let repo = tempfile::tempdir().expect("repository");
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo.path())
        .status()
        .expect("initialise git repository");
    assert!(git.success());
    let nested = repo.path().join("nested/deeper");
    std::fs::create_dir_all(&nested).expect("nested launch directory");

    let result = run_welcome_script(
        &nested,
        home.path(),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\r"),
            (b"YAML (.anvil.yaml) (default)", b"\r"),
            (b"Enter the project root directory:", b"\r"),
            (b"secret-detection", b"\r"),
            (b"Press enter to confirm", b"\r"),
            (b"anvil is ready", b"q"),
        ],
    );

    assert!(
        result.status.success(),
        "subdirectory flow failed:\n{}",
        result.transcript
    );
    assert!(repo.path().join(".anvil.yaml").is_file());
    assert!(!nested.join(".anvil.yaml").exists());
    assert!(!repo.path().join(".anvil/first-run").exists());
}

#[cfg(unix)]
#[test]
fn guided_setup_from_a_linked_worktree_stays_on_that_worktree() {
    let home = tempfile::tempdir().expect("isolated user home");
    let parent = tempfile::tempdir().expect("repository parent");
    let main = parent.path().join("main");
    let linked = parent.path().join("linked");
    std::fs::create_dir(&main).expect("main worktree");
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--allow-empty",
            "-q",
            "-m",
            "fixture",
        ],
    ] {
        let status = Command::new("git")
            .args(args)
            .current_dir(&main)
            .status()
            .expect("prepare git repository");
        assert!(status.success());
    }
    let status = Command::new("git")
        .args([
            "worktree",
            "add",
            "-q",
            "-b",
            "linked",
            linked.to_str().expect("utf-8 path"),
        ])
        .current_dir(&main)
        .status()
        .expect("create linked worktree");
    assert!(status.success());

    let result = run_welcome_script(
        &linked,
        home.path(),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\r"),
            (b"YAML (.anvil.yaml) (default)", b"\r"),
            (b"Enter the project root directory:", b"\r"),
            (b"secret-detection", b"\r"),
            (b"Press enter to confirm", b"\r"),
            (b"anvil is ready", b"q"),
        ],
    );

    assert!(
        result.status.success(),
        "worktree flow failed:\n{}",
        result.transcript
    );
    assert!(linked.join(".anvil.yaml").is_file());
    assert!(!main.join(".anvil.yaml").exists());
    assert!(!linked.join(".anvil/first-run").exists());
}

#[cfg(unix)]
#[test]
fn guided_setup_cancel_at_each_wizard_stage_never_marks_first_run_complete() {
    type ActionScript<'a> = &'a [(&'a [u8], &'a [u8])];
    type CancelCase<'a> = (&'a str, ActionScript<'a>, bool);
    let cases: &[CancelCase<'_>] = &[
        (
            "mode",
            &[(b"Set up this project", b"\r"), (b"Select Mode", b"q")],
            false,
        ),
        (
            "format",
            &[
                (b"Set up this project", b"\r"),
                (b"Select Mode", b"\r"),
                (b"YAML (.anvil.yaml) (default)", b"q"),
            ],
            false,
        ),
        (
            "directory",
            &[
                (b"Set up this project", b"\r"),
                (b"Select Mode", b"\r"),
                (b"YAML (.anvil.yaml) (default)", b"\r"),
                (b"Enter the project root directory:", b"\x03"),
            ],
            false,
        ),
        (
            "checks",
            &[
                (b"Set up this project", b"\r"),
                (b"Select Mode", b"\r"),
                (b"YAML (.anvil.yaml) (default)", b"\r"),
                (b"Enter the project root directory:", b"\r"),
                (b"secret-detection", b"q"),
            ],
            false,
        ),
        (
            "summary",
            &[
                (b"Set up this project", b"\r"),
                (b"Select Mode", b"\r"),
                (b"YAML (.anvil.yaml) (default)", b"\r"),
                (b"Enter the project root directory:", b"\r"),
                (b"secret-detection", b"\r"),
                (b"Press enter to confirm", b"q"),
            ],
            false,
        ),
        (
            "landing",
            &[
                (b"Set up this project", b"\r"),
                (b"Select Mode", b"\r"),
                (b"YAML (.anvil.yaml) (default)", b"\r"),
                (b"Enter the project root directory:", b"\r"),
                (b"secret-detection", b"\r"),
                (b"Press enter to confirm", b"\r"),
                (b"anvil is ready", b"q"),
            ],
            true,
        ),
    ];

    for (stage, actions, config_written) in cases {
        let home = tempfile::tempdir().expect("isolated user home");
        let repo = tempfile::tempdir().expect("repository");
        std::fs::create_dir(repo.path().join(".git")).expect("git marker");
        let result = run_welcome_script(repo.path(), home.path(), actions);
        assert!(
            result.status.success(),
            "{stage} cancellation failed:\n{}",
            result.transcript
        );
        assert_eq!(
            repo.path().join(".anvil.yaml").exists(),
            *config_written,
            "{stage}"
        );
        assert!(
            !repo.path().join(".anvil/first-run").exists(),
            "{stage} cancellation must not create completion evidence"
        );
    }
}

#[cfg(unix)]
#[test]
fn back_from_the_first_wizard_step_returns_to_onboarding_before_quit() {
    let home = tempfile::tempdir().expect("isolated user home");
    let repo = tempfile::tempdir().expect("repository");
    std::fs::create_dir(repo.path().join(".git")).expect("git marker");

    let result = run_welcome_script(
        repo.path(),
        home.path(),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\x1b"),
            (b"Let's get you set up.", b"q"),
        ],
    );

    assert!(
        result.status.success(),
        "back flow failed:\n{}",
        result.transcript
    );
    assert!(occurrence_count(result.transcript.as_bytes(), b"Let's get you set up.") >= 2);
    assert!(!repo.path().join(".anvil.yaml").exists());
    assert!(!repo.path().join(".anvil/first-run").exists());
}

#[cfg(unix)]
#[test]
fn guided_config_write_failure_exits_before_discovery_or_completion() {
    let home = tempfile::tempdir().expect("isolated user home");
    let parent = tempfile::tempdir().expect("repository parent");
    let repo_a = parent.path().join("repo-a");
    let repo_b = parent.path().join("repo-b");
    std::fs::create_dir_all(repo_a.join(".git")).expect("repo A git marker");
    std::fs::create_dir_all(repo_b.join(".git")).expect("repo B git marker");
    std::fs::create_dir(repo_b.join(".anvil.yaml")).expect("block config path");
    let mut directory_input = repo_b.as_os_str().as_encoded_bytes().to_vec();
    directory_input.push(b'\r');

    let result = run_welcome_script(
        &repo_a,
        home.path(),
        &[
            (b"Set up this project", b"\r"),
            (b"Select Mode", b"\r"),
            (b"YAML (.anvil.yaml) (default)", b"\r"),
            (b"Enter the project root directory:", &directory_input),
            (b"secret-detection", b"\r"),
            (b"Press enter to confirm", b"\r"),
        ],
    );

    assert!(!result.status.success(), "write failure must be an error");
    assert!(
        result
            .transcript
            .contains("could not save config for the selected project"),
        "missing selected-project error:\n{}",
        result.transcript
    );
    assert!(!result.transcript.contains("Scanning project..."));
    assert!(!repo_a.join(".anvil").exists());
    assert!(!repo_b.join(".anvil/first-run").exists());
}

#[test]
fn plain_and_json_welcome_preserve_user_global_learning() {
    for args in [&["--no-tui", "welcome"][..], &["--json", "welcome"][..]] {
        let home = tempfile::tempdir().expect("isolated user home");
        let workspace = tempfile::tempdir().expect("new repository");
        let progress_path = home.path().join(".anvil/tutorial-progress.json");
        std::fs::create_dir_all(progress_path.parent().expect("progress parent"))
            .expect("create progress parent");
        let progress = br#"{"completed_paths":["Architecture"]}"#;
        std::fs::write(&progress_path, progress).expect("seed learning");

        let output = welcome_command(workspace.path(), home.path())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("run non-interactive welcome");

        assert!(
            output.status.success(),
            "welcome {args:?} failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            std::fs::read(&progress_path).expect("learning must remain"),
            progress,
            "welcome {args:?} must not reset user-global learning"
        );
    }
}

#[test]
fn failed_project_marker_write_preserves_user_global_learning() {
    let home = tempfile::tempdir().expect("isolated user home");
    let workspace = tempfile::tempdir().expect("new repository");
    let progress_path = home.path().join(".anvil/tutorial-progress.json");
    std::fs::create_dir_all(progress_path.parent().expect("progress parent"))
        .expect("create progress parent");
    let progress = br#"{"completed_paths":["Architecture"]}"#;
    std::fs::write(&progress_path, progress).expect("seed learning");
    std::fs::write(workspace.path().join(".anvil"), "not a directory")
        .expect("block project marker directory");

    let output = welcome_command(workspace.path(), home.path())
        .args(["--json", "welcome"])
        .stdin(Stdio::null())
        .output()
        .expect("run failing welcome");

    assert!(
        !output.status.success(),
        "fixture must exercise a failed welcome: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(&progress_path).expect("learning must remain after failure"),
        progress
    );
}

#[test]
fn explicit_welcome_reset_deliberately_clears_user_global_learning() {
    let home = tempfile::tempdir().expect("isolated user home");
    let workspace = tempfile::tempdir().expect("repository");
    let progress_path = home.path().join(".anvil/tutorial-progress.json");
    std::fs::create_dir_all(progress_path.parent().expect("progress parent"))
        .expect("create progress parent");
    std::fs::write(&progress_path, br#"{"completed_paths":["Architecture"]}"#)
        .expect("seed learning");

    let output = welcome_command(workspace.path(), home.path())
        .args(["--json", "welcome", "--reset"])
        .stdin(Stdio::null())
        .output()
        .expect("run explicit welcome reset");

    assert!(
        output.status.success(),
        "explicit reset failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !progress_path.exists(),
        "the explicit reset contract must still clear tutorial progress"
    );
}
