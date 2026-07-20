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
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
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
