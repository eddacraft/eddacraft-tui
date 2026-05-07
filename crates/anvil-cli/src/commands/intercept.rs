//! `anvil intercept` — INTD-001 scaffold + INTD-011 status surface.
//!
//! Wires the shipped `anvil` binary's CLI surface to the intercept
//! daemon library. Today this implements `start --foreground` (INTD-001)
//! and `status` (INTD-011); backgrounded launch (`stop`, daemonised
//! `start`) arrives with later INTD tasks.

use anvil_intercept::{ForegroundOpts, Shutdown, run_foreground, wait_for_shutdown_signal};
use anvil_intercept_proto::status::DaemonStatusV1;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct InterceptArgs {
    #[command(subcommand)]
    command: InterceptCommand,
}

#[derive(Debug, Subcommand)]
enum InterceptCommand {
    /// Start the intercept daemon. Currently only `--foreground` is
    /// supported; backgrounded launch arrives with INTD-002.
    Start(StartArgs),
    /// Print the daemon's status snapshot — sessions, fences, and
    /// the mid-edit `validation.service` p50/p95 latency rollup
    /// (INTD-011).
    Status(StatusArgs),
}

#[derive(Debug, Args)]
struct StartArgs {
    /// Stay in the foreground; logs stream to stdout/stderr.
    /// Ctrl+C (and SIGTERM on Unix) stops the daemon cleanly. Demo
    /// runbook §4.1 fallback path.
    #[arg(long)]
    foreground: bool,
}

#[derive(Debug, Args)]
struct StatusArgs {
    /// Emit the raw JSON-RPC `query_status` result instead of the
    /// human-readable summary. Useful for scripting and CI capture.
    #[arg(long)]
    json: bool,
}

pub fn run(args: &InterceptArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        InterceptCommand::Start(start_args) => run_start(start_args),
        InterceptCommand::Status(status_args) => run_status(status_args),
    }
}

fn run_status(args: &StatusArgs) -> Result<()> {
    let snapshot = query_daemon_status()?;
    if args.json {
        let json = serde_json::to_string_pretty(&snapshot)
            .context("failed to serialise daemon status as JSON")?;
        println!("{json}");
    } else {
        print!("{}", render_status_lines(&snapshot));
    }
    Ok(())
}

/// Connect to the daemon over the per-user IPC socket / named pipe
/// and issue a JSON-RPC `query_status` request. Returns the parsed
/// proto wire shape; the caller decides whether to render it as
/// human text or JSON.
///
/// Errors carry an actionable message — "daemon is not running"
/// vs "daemon refused the request" — so an operator running this
/// during the demo §1.5 trust-signal step gets a single sentence
/// they can act on.
#[cfg(unix)]
fn query_daemon_status() -> Result<DaemonStatusV1> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

    let socket_path =
        ipc::resolve_socket_path().context("failed to resolve intercept daemon socket path")?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        // Same NotFound / ENOENT as the MCP path means the daemon
        // simply is not running; surface that as the actionable
        // single line rather than as a generic IO error.
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(anyhow::anyhow!(
                    "anvil intercept daemon is not running (no socket at {}). \
                     Start it with `anvil intercept start --foreground`.",
                    socket_path.display(),
                ))
            }
            other => Err(anyhow::anyhow!(
                "anvil intercept daemon socket is unavailable: {other}",
            )),
        };
    }
    let mut stream = UnixStream::connect(&socket_path).with_context(|| {
        format!(
            "failed to connect to intercept daemon socket {}",
            socket_path.display(),
        )
    })?;
    ipc::validate_connected_peer_for_client(&stream)
        .map_err(|err| anyhow::anyhow!("daemon peer credentials rejected: {err}"))?;
    stream
        .set_read_timeout(Some(REQUEST_TIMEOUT))
        .context("failed to configure read timeout")?;
    stream
        .set_write_timeout(Some(REQUEST_TIMEOUT))
        .context("failed to configure write timeout")?;

    // Unix path keeps the legacy `query_status` method name to avoid
    // changing on-the-wire behaviour for existing operator scripts;
    // the daemon dual-routes both names so this is a deliberate
    // continuity choice rather than a missed migration.
    let frame_bytes = build_query_status_frame_bytes(LEGACY_QUERY_STATUS_METHOD, REQUEST_ID);
    stream
        .write_all(&frame_bytes)
        .context("failed to send query_status frame")?;
    stream
        .flush()
        .context("failed to flush query_status frame")?;

    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut buf)
        .context("failed to read query_status response")?;
    parse_query_status_response_bytes(&buf, read)
}

