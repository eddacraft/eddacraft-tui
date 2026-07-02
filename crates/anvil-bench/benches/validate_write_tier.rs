//! CIB-006: `anvil_validate_write` risk-tier wall-time comparison.
//!
//! Drives the real MCP server (`anvil mcp serve --stdio`) against a
//! representative JSON metadata fixture and measures the same logical
//! edit — a single string-value rename — through both validator tiers:
//!
//! - **full** — the whole post-image is sent as `proposedContent` and
//!   the complete pipeline scans every byte;
//! - **safelist** — the edit is sent as a unified-diff `patch`, matches
//!   the `json-single-string-value` safelist entry, and is served
//!   embedded with no daemon round-trip: the whole-file secret scan
//!   still covers the complete post-image, while the remaining rules
//!   run scoped to the touched node. The tiers therefore differ in
//!   wire payload, daemon IPC, and non-secret rule scope — not in
//!   secret coverage.
//!
//! The report is informational (exit 0 unless the protocol itself
//! fails): it prints per-tier mean latency and the safelist speedup so
//! regressions in the tiered path are visible without gating on timing
//! noise. Run on a quiet box; `pnpm bench` conventions apply.

use std::error::Error;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anvil_bench::spawn::{ManagedChild, in_new_process_group, resolve_anvil_binary};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Entries in the JSON metadata fixture. Mirrors the 2026-05-18 beta
/// incident shape (a one-string rename deep inside a metadata file) at
/// a size where full-content scanning cost is clearly visible.
const FIXTURE_ENTRIES: usize = 4000;
const WARMUP_ITERATIONS: usize = 3;
const MEASURED_ITERATIONS: usize = 50;
/// 1-based index of the renamed entry (the beta incident was idx 394).
const RENAMED_ENTRY: usize = 394;

fn main() {
    let exit = match run() {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report serialises")
            );
            0
        }
        Err(err) => {
            eprintln!("validate_write_tier: {err}");
            1
        }
    };
    std::process::exit(exit);
}

fn run() -> Result<serde_json::Value> {
    let bin = resolve_anvil_binary()?;
    let tempdir = tempfile::tempdir()?;
    let fixture = fixture_body();
    let fixture_path = tempdir.path().join("meta");
    std::fs::create_dir_all(&fixture_path)?;
    std::fs::write(fixture_path.join("tags.json"), &fixture)?;

    let mut command = Command::new(&bin);
    command
        .args(["mcp", "serve", "--stdio"])
        .current_dir(tempdir.path())
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

    send(&mut stdin, &initialize_request())?;
    let init = read_line(&mut reader)?;
    if !response_has_result(&init) {
        server.ensure_running("after initialize")?;
        return Err(format!("mcp initialize did not return a result: {init}").into());
    }
    send(&mut stdin, &initialized_notification())?;
    server.ensure_running("after handshake")?;

    let full_mean = measure_tier(
        &mut stdin,
        &mut reader,
        &full_content_request(&fixture),
        "full",
    )?;
    let safelist_mean = measure_tier(&mut stdin, &mut reader, &patch_request(), "safelist")?;
    server.ensure_running("after measurement")?;
    server.shutdown();

    let speedup = full_mean.as_secs_f64() / safelist_mean.as_secs_f64().max(f64::EPSILON);
    Ok(serde_json::json!({
        "bench": "validate_write_tier",
        "fixture_bytes": fixture.len(),
        "fixture_entries": FIXTURE_ENTRIES,
        "iterations": MEASURED_ITERATIONS,
        "full_mean_ms": full_mean.as_secs_f64() * 1e3,
        "safelist_mean_ms": safelist_mean.as_secs_f64() * 1e3,
        "safelist_speedup": speedup,
    }))
}

/// Drive one tier: warm up, then measure the mean wall time of
/// `MEASURED_ITERATIONS` calls. Every response must carry the expected
/// `tier.decision`, so a silently mis-tiered run fails loudly instead
/// of comparing two identical paths.
fn measure_tier(
    stdin: &mut impl Write,
    reader: &mut impl BufRead,
    request: &serde_json::Value,
    expected_tier: &str,
) -> Result<Duration> {
    for _ in 0..WARMUP_ITERATIONS {
        drive_call(stdin, reader, request, expected_tier)?;
    }
    let started = Instant::now();
    for _ in 0..MEASURED_ITERATIONS {
        drive_call(stdin, reader, request, expected_tier)?;
    }
    Ok(started.elapsed() / u32::try_from(MEASURED_ITERATIONS)?)
}

fn drive_call(
    stdin: &mut impl Write,
    reader: &mut impl BufRead,
    request: &serde_json::Value,
    expected_tier: &str,
) -> Result<()> {
    send(stdin, request)?;
    let response = read_line(reader)?;
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    if parsed.get("result").is_none() || parsed.get("error").is_some() {
        return Err(format!("tools/call returned no result: {response}").into());
    }
    let payload_text = parsed["result"]["content"][0]["text"]
        .as_str()
        .ok_or("tool result missing text payload")?;
    let payload: serde_json::Value = serde_json::from_str(payload_text)?;
    let tier = payload["tier"]["decision"].as_str().unwrap_or("<absent>");
    if tier != expected_tier {
        return Err(format!("expected tier {expected_tier}, server took {tier}: {payload}").into());
    }
    Ok(())
}

/// The representative JSON metadata fixture: `FIXTURE_ENTRIES` objects
/// with one renamable string tag each, newline-terminated one per line
/// so the patch below is a stable single-line hunk.
fn fixture_body() -> String {
    let mut body = String::from("[\n");
    for idx in 0..FIXTURE_ENTRIES {
        let tag = if idx == RENAMED_ENTRY {
            "old-name".to_string()
        } else {
            format!("tag-{idx}")
        };
        let separator = if idx + 1 == FIXTURE_ENTRIES { "" } else { "," };
        let _ = writeln!(body, "  {{\"id\": {idx}, \"tag\": \"{tag}\"}}{separator}");
    }
    body.push_str("]\n");
    body
}

/// Full tier: the whole post-image (with the rename applied) goes over
/// the wire as `proposedContent`.
fn full_content_request(fixture: &str) -> serde_json::Value {
    let post_image = fixture.replace("\"old-name\"", "\"new-name\"");
    tools_call(&serde_json::json!({
        "path": "meta/tags.json",
        "operation": "update",
        "proposedContent": post_image
    }))
}

/// Safelist tier: the same rename as a unified diff. The touched entry
/// sits at 1-based line `RENAMED_ENTRY + 2` (one for the opening `[`,
/// one for 1-based indexing).
fn patch_request() -> serde_json::Value {
    let line = RENAMED_ENTRY + 2;
    let patch = format!(
        "--- a/meta/tags.json\n+++ b/meta/tags.json\n@@ -{line} +{line} @@\n-  {{\"id\": {RENAMED_ENTRY}, \"tag\": \"old-name\"}},\n+  {{\"id\": {RENAMED_ENTRY}, \"tag\": \"new-name\"}},\n"
    );
    tools_call(&serde_json::json!({
        "path": "meta/tags.json",
        "operation": "update",
        "patch": patch
    }))
}

fn tools_call(arguments: &serde_json::Value) -> serde_json::Value {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": NEXT_ID.fetch_add(1, Ordering::Relaxed),
        "method": "tools/call",
        "params": { "name": "anvil_validate_write", "arguments": arguments }
    })
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

fn response_has_result(line: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .is_some_and(|v| v.get("result").is_some() && v.get("error").is_none())
}
