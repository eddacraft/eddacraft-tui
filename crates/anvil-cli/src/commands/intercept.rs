//! `anvil intercept` — INTD-001 scaffold + INTD-011 status surface.
//!
//! Wires the shipped `anvil` binary's CLI surface to the intercept
//! daemon library. Today this implements `start --foreground` (INTD-001),
//! `status` (INTD-011), `unblock` (RCLI3-017b), and `stop` (V060F-002 /
//! ACTMO-008 — stop the daemon recorded in the PID file); a daemonised (non-foreground)
//! `start` arrives with later INTD tasks.

use anvil_intercept::{ForegroundOpts, Shutdown, config, run_foreground, wait_for_shutdown_signal};
use anvil_intercept_proto::status::DaemonStatusV1;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct InterceptArgs {
    #[command(subcommand)]
    command: InterceptCommand,
}

impl InterceptArgs {
    /// USAGE-004: true when the generic CLI-side `intercept` usage row
    /// should be suppressed because the daemon will record the
    /// authoritative `command.invoked` row for this action instead
    /// (founder decision 2026-06-18: the daemon row is the single source
    /// of truth for `unblock-*`, avoiding a double-count).
    ///
    /// Scoped to a **non-dry-run** `unblock`: a `--dry-run` unblock
    /// (`--worktree --dry-run` / `--all --dry-run`) previews without
    /// contacting the daemon, so it emits no daemon row — suppressing the
    /// CLI row there would drop the invocation entirely (Council). Every
    /// non-dry-run unblock mode (cascade / per-fence / all) dispatches a
    /// daemon `unblock-*` verb, so the daemon row covers it.
    #[must_use]
    pub fn suppresses_cli_usage_row(&self) -> bool {
        matches!(&self.command, InterceptCommand::Unblock(args) if !args.dry_run)
    }
}

#[derive(Debug, Subcommand)]
enum InterceptCommand {
    /// Start the intercept daemon. Use `--foreground` to keep it in
    /// the terminal; logs stream to stdout/stderr.
    Start(StartArgs),
    /// Print the daemon's status snapshot — sessions, fences, and
    /// the mid-edit validation-service latency rollup.
    Status(StatusArgs),
    /// Clear fence state from the daemon. Two distinct modes:
    ///
    /// 1. **Per-fence:** `--worktree <PATH>` removes a single fenced
    ///    worktree from in-memory state and disk persistence. `--all`
    ///    clears every fence. `--dry-run` previews without modifying
    ///    state. Idempotent.
    ///
    /// 2. **Cascade:** positional `<WORKTREE>` plus
    ///    `--acknowledge-cascade` clears a worktree's
    ///    `degraded:fence-cascade` engaged state. The two modes
    ///    target different daemon state and do NOT overlap.
    Unblock(UnblockArgs),
    /// Stop the per-user intercept daemon recorded in the daemon PID file.
    /// Unix sends SIGTERM so the daemon can flush fence state and unbind its
    /// IPC listener; Windows terminates the headless daemon process and clears
    /// the PID file. Idempotent — exits zero when no daemon is running.
    Stop,
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

#[derive(Debug, Args)]
struct UnblockArgs {
    /// Legacy positional worktree path for cascade clearing
    /// (`anvil intercept unblock <WORKTREE> --acknowledge-cascade`).
    /// Prefer the `--worktree` flag for new invocations.
    #[arg(value_name = "WORKTREE", conflicts_with_all = ["worktree", "all"])]
    worktree_arg: Option<std::path::PathBuf>,
    /// Remove a single worktree's fence record from the daemon's
    /// in-memory state and disk persistence. Idempotent — re-running
    /// on an unfenced worktree exits zero with an informational note.
    #[arg(long = "worktree", value_name = "PATH",
          conflicts_with_all = ["worktree_arg", "all", "acknowledge_cascade"])]
    worktree: Option<std::path::PathBuf>,
    /// Clear every fenced worktree in one call. Cannot be combined
    /// with `--acknowledge-cascade` (cascade state is a distinct
    /// concern and must be cleared per-worktree).
    #[arg(long, conflicts_with_all = ["worktree_arg", "worktree", "acknowledge_cascade"])]
    all: bool,
    /// Print what would be cleared without modifying daemon state.
    /// Honoured for both `--worktree` and `--all`.
    #[arg(long)]
    dry_run: bool,
    /// Confirm intent to clear a worktree's `degraded:fence-cascade`
    /// engaged state. Required when using the positional worktree
    /// path form.
    #[arg(long)]
    acknowledge_cascade: bool,
}

pub fn run(args: &InterceptArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        InterceptCommand::Start(start_args) => run_start(start_args),
        InterceptCommand::Status(status_args) => run_status(status_args),
        InterceptCommand::Unblock(unblock_args) => run_unblock(unblock_args),
        InterceptCommand::Stop => run_stop(),
    }
}

/// V060F-002 / ACTMO-008: stop the per-user intercept daemon. Delegates to the
/// `anvil_intercept` lookup-and-stop primitive (which owns the platform
/// signalling/termination and PID-file semantics) and renders the outcome.
/// Idempotent: a missing or stale PID file exits zero with an
/// informational line, matching the `unblock` no-op convention.
#[cfg(any(unix, windows))]
fn run_stop() -> Result<()> {
    use anvil_intercept::StopOutcome;

    // ACTMO-017: best-effort query the registered set BEFORE stopping, so we
    // can warn how many worktrees are about to lose protection. A daemon that
    // is already down (or unreachable) reports nothing, which is correct.
    let registered = query_daemon_status().map_or(0, |status| status.registered_worktrees().len());

    match anvil_intercept::request_daemon_stop()? {
        StopOutcome::Signalled { pid } => {
            println!("{}", stop_success_line(pid));
            if registered > 0 {
                println!(
                    "  {registered} worktree(s) registered will lose protection — \
                     re-register with `anvil workspace register` or run `anvil start`.",
                );
            }
        }
        StopOutcome::NotRunning => {
            println!("anvil intercept daemon is not running (no PID file)");
        }
        StopOutcome::StaleCleared { pid } => println!(
            "anvil intercept daemon is not running; cleared a stale PID file (recorded pid {pid})",
        ),
    }
    Ok(())
}

