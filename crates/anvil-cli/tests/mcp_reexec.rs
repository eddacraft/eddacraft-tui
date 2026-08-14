//! MCPLH-002: `anvil mcp serve --stdio` re-execs into the preferred binary
//! between JSON-RPC messages. Unix proves the replacement image via
//! `/proc/<pid>/exe` when the platform exposes it.

use std::io::{BufRead, BufReader, Read, Write};
#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn mcp_reexec_unix_lands_on_preferred_after_forced_skew() {
    let preferred = copy_anvil_as_preferred();
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
    let preferred = copy_anvil_as_preferred();
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

    drop(child.stdin.take());
    let status = wait_for_exit(&mut child);
    assert!(
        status.success(),
        "serve must exit cleanly after EOF: {status:?}"
    );

    let stderr = drain_lines(&stderr_rx);
    assert!(
        stderr.contains("ANVIL_MCP_NO_REEXEC") || stderr.contains("not the preferred"),
        "kill-switch must surface honest skew, got: {stderr}"
    );
    assert!(
        !stderr.to_ascii_lowercase().contains("restart your editor"),
        "kill-switch hint must not lead with editor restart: {stderr}"
    );
}

#[test]
fn mcp_reexec_anti_loop_stays_when_already_reexeced() {
    let preferred = copy_anvil_as_preferred();
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

fn copy_anvil_as_preferred() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir").keep();
    let dest = dir.join("anvil");
    std::fs::copy(ANVIL_BIN, &dest).expect("copy anvil");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest).expect("meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms).expect("chmod");
    }
    dest
}

fn spawn_serve(env: &[(&str, &str)]) -> Child {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .arg("mcp")
        .arg("serve")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        if value.is_empty() {
            cmd.env_remove(key);
        } else {
            cmd.env(key, value);
        }
    }
    cmd.spawn().expect("spawn anvil mcp serve --stdio")
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