#[cfg(windows)]
fn query_daemon_status() -> Result<DaemonStatusV1> {
    let pipe_name = anvil_intercept_win32::pipe_name_for_current_user()
        .context("failed to resolve intercept daemon pipe name")?;
    query_daemon_status_windows_at(&pipe_name)
}

/// Windows named-pipe equivalent of the Unix `query_daemon_status`.
///
/// The CLI runs outside any tokio runtime (top-level `run` is
/// synchronous), so the entire flow uses the synchronous Win32
/// helpers in `anvil-intercept-win32` rather than dragging in an
/// async runtime for one request.
///
/// Wire-format parity with the Unix path is enforced by reusing
/// the same `build_query_status_frame_bytes` /
/// `parse_query_status_response_bytes` helpers; the only daemon-
/// facing difference is the JSON-RPC method name. New consumers
/// (this PR is the first daemon-speaking Windows client) prefer the
/// canonical `anvil/status/query` form from
/// `anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY`; the daemon
/// dual-routes both names so the rendered output is identical.
///
/// `pipe_name` is parameterised (rather than always reading
/// `pipe_name_for_current_user()`) so the integration test can bind
/// a per-process pipe name and avoid colliding with the canonical
/// per-user pipe a real daemon would own on the same Windows runner.
#[cfg(windows)]
fn query_daemon_status_windows_at(pipe_name: &str) -> Result<DaemonStatusV1> {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY;

    // Mirror the Unix path's 2 s wall clock on the request. Synchronous
    // Win32 `ReadFile` on a named pipe has no native timeout setter
    // (`SetCommTimeouts` does not apply), so the CLI runs the IO on a
    // worker thread and gives up after `REQUEST_TIMEOUT`. A daemon that
    // accepts the connection but never writes leaves the worker
    // blocked, but the CLI is a single-shot process about to exit, so
    // a leaked blocked thread is bounded by process lifetime.
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
    // Catch on the connect side too: `WaitNamedPipe` blocks if all
    // server instances are busy.
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

    let pipe_name_owned = pipe_name.to_owned();
    let (connect_tx, connect_rx) = mpsc::sync_channel::<std::io::Result<_>>(1);
    let connect_thread = thread::spawn(move || {
        let _ = connect_tx.send(anvil_intercept_win32::connect_owner_only_pipe_client(
            &pipe_name_owned,
        ));
    });
    let connect_outcome = match connect_rx.recv_timeout(CONNECT_TIMEOUT) {
        Ok(outcome) => outcome,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            anyhow::bail!(
                "timed out connecting to intercept daemon pipe {pipe_name} \
                 (daemon may be busy or hung)",
            );
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            anyhow::bail!("connect worker exited unexpectedly");
        }
    };
    // The connect worker has produced a result and exited; reap it so
    // we don't leak a JoinHandle.
    let _ = connect_thread.join();

    let mut client = match connect_outcome {
        Ok(client) => client,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // ERROR_FILE_NOT_FOUND — the daemon has not bound the
            // per-user pipe. Match the Unix wording (and verb) so the
            // operator-facing trust signal in the demo runbook §1.5
            // reads the same on both platforms.
            anyhow::bail!(
                "anvil intercept daemon is not running (no pipe at {pipe_name}). \
                 Start it with `anvil intercept start --foreground`.",
            );
        }
        // ERROR_PIPE_BUSY — every server instance is currently
        // talking to another client. The daemon spawns a fresh
        // instance after each accept, so this is rare; surface a
        // distinct message rather than a generic IO error so the
        // operator can tell "daemon hung" from "daemon down".
        Err(err) if err.raw_os_error() == Some(231) => {
            anyhow::bail!("anvil intercept daemon pipe {pipe_name} is busy; retry shortly",);
        }
        Err(err) => {
            return Err(anyhow::Error::new(err).context(format!(
                "failed to connect to intercept daemon pipe {pipe_name}",
            )));
        }
    };

    let frame_bytes = build_query_status_frame_bytes(ANVIL_STATUS_QUERY, REQUEST_ID);
    client
        .write_all(&frame_bytes)
        .context("failed to send query_status frame")?;

    // Read up to the cap one chunk at a time, on a worker thread so
    // we can enforce the wall-clock timeout. Named pipes deliver a
    // single message in multiple ReadFile completions when the server
    // writes a response without setting MESSAGE mode, so the worker
    // accumulates until it sees a newline (the JSON-RPC framing
    // boundary), the response cap, or EOF.
    let (read_tx, read_rx) = mpsc::sync_channel::<Result<Vec<u8>>>(1);
    thread::spawn(move || {
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        let mut chunk = [0_u8; 4096];
        let outcome = loop {
            let n = match client.read(&mut chunk) {
                Ok(n) => n,
                Err(err) => {
                    break Err(
                        anyhow::Error::new(err).context("failed to read query_status response")
                    );
                }
            };
            if n == 0 {
                break Err(anyhow::anyhow!(
                    "daemon closed the connection before responding"
                ));
            }
            buf.extend_from_slice(&chunk[..n]);
            // Mirror the Unix `read_until(b'\n')` framing semantics:
            // return EXACTLY one line (up to and including the first
            // newline). If the daemon ever writes multiple lines or
            // extra bytes after the newline in a single pipe read,
            // truncating here is what lets the JSON parse succeed
            // on the same input the Unix path handles correctly.
            if let Some(newline_idx) = buf.iter().position(|b| *b == b'\n') {
                buf.truncate(newline_idx + 1);
                break Ok(buf);
            }
            if (buf.len() as u64) > RESPONSE_LINE_BYTES {
                break Err(anyhow::anyhow!(
                    "query_status response exceeded {RESPONSE_LINE_BYTES} byte cap"
                ));
            }
        };
        let _ = read_tx.send(outcome);
    });

    let buf = match read_rx.recv_timeout(REQUEST_TIMEOUT) {
        Ok(outcome) => outcome?,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            anyhow::bail!(
                "timed out waiting for daemon response on pipe {pipe_name} \
                 (daemon may be hung or under load)",
            );
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            anyhow::bail!("read worker exited unexpectedly");
        }
    };
    let read = buf.len();
    parse_query_status_response_bytes(&buf, read)
}

