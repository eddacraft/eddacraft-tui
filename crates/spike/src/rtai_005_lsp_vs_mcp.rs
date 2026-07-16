//! RTAI-005 spike: measure LSP vs MCP wire-protocol overhead for the
//! mid-edit diagnostics loop, per ADR-109 (protocol plurality — anvil
//! supports both, not one instead of the other).
//!
//! What this is:
//! - A standalone binary that spawns the *real* `anvil lsp --stdio` and
//!   `anvil mcp serve --stdio` subcommands against a *real, running*
//!   intercept daemon, drives `N` round-trips of the same fixture
//!   (a `ghp_...` token that fires `secret-detection`) over each
//!   protocol, and reports round-trip latency plus wire-payload size
//!   for both.
//!
//! What this is **not**:
//! - Production code, and not shaped like RTAI-001's spike. RTAI-001
//!   simulated the daemon in-process (`mpsc` channel) to measure a
//!   protocol-free floor; this spike does the opposite — it holds the
//!   daemon and rule fixed and measures the *protocol* difference, so
//!   it genuinely needs `anvil lsp --stdio`/`anvil mcp serve --stdio`
//!   built and a live daemon reachable at `ANVIL_HOME`/the default
//!   socket path. See the companion report for the full setup.
//!
//! Run: `cargo run -q --release -p anvil-spike --bin spike-rtai-005-lsp-vs-mcp`
//! Requires `ANVIL_BIN` (path to a built `anvil` binary) and a running
//! daemon (`anvil intercept start --foreground`, ideally under a scratch
//! `ANVIL_HOME` so it does not touch a real install).

// Spike binary: percentile/ratio reporting casts small ints to f64/u32
// for human-readable output, same rationale as spike-rtai-mid-edit —
// samples never approach the precision-loss thresholds these lints
// guard against.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ITERATIONS: usize = 200;
const SECRET_TEXT: &str = "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n";
/// ADR-031 interactive buffer class: `validation.roundtrip` p95 <= 80ms
/// warm. The same budget both protocols are checked against below.
const ADR031_ROUNDTRIP_P95_BUDGET: Duration = Duration::from_millis(80);

fn main() {
    println!("RTAI-005 spike: LSP vs MCP mid-edit wire overhead");
    println!("---------------------------------------------------");

    let anvil_bin = std::env::var("ANVIL_BIN")
        .expect("set ANVIL_BIN to a built `anvil` binary (e.g. target/debug/anvil)");

    println!("anvil binary: {anvil_bin}");
    println!("iterations  : {ITERATIONS}");
    println!("fixture     : {SECRET_TEXT:?} (fires `secret-detection`)");
    println!();

    let lsp = run_lsp_bench(&anvil_bin, ITERATIONS);
    print_summary(
        "LSP (didChange -> scan_buffer mode=midEdit -> publishDiagnostics)",
        &lsp,
    );

    let mcp = run_mcp_bench(&anvil_bin, ITERATIONS);
    print_summary(
        "MCP (tools/call anvil_validate_write -> scan_buffer mode=preWrite)",
        &mcp,
    );

    println!();
    println!("=== Comparison ===");
    println!(
        "latency p50: LSP {:.2}ms vs MCP {:.2}ms",
        lsp.p50.as_secs_f64() * 1000.0,
        mcp.p50.as_secs_f64() * 1000.0,
    );
    println!(
        "latency p95: LSP {:.2}ms vs MCP {:.2}ms",
        lsp.p95.as_secs_f64() * 1000.0,
        mcp.p95.as_secs_f64() * 1000.0,
    );
    println!(
        "payload avg total: LSP {}B vs MCP {}B ({:.2}x)",
        lsp.avg_total_bytes(),
        mcp.avg_total_bytes(),
        mcp.avg_total_bytes() as f64 / lsp.avg_total_bytes() as f64,
    );
    println!();
    println!(
        "ADR-031 mid-edit budget (validation.roundtrip p95 <= {}ms, warm daemon):",
        ADR031_ROUNDTRIP_P95_BUDGET.as_millis(),
    );
    for (name, stats) in [("LSP", &lsp), ("MCP", &mcp)] {
        let verdict = if stats.p95 <= ADR031_ROUNDTRIP_P95_BUDGET {
            "PASS"
        } else {
            "FAIL"
        };
        println!(
            "  {name} p95={:.2}ms -> {verdict}",
            stats.p95.as_secs_f64() * 1000.0,
        );
    }
}

struct BenchStats {
    p50: Duration,
    p95: Duration,
    mean: Duration,
    avg_req_bytes: usize,
    avg_resp_bytes: usize,
}

impl BenchStats {
    fn avg_total_bytes(&self) -> usize {
        self.avg_req_bytes + self.avg_resp_bytes
    }
}

