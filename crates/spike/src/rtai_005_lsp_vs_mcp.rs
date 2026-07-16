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
//! - It also covers the three GCTX-backed capabilities the full LSP
//!   suite build added on top of the diagnostics-only spike:
//!   `textDocument/references` vs `anvil_find_callers`, and the two
//!   custom extension methods `anvil/impactOfChange`/`anvil/affectedTests`
//!   vs their `anvil_impact_of_change`/`anvil_affected_tests` MCP
//!   equivalents. Unlike the mid-edit path (a per-keystroke hot-path
//!   read, ADR-031/ADR-063), these are on-demand queries against the
//!   daemon's resident call graph — reported here for the same
//!   latency/payload comparison, but *not* checked against the ADR-031
//!   mid-edit budget, which does not apply to this timing class.
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
//! `ANVIL_HOME` so it does not touch a real install) — the capability
//! benches additionally need `ANVIL_HOME` set so this binary can resolve
//! the daemon's Unix socket directly for the one-off graph warm-up call.

// Spike binary: percentile/ratio reporting casts small ints to f64/u32
// for human-readable output, same rationale as spike-rtai-mid-edit —
// samples never approach the precision-loss thresholds these lints
// guard against.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ITERATIONS: usize = 200;
const SECRET_TEXT: &str = "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n";
/// ADR-031 interactive buffer class: `validation.roundtrip` p95 <= 80ms
/// warm. The same budget both protocols are checked against below.
const ADR031_ROUNDTRIP_P95_BUDGET: Duration = Duration::from_millis(80);

/// Round trips for the three on-demand capability benches. Smaller than
/// `ITERATIONS`: each round trip walks the daemon's resident call graph
/// (`symbol_at` -> `find_callers` -> `get_snippet`, or `impact_of_change`/
/// `affected_tests`), which does real traversal work per call rather than
/// the diagnostics path's fixed-cost buffer scan.
const CAPABILITY_ITERATIONS: usize = 50;

const GREET_FIXTURE: &str = "export function greet(name: string): string {\n    return `hello ${name}`;\n}\n\nexport function main(): void {\n    console.log(greet(\"world\"));\n    console.log(greet(\"anvil\"));\n}\n";
const GREET_RELATIVE_PATH: &str = "src/greet.ts";
/// Position of `greet` in `GREET_FIXTURE`'s first line
/// (`export function greet(`) — 0-based line/UTF-16-character for the LSP
/// side, byte offset for the raw daemon warm-up probe. Plain ASCII, so all
/// three units coincide.
const GREET_NAME_LINE: u32 = 0;
const GREET_NAME_CHARACTER: u32 = 16;
const GREET_NAME_BYTE_OFFSET: u32 = 16;

fn main() {
    println!("RTAI-005 spike: LSP vs MCP wire overhead");
    println!("---------------------------------------------------");

    let anvil_bin = std::env::var("ANVIL_BIN")
        .expect("set ANVIL_BIN to a built `anvil` binary (e.g. target/debug/anvil)");

    println!("anvil binary: {anvil_bin}");
    println!();

    run_diagnostics_pair(&anvil_bin);
    run_capability_pairs(&anvil_bin);
}

// ---------- Pair 1: mid-edit diagnostics (per-keystroke, ADR-031) ----------

fn run_diagnostics_pair(anvil_bin: &str) {
    println!("=== Capability: mid-edit diagnostics (per-keystroke, ADR-031) ===");
    println!("iterations  : {ITERATIONS}");
    println!("fixture     : {SECRET_TEXT:?} (fires `secret-detection`)");
    println!();

    let lsp = run_lsp_bench(anvil_bin, ITERATIONS);
    print_summary(
        "LSP (didChange -> scan_buffer mode=midEdit -> publishDiagnostics)",
        &lsp,
    );

    let mcp = run_mcp_bench(anvil_bin, ITERATIONS);
    print_summary(
        "MCP (tools/call anvil_validate_write -> scan_buffer mode=preWrite)",
        &mcp,
    );

    print_comparison(&lsp, &mcp);
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
    println!();
}

// ---------- Pairs 2-4: on-demand GCTX capabilities (not ADR-031-gated) ----------

