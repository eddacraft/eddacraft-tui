//! MCPLH-002 / JREL-001: `anvil mcp serve --stdio` may re-exec before its
//! first stdin read. An established session preserves accepted and pipelined
//! requests on its current image and reports targeted reconnect guidance.
//! Unix proves the startup replacement image via
//! `/proc/<pid>/exe` when the platform exposes it.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn mcp_reexec_unix_lands_on_preferred_after_forced_skew() {
    let (_dir, preferred) = copy_anvil_as_preferred();
    let mut child = spawn_serve(&[
        ("ANVIL_MCP_PREFERRED", preferred.to_str().expect("utf8")),
        ("ANVIL_MCP_NO_REEXEC", ""),
        ("ANVIL_MCP_REEXECED", ""),
    ]);
    let stdout = child.stdout.take().expect("stdout");
    let stdout_rx = spawn_stdout_reader(stdout);

    send_initialize(&mut child, &stdout_rx);

    #[cfg(target_os = "linux")]
    assert_running_image(&child, &preferred);

    drop(child.stdin.take());
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "serve must exit cleanly after EOF: {status:?}"
    );
}

#[test]
fn mcp_reexec_kill_switch_stays_on_current_image() {
    let (_dir, preferred) = copy_anvil_as_preferred();
    let mut child = spawn_serve(&[
        ("ANVIL_MCP_PREFERRED", preferred.to_str().expect("utf8")),
        ("ANVIL_MCP_NO_REEXEC", "1"),
        ("ANVIL_MCP_REEXECED", ""),
    ]);
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let stdout_rx = spawn_stdout_reader(stdout);
    let stderr_rx = spawn_stdout_reader(stderr);

    send_initialize(&mut child, &stdout_rx);

    #[cfg(target_os = "linux")]
    assert_running_image(&child, Path::new(ANVIL_BIN));

    // Recovery hint is written from a detached stderr thread; await it while
    // the child is still alive so process exit cannot race the write away.
    let mut stderr = recv_lines_until(&mut child, &stderr_rx, "ANVIL_MCP_NO_REEXEC");
    assert!(
        stderr.contains("ANVIL_MCP_NO_REEXEC") || stderr.contains("not the preferred"),
        "kill-switch must surface honest skew, got: {stderr}"
    );
    assert!(
        !stderr.to_ascii_lowercase().contains("restart your editor"),
        "kill-switch hint must not lead with editor restart: {stderr}"
    );

    drop(child.stdin.take());
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "serve must exit cleanly after EOF: {status:?}"
    );
    stderr.push_str(&drain_lines(&stderr_rx));
    assert!(
        !stderr.to_ascii_lowercase().contains("restart your editor"),
        "kill-switch hint must not lead with editor restart after exit: {stderr}"
    );
}

#[test]
fn mcp_reexec_anti_loop_stays_when_already_reexeced() {
    let (_dir, preferred) = copy_anvil_as_preferred();
    let mut child = spawn_serve(&[
        ("ANVIL_MCP_PREFERRED", preferred.to_str().expect("utf8")),
        ("ANVIL_MCP_NO_REEXEC", ""),
        ("ANVIL_MCP_REEXECED", "1"),
    ]);
    let stdout = child.stdout.take().expect("stdout");
    let stdout_rx = spawn_stdout_reader(stdout);

    send_initialize(&mut child, &stdout_rx);

    #[cfg(target_os = "linux")]
    assert_running_image(&child, Path::new(ANVIL_BIN));

    drop(child.stdin.take());
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "serve must exit cleanly after EOF: {status:?}"
    );
}

