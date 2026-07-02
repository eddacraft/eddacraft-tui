//! DSV-007 Task 12: the `watch` save-time daemon client + scoped fallback.
//!
//! `anvil watch` becomes a thin client of the resident save-time daemon: on each
//! coalesced change it sends the classified changed paths to the daemon's
//! [`validate_paths`](anvil_intercept_proto::protocol::ANVIL_VALIDATE_PATHS) verb
//! and uses the daemon's verdict instead of spawning a per-save `anvil check`
//! subprocess (ADR-061 §3 — "one warm model, never double-scans"). When the
//! daemon is absent — never started, or it died mid-session — watch falls back to
//! a **scoped** `check` over exactly the changed paths (never `--all`) and
//! surfaces `workspace_assurance{state: unavailable, reason: daemon-absent}`
//! rather than a truncated `clean`.
//!
//! ## Connection lifecycle (item 8)
//!
//! `watch` is a persistent client, so it must survive the daemon dying
//! mid-session, not just being absent at start:
//!
//! - Every request is bounded by a read/write timeout. A mid-stream drop / EOF /
//!   timeout (e.g. the daemon's 250 ms `SHUTDOWN_DRAIN_DEADLINE` truncating an
//!   in-flight response) is treated as *daemon-absent for that batch* → scoped
//!   fallback + `unavailable{daemon-absent}`, never a truncated `clean`.
//! - The first-fallback WARN is latched so a wedged/absent daemon warns **once
//!   per disconnect**, not once per save.
//! - On reconnect (a `validate_paths` that succeeds after a prior failure) the
//!   latch resets and watch re-issues
//!   [`request_full_scan`](anvil_intercept_proto::protocol::ANVIL_REQUEST_FULL_SCAN)
//!   so assurance re-establishes from `stale` rather than a pre-disconnect
//!   `clean`.

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};

use anvil_intercept_proto::protocol::{
    AssuranceState, ChangeDescriptor, ChangeKindWire, StaleReason, ValidatePathsResponse,
    WorkspaceAssurance,
};
use anvil_kernel_types::Severity;

/// Why a daemon request could not produce a verdict. Every variant folds to the
/// same client behaviour — fall back to a scoped check and report
/// `unavailable{daemon-absent}` — so the type is intentionally coarse: the
/// distinction the surfaces care about is "did the daemon give us a verdict or
/// not", not the specific transport failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveTimeClientError {
    /// No verdict: the daemon was absent, refused, errored, or the connection
    /// dropped / timed out mid-response.
    Unavailable,
}

/// The daemon transport behind the [`WatchSaveTimeClient`] state machine.
/// Dependency-inverted so the connection-lifecycle logic (warn-once latch,
/// reconnect → re-scan) is unit-testable with an in-process fake, while the
/// production impl ([`SocketSaveTimeTransport`]) speaks JSON-RPC over the Unix
/// socket.
pub(crate) trait SaveTimeTransport: Send {
    /// Certify `paths` under `workspace_root`. `Err(Unavailable)` means no
    /// verdict (absent / dead / timed-out daemon) and triggers the fallback.
    fn validate_paths(
        &self,
        workspace_root: &Path,
        paths: &[ChangeDescriptor],
    ) -> Result<ValidatePathsResponse, SaveTimeClientError>;

    /// Ask the daemon to re-establish a clean baseline after a reconnect. The
    /// returned assurance is advisory here (watch does not render it); failures
    /// are swallowed because a reconnect that cannot re-scan still lets the next
    /// `validate_paths` proceed.
    fn request_full_scan(&self, workspace_root: &Path) -> Result<(), SaveTimeClientError>;

    /// Read-only assurance snapshot for `workspace_root` (the `anvil status`
    /// surface — DSV-007 Task 17), without submitting a change set.
    /// `Err(Unavailable)` ⇒ no daemon answered ⇒ status renders
    /// `unavailable{daemon-absent}`, never a stale cached `clean`.
    fn workspace_status(
        &self,
        workspace_root: &Path,
    ) -> Result<WorkspaceAssurance, SaveTimeClientError>;
}

/// Rollout posture for the user-facing save-time daemon surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DaemonRoutingMode {
    /// Explicit opt-out: keep the pre-DSV subprocess-only path.
    Disabled,
    /// Unset environment: use daemon routing only when the daemon is already live
    /// and serving the save-time verbs, avoiding daemon-absent warning storms.
    DefaultOnWhenLive,
    /// Explicit opt-in: preserve the old preview behaviour, including scoped
    /// daemon-absent fallback if the endpoint cannot answer.
    ForcedOn,
}

/// Resolve the rollout posture controlled by `ANVIL_WATCH_DAEMON`.
///
/// The v0.8 default-on flip treats an unset variable as safe default-on: route
/// only after a live save-time daemon answers an initial status probe. Explicit
/// true values keep the old opt-in preview semantics; explicit false values are
/// the documented rollout opt-out.
pub(crate) fn daemon_routing_mode() -> DaemonRoutingMode {
    daemon_routing_mode_from(std::env::var_os("ANVIL_WATCH_DAEMON").as_deref())
}

fn daemon_routing_mode_from(value: Option<&OsStr>) -> DaemonRoutingMode {
    let Some(value) = value else {
        return DaemonRoutingMode::DefaultOnWhenLive;
    };
    match value.to_string_lossy().to_ascii_lowercase().as_str() {
        "0" | "false" | "off" | "no" => DaemonRoutingMode::Disabled,
        "1" | "true" | "on" | "yes" => DaemonRoutingMode::ForcedOn,
        _ => DaemonRoutingMode::DefaultOnWhenLive,
    }
}

/// One-shot read of a worktree's [`WorkspaceAssurance`] from the daemon. `None`
/// when no daemon answered (absent socket / dead daemon / unserved verb) — the
/// `status` surface renders that as `unavailable{daemon-absent}` rather than a
/// stale cached `clean`.
#[cfg(unix)]
pub(crate) fn query_workspace_status(workspace_root: &Path) -> Option<WorkspaceAssurance> {
    SocketSaveTimeTransport::resolve()?
        .workspace_status(workspace_root)
        .ok()
}

#[cfg(windows)]
pub(crate) fn query_workspace_status(workspace_root: &Path) -> Option<WorkspaceAssurance> {
    WindowsPipeSaveTimeTransport::resolve()?
        .workspace_status(workspace_root)
        .ok()
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn query_workspace_status(_workspace_root: &Path) -> Option<WorkspaceAssurance> {
    None
}

/// Result of a best-effort GCTX cold-start warm-up (GCTX-010 C1 / ADR-085).
/// Advisory only — production callers ignore it (warm-up never changes a tool
/// result); it exists so the trigger is unit-testable and traceable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WarmupOutcome {
    /// A live daemon accepted the `request_full_scan` enqueue.
    Requested,
    /// This root was warm-enqueued within the recent cooldown window, so the
    /// request was suppressed (the scan is already queued / running, and the
    /// daemon coalesces in any case). The window is bounded, not permanent, so a
    /// still-cold root self-corrects and re-warms once it elapses.
    AlreadyRequested,
    /// Suppressed by the `ANVIL_WATCH_DAEMON=0` operator opt-out — no daemon
    /// contact at all.
    Disabled,
    /// No live daemon answered (absent / dead / unserved verb). Warm-up is
    /// skipped; the graph warms lazily on the first save instead.
    DaemonAbsent,
}

