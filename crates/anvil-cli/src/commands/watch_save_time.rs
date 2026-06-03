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

/// True when the user-facing surfaces (`watch` routing, `status` assurance)
/// should engage the resident save-time daemon (DSV-007). Opt-in and
/// default-off: the save-time daemon is not auto-started in Sub-phase A, so
/// enabling by default would change default `watch`/`status` output for every
/// user against an absent daemon. Operators / CI running the daemon set
/// `ANVIL_WATCH_DAEMON=1` (also `true`/`on`/`yes`).
pub(crate) fn daemon_routing_enabled() -> bool {
    std::env::var_os("ANVIL_WATCH_DAEMON").is_some_and(|value| {
        matches!(
            value.to_string_lossy().as_ref(),
            "1" | "true" | "on" | "yes"
        )
    })
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

#[cfg(not(unix))]
pub(crate) fn query_workspace_status(_workspace_root: &Path) -> Option<WorkspaceAssurance> {
    None
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
        let descriptors: Vec<ChangeDescriptor> = changed_paths
            .iter()
            .map(|p| classify_change(p, &self.workspace_root))
            .collect();

        match self
            .transport
            .validate_paths(&self.workspace_root, &descriptors)
        {
            Ok(response) => {
                if !self.connected {
                    // Reconnect: re-establish the baseline so assurance comes
                    // back from `stale`, not a pre-disconnect `clean`. Best
                    // effort — a failed re-scan must not block this verdict.
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
    }
}

/// Classify one changed path into a wire [`ChangeDescriptor`]. The daemon
/// re-derives identity from disk and never trusts these hints for a verdict, so
/// the classification is coarse: a path that still exists on disk is `Modified`,
/// one that no longer does is `Deleted`. `content_hash`/`mtime` are left unset —
/// watch has no cheaper-than-the-daemon hint to offer.
fn classify_change(absolute_path: &str, workspace_root: &Path) -> ChangeDescriptor {
    let abs = Path::new(absolute_path);
    let relative = abs
        .strip_prefix(workspace_root)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/");
    let change = if abs.exists() {
        ChangeKindWire::Modified
    } else {
        ChangeKindWire::Deleted
    };
    ChangeDescriptor {
        path: relative,
        change,
        content_hash: None,
        mtime: None,
    }
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
        AssuranceState::Unavailable => "unavailable",
    };
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

#[cfg(unix)]
pub(crate) use socket::SocketSaveTimeTransport;

#[cfg(unix)]
mod socket {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use anvil_intercept::ipc;
    use anvil_intercept_proto::protocol::{
        ANVIL_REQUEST_FULL_SCAN, ANVIL_VALIDATE_PATHS, ANVIL_WORKSPACE_STATUS, ChangeDescriptor,
        RequestFullScanRequest, ValidatePathsRequest, ValidatePathsResponse, WorkspaceAssurance,
        WorkspaceStatusRequest, WorkspaceStatusResponse,
    };
    use serde_json::{Value, json};

    use super::{SaveTimeClientError, SaveTimeTransport};

    /// Per-request wall-clock budget. A daemon that does not answer within this
    /// window is treated as dead-for-this-batch (scoped fallback) rather than
    /// stalling the watch loop. Matches the MCP client's `DAEMON_REQUEST_TIMEOUT`.
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
    /// Cap the single NDJSON response line so a hostile/buggy daemon cannot make
    /// watch buffer unboundedly. Matches the MCP client's response cap.
    const RESPONSE_LINE_BYTES: u64 = 1 << 20;
    const REQUEST_ID: &str = "anvil-watch-validate-paths";
    const FULL_SCAN_REQUEST_ID: &str = "anvil-watch-request-full-scan";
    const WORKSPACE_STATUS_REQUEST_ID: &str = "anvil-status-workspace-status";

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

        /// Send one JSON-RPC request frame and return the parsed `result` value.
        /// A mid-stream drop / EOF / timeout / JSON-RPC error all map to
        /// `Unavailable` — the daemon is dead-for-this-batch.
        fn round_trip(
            &self,
            method: &str,
            id: &str,
            params: &Value,
        ) -> Result<Value, SaveTimeClientError> {
            let mut stream = self.connect()?;
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
            // A JSON-RPC error (incl. -32601 "save-time not enabled") means the
            // daemon cannot serve a verdict ⇒ fall back, same as absence.
            envelope
                .get("result")
                .cloned()
                .ok_or(SaveTimeClientError::Unavailable)
        }
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
            let result = self.round_trip(ANVIL_VALIDATE_PATHS, REQUEST_ID, &params)?;
            serde_json::from_value(result).map_err(|_| SaveTimeClientError::Unavailable)
        }

        fn request_full_scan(&self, workspace_root: &Path) -> Result<(), SaveTimeClientError> {
            let request = RequestFullScanRequest {
                workspace_root: workspace_root.to_string_lossy().into_owned(),
            };
            let params =
                serde_json::to_value(&request).map_err(|_| SaveTimeClientError::Unavailable)?;
            self.round_trip(ANVIL_REQUEST_FULL_SCAN, FULL_SCAN_REQUEST_ID, &params)
                .map(|_| ())
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
            let result =
                self.round_trip(ANVIL_WORKSPACE_STATUS, WORKSPACE_STATUS_REQUEST_ID, &params)?;
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
    /// than panicking or hanging. Linux-gated to match the IPC fixture tests in
    /// `mcp::validation`.
    #[cfg(target_os = "linux")]
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
}