#[test]
fn mcp_reexec_anti_loop_holds_when_generation_file_already_exists() {
    let (_dir, preferred) = copy_anvil_as_preferred();
    let home = tempfile::tempdir().expect("anvil home");
    std::fs::write(home.path().join("mcp-refresh.generation"), "3\n")
        .expect("write existing generation");
    let home_str = home.path().to_str().expect("utf8 home");
    let preferred_str = preferred.to_str().expect("utf8 preferred");
    let mut child = spawn_serve(&[
        ("ANVIL_MCP_PREFERRED", preferred_str),
        ("ANVIL_MCP_NO_REEXEC", ""),
        ("ANVIL_MCP_REEXECED", "1"),
        ("ANVIL_HOME", home_str),
    ]);
    let stdout = child.stdout.take().expect("stdout");
    let stdout_rx = spawn_stdout_reader(stdout);

    send_initialize(&mut child, &stdout_rx);

    #[cfg(target_os = "linux")]
    assert_running_image(&child, Path::new(ANVIL_BIN));

    drop(child.stdin.take());
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "serve must exit cleanly after EOF: {status:?}"
    );
}

#[test]
fn mcp_reexec_reconnect_instruction_is_once_across_trigger_flow_for_each_gate() {
    let (_dir, preferred) = copy_anvil_as_preferred();
    let preferred = preferred.to_str().expect("utf8 preferred");
    let cases = [("kill-switch", "1", ""), ("already-reexeced", "", "1")];

    for (condition, no_reexec, reexeced) in cases {
        let mut child = spawn_serve(&[
            ("ANVIL_MCP_PREFERRED", preferred),
            ("ANVIL_MCP_NO_REEXEC", no_reexec),
            ("ANVIL_MCP_REEXECED", reexeced),
        ]);
        let stdout = child.stdout.take().expect("stdout");
        let stderr = child.stderr.take().expect("stderr");
        let stdout_rx = spawn_stdout_reader(stdout);
        let stderr_rx = spawn_stdout_reader(stderr);

        // Recovery hint is written from a detached stderr thread. Await the
        // reconnect instruction while the child is still alive so process
        // exit cannot race the write away (Windows Cross Nightly saw 0).
        let mut stderr = recv_lines_until(&mut child, &stderr_rx, "reconnect mcp for this client");
        send_initialize(&mut child, &stdout_rx);
        send_tools_list(&mut child, &stdout_rx);

        drop(child.stdin.take());
        let status = wait_for_exit(&mut child);
        assert!(
            status.success(),
            "{condition} serve must exit cleanly after EOF: {status:?}"
        );

        stderr.push_str(&drain_lines(&stderr_rx));
        assert_eq!(
            reconnect_instruction_count(&stderr),
            1,
            "{condition} must emit one reconnect instruction total across startup, initialize, and tools/list: {stderr}"
        );
    }
}

