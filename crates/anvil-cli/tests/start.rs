//! Integration tests for `anvil start` activation (LAUNCH-006 / 009).
//!
//! Uses isolated `HOME` so MCP probes never touch the developer machine.
//! Fresh repo must never claim `protecting`.

use std::fs;
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(not(target_os = "windows"))]
use std::path::Path;
use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
// The start-activation fixture tests are `cfg(not(target_os = "windows"))`,
// so their helpers carry the same gate.
#[cfg(not(target_os = "windows"))]
const START_ACTIVATION_FIXTURES: &str = "tests/fixtures/start-activation";

fn run_start_with_home(
    workdir: &std::path::Path,
    home: &std::path::Path,
    extra_args: &[&str],
) -> std::process::Output {
    let mut cmd = start_command_env(workdir, home);
    cmd.arg("--no-tui").arg("start").args(extra_args);
    cmd.output().expect("failed to invoke anvil binary")
}

/// Shared hermetic environment for spawning the anvil binary: per-test
/// `HOME`, pinned daemon paths, scrubbed tracing filters, and pinned
/// agent detection. Callers add their own args (and any per-test env
/// such as `ANVIL_NO_TUI`) on top.
fn start_command_env(workdir: &std::path::Path, home: &std::path::Path) -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.current_dir(workdir)
        .env("HOME", home)
        // Windows daemon state prefers LOCALAPPDATA and home discovery uses
        // USERPROFILE; macOS / Linux use HOME. Pin all three so the same test
        // bench cannot reuse a developer daemon on any platform.
        .env("USERPROFILE", home)
        .env("LOCALAPPDATA", home)
        // ANVIL_HOME also gives Windows a per-test named-pipe namespace. Keep
        // project writes enabled so this isolation does not change the start
        // behaviour the integration tests exercise.
        .env("ANVIL_HOME", home.join("anvil-home"))
        .env("ANVIL_TOUCH_PROJECT_STATE", "1")
        // Strip XDG so dirs::home_dir() doesn't resolve to a user
        // directory through XDG_CONFIG_HOME.
        .env_remove("XDG_CONFIG_HOME")
        // CIB-162: pin the tracing filter to the CLI default (`warn`) by
        // stripping any inherited `ANVIL_LOG` / `RUST_LOG` from the
        // developer's shell. The default-filter human surface must not
        // be interrupted by raw JSONL tracing lines; a leaked
        // `ANVIL_LOG=info` would both re-admit those lines and make the
        // skip-warning regression test non-hermetic.
        .env_remove("ANVIL_LOG")
        .env_remove("RUST_LOG")
        // CIB-270: baseline start tests own their daemon and MCP policies
        // explicitly. Do not inherit developer opt-outs; env semantics are
        // covered by child-process tests below.
        .env_remove("ANVIL_NO_DAEMON")
        .env_remove("ANVIL_NO_MCP")
        // DLIFE-003: pin the daemon socket/PID resolution to the per-test
        // tempdir so the daemon-ensure probe is deterministically isolated
        // from any real daemon on a developer box. The captured (non-TTY)
        // stdout means `anvil start` resolves a non-interactive context and
        // falls back without spawning — no `--no-daemon` needed for
        // hermeticity, so the harness exercises the real default path.
        // Unix-only env (ignored on Windows, which uses a per-user pipe).
        .env("XDG_RUNTIME_DIR", home)
        .env("ANVIL_DEV", "1")
        // ACTMO-012: `anvil start` now only writes a fresh MCP config for
        // editors it actually detects (binary on PATH / pre-existing
        // editor state). These mechanics tests assert both Cursor and
        // Claude Code entries are written, so force the all-clients
        // opt-in — otherwise the result would depend on whether the test
        // host happens to have `cursor` / `claude` on PATH, which is not
        // hermetic. The detection gate has its own dedicated coverage
        // (unit tests in `activation::orchestrator::install` and the
        // `start_without_detected_editor_does_not_write_mcp_config`
        // negative test below).
        .env("ANVIL_ALL_MCP_CLIENTS", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        // ACTTUI-007: the activation output embeds git's repo-discovery
        // stderr verbatim, and git words the "not a git repository" error
        // differently depending on where the upward walk stops (mount-point
        // variant when it hits a filesystem boundary, parent-directories
        // variant when it hits / or a ceiling). Pin a discovery ceiling at
        // the workdir's parent so the walk stops in the same place — with
        // the same wording — on every host, and so a stray repo above the
        // tempdir (e.g. a leftover /tmp/.git) can never be discovered as
        // the worktree. Note: git only honours a ceiling that is a proper
        // ancestor of the probed directory, so the workdir itself would be
        // a no-op; it must be the parent.
        .env(
            "GIT_CEILING_DIRECTORIES",
            workdir.parent().expect("test workdir has a parent"),
        );
    // ACTTUI-007: the byte-exact activation fixtures embed the "AI tools
    // detected" summary, and agent detection scans PATH binaries plus
    // ambient env vars. Without pinning, the fixtures would capture
    // whatever tooling the authoring host happens to run (claude/cursor/
    // codex on a dev box; nothing on CI) and fail everywhere else. Scrub
    // every detection env var and, on Unix, restrict PATH to a shim dir
    // containing only `git` and `anvil` so detection is deterministically
    // empty. `anvil` must stay resolvable: MCP handshake uses PATH only
    // (not current_exe).
    for var in [
        "ANTHROPIC_API_KEY",
        "CLAUDE_CODE_HOME",
        "CURSOR_HOME",
        "AIDER_MODEL",
        "AIDER_API_KEY",
        "WINDSURF_HOME",
        "CODEX_HOME",
    ] {
        cmd.env_remove(var);
    }
    #[cfg(unix)]
    cmd.env("PATH", git_only_path_shim(home));
    cmd
}

/// Build a PATH shim directory inside the per-test `home` containing
/// `git` and `anvil` only. `git` covers worktree probes; `anvil` is the
/// configured MCP command, so handshake can resolve it on PATH without
/// treating `current_exe` as live. Agent-binary detection still finds
/// nothing else.
#[cfg(unix)]
fn git_only_path_shim(home: &std::path::Path) -> std::path::PathBuf {
    let shim = home.join("path-shim");
    fs::create_dir_all(&shim).expect("create PATH shim dir");
    let git = std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join("git"))
                .find(|candidate| candidate.is_file())
        })
        .expect("git must be on PATH for the start test harness");
    let link = shim.join("git");
    if !link.exists() {
        std::os::unix::fs::symlink(&git, &link).expect("symlink git into PATH shim");
    }
    let anvil_link = shim.join("anvil");
    if !anvil_link.exists() {
        std::os::unix::fs::symlink(ANVIL_BIN, &anvil_link).expect("symlink anvil into PATH shim");
    }
    shim
}