#[cfg(unix)]
fn stop_success_line(pid: u32) -> String {
    format!(
        "sent SIGTERM to the anvil intercept daemon (pid {pid}); it will flush fence state and exit"
    )
}

#[cfg(windows)]
fn stop_success_line(pid: u32) -> String {
    format!("stopped the anvil intercept daemon (pid {pid}); cleared the PID file")
}

#[cfg(not(any(unix, windows)))]
fn run_stop() -> Result<()> {
    anyhow::bail!(
        "`anvil intercept stop` is not supported on this platform yet. Stop a foreground daemon \
         with Ctrl+C.",
    )
}

fn run_unblock(args: &UnblockArgs) -> Result<()> {
    let mode = resolve_unblock_mode(args)?;
    let result = match &mode {
        UnblockMode::Cascade(path) => run_unblock_cascade(path),
        UnblockMode::PerFence(path) => run_unblock_per_fence(path, args.dry_run),
        UnblockMode::AllFences => run_unblock_all(args.dry_run),
    };

    // 094d: a non-dry-run unblock has its CLI `command.invoked` row
    // suppressed in `main` (`suppresses_cli_usage_row`) on the assumption
    // that the daemon dispatch records the authoritative row instead. When
    // the daemon is DOWN (or otherwise unreachable), the dispatch fails
    // before any daemon row is written — so the operator action would be
    // recorded nowhere at all and become invisible to the usage views. On
    // that failure path we emit the CLI-side row as a fallback so a
    // daemon-down unblock still produces exactly one usage row. The
    // happy-path (daemon answered) keeps the single daemon row — this
    // fallback only fires when the dispatch errored, so there is no
    // double-count. Dry-run never reaches here (its CLI row is not
    // suppressed). Strictly best-effort: a usage-write failure is logged
    // and dropped, never masking the underlying dispatch error.
    // `!args.dry_run` is exactly the condition under which `main` suppressed
    // the CLI row (see `InterceptArgs::suppresses_cli_usage_row`); a dry-run
    // unblock keeps its CLI row and never needs this fallback.
    if result.is_err()
        && !args.dry_run
        && let Err(usage_err) = crate::usage::record_invocation("intercept")
    {
        tracing::warn!(
            target: "anvil_cli",
            error = %usage_err,
            "usage: failed to record fallback intercept-unblock observation after \
             daemon dispatch failure; continuing",
        );
    }
    result
}

/// Resolved CLI mode after exclusivity + completeness checks.
/// Clap enforces the `conflicts_with_*` rules; this function turns
/// the remaining ambiguity into a single bail (the user supplied no
/// target at all) or a typed mode.
enum UnblockMode {
    /// MLP2-026: legacy cascade clearing. Positional path + flag.
    Cascade(std::path::PathBuf),
    /// RCLI3-017b: per-fence unblock for a single worktree.
    PerFence(std::path::PathBuf),
    /// RCLI3-017b: clear every fence.
    AllFences,
}

fn resolve_unblock_mode(args: &UnblockArgs) -> Result<UnblockMode> {
    if args.all {
        return Ok(UnblockMode::AllFences);
    }
    if let Some(path) = &args.worktree {
        return Ok(UnblockMode::PerFence(path.clone()));
    }
    if let Some(path) = &args.worktree_arg {
        if args.acknowledge_cascade {
            return Ok(UnblockMode::Cascade(path.clone()));
        }
        anyhow::bail!(
            "anvil intercept unblock <WORKTREE> requires --acknowledge-cascade to clear a \
             degraded:fence-cascade; for per-fence unblock prefer `anvil intercept unblock \
             --worktree {}`",
            path.display(),
        );
    }
    anyhow::bail!(
        "anvil intercept unblock needs a target: pass --worktree <PATH> for a per-fence \
         unblock, --all to clear every fence, or <WORKTREE> --acknowledge-cascade for a \
         cascade clear",
    );
}

fn run_unblock_cascade(worktree: &std::path::Path) -> Result<()> {
    // Canonicalise the path before dispatch. Mirrors the daemon's
    // own `lookup_path` guard so an operator typing `./wt` and
    // an operator typing the absolute path hit the same cascade
    // record.
    let canonical = std::fs::canonicalize(worktree).with_context(|| {
        format!(
            "failed to canonicalise worktree path {}",
            worktree.display(),
        )
    })?;
    let cleared = dispatch_unblock_cascade(&canonical)?;
    if cleared {
        println!("cascade cleared for worktree {}", canonical.display());
    } else {
        println!(
            "no cascade engaged for worktree {} (no-op)",
            canonical.display(),
        );
    }
    Ok(())
}

fn run_unblock_per_fence(worktree: &std::path::Path, dry_run: bool) -> Result<()> {
    let canonical = std::fs::canonicalize(worktree).with_context(|| {
        format!(
            "failed to canonicalise worktree path {}",
            worktree.display(),
        )
    })?;
    if dry_run {
        // Query status to determine whether the worktree is
        // currently fenced. The status snapshot is the same shape
        // an operator would see from `anvil intercept status`, so
        // the preview matches their mental model.
        let status = query_daemon_status()?;
        let engaged = status
            .fences
            .iter()
            .any(|fence| fence.worktree == canonical);
        if engaged {
            println!(
                "dry-run: would clear fence for worktree {}",
                canonical.display()
            );
        } else {
            println!(
                "dry-run: no fence engaged for worktree {} (no-op)",
                canonical.display(),
            );
        }
        return Ok(());
    }
    let cleared = dispatch_unblock_worktree(&canonical)?;
    if cleared {
        println!("fence cleared for worktree {}", canonical.display());
    } else {
        println!(
            "no fence engaged for worktree {} (no-op)",
            canonical.display(),
        );
    }
    Ok(())
}

