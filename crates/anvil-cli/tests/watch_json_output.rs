//! WOUT-004: end-to-end consumer pipe harness for `anvil --json watch`.
//!
//! These tests spawn the real `anvil` binary against a fixture workspace,
//! collect stdout as NDJSON, and assert the v1 wire envelope holds in
//! practice — not just in isolated serde tests. They are the only place
//! that proves a piped consumer can read the stream end-to-end:
//!
//! 1. `watch_json_emits_initial_progress_and_snapshot` — bare watch path,
//!    initial scan produces parseable envelopes.
//! 2. `watch_json_stdout_carries_only_ndjson_when_bare_exclude_warning_present`
//!    — exercises the WOUT-003 stderr routing rule against the real binary
//!    by triggering a `--exclude <bare>` warning and asserting stdout
//!    stays clean.
//!
//! Unix-only: file-system event semantics differ on Windows
//! (`notify` Drop/Read behaviour, debounce timing) and the spec's primary
//! consumer surface is shell pipelines and CI hooks on Unix. The
//! WOUT-002 unit tests cover the cross-platform serde contract; this
//! file proves the runtime path on the platform that matters for piping.

#![cfg(not(target_os = "windows"))]

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anvil_kernel_types::{WatchEventEnvelope, WatchEventType};

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

/// Maximum wall-clock budget for waiting on the initial snapshot line.
/// 5 seconds is generous for CI; the initial scan over a one-file
/// workspace finishes in tens of milliseconds locally.
const SNAPSHOT_WAIT_BUDGET: Duration = Duration::from_secs(5);

struct WatchProcess {
    child: Child,
    rx: mpsc::Receiver<String>,
    reader: Option<thread::JoinHandle<()>>,
}