#[cfg(not(target_os = "windows"))]
fn start_activation_fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(START_ACTIVATION_FIXTURES)
        .join(name)
}

#[cfg(not(target_os = "windows"))]
fn normalise_start_activation_output(raw: &str, workdir: &Path, home: &Path) -> String {
    raw.replace(&workdir.display().to_string(), "<WORKTREE>")
        .replace(&home.display().to_string(), "<HOME>")
        .replace('\\', "/")
        // The worktree line embeds git's discovery error verbatim. The
        // harness pins GIT_CEILING_DIRECTORIES (see `start_command_env`) so
        // discovery always stops at the workdir with this exact wording;
        // collapse it to a token so the fixture doesn't hard-code git's
        // message. If git ever rewords it, the replace misses and the
        // fixture drifts loudly — the correct failure mode.
        .replace(
            "fatal: not a git repository (or any of the parent directories): .git",
            "<GIT-NOT-A-REPO>",
        )
}

#[cfg(not(target_os = "windows"))]
fn assert_start_activation_fixture(name: &str, raw: &str, workdir: &Path, home: &Path) {
    let actual = normalise_start_activation_output(raw, workdir, home);
    let path = start_activation_fixture_path(name);
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        fs::create_dir_all(path.parent().expect("fixture parent")).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "failed to read fixture {} ({error}); run UPDATE_FIXTURES=1 cargo test -p eddacraft-anvil --test start -- {name}",
            path.display(),
        )
    });
    assert_eq!(actual, expected, "start activation fixture drift: {name}");
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum PtyInteraction {
    Quit,
    EmptyApplyThenQuit,
    SelectFirstApplyThenQuit,
}

#[cfg(unix)]
struct PtyRun {
    status: std::process::ExitStatus,
    transcript: String,
    terminal_mode_before: nix::sys::termios::Termios,
    terminal_mode_after: nix::sys::termios::Termios,
}

#[cfg(unix)]
fn terminal_mode(file: &std::fs::File) -> nix::sys::termios::Termios {
    nix::sys::termios::tcgetattr(file).expect("read PTY terminal mode")
}

#[cfg(unix)]
fn occurrence_count(bytes: &[u8], needle: &[u8]) -> usize {
    bytes
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[cfg(unix)]
fn assert_screen_transitions(result: &PtyRun, expected: usize) {
    let enters = occurrence_count(result.transcript.as_bytes(), b"\x1b[?1049h");
    let leaves = occurrence_count(result.transcript.as_bytes(), b"\x1b[?1049l");
    assert_eq!(enters, expected, "unexpected screen enters");
    assert_eq!(leaves, expected, "unbalanced screen leaves");
}

#[cfg(unix)]
fn run_start_in_pty(
    workdir: &Path,
    home: &Path,
    extra_args: &[&str],
    interaction: PtyInteraction,
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
    let slave_monitor = slave.try_clone().unwrap();
    let terminal_mode_before = terminal_mode(&slave_monitor);
    let stdin = slave.try_clone().unwrap();
    let stdout = slave.try_clone().unwrap();

    let mut command = start_command_env(workdir, home);
    for variable in [
        "CI",
        "ANVIL_ACTIVATION_TUI",
        "ANVIL_NO_TUI",
        "ANVIL_NO_PROMPT",
        "NONINTERACTIVE",
        "GIT_DIR",
        "GIT_INDEX_FILE",
    ] {
        command.env_remove(variable);
    }
    // CIB-224: `start_command_env` forces ANVIL_ALL_MCP_CLIENTS for hermetic
    // install coverage. That collides with `--no-mcp` (and ANVIL_NO_MCP). Drop
    // the all-clients opt-in when the test is intentionally skipping MCP.
    if extra_args.contains(&"--no-mcp") {
        command.env_remove("ANVIL_ALL_MCP_CLIENTS");
        command.env_remove("ANVIL_NO_MCP"); // flag alone owns opt-out
    }
    // ACTTUI-013: no `--tui` and no `ANVIL_ACTIVATION_TUI` — a real terminal
    // enters the activation TUI on the bare `anvil start` default path.
    command
        .arg("start")
        .args(extra_args)
        .stdin(std::process::Stdio::from(stdin))
        .stdout(std::process::Stdio::from(stdout))
        .stderr(std::process::Stdio::from(slave.try_clone().unwrap()));
    let mut child = command.spawn().expect("spawn anvil in PTY");
    drop(slave);
    nix::fcntl::fcntl(
        &master,
        nix::fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::O_NONBLOCK),
    )
    .expect("set PTY master non-blocking");

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut interaction_stage = 0_u8;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let status = loop {
        match master.read(&mut buffer) {
            Ok(0) => {}
            Ok(read) => bytes.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) if error.raw_os_error() == Some(libc::EIO) => {}
            Err(error) => panic!("read PTY output: {error}"),
        }
        let screen_enters = occurrence_count(&bytes, b"\x1b[?1049h");
        if interaction_stage == 0 && screen_enters >= 1 {
            let keys = match interaction {
                PtyInteraction::Quit => b"q".as_slice(),
                PtyInteraction::EmptyApplyThenQuit => b"a".as_slice(),
                PtyInteraction::SelectFirstApplyThenQuit => b" a".as_slice(),
            };
            master
                .write_all(keys)
                .expect("send interaction keys to PTY");
            master.flush().unwrap();
            interaction_stage = 1;
        }
        if interaction_stage == 1
            && !matches!(interaction, PtyInteraction::Quit)
            && bytes
                .windows(b"[Verdict]".len())
                .any(|window| window == b"[Verdict]")
        {
            master.write_all(b"q").expect("quit post-consent PTY");
            master.flush().unwrap();
            interaction_stage = 2;
        }
        if let Some(status) = child.try_wait().expect("poll PTY child") {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().expect("kill hung PTY child");
            child.wait().expect("reap hung PTY child");
            let transcript = String::from_utf8_lossy(&bytes);
            panic!("`anvil start` did not complete PTY interaction:\n{transcript}");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
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
        terminal_mode_before,
        terminal_mode_after: terminal_mode(&slave_monitor),
    }
}

#[cfg(unix)]
#[test]
fn start_pty_no_tui_stays_on_the_plain_path() {
    // ACTTUI-013: `--no-tui` is the permanent escape hatch, and it has to hold
    // in a *real* terminal — the one context where the flip changed what the
    // default does. Before the flip a missing opt-in made this unobservable.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();

    let result = run_start_in_pty(
        dir.path(),
        home.path(),
        &["--no-tui", "--no-daemon", "--no-mcp"],
        PtyInteraction::Quit,
    );

    assert!(
        result.status.success(),
        "PTY start --no-tui failed:\n{}",
        result.transcript
    );
    assert!(
        !result.transcript.contains("\u{1b}[?1049h"),
        "--no-tui entered the alternate screen in a PTY:\n{}",
        result.transcript,
    );
    assert!(
        result.transcript.contains("ACTIVATION"),
        "--no-tui should print the plain activation dossier:\n{}",
        result.transcript,
    );
    assert_eq!(result.terminal_mode_after, result.terminal_mode_before);
}