#[cfg(unix)]
#[test]
fn mcp_reexec_failed_exec_recovery_warning_is_once_across_trigger_flow() {
    let dir = tempfile::tempdir().expect("preferred parent");
    let missing_preferred = dir.path().join("missing-anvil");
    let mut child = spawn_serve(&[
        (
            "ANVIL_MCP_PREFERRED",
            missing_preferred.to_str().expect("utf8 preferred"),
        ),
        ("ANVIL_MCP_NO_REEXEC", ""),
        ("ANVIL_MCP_REEXECED", ""),
    ]);
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let stdout_rx = spawn_stdout_reader(stdout);
    let stderr_rx = spawn_stdout_reader(stderr);

    send_initialize(&mut child, &stdout_rx);
    send_tools_list(&mut child, &stdout_rx);
    send_tools_list(&mut child, &stdout_rx);

    drop(child.stdin.take());
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "serve must exit cleanly after failed re-exec: {status:?}"
    );

    let stderr = drain_lines(&stderr_rx);
    assert!(
        stderr.contains(&format!(
            "failed to re-exec {}",
            missing_preferred.display()
        )),
        "failed re-exec must retain its cause details: {stderr}"
    );
    assert_eq!(
        reconnect_instruction_count(&stderr),
        1,
        "failed re-exec must emit one reconnect instruction total across startup, initialize, and repeated tools/list: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn established_session_preserves_pipelined_legacy_and_modern_requests_after_update() {
    let (_preferred_dir, preferred, replacement) = swappable_preferred();
    let home = tempfile::tempdir().expect("anvil home");
    publish_generation(home.path(), 1);
    let mut child = spawn_serve(&[
        (
            "ANVIL_MCP_PREFERRED",
            preferred.to_str().expect("utf8 preferred"),
        ),
        ("ANVIL_HOME", home.path().to_str().expect("utf8 home")),
        ("ANVIL_MCP_NO_REEXEC", ""),
        ("ANVIL_MCP_REEXECED", ""),
    ]);
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let stdout_rx = spawn_stdout_reader(stdout);
    let stderr_rx = spawn_stdout_reader(stderr);

    send_initialize(&mut child, &stdout_rx);
    replace_preferred(&preferred, &replacement);
    publish_generation(home.path(), 2);

    let legacy = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    let modern = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list",
        "params": {
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{legacy}").expect("pipeline legacy request");
        writeln!(stdin, "{modern}").expect("pipeline modern request");
        stdin.flush().expect("flush pipeline");
    }

    let legacy_response = recv_json_response(&mut child, &stdout_rx, "legacy");
    let modern_response = recv_json_response(&mut child, &stdout_rx, "modern");
    assert_eq!(legacy_response["id"], 2);
    assert!(legacy_response["result"]["tools"].is_array());
    for modern_field in ["resultType", "ttlMs", "cacheScope", "_meta"] {
        assert!(
            legacy_response["result"].get(modern_field).is_none(),
            "legacy response must omit modern field {modern_field}: {legacy_response}"
        );
    }
    assert_eq!(modern_response["id"], 3);
    assert!(modern_response["result"]["tools"].is_array());
    assert_eq!(modern_response["result"]["resultType"], "complete");
    assert_eq!(
        modern_response["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "anvil"
    );

    drop(child.stdin.take());
    assert!(wait_for_exit(&mut child).success());
    let stderr = drain_lines(&stderr_rx);
    assert_eq!(
        reconnect_instruction_count(&stderr),
        1,
        "established session must require one explicit reconnect: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn established_session_processes_mutation_once_when_replacement_is_missing() {
    let workspace = tempfile::tempdir().expect("workspace");
    let source = workspace.path().join("source.ts");
    std::fs::write(&source, "const value: any = 1;\n").expect("write fixture");
    let (_preferred_dir, preferred, _replacement) = swappable_preferred();
    let mut child = spawn_serve_in(
        workspace.path(),
        &[
            (
                "ANVIL_MCP_PREFERRED",
                preferred.to_str().expect("utf8 preferred"),
            ),
            ("ANVIL_DEV", "1"),
            ("ANVIL_MCP_NO_REEXEC", ""),
            ("ANVIL_MCP_REEXECED", ""),
        ],
    );
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let stdout_rx = spawn_stdout_reader(stdout);
    let stderr_rx = spawn_stdout_reader(stderr);

    send_initialize(&mut child, &stdout_rx);
    std::fs::remove_file(&preferred).expect("remove initial preferred link");
    send_suppress(&mut child, 2, workspace.path(), "JREL-001 continuity proof");
    let response = recv_json_response(&mut child, &stdout_rx, "mutation");
    assert_eq!(response["id"], 2);
    assert_eq!(response["result"]["isError"], false, "{response}");

    let mut stderr = recv_lines_until(&mut child, &stderr_rx, "reconnect mcp for this client");
    drop(child.stdin.take());
    assert!(wait_for_exit(&mut child).success());
    let on_disk = std::fs::read_to_string(&source).expect("read mutated fixture");
    assert_eq!(
        on_disk.matches("JREL-001 continuity proof").count(),
        1,
        "the accepted mutation must run exactly once: {on_disk}"
    );
    stderr.push_str(&drain_lines(&stderr_rx));
    assert_eq!(
        reconnect_instruction_count(&stderr),
        1,
        "unresolved preferred binary must leave one explicit reconnect instruction: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn established_session_survives_closed_stderr_and_keeps_sequential_requests() {
    let workspace = tempfile::tempdir().expect("workspace");
    let source = workspace.path().join("source.ts");
    std::fs::write(&source, "const value: any = 1;\n").expect("write fixture");
    let (_preferred_dir, preferred, replacement) = swappable_preferred();
    let mut child = spawn_serve_in(
        workspace.path(),
        &[
            (
                "ANVIL_MCP_PREFERRED",
                preferred.to_str().expect("utf8 preferred"),
            ),
            ("ANVIL_DEV", "1"),
        ],
    );
    let stdout = child.stdout.take().expect("stdout");
    let stdout_rx = spawn_stdout_reader(stdout);
    let stderr = child.stderr.take().expect("stderr");

    send_initialize(&mut child, &stdout_rx);
    drop(stderr);
    replace_preferred(&preferred, &replacement);
    send_tools_list(&mut child, &stdout_rx);
    send_suppress(
        &mut child,
        3,
        workspace.path(),
        "JREL-001 closed stderr continuity",
    );
    let response = recv_json_response(&mut child, &stdout_rx, "mutation");
    assert_eq!(response["id"], 3);
    assert_eq!(response["result"]["isError"], false, "{response}");

    drop(child.stdin.take());
    assert!(wait_for_exit(&mut child).success());
    let on_disk = std::fs::read_to_string(&source).expect("read mutated fixture");
    assert_eq!(
        on_disk.matches("JREL-001 closed stderr continuity").count(),
        1,
        "the accepted mutation reason must occur exactly once: {on_disk}"
    );
}

#[cfg(unix)]
#[test]
fn established_session_survives_saturated_stderr_and_keeps_pipelined_requests() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    let workspace = tempfile::tempdir().expect("workspace");
    let source = workspace.path().join("source.ts");
    std::fs::write(&source, "const value: any = 1;\n").expect("write fixture");
    let (_preferred_dir, preferred, replacement) = swappable_preferred();
    let (child_stderr, blocked_reader) = UnixStream::pair().expect("stderr socket pair");
    let mut saturator = child_stderr.try_clone().expect("clone child stderr");
    let mut child = spawn_serve_in_with_stderr(
        workspace.path(),
        &[
            (
                "ANVIL_MCP_PREFERRED",
                preferred.to_str().expect("utf8 preferred"),
            ),
            ("ANVIL_DEV", "1"),
        ],
        Stdio::from(OwnedFd::from(child_stderr)),
    );
    let stdout = child.stdout.take().expect("stdout");
    let stdout_rx = spawn_stdout_reader(stdout);

    send_initialize(&mut child, &stdout_rx);
    saturator
        .set_nonblocking(true)
        .expect("nonblocking stderr filler");
    let block = [b'x'; 16 * 1024];
    loop {
        match saturator.write(&block) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(err) => panic!("saturate stderr: {err}"),
        }
    }
    // UnixStream clones share their file status flags. Restore blocking mode
    // after filling the shared socket so the child's inherited stderr really
    // exercises a blocking diagnostic write.
    saturator
        .set_nonblocking(false)
        .expect("restore blocking stderr");
    replace_preferred(&preferred, &replacement);

    let list = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    let suppress = suppress_request(3, workspace.path(), "JREL-001 saturated stderr continuity");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{list}").expect("pipeline tools/list");
        writeln!(stdin, "{suppress}").expect("pipeline mutation");
        stdin.flush().expect("flush pipeline");
    }

    let list_response = recv_json_response(&mut child, &stdout_rx, "tools/list");
    let mutation_response = recv_json_response(&mut child, &stdout_rx, "mutation");
    assert_eq!(list_response["id"], 2);
    assert!(list_response["result"]["tools"].is_array());
    assert_eq!(mutation_response["id"], 3);
    assert_eq!(mutation_response["result"]["isError"], false);

    drop(child.stdin.take());
    assert!(wait_for_exit(&mut child).success());
    drop(saturator);
    drop(blocked_reader);
    let on_disk = std::fs::read_to_string(&source).expect("read mutated fixture");
    assert_eq!(
        on_disk
            .matches("JREL-001 saturated stderr continuity")
            .count(),
        1,
        "the accepted mutation reason must occur exactly once: {on_disk}"
    );
}