/// JSON-RPC method name pinned by INTD-011 + #1322 (B1). The daemon
/// dual-routes both `query_status` (legacy) and
/// `anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY`
/// (`anvil/status/query`) to the same handler. The Unix path keeps
/// the legacy name for continuity; the Windows path (this PR is its
/// first client) uses the canonical name new consumers should prefer.
#[cfg(unix)]
const LEGACY_QUERY_STATUS_METHOD: &str = "query_status";
const REQUEST_ID: &str = "anvil-cli-intercept-status";
/// 1 MiB cap on a single response line. The daemon's status snapshot
/// is well under this for any plausible session/fence count; we cap
/// to bound a misbehaving (or hostile) peer's memory pressure on the
/// CLI client.
const RESPONSE_LINE_BYTES: u64 = 1 << 20;

/// Build the on-the-wire bytes for a `query_status` JSON-RPC frame.
/// Centralised so the Unix and Windows paths cannot drift on
/// jsonrpc/version/id semantics.
fn build_query_status_frame_bytes(method: &str, id: &str) -> Vec<u8> {
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "id": id,
    });
    let mut out = frame.to_string().into_bytes();
    out.push(b'\n');
    out
}

/// Validate the framing + JSON-RPC envelope of one response line and
/// return the parsed `DaemonStatusV1`. `read == 0` means EOF before
/// any data; `buf.len() > RESPONSE_LINE_BYTES` means the daemon
/// exceeded the line cap. Errors carry the same wording the Unix
/// path used inline before this helper existed.
fn parse_query_status_response_bytes(buf: &[u8], read: usize) -> Result<DaemonStatusV1> {
    if read == 0 {
        anyhow::bail!("daemon closed the connection before responding");
    }
    if (buf.len() as u64) > RESPONSE_LINE_BYTES {
        anyhow::bail!("query_status response exceeded {RESPONSE_LINE_BYTES} byte cap");
    }
    let line = std::str::from_utf8(buf.trim_ascii_end())
        .context("query_status response is not valid UTF-8")?;
    let response: serde_json::Value =
        serde_json::from_str(line).context("query_status response is not valid JSON")?;
    // Match the daemon's JSON-RPC version pin so a server speaking a
    // newer envelope cannot ship subtly different semantics under the
    // same method name.
    if response.get("jsonrpc") != Some(&serde_json::Value::String("2.0".to_string())) {
        anyhow::bail!(
            "daemon response missing or wrong jsonrpc version (expected \"2.0\"): {response}",
        );
    }
    if response.get("id") != Some(&serde_json::Value::String(REQUEST_ID.to_string())) {
        anyhow::bail!(
            "daemon response id does not match request (expected {REQUEST_ID:?}): {response}",
        );
    }
    if let Some(error) = response.get("error") {
        anyhow::bail!("daemon returned JSON-RPC error: {error}");
    }
    let result = response
        .get("result")
        .cloned()
        .context("query_status response missing result")?;
    serde_json::from_value::<DaemonStatusV1>(result)
        .context("query_status response did not match the proto shape")
}