#[cfg(unix)]
#[test]
fn start_tui_pty_enters_and_restores_the_alternate_screen() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();

    let result = run_start_in_pty(
        dir.path(),
        home.path(),
        &["--no-daemon", "--no-mcp"],
        PtyInteraction::Quit,
    );

    assert!(
        result.status.success(),
        "PTY start failed:\n{}",
        result.transcript
    );
    assert!(
        result.transcript.contains("\u{1b}[?1049h"),
        "TUI never entered the alternate screen:\n{}",
        result.transcript,
    );
    assert!(
        result.transcript.contains("\u{1b}[?1049l"),
        "TUI did not restore the terminal on q:\n{}",
        result.transcript,
    );
    assert!(
        result.transcript.contains("[Preflight]"),
        "activation did not render before work began:\n{}",
        result.transcript,
    );
    assert!(
        result.transcript.contains("[Working]"),
        "typed activation events did not update the live surface:\n{}",
        result.transcript,
    );
    assert_screen_transitions(&result, 1);
    assert_eq!(result.terminal_mode_after, result.terminal_mode_before);
}

#[cfg(unix)]
fn assert_no_tui_project_writes(root: &Path) {
    for relative in [
        ".anvilrc",
        ".anvil.yaml",
        "anvil/project-id",
        ".gitattributes",
        ".anvil/baseline.json",
        ".gitignore",
    ] {
        assert!(
            !root.join(relative).exists(),
            "TUI wrote {relative} without explicit selection"
        );
    }
}

#[cfg(unix)]
#[test]
fn start_tui_cancel_on_fresh_repo_writes_nothing_and_restores_raw_mode() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();

    let result = run_start_in_pty(
        dir.path(),
        home.path(),
        &["--no-daemon", "--no-mcp"],
        PtyInteraction::Quit,
    );

    assert!(
        result.status.success(),
        "PTY start failed:\n{}",
        result.transcript
    );
    assert_no_tui_project_writes(dir.path());
    assert_screen_transitions(&result, 1);
    assert_eq!(result.terminal_mode_after, result.terminal_mode_before);
}

#[cfg(unix)]
#[test]
fn start_tui_empty_apply_reaches_verdict_without_writes_or_false_pass() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();

    let result = run_start_in_pty(
        dir.path(),
        home.path(),
        &["--no-daemon", "--no-mcp"],
        PtyInteraction::EmptyApplyThenQuit,
    );

    assert!(
        result.status.success(),
        "PTY start failed:\n{}",
        result.transcript
    );
    assert_screen_transitions(&result, 1);
    assert!(
        result.transcript.contains("[Verdict]"),
        "post-consent verdict was not rendered:\n{}",
        result.transcript
    );
    assert_no_tui_project_writes(dir.path());
    assert_eq!(result.terminal_mode_after, result.terminal_mode_before);
}