fn copy_anvil_as_preferred() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("anvil");
    std::fs::copy(ANVIL_BIN, &dest).expect("copy anvil");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest).expect("meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms).expect("chmod");
    }
    (dir, dest)
}

#[cfg(unix)]
fn swappable_preferred() -> (tempfile::TempDir, PathBuf, PathBuf) {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let dir = tempfile::tempdir().expect("preferred parent");
    let preferred = dir.path().join("anvil");
    symlink(ANVIL_BIN, &preferred).expect("link initial preferred");
    let replacement = dir.path().join("anvil-replacement");
    std::fs::copy(ANVIL_BIN, &replacement).expect("copy replacement");
    let mut permissions = std::fs::metadata(&replacement)
        .expect("replacement metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&replacement, permissions).expect("chmod replacement");
    (dir, preferred, replacement)
}

#[cfg(unix)]
fn replace_preferred(preferred: &Path, replacement: &Path) {
    use std::os::unix::fs::symlink;

    let next = preferred.with_extension("next");
    symlink(replacement, &next).expect("link replacement preferred");
    std::fs::rename(next, preferred).expect("publish preferred update");
}

#[cfg(unix)]
fn publish_generation(home: &Path, generation: u64) {
    std::fs::write(
        home.join("mcp-refresh.generation"),
        format!("{generation}\n"),
    )
    .expect("publish update generation");
}

fn spawn_serve(env: &[(&str, &str)]) -> Child {
    spawn_serve_in(Path::new("."), env)
}

fn spawn_serve_in(cwd: &Path, env: &[(&str, &str)]) -> Child {
    spawn_serve_in_with_stderr(cwd, env, Stdio::piped())
}

fn init_git_worktree(root: &Path) {
    let git = root.join(".git");
    if git.exists() {
        return;
    }
    fs::create_dir_all(git.join("refs")).expect("git refs");
    fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").expect("HEAD");
}

fn spawn_serve_in_with_stderr(cwd: &Path, env: &[(&str, &str)], stderr: Stdio) -> Child {
    init_git_worktree(cwd);
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.current_dir(cwd)
        .arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(stderr);
    for (key, value) in env {
        if value.is_empty() {
            cmd.env_remove(key);
        } else {
            cmd.env(key, value);
        }
    }
    cmd.spawn().expect("spawn anvil mcp serve --stdio")
}

#[cfg(unix)]
fn suppress_request(id: u64, workspace: &Path, reason: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": "anvil_suppress",
            "arguments": {
                "workspaceRoot": workspace,
                "filePath": "source.ts",
                "warningId": "AP-003",
                "line": 1,
                "reason": reason
            }
        }
    })
}