impl WatchProcess {
    fn spawn(workdir: &Path, home: &Path, extra_args: &[&str]) -> Self {
        let mut cmd = Command::new(ANVIL_BIN);
        cmd.arg("--no-tui")
            .arg("--json")
            .arg("watch")
            .arg("--debounce")
            .arg("50");
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.current_dir(workdir)
            .env("HOME", home)
            .env("USERPROFILE", home)
            .env_remove("XDG_CONFIG_HOME")
            .env("ANVIL_DEV", "1")
            .env("ANVIL_SKIP_WELCOME", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().expect("spawn anvil --json watch");
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, rx) = mpsc::channel();

        // Pump stdout lines into a channel so the test thread can wait
        // on them with a timeout without blocking on `read_line`.
        let reader = thread::spawn(move || {
            let buf = BufReader::new(stdout);
            for line in buf.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            child,
            rx,
            reader: Some(reader),
        }
    }

    /// Block until a line matching `predicate` arrives or the budget
    /// expires. Returns every collected line (including the matching
    /// one) so failure messages can show the full prefix.
    ///
    /// Also polls `child.try_wait()` periodically: if the spawned
    /// `anvil` process exits before the predicate fires (typically
    /// because of a startup failure on a misconfigured host), this
    /// returns whatever it has so the caller can fail fast with the
    /// captured stderr rather than waiting the full wall-clock budget.
    fn collect_until<F>(&mut self, budget: Duration, mut predicate: F) -> Vec<String>
    where
        F: FnMut(&str) -> bool,
    {
        let deadline = Instant::now() + budget;
        let mut collected = Vec::new();
        let poll_step = Duration::from_millis(50);
        while Instant::now() < deadline {
            // Detect early child exit so we don't burn the full budget
            // on a dead process.
            if let Ok(Some(_status)) = self.child.try_wait() {
                // Drain any final lines that landed in the channel.
                while let Ok(line) = self.rx.try_recv() {
                    collected.push(line);
                }
                return collected;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            let wait = remaining.min(poll_step);
            match self.rx.recv_timeout(wait) {
                Ok(line) => {
                    let matched = predicate(&line);
                    collected.push(line);
                    if matched {
                        return collected;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        collected
    }

    fn shutdown(mut self) -> Vec<u8> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        // Drain stderr after the child is dead so reads always return.
        let mut stderr_bytes = Vec::new();
        if let Some(mut stderr) = self.child.stderr.take() {
            use std::io::Read;
            let _ = stderr.read_to_end(&mut stderr_bytes);
        }
        stderr_bytes
    }
}

fn seed_workspace(dir: &Path) {
    // A single TS file is enough for the kernel's initial scan to
    // produce a Snapshot event. The file's content is intentionally
    // trivial — the test cares about events being emitted, not about
    // what the parser sees inside.
    std::fs::write(dir.join("seed.ts"), "export const seed: number = 1;\n")
        .expect("seed workspace");
}

#[test]
fn watch_json_emits_initial_progress_and_snapshot() {
    let workdir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home tempdir");
    seed_workspace(workdir.path());

    let mut proc = WatchProcess::spawn(workdir.path(), home.path(), &[]);
    let lines = proc.collect_until(SNAPSHOT_WAIT_BUDGET, |line| {
        // Stop as soon as the first snapshot line appears — that proves
        // the initial scan ran and shipped a typed envelope to stdout.
        line.contains("\"event_type\":\"snapshot\"")
    });
    let stderr = proc.shutdown();
    let stderr_text = String::from_utf8_lossy(&stderr);

    let envelopes = parse_envelopes(&lines, &stderr_text);

    assert!(
        envelopes
            .iter()
            .any(|e| e.event_type == WatchEventType::Snapshot),
        "no snapshot envelope arrived within {SNAPSHOT_WAIT_BUDGET:?}; \
         stdout lines={lines:?} stderr={stderr_text}"
    );

    // Every emitted line MUST be a v1 envelope — no banners or
    // human-readable text leaks through.
    for line in &lines {
        assert!(
            line.starts_with('{'),
            "stdout line must be a JSON object: {line:?}"
        );
        assert!(
            line.contains("\"schema_version\":\"anvil.watch.event.v1\""),
            "stdout line missing v1 schema_version: {line:?}"
        );
    }

    // Sequence numbers must be unique within the captured stream. The
    // v1 spec promises monotonic-per-process seq values that consumers
    // can use to detect *drops or reordering*, not strict ordering —
    // the kernel emits from a thread-pool worker via `fetch_add`+
    // channel send, so two events with consecutive seqs can arrive at
    // the consumer in either order. Asserting strict `seq > prev` over-
    // promises the contract and is brittle on loaded CI workers. The
    // useful invariant is uniqueness: no two events share a seq.
    let mut seen = std::collections::HashSet::new();
    for env in &envelopes {
        assert!(
            seen.insert(env.seq),
            "seq {} appeared twice in the same process stream: {envelopes:?}",
            env.seq
        );
    }
}

#[test]
fn watch_json_stdout_carries_only_ndjson_when_bare_exclude_warning_present() {
    // `--exclude vendor` (without `/**`) triggers the WOUT-003 bare-name
    // warning. In JSON mode the warning MUST route to stderr; stdout
    // must remain pure NDJSON. This is the exact regression the
    // hand-extracted policy in watch.rs is supposed to prevent.
    let workdir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home tempdir");
    seed_workspace(workdir.path());

    let mut proc = WatchProcess::spawn(workdir.path(), home.path(), &["--exclude", "vendor"]);
    let lines = proc.collect_until(SNAPSHOT_WAIT_BUDGET, |line| {
        line.contains("\"event_type\":\"snapshot\"")
    });
    let stderr = proc.shutdown();
    let stderr_text = String::from_utf8_lossy(&stderr);

    // The warning landed on stderr, not stdout. We avoid asserting
    // against the literal English wording — the test should fail when
    // routing breaks, not when the help text gets reworded. What MUST
    // stay stable is the information content: the original pattern
    // (`vendor`) and the corrected glob (`vendor/**`) both appear on
    // stderr, alongside a "warn" indicator.
    assert!(
        stderr_text.to_ascii_lowercase().contains("warn"),
        "expected a 'warn' indicator on stderr; stderr={stderr_text}"
    );
    assert!(
        stderr_text.contains("vendor"),
        "expected the offending pattern on stderr; stderr={stderr_text}"
    );
    assert!(
        stderr_text.contains("vendor/**"),
        "expected the corrected glob on stderr; stderr={stderr_text}"
    );
    for line in &lines {
        let lc = line.to_ascii_lowercase();
        assert!(
            !lc.contains("[warn]"),
            "stdout must NOT contain the [warn] banner in JSON mode: {line:?}"
        );
        assert!(
            !lc.contains("[watching]"),
            "stdout must NOT contain the [watching] banner in JSON mode: {line:?}"
        );
    }

    // CLAWP-015: prove the test exercised the actual contract. Without
    // this, `collect_until` could time out before any stdout event
    // arrived (or the watcher could regress and emit nothing at all),
    // the for-loop above would pass vacuously on an empty `lines`, and
    // the stderr-routing assertions would mask the failure. Assert the
    // snapshot envelope is present — same shape as the sibling
    // `watch_json_emits_initial_progress_and_snapshot` test.
    let envelopes = parse_envelopes(&lines, &stderr_text);
    assert!(
        envelopes
            .iter()
            .any(|e| e.event_type == WatchEventType::Snapshot),
        "no snapshot envelope arrived within {SNAPSHOT_WAIT_BUDGET:?}; \
         stdout lines={lines:?} stderr={stderr_text}"
    );
}

// --- WOUT-005: golden fixture drift guard ---

const FIXTURE_DIR: &str = "tests/fixtures/watch-json";

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_DIR)
        .join(name)
}

fn read_fixture_line(name: &str) -> String {
    let path = fixture_path(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} unreadable: {err}", path.display()));
    let line = text.lines().next().unwrap_or_else(|| {
        panic!(
            "fixture {} is empty; v1 NDJSON fixtures must contain one event line",
            path.display()
        )
    });
    line.to_string()
}

/// Required envelope fields per the v1 spec. Drift here is breaking.
const REQUIRED_ENVELOPE_FIELDS: &[(&str, &str)] = &[
    ("schema_version", "string"),
    ("seq", "u64"),
    ("timestamp", "string"),
    ("event_type", "string"),
    ("payload", "object"),
];

fn assert_envelope_required_fields(value: &serde_json::Value, fixture: &str) {
    for (field, kind) in REQUIRED_ENVELOPE_FIELDS {
        let got = value
            .get(*field)
            .unwrap_or_else(|| panic!("{fixture}: envelope missing required field {field:?}"));
        let ok = match *kind {
            "string" => got.is_string(),
            "u64" => got.is_u64(),
            "object" => got.is_object(),
            _ => unreachable!(),
        };
        assert!(
            ok,
            "{fixture}: envelope field {field:?} expected to be {kind}, got {got:?}"
        );
    }
    assert_eq!(
        value["schema_version"], "anvil.watch.event.v1",
        "{fixture}: schema_version must pin to anvil.watch.event.v1"
    );
}

fn assert_payload_required_fields(payload: &serde_json::Value, fields: &[&str], fixture: &str) {
    for f in fields {
        assert!(
            payload.get(*f).is_some(),
            "{fixture}: payload missing required field {f:?} (drift from v1 spec)"
        );
    }
}

#[test]
fn fixture_progress_envelope_has_v1_required_fields() {
    let line = read_fixture_line("progress.jsonl");
    let value: serde_json::Value = serde_json::from_str(&line).expect("progress fixture parses");
    assert_envelope_required_fields(&value, "progress.jsonl");
    assert_eq!(value["event_type"], "progress");
    assert_payload_required_fields(
        &value["payload"],
        &["phase", "current", "total"],
        "progress.jsonl",
    );
    // The fixture must also round-trip through the typed envelope.
    let _: WatchEventEnvelope = serde_json::from_str(&line).expect("progress fixture round-trips");
}

#[test]
fn fixture_snapshot_envelope_has_v1_required_fields() {
    let line = read_fixture_line("snapshot.jsonl");
    let value: serde_json::Value = serde_json::from_str(&line).expect("snapshot fixture parses");
    assert_envelope_required_fields(&value, "snapshot.jsonl");
    assert_eq!(value["event_type"], "snapshot");
    assert_payload_required_fields(
        &value["payload"],
        &["node_count", "edge_count", "files_watched"],
        "snapshot.jsonl",
    );
    let _: WatchEventEnvelope = serde_json::from_str(&line).expect("snapshot fixture round-trips");
}

#[test]
fn fixture_violation_envelope_has_v1_required_fields() {
    let line = read_fixture_line("violation.jsonl");
    let value: serde_json::Value = serde_json::from_str(&line).expect("violation fixture parses");
    assert_envelope_required_fields(&value, "violation.jsonl");
    assert_eq!(value["event_type"], "violation");
    assert_payload_required_fields(
        &value["payload"],
        &["policy_id", "file", "symbol", "message"],
        "violation.jsonl",
    );
    let _: WatchEventEnvelope = serde_json::from_str(&line).expect("violation fixture round-trips");
}

#[test]
fn fixture_error_envelope_has_v1_required_fields() {
    let line = read_fixture_line("error.jsonl");
    let value: serde_json::Value = serde_json::from_str(&line).expect("error fixture parses");
    assert_envelope_required_fields(&value, "error.jsonl");
    assert_eq!(value["event_type"], "error");
    assert_payload_required_fields(
        &value["payload"],
        &["code", "message", "recoverable"],
        "error.jsonl",
    );
    // `file` is optional and present here; the engine-wide variant omits it.
    assert!(value["payload"]["file"].is_string());
    let _: WatchEventEnvelope = serde_json::from_str(&line).expect("error fixture round-trips");
}

#[test]
fn fixture_engine_wide_error_envelope_omits_file_field() {
    let line = read_fixture_line("error-engine-wide.jsonl");
    let value: serde_json::Value = serde_json::from_str(&line).expect("engine-wide error parses");
    assert_envelope_required_fields(&value, "error-engine-wide.jsonl");
    assert_eq!(value["event_type"], "error");
    assert_payload_required_fields(
        &value["payload"],
        &["code", "message", "recoverable"],
        "error-engine-wide.jsonl",
    );
    assert!(
        value["payload"].get("file").is_none(),
        "engine-wide error fixture must omit the optional `file` field"
    );
    let _: WatchEventEnvelope =
        serde_json::from_str(&line).expect("engine-wide error fixture round-trips");
}

/// Extract every fenced `json` block from a markdown document, parse
/// each block as `serde_json::Value`, and return them in document order.
/// Used by `public_docs_examples_match_fixtures` to compare documented
/// examples against fixtures semantically — `oxfmt` pretty-prints fenced
/// JSON, so a byte-string match against single-line fixtures would
/// break on every doc reflow.
fn extract_json_blocks(markdown: &str) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let mut iter = markdown.lines().peekable();
    let mut block_idx = 0usize;
    while let Some(line) = iter.next() {
        if line.trim_start().starts_with("```json") {
            block_idx += 1;
            let mut buf = String::new();
            for inner in iter.by_ref() {
                if inner.trim_start().starts_with("```") {
                    break;
                }
                buf.push_str(inner);
                buf.push('\n');
            }
            // CLAWP-048: fail loud on an unparseable fenced `json` block.
            // The prior `if let Ok(..)` silently dropped a block that no
            // longer parsed, so a broken copy-paste example in the public
            // docs would just vanish from the comparison set and the test
            // would still pass — exactly the regression this guards.
            let value = serde_json::from_str::<serde_json::Value>(&buf).unwrap_or_else(|err| {
                panic!(
                    "fenced ```json block #{block_idx} in the consumer doc failed to parse: {err}\n\
                     block content:\n{buf}"
                )
            });
            out.push(value);
        }
    }
    out
}

#[test]
fn public_docs_examples_match_fixtures() {
    // The consumer docs in docs/public/anvil/integrations/watch-output.md
    // ship copy-pasteable NDJSON examples. They must stay *semantically*
    // identical to the fixtures so a `jq` example a reader copies still
    // parses against the same shape the binary emits. We do not assert
    // byte-equality because `oxfmt` pretty-prints fenced JSON in
    // markdown, which is a deliberate house-style choice — what matters
    // is the parsed shape, not the whitespace.
    let docs_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("public")
        .join("anvil")
        .join("integrations")
        .join("watch-output.md");
    let docs = std::fs::read_to_string(&docs_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", docs_path.display()));
    let documented_examples = extract_json_blocks(&docs);

    // `error-engine-wide.jsonl` is intentionally not checked against the
    // public docs: the docs show the common `error` shape with a `file`
    // anchor; the engine-wide variant exists only to pin the contract's
    // "file is optional" branch in the fixture suite. If the consumer
    // guide grows an explicit no-`file` example, add the fixture here.
    for fixture in [
        "progress.jsonl",
        "snapshot.jsonl",
        "violation.jsonl",
        "error.jsonl",
    ] {
        let fixture_line = read_fixture_line(fixture);
        let fixture_value: serde_json::Value =
            serde_json::from_str(&fixture_line).expect("fixture parses");
        assert!(
            documented_examples.contains(&fixture_value),
            "public consumer docs missing example with the same shape as {fixture}:\n\
             fixture parsed: {fixture_value}\n\
             documented examples: {documented_examples:#?}",
        );
    }
}

fn parse_envelopes(lines: &[String], stderr: &str) -> Vec<WatchEventEnvelope> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let env: WatchEventEnvelope = serde_json::from_str(line).unwrap_or_else(|err| {
            panic!(
                "stdout line failed to parse as v1 envelope: {err}\n\
                 line={line:?}\n\
                 all-stdout={lines:?}\n\
                 stderr={stderr}"
            )
        });
        assert_eq!(
            env.schema_version, "anvil.watch.event.v1",
            "envelope schema_version must pin to v1: {env:?}"
        );
        out.push(env);
    }
    out
}