fn run_unblock_all(dry_run: bool) -> Result<()> {
    // `--all` is implemented client-side: query the daemon for the
    // current fence list, then issue one unblock per worktree. The
    // alternative (a single daemon-side `unblock-all` verb) would
    // close the window between query and unblock more cleanly, but
    // keeping the wire-protocol surface to one new verb keeps the
    // back-compat footprint small. Concurrent fences engaged
    // between query and unblock simply remain — the operator can
    // re-run `--all`.
    let status = query_daemon_status()?;
    if status.fences.is_empty() {
        println!("no fences engaged (no-op)");
        return Ok(());
    }
    if dry_run {
        println!("dry-run: would clear {} fence(s):", status.fences.len());
        for fence in &status.fences {
            println!("  {}", fence.worktree.display());
        }
        return Ok(());
    }
    let mut cleared = 0_usize;
    for fence in &status.fences {
        if dispatch_unblock_worktree(&fence.worktree)? {
            cleared += 1;
            println!("fence cleared for worktree {}", fence.worktree.display());
        }
    }
    println!("cleared {cleared} fence(s)");
    Ok(())
}

fn run_status(args: &StatusArgs) -> Result<()> {
    let snapshot = query_daemon_status()?;
    if args.json {
        let json = serde_json::to_string_pretty(&snapshot)
            .context("failed to serialise daemon status as JSON")?;
        println!("{json}");
    } else {
        print!(
            "{}",
            render_status_lines_with_pid(&snapshot, daemon_pid_for_display())
        );
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
pub(crate) fn query_daemon_status() -> Result<DaemonStatusV1> {
    use anvil_intercept::ipc;

    let socket_path =
        ipc::resolve_socket_path().context("failed to resolve intercept daemon socket path")?;
    query_daemon_status_at(&socket_path)
}

/// MLP2-051f: like [`query_daemon_status`] but with a caller-chosen
/// wall-clock budget. The activation diagnostic uses this with a
/// 500 ms cap so interactive `anvil start --verify` does not inherit
/// the 2 s `query_daemon_status` default when the daemon is hung or
/// the per-user socket is wedged.
pub(crate) fn query_daemon_status_with_timeout(
    timeout: std::time::Duration,
) -> Result<DaemonStatusV1> {
    #[cfg(unix)]
    {
        use anvil_intercept::ipc;

        let socket_path =
            ipc::resolve_socket_path().context("failed to resolve intercept daemon socket path")?;
        query_daemon_status_at_with_timeout(&socket_path, timeout)
    }
    #[cfg(windows)]
    {
        let pipe_name = anvil_intercept::ipc::resolve_pipe_name()
            .context("failed to resolve intercept daemon pipe name")?;
        query_daemon_status_windows_at_with_timeout(&pipe_name, timeout)
    }
    #[cfg(all(not(unix), not(windows)))]
    {
        let _ = timeout;
        anyhow::bail!("intercept daemon IPC is not supported on this platform")
    }
}

/// DLIFE-003: the thin CLI entry point for the DLIFE-002 daemon ensure
/// primitive. `anvil start` (DLIFE-003) and `anvil watch` (DLIFE-004)
/// call this to bring up the per-user save-time daemon, passing the
/// [`StartCapability`] they decided from their own flags/context. The
/// primitive itself (probe → same-user lock → re-probe → detached spawn
/// → bound-wait) lives in `anvil_intercept::ensure`; this wrapper only
/// builds the platform launcher that re-execs *this* binary as
/// `anvil intercept start --foreground` (the operator daemon surface
/// the bail at [`run_start`] still guards for direct callers).
///
/// On Unix and Windows the launcher is a [`DetachedCommandLauncher`] over
/// `current_exe()`; if `current_exe()` cannot be resolved the ensure
/// degrades to [`EnsureOutcome::Failed`] with an actionable hint rather
/// than spawning a daemon from an unknown path. Other platforms forward
/// the deterministic [`NoStartReason::PlatformUnsupported`] outcome.
#[cfg(unix)]
pub(crate) fn ensure_save_time_daemon(
    capability: anvil_intercept::ensure::StartCapability,
) -> anvil_intercept::ensure::EnsureOutcome {
    use anvil_intercept::ensure::{DetachedCommandLauncher, EnsureOutcome, ensure_daemon};

    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            return EnsureOutcome::Failed {
                recovery: format!(
                    "could not resolve the anvil executable to launch the daemon ({err}); \
                     run `anvil intercept start --foreground` to start it manually"
                ),
            };
        }
    };
    let launcher = DetachedCommandLauncher::new(
        exe,
        vec!["intercept".into(), "start".into(), "--foreground".into()],
    );
    ensure_daemon(capability, &launcher)
}

/// Windows entry: same detached re-exec launcher as Unix, probing the
/// per-user named pipe (CIB-072 / GH #2609).
#[cfg(windows)]
pub(crate) fn ensure_save_time_daemon(
    capability: anvil_intercept::ensure::StartCapability,
) -> anvil_intercept::ensure::EnsureOutcome {
    use anvil_intercept::ensure::{DetachedCommandLauncher, EnsureOutcome, ensure_daemon};

    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            return EnsureOutcome::Failed {
                recovery: format!(
                    "could not resolve the anvil executable to launch the daemon ({err}); \
                     run `anvil intercept start --foreground` to start it manually"
                ),
            };
        }
    };
    let launcher = DetachedCommandLauncher::new(
        exe,
        vec!["intercept".into(), "start".into(), "--foreground".into()],
    );
    ensure_daemon(capability, &launcher)
}