#[cfg(unix)]
#[test]
fn start_tui_selected_apply_writes_only_selection_then_reaches_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();

    let result = run_start_in_pty(
        dir.path(),
        home.path(),
        &["--no-daemon", "--no-mcp"],
        PtyInteraction::SelectFirstApplyThenQuit,
    );

    assert!(
        result.status.success(),
        "PTY start failed:\n{}",
        result.transcript
    );
    assert_screen_transitions(&result, 1);
    assert!(
        result.transcript.contains("[Verdict]"),
        "post-consent verdict was not rendered:\n{}",
        result.transcript
    );
    assert!(dir.path().join(".anvil.yaml").exists());
    assert!(!dir.path().join("anvil/project-id").exists());
    assert!(!dir.path().join(".gitattributes").exists());
    assert!(!dir.path().join(".anvil/baseline.json").exists());
    assert_eq!(result.terminal_mode_after, result.terminal_mode_before);
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_on_fresh_repo_runs_init_and_lands_ready_restart_required() {
    // The composed flow's headline outcome on an empty HOME: init
    // writes `.anvil.yaml`, the MCP install step writes Cursor + Claude
    // Code entries into HOME, and the diagnostic ends at
    // `ready_restart_required` (the user must restart their editor
    // for the entries to attach).
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert!(
        !dir.path().join(".anvil.yaml").exists(),
        "pre-condition: fresh temp repo has no config"
    );

    let out = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(
        out.status.success(),
        "anvil start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Init ran — the only stable proof is the canonical config on disk.
    assert!(
        dir.path().join(".anvil.yaml").exists(),
        ".anvil.yaml must exist after `anvil start` on a fresh repo"
    );

    // Install ran — Cursor + Claude Code entries written into HOME.
    assert!(
        home.path().join(".cursor/mcp.json").exists(),
        "Cursor MCP config must exist in HOME after `anvil start`"
    );
    assert!(
        home.path().join(".claude.json").exists(),
        "Claude Code MCP config must exist in HOME after `anvil start`"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: ready_restart_required"),
        "expected `state: ready_restart_required` after MCP install, got:\n{stdout}"
    );
    // Truthfulness guardrail.
    assert!(
        !stdout.contains("state: protecting"),
        "fresh repo MUST NOT claim protection, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_without_detected_editor_does_not_write_mcp_config() {
    // ACTMO-012 (Matt beta smoke): on a host where no editor is detected
    // — no `cursor` / `claude` binary on PATH, no pre-existing editor
    // state under HOME — and without `--all-mcp-clients`, `anvil start`
    // must NOT create `~/.cursor/mcp.json` or `~/.claude.json`. The spine
    // (`.anvilrc`, daemon, hooks) still activates; MCP is optional.
    //
    // Determinism: PATH is emptied and the AI-tool env hints are removed
    // so detection cannot fire from the test host's real environment.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("start")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("XDG_RUNTIME_DIR", home.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        // Force a deterministic "no editor detected" environment.
        .env("PATH", "")
        .env_remove("ANVIL_ALL_MCP_CLIENTS")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("CLAUDE_CODE_HOME")
        .env_remove("CURSOR_HOME")
        .env_remove("OPENAI_API_KEY");
    let out = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        out.status.success(),
        "anvil start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The spine still activated.
    assert!(
        dir.path().join(".anvil.yaml").exists(),
        ".anvil.yaml must exist after `anvil start` even with no editor detected"
    );
    // But no MCP config was written for an editor the user does not have.
    assert!(
        !home.path().join(".cursor/mcp.json").exists(),
        "must NOT write a Cursor MCP config when Cursor is not detected"
    );
    assert!(
        !home.path().join(".claude.json").exists(),
        "must NOT write a Claude Code MCP config when Claude Code is not detected"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_in_non_interactive_context_falls_back_without_spawning() {
    // DLIFE-003 (ADR-082 §4, owner-confirmed headless posture): a mutating
    // `anvil start` with captured (non-TTY) stdout — the CI / scripted shape
    // — must NOT auto-start a daemon. It reports the deterministic
    // non-interactive fallback on the `daemon:` line and never makes a
    // protection claim (module honesty contract: lifecycle action only).
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(
        out.status.success(),
        "anvil start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("daemon: not auto-started (non-interactive"),
        "expected the non-interactive fallback line, got:\n{stdout}"
    );
    assert!(
        stdout.contains("scoped fallback"),
        "the fallback line must name the preserved scoped fallback, got:\n{stdout}"
    );
    // The fallback line must never graduate the protection claim: the
    // diagnostic still owns `state:`.
    assert!(
        !stdout.contains("state: protecting"),
        "the daemon line must not push the protection state to protecting, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_no_daemon_flag_reports_opt_out_end_to_end() {
    // DLIFE-003: the explicit `--no-daemon` opt-out is plumbed through clap
    // and rendered distinctly from the non-interactive fallback, so a user
    // who opted out sees their flag acknowledged rather than a generic
    // context message.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &["--no-daemon"]);
    assert!(
        out.status.success(),
        "anvil start --no-daemon failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("daemon: not started (--no-daemon)"),
        "expected the --no-daemon opt-out line, got:\n{stdout}"
    );
}

#[test]
fn start_no_daemon_env_reports_opt_out_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = start_command_env(dir.path(), home.path());
    let out = cmd
        .arg("--no-tui")
        .arg("start")
        .env("ANVIL_NO_DAEMON", "1")
        .output()
        .expect("failed to invoke anvil start with ANVIL_NO_DAEMON");

    assert!(
        out.status.success(),
        "ANVIL_NO_DAEMON start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("daemon: not started (--no-daemon)"),
        "env opt-out must suppress daemon start: {stdout}"
    );
}

#[test]
fn start_empty_no_daemon_env_uses_non_interactive_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = start_command_env(dir.path(), home.path());
    let out = cmd
        .arg("--no-tui")
        .arg("start")
        .env("ANVIL_NO_DAEMON", "")
        .output()
        .expect("failed to invoke anvil start with empty ANVIL_NO_DAEMON");

    assert!(
        out.status.success(),
        "empty ANVIL_NO_DAEMON start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("daemon: not auto-started (non-interactive"),
        "empty env value must behave as unset: {stdout}"
    );
    assert!(
        !stdout.contains("daemon: not started (--no-daemon)"),
        "empty env value must not report an explicit opt-out: {stdout}"
    );
}

#[test]
fn start_no_mcp_env_reports_install_skipped_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = start_command_env(dir.path(), home.path());
    let out = cmd
        .arg("--no-tui")
        .arg("start")
        .arg("--no-daemon")
        .env_remove("ANVIL_ALL_MCP_CLIENTS")
        .env("ANVIL_NO_MCP", "1")
        .output()
        .expect("failed to invoke anvil start with ANVIL_NO_MCP");

    assert!(
        out.status.success(),
        "ANVIL_NO_MCP start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("MCP config installation disabled"),
        "env opt-out must report the skipped install: {stdout}"
    );
}

#[test]
fn start_empty_no_mcp_env_does_not_skip_install() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = start_command_env(dir.path(), home.path());
    let out = cmd
        .arg("--no-tui")
        .arg("start")
        .arg("--no-daemon")
        .env("ANVIL_NO_MCP", "")
        .output()
        .expect("failed to invoke anvil start with empty ANVIL_NO_MCP");

    assert!(
        out.status.success(),
        "empty ANVIL_NO_MCP start failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("MCP config installation disabled"),
        "empty env value must behave as unset: {stdout}"
    );
}

#[test]
fn start_no_mcp_env_with_all_clients_env_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = start_command_env(dir.path(), home.path());
    let out = cmd
        .arg("--no-tui")
        .arg("start")
        .arg("--no-daemon")
        .env("ANVIL_NO_MCP", "1")
        .output()
        .expect("failed to invoke conflicting MCP env forms");

    assert!(!out.status.success(), "conflicting env forms must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "env forms must report their conflict: {stderr}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_is_byte_identical_without_a_daemon_line() {
    // DLIFE-003: read-only probes never start a daemon and never emit a
    // `daemon:` lifecycle line — the auto-start is a mutating-path
    // behaviour only. Pins the read-only contract end-to-end.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("daemon:"),
        "--verify must not emit a daemon lifecycle line, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_idempotent_rerun_skips_init_and_install() {
    // Idempotency contract: a second `anvil start` against the same
    // repo + same HOME must:
    //   1. Not rewrite `.anvil.yaml` (mtime unchanged).
    //   2. Not rewrite the MCP entries (mtime unchanged on each
    //      target file).
    //   3. Still emit the diagnostic, ending at the same final state.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let first = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(first.status.success());

    let cursor_path = home.path().join(".cursor/mcp.json");
    let claude_path = home.path().join(".claude.json");
    let config = dir.path().join(".anvil.yaml");

    let mtime_config_before = std::fs::metadata(&config).unwrap().modified().unwrap();
    let mtime_cursor_before = std::fs::metadata(&cursor_path).unwrap().modified().unwrap();
    let mtime_claude_before = std::fs::metadata(&claude_path).unwrap().modified().unwrap();

    // Sleep past one-second mtime granularity so any rewrite would be
    // detectable on filesystems with HFS+-style coarse mtimes.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(
        second.status.success(),
        "second start failed: stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );

    let mtime_config_after = std::fs::metadata(&config).unwrap().modified().unwrap();
    let mtime_cursor_after = std::fs::metadata(&cursor_path).unwrap().modified().unwrap();
    let mtime_claude_after = std::fs::metadata(&claude_path).unwrap().modified().unwrap();
    assert_eq!(
        mtime_config_before, mtime_config_after,
        "second start must not rewrite .anvil.yaml (idempotent rerun)"
    );
    assert_eq!(
        mtime_cursor_before, mtime_cursor_after,
        "second start must not rewrite Cursor MCP config when already up to date"
    );
    assert_eq!(
        mtime_claude_before, mtime_claude_after,
        "second start must not rewrite Claude Code MCP config when already up to date"
    );

    // CIB-183: the first run renders the rich recipe; the repeat run
    // collapses to state + posture + one next step and must NOT reprint
    // the first-run recipe.
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    assert!(
        first_stdout.contains("recipe (try this now"),
        "first start must render the rich first-run recipe, got:\n{first_stdout}"
    );
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("state: ready_restart_required"),
        "second start must still emit the protection state, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("recipe (try this now"),
        "repeat start must not reprint the first-run recipe, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("active layers"),
        "repeat start must not reprint the layer breakdown, got:\n{stdout}"
    );
    assert!(
        stdout.contains("daemon:") && stdout.contains("save-time driver:"),
        "repeat start must keep the daemon/driver posture, got:\n{stdout}"
    );
    assert_eq!(
        stdout.matches("next:").count() + stdout.matches("Next:").count(),
        1,
        "repeat start carries exactly one next step, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_on_fresh_repo_reports_needs_action() {
    // `activation::verify` is read-only. With an empty HOME override,
    // no MCP entry exists anywhere, so the diagnostic maps
    // ConfigStatus::Absent → ProtectionState::NeedsAction.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(
        out.status.success(),
        "anvil start --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    // --verify is read-only: no project config must be written, and
    // neither HOME's `.cursor/mcp.json` nor `.claude.json`.
    for name in [
        ".anvil.yaml",
        ".anvil.yml",
        ".anvil.json",
        ".anvil.toml",
        ".anvilrc",
    ] {
        assert!(
            !dir.path().join(name).exists(),
            "--verify must not write a project config ({name})"
        );
    }
    assert!(
        !home.path().join(".cursor/mcp.json").exists(),
        "--verify must not install Cursor MCP entry"
    );
    assert!(
        !home.path().join(".claude.json").exists(),
        "--verify must not install Claude Code MCP entry"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: needs_action"),
        "fresh-repo --verify should report needs_action (config absent → NeedsAction), got:\n{stdout}"
    );
    assert!(
        stdout.contains("config: absent"),
        "config status should be reported as absent, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_matches_ready_restart_required_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let first = run_start_with_home(dir.path(), home.path(), &["--no-daemon"]);
    assert!(
        first.status.success(),
        "setup start failed: stderr={}",
        String::from_utf8_lossy(&first.stderr)
    );

    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(
        out.status.success(),
        "anvil start --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("state: ready_restart_required"));
    assert_start_activation_fixture(
        "verify-ready-restart-required.stdout",
        &stdout,
        dir.path(),
        home.path(),
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_json_matches_ready_restart_required_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let first = run_start_with_home(dir.path(), home.path(), &["--no-daemon"]);
    assert!(
        first.status.success(),
        "setup start failed: stderr={}",
        String::from_utf8_lossy(&first.stderr)
    );

    let out = run_start_with_home(dir.path(), home.path(), &["--json"]);
    assert!(
        out.status.success(),
        "anvil start --json failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");
    assert_eq!(json["state"].as_str(), Some("ready_restart_required"));
    assert!(
        String::from_utf8_lossy(&out.stderr).trim().is_empty(),
        "--json must not emit a human stderr block"
    );
    assert_start_activation_fixture(
        "json-ready-restart-required.stdout",
        &stdout,
        dir.path(),
        home.path(),
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_no_tui_and_env_no_tui_match_compact_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let first = run_start_with_home(dir.path(), home.path(), &["--no-daemon"]);
    assert!(
        first.status.success(),
        "setup start failed: stderr={}",
        String::from_utf8_lossy(&first.stderr)
    );
    // CIB-183: the first run keeps the rich recipe — pin it so the
    // repeat-run collapse can never silently eat the first-run surface.
    assert_start_activation_fixture(
        "compact-first-run.stdout",
        &String::from_utf8_lossy(&first.stdout),
        dir.path(),
        home.path(),
    );

    // CIB-183: the reruns below are repeat successes (config pre-existing,
    // MCP entries already up to date), so the compact fixture pins the
    // COLLAPSED repeat output: state + posture + one next step.
    let flag = run_start_with_home(dir.path(), home.path(), &["--no-daemon"]);
    assert!(
        flag.status.success(),
        "--no-tui rerun failed: stderr={}",
        String::from_utf8_lossy(&flag.stderr)
    );

    let mut cmd = start_command_env(dir.path(), home.path());
    cmd.arg("start").arg("--no-daemon").env("ANVIL_NO_TUI", "1");
    let env = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        env.status.success(),
        "ANVIL_NO_TUI rerun failed: stderr={}",
        String::from_utf8_lossy(&env.stderr)
    );

    let flag_stdout = String::from_utf8_lossy(&flag.stdout);
    let env_stdout = String::from_utf8_lossy(&env.stdout);
    assert_eq!(
        normalise_start_activation_output(&flag_stdout, dir.path(), home.path()),
        normalise_start_activation_output(&env_stdout, dir.path(), home.path()),
        "ANVIL_NO_TUI=1 must match --no-tui compact output",
    );
    assert!(!flag_stdout.contains("\u{1b}[?1049h"));
    assert_start_activation_fixture(
        "compact-ready-restart-required.stdout",
        &flag_stdout,
        dir.path(),
        home.path(),
    );
}

// Gated like every other test that uses `normalise_start_activation_output`
// and `assert_start_activation_fixture`: those helpers are
// `cfg(not(target_os = "windows"))`, so without this the file does not compile
// for Windows at all.
#[cfg(not(target_os = "windows"))]
#[test]
fn start_tui_flag_is_an_accepted_no_op_alias() {
    // ACTTUI-013 / ADR-103: `--tui` and `ANVIL_ACTIVATION_TUI=1` are retired to
    // accepted no-op aliases — they must still parse (no deprecation error) and
    // must not change a single byte of output. Hidden from `--help` because
    // they no longer do anything.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let baseline = run_start_with_home(dir.path(), home.path(), &["--no-daemon"]);
    assert!(
        baseline.status.success(),
        "setup start failed: stderr={}",
        String::from_utf8_lossy(&baseline.stderr)
    );

    let flag = run_start_with_home(dir.path(), home.path(), &["--no-daemon", "--tui"]);
    assert!(
        flag.status.success(),
        "--tui must still parse: stderr={}",
        String::from_utf8_lossy(&flag.stderr)
    );

    let mut cmd = start_command_env(dir.path(), home.path());
    cmd.arg("--no-tui")
        .arg("start")
        .arg("--no-daemon")
        .env("ANVIL_ACTIVATION_TUI", "1");
    let env = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        env.status.success(),
        "ANVIL_ACTIVATION_TUI rerun failed: stderr={}",
        String::from_utf8_lossy(&env.stderr)
    );

    let flag_stdout = normalise_start_activation_output(
        &String::from_utf8_lossy(&flag.stdout),
        dir.path(),
        home.path(),
    );
    let env_stdout = normalise_start_activation_output(
        &String::from_utf8_lossy(&env.stdout),
        dir.path(),
        home.path(),
    );
    assert_eq!(
        flag_stdout, env_stdout,
        "the retired aliases must produce identical output",
    );

    let help = Command::new(ANVIL_BIN)
        .arg("start")
        .arg("--help")
        .output()
        .expect("failed to invoke anvil binary");
    assert!(
        !String::from_utf8_lossy(&help.stdout).contains("--tui"),
        "a retired no-op flag must not be advertised in start --help"
    );
}

#[test]
fn start_json_emits_state_literal_in_status_verify_shape() {
    // LAUNCH-012 acceptance: `anvil start --json` is read-only — the
    // flag implies `--verify` (see `start.rs` `read_only = verify ||
    // json`). On a fresh repo with an empty HOME override no MCP entry
    // exists and no project config is written, so the diagnostic maps
    // `ConfigStatus::Absent → ProtectionState::NeedsAction` — the same
    // outcome as `start --verify` (covered by
    // `start_verify_on_fresh_repo_reports_needs_action`).
    //
    // Council-locked truthfulness (CLAWP-022): a fresh repo MUST NEVER
    // claim `protecting`, `watching`, or `ready_restart_required` on
    // this read-only path. Accepting any of those would let a
    // regression silently graduate the diagnostic to a stronger claim
    // than read-only evidence supports.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // --json implies --verify: no writes should land on disk.
    assert!(
        !dir.path().join(".anvil.yaml").exists() && !dir.path().join(".anvilrc").exists(),
        "--json must not write a project config (read-only)"
    );
    assert!(
        !home.path().join(".cursor/mcp.json").exists(),
        "--json must not install Cursor MCP entry (read-only)"
    );
    assert!(
        !home.path().join(".claude.json").exists(),
        "--json must not install Claude Code MCP entry (read-only)"
    );

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");
    let state = json["state"]
        .as_str()
        .expect("state must be present as a string");
    // Truthfulness-specific rejection runs first so a regression that
    // graduates the diagnostic to a stronger claim fails with the
    // CLAWP-022-locked message, not the generic equality mismatch.
    for forbidden in ["protecting", "watching", "ready_restart_required"] {
        assert_ne!(
            state, forbidden,
            "fresh repo MUST NOT claim `{forbidden}` on the read-only --json path"
        );
    }
    assert_eq!(
        state, "needs_action",
        "fresh repo + empty HOME under read-only --json must land on needs_action, got {state}"
    );
    assert!(json["headline"].is_string(), "headline must be a string");
    assert!(json["config"].is_string(), "config must be a string");
}

#[test]
fn welcome_still_runs_after_start_promotion() {
    // #1280 review: don't assert on welcome's description copy — that's
    // owned by other UX work and likely to change. Just prove the
    // command still resolves and shows its clap-generated usage block.
    let out = Command::new(ANVIL_BIN)
        .arg("welcome")
        .arg("--help")
        .output()
        .expect("failed to invoke anvil binary");
    assert!(
        out.status.success(),
        "anvil welcome --help failed after LAUNCH-006 promotion: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Stable: clap always emits a `Usage:` block with the subcommand
    // name. If the alias-removal regressed, clap would error out before
    // reaching this point (non-zero exit, caught above).
    assert!(
        stdout.contains("Usage:") && stdout.contains("welcome"),
        "welcome --help should emit clap's Usage block, got:\n{stdout}"
    );
}

#[test]
fn start_help_documents_daemon_lifecycle() {
    // DLIFE-005: `anvil start --help` must keep documenting the daemon
    // lifecycle opt-out so the CLI long help stays aligned with the
    // public docs (auto-start + `--no-daemon` / `ANVIL_NO_DAEMON`).
    let out = Command::new(ANVIL_BIN)
        .arg("start")
        .arg("--help")
        .output()
        .expect("failed to invoke anvil binary");
    assert!(
        out.status.success(),
        "anvil start --help failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--no-daemon"),
        "start --help should list the --no-daemon opt-out, got:\n{stdout}"
    );
    assert!(
        stdout.contains("ANVIL_NO_DAEMON"),
        "start --help should name the ANVIL_NO_DAEMON env opt-out, got:\n{stdout}"
    );
}

#[test]
fn start_on_invalid_config_emits_error_state_not_panic() {
    // Adversarial guardrail: a malformed config must not panic the
    // start orchestrator. The diagnostic surfaces it as `state: error`.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "{this is not valid in any format::",
    )
    .unwrap();

    let out = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(
        out.status.success(),
        "anvil start on invalid config failed (should report error state, not exit non-zero): stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: error"),
        "expected `state: error` on malformed config, got:\n{stdout}"
    );
}

// ---- LAUNCH-011: honest watch fallback --------------------------

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_on_initialised_repo_surfaces_partial_protection_note() {
    // LAUNCH-011 acceptance: on an initialised repo (config valid)
    // where MCP cannot pre-write attach, the human render must say
    // so explicitly — never let the user infer protection from a
    // weaker tier or from config-only state.
    //
    // The repo is pre-initialised with a valid config so the
    // diagnostic does not bypass the offer logic via the
    // `ConfigStatus::Absent` suppression (council remediation: the
    // primary action on Absent is `anvil init`, not watch fallback).
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    // Drop a TS file so the language profile reports a supported
    // language and the diagnostic does not collapse to `Unsupported`
    // (which would also suppress the offer).
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(
        out.status.success(),
        "anvil start --verify failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The diagnostic must include the literal honesty note.
    assert!(
        stdout.contains("MCP pre-write validation is not attached"),
        "initialised-repo --verify must include the partial-protection note, got:\n{stdout}"
    );

    // It must surface the offered watch tier so the user sees the
    // fallback option in the structured output, not just in the prose
    // hint.
    assert!(
        stdout.contains("watch: offered"),
        "initialised-repo --verify must show watch tier as `offered`, got:\n{stdout}"
    );

    // Truthfulness guardrails — the language LAUNCH-011 explicitly
    // forbids must NOT appear anywhere in the rendered output.
    let lower = stdout.to_lowercase();
    assert!(
        !lower.contains("fully protected"),
        "rendered output must never claim `fully protected`, got:\n{stdout}"
    );
    assert!(
        !lower.contains("mcp activated"),
        "rendered output must never claim `MCP activated`, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("state: protecting"),
        "initialised-repo --verify MUST NOT claim `state: protecting`, got:\n{stdout}"
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_emits_no_raw_json_tracing_lines_at_default_filter() {
    // CIB-162: without an attesting daemon, `anvil start` reaches
    // `ready_restart_required` and probes the daemon; the probe fails
    // (no daemon on the per-test `XDG_RUNTIME_DIR`) and emits a
    // "daemon attestation skipped" tracing event. That event MUST NOT
    // reach the human surface as a raw JSONL line at the CLI default
    // (`warn`) filter — the daemon-skip signal is rendered as human
    // copy by the render layer (`daemon:` / `meaning:` lines), so the
    // machine JSONL is demoted below the default filter. Regression for
    // the user-journey finding where a `{"timestamp":…,"level":"WARN"…}`
    // line printed mid-flow and read as a crash.
    //
    // Covers both the full activation flow (`anvil start`) and the
    // read-only diagnostic (`anvil start --verify`) on an already
    // initialised repo, since both drive the daemon-attestation probe.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let assert_no_json_lines = |out: &std::process::Output, label: &str| {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{label} failed: stderr={stderr}");
        for (stream, text) in [("stdout", &stdout), ("stderr", &stderr)] {
            for line in text.lines() {
                assert!(
                    !line.trim_start().starts_with("{\"timestamp\""),
                    "{label}: raw JSON tracing line leaked onto {stream} at the default \
                     log filter:\n{line}\n\nfull {stream}:\n{text}"
                );
            }
        }
    };

    // Full activation flow: init + install + reach ready_restart_required
    // (the state that probes the daemon).
    let out = run_start_with_home(dir.path(), home.path(), &[]);
    assert_no_json_lines(&out, "anvil start");

    // Read-only re-run on the now-initialised repo: the probe fires
    // again, but no JSONL may reach the human surface.
    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert_no_json_lines(&out, "anvil start --verify");
}

#[cfg(not(target_os = "windows"))]
#[test]
fn start_verify_on_fresh_repo_with_absent_config_does_not_advertise_watch() {
    // Council remediation: when config is absent, the user's primary
    // action is `anvil init`, not watch fallback. The note and the
    // `Offered` tier must both be suppressed so the init nudge is
    // not diluted.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("config: absent"),
        "expected config: absent on fresh repo, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("watch: offered"),
        "fresh repo with absent config MUST NOT advertise watch, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("MCP pre-write validation is not attached"),
        "fresh repo with absent config must defer to init copy, got:\n{stdout}"
    );
    assert!(
        stdout.contains("anvil init"),
        "fresh repo with absent config must surface the init nudge, got:\n{stdout}"
    );
}

#[test]
fn start_after_install_communicates_restart_or_daemon_recovery_action() {
    // Council round-2 remediation: at `ready_restart_required`, the
    // headline already conveys the partial state ("restart your
    // editor or agent so the MCP server attaches"). The
    // partial-protection NOTE is suppressed here because:
    //   1. The headline already says MCP isn't yet attached.
    //   2. There is no `watch: offered` line — appending the watch-
    //      fallback note alone would orphan watch copy and nudge the
    //      user toward watch when they should restart.
    // The honesty contract is preserved by the headline / hint. When
    // daemon evidence is already available and says the daemon is not
    // reachable, DLIFE-006 deliberately replaces the generic restart
    // wording with the terminating `anvil intercept start --foreground`
    // recovery path.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &[]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("state: ready_restart_required"),
        "post-install state should be ready_restart_required, got:\n{stdout}"
    );
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("restart your editor")
            || lower.contains("restart required")
            || lower.contains("anvil intercept start --foreground"),
        "ready_restart_required render must surface the next concrete \
         recovery action in headline / hint, got:\n{stdout}"
    );
    assert!(
        lower.contains("attach")
            || lower.contains("mcp server")
            || (lower.contains("mcp config") && lower.contains("daemon not reachable")),
        "ready_restart_required render must explain why pre-write \
         protection is not yet live, got:\n{stdout}"
    );
    // The note belongs with `Watching` / `NeedsAction (config valid)` /
    // the `Offered` watch tier. At ready_restart_required, suppress.
    assert!(
        !stdout.contains("MCP pre-write validation is not attached"),
        "ready_restart_required render must NOT include the orphaned \
         watch-fallback note (the headline already conveys the \
         partial state), got:\n{stdout}"
    );
    // Truthfulness guardrails.
    let lower = stdout.to_lowercase();
    assert!(
        !lower.contains("fully protected"),
        "ready_restart_required must never claim fully protected, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("state: protecting"),
        "ready_restart_required must never claim state: protecting, got:\n{stdout}"
    );
}

#[test]
fn start_watch_with_verify_is_rejected() {
    // LAUNCH-011: `--watch` spawns a process; `--verify` is read-only.
    // Combining them would silently downgrade one or synthesise watch
    // state without actually starting it. Reject the combination so
    // the user gets a clear error instead.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--watch", "--verify"]);
    assert!(
        !out.status.success(),
        "`--watch --verify` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--watch` and `--verify` are mutually exclusive"),
        "error message must explain the conflict, got:\n{stderr}"
    );
}

#[test]
fn start_verify_with_format_is_rejected() {
    // CIB-051: `--format` pre-writes `.anvil.<ext>` — a durable project
    // mutation — while `--verify` is read-only. The combination used to
    // silently drop `--format`; reject it explicitly like the
    // `--watch` / `--new-identity` siblings so the user gets a clear
    // error instead.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let out = run_start_with_home(dir.path(), home.path(), &["--verify", "--format", "yaml"]);
    assert!(
        !out.status.success(),
        "`--verify --format` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--format` is incompatible with `--verify` / `--json` (read-only)"),
        "error message must explain the conflict, got:\n{stderr}"
    );
    // Clean bail before side effects — no config file may be written.
    assert!(
        !dir.path().join(".anvil.yaml").exists(),
        "`--verify --format` must not write .anvil.yaml"
    );
}

#[test]
fn start_json_with_format_is_rejected() {
    // CIB-051: `--json` implies read-only exactly like `--verify`, so
    // the `--format` pre-write is rejected for the same reason. Pin the
    // second half of the read-only condition the way
    // `start_watch_with_json_is_rejected` pins its sibling.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("--json")
        .arg("start")
        .arg("--format")
        .arg("yaml")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        !out.status.success(),
        "`--json` + `--format` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--format` is incompatible with `--verify` / `--json` (read-only)"),
        "error message must explain the conflict, got:\n{stderr}"
    );
    // Clean bail before side effects — no config file may be written.
    assert!(
        !dir.path().join(".anvil.yaml").exists(),
        "`--json` + `--format` must not write .anvil.yaml"
    );
}

