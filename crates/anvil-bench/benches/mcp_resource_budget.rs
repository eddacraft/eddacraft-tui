//! MCP server CPU/RSS budget bench (RLB-004).
//!
//! The MCP server (`anvil mcp serve --stdio`) is the third long-running Anvil
//! process and, before this, had no resource coverage. It speaks newline-
//! delimited JSON-RPC over stdio and is single-threaded and strictly 1:1
//! (one client, one request in flight), so "sustained load" is a single
//! driver writing `tools/call` requests as fast as the server answers.
//!
//! The bench drives `anvil_validate_write` — the production pre-write gate —
//! against a real synthetic workspace with `ANVIL_DEV=1` (which makes the tool
//! auth check pass, so the server runs the real embedded secret/anti-pattern
//! scan on every proposed buffer rather than short-circuiting). It measures the
//! server's process tree and evaluates against
//! [`ResourceBudget::ANVIL_MCP_BUSY_V1`].

use std::error::Error;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::Duration;

use anvil_bench::budget::{ResourceBudget, evaluate};
use anvil_bench::fixture::{RepoSpec, generate_repo};
use anvil_bench::proc_sampler::TreeSampler;
use anvil_bench::spawn::{ManagedChild, in_new_process_group, resolve_anvil_binary};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const SETTLE: Duration = Duration::from_millis(500);
const MEASURE_WINDOW: Duration = Duration::from_secs(5);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);

fn main() {
    let exit = match run() {
        Ok(verdict) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&verdict).expect("verdict serialises")
            );
            i32::from(verdict.status.is_fail())
        }
        Err(err) => {
            eprintln!("mcp_resource_budget: {err}");
            1
        }
    };
    std::process::exit(exit);
}

fn run() -> Result<anvil_bench::budget::BudgetVerdict> {
    if !cfg!(target_os = "linux") {
        return Err("mcp resource budget sampling requires Linux /proc".into());
    }

    let bin = resolve_anvil_binary()?;
    let tempdir = tempfile::tempdir()?;
    // validate_write resolves the workspace from the server cwd, so the server
    // must launch inside a real repo directory.
    let repo = generate_repo(&RepoSpec::small(), tempdir.path())?;

    let mut command = Command::new(&bin);
    command
        .args(["mcp", "serve", "--stdio"])
        .current_dir(repo.root())
        .env("ANVIL_DEV", "1")
        .env("ANVIL_DISABLE_UPDATE_HINT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = in_new_process_group(&mut command).spawn()?;

    let mut stdin = child.stdin.take().ok_or("child stdin not piped")?;
    let stdout = child.stdout.take().ok_or("child stdout not piped")?;
    let mut reader = BufReader::new(stdout);
    let mut server = ManagedChild::new(child, "anvil mcp serve");
    let pid = server.id();

    // MCP handshake: initialize → read result → initialized notification.
    send(&mut stdin, &initialize_request())?;
    let init = read_line(&mut reader)?;
    if !response_has_result(&init) {
        server.ensure_running("after initialize")?;
        return Err(format!("mcp initialize did not return a result: {init}").into());
    }
    send(&mut stdin, &initialized_notification())?;

    // One real tool call before measuring, so a broken tool path is a loud
    // error rather than a misleadingly-idle measurement.
    server.ensure_running("after handshake")?;
    drive_tool_call(&mut stdin, &mut reader, 0)?;

    std::thread::sleep(SETTLE);
    server.ensure_running("after settle")?;

    // Drive sustained tools/call load on a worker thread while the sampler
    // ticks RSS on this thread; the server CPU is captured by the tree sampler.
    // The driver breaks on stop OR on the first request error (e.g. when we kill
    // the server during teardown) and reports the call count — a *real* mid-run
    // server death is caught separately by the post-window liveness check below.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let driver_stop = std::sync::Arc::clone(&stop);
    let driver = std::thread::spawn(move || -> u64 {
        let mut seq: u64 = 1;
        while !driver_stop.load(Ordering::Acquire) {
            if drive_tool_call(&mut stdin, &mut reader, seq).is_err() {
                break;
            }
            seq += 1;
        }
        seq - 1
    });

    let mut sampler = TreeSampler::start(pid)?;
    sampler.sample_for(MEASURE_WINDOW, SAMPLE_INTERVAL);
    // Validate the server survived the window, then close the measurement before
    // tearing anything down.
    server.ensure_running("after measurement window")?;
    let sample = sampler.finish()?;

    // Teardown order matters: signal stop, then kill the server so a driver
    // blocked in read_line is unblocked by EOF — otherwise driver.join() could
    // hang past the window with no timeout.
    stop.store(true, Ordering::Release);
    server.shutdown();
    let calls = driver
        .join()
        .map_err(|_| -> Box<dyn Error + Send + Sync> { "driver thread panicked".into() })?;

    eprintln!("mcp_resource_budget: drove {calls} tools/call requests over the window");
    Ok(evaluate(ResourceBudget::ANVIL_MCP_BUSY_V1, sample))
}

fn send(stdin: &mut impl Write, message: &serde_json::Value) -> Result<()> {
    let mut line = serde_json::to_vec(message)?;
    line.push(b'\n');
    stdin.write_all(&line)?;
    stdin.flush()?;
    Ok(())
}

fn read_line(reader: &mut impl BufRead) -> Result<String> {
    let mut line = String::new();
    let n = reader.read_line(&mut line)?;
    if n == 0 {
        return Err("mcp server closed stdout".into());
    }
    Ok(line)
}

/// Send one `tools/call` and read its response; error on a non-result reply so
/// the driver loop does not silently spin on a failing server.
fn drive_tool_call(stdin: &mut impl Write, reader: &mut impl BufRead, seq: u64) -> Result<()> {
    send(stdin, &tools_call_request(seq))?;
    let response = read_line(reader)?;
    if !response_has_result(&response) {
        return Err(format!("tools/call {seq} returned no result: {response}").into());
    }
    Ok(())
}

/// A JSON-RPC reply counts as success iff it parses and carries a top-level
/// `result` with no `error` — more robust than a `contains("result")` substring
/// match, which an error message mentioning "result" would spuriously satisfy.
fn response_has_result(line: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .is_some_and(|v| v.get("result").is_some() && v.get("error").is_none())
}

fn initialize_request() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": "init",
        "method": "initialize",
        "params": { "protocolVersion": "2024-11-05", "capabilities": {} }
    })
}

fn initialized_notification() -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
}

/// A realistic `anvil_validate_write` call. The path varies per request (so the
/// correlation id differs and nothing is trivially cached) and the proposed
/// content is a small but non-trivial source buffer the embedded scanner walks.
fn tools_call_request(seq: u64) -> serde_json::Value {
    let mut content = String::from("import { readFileSync } from 'node:fs';\n");
    for i in 0..40 {
        let _ = writeln!(
            content,
            "export function compute_{seq}_{i}(x: number): number {{ return x + {i}; }}"
        );
    }
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": seq,
        "method": "tools/call",
        "params": {
            "name": "anvil_validate_write",
            "arguments": {
                "path": format!("src/bench_{seq}.ts"),
                "operation": "create",
                "proposedContent": content
            }
        }
    })
}