/// Platforms without a detached launcher implementation.
#[cfg(all(not(unix), not(windows)))]
pub(crate) fn ensure_save_time_daemon(
    _capability: anvil_intercept::ensure::StartCapability,
) -> anvil_intercept::ensure::EnsureOutcome {
    anvil_intercept::ensure::platform_unsupported_outcome()
}

/// Issue a `query_status` JSON-RPC request against an already-resolved
/// daemon socket. Factored from [`query_daemon_status`] so MLP2-051b's
/// MCP shim can reuse the same wire path against its existing
/// per-client socket without re-resolving the per-user default.
#[cfg(unix)]
pub(crate) fn query_daemon_status_at(socket_path: &std::path::Path) -> Result<DaemonStatusV1> {
    query_daemon_status_at_with_timeout(socket_path, std::time::Duration::from_secs(2))
}

/// MLP2-051f: timeout-parameterised body of [`query_daemon_status_at`].
/// Existing callers use the 2 s wrapper; the activation diagnostic
/// (`activation::daemon_evidence`) overrides to 500 ms via
/// [`query_daemon_status_with_timeout`].
#[cfg(unix)]
pub(crate) fn query_daemon_status_at_with_timeout(
    socket_path: &std::path::Path,
    request_timeout: std::time::Duration,
) -> Result<DaemonStatusV1> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Instant;

    use anvil_intercept::ipc;

    if let Err(err) = ipc::validate_socket_path_for_client(socket_path) {
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
    let mut stream = UnixStream::connect(socket_path).with_context(|| {
        format!(
            "failed to connect to intercept daemon socket {}",
            socket_path.display(),
        )
    })?;
    ipc::validate_connected_peer_for_client(&stream)
        .map_err(|err| anyhow::anyhow!("daemon peer credentials rejected: {err}"))?;
    stream
        .set_write_timeout(Some(request_timeout))
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

    // MLP2-051f post-ship hardening (council 2026-05-22): enforce a
    // single wall-clock deadline across the read loop. `SO_RCVTIMEO`
    // is a per-syscall timeout — a daemon that drip-feeds one byte
    // every (timeout − 1ms) keeps `read_until(b'\n')` alive for
    // `RESPONSE_LINE_BYTES * request_timeout` (~524 s at 500 ms /
    // 1 MiB cap) before the previous implementation gave up. The new
    // loop samples `deadline = Instant::now() + request_timeout` once,
    // then refreshes `set_read_timeout(remaining)` before each read
    // so the total wall-clock spent on reads cannot exceed the
    // budget — independent of how the daemon paces its writes.
    let deadline = Instant::now() + request_timeout;
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    // Track the index from which to search for the framing newline.
    // Without this cursor, each iteration would `buf.iter().position`
    // over the entire accumulated buffer — O(n²) work near the 1 MiB
    // cap (Copilot review #1848). The newline can only land in
    // bytes read this iteration; scan only those, then advance the
    // cursor in lock-step with `buf.len()`.
    let mut scan_from = 0_usize;
    loop {
        let now = Instant::now();
        let remaining = deadline
            .checked_duration_since(now)
            .filter(|d| !d.is_zero());
        let Some(remaining) = remaining else {
            anyhow::bail!(
                "timed out waiting for daemon response on socket {} (deadline exhausted across read iterations)",
                socket_path.display(),
            );
        };
        stream
            .set_read_timeout(Some(remaining))
            .context("failed to refresh read timeout")?;
        let n = match stream.read(&mut chunk) {
            Ok(n) => n,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                anyhow::bail!(
                    "timed out waiting for daemon response on socket {} (no bytes within wall-clock budget)",
                    socket_path.display(),
                );
            }
            Err(err) => {
                return Err(anyhow::Error::new(err).context("failed to read query_status response"));
            }
        };
        if n == 0 {
            anyhow::bail!("daemon closed the connection before responding");
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(rel_idx) = buf[scan_from..].iter().position(|b| *b == b'\n') {
            let newline_idx = scan_from + rel_idx;
            buf.truncate(newline_idx + 1);
            break;
        }
        scan_from = buf.len();
        if (buf.len() as u64) > RESPONSE_LINE_BYTES {
            anyhow::bail!("query_status response exceeded {RESPONSE_LINE_BYTES} byte cap");
        }
    }
    let read = buf.len();
    parse_query_status_response_bytes(&buf, read)
}