/// What [`warm_up_root`] should do, decided purely from policy inputs so the
/// opt-out and per-session dedup are unit-testable without env vars or globals.
#[cfg(any(unix, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WarmupPlan {
    /// Suppress with this outcome, without touching the daemon.
    Skip(WarmupOutcome),
    /// Attempt the enqueue against the resolved transport.
    Attempt,
}

/// Pure warm-up policy (GCTX-010 C1): respect the `ANVIL_WATCH_DAEMON=0`
/// opt-out, then dedupe a root warmed within the recent cooldown. Kept separate
/// from transport resolution and global state so it can be exhaustively tested.
#[cfg(any(unix, windows))]
fn plan_warm_up(mode: DaemonRoutingMode, recently_warmed: bool) -> WarmupPlan {
    if mode == DaemonRoutingMode::Disabled {
        return WarmupPlan::Skip(WarmupOutcome::Disabled);
    }
    if recently_warmed {
        return WarmupPlan::Skip(WarmupOutcome::AlreadyRequested);
    }
    WarmupPlan::Attempt
}

/// How long a warm-enqueued root is deduped before it may be re-warmed. The
/// dedup is *time-bounded* on purpose: because the enqueue is fire-and-forget
/// (`request_full_scan` returns `Ok` once the round-trip is detached, even if
/// the daemon was transiently absent and the send later failed), a permanent
/// mark could suppress the on-demand re-warm for a root that never actually
/// enqueued — leaving a read-only session (no saves, so no save-time backstop)
/// cold. A bounded window keeps the anti-spam property (one enqueue per root per
/// window, not one per `NotReady` miss) while letting an optimistic mark
/// self-correct.
#[cfg(any(unix, windows))]
const WARMUP_DEDUP_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(30);

/// Roots warm-enqueued in this process and when, so [`recently_warmed`] can
/// suppress repeat enqueues within [`WARMUP_DEDUP_COOLDOWN`]. Only successful
/// (`Requested`) enqueues are recorded.
#[cfg(any(unix, windows))]
fn session_warmed_roots()
-> &'static std::sync::Mutex<std::collections::HashMap<PathBuf, std::time::Instant>> {
    static WARMED: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<PathBuf, std::time::Instant>>,
    > = std::sync::OnceLock::new();
    WARMED.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

#[cfg(any(unix, windows))]
fn recently_warmed(workspace_root: &Path) -> bool {
    session_warmed_roots()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(workspace_root)
        .is_some_and(|marked| marked.elapsed() < WARMUP_DEDUP_COOLDOWN)
}

#[cfg(any(unix, windows))]
fn mark_warmed(workspace_root: &Path) {
    session_warmed_roots()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(workspace_root.to_path_buf(), std::time::Instant::now());
}