#[test]
fn start_watch_with_json_is_rejected() {
    // LAUNCH-011: the watcher streams event lines; `--json` expects a
    // single parseable document. Reject the combination explicitly.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("--json")
        .arg("start")
        .arg("--watch")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    let out = cmd.output().expect("failed to invoke anvil binary");
    assert!(
        !out.status.success(),
        "`--watch --json` must fail, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`--watch` and `--json` are mutually exclusive"),
        "error message must explain the conflict, got:\n{stderr}"
    );
}

#[test]
fn start_watch_renders_partial_protection_and_starts_watcher() {
    // LAUNCH-011 acceptance: with no supported MCP client live,
    // `anvil start --watch` runs the orchestrator, prints the
    // diagnostic, and lands the user in the kernel watcher. The
    // pre-handoff render must:
    //
    //   1. Not claim `state: protecting` (no LiveValidation evidence).
    //   2. Include the explicit "MCP pre-write validation is not
    //      attached" note.
    //   3. Print the watch hand-off marker so the user sees the
    //      transition into the fallback.
    //
    // Implementation note: the watcher is long-running. We read
    // stdout in a separate thread until we see the hand-off marker
    // or the deadline expires, then SIGKILL the child via
    // `Child::kill` (cross-platform). The test process is the parent
    // and is fine; we deliberately do not assert on graceful shutdown
    // — that path is covered by `commands::watch` unit tests.
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    // Seed the workspace so the diagnostic does not depend on
    // orchestrator init behaviour — config is `Valid` from the start
    // and a TS file forces a supported-language profile, locking the
    // path that the assertions below exercise.
    fs::write(
        dir.path().join(".anvil.yaml"),
        "profile: default\nchecks: []\n",
    )
    .unwrap();
    fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut child = Command::new(ANVIL_BIN)
        .arg("--no-tui")
        .arg("start")
        .arg("--watch")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("XDG_CONFIG_HOME")
        // DLIFE-003: isolate the daemon socket to the per-test tempdir. The
        // captured (non-TTY) stdout already makes `start` fall back without
        // spawning a daemon; this also keeps the read-only ensure probe from
        // touching a developer box's real daemon.
        .env("XDG_RUNTIME_DIR", home.path())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn anvil start --watch");

    let mut stdout_handle = child.stdout.take().expect("piped stdout");

    // Drain stdout in a worker thread so the parent can enforce a
    // wall-clock deadline. A blocking read on the main thread would
    // hang if the child wrote nothing for some reason.
    let buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let buf_clone = Arc::clone(&buf);
    let reader = std::thread::spawn(move || {
        let mut chunk = [0u8; 1024];
        loop {
            match stdout_handle.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if let Ok(mut guard) = buf_clone.lock() {
                        guard.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    }
                }
            }
        }
    });

    // Poll the buffer for the hand-off marker.
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(guard) = buf.lock()
            && guard.contains("watch: starting save-time fallback")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Stop the child; the reader thread will exit on EOF.
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();

    let captured = buf.lock().map(|g| g.clone()).unwrap_or_default();

    assert!(
        captured.contains("MCP pre-write validation is not attached"),
        "pre-handoff render must include the partial-protection note, got:\n{captured}"
    );
    assert!(
        captured.contains("watch: starting save-time fallback"),
        "must print the watch hand-off marker before entering the watcher, \
         got:\n{captured}"
    );

    // LAUNCH-011 spec acceptance: the rendered state must literally
    // be `watching`, not `protecting`. The pre-handoff diagnostic
    // synthesises `WatchTier::Running` so the printed `state:` line
    // matches the protection layer about to take over.
    //
    // Two acceptable renderings (depending on the test runner's HOME
    // contents):
    //   - empty HOME → MCP at `ConfigAbsent` → `state: watching`
    //   - HOME with stale anvil entry → MCP at `RestartRequired` →
    //     `state: ready_restart_required` (watch + restart-pending
    //     prefers the stronger label per the diagnostic mapping)
    // Either is honest; the forbidden literal is `state: protecting`.
    assert!(
        captured.contains("state: watching") || captured.contains("state: ready_restart_required"),
        "pre-handoff state must be `watching` or `ready_restart_required` \
         (the only honest options when MCP is below LiveValidation), got:\n{captured}"
    );
    assert!(
        !captured.contains("state: protecting"),
        "fallback path MUST NOT claim `state: protecting`, got:\n{captured}"
    );
    let lower = captured.to_lowercase();
    assert!(
        !lower.contains("fully protected"),
        "fallback path MUST NEVER claim `fully protected`, got:\n{captured}"
    );
}