#[cfg(unix)]
fn run_capability_pairs(anvil_bin: &str) {
    let workspace = tempfile::tempdir().expect("create fixture workspace tempdir");
    let src_dir = workspace.path().join("src");
    std::fs::create_dir_all(&src_dir).expect("create fixture src dir");
    std::fs::write(src_dir.join("greet.ts"), GREET_FIXTURE).expect("write fixture file");
    let workspace_root = workspace
        .path()
        .canonicalize()
        .expect("canonicalize fixture workspace root");

    warm_daemon_graph(&workspace_root);
    println!();

    println!("iterations  : {CAPABILITY_ITERATIONS}");
    println!("fixture     : {GREET_RELATIVE_PATH} (`greet` called twice from `main`)");
    println!();

    let refs_lsp = run_references_bench_lsp(anvil_bin, &workspace_root, CAPABILITY_ITERATIONS);
    print_summary(
        "LSP (textDocument/references -> symbol_at -> find_callers -> get_snippet)",
        &refs_lsp,
    );
    let refs_mcp = run_find_callers_bench_mcp(anvil_bin, &workspace_root, CAPABILITY_ITERATIONS);
    print_summary("MCP (tools/call anvil_find_callers)", &refs_mcp);
    print_comparison(&refs_lsp, &refs_mcp);
    println!();

    let impact_lsp = run_impact_bench_lsp(anvil_bin, &workspace_root, CAPABILITY_ITERATIONS);
    print_summary(
        "LSP (anvil/impactOfChange -> impact_of_change)",
        &impact_lsp,
    );
    let impact_mcp = run_impact_bench_mcp(anvil_bin, &workspace_root, CAPABILITY_ITERATIONS);
    print_summary("MCP (tools/call anvil_impact_of_change)", &impact_mcp);
    print_comparison(&impact_lsp, &impact_mcp);
    println!();

    let tests_lsp = run_affected_tests_bench_lsp(anvil_bin, &workspace_root, CAPABILITY_ITERATIONS);
    print_summary("LSP (anvil/affectedTests -> affected_tests)", &tests_lsp);
    let tests_mcp = run_affected_tests_bench_mcp(anvil_bin, &workspace_root, CAPABILITY_ITERATIONS);
    print_summary("MCP (tools/call anvil_affected_tests)", &tests_mcp);
    print_comparison(&tests_lsp, &tests_mcp);
    println!();
}

#[cfg(not(unix))]
fn run_capability_pairs(_anvil_bin: &str) {
    println!(
        "skipping references/impactOfChange/affectedTests capability benches: \
         the daemon warm-up probe requires a Unix domain socket (not available on this platform)"
    );
}

/// The 3 capability benches hit the daemon's resident call graph
/// (`symbol_at`/`find_callers`/`impact_of_change`/`affected_tests`), which —
/// unlike the diagnostics path's mid-edit buffer scan — needs the graph
/// actually built first. A never-scanned workspace has nothing to
/// auto-warm via the daemon's normal restore path (that only restores
/// previously *persisted* graphs), so this explicitly requests a full
/// scan and polls `symbol_at` until the graph reports `ready`.
#[cfg(unix)]
fn warm_daemon_graph(workspace_root: &Path) {
    let root = workspace_root.to_string_lossy().into_owned();
    println!("warming daemon graph for {root}...");
    daemon_rpc_call(
        "anvil/request_full_scan",
        &json!({ "workspace_root": root }),
    );

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let response = daemon_rpc_call(
            "anvil/gctx/symbol_at",
            &json!({
                "workspace_root": root,
                "query": { "file": GREET_RELATIVE_PATH, "byte_offset": GREET_NAME_BYTE_OFFSET }
            }),
        );
        let status = response
            .pointer("/result/outcome/status")
            .and_then(Value::as_str);
        if status == Some("ready") {
            println!("  graph ready");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "daemon graph for {root} did not become ready within 30s (last status: {status:?})"
        );
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(unix)]
fn daemon_socket_path() -> std::path::PathBuf {
    let anvil_home = std::env::var("ANVIL_HOME").expect(
        "set ANVIL_HOME to the scratch daemon home (needed to resolve the intercept socket \
         for the pre-benchmark graph warm-up call)",
    );
    std::path::PathBuf::from(anvil_home).join("intercept.sock")
}