/// Best-effort cold-start warm-up (GCTX-010 C1 / ADR-085): ask the daemon to
/// drive a full scan of `workspace_root` so a fresh assistant session reaches a
/// populated graph without a manual save (the daemon's warm graph cache is
/// otherwise only save-populated).
///
/// This only *enqueues* via the existing
/// [`request_full_scan`](anvil_intercept_proto::protocol::ANVIL_REQUEST_FULL_SCAN)
/// verb; the daemon-side full-scan executor (DSV-045) performs the actual
/// `Pending → Running → Clean` drive and coalesces repeated requests per
/// worktree. Strictly best-effort: it honours the `ANVIL_WATCH_DAEMON=0`
/// opt-out, dedupes a root within [`WARMUP_DEDUP_COOLDOWN`], and treats an
/// absent or unresponsive daemon as a silent skip (GCTX is daemon-required and
/// degrades — never a tool error, never a panic). The enqueue is fire-and-forget
/// (the transport detaches the round-trip), so it never blocks the MCP
/// handshake; on a very short session the detached thread may not complete, in
/// which case the daemon's own first-contact auto-enqueue (DSV-045) is the
/// backstop.
#[cfg(any(unix, windows))]
pub(crate) fn warm_up_root(workspace_root: &Path) -> WarmupOutcome {
    match plan_warm_up(daemon_routing_mode(), recently_warmed(workspace_root)) {
        WarmupPlan::Skip(outcome) => outcome,
        WarmupPlan::Attempt => {
            let outcome = resolve_and_warm(workspace_root);
            if outcome == WarmupOutcome::Requested {
                mark_warmed(workspace_root);
            }
            outcome
        }
    }
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn warm_up_root(_workspace_root: &Path) -> WarmupOutcome {
    WarmupOutcome::DaemonAbsent
}

/// Resolve the per-platform daemon transport and attempt the enqueue. Split out
/// so [`warm_up_root`]'s opt-out + dedup policy is platform-agnostic.
#[cfg(unix)]
fn resolve_and_warm(workspace_root: &Path) -> WarmupOutcome {
    match SocketSaveTimeTransport::resolve() {
        Some(transport) => warm_up_with(&transport, workspace_root),
        None => WarmupOutcome::DaemonAbsent,
    }
}

#[cfg(windows)]
fn resolve_and_warm(workspace_root: &Path) -> WarmupOutcome {
    match WindowsPipeSaveTimeTransport::resolve() {
        Some(transport) => warm_up_with(&transport, workspace_root),
        None => WarmupOutcome::DaemonAbsent,
    }
}

/// Core of [`warm_up_root`], split from transport resolution so the enqueue
/// policy is unit-testable against an injected [`SaveTimeTransport`] without a
/// live daemon. All transport failures fold to `Unavailable` (the trait's
/// coarse, intentional contract), which is the silent-skip case here.
#[cfg(any(unix, windows))]
fn warm_up_with(transport: &impl SaveTimeTransport, workspace_root: &Path) -> WarmupOutcome {
    match transport.request_full_scan(workspace_root) {
        Ok(()) => WarmupOutcome::Requested,
        Err(SaveTimeClientError::Unavailable) => WarmupOutcome::DaemonAbsent,
    }
}

/// What a save-time cycle resolved to.
#[derive(Debug)]
pub(crate) enum SaveTimeDecision {
    /// The daemon answered; render this verdict and skip the subprocess.
    Validated(Box<ValidatePathsResponse>),
    /// No daemon verdict; run a scoped `check` over `scoped_paths` (never
    /// `--all`) and surface `assurance` (always `unavailable{daemon-absent}`).
    FellBack {
        /// Always `unavailable{daemon-absent}` — never a truncated `clean`.
        assurance: WorkspaceAssurance,
        /// The changed paths to scope the fallback `check` to. Empty means a
        /// delete-/initial-driven cycle with nothing to scope: the surface
        /// reports `unavailable` without an `--all` walk.
        scoped_paths: Vec<String>,
        /// `true` only on the first fallback of a disconnect, so the caller
        /// WARNs once per disconnect rather than once per save.
        warned: bool,
    },
}

/// The persistent per-`watch` save-time client. Holds the daemon connection
/// posture across coalesced dispatches: whether we believe the daemon is
/// reachable (`connected`) and whether the current disconnect has already
/// warned (`warned`).
pub(crate) struct WatchSaveTimeClient {
    transport: Box<dyn SaveTimeTransport>,
    workspace_root: PathBuf,
    /// Optimistic at construction: the first cycle attempts the daemon, and a
    /// daemon-absent-at-start failure still WARNs once.
    connected: bool,
    /// Latched on the first fallback of a disconnect; cleared on reconnect.
    warned: bool,
}

impl WatchSaveTimeClient {
    pub(crate) fn new(transport: Box<dyn SaveTimeTransport>, workspace_root: PathBuf) -> Self {
        Self {
            transport,
            workspace_root,
            connected: true,
            warned: false,
        }
    }

    /// Run one save-time cycle for `changed_paths` (absolute paths of the files
    /// changed since the last dispatch).
    pub(crate) fn validate(&mut self, changed_paths: Vec<String>) -> SaveTimeDecision {
        // Drop any path that is not under the workspace root: the wire is
        // root-relative and the daemon authorises against the admitted root, so
        // an out-of-root path could never be certified and must never be sent as
        // a bogus relative path.
        let descriptors: Vec<ChangeDescriptor> = changed_paths
            .iter()
            .filter_map(|p| classify_change(p, &self.workspace_root))
            .collect();

        match self
            .transport
            .validate_paths(&self.workspace_root, &descriptors)
        {
            Ok(response) => {
                if !self.connected {
                    // Reconnect: re-establish the baseline so assurance comes
                    // back from `stale`, not a pre-disconnect `clean`. The socket
                    // transport fires this off-thread (the daemon starts the scan
                    // on receipt; the ack is irrelevant), so it never stalls this
                    // reconnect verdict.
                    let _ = self.transport.request_full_scan(&self.workspace_root);
                    self.connected = true;
                    self.warned = false;
                }
                SaveTimeDecision::Validated(Box::new(response))
            }
            Err(SaveTimeClientError::Unavailable) => {
                let warned = !self.warned;
                self.warned = true;
                self.connected = false;
                SaveTimeDecision::FellBack {
                    assurance: daemon_absent_assurance(),
                    scoped_paths: changed_paths,
                    warned,
                }
            }
        }
    }
}

/// The assurance snapshot surfaced on every daemon-absent fallback. The
/// invariant (proto): `Unavailable` always carries [`StaleReason::DaemonAbsent`]
/// — never a truncated `clean`.
pub(crate) fn daemon_absent_assurance() -> WorkspaceAssurance {
    WorkspaceAssurance {
        state: AssuranceState::Unavailable,
        reason: Some(StaleReason::DaemonAbsent),
        generation: 0,
        last_full_scan: None,
        scan_coverage: None,
    }
}

/// Classify one changed path into a wire [`ChangeDescriptor`], or `None` when the
/// path is not under `workspace_root` (the wire is root-relative; an out-of-root
/// path is dropped rather than sent as a bogus absolute "relative" path). The
/// daemon re-derives identity from disk and never trusts these hints for a
/// verdict, so the classification is coarse: a path that still exists on disk is
/// `Modified`, one that no longer does is `Deleted`. `content_hash`/`mtime` are
/// left unset — watch has no cheaper-than-the-daemon hint to offer.
fn classify_change(absolute_path: &str, workspace_root: &Path) -> Option<ChangeDescriptor> {
    let abs = Path::new(absolute_path);
    let relative = abs
        .strip_prefix(workspace_root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    let change = if abs.exists() {
        ChangeKindWire::Modified
    } else {
        ChangeKindWire::Deleted
    };
    Some(ChangeDescriptor {
        path: relative,
        change,
        content_hash: None,
        mtime: None,
    })
}

/// Exit-code parity with `anvil check`: any `Error`-severity finding is a
/// non-zero (blocking) result; `Info`/`Warning` keep a zero result. Used to
/// label the TUI action footer and decide whether the plain surface prints a
/// failure line.
#[must_use]
pub(crate) fn verdict_exit_code(response: &ValidatePathsResponse) -> i32 {
    i32::from(
        response
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
    )
}

/// Render a daemon verdict to a plain-text sink (the non-TUI, non-JSON watch
/// surface). Mirrors the shape `anvil check` prints — one line per finding plus
/// a trailing assurance line — so a watch user sees equivalent output whether
/// the verdict came from the daemon or the subprocess fallback.
pub(crate) fn render_daemon_verdict_plain<W: Write>(
    sink: &mut W,
    response: &ValidatePathsResponse,
) -> std::io::Result<()> {
    for diag in &response.diagnostics {
        let severity = match diag.severity {
            Severity::Info => "info",
            Severity::Warning => "warn",
            Severity::Error => "error",
        };
        let line = diag.location.line.map_or_else(
            || diag.location.file.clone(),
            |l| format!("{}:{l}", diag.location.file),
        );
        writeln!(
            sink,
            "  {} ({severity}) — {} [{line}]",
            diag.id, diag.summary
        )?;
    }
    writeln!(
        sink,
        "anvil watch: {} ({} finding(s))",
        assurance_label(&response.workspace_assurance),
        response.diagnostics.len(),
    )
}

/// Human label for an assurance snapshot: `state` plus the `reason` when one is
/// present (`stale`/`unavailable`).
#[must_use]
pub(crate) fn assurance_label(assurance: &WorkspaceAssurance) -> String {
    let state = match assurance.state {
        AssuranceState::Clean => "clean",
        AssuranceState::Stale => "stale",
        AssuranceState::Pending => "pending",
        AssuranceState::Running => "running",
        AssuranceState::Bounded => "bounded",
        AssuranceState::Unavailable => "unavailable",
        // Deser-only forward-compat fallback (ADR-085): a newer daemon's
        // unrecognised state. Surface it fail-safe, never as clean.
        AssuranceState::Unknown => "unknown",
    };
    // DSV-045: a `Bounded` snapshot carries coverage but no reason; append the
    // scanned/total so the human label conveys the bound.
    if let Some(coverage) = assurance.scan_coverage {
        return format!(
            "{state} ({}/{} files)",
            coverage.scanned_files, coverage.total_files
        );
    }
    match assurance.reason {
        Some(reason) => format!("{state}{{{}}}", stale_reason_str(reason)),
        None => state.to_string(),
    }
}

/// Frozen kebab-case wire string for a [`StaleReason`] (mirrors the proto
/// serialiser; used for the human surface so the label matches the wire).
#[must_use]
pub(crate) fn stale_reason_str(reason: StaleReason) -> &'static str {
    match reason {
        StaleReason::CrossFileResolutionNeeded => "cross-file-resolution-needed",
        StaleReason::Deleted => "deleted",
        StaleReason::Renamed => "renamed",
        StaleReason::SymlinkRetarget => "symlink-retarget",
        StaleReason::ConfigBoundaryPolicyEdit => "config-boundary-policy-edit",
        StaleReason::GitignoreScopeChange => "gitignore-scope-change",
        StaleReason::ImpactSetOverflow => "impact-set-overflow",
        StaleReason::WarmStateEvicted => "warm-state-evicted",
        StaleReason::ScanTimeout => "scan-timeout",
        StaleReason::DaemonAbsent => "daemon-absent",
        StaleReason::UnknownClass => "unknown-class",
        StaleReason::Unknown => "unknown",
    }
}

/// Shared JSON-RPC-over-stream framing for the save-time transports (the Unix
/// socket and the Windows named pipe). Platform-neutral — both transports
/// connect their own way, then hand the connected stream here.
mod framing {
    use std::io::{BufRead, BufReader, Read, Write};

    use serde_json::{Value, json};

    use super::SaveTimeClientError;

    /// Cap the single NDJSON response line so a hostile/buggy daemon cannot make
    /// the client buffer unboundedly. Matches the MCP client's response cap.
    pub(super) const RESPONSE_LINE_BYTES: u64 = 1 << 20;
    pub(super) const REQUEST_ID: &str = "anvil-watch-validate-paths";
    pub(super) const FULL_SCAN_REQUEST_ID: &str = "anvil-watch-request-full-scan";
    pub(super) const WORKSPACE_STATUS_REQUEST_ID: &str = "anvil-status-workspace-status";

    /// Send one JSON-RPC request frame over `stream` and return the parsed
    /// `result` value. A mid-stream drop / EOF / id mismatch / JSON-RPC error all
    /// map to `Unavailable` — the daemon is dead-for-this-batch.
    pub(super) fn round_trip_over<S: Read + Write>(
        mut stream: S,
        method: &str,
        id: &str,
        params: &Value,
    ) -> Result<Value, SaveTimeClientError> {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id,
        });
        writeln!(stream, "{frame}").map_err(|_| SaveTimeClientError::Unavailable)?;
        stream
            .flush()
            .map_err(|_| SaveTimeClientError::Unavailable)?;

        let mut reader = BufReader::new(stream);
        let line = read_capped_line(&mut reader)?;
        let envelope: Value =
            serde_json::from_str(&line).map_err(|_| SaveTimeClientError::Unavailable)?;
        // Reject a response whose id does not match the request id — a stale or
        // misrouted frame must not be accepted as this request's verdict.
        if envelope.get("id").and_then(Value::as_str) != Some(id) {
            return Err(SaveTimeClientError::Unavailable);
        }
        // A JSON-RPC error (incl. -32601 "save-time not enabled") means the
        // daemon cannot serve a verdict ⇒ fall back, same as absence.
        envelope
            .get("result")
            .cloned()
            .ok_or(SaveTimeClientError::Unavailable)
    }

    /// Read a single newline-terminated response line under the byte cap. An
    /// empty read (daemon closed mid-response) or an over-cap / unframed line is
    /// `Unavailable`.
    fn read_capped_line(reader: &mut impl BufRead) -> Result<String, SaveTimeClientError> {
        let mut buf = Vec::new();
        let read = reader
            .by_ref()
            .take(RESPONSE_LINE_BYTES + 1)
            .read_until(b'\n', &mut buf)
            .map_err(|_| SaveTimeClientError::Unavailable)?;
        if read == 0 || buf.len() as u64 > RESPONSE_LINE_BYTES || !buf.ends_with(b"\n") {
            return Err(SaveTimeClientError::Unavailable);
        }
        String::from_utf8(buf).map_err(|_| SaveTimeClientError::Unavailable)
    }
}