#[cfg(unix)]
fn send_suppress(child: &mut Child, id: u64, workspace: &Path, reason: &str) {
    let request = suppress_request(id, workspace, reason);
    let stdin = child.stdin.as_mut().expect("stdin");
    writeln!(stdin, "{request}").expect("send mutating request");
}

#[cfg(unix)]
fn recv_json_response(
    child: &mut Child,
    rx: &Receiver<std::io::Result<String>>,
    label: &str,
) -> Value {
    let line = recv_stdout_line(child, rx);
    serde_json::from_str(line.trim())
        .unwrap_or_else(|err| panic!("{label} response must be JSON-RPC JSON, got {line:?}: {err}"))
}

fn reconnect_instruction_count(stderr: &str) -> usize {
    stderr
        .to_ascii_lowercase()
        .matches("reconnect mcp for this client")
        .count()
}

fn send_initialize(child: &mut Child, rx: &Receiver<std::io::Result<String>>) {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "mcplh-002-test",
                "version": "0.0.0"
            }
        }
    });
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{request}").expect("write initialize");
    }
    let line = recv_stdout_line(child, rx);
    let parsed: Value = serde_json::from_str(line.trim()).unwrap_or_else(|err| {
        panic!("initialize must be JSON-RPC JSON, got {line:?}: {err}");
    });
    assert_eq!(parsed["result"]["serverInfo"]["name"], "anvil");
    assert_eq!(
        parsed["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION"),
        "preferred image must still be this crate's version: {parsed}"
    );
}