#[cfg(windows)]
pub(crate) fn query_daemon_status() -> Result<DaemonStatusV1> {
    let pipe_name = anvil_intercept::ipc::resolve_pipe_name()
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
/// `ipc::resolve_pipe_name()`) so the integration test can bind
/// a per-process pipe name and avoid colliding with the canonical
/// per-user pipe a real daemon would own on the same Windows runner.
#[cfg(windows)]
pub(crate) fn query_daemon_status_windows_at(pipe_name: &str) -> Result<DaemonStatusV1> {
    query_daemon_status_windows_at_with_timeout(pipe_name, std::time::Duration::from_secs(2))
}

/// MLP2-051f: timeout-parameterised body of
/// [`query_daemon_status_windows_at`]. Existing callers use the 2 s
/// wrapper; the activation diagnostic overrides to 500 ms.
#[cfg(windows)]
pub(crate) fn query_daemon_status_windows_at_with_timeout(
    pipe_name: &str,
    timeout: std::time::Duration,
) -> Result<DaemonStatusV1> {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    use anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY;

    // The `timeout` budget is a SINGLE wall-clock cap for the whole
    // operation: connect + write + read. The connect side
    // (`WaitNamedPipe` can block when every server instance is busy)
    // gets the full budget up front; the request side then gets
    // `timeout - elapsed_since_start`. Without that split, a daemon
    // that accepts the connection slowly *and* writes slowly could
    // burn ~2× the budget (one full timeout each on connect and read)
    // — Copilot review #1840 caught that and the activation
    // interactive verify (500 ms) needs the single-deadline contract.
    //
    // Synchronous Win32 `ReadFile` on a named pipe has no native
    // timeout setter; the CLI runs the IO on a worker thread and
    // gives up after the remaining budget. A daemon that accepts the
    // connection but never writes leaves the worker blocked, but the
    // CLI is a single-shot process about to exit, so a leaked
    // blocked thread is bounded by process lifetime.
    let deadline_started = Instant::now();
    let connect_timeout = timeout;

    let pipe_name_owned = pipe_name.to_owned();
    let (connect_tx, connect_rx) = mpsc::sync_channel::<std::io::Result<_>>(1);
    let connect_thread = thread::spawn(move || {
        let _ = connect_tx.send(anvil_intercept_win32::connect_owner_only_pipe_client(
            &pipe_name_owned,
        ));
    });
    let connect_outcome = match connect_rx.recv_timeout(connect_timeout) {
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

    // MLP2-051f (Copilot review #1840 follow-up): enforce a single
    // wall-clock cap across connect + read by subtracting the
    // already-spent budget. `saturating_sub` makes a zero-remaining
    // case fall straight through `recv_timeout` with a `Timeout` —
    // the same surface error we'd emit explicitly.
    let request_timeout = timeout.saturating_sub(deadline_started.elapsed());
    let buf = match read_rx.recv_timeout(request_timeout) {
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

/// MLP2-026: send `IpcCommand::UnblockCascade` to the daemon and
/// parse the `{"ok": bool}` response. Returns `Ok(true)` when a
/// cascade was actually cleared, `Ok(false)` on the idempotent
/// no-op case (no cascade was engaged), `Err(_)` on transport
/// or protocol failures.
///
/// Mirrors `query_daemon_status` for connection / framing /
/// response-validation; uses the canonical `unblock-cascade`
/// method name.
#[cfg(unix)]
fn dispatch_unblock_cascade(worktree: &std::path::Path) -> Result<bool> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
    const UNBLOCK_REQUEST_ID: &str = "anvil-cli-intercept-unblock-cascade";

    let socket_path =
        ipc::resolve_socket_path().context("failed to resolve intercept daemon socket path")?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
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

    let mut frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "unblock-cascade",
        "params": { "worktree": worktree.to_string_lossy() },
        "id": UNBLOCK_REQUEST_ID,
    });
    // USAGE-004: attach the operator's salted-hash principal so the
    // daemon records the single source-of-truth `command.invoked` row
    // (the CLI-side row is suppressed for unblock — see main.rs).
    crate::usage::attach_principal(&mut frame);
    let mut frame_bytes = frame.to_string().into_bytes();
    frame_bytes.push(b'\n');
    stream
        .write_all(&frame_bytes)
        .context("failed to send unblock-cascade frame")?;
    stream
        .flush()
        .context("failed to flush unblock-cascade frame")?;

    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut buf)
        .context("failed to read unblock-cascade response")?;
    parse_unblock_cascade_response_bytes(&buf, read, UNBLOCK_REQUEST_ID)
}

#[cfg(windows)]
fn dispatch_unblock_cascade(_worktree: &std::path::Path) -> Result<bool> {
    // Windows CLI surface follows the same pattern as the Unix
    // path but the daemon-side win32 plumbing under MLP2-028 has
    // not landed yet; surface a clear error rather than panic.
    anyhow::bail!(
        "anvil intercept unblock --acknowledge-cascade is not yet supported on Windows; \
         see MLP2-028 for peer-credential support"
    );
}

/// RCLI3-017b: send `IpcCommand::UnblockWorktree` to the daemon and
/// parse the `{"ok": bool}` response. Returns `Ok(true)` when a
/// fence was actually removed, `Ok(false)` on the idempotent no-op
/// (no fence was engaged), `Err(_)` on transport / protocol failure.
///
/// Mirrors `dispatch_unblock_cascade` for connection / framing /
/// response-validation. The wire-level method name is the canonical
/// `unblock-worktree` (kebab-case form pinned by the proto
/// enum's `rename_all = "kebab-case"`).
#[cfg(unix)]
fn dispatch_unblock_worktree(worktree: &std::path::Path) -> Result<bool> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use anvil_intercept::ipc;

    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
    const UNBLOCK_WORKTREE_REQUEST_ID: &str = "anvil-cli-intercept-unblock-worktree";

    let socket_path =
        ipc::resolve_socket_path().context("failed to resolve intercept daemon socket path")?;
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
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

    let mut frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "unblock-worktree",
        "params": { "worktree": worktree.to_string_lossy() },
        "id": UNBLOCK_WORKTREE_REQUEST_ID,
    });
    // USAGE-004: attach the operator's salted-hash principal so the
    // daemon records the single source-of-truth `command.invoked` row
    // (the CLI-side row is suppressed for unblock — see main.rs).
    crate::usage::attach_principal(&mut frame);
    let mut frame_bytes = frame.to_string().into_bytes();
    frame_bytes.push(b'\n');
    stream
        .write_all(&frame_bytes)
        .context("failed to send unblock-worktree frame")?;
    stream
        .flush()
        .context("failed to flush unblock-worktree frame")?;

    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut buf)
        .context("failed to read unblock-worktree response")?;
    parse_unblock_response_bytes(&buf, read, UNBLOCK_WORKTREE_REQUEST_ID, "unblock-worktree")
}

