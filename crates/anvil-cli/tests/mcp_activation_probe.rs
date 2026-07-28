//! MCP26-007: black-box contract for the activation modern discover probe.
//!
//! The unit suite covers dual-era fallback and child reaping. This test pins
//! the built `anvil` binary's discover response shape the probe depends on.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Ensures a spawned MCP server is killed and reaped even when the test panics
/// (for example on discover response timeout).
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn installed_anvil_answers_modern_discover_for_activation_probe() {
    let child = Command::new(ANVIL_BIN)
        .args(["mcp", "serve", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn anvil mcp serve");
    let mut child = KillOnDrop(child);

    let mut stdin = child.0.stdin.take().expect("stdin");
    let stdout = child.0.stdout.take().expect("stdout");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        let _ = tx.send(line);
    });

    let request = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{{"_meta":{{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{{}},"io.modelcontextprotocol/clientInfo":{{"name":"anvil-probe","version":"{}"}}}}}}}}"#,
        env!("CARGO_PKG_VERSION")
    );
    writeln!(stdin, "{request}").expect("write discover");
    drop(stdin);

    let line = match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(line) => line,
        Err(_) => panic!("discover response within timeout"),
    };

    let value: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
    assert_eq!(
        value["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "anvil"
    );
    assert_eq!(value["result"]["supportedVersions"][0], "2026-07-28");
    assert_eq!(value["result"]["resultType"], "complete");
}
