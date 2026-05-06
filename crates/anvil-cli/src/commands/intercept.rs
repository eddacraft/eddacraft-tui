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

    const REQUEST_ID: &str = "anvil-cli-intercept-status";
    const RESPONSE_LINE_BYTES: u64 = 1 << 20;
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

    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "query_status",
        "id": REQUEST_ID,
    });
    writeln!(stream, "{frame}").context("failed to send query_status frame")?;
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

#[cfg(not(unix))]
fn query_daemon_status() -> Result<DaemonStatusV1> {
    anyhow::bail!(
        "`anvil intercept status` over the daemon IPC is only supported on Unix in this build; \
         Windows named-pipe client support lands with the DRVR-001 client port",
    )
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
}