#[cfg(unix)]
pub(crate) use socket::SocketSaveTimeTransport;

#[cfg(windows)]
pub(crate) use pipe::WindowsPipeSaveTimeTransport;

#[cfg(unix)]
mod socket {
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use anvil_intercept::ipc;
    use anvil_intercept_proto::protocol::{
        ANVIL_REQUEST_FULL_SCAN, ANVIL_VALIDATE_PATHS, ANVIL_WORKSPACE_STATUS, ChangeDescriptor,
        RequestFullScanRequest, ValidatePathsRequest, ValidatePathsResponse, WorkspaceAssurance,
        WorkspaceStatusRequest, WorkspaceStatusResponse,
    };
    use serde_json::Value;

    use super::framing;
    use super::{SaveTimeClientError, SaveTimeTransport};

    /// Per-request wall-clock budget. A daemon that does not answer within this
    /// window is treated as dead-for-this-batch (scoped fallback) rather than
    /// stalling the watch loop. Matches the MCP client's `DAEMON_REQUEST_TIMEOUT`.
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

    /// Production [`SaveTimeTransport`]: JSON-RPC over the per-user Unix socket.
    pub(crate) struct SocketSaveTimeTransport {
        socket_path: PathBuf,
    }

    impl SocketSaveTimeTransport {
        /// Resolve the canonical per-user socket. `None` when no socket dir can
        /// be resolved (treated by the caller as a permanently-absent daemon).
        pub(crate) fn resolve() -> Option<Self> {
            ipc::resolve_socket_path()
                .ok()
                .map(|socket_path| Self { socket_path })
        }

        #[cfg(test)]
        pub(crate) fn with_socket_path(socket_path: impl Into<PathBuf>) -> Self {
            Self {
                socket_path: socket_path.into(),
            }
        }

        /// Open a validated, peer-checked, timeout-bounded connection. Any
        /// failure maps to `Unavailable` (absent / dead daemon → fallback).
        fn connect(&self) -> Result<UnixStream, SaveTimeClientError> {
            ipc::validate_socket_path_for_client(&self.socket_path)
                .map_err(|_| SaveTimeClientError::Unavailable)?;
            let stream = UnixStream::connect(&self.socket_path)
                .map_err(|_| SaveTimeClientError::Unavailable)?;
            ipc::validate_connected_peer_for_client(&stream)
                .map_err(|_| SaveTimeClientError::Unavailable)?;
            stream
                .set_read_timeout(Some(REQUEST_TIMEOUT))
                .map_err(|_| SaveTimeClientError::Unavailable)?;
            stream
                .set_write_timeout(Some(REQUEST_TIMEOUT))
                .map_err(|_| SaveTimeClientError::Unavailable)?;
            Ok(stream)
        }

        /// Connect and run one JSON-RPC round-trip over the socket, via the
        /// shared `framing` helper.
        fn round_trip(
            &self,
            method: &str,
            id: &str,
            params: &Value,
        ) -> Result<Value, SaveTimeClientError> {
            framing::round_trip_over(self.connect()?, method, id, params)
        }
    }

    impl SaveTimeTransport for SocketSaveTimeTransport {
        fn validate_paths(
            &self,
            workspace_root: &Path,
            paths: &[ChangeDescriptor],
        ) -> Result<ValidatePathsResponse, SaveTimeClientError> {
            let request = ValidatePathsRequest {
                workspace_root: workspace_root.to_string_lossy().into_owned(),
                paths: paths.to_vec(),
            };
            let params =
                serde_json::to_value(&request).map_err(|_| SaveTimeClientError::Unavailable)?;
            let result = self.round_trip(ANVIL_VALIDATE_PATHS, framing::REQUEST_ID, &params)?;
            serde_json::from_value(result).map_err(|_| SaveTimeClientError::Unavailable)
        }

        fn request_full_scan(&self, workspace_root: &Path) -> Result<(), SaveTimeClientError> {
            // Fire-and-forget: the daemon starts the scan on receipt, so the ack
            // is irrelevant and we must not stall the interactive watch loop (up
            // to `REQUEST_TIMEOUT`) waiting for it on the reconnect hot path. A
            // detached thread runs the whole round-trip (connect + send + read)
            // off the watch loop and discards the parsed reply; any failure is
            // best-effort-ignored (the next `validate_paths` still proceeds).
            let request = RequestFullScanRequest {
                workspace_root: workspace_root.to_string_lossy().into_owned(),
            };
            let Ok(params) = serde_json::to_value(&request) else {
                return Ok(());
            };
            let socket_path = self.socket_path.clone();
            std::thread::spawn(move || {
                let transport = SocketSaveTimeTransport { socket_path };
                let _ = transport.round_trip(
                    ANVIL_REQUEST_FULL_SCAN,
                    framing::FULL_SCAN_REQUEST_ID,
                    &params,
                );
            });
            Ok(())
        }