/// CIB-049: `start --verify` is a read-only local probe and must skip
/// the auth wall through `skips_auth_for_local_probe` itself — NOT via
/// the `ANVIL_DEV` escape hatch the other tests in this file lean on.
/// Unauthenticated human mode runs the probe instead of printing the
/// auth-required message.
#[test]
fn start_verify_runs_probe_unauthenticated_without_dev() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("start")
        .arg("--verify")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        // Point XDG at an empty dir so no credentials resolve.
        .env("XDG_CONFIG_HOME", home.path().join("xdg"))
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1");
    let output = cmd.output().expect("failed to invoke anvil binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Authentication required"),
        "start --verify must skip the auth wall via the local-probe \
         predicate, not ANVIL_DEV: stderr=\n{stderr}",
    );
    assert!(
        stdout.contains("ACTIVATION"),
        "expected the human activation diagnostic on stdout: \
         stdout=\n{stdout}\nstderr=\n{stderr}",
    );
}

/// CIB-169: full (non-`--verify`) `anvil start` on an unauthenticated repo
/// carries the auth signal in the exit code (exit 3) so `anvil start &&
/// deploy` stops at an unactivated repo instead of advancing past the auth
/// wall. The human message stays actionable, and a `&& echo reached` chain
/// must NOT print `reached`. Supersedes issue #1822's exit-0 mapping on
/// action commands.
#[cfg(not(target_os = "windows"))]
#[test]
fn start_unauthenticated_exits_three_and_breaks_and_chain() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Drive the invocation through a shell so the `&&` chain is exercised
    // exactly as an operator's `anvil start && deploy` would be — the
    // whole point of the exit-3 contract is that the chain stops here.
    let script = format!("{ANVIL_BIN:?} --no-tui start && echo reached");
    let output = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        // Empty XDG + isolated ANVIL_HOME so no credentials resolve on any
        // platform: the repo is genuinely unauthenticated.
        .env("XDG_CONFIG_HOME", home.path().join("xdg"))
        .env("ANVIL_HOME", home.path().join("anvil-home"))
        // No dev bypass — the auth wall must actually fire (not the
        // ANVIL_DEV escape hatch the mechanics tests above lean on).
        .env_remove("ANVIL_DEV")
        .env_remove("ANVIL_LICENSE")
        .env_remove("ANVIL_LOG")
        .env_remove("RUST_LOG")
        .env("ANVIL_SKIP_WELCOME", "1")
        .env("ANVIL_NO_PROMPT", "1")
        .output()
        .expect("failed to invoke anvil binary via shell");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The `&&` chain stops: the shell's exit status is `anvil`'s exit 3,
    // and the second command never ran.
    assert_eq!(
        output.status.code(),
        Some(3),
        "unauthenticated `anvil start` must exit 3 (CIB-169); \
         stdout=\n{stdout}\nstderr=\n{stderr}",
    );
    assert!(
        !stdout.contains("reached"),
        "the `&& echo reached` chain must NOT advance past an unactivated \
         repo: stdout=\n{stdout}",
    );
    // The message stays actionable — humans still learn what to do next.
    assert!(
        stderr.contains("anvil auth login"),
        "the auth-required message must stay actionable on stderr: \
         stderr=\n{stderr}",
    );
}