#[cfg(windows)]
fn dispatch_unblock_worktree(_worktree: &std::path::Path) -> Result<bool> {
    anyhow::bail!(
        "anvil intercept unblock --worktree is not yet supported on Windows; \
         see MLP2-028 for peer-credential support"
    );
}

#[cfg(unix)]
fn parse_unblock_cascade_response_bytes(buf: &[u8], read: usize, request_id: &str) -> Result<bool> {
    parse_unblock_response_bytes(buf, read, request_id, "unblock-cascade")
}

/// Shared JSON-RPC envelope validator for both `unblock-cascade` and
/// `unblock-worktree` responses. Extracted so the two CLI dispatchers
/// cannot drift on JSON-RPC version pinning, id matching, or the
/// `result.ok: bool` shape contract.
#[cfg(unix)]
fn parse_unblock_response_bytes(
    buf: &[u8],
    read: usize,
    request_id: &str,
    method_label: &str,
) -> Result<bool> {
    if read == 0 {
        anyhow::bail!("daemon closed the connection before responding");
    }
    if (buf.len() as u64) > RESPONSE_LINE_BYTES {
        anyhow::bail!("{method_label} response exceeded {RESPONSE_LINE_BYTES} byte cap");
    }
    let line = std::str::from_utf8(buf.trim_ascii_end())
        .with_context(|| format!("{method_label} response is not valid UTF-8"))?;
    let response: serde_json::Value = serde_json::from_str(line)
        .with_context(|| format!("{method_label} response is not valid JSON"))?;
    if response.get("jsonrpc") != Some(&serde_json::Value::String("2.0".to_string())) {
        anyhow::bail!(
            "daemon response missing or wrong jsonrpc version (expected \"2.0\"): {response}",
        );
    }
    if response.get("id") != Some(&serde_json::Value::String(request_id.to_string())) {
        anyhow::bail!(
            "daemon response id does not match request (expected {request_id:?}): {response}",
        );
    }
    if let Some(error) = response.get("error") {
        anyhow::bail!("daemon returned a JSON-RPC error: {error}");
    }
    let result = response
        .get("result")
        .ok_or_else(|| anyhow::anyhow!("daemon response missing `result` field: {response}"))?;
    result
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            anyhow::anyhow!("daemon response `result.ok` missing or not a bool: {result}")
        })
}

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
#[cfg(test)]
fn render_status_lines(status: &DaemonStatusV1) -> String {
    render_status_lines_with_pid(status, None)
}

fn render_status_lines_with_pid(status: &DaemonStatusV1, daemon_pid: Option<u32>) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    if let Some(pid) = daemon_pid {
        let _ = writeln!(
            out,
            "daemon:    running (pid {pid}, uptime {}s, version {})",
            status.health.uptime_seconds, status.health.version,
        );
    } else {
        let _ = writeln!(
            out,
            "daemon:    running (uptime {}s, version {})",
            status.health.uptime_seconds, status.health.version,
        );
    }
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
    let _ = writeln!(out, "control:   anvil intercept stop");
    out.push('\n');
    out
}

fn daemon_pid_for_display() -> Option<u32> {
    let path = anvil_intercept::default_pid_file_path().ok()?;
    let record = std::fs::read_to_string(path).ok()?;
    record
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
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
            "`anvil intercept start` requires --foreground; this is the low-level operator/debugging daemon surface. Backgrounded daemon launch is provided to `anvil start` / `anvil watch` via the daemon-lifecycle ensure primitive (DLIFE, ADR-082)."
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
        // INTD-016 / MLP2-024 / #1671 audit closure: load
        // `.anvil.yaml` from the daemon's launch CWD via the shared
        // helper so both daemon entry points (standalone
        // `anvil-intercept` and `anvil intercept start`) honour the
        // same propagation contract. See `config::load_for_daemon_cwd`
        // for why parse / IO failures must be fatal.
        let enforcement_config = config::load_for_daemon_cwd()
            .context("anvil intercept: failed to load enforcement config")?;
        let opts = ForegroundOpts::default().with_enforcement_config(enforcement_config);
        // USAGE-004: inject the command-invocation usage producer so the
        // daemon records `command.invoked` rows for allowlisted JSON-RPC
        // methods to the shared usage sidecar. `None` (unresolvable state
        // dir) ⇒ the daemon still serves, just without usage rows.
        #[cfg(any(unix, windows))]
        let opts = match crate::usage::daemon_usage_emitter() {
            Some(emitter) => opts.with_usage_emitter(emitter),
            None => opts,
        };
        // DPO-001 / DPO-002: inject the save-time gate_evaluated producer
        // and the shared fence constraint_applied sink so the daemon
        // records save-time verdict rows and fence-engage constraint rows
        // to the same usage sidecar. `None` (unresolvable state dir) ⇒ the
        // daemon still serves, just without these observation rows.
        #[cfg(any(unix, windows))]
        let opts = {
            let (em, sink, include_paths) = crate::usage::daemon_observation_producers();
            let opts = match em {
                Some(e) => opts.with_observation_emitter(e),
                None => opts,
            };
            match sink {
                Some(s) => opts.with_observation_sink(s, include_paths),
                None => opts,
            }
        };
        // DSV-005: inject the kernel-backed symbol parser so the daemon can
        // return Certified verdicts (ADR-064: tree-sitter links into this
        // binary, never the `anvil-intercept` crate). Unix-only — the verdict
        // path is unix-gated. `ANVIL_INTERCEPT_DISABLE_SYMBOL_PARSER=1` is a
        // break-glass that withholds the parser (the daemon then returns safe
        // `Partial` verdicts) without a redeploy if the parser ever misbehaves.
        #[cfg(unix)]
        let opts = if std::env::var_os("ANVIL_INTERCEPT_DISABLE_SYMBOL_PARSER")
            .is_some_and(|value| value == "1")
        {
            tracing::warn!(
                target: "anvil_intercept::save_time",
                "ANVIL_INTERCEPT_DISABLE_SYMBOL_PARSER=1 — symbol parser withheld; \
                 validate_paths will return Partial verdicts only",
            );
            opts
        } else {
            opts.with_symbol_parser(std::sync::Arc::new(
                crate::intercept_symbol_parser::KernelSymbolParser::new(),
            ))
        };
        run_foreground(opts, token).await
    })
}