        fn workspace_status(
            &self,
            workspace_root: &Path,
        ) -> Result<WorkspaceAssurance, SaveTimeClientError> {
            let request = WorkspaceStatusRequest {
                workspace_root: workspace_root.to_string_lossy().into_owned(),
            };
            let params =
                serde_json::to_value(&request).map_err(|_| SaveTimeClientError::Unavailable)?;
            let result = self.round_trip(
                ANVIL_WORKSPACE_STATUS,
                framing::WORKSPACE_STATUS_REQUEST_ID,
                &params,
            )?;
            let response: WorkspaceStatusResponse =
                serde_json::from_value(result).map_err(|_| SaveTimeClientError::Unavailable)?;
            Ok(response.workspace_assurance)
        }
    }
}

#[cfg(windows)]
mod pipe {
    use std::path::Path;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use anvil_intercept_proto::protocol::{
        ANVIL_REQUEST_FULL_SCAN, ANVIL_VALIDATE_PATHS, ANVIL_WORKSPACE_STATUS, ChangeDescriptor,
        RequestFullScanRequest, ValidatePathsRequest, ValidatePathsResponse, WorkspaceAssurance,
        WorkspaceStatusRequest, WorkspaceStatusResponse,
    };
    use serde_json::Value;

    use super::framing;
    use super::{SaveTimeClientError, SaveTimeTransport};

    /// Per-request wall-clock budget for the Windows pipe transport.
    /// Matches the Unix `SocketSaveTimeTransport` and the MCP client budget.
    /// A wedged daemon (accepts but never replies) is treated as
    /// Unavailable for this batch (triggers scoped fallback).
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

    /// DSV-011: production [`SaveTimeTransport`] on Windows — JSON-RPC over the
    /// per-user named pipe. The owner-only pipe DACL (`anvil-intercept-win32`) is
    /// the same-user boundary, the SO_PEERCRED analogue; the JSON-RPC framing is
    /// shared with the Unix socket transport via [`framing`].
    ///
    /// Until DSV-010b serves the save-time verbs on Windows the daemon replies
    /// `Method not found` (or "save-time not enabled"), which folds to
    /// `Unavailable` → the watch/status surfaces fall back exactly as they do
    /// against an absent daemon.
    ///
    /// Hardened: the synchronous pipe client now bounds the entire connect+IO
    /// with a 2 s wall-clock cap (worker thread + recv_timeout) so a wedged
    /// daemon cannot stall `watch` or `status`. Mirrors the established
    /// `query_daemon_status_windows_at_with_timeout` pattern; Unix uses
    /// `set_read_timeout` on the stream.
    pub(crate) struct WindowsPipeSaveTimeTransport {
        pipe_name: String,
    }

    impl WindowsPipeSaveTimeTransport {
        /// Resolve the canonical per-user pipe (install-root aware since
        /// CIB-106). `None` when the pipe name cannot be resolved (treated
        /// as a permanently-absent daemon → fallback).
        pub(crate) fn resolve() -> Option<Self> {
            anvil_intercept::ipc::resolve_pipe_name()
                .ok()
                .map(|pipe_name| Self { pipe_name })
        }

        /// MLP2-075 / DSV-011: Windows equivalent of `SocketSaveTimeTransport::with_socket_path`.
        /// Allows tests to bind the save-time client to a per-PID pipe name so the
        /// fixture `IpcListener` never races the real per-user daemon (or other
        /// concurrent test crates) on the same Windows runner.
        #[cfg(test)]
        #[allow(dead_code)]
        pub(crate) fn with_pipe_name(pipe_name: impl Into<String>) -> Self {
            Self {
                pipe_name: pipe_name.into(),
            }
        }

        /// Connect and run one JSON-RPC round-trip over the pipe (bounded).
        /// Uses a worker thread because synchronous named-pipe ReadFile/WriteFile
        /// have no direct timeout setter; the framing (write + capped read) is
        /// executed under the REQUEST_TIMEOUT cap. Timeouts and any IO error
        /// fold to `Unavailable` (triggers the scoped fallback).
        fn round_trip(
            &self,
            method: &str,
            id: &str,
            params: &Value,
        ) -> Result<Value, SaveTimeClientError> {
            let pipe_name = self.pipe_name.clone();
            let method = method.to_owned();
            let id = id.to_owned();
            let params = params.clone();
            let (tx, rx) = mpsc::sync_channel(1);
            let worker = thread::spawn(move || {
                let outcome: Result<Value, SaveTimeClientError> = (|| {
                    let client = anvil_intercept_win32::connect_owner_only_pipe_client(&pipe_name)
                        .map_err(|_| SaveTimeClientError::Unavailable)?;
                    framing::round_trip_over(client, &method, &id, &params)
                })();
                let _ = tx.send(outcome);
            });
            match rx.recv_timeout(REQUEST_TIMEOUT) {
                Ok(outcome) => {
                    // Reap on success path for consistency with the status-query
                    // Windows timeout helper (avoids leaking a JoinHandle when the
                    // worker has already exited). On timeout arms we intentionally
                    // drop without join (the worker is blocked in ReadFile; leak is
                    // bounded by process lifetime, matching the established pattern).
                    let _ = worker.join();
                    outcome
                }
                Err(mpsc::RecvTimeoutError::Timeout) => Err(SaveTimeClientError::Unavailable),
                Err(mpsc::RecvTimeoutError::Disconnected) => Err(SaveTimeClientError::Unavailable),
            }
        }
    }

    impl SaveTimeTransport for WindowsPipeSaveTimeTransport {
        fn validate_paths(
            &self,
            workspace_root: &Path,
            paths: &[ChangeDescriptor],
        ) -> Result<ValidatePathsResponse, SaveTimeClientError> {
            let request = ValidatePathsRequest {
                workspace_root: workspace_root.to_string_lossy().into_owned(),
                paths: paths.to_vec(),
            };
            let params =
                serde_json::to_value(&request).map_err(|_| SaveTimeClientError::Unavailable)?;
            let result = self.round_trip(ANVIL_VALIDATE_PATHS, framing::REQUEST_ID, &params)?;
            serde_json::from_value(result).map_err(|_| SaveTimeClientError::Unavailable)
        }

        fn request_full_scan(&self, workspace_root: &Path) -> Result<(), SaveTimeClientError> {
            // Fire-and-forget on a detached thread (mirrors the socket transport):
            // the daemon starts the scan on receipt, so do not stall the watch loop.
            let request = RequestFullScanRequest {
                workspace_root: workspace_root.to_string_lossy().into_owned(),
            };
            let Ok(params) = serde_json::to_value(&request) else {
                return Ok(());
            };
            let pipe_name = self.pipe_name.clone();
            std::thread::spawn(move || {
                let transport = WindowsPipeSaveTimeTransport { pipe_name };
                let _ = transport.round_trip(
                    ANVIL_REQUEST_FULL_SCAN,
                    framing::FULL_SCAN_REQUEST_ID,
                    &params,
                );
            });
            Ok(())
        }