/// Render the daemon status snapshot in the operator-facing text
/// format the demo runbook §1.5 references. The latency line MUST
/// match `latency: p50 <X>ms p95 <Y>ms (mid-edit)` literally — see
/// `anvil_intercept::status::render_latency_line` for the contract
/// pin. Building on the proto wire shape (rather than the daemon's
/// in-memory `DaemonStatus`) means future driver consumers can reuse
/// this exact rendering against a daemon they did not link against.
fn render_status_lines(status: &DaemonStatusV1) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = writeln!(
        out,
        "daemon:    running (uptime {}s, version {})",
        status.health.uptime_seconds, status.health.version,
    );
    let active_session_count = status.sessions.len();
    let session_word = if active_session_count == 1 {
        "session"
    } else {
        "sessions"
    };
    let _ = writeln!(
        out,
        "sessions:  {active_session_count} active   ({session_word})",
    );
    let _ = writeln!(out, "fences:    {}", status.fences.len());
    out.push_str(&render_latency_line_for_wire(
        status.latency.mid_edit.as_ref(),
    ));
    out.push('\n');
    out
}

fn render_latency_line_for_wire(
    rollup: Option<&anvil_intercept_proto::status::LatencyRollupV1>,
) -> String {
    match rollup {
        Some(r) => format!(
            "latency: p50 {}ms p95 {}ms (mid-edit)",
            round_to_int(r.p50_ms),
            round_to_int(r.p95_ms),
        ),
        None => "latency: (no mid-edit traffic yet)".to_owned(),
    }
}

/// 2^53 — the largest integer that round-trips through `f64` exactly.
const SAFE_F64_INT_CAP: f64 = 9_007_199_254_740_992.0;

fn round_to_int(value: f64) -> u64 {
    if !value.is_finite() || value < 0.0 {
        return 0;
    }
    let rounded = value.round();
    if rounded >= SAFE_F64_INT_CAP {
        return u64::MAX;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        rounded as u64
    }
}

fn run_start(args: &StartArgs) -> Result<()> {
    if !args.foreground {
        anyhow::bail!(
            "`anvil intercept start` currently requires --foreground; backgrounded launch lands with INTD-002"
        );
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let (shutdown, token) = Shutdown::new();
        let signal_shutdown = shutdown.clone();
        tokio::spawn(async move {
            // Trigger shutdown on either a signal arriving or a
            // signal-handler installation failure. Swallowing the Err
            // would leave the daemon hanging in restricted /
            // containerised environments where signal install fails;
            // the operator-visible diagnostic plus a triggered
            // shutdown lets the foreground loop unwind cleanly.
            if let Err(err) = wait_for_shutdown_signal().await {
                let mut message = String::from("anvil intercept: shutdown signal handler failed: ");
                message.push_str(&err.to_string());
                message.push('\n');
                let mut stderr = std::io::stderr().lock();
                let _ = std::io::Write::write_all(&mut stderr, message.as_bytes());
            }
            signal_shutdown.trigger();
        });
        run_foreground(ForegroundOpts::default(), token).await
    })
}

#[cfg(test)]
mod tests {
    use anvil_intercept_proto::status::{
        DaemonStatusV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1, LatencyRollupV1,
    };

    use super::*;