#[cfg(test)]
mod tests {
    use anvil_intercept_proto::status::{
        DaemonStatusV1, HealthStateV1, IpcStateV1, LatencyMidEditMapV1, LatencyRollupV1,
    };

    use super::*;

    /// USAGE-004: `suppresses_cli_usage_row` is the CLI-row suppression
    /// trigger — true only for a NON-dry-run `intercept unblock` (which
    /// dispatches a daemon `unblock-*` verb that records the
    /// authoritative row). A dry-run unblock contacts no daemon, so its
    /// CLI row must be kept; other intercept subcommands always keep it.
    /// Parsed via clap so it tracks the real CLI.
    #[test]
    fn suppresses_cli_usage_row_only_for_non_dry_run_unblock() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(flatten)]
            args: InterceptArgs,
        }

        let parse = |argv: &[&str]| TestCli::try_parse_from(argv).expect("parse").args;
        // Non-dry-run unblock → suppress (daemon records it).
        assert!(parse(&["x", "unblock", "--all"]).suppresses_cli_usage_row());
        assert!(parse(&["x", "unblock", "--worktree", "/tmp/wt"]).suppresses_cli_usage_row());
        // Dry-run unblock → keep the CLI row (no daemon contact).
        assert!(!parse(&["x", "unblock", "--all", "--dry-run"]).suppresses_cli_usage_row());
        assert!(
            !parse(&["x", "unblock", "--worktree", "/tmp/wt", "--dry-run"])
                .suppresses_cli_usage_row()
        );
        // Other intercept subcommands always keep the CLI row.
        assert!(!parse(&["x", "status"]).suppresses_cli_usage_row());
        assert!(!parse(&["x", "start", "--foreground"]).suppresses_cli_usage_row());
    }

    /// Pin the contract: `anvil intercept start` without `--foreground`
    /// exits with a clear error directing the user at the missing flag,
    /// not a silent no-op or a confusing panic. `anvil intercept start`
    /// is the low-level operator surface (module Purpose); backgrounded
    /// launch is reached through `anvil start` / `anvil watch` via the
    /// DLIFE-002 ensure primitive (`anvil_intercept::ensure::ensure_daemon`),
    /// wired into `anvil start` in DLIFE-003 through
    /// [`ensure_save_time_daemon`] (with `anvil watch` following in
    /// DLIFE-004), so this operator bail stays. A future change that wires
    /// backgrounding into this operator command must update this test.
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
            msg.contains("DLIFE"),
            "bail message must point at the daemon-lifecycle backgrounded path (DLIFE), got: {msg}",
        );
    }

    /// Helper: build a fully-defaulted `UnblockArgs` and let the test
    /// flip only the fields it cares about. Keeps the per-test
    /// noise low as the struct grows new flags.
    fn unblock_args_default() -> UnblockArgs {
        UnblockArgs {
            worktree_arg: None,
            worktree: None,
            all: false,
            dry_run: false,
            acknowledge_cascade: false,
        }
    }

    /// MLP2-026: a positional path WITHOUT `--acknowledge-cascade`
    /// still bails — the cascade form requires the affordance
    /// flag, and refusing to silently fall through to per-fence
    /// semantics keeps the two modes disambiguated for legacy
    /// callers. The bail message must point at BOTH `--worktree`
    /// (the new per-fence path) and `--acknowledge-cascade` (the
    /// cascade affordance) so an operator knows which mode they
    /// actually want.
    #[test]
    fn run_unblock_positional_without_acknowledge_flag_bails_with_actionable_message() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let args = UnblockArgs {
            worktree_arg: Some(tmp.path().to_path_buf()),
            ..unblock_args_default()
        };
        let err = run_unblock(&args).expect_err("expected bail without --acknowledge-cascade");
        let msg = format!("{err}");
        assert!(
            msg.contains("--acknowledge-cascade"),
            "bail message must mention --acknowledge-cascade, got: {msg}",
        );
        assert!(
            msg.contains("--worktree"),
            "bail message must point at the per-fence alternative, got: {msg}",
        );
    }

    /// RCLI3-017b: when no target is supplied at all, the bail
    /// message lists every available mode so the operator can
    /// pick one without re-reading `--help`.
    #[test]
    fn run_unblock_without_any_target_bails_listing_all_modes() {
        let args = unblock_args_default();
        let err = run_unblock(&args).expect_err("expected bail without any target");
        let msg = format!("{err}");
        assert!(msg.contains("--worktree"), "got: {msg}");
        assert!(msg.contains("--all"), "got: {msg}");
        assert!(msg.contains("--acknowledge-cascade"), "got: {msg}");
    }

    /// RCLI3-017b: the resolver classifies clap-parsed args into
    /// the typed `UnblockMode` before any IPC dispatch. Pin the
    /// classification so a future refactor cannot silently route
    /// `--worktree` through the cascade path or vice versa.
    #[test]
    fn resolve_unblock_mode_per_fence_flag_routes_to_per_fence_mode() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let args = UnblockArgs {
            worktree: Some(tmp.path().to_path_buf()),
            ..unblock_args_default()
        };
        let mode = resolve_unblock_mode(&args).expect("classification succeeds");
        assert!(
            matches!(mode, UnblockMode::PerFence(ref p) if p == tmp.path()),
            "expected PerFence({:?})",
            tmp.path(),
        );
    }

    #[test]
    fn resolve_unblock_mode_all_flag_routes_to_all_fences() {
        let args = UnblockArgs {
            all: true,
            ..unblock_args_default()
        };
        let mode = resolve_unblock_mode(&args).expect("classification succeeds");
        assert!(matches!(mode, UnblockMode::AllFences));
    }

    #[test]
    fn resolve_unblock_mode_positional_with_cascade_flag_routes_to_cascade() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let args = UnblockArgs {
            worktree_arg: Some(tmp.path().to_path_buf()),
            acknowledge_cascade: true,
            ..unblock_args_default()
        };
        let mode = resolve_unblock_mode(&args).expect("classification succeeds");
        assert!(
            matches!(mode, UnblockMode::Cascade(ref p) if p == tmp.path()),
            "expected Cascade({:?})",
            tmp.path(),
        );
    }

    /// MLP2-026: `parse_unblock_cascade_response_bytes` returns
    /// the inner `ok: bool` on a well-formed response.
    #[cfg(unix)]
    #[test]
    fn parse_unblock_cascade_response_extracts_ok_true() {
        let raw = r#"{"jsonrpc":"2.0","id":"test-id","result":{"ok":true}}"#;
        let bytes = raw.as_bytes();
        let result =
            parse_unblock_cascade_response_bytes(bytes, bytes.len(), "test-id").expect("parse");
        assert!(result);
    }

    #[cfg(unix)]
    #[test]
    fn parse_unblock_cascade_response_extracts_ok_false() {
        let raw = r#"{"jsonrpc":"2.0","id":"test-id","result":{"ok":false}}"#;
        let bytes = raw.as_bytes();
        let result =
            parse_unblock_cascade_response_bytes(bytes, bytes.len(), "test-id").expect("parse");
        assert!(!result);
    }

    #[cfg(unix)]
    #[test]
    fn parse_unblock_cascade_response_rejects_mismatched_id() {
        let raw = r#"{"jsonrpc":"2.0","id":"other-id","result":{"ok":true}}"#;
        let bytes = raw.as_bytes();
        let err =
            parse_unblock_cascade_response_bytes(bytes, bytes.len(), "test-id").expect_err("err");
        assert!(format!("{err}").contains("id does not match"));
    }

    #[cfg(unix)]
    #[test]
    fn parse_unblock_cascade_response_surfaces_jsonrpc_error() {
        let raw = r#"{"jsonrpc":"2.0","id":"test-id","error":{"code":-32603,"message":"boom"}}"#;
        let bytes = raw.as_bytes();
        let err =
            parse_unblock_cascade_response_bytes(bytes, bytes.len(), "test-id").expect_err("err");
        assert!(format!("{err}").contains("JSON-RPC error"));
    }

    /// RCLI3-017b: the shared parser must accept the same `result.ok`
    /// shape under the per-fence `unblock-worktree` label and surface
    /// the new label in error wording. Pinning this keeps the cascade
    /// and per-fence paths from drifting on JSON-RPC envelope rules.
    #[cfg(unix)]
    #[test]
    fn parse_unblock_response_handles_worktree_label() {
        let raw = r#"{"jsonrpc":"2.0","id":"test-id","result":{"ok":true}}"#;
        let bytes = raw.as_bytes();
        let result =
            parse_unblock_response_bytes(bytes, bytes.len(), "test-id", "unblock-worktree")
                .expect("parse");
        assert!(result);
    }

    #[cfg(unix)]
    #[test]
    fn parse_unblock_response_oversize_cites_method_label() {
        // Fabricate a response that exceeds the byte cap; the bail
        // wording must name the method so an operator parsing CI
        // logs can tell which path tripped the cap.
        let big = vec![
            b'a';
            usize::try_from(RESPONSE_LINE_BYTES + 2)
                .expect("response byte cap fits usize")
        ];
        let err = parse_unblock_response_bytes(&big, big.len(), "test-id", "unblock-worktree")
            .expect_err("oversize must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("unblock-worktree"),
            "error must cite the method label, got: {msg}",
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
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
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
            agent_tag: None,
            daemon_issued_tag: None,
        }];
        let rendered = render_status_lines(&status);
        assert!(rendered.contains("sessions:  1 active"));
    }

    #[test]
    fn cli_status_names_daemon_pid_when_available() {
        let rendered = render_status_lines_with_pid(&empty_status(), Some(4242));
        assert!(
            rendered.contains("daemon:    running (pid 4242, uptime 12s, version 0.5.1-beta)"),
            "status should name the daemon PID when available; got:\n{rendered}",
        );
    }

    #[test]
    fn cli_status_names_intercept_stop_recovery_command() {
        let rendered = render_status_lines(&empty_status());
        assert!(
            rendered.contains("control:   anvil intercept stop"),
            "status should show the daemon stop command; got:\n{rendered}",
        );
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
    /// `ipc::resolve_pipe_name()` value) so the test never
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
                    &[],
                    None,
                    std::time::Instant::now(),
                    std::time::Instant::now(),
                    "0.0.0-windows-test",
                    anvil_intercept::status::IpcState::Serving,
                    None,
                    None,
                    // MLP2-051h: synthetic Windows test fixture has
                    // no live wall clock; 0 is the no-anchor sentinel.
                    0,
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
        let _runtime_guard = runtime.enter();
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