        fn workspace_status(
            &self,
            workspace_root: &Path,
        ) -> Result<WorkspaceAssurance, SaveTimeClientError> {
            let request = WorkspaceStatusRequest {
                workspace_root: workspace_root.to_string_lossy().into_owned(),
            };
            let params =
                serde_json::to_value(&request).map_err(|_| SaveTimeClientError::Unavailable)?;
            let result = self.round_trip(
                ANVIL_WORKSPACE_STATUS,
                framing::WORKSPACE_STATUS_REQUEST_ID,
                &params,
            )?;
            let response: WorkspaceStatusResponse =
                serde_json::from_value(result).map_err(|_| SaveTimeClientError::Unavailable)?;
            Ok(response.workspace_assurance)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use anvil_intercept_proto::protocol::{
        AssuranceState, Coverage, EvaluatedPath, StaleReason, WorkspaceAssurance,
    };

    use super::*;

    #[test]
    fn warm_up_enqueues_one_scan_when_daemon_accepts() {
        // GCTX-010 C1: a live daemon that accepts the enqueue → Requested, and
        // exactly one `request_full_scan` is issued per warm-up.
        let fake = FakeTransport::new(Vec::new());
        assert_eq!(
            warm_up_with(&fake, Path::new("/ws")),
            WarmupOutcome::Requested,
        );
        assert_eq!(
            fake.full_scan_calls(),
            1,
            "warm-up enqueues exactly one full scan",
        );
    }

    #[test]
    fn warm_up_skips_silently_when_daemon_absent() {
        // GCTX-010 C1: no verdict (absent / dead / unserved verb) → DaemonAbsent,
        // never an error bubbled to the caller and never a panic.
        struct AbsentTransport;
        impl SaveTimeTransport for AbsentTransport {
            fn validate_paths(
                &self,
                _workspace_root: &Path,
                _paths: &[ChangeDescriptor],
            ) -> Result<ValidatePathsResponse, SaveTimeClientError> {
                Err(SaveTimeClientError::Unavailable)
            }

            fn request_full_scan(&self, _workspace_root: &Path) -> Result<(), SaveTimeClientError> {
                Err(SaveTimeClientError::Unavailable)
            }

            fn workspace_status(
                &self,
                _workspace_root: &Path,
            ) -> Result<WorkspaceAssurance, SaveTimeClientError> {
                Err(SaveTimeClientError::Unavailable)
            }
        }

        assert_eq!(
            warm_up_with(&AbsentTransport, Path::new("/ws")),
            WarmupOutcome::DaemonAbsent,
        );
    }

    #[test]
    fn warm_up_plan_respects_opt_out_and_session_dedup() {
        // GCTX-010 C1: `ANVIL_WATCH_DAEMON=0` suppresses all daemon contact,
        // regardless of dedup state.
        assert_eq!(
            plan_warm_up(DaemonRoutingMode::Disabled, false),
            WarmupPlan::Skip(WarmupOutcome::Disabled),
        );
        assert_eq!(
            plan_warm_up(DaemonRoutingMode::Disabled, true),
            WarmupPlan::Skip(WarmupOutcome::Disabled),
        );
        // A root already enqueued this session is suppressed (the on-demand
        // re-warm fires on every miss; this bounds it to one request per root).
        assert_eq!(
            plan_warm_up(DaemonRoutingMode::DefaultOnWhenLive, true),
            WarmupPlan::Skip(WarmupOutcome::AlreadyRequested),
        );
        // A fresh root with daemon routing enabled → attempt the enqueue.
        assert_eq!(
            plan_warm_up(DaemonRoutingMode::DefaultOnWhenLive, false),
            WarmupPlan::Attempt,
        );
        assert_eq!(
            plan_warm_up(DaemonRoutingMode::ForcedOn, false),
            WarmupPlan::Attempt,
        );
    }

    #[test]
    fn session_dedup_marks_then_expires_after_cooldown() {
        use std::time::{Duration, Instant};

        // A unique path so this never collides with other tests that share the
        // process-global warmed map.
        let root = std::env::temp_dir().join("gctx-warmup-dedup-probe-7f3a");
        assert!(!recently_warmed(&root), "a fresh root is not yet warmed");

        mark_warmed(&root);
        assert!(
            recently_warmed(&root),
            "a just-marked root is deduped within the cooldown",
        );

        // A mark older than the cooldown self-corrects: the root is re-warmable.
        // (Backdate directly so the test is deterministic and does not sleep.)
        let stale = Instant::now()
            .checked_sub(WARMUP_DEDUP_COOLDOWN + Duration::from_secs(1))
            .expect("cooldown fits before now");
        session_warmed_roots()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(root.clone(), stale);
        assert!(
            !recently_warmed(&root),
            "a mark older than the cooldown no longer suppresses re-warm",
        );

        // Tidy up the shared map so the probe path leaves no residue.
        session_warmed_roots()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&root);
    }

    #[test]
    fn daemon_routing_unset_defaults_on_when_live() {
        assert_eq!(
            daemon_routing_mode_from(None),
            DaemonRoutingMode::DefaultOnWhenLive,
            "unset ANVIL_WATCH_DAEMON is the v0.8 safe default-on posture",
        );
    }

    #[test]
    fn daemon_routing_false_values_disable() {
        for value in ["0", "false", "off", "no"] {
            assert_eq!(
                daemon_routing_mode_from(Some(std::ffi::OsStr::new(value))),
                DaemonRoutingMode::Disabled,
                "{value:?} must be the documented rollout opt-out",
            );
        }
    }

    #[test]
    fn daemon_routing_true_values_force_on() {
        for value in ["1", "true", "on", "yes"] {
            assert_eq!(
                daemon_routing_mode_from(Some(std::ffi::OsStr::new(value))),
                DaemonRoutingMode::ForcedOn,
                "{value:?} must preserve the old explicit opt-in behaviour",
            );
        }
    }

    #[test]
    fn daemon_routing_unrecognized_values_fall_back_to_unset_default() {
        // An empty value (`ANVIL_WATCH_DAEMON=`), whitespace, or any string that
        // is neither a documented false nor true token carries no explicit
        // opinion, so it resolves to the same posture as unset — the safe
        // DefaultOnWhenLive default, never a silent disable. Operators opt out
        // with an explicit false value, not by blanking.
        for value in ["", "  ", "maybe", "default", "2"] {
            assert_eq!(
                daemon_routing_mode_from(Some(std::ffi::OsStr::new(value))),
                DaemonRoutingMode::DefaultOnWhenLive,
                "{value:?} carries no opt-out/force opinion → unset default",
            );
        }
    }

    #[test]
    fn daemon_routing_value_matching_is_case_insensitive() {
        assert_eq!(
            daemon_routing_mode_from(Some(std::ffi::OsStr::new("FALSE"))),
            DaemonRoutingMode::Disabled,
        );
        assert_eq!(
            daemon_routing_mode_from(Some(std::ffi::OsStr::new("On"))),
            DaemonRoutingMode::ForcedOn,
        );
    }

    /// Scripted fake: each `validate_paths` call pops the next outcome; if the
    /// script is exhausted it returns a clean verdict. Records the call counts so
    /// the reconnect/re-scan contract can be asserted.
    struct FakeTransport {
        outcomes:
            Mutex<std::collections::VecDeque<Result<ValidatePathsResponse, SaveTimeClientError>>>,
        validate_calls: Mutex<usize>,
        full_scan_calls: Mutex<usize>,
    }

    impl FakeTransport {
        fn new(outcomes: Vec<Result<ValidatePathsResponse, SaveTimeClientError>>) -> Self {
            Self {
                outcomes: Mutex::new(outcomes.into_iter().collect()),
                validate_calls: Mutex::new(0),
                full_scan_calls: Mutex::new(0),
            }
        }

        fn full_scan_calls(&self) -> usize {
            *self.full_scan_calls.lock().unwrap()
        }

        fn validate_calls(&self) -> usize {
            *self.validate_calls.lock().unwrap()
        }
    }