fn print_summary(label: &str, stats: &BenchStats) {
    println!();
    println!("=== {label} ===");
    println!(
        "latency  p50={:.2}ms  p95={:.2}ms  mean={:.2}ms",
        stats.p50.as_secs_f64() * 1000.0,
        stats.p95.as_secs_f64() * 1000.0,
        stats.mean.as_secs_f64() * 1000.0,
    );
    println!(
        "payload  avg_request={}B  avg_response={}B  avg_total={}B",
        stats.avg_req_bytes,
        stats.avg_resp_bytes,
        stats.avg_total_bytes(),
    );
}

fn summarize(
    mut latencies: Vec<Duration>,
    req_bytes: &[usize],
    resp_bytes: &[usize],
) -> BenchStats {
    latencies.sort_unstable();
    let n = latencies.len();
    let p50 = latencies[n / 2];
    let p95 = latencies[(n * 95) / 100];
    let mean = latencies.iter().sum::<Duration>() / n as u32;
    let avg_req_bytes = req_bytes.iter().sum::<usize>() / req_bytes.len();
    let avg_resp_bytes = resp_bytes.iter().sum::<usize>() / resp_bytes.len();
    BenchStats {
        p50,
        p95,
        mean,
        avg_req_bytes,
        avg_resp_bytes,
    }
}

// ---------- LSP path: Content-Length framing ----------

fn spawn(anvil_bin: &str, args: &[&str]) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(anvil_bin)
        .args(args)
        .env("ANVIL_DEV", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn `anvil {}` failed: {err}", args.join(" ")));
    let stdin = child.stdin.take().expect("piped stdin");
    let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    (child, stdin, stdout)
}

fn lsp_write(stdin: &mut ChildStdin, message: &Value) -> usize {
    let body = serde_json::to_vec(message).expect("serialise LSP message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write LSP header");
    stdin.write_all(&body).expect("write LSP body");
    stdin.flush().expect("flush LSP frame");
    body.len()
}

fn lsp_read(stdout: &mut BufReader<ChildStdout>) -> (Value, usize) {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).expect("read LSP header line");
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let len = content_length.expect("LSP frame carried Content-Length");
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).expect("read LSP body");
    (
        serde_json::from_slice(&body).expect("parse LSP body as JSON"),
        len,
    )
}

fn run_lsp_bench(anvil_bin: &str, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) = spawn(anvil_bin, &["lsp", "--stdio"]);

    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    lsp_read(&mut stdout);
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );

    let mut latencies = Vec::with_capacity(n);
    let mut req_bytes = Vec::with_capacity(n);
    let mut resp_bytes = Vec::with_capacity(n);
    for i in 0..n {
        let message = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": format!("file:///tmp/secret-{i}.ts"), "version": i },
                "contentChanges": [{ "text": SECRET_TEXT }]
            }
        });
        let start = Instant::now();
        let sent = lsp_write(&mut stdin, &message);
        let (_notification, received) = lsp_read(&mut stdout);
        latencies.push(start.elapsed());
        req_bytes.push(sent);
        resp_bytes.push(received);
    }

    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"shutdown"}),
    );
    lsp_read(&mut stdout);
    lsp_write(&mut stdin, &json!({"jsonrpc":"2.0","method":"exit"}));
    drop(stdin);
    child.wait().expect("lsp subprocess exits");

    summarize(latencies, &req_bytes, &resp_bytes)
}

// ---------- MCP path: NDJSON framing ----------

fn mcp_write(stdin: &mut ChildStdin, message: &Value) -> usize {
    let mut line = serde_json::to_string(message).expect("serialise MCP message");
    line.push('\n');
    stdin.write_all(line.as_bytes()).expect("write MCP frame");
    stdin.flush().expect("flush MCP frame");
    line.len()
}

fn mcp_read(stdout: &mut BufReader<ChildStdout>) -> (Value, usize) {
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read MCP response line");
    let len = line.len();
    (
        serde_json::from_str(&line).expect("parse MCP response as JSON"),
        len,
    )
}

fn run_mcp_bench(anvil_bin: &str, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) = spawn(anvil_bin, &["mcp", "serve", "--stdio"]);

    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    mcp_read(&mut stdout);
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
    );

    let mut latencies = Vec::with_capacity(n);
    let mut req_bytes = Vec::with_capacity(n);
    let mut resp_bytes = Vec::with_capacity(n);
    for i in 0..n {
        let message = json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "tools/call",
            "params": {
                "name": "anvil_validate_write",
                "arguments": {
                    "path": format!("src/secret-{i}.ts"),
                    "operation": "update",
                    "proposedContent": SECRET_TEXT
                }
            }
        });
        let start = Instant::now();
        let sent = mcp_write(&mut stdin, &message);
        let (_response, received) = mcp_read(&mut stdout);
        latencies.push(start.elapsed());
        req_bytes.push(sent);
        resp_bytes.push(received);
    }

    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":9999,"method":"shutdown"}),
    );
    mcp_read(&mut stdout);
    mcp_write(&mut stdin, &json!({"jsonrpc":"2.0","method":"exit"}));
    drop(stdin);
    child.wait().expect("mcp subprocess exits");

    summarize(latencies, &req_bytes, &resp_bytes)
}