#[cfg(unix)]
fn daemon_rpc_call(method: &str, params: &Value) -> Value {
    use std::os::unix::net::UnixStream;

    let socket_path = daemon_socket_path();
    let mut stream = UnixStream::connect(&socket_path).unwrap_or_else(|err| {
        panic!(
            "connect to daemon socket {} failed: {err}",
            socket_path.display()
        )
    });
    let mut line = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": "spike-warmup"
    }))
    .expect("serialise daemon warm-up request");
    line.push('\n');
    stream
        .write_all(line.as_bytes())
        .expect("write daemon warm-up request");
    stream.flush().expect("flush daemon warm-up request");

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .expect("read daemon warm-up response");
    serde_json::from_str(&response_line).expect("parse daemon warm-up response as JSON")
}

#[cfg(unix)]
fn run_references_bench_lsp(anvil_bin: &str, workspace_root: &Path, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) =
        spawn(anvil_bin, &["lsp", "--stdio"], Some(workspace_root));
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    lsp_read(&mut stdout);
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );

    let uri = format!(
        "file://{}",
        workspace_root.join(GREET_RELATIVE_PATH).display()
    );
    lsp_write(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": { "uri": uri, "text": GREET_FIXTURE, "version": 1 }
            }
        }),
    );

    let messages = (0..n).map(|i| {
        json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "textDocument/references",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": GREET_NAME_LINE, "character": GREET_NAME_CHARACTER },
                "context": { "includeDeclaration": false }
            }
        })
    });
    let stats = timed_lsp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_lsp(&mut child, stdin, &mut stdout);
    stats
}

#[cfg(unix)]
fn run_find_callers_bench_mcp(anvil_bin: &str, workspace_root: &Path, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) = spawn(
        anvil_bin,
        &["mcp", "serve", "--stdio"],
        Some(workspace_root),
    );
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    mcp_read(&mut stdout);
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
    );

    let root = workspace_root.to_string_lossy().into_owned();
    let messages = (0..n).map(|i| {
        json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "tools/call",
            "params": {
                "name": "anvil_find_callers",
                "arguments": {
                    "workspaceRoot": root,
                    "target": {
                        "file": GREET_RELATIVE_PATH,
                        "kind": "Function",
                        "name": "greet",
                        "ordinal": 0
                    },
                    "maxDepth": 1
                }
            }
        })
    });
    let stats = timed_mcp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_mcp(&mut child, stdin, &mut stdout);
    stats
}

#[cfg(unix)]
fn run_impact_bench_lsp(anvil_bin: &str, workspace_root: &Path, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) =
        spawn(anvil_bin, &["lsp", "--stdio"], Some(workspace_root));
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    lsp_read(&mut stdout);
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );

    let messages = (0..n).map(|i| {
        json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "anvil/impactOfChange",
            "params": { "changedFiles": [GREET_RELATIVE_PATH] }
        })
    });
    let stats = timed_lsp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_lsp(&mut child, stdin, &mut stdout);
    stats
}

#[cfg(unix)]
fn run_impact_bench_mcp(anvil_bin: &str, workspace_root: &Path, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) = spawn(
        anvil_bin,
        &["mcp", "serve", "--stdio"],
        Some(workspace_root),
    );
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    mcp_read(&mut stdout);
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
    );

    let root = workspace_root.to_string_lossy().into_owned();
    let messages = (0..n).map(|i| {
        json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "tools/call",
            "params": {
                "name": "anvil_impact_of_change",
                "arguments": {
                    "workspaceRoot": root,
                    "changedFiles": [GREET_RELATIVE_PATH],
                    "maxDepth": 1
                }
            }
        })
    });
    let stats = timed_mcp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_mcp(&mut child, stdin, &mut stdout);
    stats
}

#[cfg(unix)]
fn run_affected_tests_bench_lsp(anvil_bin: &str, workspace_root: &Path, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) =
        spawn(anvil_bin, &["lsp", "--stdio"], Some(workspace_root));
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    lsp_read(&mut stdout);
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );

    let messages = (0..n).map(|i| {
        json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "anvil/affectedTests",
            "params": { "changedFiles": [GREET_RELATIVE_PATH] }
        })
    });
    let stats = timed_lsp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_lsp(&mut child, stdin, &mut stdout);
    stats
}