    /// Pin the contract: `anvil intercept start` without `--foreground`
    /// exits with a clear error directing the user at the missing
    /// flag, not a silent no-op or a confusing panic. Future flag
    /// changes that drop the bail (e.g. flipping the default to
    /// foreground) must update this test.
    #[test]
    fn run_start_without_foreground_bails_with_actionable_message() {
        let args = StartArgs { foreground: false };
        let err = run_start(&args).expect_err("expected bail when --foreground omitted");
        let msg = format!("{err}");
        assert!(
            msg.contains("--foreground"),
            "bail message must mention --foreground, got: {msg}",
        );
        assert!(
            msg.contains("INTD-002"),
            "bail message must point at the future backgrounded path (INTD-002), got: {msg}",
        );
    }

    fn empty_status() -> DaemonStatusV1 {
        DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 12,
                version: "0.5.1-beta".into(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
        }
    }

    /// **Contract pin (demo runbook §1.5):** with traffic the line
    /// MUST read exactly `latency: p50 <X>ms p95 <Y>ms (mid-edit)`.
    /// Any change to this string MUST update the runbook in the
    /// same commit.
    #[test]
    fn cli_renders_runbook_pinned_latency_line_with_traffic() {
        let mut status = empty_status();
        status.latency.mid_edit = Some(LatencyRollupV1 {
            p50_ms: 11.4,
            p95_ms: 47.6,
            sample_count: 12,
            window_seconds: 22.0,
        });
        let rendered = render_status_lines(&status);
        assert!(
            rendered.contains("latency: p50 11ms p95 48ms (mid-edit)"),
            "renderer must match the runbook pin; got:\n{rendered}",
        );
    }

    /// **Contract pin (INTD-011 hard rule):** without traffic the
    /// line MUST read `latency: (no mid-edit traffic yet)`. We do
    /// NOT print `0ms / 0ms` — reality, not assumed readiness.
    #[test]
    fn cli_renders_no_traffic_message_when_aggregator_is_empty() {
        let rendered = render_status_lines(&empty_status());
        assert!(
            rendered.contains("latency: (no mid-edit traffic yet)"),
            "no-traffic case must use the honest message; got:\n{rendered}",
        );
        assert!(
            !rendered.contains("p50 0ms"),
            "no-traffic case must not print 0ms; got:\n{rendered}",
        );
    }

    #[test]
    fn cli_renders_session_pluralisation_correctly() {
        let mut status = empty_status();
        status.sessions = vec![anvil_intercept_proto::SessionRecord {
            id: anvil_intercept_proto::SessionId::new("sess-1"),
            worktree: std::path::PathBuf::from("/tmp/wt"),
            pid: Some(1),
            pgid: Some(1),
            started_at_unix: 0,
            last_heartbeat_unix: 0,
            status: anvil_intercept_proto::SessionStatus::Active,
        }];
        let rendered = render_status_lines(&status);
        assert!(rendered.contains("sessions:  1 active"));
    }

    #[test]
    fn round_to_int_handles_edge_cases() {
        assert_eq!(round_to_int(0.0), 0);
        assert_eq!(round_to_int(0.4), 0);
        assert_eq!(round_to_int(0.5), 1);
        assert_eq!(round_to_int(-1.0), 0);
        assert_eq!(round_to_int(f64::NAN), 0);
        assert_eq!(round_to_int(f64::INFINITY), 0);
    }

    /// Pin the wire-format helpers shared across Unix and Windows.
    /// A frame is one JSON line terminated by `\n`; if the trailing
    /// newline ever vanishes the daemon's per-line framing breaks
    /// silently.
    #[test]
    fn build_query_status_frame_bytes_emits_jsonrpc_2_envelope() {
        let bytes = super::build_query_status_frame_bytes("anvil/status/query", "id-1");
        let s = std::str::from_utf8(&bytes).expect("utf8 frame");
        assert!(s.ends_with('\n'), "frame must be newline-terminated: {s:?}");
        assert!(s.contains("\"jsonrpc\":\"2.0\""), "got {s}");
        assert!(s.contains("\"method\":\"anvil/status/query\""), "got {s}");
        assert!(s.contains("\"id\":\"id-1\""), "got {s}");
    }

