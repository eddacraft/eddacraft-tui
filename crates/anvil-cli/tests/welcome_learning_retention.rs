//! JREL-006: project first-run state must never erase user-global learning.

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