#[cfg(unix)]
fn run_affected_tests_bench_mcp(anvil_bin: &str, workspace_root: &Path, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) = spawn(
        anvil_bin,
        &["mcp", "serve", "--stdio"],
        Some(workspace_root),
    );
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    mcp_read(&mut stdout);
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
    );

    let root = workspace_root.to_string_lossy().into_owned();
    let messages = (0..n).map(|i| {
        json!({
            "jsonrpc": "2.0",
            "id": i + 2,
            "method": "tools/call",
            "params": {
                "name": "anvil_affected_tests",
                "arguments": {
                    "workspaceRoot": root,
                    "changedFiles": [GREET_RELATIVE_PATH],
                    "maxDepth": 1
                }
            }
        })
    });
    let stats = timed_mcp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_mcp(&mut child, stdin, &mut stdout);
    stats
}

// ---------- Shared reporting ----------

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

fn print_comparison(lsp: &BenchStats, mcp: &BenchStats) {
    println!();
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

fn spawn(
    anvil_bin: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut command = Command::new(anvil_bin);
    command
        .args(args)
        .env("ANVIL_DEV", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    let mut child = command
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

fn timed_lsp_roundtrips(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    messages: impl Iterator<Item = Value>,
) -> BenchStats {
    let mut latencies = Vec::new();
    let mut req_bytes = Vec::new();
    let mut resp_bytes = Vec::new();
    for message in messages {
        let start = Instant::now();
        let sent = lsp_write(stdin, &message);
        let (_response, received) = lsp_read(stdout);
        latencies.push(start.elapsed());
        req_bytes.push(sent);
        resp_bytes.push(received);
    }
    summarize(latencies, &req_bytes, &resp_bytes)
}

fn shutdown_lsp(child: &mut Child, mut stdin: ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":9999,"method":"shutdown"}),
    );
    lsp_read(stdout);
    lsp_write(&mut stdin, &json!({"jsonrpc":"2.0","method":"exit"}));
    // Dropping stdin (closing the pipe) is what actually unblocks the
    // subprocess's read loop; `child.wait()` alone would hang.
    drop(stdin);
    child.wait().expect("lsp subprocess exits");
}

fn run_lsp_bench(anvil_bin: &str, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) = spawn(anvil_bin, &["lsp", "--stdio"], None);

    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    lsp_read(&mut stdout);
    lsp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    );

    let messages = (0..n).map(|i| {
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": format!("file:///tmp/secret-{i}.ts"), "version": i },
                "contentChanges": [{ "text": SECRET_TEXT }]
            }
        })
    });
    let stats = timed_lsp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_lsp(&mut child, stdin, &mut stdout);
    stats
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

fn timed_mcp_roundtrips(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    messages: impl Iterator<Item = Value>,
) -> BenchStats {
    let mut latencies = Vec::new();
    let mut req_bytes = Vec::new();
    let mut resp_bytes = Vec::new();
    for message in messages {
        let start = Instant::now();
        let sent = mcp_write(stdin, &message);
        let (_response, received) = mcp_read(stdout);
        latencies.push(start.elapsed());
        req_bytes.push(sent);
        resp_bytes.push(received);
    }
    summarize(latencies, &req_bytes, &resp_bytes)
}

fn shutdown_mcp(child: &mut Child, mut stdin: ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":9999,"method":"shutdown"}),
    );
    mcp_read(stdout);
    mcp_write(&mut stdin, &json!({"jsonrpc":"2.0","method":"exit"}));
    drop(stdin);
    child.wait().expect("mcp subprocess exits");
}

fn run_mcp_bench(anvil_bin: &str, n: usize) -> BenchStats {
    let (mut child, mut stdin, mut stdout) = spawn(anvil_bin, &["mcp", "serve", "--stdio"], None);

    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
    );
    mcp_read(&mut stdout);
    mcp_write(
        &mut stdin,
        &json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
    );

    let messages = (0..n).map(|i| {
        json!({
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
        })
    });
    let stats = timed_mcp_roundtrips(&mut stdin, &mut stdout, messages);

    shutdown_mcp(&mut child, stdin, &mut stdout);
    stats
}