    /// Pin: response parser rejects the reply if the daemon answers
    /// with the wrong request id. This is the same fail-closed check
    /// the Linux MCP path applies in
    /// `local_daemon_client_rejects_mismatched_jsonrpc_response_id`;
    /// keeping it on the status path means a stale or stitched-from-
    /// somewhere-else response cannot be rendered as fresh status.
    #[test]
    fn parse_query_status_response_bytes_rejects_mismatched_id() {
        // `REQUEST_ID` is `anvil-cli-intercept-status`; this fixture
        // deliberately answers with the wrong id.
        let line = br#"{"jsonrpc":"2.0","id":"some-other-id","result":null}
"#;
        let err = super::parse_query_status_response_bytes(line, line.len())
            .expect_err("mismatched id must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("id does not match"),
            "expected id-mismatch wording, got: {msg}",
        );
    }

    /// **Windows-only integration test (INTD-011 §1.5 parity):** spin
    /// up the daemon's named-pipe `IpcListener` in-process and call
    /// the synchronous CLI client against it. Mirrors the Linux
    /// pattern at
    /// `crates/anvil-cli/src/mcp/validation.rs::local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`.
    ///
    /// The runbook §1.5 latency-line wording is asserted on the
    /// rendered text path (the same surface an operator sees during
    /// the demo trust-signal step), so a future change that desyncs
    /// the Windows status renderer from the Unix one fails closed.
    ///
    /// The pipe name is per-PID (rather than the canonical
    /// `pipe_name_for_current_user()` value) so the test never
    /// collides with a real daemon that might be bound on the same
    /// Windows runner, and so concurrent test crates do not race
    /// each other on the singleton-claiming first instance.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_query_daemon_status_round_trips_against_local_pipe() {
        use std::sync::Arc;

        use anvil_intercept::Shutdown;
        use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
        use anvil_intercept::status::{DaemonStatus, StatusProvider};

        struct Fixture;
        impl StatusProvider for Fixture {
            fn query_status(&self) -> DaemonStatus {
                anvil_intercept::status::build_status(
                    Vec::new(),
                    &[],
                    None,
                    std::time::Instant::now(),
                    std::time::Instant::now(),
                    "0.0.0-windows-test",
                    anvil_intercept::status::IpcState::Serving,
                )
            }
        }

        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-cli-status-test-{}",
            std::process::id(),
        );
        // Use a multi-thread runtime so a worker thread drives the
        // server task while the main thread runs the synchronous
        // client. A `current_thread` runtime + `runtime.enter()` would
        // never poll the spawned server because the only thread that
        // could poll it is blocked on the client call below, leading
        // to a deadlock on Windows.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let listener = IpcListener::bind(&pipe_name, NoopDispatcher)
            .expect("daemon pipe binds")
            .with_status_provider(Arc::new(Fixture));
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        let snapshot =
            super::query_daemon_status_windows_at(&pipe_name).expect("status query succeeds");
        assert_eq!(snapshot.health.version, "0.0.0-windows-test");

        // Render the snapshot and assert the runbook §1.5 latency
        // wording for the no-traffic case (the fixture provides no
        // mid-edit samples). Verifying this here — at the
        // platform-specific surface — is the load-bearing parity
        // claim with the Unix `cli_renders_no_traffic_message_when_aggregator_is_empty`
        // test.
        let rendered = super::render_status_lines(&snapshot);
        assert!(
            rendered.contains("latency: (no mid-edit traffic yet)"),
            "no-traffic line must be honoured on Windows; got:\n{rendered}",
        );

        shutdown.trigger();
        runtime.block_on(async {
            server
                .await
                .expect("daemon task joins")
                .expect("daemon exits cleanly");
        });
    }

    /// **Windows-only sanity:** with no daemon bound, the CLI emits
    /// the same actionable "daemon is not running" wording the Unix
    /// path uses, so operator-facing diagnostics are platform-
    /// neutral.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_query_daemon_status_says_daemon_is_not_running_when_pipe_absent() {
        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-cli-status-missing-{}",
            std::process::id(),
        );
        let err = super::query_daemon_status_windows_at(&pipe_name)
            .expect_err("missing pipe must surface as actionable error");
        let msg = format!("{err}");
        assert!(
            msg.contains("daemon is not running"),
            "wording must match Unix path; got: {msg}",
        );
        assert!(
            msg.contains("anvil intercept start --foreground"),
            "must point operator at the recovery command; got: {msg}",
        );
    }
}