    impl SaveTimeTransport for FakeTransport {
        fn validate_paths(
            &self,
            _workspace_root: &Path,
            _paths: &[ChangeDescriptor],
        ) -> Result<ValidatePathsResponse, SaveTimeClientError> {
            *self.validate_calls.lock().unwrap() += 1;
            self.outcomes
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(clean_response()))
        }

        fn request_full_scan(&self, _workspace_root: &Path) -> Result<(), SaveTimeClientError> {
            *self.full_scan_calls.lock().unwrap() += 1;
            Ok(())
        }

        fn workspace_status(
            &self,
            _workspace_root: &Path,
        ) -> Result<WorkspaceAssurance, SaveTimeClientError> {
            Ok(clean_response().workspace_assurance)
        }
    }

    fn clean_response() -> ValidatePathsResponse {
        ValidatePathsResponse {
            diagnostics: Vec::new(),
            evaluated: vec![EvaluatedPath {
                path: "src/a.ts".into(),
                content_hash: Some("hash".into()),
            }],
            workspace_assurance: WorkspaceAssurance {
                state: AssuranceState::Clean,
                reason: None,
                generation: 1,
                last_full_scan: None,
                scan_coverage: None,
            },
            coverage: Coverage::Certified,
            check_families: vec![anvil_intercept_proto::protocol::CheckFamily::Antipattern],
        }
    }

    fn client_with(
        outcomes: Vec<Result<ValidatePathsResponse, SaveTimeClientError>>,
    ) -> (WatchSaveTimeClient, std::sync::Arc<FakeTransport>) {
        // The client owns the transport, but tests need to read its call
        // counters afterwards. Wrap the fake in an `Arc` and hand the client a
        // thin forwarder so both share one set of counters.
        let fake = std::sync::Arc::new(FakeTransport::new(outcomes));
        let forwarder = ArcTransport(std::sync::Arc::clone(&fake));
        (
            WatchSaveTimeClient::new(Box::new(forwarder), PathBuf::from("/ws")),
            fake,
        )
    }

    struct ArcTransport(std::sync::Arc<FakeTransport>);

    impl SaveTimeTransport for ArcTransport {
        fn validate_paths(
            &self,
            workspace_root: &Path,
            paths: &[ChangeDescriptor],
        ) -> Result<ValidatePathsResponse, SaveTimeClientError> {
            self.0.validate_paths(workspace_root, paths)
        }

        fn request_full_scan(&self, workspace_root: &Path) -> Result<(), SaveTimeClientError> {
            self.0.request_full_scan(workspace_root)
        }

        fn workspace_status(
            &self,
            workspace_root: &Path,
        ) -> Result<WorkspaceAssurance, SaveTimeClientError> {
            self.0.workspace_status(workspace_root)
        }
    }

    #[test]
    fn watch_routes_to_validate_paths_when_daemon_present() {
        let (mut client, fake) = client_with(vec![Ok(clean_response())]);

        let decision = client.validate(vec!["/ws/src/a.ts".into()]);

        assert!(
            matches!(decision, SaveTimeDecision::Validated(_)),
            "a present daemon must route to validate_paths, got {decision:?}",
        );
        assert_eq!(fake.validate_calls(), 1, "the daemon must be consulted");
        assert_eq!(
            fake.full_scan_calls(),
            0,
            "no reconnect on the first successful cycle ⇒ no full scan",
        );
    }

    /// GV2-028: the user-facing `anvil watch` path drives a **real** daemon —
    /// backed by the real tree-sitter [`KernelSymbolParser`] — to a `Certified`
    /// verdict, proving the parser feed is live in production end to end, not an
    /// uncalled library. This joins the two halves the other proofs cover
    /// separately: the daemon-side `real_parser_certifies_repeat_save_through_daemon`
    /// exercises `SaveTimeConn` directly, and the `FakeTransport` tests above
    /// exercise the client against a canned verdict. Here `WatchSaveTimeClient`
    /// (with its `classify_change`) drives a real `SaveTimeConn` over the real
    /// parser: a cold first save warms the graph (`Partial`), and a
    /// self-contained re-save certifies through the client.
    #[cfg(unix)]
    #[test]
    fn watch_client_certifies_through_real_daemon_parser() {
        use anvil_checks::antipattern::types::AntipatternCheckConfig;
        use anvil_intercept::confinement::Confinement;
        use anvil_intercept::ipc::SaveTimeDispatch;
        use anvil_intercept::save_time::{SaveTimeConn, SaveTimeState};
        use anvil_intercept::workspace_pool::WorkScheduler;
        use anvil_intercept_proto::protocol::ValidatePathsRequest;

        /// A `SaveTimeTransport` that speaks to a real in-process daemon instead
        /// of a socket: each call opens a `SaveTimeConn` over a shared
        /// `SaveTimeState` (so the warm graph persists across saves) backed by
        /// the real `KernelSymbolParser`. `Err(SaveTimeError)` maps to the same
        /// `Unavailable` the socket transport would surface.
        struct RealDaemonTransport {
            state: SaveTimeState,
        }

        impl SaveTimeTransport for RealDaemonTransport {
            fn validate_paths(
                &self,
                workspace_root: &Path,
                paths: &[ChangeDescriptor],
            ) -> Result<ValidatePathsResponse, SaveTimeClientError> {
                let request = ValidatePathsRequest {
                    workspace_root: workspace_root.to_string_lossy().into_owned(),
                    paths: paths.to_vec(),
                };
                let mut conn = SaveTimeConn::new(&self.state);
                conn.validate_paths(&request)
                    .map_err(|_| SaveTimeClientError::Unavailable)
            }

            fn request_full_scan(&self, _workspace_root: &Path) -> Result<(), SaveTimeClientError> {
                Ok(())
            }

            fn workspace_status(
                &self,
                _workspace_root: &Path,
            ) -> Result<WorkspaceAssurance, SaveTimeClientError> {
                Err(SaveTimeClientError::Unavailable)
            }
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let src = tmp.path().join("src");
        std::fs::create_dir(&src).expect("mkdir");
        std::fs::write(src.join("a.ts"), b"export function foo() { return 1; }").expect("write");

        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        )
        .with_parser(std::sync::Arc::new(
            crate::intercept_symbol_parser::KernelSymbolParser::new(),
        ));
        let mut client = WatchSaveTimeClient::new(
            Box::new(RealDaemonTransport { state }),
            tmp.path().to_path_buf(),
        );
        // The watch worker hands absolute paths; the client classifies them
        // root-relative before they reach the daemon.
        let changed = vec![src.join("a.ts").to_string_lossy().into_owned()];

        // First save: cold cache ⇒ Partial, but it warms the graph with foo.
        let first = client.validate(changed.clone());
        let SaveTimeDecision::Validated(first) = first else {
            panic!("a present real daemon must route to validate_paths, got {first:?}");
        };
        assert_eq!(
            first.coverage,
            Coverage::Partial,
            "cold first save through the real parser is Partial",
        );

        // Second save of the same clean body: self-contained ⇒ Certified,
        // proving the watch client surfaces a real certified verdict end to end.
        let second = client.validate(changed);
        let SaveTimeDecision::Validated(second) = second else {
            panic!("the warm re-save must still route to the daemon, got {second:?}");
        };
        assert_eq!(
            second.coverage,
            Coverage::Certified,
            "a self-contained re-save certifies through `anvil watch` → real daemon → real parser",
        );
    }

