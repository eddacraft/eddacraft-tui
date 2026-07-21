use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn daemon_down_lifecycle_publishes_an_empty_versioned_result() {
    let anvil_home = tempfile::tempdir().expect("temporary anvil home");
    let mut child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_anvil"))
            .args(["lsp", "--stdio"])
            .env("ANVIL_DEV", "1")
            .env("ANVIL_HOME", anvil_home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn anvil lsp"),
    );
    let mut stdin = child.0.stdin.take().expect("piped stdin");
    let stdout = child.0.stdout.take().expect("piped stdout");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Ok(Some(message)) = read_message(&mut reader) {
            if sender.send(message).is_err() {
                break;
            }
        }
    });

    write_message(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "rootUri":"file:///tmp"
        }}),
    );
    let initialize = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("initialize response");
    assert_eq!(initialize["result"]["capabilities"]["textDocumentSync"], 1);

    write_message(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );
    write_message(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didOpen",
            "params":{"textDocument":{
                "uri":"file:///tmp/anvil-lsp-daemon-down.rs",
                "languageId":"rust",
                "version":7,
                "text":"fn main() {}"
            }}
        }),
    );
    let publication = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("daemon-down diagnostics publication");
    assert_eq!(publication["method"], "textDocument/publishDiagnostics");
    assert_eq!(publication["params"]["version"], 7);
    assert_eq!(publication["params"]["diagnostics"], json!([]));

    write_message(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didClose",
            "params":{"textDocument":{"uri":"file:///tmp/anvil-lsp-daemon-down.rs"}}
        }),
    );
    let clear = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("diagnostic clear on close");
    assert_eq!(clear["params"]["diagnostics"], json!([]));
    assert!(clear["params"].get("version").is_none());

    write_message(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"shutdown"}),
    );
    let shutdown = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("shutdown response");
    assert_eq!(shutdown["id"], 2);
    write_message(&mut stdin, &json!({"jsonrpc":"2.0","method":"exit"}));
    drop(stdin);
    let status = child.0.wait().expect("wait for LSP process");
    assert!(status.success());
}

#[test]
fn malformed_frame_is_reported_then_terminates_without_hanging() {
    let anvil_home = tempfile::tempdir().expect("temporary anvil home");
    let mut child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_anvil"))
            .args(["lsp", "--stdio"])
            .env("ANVIL_DEV", "1")
            .env("ANVIL_HOME", anvil_home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn anvil lsp"),
    );
    let mut stdin = child.0.stdin.take().expect("piped stdin");
    stdin
        .write_all(b"Content-Length: 2\r\ncontent-length: 2\r\n\r\n{}")
        .expect("write malformed frame");
    stdin.flush().expect("flush malformed frame");
    drop(stdin);

    let mut reader = BufReader::new(child.0.stdout.take().expect("piped stdout"));
    let response = read_message(&mut reader)
        .expect("read parse error")
        .expect("parse error frame");
    assert_eq!(response["error"]["code"], -32700);
    let status = child.0.wait().expect("malformed frame process exits");
    assert!(status.success());
}

#[test]
fn exit_before_shutdown_is_an_abnormal_termination() {
    let anvil_home = tempfile::tempdir().expect("temporary anvil home");
    let mut child = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_anvil"))
            .args(["lsp", "--stdio"])
            .env("ANVIL_DEV", "1")
            .env("ANVIL_HOME", anvil_home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn anvil lsp"),
    );
    let mut stdin = child.0.stdin.take().expect("piped stdin");
    write_message(&mut stdin, &json!({"jsonrpc":"2.0","method":"exit"}));
    drop(stdin);
    let status = child.0.wait().expect("premature exit process exits");
    assert!(!status.success());
}

#[cfg(unix)]
#[test]
#[allow(clippy::too_many_lines)]
fn live_daemon_publishes_a_real_mid_edit_diagnostic() {
    let anvil_home = tempfile::tempdir().expect("temporary anvil home");
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let mut daemon = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_anvil"))
            .args(["intercept", "start", "--foreground"])
            .env("ANVIL_DEV", "1")
            .env("ANVIL_HOME", anvil_home.path())
            .env("HOME", anvil_home.path())
            .env("USERPROFILE", anvil_home.path())
            .current_dir(workspace.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn intercept daemon"),
    );
    let socket = anvil_home.path().join("intercept.sock");
    for _ in 0..60 {
        if socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(socket.exists(), "daemon socket did not become ready");

    let mut lsp = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_anvil"))
            .args(["lsp", "--stdio"])
            .env("ANVIL_DEV", "1")
            .env("ANVIL_HOME", anvil_home.path())
            .env("HOME", anvil_home.path())
            .env("USERPROFILE", anvil_home.path())
            .current_dir(workspace.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn anvil lsp"),
    );
    let mut stdin = lsp.0.stdin.take().expect("piped stdin");
    let stdout = lsp.0.stdout.take().expect("piped stdout");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Ok(Some(message)) = read_message(&mut reader) {
            if sender.send(message).is_err() {
                break;
            }
        }
    });
    let root_uri = format!("file://{}", workspace.path().display());
    let document_uri = format!("{root_uri}/src/app.ts");
    write_message(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "workspaceFolders":[{"uri":root_uri,"name":"fixture"}]
        }}),
    );
    receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("initialize response");
    write_message(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );
    write_message(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didOpen",
            "params":{"textDocument":{
                "uri":document_uri,
                "languageId":"typescript",
                "version":1,
                "text":"const AWS_KEY = \"AKIAIOSFODNN7EXAMPLE\";\n"
            }}
        }),
    );
    let publication = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("live diagnostics publication");
    assert_eq!(publication["params"]["version"], 1);
    assert!(
        publication["params"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty()),
        "known fake credential should produce a real daemon diagnostic: {publication}"
    );

    write_message(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didChange",
            "params":{
                "textDocument":{"uri":document_uri,"version":2},
                "contentChanges":[{"text":"const value = 1;\n"}]
            }
        }),
    );
    let clean = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("clean-buffer diagnostics publication");
    assert_eq!(clean["params"]["version"], 2);
    assert_eq!(clean["params"]["diagnostics"], json!([]));

    // A repeated/out-of-order version is ignored and cannot trigger another
    // daemon round-trip or stale publication.
    write_message(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didChange",
            "params":{
                "textDocument":{"uri":document_uri,"version":2},
                "contentChanges":[{"text":"const AWS_KEY = \"AKIAIOSFODNN7EXAMPLE\";\n"}]
            }
        }),
    );
    assert!(receiver.recv_timeout(Duration::from_millis(250)).is_err());

    write_message(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didClose",
            "params":{"textDocument":{"uri":document_uri}}
        }),
    );
    let close_publication = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("close diagnostics clear");
    assert_eq!(close_publication["params"]["diagnostics"], json!([]));
    assert!(close_publication["params"].get("version").is_none());

    write_message(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"shutdown"}),
    );
    receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("shutdown response");
    write_message(&mut stdin, &json!({"jsonrpc":"2.0","method":"exit"}));
    drop(stdin);
    assert!(lsp.0.wait().expect("wait for LSP").success());
    let _ = &mut daemon;
}

fn write_message(stdin: &mut ChildStdin, message: &Value) {
    let body = serde_json::to_vec(message).expect("serialise LSP message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write LSP header");
    stdin.write_all(&body).expect("write LSP body");
    stdin.flush().expect("flush LSP frame");
}

fn read_message(reader: &mut impl BufRead) -> std::io::Result<Option<Value>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(length) = content_length else {
        return Ok(None);
    };
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}