fn send_tools_list(child: &mut Child, rx: &Receiver<std::io::Result<String>>) {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{request}").expect("write tools/list");
    }
    let line = recv_stdout_line(child, rx);
    let parsed: Value = serde_json::from_str(line.trim()).unwrap_or_else(|err| {
        panic!("tools/list must be JSON-RPC JSON, got {line:?}: {err}");
    });
    assert!(
        parsed["result"]["tools"].is_array(),
        "tools/list must return the tools array: {parsed}"
    );
}

#[cfg(target_os = "linux")]
fn assert_running_image(child: &Child, expected: &Path) {
    let proc_exe = PathBuf::from(format!("/proc/{}/exe", child.id()));
    let running = std::fs::canonicalize(&proc_exe)
        .unwrap_or_else(|err| panic!("canonicalize {}: {err}", proc_exe.display()));
    let want = std::fs::canonicalize(expected)
        .unwrap_or_else(|err| panic!("canonicalize {}: {err}", expected.display()));
    assert_eq!(
        running,
        want,
        "running image must be {} (pid {})",
        want.display(),
        child.id()
    );
}

fn spawn_stdout_reader(pipe: impl Read + Send + 'static) -> Receiver<std::io::Result<String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if tx.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = tx.send(Err(err));
                    break;
                }
            }
        }
    });
    rx
}

fn recv_stdout_line(child: &mut Child, rx: &Receiver<std::io::Result<String>>) -> String {
    match rx.recv_timeout(CHILD_TIMEOUT) {
        Ok(Ok(line)) => line,
        Ok(Err(err)) => panic!("failed to read child stdout: {err}"),
        Err(err) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("timed out waiting for child stdout: {err}");
        }
    }
}

fn drain_lines(rx: &Receiver<std::io::Result<String>>) -> String {
    let mut out = String::new();
    while let Ok(Ok(line)) = rx.recv_timeout(Duration::from_millis(200)) {
        out.push_str(&line);
    }
    out
}

fn recv_lines_until(
    child: &mut Child,
    rx: &Receiver<std::io::Result<String>>,
    expected: &str,
) -> String {
    let started = Instant::now();
    let expected = expected.to_ascii_lowercase();
    let mut observed = String::new();
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(line)) => {
                observed.push_str(&line);
                if observed.to_ascii_lowercase().contains(&expected) {
                    return observed;
                }
            }
            Ok(Err(err)) => panic!("failed to read child stderr: {err}"),
            Err(mpsc::RecvTimeoutError::Timeout) if started.elapsed() <= CHILD_TIMEOUT => {}
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "timed out waiting for child stderr to contain {expected:?}; observed: {observed}"
                );
            }
        }
    }
}

fn wait_for_exit(child: &mut Child) -> std::process::ExitStatus {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) if started.elapsed() <= CHILD_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                return child.wait().expect("wait after kill");
            }
            Err(err) => panic!("wait failed: {err}"),
        }
    }
}