    #[test]
    fn watch_fallback_reports_unavailable_not_clean() {
        let (mut client, _fake) = client_with(vec![Err(SaveTimeClientError::Unavailable)]);

        let decision = client.validate(vec!["/ws/src/a.ts".into()]);

        let SaveTimeDecision::FellBack { assurance, .. } = decision else {
            panic!("daemon-absent must fall back, got {decision:?}");
        };
        assert_eq!(
            assurance.state,
            AssuranceState::Unavailable,
            "fallback must report unavailable, never a truncated clean",
        );
        assert_eq!(
            assurance.reason,
            Some(StaleReason::DaemonAbsent),
            "an unavailable snapshot must carry daemon-absent",
        );
    }

    #[test]
    fn first_fallback_warns_once() {
        let (mut client, _fake) = client_with(vec![
            Err(SaveTimeClientError::Unavailable),
            Err(SaveTimeClientError::Unavailable),
        ]);

        let first = client.validate(vec!["/ws/src/a.ts".into()]);
        let second = client.validate(vec!["/ws/src/b.ts".into()]);

        assert!(
            matches!(first, SaveTimeDecision::FellBack { warned: true, .. }),
            "the first fallback of a disconnect must warn",
        );
        assert!(
            matches!(second, SaveTimeDecision::FellBack { warned: false, .. }),
            "a second consecutive fallback must not warn again",
        );
    }

    #[test]
    fn daemon_death_midsession_falls_back_scoped() {
        // A verdict, then the daemon dies mid-session (its in-flight response is
        // truncated ⇒ Unavailable). The death cycle must fall back scoped to the
        // changed paths and report unavailable — not a stale cached clean.
        let (mut client, _fake) = client_with(vec![
            Ok(clean_response()),
            Err(SaveTimeClientError::Unavailable),
        ]);

        let alive = client.validate(vec!["/ws/src/a.ts".into()]);
        let dead = client.validate(vec!["/ws/src/b.ts".into()]);

        assert!(matches!(alive, SaveTimeDecision::Validated(_)));
        let SaveTimeDecision::FellBack {
            assurance,
            scoped_paths,
            ..
        } = dead
        else {
            panic!("a mid-session death must fall back, got {dead:?}");
        };
        assert_eq!(assurance.state, AssuranceState::Unavailable);
        assert_eq!(
            scoped_paths,
            vec!["/ws/src/b.ts".to_string()],
            "the fallback must scope to exactly the changed paths (never --all)",
        );
    }

    #[test]
    fn reconnect_reissues_full_scan() {
        // Disconnect, then the daemon returns. The reconnect must re-issue
        // request_full_scan so assurance re-establishes from stale.
        let (mut client, fake) = client_with(vec![
            Err(SaveTimeClientError::Unavailable),
            Ok(clean_response()),
        ]);

        client.validate(vec!["/ws/src/a.ts".into()]);
        assert_eq!(fake.full_scan_calls(), 0, "no scan while disconnected");

        let reconnected = client.validate(vec!["/ws/src/a.ts".into()]);
        assert!(matches!(reconnected, SaveTimeDecision::Validated(_)));
        assert_eq!(
            fake.full_scan_calls(),
            1,
            "reconnect must re-issue exactly one request_full_scan",
        );
    }

    /// Prove the production socket transport actually connects, frames a
    /// `validate_paths` request, reads the reply, and maps a daemon that does
    /// not serve save-time (a `NoopDispatcher` listener answers `-32601`) to
    /// `Unavailable` — i.e. the real wire degrades to the scoped fallback rather
    /// than panicking or hanging. Runs on Linux **and macOS** — macOS is a
    /// short-term-supported save-time target and the `cfg(unix)` transport runs
    /// there, so the round-trip is exercised on both (matching the MCP parity
    /// test `live_daemon_mcp_tool_call_matches_embedded_diagnostic_envelope`,
    /// which is `any(linux, macos)`). This is a round-trip parity check, not a
    /// timing assertion, so it does not inherit the Darwin `UnixStream`-timeout
    /// caveat that keeps `mcp::validation`'s *timing* tests Linux-only.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn socket_transport_maps_unserved_daemon_to_unavailable() {
        use std::os::unix::fs::PermissionsExt;

        use anvil_intercept::Shutdown;
        use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
        use tokio::runtime::Runtime;

        use super::{SaveTimeTransport, SocketSaveTimeTransport};

        let runtime = Runtime::new().expect("tokio runtime starts");
        let dir = tempfile::tempdir().expect("runtime dir exists");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("runtime dir permissions tightened");
        let socket = dir.path().join("intercept.sock");
        let _guard = runtime.enter();
        let listener = IpcListener::bind(&socket, NoopDispatcher).expect("daemon socket binds");
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        let transport = SocketSaveTimeTransport::with_socket_path(&socket);
        let outcome = transport.validate_paths(Path::new("/ws"), &[]);

        shutdown.trigger();
        runtime.block_on(async {
            server
                .await
                .expect("daemon task joins")
                .expect("daemon exits cleanly");
        });

        assert_eq!(
            outcome,
            Err(SaveTimeClientError::Unavailable),
            "a daemon that does not serve save-time must degrade to the scoped fallback",
        );
    }

    #[test]
    fn warn_once_latch_resets_after_reconnect() {
        // Disconnect (warn), reconnect, disconnect again (must warn again).
        let (mut client, _fake) = client_with(vec![
            Err(SaveTimeClientError::Unavailable),
            Ok(clean_response()),
            Err(SaveTimeClientError::Unavailable),
        ]);

        let first = client.validate(vec!["/ws/src/a.ts".into()]);
        let _reconnect = client.validate(vec!["/ws/src/a.ts".into()]);
        let third = client.validate(vec!["/ws/src/a.ts".into()]);

        assert!(
            matches!(first, SaveTimeDecision::FellBack { warned: true, .. }),
            "first disconnect warns",
        );
        assert!(
            matches!(third, SaveTimeDecision::FellBack { warned: true, .. }),
            "a fresh disconnect after a reconnect must warn again",
        );
    }

    /// DSV-011: end-to-end proof that the Windows named-pipe transport
    /// round-trips a `validate_paths` against a local in-process listener
    /// (NoopDispatcher → method-not-found → Unavailable fallback). Mirrors
    /// the Unix socket test immediately above and the MLP2-075 Windows
    /// pattern (per-PID pipe name so the fixture never collides with a
    /// real per-user daemon on the same runner).
    ///
    /// This is the Windows leg of "the watch socket round-trip + status
    /// render tests extended to a Windows named-pipe fixture".
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_pipe_transport_maps_unserved_daemon_to_unavailable() {
        use anvil_intercept::Shutdown;
        use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
        use tokio::runtime::Runtime;

        use super::WindowsPipeSaveTimeTransport;

        let runtime = Runtime::new().expect("tokio runtime starts");
        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-save-time-test-{}",
            std::process::id()
        );
        let _guard = runtime.enter();
        let listener = IpcListener::bind(&pipe_name, NoopDispatcher).expect("daemon pipe binds");
        let (shutdown, token) = Shutdown::new();
        let server = runtime.spawn(listener.serve(token));

        let transport = WindowsPipeSaveTimeTransport::with_pipe_name(&pipe_name);
        let outcome = transport.validate_paths(std::path::Path::new(r"C:\ws"), &[]);

        shutdown.trigger();
        runtime.block_on(async {
            server
                .await
                .expect("daemon task joins")
                .expect("daemon exits cleanly");
        });

        assert_eq!(
            outcome,
            Err(SaveTimeClientError::Unavailable),
            "a daemon that does not serve save-time must degrade to the scoped fallback",
        );
    }
}
