//! INTD-002: NDJSON IPC listener.
//!
//! The daemon listens on a Unix domain socket (Linux/macOS) or a named
//! pipe (Windows) and parses one JSON envelope per line. This module
//! owns:
//!
//! - **Path resolution** — `$XDG_RUNTIME_DIR/anvil` (else
//!   `$HOME/.local/state/anvil`) on Unix; `\\.\pipe\anvil-intercept-<sid>`
//!   on Windows. A non-empty `ANVIL_HOME` re-roots both (DISTRIB-006 /
//!   CIB-106): the Unix socket dir moves under the prefix, the Windows
//!   pipe name gains a hashed install-root suffix. The launcher
//!   (DRVR-001) reads the same algorithm.
//! - **Permission pinning** — symlink refusal, owner-and-mode checks,
//!   `0700` directories, `0600` socket files. None of this is left to
//!   umask.
//! - **NDJSON framing** — a custom line reader (see [`read_one_line`])
//!   so the per-line cap is enforced byte-by-byte before UTF-8
//!   conversion. Malformed lines are logged and skipped without
//!   tearing the connection down. The stock
//!   `tokio::io::AsyncBufReadExt::lines()` API has no size cap, which
//!   is why the listener does not use it.
//! - **Per-connection task spawning** — handlers go on a `JoinSet` so
//!   shutdown can drain them with a bounded deadline.
//!
//! Session-state mutation lives behind the
//! [`registry::SessionDispatcher`](crate::registry::SessionDispatcher)
//! trait. The listener parameterises over it so tests can substitute a
//! recording double and the daemon can plug the concrete
//! [`SessionRegistry`](crate::registry::SessionRegistry) without
//! touching the listener body. There is exactly one dispatcher trait
//! across the crate — keeping it in `registry` (rather than duplicating
//! it here) avoids the wire surface and the registry surface drifting.
//!
//! See `plans/modules/intercept-daemon.aps.md` INTD-002 for the
//! end-to-end pinning council review M8 demanded.

use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anvil_intercept_proto::protocol::{
    GctxAffectedTestsRequest, GctxAffectedTestsResponse, GctxFindCallersRequest,
    GctxFindCallersResponse, GctxFindDependentsRequest, GctxFindDependentsResponse,
    GctxGetSnippetRequest, GctxGetSnippetResponse, GctxGraphEdgesRequest, GctxGraphEdgesResponse,
    GctxGraphStatsRequest, GctxGraphStatsResponse, GctxImpactOfChangeRequest,
    GctxImpactOfChangeResponse, GctxSearchSymbolsRequest, GctxSearchSymbolsResponse,
    GctxSymbolContextRequest, GctxSymbolContextResponse, RequestFullScanRequest,
    RequestFullScanResponse, ValidatePathsRequest, ValidatePathsResponse, WitnessAppendRequest,
    WitnessAppendResponse, WorkspaceStatusRequest, WorkspaceStatusResponse,
};
use anvil_intercept_proto::{IpcCommand, IpcEnvelope};
use anvil_observability::{TraceContext, bind_traceparent_to_span};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::task::JoinSet;
use tracing::{Instrument, field};

#[cfg(any(unix, windows))]
use crate::save_time::{SaveTimeConn, SaveTimeState};

/// DSV-005: why a save-time verb could not be honoured for a `workspace_root`.
/// Defined cross-platform so the dispatch signature is uniform.
#[derive(Debug)]
pub enum SaveTimeError {
    /// The root is not admitted on this connection (allowlist refusal, or a
    /// root that no longer resolves). Maps to a `workspace-not-admitted` reply.
    ///
    /// The carried `root`/`allow_entries` are for the **server-side** warn only
    /// (an operator's everyday Allowlist diagnostic) and are never placed on the
    /// wire — the reply stays a static, path-free `workspace-not-admitted`
    /// (N5 / CIB-091b: no path detail leaves the daemon).
    NotAdmitted {
        /// The refused workspace root (server-side log only).
        root: PathBuf,
        /// Configured allow-entry count at refusal time (`0` ⇒ empty allow-list,
        /// fail-closed).
        allow_entries: usize,
    },
    /// CIB-154: the connection is already at its per-connection admitted-root
    /// budget and the named root is an as-yet-unadmitted (but otherwise
    /// admissible) distinct root. Distinct from [`Self::NotAdmitted`] so a peer
    /// probing the descriptor-exhaustion vector gets an unambiguous structured
    /// signal rather than a silent/ambiguous allowlist-style refusal.
    ///
    /// Like `NotAdmitted`, the carried `root`/`budget` are for the **server-side**
    /// warn only and never placed on the wire — the reply stays a static,
    /// path-free `workspace-root-budget-exceeded` (N5 / CIB-091b: no path detail
    /// leaves the daemon).
    RootBudgetExceeded {
        /// The refused workspace root (server-side log only).
        root: PathBuf,
        /// The per-connection root budget in force at refusal time.
        budget: usize,
    },
    /// The admitted root's anchor could not be opened. Maps to an internal error.
    Io(std::io::Error),
}

/// GCTX-010 / ADR-084: the read-only assistant-context verb surface, kept a
/// **separate trait** from [`SaveTimeDispatch`] so GCTX queries never sit on the
/// enforcement (`validate_paths`) hot path. It is a supertrait of
/// `SaveTimeDispatch` purely as a wiring convenience: the same per-connection
/// [`SaveTimeConn`] answers both, so the existing `&mut dyn SaveTimeDispatch`
/// the dispatch loop holds can serve a GCTX read without a second threaded
/// trait object. (Re-evaluate this coupling in Phase 2, when GCTX gains its own
/// service surface.) A listener with no save-time state (no admitted roots / no
/// warm cache) replies `Method not found`, which the MCP consumer treats as
/// `Unavailable`.
pub trait GctxDispatch: Send {
    /// Project an identity-only symbol search from the warm graph (daemon-side
    /// CE-5 projection). The `workspace_root` is admitted against this
    /// connection's admitted-root set (ADR-084 C3) before any read.
    ///
    /// Degradation (warming / cold / absent graph) is carried **in-band** in the
    /// response's `outcome` alongside the assurance snapshot (CE-7), so the only
    /// `Err` returns are connection-level: a refused root or an anchor IO error.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn search_symbols(
        &mut self,
        request: &GctxSearchSymbolsRequest,
    ) -> Result<GctxSearchSymbolsResponse, SaveTimeError>;

    /// Project a file-keyed, depth-bounded dependents (reverse-impact) traversal
    /// from the warm dependency graph (daemon-side CE-5 projection). The
    /// `workspace_root` is admitted against this connection's admitted-root set
    /// (ADR-084 C3) before any read.
    ///
    /// Like [`Self::search_symbols`], degradation rides in-band in the response
    /// `outcome` alongside the assurance snapshot (CE-7); the only `Err` returns
    /// are connection-level (a refused root or an anchor IO error).
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn find_dependents(
        &mut self,
        request: &GctxFindDependentsRequest,
    ) -> Result<GctxFindDependentsResponse, SaveTimeError>;

    /// Project an identity-only, depth-bounded **caller** traversal (the symbols
    /// that call the target) from the warm symbol graph (daemon-side CE-5
    /// projection, GCTX-014). The `workspace_root` is admitted against this
    /// connection's admitted-root set (ADR-084 C3) before any read.
    ///
    /// Like [`Self::find_dependents`], degradation rides in-band in the response
    /// `outcome` (CE-7); the only `Err` returns are connection-level.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn find_callers(
        &mut self,
        request: &GctxFindCallersRequest,
    ) -> Result<GctxFindCallersResponse, SaveTimeError>;

    /// Project an identity-only impact-of-change report (affected symbols +
    /// dependent-file closure + heuristic known tests) from the warm graph pair
    /// for a set of changed file paths (daemon-side CE-5 projection). The
    /// `workspace_root` is admitted against this connection's admitted-root set
    /// (ADR-084 C3) before any read.
    ///
    /// Like the sibling GCTX verbs, degradation rides in-band in the response
    /// `outcome` (CE-7); the only `Err` returns are connection-level (a refused
    /// root or an anchor IO error).
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn impact_of_change(
        &mut self,
        request: &GctxImpactOfChangeRequest,
    ) -> Result<GctxImpactOfChangeResponse, SaveTimeError>;

    /// Project an identity-only affected-tests report (likely-relevant test files
    /// with evidence edges + coverage gaps) from the warm graph for a set of
    /// changed file paths (daemon-side CE-5 projection). The `workspace_root` is
    /// admitted against this connection's admitted-root set (ADR-084 C3) before
    /// any read.
    ///
    /// Like the sibling GCTX verbs, degradation rides in-band in the response
    /// `outcome` (CE-7); the only `Err` returns are connection-level (a refused
    /// root or an anchor IO error).
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn affected_tests(
        &mut self,
        request: &GctxAffectedTestsRequest,
    ) -> Result<GctxAffectedTestsResponse, SaveTimeError>;

    /// Project workspace-wide graph counts (GCTX-030 `graph://stats`): resident
    /// symbols, symbol-graph edges, files, and dependency edges. Counts-only, so
    /// the response is trivially CE-5-safe. The `workspace_root` is admitted
    /// against this connection's admitted-root set (ADR-084 C3) before any read.
    ///
    /// Like the sibling GCTX verbs, degradation rides in-band in the response
    /// `outcome` (CE-7); the only `Err` returns are connection-level.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn graph_stats(
        &mut self,
        request: &GctxGraphStatsRequest,
    ) -> Result<GctxGraphStatsResponse, SaveTimeError>;

    /// Project an identity-only, paginated edge enumeration (GCTX-030
    /// `graph://edges`): `(from, to, edge_type)` summaries over the resident
    /// symbol graph, optionally filtered to one source file (daemon-side CE-5
    /// projection). The `workspace_root` is admitted against this connection's
    /// admitted-root set (ADR-084 C3) before any read.
    ///
    /// Like the sibling GCTX verbs, degradation rides in-band in the response
    /// `outcome` (CE-7); the only `Err` returns are connection-level.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn graph_edges(
        &mut self,
        request: &GctxGraphEdgesRequest,
    ) -> Result<GctxGraphEdgesResponse, SaveTimeError>;

    /// Extract a bounded source snippet for a single symbol (GCTX-021, ADR-084 /
    /// PV-9). The `workspace_root` is admitted against this connection's
    /// admitted-root set (ADR-084 C3 / CE-8) before any read, and the span bytes
    /// are read inside that root. Source text rides only under the CE-1 gates
    /// (`gctx.egress` flag + the request capability) after the CE-2 secret scan /
    /// CE-3 path filter / CE-7 freshness check; otherwise the outcome is an
    /// identity-only location. Degradation rides in-band in the response `outcome`
    /// (CE-7); the only `Err` returns are connection-level.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn get_snippet(
        &mut self,
        request: &GctxGetSnippetRequest,
    ) -> Result<GctxGetSnippetResponse, SaveTimeError>;

    /// Project a bounded symbol-context slice (GCTX-023, ADR-084 / PV-9). The
    /// `workspace_root` is admitted before any read. Source text rides only under
    /// the CE-1 gates after CE-2/CE-3/CE-7; degradation rides in-band.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn symbol_context(
        &mut self,
        request: &GctxSymbolContextRequest,
    ) -> Result<GctxSymbolContextResponse, SaveTimeError>;
}

/// DSV-005: the save-time verb surface the JSON-RPC dispatch arm routes to.
/// Implemented by [`crate::save_time::SaveTimeConn`] on both Unix and Windows
/// (DSV-010b); a listener without a wired [`SaveTimeState`] still replies
/// `Method not found` (tests, embedded callers).
///
/// `Send` so the per-connection trait object can be held across the connection
/// handler's `.await` points (the handler future is spawned on a tokio
/// `JoinSet`, which requires `Send`). `SaveTimeConn` satisfies this: its shared
/// state is `Sync` and its admitted-root set is `Send`.
pub trait SaveTimeDispatch: GctxDispatch + Send {
    /// Record the session/worktree pair that this authenticated connection
    /// registered. DSV-044 producers use it as `originating_session_id` only
    /// when the transition root matches the registered worktree.
    fn set_originating_session(&mut self, _session_id: &str, _worktree: &Path) {}

    /// Certify a change set, returning the verdict-shaped response.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when an admissible root cannot be opened.
    fn validate_paths(
        &mut self,
        request: &ValidatePathsRequest,
    ) -> Result<ValidatePathsResponse, SaveTimeError>;

    /// Report the read-only workspace-assurance snapshot.
    ///
    /// # Errors
    /// As for [`Self::validate_paths`].
    fn workspace_status(
        &mut self,
        request: &WorkspaceStatusRequest,
    ) -> Result<WorkspaceStatusResponse, SaveTimeError>;

    /// Queue a full scan and return the post-request assurance snapshot.
    ///
    /// # Errors
    /// As for [`Self::validate_paths`].
    fn request_full_scan(
        &mut self,
        request: &RequestFullScanRequest,
    ) -> Result<RequestFullScanResponse, SaveTimeError>;

    /// Append a witness line to the admitted root's chain (MLP2-005). The daemon
    /// derives `(seq, prev_line_hash)` and appends atomically via
    /// `WitnessWriter::append_chained`; the append outcome (appended / chain
    /// broken / write failed) rides in the response, not as an `Err`.
    ///
    /// # Errors
    /// [`SaveTimeError::NotAdmitted`] when the root is refused;
    /// [`SaveTimeError::Io`] when the admitted root cannot be canonicalised.
    fn witness_append(
        &mut self,
        request: &WitnessAppendRequest,
    ) -> Result<WitnessAppendResponse, SaveTimeError>;

    /// DPO-001: the save-time `gate_evaluated` emitter wired on the shared
    /// state, if any. Defaulted to `None` so test / embedded dispatch impls
    /// auto-satisfy the trait; the production `SaveTimeConn` overrides it to
    /// delegate to the shared state. The IPC `validate_paths` arm reads this to
    /// emit a Kindling row after each verdict.
    fn observation_emitter(
        &self,
    ) -> Option<&Arc<crate::kindling_observation::SaveTimeObservationEmitter>> {
        None
    }
}

use crate::ShutdownToken;
use crate::dos::{IpcLimits, RpsBucket};
use crate::enforcement::CONTENT_SIZE_CAP_BYTES_USIZE;
use crate::fence::FenceStore;
use crate::kindling_observation::MidEditEmissionRequest;
use crate::kindling_observation::SaveTimeEmissionRequest;
use crate::kindling_observation::{CommandInvokedEmissionRequest, CommandInvokedEmitter};
use crate::midedit::{self, ScanBufferMode, ScanBufferRequest, ScanBufferService, SpoofBlockInfo};
use crate::registry::{Cross, SessionDispatcher, SessionRegistry};
use crate::status::{DaemonStatus, StatusProvider};

/// MLP2-025b: capability bundle the daemon control-lane needs to run
/// the write-time env-tag spoof cross-check. Carries the
/// `SessionRegistry` (for the cross-check) and the `FenceStore` (for
/// the side-effect fence on `Cross::Spoofed`).
///
/// `None` in tests, embedded fallback callers, and any listener
/// constructed without a daemon-backed cross-check — in those modes
/// the scan-buffer handler skips the cross-check entirely and
/// proceeds to the rule engine. Production wires this via
/// `IpcListener::with_cross_check_context`.
#[derive(Clone)]
pub struct CrossCheckContext {
    pub registry: Arc<SessionRegistry>,
    pub fence_store: Arc<FenceStore>,
}

/// Maximum size of a single NDJSON line, in bytes. Lines larger than
/// this cause the connection to be torn down with [`IpcError::OversizedLine`]
/// — protects the daemon from a same-UID peer streaming an unbounded
/// blob into one line. The cap allows a 1 MiB validation buffer in
/// worst-case JSON string encoding plus JSON-RPC framing overhead so
/// `scan_buffer` can reject over-cap content as a structured protocol
/// error instead of transport EOF.
pub const MAX_LINE_BYTES: usize = (CONTENT_SIZE_CAP_BYTES_USIZE * 6) + (64 << 10);
pub const LEGACY_MAX_LINE_BYTES: usize = 1 << 20;
pub const MAX_JSONRPC_BATCH_ITEMS: usize = 32;
pub const MAX_ACTIVE_CONNECTIONS: usize = 32;
pub const CONNECTION_READ_TIMEOUT: Duration = Duration::from_mins(1);
const MAX_JSONRPC_ID_BYTES: usize = 256;
const MAX_SCAN_BUFFER_MODE_BYTES: usize = 32;
/// Cap for the `env_agent_tag` wire field in the oversized fast-path
/// validator. The field carries a JSON-encoded `AgentTag` —
/// `{tag_kind, session_id, pid_starttime}` — which is structurally
/// small. 1 KiB is well above any realistic payload while keeping the
/// fast-path memory bound tight.
const MAX_SCAN_BUFFER_ENV_AGENT_TAG_BYTES: usize = 1024;
/// CLAWP-065: cap for the optional `session_id` wire field in the
/// oversized fast-path validator. Session ids are short opaque strings
/// (the launcher mints a UUID-class value); 256 bytes is well above
/// any realistic id while keeping the fast-path memory bound tight. It
/// matches [`MAX_JSONRPC_ID_BYTES`] — both bound caller-supplied
/// identifier strings.
const MAX_SCAN_BUFFER_SESSION_ID_BYTES: usize = 256;
/// USAGE-004: cap for the optional envelope `principal` wire field. The
/// principal is the same salted hash CLI usage rows carry —
/// `hex(SHA-256(salt ‖ ":" ‖ email))`, exactly 64 lowercase hex chars —
/// or the literal `"anonymous"`. We do NOT validate the hex shape on the
/// wire (keeps the field forward-compatible if the hash algorithm
/// changes); we only bound its length and reject non-strings. 256 bytes
/// is well above the 64-char real value and matches the established
/// identifier-cap family ([`MAX_SCAN_BUFFER_SESSION_ID_BYTES`] /
/// [`MAX_JSONRPC_ID_BYTES`]).
const MAX_PRINCIPAL_BYTES: usize = 256;
const MAX_TRACE_METHOD_LEN: usize = 128;

/// How long [`IpcListener::shutdown`] waits for in-flight handler tasks
/// to drain before aborting them. Kept small so a misbehaving handler
/// can't block daemon shutdown — INTD-006 owns the harder
/// signal-ladder semantics.
pub const SHUTDOWN_DRAIN_DEADLINE: Duration = Duration::from_millis(250);

/// Default no-op dispatcher used by tests and by the very first
/// listener spin-up before the registry is wired in. Production
/// callers always supply a real
/// [`SessionRegistry`](crate::registry::SessionRegistry).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopDispatcher;

/// Default empty status used by listeners that have not been wired
/// to a daemon. The IPC handler treats `query_status` as a stable
/// JSON-RPC method even on these listeners — a `NoopDispatcher`
/// listener answers with the empty snapshot rather than
/// `Method not found`, so test fixtures and embedded callers can
/// exercise the wire shape without a full daemon mock.
///
/// MLP2-051h contract: `query_status` MUST surface
/// `generated_at_unix == 0` so consumers can distinguish a synthetic
/// noop snapshot from a real [`crate::status::DaemonStatusProvider`]
/// snapshot. The production `run_foreground` path always swaps this
/// default out via [`IpcListener::with_status_provider`] /
/// [`IpcServer::with_status_provider`] before serving real clients —
/// see `anvil_intercept::lib::run_foreground` where the Unix and
/// Windows listener-bind branches both wire the real provider. If
/// this provider ever serves a real production request, the `debug`
/// trace below is the operator's diagnostic; pinned by
/// `crate::status::tests::generated_at_unix_zero_is_the_no_anchor_sentinel`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopStatusProvider;

impl StatusProvider for NoopStatusProvider {
    fn query_status(&self) -> DaemonStatus {
        // MLP2-051h: emit a debug trace every time the noop provider
        // is exercised. In production this listener default is always
        // swapped for a real `DaemonStatusProvider` before the
        // listener serves clients — see the `with_status_provider`
        // calls in `run_foreground`. A live `debug` event from
        // production therefore signals a wiring regression, distinct
        // from any test-side traffic. Pinned at `debug` (not `warn`)
        // because the in-tree test suite exercises this path tens of
        // times per run; a warn-level event would drown out real
        // signals in CI logs.
        tracing::debug!(
            target: "anvil_intercept::status",
            "NoopStatusProvider::query_status invoked — emitting empty \
             snapshot with generated_at_unix=0 (no-anchor sentinel). \
             Production callers MUST swap to DaemonStatusProvider via \
             with_status_provider; a live trace here from a production \
             binary is a wiring regression."
        );
        crate::status::build_status(
            Vec::new(),
            &[],
            // MLP2-026: NoopStatusProvider has no fence-store
            // access; the cascade overlay is empty for these
            // synthetic listeners.
            &[],
            None,
            std::time::Instant::now(),
            std::time::Instant::now(),
            env!("CARGO_PKG_VERSION"),
            crate::status::IpcState::Serving,
            None,
            None,
            // MLP2-051h: noop provider has no live wall clock to
            // stamp; consumers already treat `0` as the no-anchor
            // sentinel — fall back to per-session heartbeat freshness.
            0,
        )
    }
}

impl SessionDispatcher for NoopDispatcher {
    fn register(
        &self,
        _id: &anvil_intercept_proto::SessionId,
        _worktree: &Path,
        _agent_tag: Option<&anvil_intercept_proto::session::AgentTag>,
        _lineage: Option<&anvil_intercept_proto::session::LineageAnchor>,
    ) -> Result<(), crate::registry::RegistryError> {
        Ok(())
    }
    fn heartbeat(
        &self,
        _id: &anvil_intercept_proto::SessionId,
        _peer_pid: Option<u32>,
    ) -> Result<(), crate::registry::RegistryError> {
        // No-op: NoopDispatcher carries no per-session state, so there
        // is no launcher anchor to bind against (CIB-153). Listeners
        // that need real ownership enforcement wire a SessionRegistry.
        Ok(())
    }
    fn unregister(
        &self,
        _id: &anvil_intercept_proto::SessionId,
        _peer_pid: Option<u32>,
    ) -> Result<bool, crate::registry::RegistryError> {
        Ok(false)
    }
    fn list(&self) -> Vec<anvil_intercept_proto::SessionRecord> {
        Vec::new()
    }
    fn report_process(
        &self,
        _id: &anvil_intercept_proto::SessionId,
        _child_pid: u32,
        _child_pid_starttime: u64,
        _peer_pid: u32,
    ) -> Result<(), crate::registry::RegistryError> {
        // No-op: NoopDispatcher carries no per-session state, so
        // there is no lineage index to narrow. The IPC dispatch arm
        // still gates on `peer_pid.is_some()` before calling this,
        // so the legacy NDJSON / no-peer-credential paths cannot
        // smuggle a report_process through the noop.
        Ok(())
    }
}

/// Errors that the IPC listener surfaces. Bind-time failures bubble
/// up; per-connection failures are logged and the connection torn
/// down without taking the listener with it.
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("socket directory is a symlink: {0}")]
    SocketDirIsSymlink(PathBuf),
    #[error("socket path is a symlink: {0}")]
    SocketPathIsSymlink(PathBuf),
    #[error(
        "socket directory has wrong permissions: {path} (mode={mode:o}, expected 0o700, owner={owner_uid}, current={current_uid})"
    )]
    SocketDirPermissions {
        path: PathBuf,
        mode: u32,
        owner_uid: u32,
        current_uid: u32,
    },
    #[error("socket path exists and is not a socket: {0}")]
    SocketPathNotASocket(PathBuf),
    #[error(
        "socket path has wrong permissions: {path} (mode={mode:o}, expected 0o600, owner={owner_uid}, current={current_uid})"
    )]
    SocketPathPermissions {
        path: PathBuf,
        mode: u32,
        owner_uid: u32,
        current_uid: u32,
    },
    #[error("socket peer has wrong owner: peer={peer_uid}, current={current_uid}")]
    SocketPeerPermissions { peer_uid: u32, current_uid: u32 },
    #[error("another anvil-intercept daemon is already listening at {0}")]
    AnotherDaemonRunning(PathBuf),
    #[error("could not resolve socket directory: $XDG_RUNTIME_DIR is unset and $HOME is unset")]
    NoSocketDirCandidate,
    #[error("could not resolve current user: neither USER nor USERNAME nor LOGNAME env var is set")]
    NoCurrentUser,
    #[error("NDJSON line exceeded the {MAX_LINE_BYTES}-byte cap")]
    OversizedLine,
    /// A complete line was framed but its bytes are not valid UTF-8.
    /// Soft error: the connection handler logs and continues to the
    /// next line (same policy as malformed JSON), so a single bad
    /// frame on a long-lived stream cannot kill subsequent commands.
    #[error("NDJSON line is not valid UTF-8 ({len} bytes)")]
    InvalidUtf8 { len: usize },
}

// --------------------------------------------------------------------
// Path resolution — testable on every platform.
// --------------------------------------------------------------------

/// Resolve the directory the Unix socket lives in. Looks up
/// `$XDG_RUNTIME_DIR` first, then falls back to
/// `$HOME/.local/state/anvil`. Never returns `/tmp`. Pure-environment;
/// no filesystem access.
///
/// Callers wanting to inject a directory in tests should use the
/// env-var-driven [`resolve_socket_dir_with_env`] helper.
#[cfg(unix)]
pub fn resolve_socket_dir() -> Result<PathBuf, IpcError> {
    resolve_socket_dir_with_env(
        // DISTRIB-006: absolutised so a relative `ANVIL_HOME` resolves to the same
        // socket dir for the CLI client and the separately-spawned daemon.
        crate::anvil_home_prefix().map(std::path::PathBuf::into_os_string),
        std::env::var_os("XDG_RUNTIME_DIR"),
        std::env::var_os("HOME"),
    )
}

#[cfg(unix)]
fn resolve_socket_dir_with_env(
    anvil_home: Option<std::ffi::OsString>,
    xdg_runtime_dir: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, IpcError> {
    // DISTRIB-006 (ADR-060): a non-empty `ANVIL_HOME` re-roots the daemon socket
    // directly under the prefix so a pre-release candidate daemon and the
    // production daemon get distinct sockets and coexist (the per-`(uid, os)`
    // single-instance rule of ADR-036 keys off the socket/PID path). Takes
    // precedence over the runtime dir; unset = byte-for-byte default below.
    if let Some(prefix) = anvil_home.filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(prefix));
    }
    if let Some(dir) = xdg_runtime_dir.filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir).join("anvil"));
    }
    if let Some(dir) = home.filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(dir).join(".local/state/anvil"));
    }
    Err(IpcError::NoSocketDirCandidate)
}

/// Resolve the absolute Unix socket path used by the daemon. The
/// socket file itself is named `intercept.sock`.
#[cfg(unix)]
pub fn resolve_socket_path() -> Result<PathBuf, IpcError> {
    Ok(resolve_socket_dir()?.join("intercept.sock"))
}

/// Validate the client side of the Unix daemon rendezvous before a peer
/// sends proposed file content to the socket. Mirrors the listener's
/// owner-only posture without creating or unlinking anything.
#[cfg(unix)]
pub fn validate_socket_path_for_client(path: &Path) -> Result<(), IpcError> {
    use nix::unistd::Uid;
    use std::os::unix::fs::FileTypeExt;

    let current_uid = Uid::current().as_raw();
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "socket path has no parent",
        )
    })?;
    unix_perms::ensure_existing_dir(parent, current_uid)?;

    let meta = std::fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(IpcError::SocketPathIsSymlink(path.to_path_buf()));
    }
    if !meta.file_type().is_socket() {
        return Err(IpcError::SocketPathNotASocket(path.to_path_buf()));
    }
    unix_perms::ensure_socket_file(path, &meta, current_uid)?;

    Ok(())
}

/// Validate the connected Unix peer before writing proposed content. The
/// daemon IPC trust boundary is owner-only, so a peer with a different uid is
/// rejected even if the path preflight passed.
#[cfg(all(unix, target_os = "linux"))]
pub fn validate_connected_peer_for_client(
    stream: &std::os::unix::net::UnixStream,
) -> Result<(), IpcError> {
    use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
    use nix::unistd::Uid;

    let current_uid = Uid::current().as_raw();
    let credentials = getsockopt(stream, PeerCredentials)
        .map_err(|e| std::io::Error::other(format!("SO_PEERCRED: {e}")))?;
    let peer_uid = credentials.uid();
    if peer_uid != current_uid {
        return Err(IpcError::SocketPeerPermissions {
            peer_uid,
            current_uid,
        });
    }
    Ok(())
}

/// MLP2-025b: read the peer process id for an accepted tokio Unix
/// socket. Returns `None` when the platform / kernel does not
/// expose the peer PID, or when the peer has already exited
/// between accept and the credential read.
///
/// On Linux this is `SO_PEERCRED.pid()`, the same syscall
/// `validate_connected_peer_for_client` already issues for the UID
/// check. On macOS it is `LOCAL_PEERPID` (via tokio's `peer_cred`
/// wrapper). On Windows / non-Linux/-macOS Unix the function
/// returns `None` and MLP2-025b's cross-check treats that as
/// `Cross::Spoofed` whenever an env tag is supplied (spec §7
/// fail-closed verdict).
///
/// Used by the daemon control-lane (B7) to pass `writer_pid` into
/// `SessionRegistry::cross_check_env_tag`.
#[cfg(unix)]
pub fn peer_pid_for_tokio_unix_stream(stream: &tokio::net::UnixStream) -> Option<u32> {
    let cred = stream.peer_cred().ok()?;
    let pid = cred.pid()?;
    if pid <= 0 {
        return None;
    }
    u32::try_from(pid).ok()
}

/// macOS peer-credential validation. macOS has no `SO_PEERCRED`; the equivalent
/// is `getpeereid(2)` which fills in the peer's effective uid + gid at the time
/// the socket connection was established. The effective uid is the identity
/// that matters for the v1 same-UID trust boundary — a peer whose effective uid
/// drops back to the operator's after a `seteuid` call would still be
/// reported as the operator's uid here, which matches what the Linux
/// `SO_PEERCRED` path returns. The two branches are observably identical from
/// any caller's perspective.
#[cfg(all(unix, target_os = "macos"))]
pub fn validate_connected_peer_for_client(
    stream: &std::os::unix::net::UnixStream,
) -> Result<(), IpcError> {
    use nix::unistd::{Uid, getpeereid};

    let current_uid = Uid::current().as_raw();
    let (peer, _gid) =
        getpeereid(stream).map_err(|e| std::io::Error::other(format!("getpeereid: {e}")))?;
    let peer_uid = peer.as_raw();
    if peer_uid != current_uid {
        return Err(IpcError::SocketPeerPermissions {
            peer_uid,
            current_uid,
        });
    }
    Ok(())
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
pub fn validate_connected_peer_for_client(
    _stream: &std::os::unix::net::UnixStream,
) -> Result<(), IpcError> {
    Err(std::io::Error::other(
        "connected peer credential validation is not implemented on this Unix platform",
    )
    .into())
}

/// Resolve the Windows named-pipe path used by the daemon.
///
/// Format: `\\.\pipe\anvil-intercept-<current-user-sid>` by default;
/// with a non-empty `ANVIL_HOME` a stable hashed install-root suffix is
/// appended (see [`derive_pipe_name`]) so a candidate daemon and the
/// production daemon get distinct pipes and coexist (CIB-106, the
/// Windows half of DISTRIB-006 / ADR-060 — the Unix analogue is the
/// `ANVIL_HOME` branch of [`resolve_socket_dir`]).
///
/// This is the **only** pipe-name resolver: every daemon and client
/// surface (daemon bind, `ensure`, `intercept status`, the MCP
/// protection claim, the watch save-time transport, GCTX, registration,
/// `anvil-run`) MUST rendezvous through this function — the launcher
/// (`DriverClient` in DRVR-001) included; the helper is `pub` so
/// consumers re-export rather than re-implement. The SID — not an env
/// username — anchors the name, so account-name spoofing and
/// local/domain username collisions cannot move the rendezvous point.
#[cfg(windows)]
pub fn resolve_pipe_name() -> Result<String, IpcError> {
    let sid = anvil_intercept_win32::current_user_sid_string()?;
    Ok(derive_pipe_name(
        &sid,
        crate::anvil_home_prefix().as_deref(),
    ))
}

/// CIB-106: pure derivation of the Windows named-pipe rendezvous name from
/// the current-user SID and the active install root. Platform-independent so
/// it unit-tests on any host (mirrors [`resolve_socket_dir_with_env`]).
///
/// - `anvil_home` unset (`None`) → the legacy `\\.\pipe\anvil-intercept-<sid>`
///   name, byte-for-byte, so existing installs keep their rendezvous point.
/// - `anvil_home` set → `\\.\pipe\anvil-intercept-<sid>-r<fnv1a64-hex>`, a
///   stable bounded (16 hex chars) install-root namespace so two same-user
///   candidate daemons coexist (DISTRIB-006 / ADR-060). The root is hashed,
///   never embedded: pipe names are enumerable by other local users, so a raw
///   path would leak directory layout.
///
/// `anvil_home` must be the normalised prefix from
/// [`crate::anvil_home_prefix`] (blank treated as unset, relative values
/// absolutised against the cwd) so a CLI client and the separately-spawned
/// daemon agree on the name — the same guarantee the Unix socket resolver
/// gives. The hash input is the path's lossy UTF-8 text: deterministic for
/// any given env value, which is what the rendezvous needs (both ends
/// inherit the same `ANVIL_HOME`). No case or separator canonicalisation is
/// applied — mirroring the Unix side, which keys the socket path off the
/// uncanonicalised prefix too.
#[cfg_attr(not(windows), allow(dead_code))]
fn derive_pipe_name(sid: &str, anvil_home: Option<&Path>) -> String {
    match anvil_home {
        Some(root) => {
            // The raw path text is hashed without case or separator
            // canonicalisation (mirrors the Unix socket-dir resolver).
            // Assumption made explicit: on Windows — a case-insensitive
            // filesystem — the daemon and its clients must inherit the same
            // literal `ANVIL_HOME` string (the normal case: one parent env
            // spawns both). Two spellings of the same directory (`C:\Anvil`
            // vs `c:\anvil`, or `/` vs `\` separators) hash to different
            // names and will NOT rendezvous.
            let hash = fnv1a_64(root.to_string_lossy().as_bytes());
            format!(r"\\.\pipe\anvil-intercept-{sid}-r{hash:016x}")
        }
        None => format!(r"\\.\pipe\anvil-intercept-{sid}"),
    }
}

/// FNV-1a 64-bit over `bytes`. Used by [`derive_pipe_name`] for the
/// install-root namespace suffix: stability across builds is part of the
/// daemon/client rendezvous contract (pinned by the golden test), so this is
/// a fixed local implementation, not `DefaultHasher` (whose algorithm is
/// explicitly unspecified across releases).
#[cfg_attr(not(windows), allow(dead_code))]
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

/// Lookup the current OS user's account name from environment vars.
/// Used by [`resolve_pipe_name`]; left platform-independent so the
/// helper can be unit-tested on any host.
#[cfg_attr(not(test), allow(dead_code))]
fn current_user_name() -> Result<String, IpcError> {
    for var in ["USERNAME", "USER", "LOGNAME"] {
        if let Some(val) = std::env::var_os(var)
            && let Some(s) = val.to_str()
            && !s.is_empty()
        {
            return Ok(s.to_owned());
        }
    }
    Err(IpcError::NoCurrentUser)
}

// --------------------------------------------------------------------
// Unix permission ladder.
// --------------------------------------------------------------------

#[cfg(unix)]
mod unix_perms {
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::Path;

    use super::IpcError;

    /// Mode bits that matter for the 0700 / 0600 checks. Strips file
    /// type bits and the high-order setuid/setgid/sticky bits we
    /// don't care about here.
    pub fn mode_bits(mode: u32) -> u32 {
        mode & 0o777
    }

    /// Ensure the socket directory exists with mode 0700 owned by the
    /// current user. Creates it if missing — otherwise verifies the
    /// existing one. Refuses if the target itself is a symlink. Parent
    /// path components are NOT lstat-checked individually — `mkdir`
    /// does follow symlinks while resolving the parent path. The
    /// strong invariant we enforce is on the socket directory and
    /// socket file we ultimately bind on; the launcher's `DriverClient`
    /// (DRVR-001) re-validates owner / mode from its side. Tightening
    /// to per-component `O_NOFOLLOW`-style traversal would add
    /// platform-specific code on Windows and is deferred until a
    /// concrete attack on parent traversal surfaces.
    pub fn ensure_dir(dir: &Path, current_uid: u32) -> Result<(), IpcError> {
        match fs::symlink_metadata(dir) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(IpcError::SocketDirIsSymlink(dir.to_path_buf()));
                }
                // Already exists and is not a symlink — verify owner + mode.
                let mode = mode_bits(meta.permissions().mode());
                let owner_uid = meta.uid();
                if mode != 0o700 || owner_uid != current_uid {
                    return Err(IpcError::SocketDirPermissions {
                        path: dir.to_path_buf(),
                        mode,
                        owner_uid,
                        current_uid,
                    });
                }
                Ok(())
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // Create the parent recursively first (without our
                // strict mode — the parent only needs to exist), then
                // create our directory with explicit 0700 via
                // `nix::unistd::mkdir` (avoids the umask race that
                // `fs::create_dir_all` runs).
                if let Some(parent) = dir.parent() {
                    fs::create_dir_all(parent)?;
                }
                nix::unistd::mkdir(dir, nix::sys::stat::Mode::S_IRWXU)
                    .map_err(|e| std::io::Error::other(format!("mkdir: {e}")))?;
                // Re-verify after creation. If umask or ACLs widened
                // the mode beyond 0700, we surface the failure now
                // instead of silently shipping a wider socket dir.
                let meta = fs::symlink_metadata(dir)?;
                let mode = mode_bits(meta.permissions().mode());
                let owner_uid = meta.uid();
                if mode != 0o700 || owner_uid != current_uid {
                    // Tighten and re-check once — avoids spurious
                    // failures on filesystems where mkdir respects
                    // the umask but not the requested mode bits
                    // exactly.
                    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
                    let meta = fs::symlink_metadata(dir)?;
                    let mode = mode_bits(meta.permissions().mode());
                    let owner_uid = meta.uid();
                    if mode != 0o700 || owner_uid != current_uid {
                        return Err(IpcError::SocketDirPermissions {
                            path: dir.to_path_buf(),
                            mode,
                            owner_uid,
                            current_uid,
                        });
                    }
                }
                Ok(())
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn ensure_existing_dir(dir: &Path, current_uid: u32) -> Result<(), IpcError> {
        let meta = fs::symlink_metadata(dir)?;
        if meta.file_type().is_symlink() {
            return Err(IpcError::SocketDirIsSymlink(dir.to_path_buf()));
        }
        let mode = mode_bits(meta.permissions().mode());
        let owner_uid = meta.uid();
        if mode != 0o700 || owner_uid != current_uid {
            return Err(IpcError::SocketDirPermissions {
                path: dir.to_path_buf(),
                mode,
                owner_uid,
                current_uid,
            });
        }
        Ok(())
    }

    pub fn ensure_socket_file(
        path: &Path,
        meta: &std::fs::Metadata,
        current_uid: u32,
    ) -> Result<(), IpcError> {
        let mode = mode_bits(meta.permissions().mode());
        let owner_uid = meta.uid();
        if mode != 0o600 || owner_uid != current_uid {
            return Err(IpcError::SocketPathPermissions {
                path: path.to_path_buf(),
                mode,
                owner_uid,
                current_uid,
            });
        }
        Ok(())
    }
}

// --------------------------------------------------------------------
// Listener.
// --------------------------------------------------------------------

/// Bound IPC listener. On Unix, a Unix domain socket. On Windows, a
/// named pipe created with an owner-only security descriptor.
///
/// Construct with [`IpcListener::bind`]; drive with
/// [`IpcListener::serve`] until the supplied [`ShutdownToken`] fires.
pub struct IpcListener<D: SessionDispatcher> {
    #[cfg(unix)]
    inner: tokio::net::UnixListener,
    #[cfg(unix)]
    socket_path: PathBuf,
    #[cfg(unix)]
    dispatcher: Arc<D>,
    #[cfg(unix)]
    scan_buffer: ScanBufferService,
    #[cfg(unix)]
    limits: IpcLimits,
    #[cfg(unix)]
    status_provider: Arc<dyn StatusProvider>,
    /// MLP2-025b: optional cross-check capability. Set by the
    /// daemon at startup via [`Self::with_cross_check_context`];
    /// `None` for tests and embedded listeners that don't run the
    /// spoof check.
    #[cfg(unix)]
    cross_check: Option<CrossCheckContext>,
    /// DSV-005: the shared save-time verdict state (warm graph cache,
    /// per-worktree assurance, confinement policy). Set by the daemon at
    /// startup via [`Self::with_save_time_state`]; `None` for tests and
    /// embedded listeners that do not serve the save-time verbs.
    #[cfg(unix)]
    save_time: Option<Arc<SaveTimeState>>,
    /// MLP2-071 Phase 2: telemetry broadcaster the IPC subscriber
    /// surface registers connections with. Set by the daemon at startup
    /// via [`Self::with_broadcaster`]; `None` for tests and embedded
    /// listeners that do not serve telemetry subscriptions (those reply
    /// to `SubscribeTelemetry` with a "not available" error).
    #[cfg(any(unix, windows))]
    broadcaster: Option<Arc<crate::broadcaster::TelemetryBroadcaster>>,
    /// USAGE-004: the command-invocation usage producer. Set by the
    /// daemon at startup via [`Self::with_usage_emitter`]; `None` for
    /// tests and embedded listeners that do not record usage.
    #[cfg(any(unix, windows))]
    usage_emitter: Option<Arc<CommandInvokedEmitter>>,
    #[cfg(windows)]
    inner: tokio::net::windows::named_pipe::NamedPipeServer,
    #[cfg(windows)]
    pipe_name: String,
    #[cfg(windows)]
    dispatcher: Arc<D>,
    #[cfg(windows)]
    scan_buffer: ScanBufferService,
    #[cfg(windows)]
    limits: IpcLimits,
    #[cfg(windows)]
    status_provider: Arc<dyn StatusProvider>,
    #[cfg(windows)]
    cross_check: Option<CrossCheckContext>,
    /// DSV-010b: the shared save-time verdict state, served over the named pipe
    /// just as on Unix. `None` ⇒ the three save-time verbs reply `Method not
    /// found` (tests, embedded callers).
    #[cfg(windows)]
    save_time: Option<Arc<SaveTimeState>>,
    #[cfg(not(any(unix, windows)))]
    _marker: std::marker::PhantomData<D>,
}

impl<D: SessionDispatcher> IpcListener<D> {
    /// Override the INTD-016 `DoS` budgets for this listener. Builder
    /// pattern so existing callers continue to use `IpcLimits::default()`.
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_limits(mut self, limits: IpcLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Plug the INTD-011 status provider. Listeners default to
    /// [`NoopStatusProvider`] (an empty snapshot) so existing
    /// callers — fanout tests, fixture servers, parity probes —
    /// keep working without a daemon-backed status feed.
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_status_provider(mut self, provider: Arc<dyn StatusProvider>) -> Self {
        self.status_provider = provider;
        self
    }

    /// MLP2-025b: wire the cross-check capability. The daemon's
    /// `run_foreground` builder calls this with a context bundling
    /// the session registry and the fence store; tests and
    /// embedded listeners can skip it (and get legacy semantics —
    /// no spoof cross-check).
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_cross_check_context(mut self, context: CrossCheckContext) -> Self {
        self.cross_check = Some(context);
        self
    }

    /// DSV-005 / DSV-010b: wire the shared save-time verdict state. The daemon's
    /// `run_foreground` builder calls this once with the warm graph cache,
    /// per-worktree assurance machines, antipattern config, interactive pool,
    /// and operator confinement policy. Served over the Unix socket and the
    /// Windows named pipe alike. Listeners without it reply `Method not found`
    /// to the three save-time verbs (tests, embedded callers).
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_save_time_state(mut self, state: Arc<SaveTimeState>) -> Self {
        self.save_time = Some(state);
        self
    }

    /// MLP2-071 Phase 2: wire the telemetry broadcaster. The daemon's
    /// `run_foreground` builder calls this with the broadcaster built
    /// over `DaemonState`'s per-startup [`crate::fanout::Fanout`], so a
    /// `SubscribeTelemetry` connection registers against the same
    /// fan-out (and therefore the same operator-configured cross-session
    /// policy + redaction salt) that the producer broadcasts through.
    /// Listeners without it reply to `SubscribeTelemetry` with a
    /// structured "telemetry subscription not available" error (tests,
    /// embedded callers).
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_broadcaster(
        mut self,
        broadcaster: Arc<crate::broadcaster::TelemetryBroadcaster>,
    ) -> Self {
        self.broadcaster = Some(broadcaster);
        self
    }

    /// USAGE-004: plug the command-invocation usage producer. Listeners
    /// default to `None` (no usage rows) so tests and embedded callers
    /// keep working; the daemon host wires a real NDJSON-backed emitter.
    #[cfg(any(unix, windows))]
    #[must_use]
    pub fn with_usage_emitter(mut self, emitter: Arc<CommandInvokedEmitter>) -> Self {
        self.usage_emitter = Some(emitter);
        self
    }
}

impl<D: SessionDispatcher> IpcListener<D> {
    /// Bind a fresh listener at the platform-default path with the
    /// supplied dispatcher. Performs the full directory and socket
    /// permission ladder.
    #[cfg(unix)]
    pub fn bind_default(dispatcher: D) -> Result<Self, IpcError> {
        Self::bind_default_with_scan_buffer_service(dispatcher, ScanBufferService::default())
    }

    #[cfg(unix)]
    pub fn bind_default_with_scan_buffer_service(
        dispatcher: D,
        scan_buffer: ScanBufferService,
    ) -> Result<Self, IpcError> {
        let socket_path = resolve_socket_path()?;
        Self::bind_with_scan_buffer_service(&socket_path, dispatcher, scan_buffer)
    }

    /// Bind a fresh listener at `path`. The path's parent directory
    /// is checked / created with the strict permission ladder. The
    /// socket file itself is `fchmod`-ed to 0600 before connections
    /// are accepted.
    #[cfg(unix)]
    pub fn bind(path: &Path, dispatcher: D) -> Result<Self, IpcError> {
        Self::bind_with_scan_buffer_service(path, dispatcher, ScanBufferService::default())
    }

    #[cfg(unix)]
    pub fn bind_with_scan_buffer_service(
        path: &Path,
        dispatcher: D,
        scan_buffer: ScanBufferService,
    ) -> Result<Self, IpcError> {
        use nix::sys::stat::{Mode, fchmod};
        use nix::unistd::Uid;
        use std::os::unix::fs::{FileTypeExt, PermissionsExt};

        let current_uid = Uid::current().as_raw();
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "socket path has no parent",
            )
        })?;

        unix_perms::ensure_dir(parent, current_uid)?;

        // Refuse if the socket path itself is a symlink — defends
        // against pre-positioned symlinks that would redirect bind
        // somewhere we don't control.
        match std::fs::symlink_metadata(path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(IpcError::SocketPathIsSymlink(path.to_path_buf()));
                }
                if !meta.file_type().is_socket() {
                    return Err(IpcError::SocketPathNotASocket(path.to_path_buf()));
                }
                // Existing socket: try to connect. If a live daemon
                // answers, refuse — we are not the singleton. Unlink
                // ONLY on connect errors that reliably mean "no
                // listener" (`ConnectionRefused` / `NotFound`). Other
                // errors (transient resource exhaustion such as
                // `EMFILE`/`ENFILE`, permission failures, etc.) MUST
                // be surfaced — silently unlinking on those would let
                // a second daemon overwrite the well-known path while
                // a live one is still running, breaking the singleton
                // guarantee.
                match std::os::unix::net::UnixStream::connect(path) {
                    Ok(_) => return Err(IpcError::AnotherDaemonRunning(path.to_path_buf())),
                    Err(err)
                        if matches!(
                            err.kind(),
                            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                        ) =>
                    {
                        std::fs::remove_file(path)?;
                    }
                    Err(err) => return Err(err.into()),
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }

        let listener = tokio::net::UnixListener::bind(path)?;
        // Tighten the socket to 0600 before any peer can connect.
        // We try `fchmod` first — going via the fd defeats the
        // symlink-substitution window between bind and chmod-by-path.
        // On some platforms (notably some Linux variants) `fchmod` on
        // an `AF_UNIX` socket inode is a no-op, so we follow up with
        // a `chmod`-by-path. The path-based call is safe here only
        // because we have just verified that `path` did not exist
        // (or was a stale socket we unlinked) before `bind()`, and
        // we re-verify with `symlink_metadata` immediately afterwards
        // that the file we just bound is not a symlink.
        // `tokio::net::UnixListener` implements `AsFd`, which is what
        // `nix::sys::stat::fchmod` (a safe wrapper) wants — no unsafe
        // is needed here, keeping `forbid(unsafe_code)` honest.
        let _ = fchmod(&listener, Mode::S_IRUSR | Mode::S_IWUSR);
        let post_meta = std::fs::symlink_metadata(path)?;
        if post_meta.file_type().is_symlink() {
            return Err(IpcError::SocketPathIsSymlink(path.to_path_buf()));
        }
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;

        Ok(Self {
            inner: listener,
            socket_path: path.to_path_buf(),
            dispatcher: Arc::new(dispatcher),
            scan_buffer,
            limits: IpcLimits::default(),
            status_provider: Arc::new(NoopStatusProvider),
            cross_check: None,
            save_time: None,
            broadcaster: None,
            usage_emitter: None,
        })
    }

    /// Accept connections until `token` fires, spawning one handler
    /// per connection on a [`JoinSet`]. On shutdown, in-flight
    /// handlers are given [`SHUTDOWN_DRAIN_DEADLINE`] to finish before
    /// being aborted. The PID-file tie-in lands in INTD-001 follow-on;
    /// for INTD-002, socket-bind contention is the singleton guard.
    #[cfg(unix)]
    pub async fn serve(self, mut token: ShutdownToken) -> Result<(), IpcError> {
        let mut joinset: JoinSet<()> = JoinSet::new();
        let dispatcher = Arc::clone(&self.dispatcher);
        let scan_buffer = self.scan_buffer.clone();
        let limits = self.limits;
        let status_provider = Arc::clone(&self.status_provider);
        // MLP2-025b: captured once for the listener; cloned per spawn.
        let cross_check = self.cross_check.clone();
        // DSV-005: shared save-time state, cloned per spawn.
        let save_time = self.save_time.clone();
        // MLP2-071 Phase 2: telemetry broadcaster, cloned per spawn so
        // each subscriber connection registers against the shared
        // fan-out.
        let broadcaster = self.broadcaster.clone();
        // USAGE-004: usage producer, cloned per spawn.
        let usage_emitter = self.usage_emitter.clone();
        let connection_permits = Arc::new(tokio::sync::Semaphore::new(
            limits.max_concurrent_connections,
        ));

        loop {
            tokio::select! {
                biased;
                () = token.cancelled() => break,
                // Reap finished handler tasks as they complete so the
                // JoinSet only ever tracks live connections. The `if`
                // guard disables this arm when the JoinSet is empty,
                // because `join_next` on an empty set returns `None`
                // immediately and would busy-loop the select. This
                // also surfaces handler-task panics (via JoinError) at
                // the moment they happen rather than at shutdown.
                Some(res) = joinset.join_next(), if !joinset.is_empty() => {
                    if let Err(err) = res
                        && !err.is_cancelled()
                    {
                        tracing::warn!(
                            target: "anvil_intercept::ipc",
                            error = %err,
                            "ipc handler task panicked",
                        );
                        eprintln!("anvil-intercept: ipc handler task panicked: {err}");
                    }
                }
                accepted = self.inner.accept() => {
                    match accepted {
                        Ok((stream, _addr)) => {
                            let Ok(connection_permit) = connection_permits.clone().try_acquire_owned() else {
                                tracing::warn!(target: "anvil_intercept::ipc", "dropping ipc connection: active connection limit reached");
                                eprintln!("anvil-intercept: dropping ipc connection: active connection limit reached");
                                continue;
                            };
                            let dispatcher = Arc::clone(&dispatcher);
                            let scan_buffer = scan_buffer.clone();
                            let conn_status = Arc::clone(&status_provider);
                            let conn_token = token.clone();
                            // MLP2-025b: capture peer PID before moving
                            // the stream into the handler.
                            let peer_pid = peer_pid_for_tokio_unix_stream(&stream);
                            let conn_cross_check = cross_check.clone();
                            let conn_save_time = save_time.clone();
                            let conn_broadcaster = broadcaster.clone();
                            let conn_usage_emitter = usage_emitter.clone();
                            joinset.spawn(async move {
                                let _connection_permit = connection_permit;
                                if let Err(err) = handle_connection(stream, dispatcher, scan_buffer, conn_status, conn_token, limits, peer_pid, conn_cross_check, conn_save_time, conn_broadcaster, conn_usage_emitter).await {
                                    tracing::warn!(target: "anvil_intercept::ipc", error = %err, "ipc connection ended with error");
                                    eprintln!("anvil-intercept: ipc connection ended with error: {err}");
                                }
                            });
                        }
                        Err(err) => {
                            // A single bad accept — typically a peer
                            // that disconnected mid-handshake — must
                            // not take the listener down. Log and
                            // keep serving.
                            tracing::warn!(target: "anvil_intercept::ipc", error = %err, "accept failed");
                            eprintln!("anvil-intercept: accept failed: {err}");
                        }
                    }
                }
            }
        }

        // Drain in-flight handlers within the deadline. After the
        // deadline expires, abort what's left so daemon shutdown
        // cannot stall behind a misbehaving peer.
        let drain = async { while joinset.join_next().await.is_some() {} };
        if tokio::time::timeout(SHUTDOWN_DRAIN_DEADLINE, drain)
            .await
            .is_err()
        {
            joinset.shutdown().await;
        }

        // Best-effort cleanup of the socket file. If the file has
        // already been replaced (race with another daemon starting),
        // surfacing the error is more confusing than helpful.
        let _ = std::fs::remove_file(&self.socket_path);

        Ok(())
    }
}

#[cfg(windows)]
impl<D: SessionDispatcher> IpcListener<D> {
    /// Bind a Windows named pipe at the platform-default name.
    pub fn bind_default(dispatcher: D) -> Result<Self, IpcError> {
        Self::bind_default_with_scan_buffer_service(dispatcher, ScanBufferService::default())
    }

    pub fn bind_default_with_scan_buffer_service(
        dispatcher: D,
        scan_buffer: ScanBufferService,
    ) -> Result<Self, IpcError> {
        let pipe_name = resolve_pipe_name()?;
        Self::bind_with_scan_buffer_service(&pipe_name, dispatcher, scan_buffer)
    }

    /// Bind a Windows named pipe using an owner-only DACL and local-only clients.
    pub fn bind(pipe_name: &str, dispatcher: D) -> Result<Self, IpcError> {
        Self::bind_with_scan_buffer_service(pipe_name, dispatcher, ScanBufferService::default())
    }

    pub fn bind_with_scan_buffer_service(
        pipe_name: &str,
        dispatcher: D,
        scan_buffer: ScanBufferService,
    ) -> Result<Self, IpcError> {
        let server = anvil_intercept_win32::create_owner_only_pipe_server(
            pipe_name,
            anvil_intercept_win32::PipeInstance::First,
        )?;
        Ok(Self {
            inner: server,
            pipe_name: pipe_name.to_owned(),
            dispatcher: Arc::new(dispatcher),
            scan_buffer,
            limits: IpcLimits::default(),
            status_provider: Arc::new(NoopStatusProvider),
            cross_check: None,
            save_time: None,
            broadcaster: None,
            usage_emitter: None,
        })
    }

    /// Accept named-pipe clients until `token` fires, spawning one handler per client.
    pub async fn serve(self, mut token: ShutdownToken) -> Result<(), IpcError> {
        let mut server = self.inner;
        let pipe_name = self.pipe_name;
        let dispatcher = self.dispatcher;
        let scan_buffer = self.scan_buffer;
        let limits = self.limits;
        let status_provider = Arc::clone(&self.status_provider);
        let cross_check = self.cross_check.clone();
        // DSV-010b: shared save-time state, cloned per spawn (parallels the Unix
        // serve loop).
        let save_time = self.save_time.clone();
        // MLP2-071 Phase 2: telemetry broadcaster, cloned per spawn so
        // each subscriber connection registers against the shared
        // fan-out.
        let broadcaster = self.broadcaster.clone();
        // USAGE-004: usage producer, cloned per spawn.
        let usage_emitter = self.usage_emitter.clone();
        let mut joinset: JoinSet<()> = JoinSet::new();
        let connection_permits = Arc::new(tokio::sync::Semaphore::new(
            limits.max_concurrent_connections,
        ));

        loop {
            tokio::select! {
                biased;
                () = token.cancelled() => break,
                Some(res) = joinset.join_next(), if !joinset.is_empty() => {
                    if let Err(err) = res
                        && !err.is_cancelled()
                    {
                        tracing::warn!(
                            target: "anvil_intercept::ipc",
                            error = %err,
                            "ipc handler task panicked",
                        );
                        eprintln!("anvil-intercept: ipc handler task panicked: {err}");
                    }
                }
                connected = server.connect() => {
                    match connected {
                        Ok(()) => {
                            let connected_server = server;
                            server = anvil_intercept_win32::create_owner_only_pipe_server(
                                &pipe_name,
                                anvil_intercept_win32::PipeInstance::Additional,
                            )?;
                            // DSV-010b / ADR-070 step 4: belt-and-suspenders
                            // peer-SID check (parity for the Unix `SO_PEERCRED`
                            // same-uid gate). The owner-only pipe DACL already
                            // refuses a different-SID client at the kernel; this
                            // explicit `GetNamedPipeClientProcessId → token SID`
                            // compare is defence in depth.
                            //
                            // DSV-010b hardening: run it on a blocking thread —
                            // it issues several synchronous Win32 kernel calls
                            // (`OpenProcess` + `GetTokenInformation`), and doing
                            // them inline would block the accept loop's reactor
                            // thread on a pathologically slow same-uid peer.
                            // `connected_server` is held alive across the await so
                            // its handle stays valid; the raw handle is passed as
                            // `usize` (not the `RawHandle` pointer) only to satisfy
                            // `spawn_blocking`'s `Send + 'static` capture bound
                            // without reaching for `unsafe` in this
                            // `forbid(unsafe_code)` crate — it is cast straight
                            // back to a handle inside the closure. Fail closed on a
                            // non-owner, a validation error, or a join failure.
                            use std::os::windows::io::{AsRawHandle, RawHandle};
                            let raw_handle = connected_server.as_raw_handle() as usize;
                            let owner = tokio::task::spawn_blocking(move || {
                                anvil_intercept_win32::named_pipe_client_is_owner(
                                    raw_handle as RawHandle,
                                )
                            })
                            .await;
                            match owner {
                                Ok(Ok(true)) => {}
                                Ok(Ok(false)) => {
                                    tracing::warn!(target: "anvil_intercept::ipc", "rejecting named-pipe client: peer SID is not the pipe owner");
                                    eprintln!("anvil-intercept: rejecting named-pipe client: peer SID is not the pipe owner");
                                    drop(connected_server);
                                    continue;
                                }
                                Ok(Err(err)) => {
                                    tracing::warn!(target: "anvil_intercept::ipc", error = %err, "rejecting named-pipe client: peer SID validation failed");
                                    eprintln!("anvil-intercept: rejecting named-pipe client: peer SID validation failed: {err}");
                                    drop(connected_server);
                                    continue;
                                }
                                Err(join_err) => {
                                    tracing::warn!(target: "anvil_intercept::ipc", error = %join_err, "rejecting named-pipe client: peer SID validation task failed");
                                    eprintln!("anvil-intercept: rejecting named-pipe client: peer SID validation task failed: {join_err}");
                                    drop(connected_server);
                                    continue;
                                }
                            }
                            let Ok(connection_permit) = connection_permits.clone().try_acquire_owned() else {
                                tracing::warn!(target: "anvil_intercept::ipc", "dropping named-pipe connection: active connection limit reached");
                                eprintln!("anvil-intercept: dropping named-pipe connection: active connection limit reached");
                                drop(connected_server);
                                continue;
                            };
                            let dispatcher = Arc::clone(&dispatcher);
                            let scan_buffer = scan_buffer.clone();
                            let conn_status = Arc::clone(&status_provider);
                            let conn_token = token.clone();
                            let conn_cross_check = cross_check.clone();
                            let conn_save_time = save_time.clone();
                            let conn_broadcaster = broadcaster.clone();
                            let conn_usage_emitter = usage_emitter.clone();
                            joinset.spawn(async move {
                                let _connection_permit = connection_permit;
                                // MLP2-025b: Windows peer-PID is
                                // greenfield (tracked under MLP2-028).
                                // `None` here is paired with the
                                // Linux-only cross-check wire-up in
                                // `run_foreground` — on Windows
                                // `cross_check` is `None` and this
                                // value never reaches the fail-closed
                                // path. Once MLP2-028 lands the
                                // wire-up gate widens and `None`
                                // becomes the documented fail-closed
                                // default for un-validated peers.
                                let peer_pid: Option<u32> = None;
                                if let Err(err) = handle_connection(connected_server, dispatcher, scan_buffer, conn_status, conn_token, limits, peer_pid, conn_cross_check, conn_save_time, conn_broadcaster, conn_usage_emitter).await {
                                    tracing::warn!(target: "anvil_intercept::ipc", error = %err, "ipc connection ended with error");
                                    eprintln!("anvil-intercept: ipc connection ended with error: {err}");
                                }
                            });
                        }
                        Err(err) => {
                            tracing::warn!(target: "anvil_intercept::ipc", error = %err, "named-pipe connect failed");
                            eprintln!("anvil-intercept: named-pipe connect failed: {err}");
                            drop(server);
                            tokio::time::sleep(Duration::from_millis(25)).await;
                            server = anvil_intercept_win32::create_owner_only_pipe_server(
                                &pipe_name,
                                anvil_intercept_win32::PipeInstance::Additional,
                            )?;
                        }
                    }
                }
            }
        }

        let drain = async { while joinset.join_next().await.is_some() {} };
        if tokio::time::timeout(SHUTDOWN_DRAIN_DEADLINE, drain)
            .await
            .is_err()
        {
            joinset.shutdown().await;
        }

        Ok(())
    }
}

// --------------------------------------------------------------------
// MLP2-071 Phase 2: telemetry subscriber surface.
// --------------------------------------------------------------------

/// Write a pre-serialised telemetry notification frame (one NDJSON
/// line) to a subscriber connection. The broadcaster already produced
/// the JSON object; this only appends the line delimiter.
async fn write_telemetry_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    frame: &str,
) -> Result<(), IpcError> {
    writer.write_all(frame.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

/// A connection's live telemetry subscription: the outbound frame
/// receiver the read loop drains, plus the broadcaster handle + minted
/// `SubscriberId` needed to unregister.
///
/// The [`Drop`] impl is the single source of truth for teardown:
/// dropping a `Subscription` (on connection end, on
/// `UnsubscribeTelemetry`, or when replaced by a re-subscribe)
/// unregisters from the broadcaster, so a connection drop can never
/// leak a registration (D5: "disconnecting the IPC socket also
/// unregisters the subscriber").
struct Subscription {
    rx: tokio::sync::mpsc::Receiver<String>,
    broadcaster: Arc<crate::broadcaster::TelemetryBroadcaster>,
    id: crate::fanout::SubscriberId,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.broadcaster.unregister(&self.id);
    }
}

/// Does this JSON-RPC method name request a telemetry subscription?
/// Accepts the kebab-case discriminator, the underscore form the
/// launcher emits, and the dotted namespace form, mirroring the
/// multi-spelling convention `command_from_jsonrpc` already uses.
fn is_subscribe_telemetry_method(method: &str) -> bool {
    matches!(
        method,
        "subscribe-telemetry" | "subscribe_telemetry" | "telemetry.subscribe"
    )
}

/// Does this JSON-RPC method name tear down a telemetry subscription?
fn is_unsubscribe_telemetry_method(method: &str) -> bool {
    matches!(
        method,
        "unsubscribe-telemetry" | "unsubscribe_telemetry" | "telemetry.unsubscribe"
    )
}

/// Parse the optional `session_ids` narrowing filter from a
/// `subscribe-telemetry` frame's `params.filter`. A malformed or
/// absent filter yields `None` (no narrowing) — the daemon's fan-out
/// is the load-bearing boundary, so a bad client filter degrades to
/// "see everything the fan-out approves", never to over-disclosure.
///
/// An empty `session_ids` array also yields `None` (treated as "no
/// filter", not "allow nothing"): an empty allow-list would silently
/// suppress every envelope for a subscriber the daemon reports as
/// subscribed — a confusing footgun. A client that genuinely wants no
/// telemetry simply does not subscribe.
fn parse_subscriber_session_filter(value: &Value) -> Option<Vec<String>> {
    let ids = value
        .get("params")?
        .get("filter")?
        .get("session_ids")?
        .as_array()?;
    let ids: Vec<String> = ids
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    if ids.is_empty() { None } else { Some(ids) }
}

// --------------------------------------------------------------------
// Per-connection handler.
// --------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
// INTD-016 layered budgets share a single connection loop; splitting obscures the per-frame ordering of RPS / size / parse checks.
#[allow(clippy::too_many_arguments)] // MLP2-025b adds peer_pid + cross_check beside the existing dispatcher / scan_buffer / status / token / limits parameters; the chain is per-connection state, not bundleable without churn across every test caller.
async fn handle_connection<D: SessionDispatcher, R: AsyncRead + AsyncWrite + Unpin>(
    stream: R,
    dispatcher: Arc<D>,
    scan_buffer: ScanBufferService,
    status_provider: Arc<dyn StatusProvider>,
    mut token: ShutdownToken,
    limits: IpcLimits,
    // MLP2-025b: peer PID captured at accept time and threaded
    // through to `handle_scan_buffer_jsonrpc` so the daemon
    // control-lane can call
    // `SessionRegistry::cross_check_env_tag(env_tag, writer_pid)`.
    // `None` on platforms / kernels where the PID is not
    // available; the cross-check treats `None` + present env_tag
    // as `Cross::Spoofed` (fail-closed, spec §7).
    peer_pid: Option<u32>,
    // MLP2-025b: optional cross-check capability bundle. `None`
    // disables the write-time spoof check (tests, embedded
    // callers, listeners not wired to a daemon registry+fence).
    cross_check: Option<CrossCheckContext>,
    // DSV-005 / DSV-010b: shared save-time verdict state. `None` for listeners
    // that do not serve the save-time verbs. Served on both Unix and Windows —
    // the reads go through a platform-neutral `WorkspaceAnchor`.
    #[cfg(any(unix, windows))] save_time: Option<Arc<SaveTimeState>>,
    // MLP2-071 Phase 2: telemetry broadcaster for the subscriber surface.
    // `None` for listeners that do not serve telemetry subscriptions —
    // those reply to `SubscribeTelemetry` with a structured "not
    // available" error rather than silently accepting a no-op.
    #[cfg(any(unix, windows))] broadcaster: Option<Arc<crate::broadcaster::TelemetryBroadcaster>>,
    // USAGE-004: optional usage producer. `None` for tests and embedded
    // listeners that do not record command-invocation usage; the daemon
    // host wires a real NDJSON-backed emitter at startup.
    #[cfg(any(unix, windows))] usage_emitter: Option<Arc<CommandInvokedEmitter>>,
) -> Result<(), IpcError> {
    let mut reader = BufReader::new(stream);
    let mut buf = String::new();

    // DSV-005: the per-connection save-time context owns this connection's
    // admitted-root set (built lazily on the first verb) over the shared state.
    #[cfg(any(unix, windows))]
    let mut save_time_conn = save_time.as_deref().map(SaveTimeConn::new);

    // CIB-149: `Allowlist` mode has no implicit primary — the admitted-root set
    // is exactly the operator's allow entries, built lazily on the first verb.
    // A connection's own worktree (a client-declared `RegisterSession.worktree`
    // or a registry-lineage match) is never auto-admitted, because the daemon
    // only verifies the peer's identity, not that the path should be admitted.

    // MLP2-071 Phase 2: per-connection telemetry-subscriber state. When
    // `Some`, this connection has subscribed: the read loop drains its
    // `rx`, and dropping it (connection end, `UnsubscribeTelemetry`, or
    // re-subscribe) unregisters from the broadcaster. Listeners without
    // a broadcaster never set it and the read loop behaves as before.
    #[cfg(any(unix, windows))]
    let mut subscription: Option<Subscription> = None;

    // INTD-016: per-connection RPS bucket.
    let mut bucket = RpsBucket::from_limits(&limits, std::time::Instant::now());
    // INTD-016: handshake timeout — first line must arrive within
    // `limits.handshake_timeout` of accept. After the first line is
    // framed, subsequent reads use `limits.idle_timeout`.
    let mut first_frame_seen = false;

    loop {
        buf.clear();
        let deadline = if first_frame_seen {
            limits.idle_timeout
        } else {
            limits.handshake_timeout
        };
        // MLP2-071 Phase 2: when this connection is a telemetry
        // subscriber, race inbound command frames against outbound
        // telemetry frames so a pushed notification does not wait for
        // the next client frame. This select is deliberately NOT
        // `biased`: under a high-rate producer a biased telemetry-first
        // arm would drain up to the full channel cap before ever polling
        // the inbound read, starving the control lane (a client could
        // not get its `unsubscribe-telemetry` processed). The fair
        // round-robin lets inbound control frames make progress even
        // while telemetry is flowing; the bounded channel cap
        // ([`TELEMETRY_SUBSCRIBER_CHANNEL_CAP`]) plus drop-on-full keeps
        // the backlog finite either way. The subscription is taken out
        // of the `Option` for the `select!` and restored unless the
        // broadcaster closed the channel (then we drop it, which
        // unregisters, and keep the connection open for control frames).
        #[cfg(any(unix, windows))]
        let read = match subscription.take() {
            Some(mut sub) => {
                tokio::select! {
                    maybe_frame = sub.rx.recv() => {
                        match maybe_frame {
                            Some(frame) => {
                                subscription = Some(sub);
                                // Telemetry frames are outbound and
                                // producer-driven, so they intentionally
                                // bypass the per-connection RPS bucket
                                // (which rate-limits inbound *requests*).
                                // Backpressure for telemetry is the bounded
                                // channel cap + drop-on-full, not the bucket.
                                write_telemetry_frame(reader.get_mut(), &frame).await?;
                                continue;
                            }
                            None => {
                                // Broadcaster dropped our channel; drop
                                // `sub` (unregisters, idempotent) and
                                // stop streaming.
                                continue;
                            }
                        }
                    }
                    read = read_connection_line_with_deadline(&mut reader, &mut buf, &mut token, deadline) => {
                        subscription = Some(sub);
                        read?
                    }
                }
            }
            None => {
                read_connection_line_with_deadline(&mut reader, &mut buf, &mut token, deadline)
                    .await?
            }
        };
        #[cfg(not(any(unix, windows)))]
        let read =
            read_connection_line_with_deadline(&mut reader, &mut buf, &mut token, deadline).await?;
        let read = match read {
            ConnectionRead::Line(read) => read,
            ConnectionRead::Skip => continue,
            ConnectionRead::Closed => return Ok(()),
        };
        first_frame_seen = true;
        if read == 0 {
            // Peer closed cleanly.
            return Ok(());
        }
        // `read_line` keeps the trailing `\n`; trim it before parsing.
        let line = buf.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            continue;
        }

        // INTD-016: per-connection RPS bucket. Exhaustion returns a
        // structured error and lets the connection continue —
        // killing the connection on rate-limit would cause innocent
        // retries to escalate (see module doc).
        if !bucket.try_consume(std::time::Instant::now()) {
            tracing::warn!(
                target: "anvil_intercept::ipc",
                "rps bucket exhausted; replying with rate-limit error and continuing",
            );
            write_json_response(reader.get_mut(), &rate_limit_response(), "rate limit").await?;
            continue;
        }

        // INTD-016: control-frame size cap — applies to every frame
        // smaller than the legacy scan_buffer cap. The
        // 1 MiB scan_buffer payload survives untouched (the
        // oversize-scan_buffer fast path below handles that case).
        // Frames larger than the control cap but smaller than the
        // scan_buffer cap are inspected: if they declare a
        // non-scan_buffer method, the listener rejects them with a
        // structured error BEFORE attempting to parse the body.
        if line.len() > limits.control_frame_max_bytes
            && line.len() <= LEGACY_MAX_LINE_BYTES
            && !is_scan_buffer_frame(line)
        {
            tracing::warn!(
                target: "anvil_intercept::ipc",
                bytes = line.len(),
                cap = limits.control_frame_max_bytes,
                "control-lane frame exceeds INTD-016 cap; rejecting before parse",
            );
            write_json_response(
                reader.get_mut(),
                &control_frame_oversized_response(limits.control_frame_max_bytes),
                "control frame oversized",
            )
            .await?;
            continue;
        }

        if line.len() > LEGACY_MAX_LINE_BYTES
            && let Err(rejection) = validate_oversized_scan_buffer_frame(line)
        {
            // JSON-RPC 2.0: notifications never receive a response,
            // including for invalid request errors. Drop silently when
            // we've parsed enough of the frame to know it carries no
            // id; otherwise reply with an Invalid Request error so a
            // caller waiting on a correlation id is not left hanging.
            if rejection.is_notification {
                tracing::warn!(
                    target: "anvil_intercept::ipc",
                    reason = rejection.reason,
                    "dropping oversized scan_buffer notification without response",
                );
                continue;
            }
            write_json_response(
                reader.get_mut(),
                &oversized_frame_response(rejection.reason),
                "size error",
            )
            .await?;
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(value) => {
                if is_jsonrpc_frame(&value) {
                    // MLP2-071 Phase 2: intercept telemetry
                    // subscribe/unsubscribe before the generic dispatcher.
                    // They mutate this connection's outbound streaming
                    // state (the per-connection channel), which the
                    // request/response dispatcher cannot reach, and they
                    // need the peer credentials minted at accept time.
                    #[cfg(any(unix, windows))]
                    if let Some(method) = value.get("method").and_then(Value::as_str) {
                        if is_subscribe_telemetry_method(method) {
                            let id = value.get("id").cloned();
                            let is_notification = value.get("id").is_none();
                            let response = match (
                                broadcaster.as_ref(),
                                mint_subscriber_id(peer_pid),
                            ) {
                                (Some(bc), Some(subscriber)) => {
                                    // Idempotent (re)subscribe: drop any prior
                                    // subscription FIRST so its `Drop`
                                    // unregisters the old id before we
                                    // re-register. A same-peer re-subscribe
                                    // mints the same id, so registering before
                                    // dropping would unregister the entry we
                                    // just created.
                                    drop(subscription.take());
                                    let filter = parse_subscriber_session_filter(&value);
                                    let rx = bc.register(subscriber.clone(), filter);
                                    tracing::info!(
                                        target: "anvil_intercept::ipc",
                                        subscriber = subscriber.as_str(),
                                        peer_pid,
                                        "telemetry subscriber registered",
                                    );
                                    subscription = Some(Subscription {
                                        rx,
                                        broadcaster: Arc::clone(bc),
                                        id: subscriber,
                                    });
                                    if is_notification {
                                        None
                                    } else {
                                        Some(jsonrpc_success(id, None, json!({"subscribed": true})))
                                    }
                                }
                                (None, _) => jsonrpc_request_error(
                                    id,
                                    None,
                                    is_notification,
                                    -32601,
                                    "Method not found",
                                    json!({
                                        "reason":
                                            "telemetry subscription is not available on this listener"
                                    }),
                                ),
                                (Some(_), None) => {
                                    // Peer credentials could not be minted
                                    // (no SO_PEERCRED peer_pid, or a non-Linux
                                    // platform where pid_starttime is
                                    // unavailable). Surface it server-side so
                                    // a silent macOS/Windows degradation is
                                    // visible in daemon logs, not just to the
                                    // client.
                                    tracing::warn!(
                                        target: "anvil_intercept::ipc",
                                        peer_pid,
                                        "telemetry subscribe denied: could not mint subscriber id \
                                         from peer credentials",
                                    );
                                    jsonrpc_request_error(
                                        id,
                                        None,
                                        is_notification,
                                        -32000,
                                        "Server error",
                                        json!({
                                            "reason":
                                                "could not authenticate subscriber peer credentials"
                                        }),
                                    )
                                }
                            };
                            if let Some(resp) = response {
                                write_json_response(reader.get_mut(), &resp, "subscribe-telemetry")
                                    .await?;
                            }
                            continue;
                        }
                        if is_unsubscribe_telemetry_method(method) {
                            let id = value.get("id").cloned();
                            let is_notification = value.get("id").is_none();
                            // Dropping the subscription unregisters from the
                            // broadcaster and stops the outbound drain.
                            // `.take()` is idempotent if not subscribed.
                            if subscription.is_some() {
                                tracing::debug!(
                                    target: "anvil_intercept::ipc",
                                    peer_pid,
                                    "telemetry subscriber unregistered (unsubscribe)",
                                );
                            }
                            drop(subscription.take());
                            if !is_notification {
                                let resp = jsonrpc_success(id, None, json!({"subscribed": false}));
                                write_json_response(
                                    reader.get_mut(),
                                    &resp,
                                    "unsubscribe-telemetry",
                                )
                                .await?;
                            }
                            continue;
                        }
                    }
                    // DSV-005: hand the per-connection save-time context to the
                    // dispatcher (as the cross-platform trait object). `None`
                    // only on exotic non-unix/-windows targets, or listeners
                    // without save-time state.
                    #[cfg(any(unix, windows))]
                    let save_time_arg: Option<&mut dyn SaveTimeDispatch> = save_time_conn
                        .as_mut()
                        .map(|conn| conn as &mut dyn SaveTimeDispatch);
                    #[cfg(not(any(unix, windows)))]
                    let save_time_arg: Option<&mut dyn SaveTimeDispatch> = None;
                    // USAGE-004: record command-invocation usage for any
                    // allowlisted method in this frame BEFORE dispatch
                    // (keyed on invocation, not outcome). Borrows `value`
                    // so it must run before the move into the dispatcher.
                    #[cfg(any(unix, windows))]
                    if let Some(emitter) = usage_emitter.as_deref() {
                        let timestamp =
                            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                        emit_command_invocations(&value, emitter, &timestamp);
                    }
                    if let Some(response) = handle_jsonrpc_value(
                        value,
                        &dispatcher,
                        &scan_buffer,
                        &status_provider,
                        peer_pid,
                        cross_check.as_ref(),
                        save_time_arg,
                    )
                    .await
                    {
                        write_json_response(reader.get_mut(), &response, "response").await?;
                    }
                } else {
                    match serde_json::from_value::<IpcEnvelope>(value) {
                        Ok(envelope) => dispatch_envelope(&envelope, &dispatcher),
                        Err(err) => {
                            // Per the module doc: parse errors are logged and
                            // skipped, the connection stays open. Unknown command
                            // names take this branch too — see the proto crate's
                            // `unknown_command_variant_fails_to_deserialise` test.
                            tracing::warn!(
                                target: "anvil_intercept::ipc",
                                error = %err,
                                line_len = line.len(),
                                "skipping malformed NDJSON line"
                            );
                            eprintln!(
                                "anvil-intercept: skipping malformed NDJSON line ({} bytes): {}",
                                line.len(),
                                err
                            );
                        }
                    }
                }
            }
            Err(err) => {
                let response = jsonrpc_error(
                    None,
                    None,
                    -32700,
                    "Parse error",
                    json!({"reason": err.to_string()}),
                );
                write_json_response(reader.get_mut(), &response, "parse error").await?;
                tracing::warn!(
                    target: "anvil_intercept::ipc",
                    error = %err,
                    line_len = line.len(),
                    "skipping malformed NDJSON line"
                );
                eprintln!(
                    "anvil-intercept: skipping malformed NDJSON line ({} bytes): {}",
                    line.len(),
                    err
                );
            }
        }
    }
}

enum ConnectionRead {
    Line(usize),
    Skip,
    Closed,
}

async fn read_connection_line_with_deadline<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    buf: &mut String,
    token: &mut ShutdownToken,
    deadline: Duration,
) -> Result<ConnectionRead, IpcError> {
    tokio::select! {
        biased;
        () = token.cancelled() => Ok(ConnectionRead::Closed),
        res = tokio::time::timeout(deadline, read_one_line(reader, buf)) => match res {
            Ok(Ok(read)) => Ok(ConnectionRead::Line(read)),
            Ok(Err(IpcError::InvalidUtf8 { len })) => {
                log_invalid_utf8_line(len);
                Ok(ConnectionRead::Skip)
            }
            Ok(Err(err)) => Err(err),
            Err(_) => {
                log_idle_connection_timeout_with(deadline);
                Ok(ConnectionRead::Closed)
            }
        },
    }
}

fn log_invalid_utf8_line(len: usize) {
    tracing::warn!(
        target: "anvil_intercept::ipc",
        bytes = len,
        "skipping NDJSON line: invalid UTF-8",
    );
    eprintln!("anvil-intercept: skipping NDJSON line ({len} bytes): invalid UTF-8");
}

fn log_idle_connection_timeout_with(deadline: Duration) {
    tracing::warn!(
        target: "anvil_intercept::ipc",
        timeout_ms = deadline.as_millis(),
        "closing idle ipc connection",
    );
    eprintln!("anvil-intercept: closing idle ipc connection after {deadline:?}");
}

/// Pre-parse heuristic for "is this frame a `scan_buffer` request?".
/// Used by the INTD-016 control-frame size cap to skip enforcement on
/// `scan_buffer` frames (whose 1 MiB payload cap is a separate, larger
/// allowance owned by INTD-005).
///
/// Substring match on the literal `"scan_buffer"` is sufficient
/// because:
///
/// 1. The full JSON-RPC parser is invoked unconditionally afterwards
///    — a malformed frame that "looks" `scan_buffer`-shaped still falls
///    through to the structured parse error (-32700) below.
/// 2. False positives only weaken our `DoS` budget on a frame that
///    contains the literal `"scan_buffer"` somewhere (e.g. inside a
///    file path); the frame still has to fit inside `MAX_LINE_BYTES`
///    (≈ 6 MiB) and pass the JSON parser, so the worst case is the
///    operator pays JSON-parse cost on an outsize control-lane frame
///    before the `Method not found` error fires.
/// 3. False negatives (a real `scan_buffer` frame that does not
///    contain the literal) would be filtered out as oversize even
///    when legitimate. This is impossible: `scan_buffer` frames must
///    declare `"method": "scan_buffer"` per the JSON-RPC contract.
fn is_scan_buffer_frame(line: &str) -> bool {
    line.contains(midedit::SCAN_BUFFER_METHOD)
}

fn rate_limit_response() -> Value {
    // -32005 is in the implementation-defined server-error range
    // (-32000 .. -32099). The `Server busy` shape mirrors the
    // existing scan_buffer Busy error so consumers that already
    // handle it can reuse their backoff logic.
    jsonrpc_error(
        None,
        None,
        -32005,
        "Server busy",
        json!({"reason": "rate limit exceeded"}),
    )
}

fn control_frame_oversized_response(cap: usize) -> Value {
    jsonrpc_error(
        None,
        None,
        -32600,
        "Invalid Request",
        json!({
            "reason": format!("control-lane frame exceeds {cap}-byte cap"),
        }),
    )
}

async fn write_json_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &Value,
    context: &str,
) -> Result<(), IpcError> {
    let mut response = serde_json::to_string(response)
        .map_err(|err| io::Error::other(format!("serialise JSON-RPC {context}: {err}")))?;
    response.push('\n');
    writer.write_all(response.as_bytes()).await?;
    Ok(())
}

fn oversized_frame_response(reason: &str) -> Value {
    // Pre-parse rejection — the frame was too large to safely deserialise,
    // so we never recover a `traceparent` to echo here.
    jsonrpc_error(
        None,
        None,
        -32600,
        "Invalid Request",
        json!({ "reason": reason }),
    )
}

#[doc(hidden)]
#[cfg(feature = "bench-internals")]
pub async fn handle_jsonrpc_value_for_benchmark<D: SessionDispatcher>(
    value: Value,
    dispatcher: &Arc<D>,
    scan_buffer: &ScanBufferService,
) -> Option<Value> {
    let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);
    // MLP2-025b: benchmark fixture; no real socket, so no peer PID,
    // and no cross-check context (we're measuring the rule-engine
    // hot path, not the security cross-check).
    handle_jsonrpc_value(value, dispatcher, scan_buffer, &status, None, None, None).await
}

fn is_jsonrpc_frame(value: &Value) -> bool {
    match value {
        Value::Array(_) => true,
        Value::Object(map) => map.contains_key("jsonrpc") || map.contains_key("method"),
        _ => false,
    }
}

/// Rejection from the oversized-frame fast path. `is_notification` is
/// `true` only when the validator parsed the full object structure and
/// confirmed no `id` field is present; for early parse errors (where
/// the frame might still have an `id` further in) it stays `false` so
/// the caller defaults to writing an error response. Per JSON-RPC 2.0
/// the daemon MUST NOT reply to a notification, even on Invalid
/// Request — see the call site in `handle_connection`.
struct OversizedFrameRejection {
    reason: &'static str,
    is_notification: bool,
}

impl OversizedFrameRejection {
    const fn request(reason: &'static str) -> Self {
        Self {
            reason,
            is_notification: false,
        }
    }

    const fn notification(reason: &'static str) -> Self {
        Self {
            reason,
            is_notification: true,
        }
    }
}

#[allow(clippy::too_many_lines)] // Inline parser; splitting obscures the field-by-field flow.
fn validate_oversized_scan_buffer_frame(line: &str) -> Result<(), OversizedFrameRejection> {
    let bytes = line.as_bytes();
    let mut index = skip_json_whitespace(bytes, 0);
    if bytes.get(index) == Some(&b'[') {
        return Err(OversizedFrameRejection::request(
            "frames above the legacy cap must be a single scan_buffer request; batches are unsupported",
        ));
    }
    if bytes.get(index) != Some(&b'{') {
        return Err(OversizedFrameRejection::request(
            "frame exceeds the legacy cap for non-scan_buffer methods",
        ));
    }
    index += 1;

    let mut saw_jsonrpc = false;
    let mut saw_method = false;
    let mut saw_params = false;
    let mut saw_id = false;
    let mut saw_traceparent = false;

    loop {
        index = skip_json_whitespace(bytes, index);
        if bytes.get(index) == Some(&b'}') {
            index += 1;
            break;
        }
        if bytes.get(index) != Some(&b'"') {
            return Err(OversizedFrameRejection::request(
                "oversized scan_buffer frame is malformed",
            ));
        }
        let Some(key) = parse_simple_json_string(bytes, &mut index) else {
            return Err(OversizedFrameRejection::request(
                "oversized scan_buffer frame uses escaped or malformed field names",
            ));
        };
        index = skip_json_whitespace(bytes, index);
        if bytes.get(index) != Some(&b':') {
            return Err(OversizedFrameRejection::request(
                "oversized scan_buffer frame is malformed",
            ));
        }
        index += 1;
        index = skip_json_whitespace(bytes, index);

        match key {
            "jsonrpc" => {
                if saw_jsonrpc {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame contains duplicate jsonrpc fields",
                    ));
                }
                saw_jsonrpc = true;
                if parse_simple_json_string(bytes, &mut index) != Some("2.0") {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame must declare jsonrpc 2.0",
                    ));
                }
            }
            "method" => {
                if saw_method {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame contains duplicate method fields",
                    ));
                }
                saw_method = true;
                // DRVR-002 dual-routing: the oversize fast-path must
                // accept both the legacy bare name and the canonical
                // namespaced form. Any other method declared in an
                // oversize frame falls through to the legacy-cap
                // rejection below.
                let parsed_method = parse_simple_json_string(bytes, &mut index);
                if parsed_method != Some(midedit::SCAN_BUFFER_METHOD)
                    && parsed_method != Some(anvil_intercept_proto::protocol::ANVIL_SCAN_BUFFER)
                {
                    return Err(OversizedFrameRejection::request(
                        "frame exceeds the legacy cap for non-scan_buffer methods",
                    ));
                }
            }
            "params" => {
                if saw_params {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame contains duplicate params fields",
                    ));
                }
                saw_params = true;
                validate_oversized_scan_buffer_params(bytes, &mut index)
                    .map_err(OversizedFrameRejection::request)?;
            }
            "id" => {
                if saw_id {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame contains duplicate id fields",
                    ));
                }
                saw_id = true;
                if !skip_bounded_jsonrpc_id(bytes, &mut index) {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame id is missing or too large",
                    ));
                }
            }
            "traceparent" => {
                // TRACE-001: `traceparent` is a fixed-shape ASCII header
                // (W3C Trace Context). The fast path here only checks
                // it is a bounded simple string — full validation is
                // re-done after deserialisation in
                // `extract_traceparent`. Keeping this check tight
                // prevents an attacker from smuggling kilobytes of
                // padding through a "traceparent" key on an over-cap
                // frame.
                if saw_traceparent {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame contains duplicate traceparent fields",
                    ));
                }
                saw_traceparent = true;
                // `parse_simple_json_string` returns `None` for any
                // string containing escape sequences, so `value` here
                // is the raw bytes between the JSON quotes. Its
                // `.len()` is byte length (the same length the W3C
                // header is measured in), so the cap below is
                // meaningful even before any further validation runs.
                // The `>` (not `==`) is intentional: full format /
                // ASCII / hex validation is re-done in
                // `extract_traceparent` after deserialisation. This
                // guard only prevents an attacker padding kilobytes of
                // bytes through the `traceparent` key on an over-cap
                // frame.
                let Some(value) = parse_simple_json_string(bytes, &mut index) else {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame traceparent is missing or malformed",
                    ));
                };
                if value.len() > anvil_observability::traceparent::TRACEPARENT_LEN {
                    return Err(OversizedFrameRejection::request(
                        "oversized scan_buffer frame traceparent exceeds W3C length",
                    ));
                }
            }
            _ => {
                return Err(OversizedFrameRejection::request(
                    "oversized scan_buffer frame contains unsupported top-level fields",
                ));
            }
        }

        if consume_json_object_end_or_comma(bytes, &mut index)
            .map_err(OversizedFrameRejection::request)?
        {
            break;
        }
    }

    index = skip_json_whitespace(bytes, index);
    if index != bytes.len() {
        return Err(OversizedFrameRejection::request(
            "oversized scan_buffer frame has trailing data",
        ));
    }
    if !saw_jsonrpc || !saw_method || !saw_params {
        return Err(OversizedFrameRejection::request(
            "oversized scan_buffer frame must include jsonrpc, method, and params",
        ));
    }
    if !saw_id {
        // We have parsed the full object and no `id` is present, so the
        // frame is a notification by JSON-RPC definition. The caller
        // drops these silently — the request is structurally too large
        // to honour, and there is no caller to send an error to.
        return Err(OversizedFrameRejection::notification(
            "oversized scan_buffer notification dropped (no id)",
        ));
    }

    Ok(())
}

fn validate_oversized_scan_buffer_params(
    bytes: &[u8],
    index: &mut usize,
) -> Result<(), &'static str> {
    if bytes.get(*index) != Some(&b'{') {
        return Err("oversized scan_buffer params must be an object");
    }
    *index += 1;

    let mut saw_path = false;
    let mut saw_text = false;
    let mut saw_version = false;
    let mut saw_mode = false;
    // MLP2-025b: `env_agent_tag` is an optional wire field — the
    // post-parse validator accepts it; the fast-path must too,
    // otherwise oversized scan_buffer requests carrying a tag are
    // rejected before the cross-check sees them.
    let mut saw_env_agent_tag = false;
    // CLAWP-065: `session_id` is an optional wire field — the
    // post-parse validator accepts it; the fast-path must too, so an
    // oversized scan_buffer request carrying a session binding is not
    // rejected before the ownership check can see it.
    let mut saw_session_id = false;

    loop {
        *index = skip_json_whitespace(bytes, *index);
        if bytes.get(*index) == Some(&b'}') {
            *index += 1;
            break;
        }
        if bytes.get(*index) != Some(&b'"') {
            return Err("oversized scan_buffer params are malformed");
        }
        let Some(key) = parse_simple_json_string(bytes, index) else {
            return Err("oversized scan_buffer params use escaped or malformed field names");
        };
        *index = skip_json_whitespace(bytes, *index);
        if bytes.get(*index) != Some(&b':') {
            return Err("oversized scan_buffer params are malformed");
        }
        *index += 1;
        *index = skip_json_whitespace(bytes, *index);

        match key {
            "path" => {
                if saw_path {
                    return Err("oversized scan_buffer params contain duplicate path fields");
                }
                saw_path = true;
                if !skip_bounded_json_string(bytes, index, midedit::MAX_SCAN_BUFFER_PATH_BYTES) {
                    return Err("oversized scan_buffer path is missing or too large");
                }
            }
            "text" => {
                if saw_text {
                    return Err("oversized scan_buffer params contain duplicate text fields");
                }
                saw_text = true;
                if !skip_bounded_json_string(bytes, index, MAX_LINE_BYTES) {
                    return Err("oversized scan_buffer text is malformed");
                }
            }
            "version" => {
                if saw_version {
                    return Err("oversized scan_buffer params contain duplicate version fields");
                }
                saw_version = true;
                if !skip_bounded_json_number(bytes, index, 20) {
                    return Err("oversized scan_buffer version is missing or too large");
                }
            }
            "mode" => {
                if saw_mode {
                    return Err("oversized scan_buffer params contain duplicate mode fields");
                }
                saw_mode = true;
                if !skip_bounded_json_string(bytes, index, MAX_SCAN_BUFFER_MODE_BYTES) {
                    return Err("oversized scan_buffer mode is missing or too large");
                }
            }
            "env_agent_tag" => {
                if saw_env_agent_tag {
                    return Err(
                        "oversized scan_buffer params contain duplicate env_agent_tag fields",
                    );
                }
                saw_env_agent_tag = true;
                if !skip_bounded_json_string_or_null(
                    bytes,
                    index,
                    MAX_SCAN_BUFFER_ENV_AGENT_TAG_BYTES,
                ) {
                    return Err("oversized scan_buffer env_agent_tag is missing or too large");
                }
            }
            "session_id" => {
                if saw_session_id {
                    return Err("oversized scan_buffer params contain duplicate session_id fields");
                }
                saw_session_id = true;
                if !skip_bounded_json_string_or_null(bytes, index, MAX_SCAN_BUFFER_SESSION_ID_BYTES)
                {
                    return Err("oversized scan_buffer session_id is missing or too large");
                }
            }
            _ => return Err("oversized scan_buffer params contain unsupported fields"),
        }

        if consume_json_object_end_or_comma(bytes, index)? {
            break;
        }
    }

    if !saw_path || !saw_text || !saw_version || !saw_mode {
        return Err("oversized scan_buffer params must include path, text, version, and mode");
    }

    Ok(())
}

fn skip_json_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while matches!(bytes.get(index), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        index += 1;
    }
    index
}

/// Parse a JSON string with NO escape sequences, advancing `index`
/// past the closing quote on success.
///
/// Returns `None` for any string containing a backslash escape (so the
/// returned `&str` is always raw bytes between the quotes whose length
/// equals the decoded length), for non-UTF-8 input, and for malformed /
/// unterminated strings. Callers that need escape-decoded content must
/// use a full JSON parser instead.
///
/// On `None`, the position of `index` is undefined — callers must treat
/// the frame as malformed and stop scanning, not retry from the next
/// byte.
fn parse_simple_json_string<'a>(bytes: &'a [u8], index: &mut usize) -> Option<&'a str> {
    if bytes.get(*index) != Some(&b'"') {
        return None;
    }
    *index += 1;
    let start = *index;
    let mut escaped = false;
    while let Some(byte) = bytes.get(*index) {
        match *byte {
            b'\\' => {
                escaped = true;
                *index += 2;
            }
            b'"' => {
                let end = *index;
                *index += 1;
                if escaped {
                    return None;
                }
                return std::str::from_utf8(&bytes[start..end]).ok();
            }
            _ => *index += 1,
        }
    }
    None
}

fn skip_bounded_json_string(bytes: &[u8], index: &mut usize, max_raw_bytes: usize) -> bool {
    if bytes.get(*index) != Some(&b'"') {
        return false;
    }
    *index += 1;
    let start = *index;
    let mut escaped = false;

    while let Some(byte) = bytes.get(*index) {
        if escaped {
            escaped = false;
            *index += 1;
            continue;
        }
        match *byte {
            b'\\' => {
                escaped = true;
                *index += 1;
            }
            b'"' => {
                if *index - start > max_raw_bytes {
                    return false;
                }
                *index += 1;
                return true;
            }
            _ => *index += 1,
        }
    }

    false
}

/// Oversized fast-path skip for an optional `string | null` field.
/// Accepts a bare JSON `null` (which the post-parse path folds to
/// "absent") or a bounded JSON string. Without the `null` arm an
/// oversized `scan_buffer` frame carrying `"env_agent_tag": null` or
/// `"session_id": null` would be rejected even though the normal parse
/// path accepts it — diverging from the documented "string or null"
/// shape contract. Mirrors the `null` handling the top-level `id`
/// validator already uses.
fn skip_bounded_json_string_or_null(bytes: &[u8], index: &mut usize, max_raw_bytes: usize) -> bool {
    if bytes.get(*index..*index + 4) == Some(b"null") {
        *index += 4;
        return true;
    }
    skip_bounded_json_string(bytes, index, max_raw_bytes)
}

fn skip_bounded_json_number(bytes: &[u8], index: &mut usize, max_bytes: usize) -> bool {
    let start = *index;
    if bytes.get(*index) == Some(&b'-') {
        *index += 1;
    }

    let digit_start = *index;
    while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
        *index += 1;
    }
    if *index == digit_start {
        *index = start;
        return false;
    }

    if bytes.get(*index) == Some(&b'.') {
        *index += 1;
        let fraction_start = *index;
        while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
            *index += 1;
        }
        if *index == fraction_start {
            *index = start;
            return false;
        }
    }

    if matches!(bytes.get(*index), Some(b'e' | b'E')) {
        *index += 1;
        if matches!(bytes.get(*index), Some(b'+' | b'-')) {
            *index += 1;
        }
        let exponent_start = *index;
        while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
            *index += 1;
        }
        if *index == exponent_start {
            *index = start;
            return false;
        }
    }

    *index - start <= max_bytes
}

fn skip_bounded_jsonrpc_id(bytes: &[u8], index: &mut usize) -> bool {
    match bytes.get(*index) {
        Some(b'"') => skip_bounded_json_string(bytes, index, MAX_JSONRPC_ID_BYTES),
        Some(b'n') if bytes.get(*index..*index + 4) == Some(b"null") => {
            *index += 4;
            true
        }
        Some(b'-' | b'0'..=b'9') => skip_bounded_json_number(bytes, index, 64),
        _ => false,
    }
}

fn consume_json_object_end_or_comma(bytes: &[u8], index: &mut usize) -> Result<bool, &'static str> {
    *index = skip_json_whitespace(bytes, *index);
    match bytes.get(*index) {
        Some(b',') => {
            *index += 1;
            Ok(false)
        }
        Some(b'}') => {
            *index += 1;
            Ok(true)
        }
        _ => Err("oversized scan_buffer frame is malformed"),
    }
}

fn is_valid_jsonrpc_notification(value: &Value) -> bool {
    let Value::Object(map) = value else {
        return false;
    };
    !map.contains_key("id")
        && map.get("jsonrpc") == Some(&Value::String("2.0".to_owned()))
        && map.get("method").and_then(Value::as_str).is_some()
}

async fn handle_jsonrpc_value<D: SessionDispatcher>(
    value: Value,
    dispatcher: &Arc<D>,
    scan_buffer: &ScanBufferService,
    status_provider: &Arc<dyn StatusProvider>,
    // MLP2-025b: per-connection peer PID. `None` on platforms / kernels
    // where the PID is not available, or in synthetic test fixtures.
    peer_pid: Option<u32>,
    // MLP2-025b: optional cross-check capability bundle threaded
    // from `handle_connection`. `None` disables the spoof check.
    cross_check: Option<&CrossCheckContext>,
    // DSV-005: per-connection save-time verb dispatcher. `None` disables the
    // three save-time verbs (no save-time state, or Windows). The `+ '_`
    // decouples the trait-object lifetime from the `&mut` reborrow so the batch
    // loop can hand each item a short reborrow.
    mut save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                return Some(jsonrpc_error(
                    None,
                    None,
                    -32600,
                    "Invalid Request",
                    json!({
                        "reason": "batch must not be empty"
                    }),
                ));
            }
            if items.len() > MAX_JSONRPC_BATCH_ITEMS {
                if items.iter().all(is_valid_jsonrpc_notification) {
                    return None;
                }
                return Some(jsonrpc_error(
                    None,
                    None,
                    -32600,
                    "Invalid Request",
                    json!({
                        "reason": format!(
                            "batch must not contain more than {MAX_JSONRPC_BATCH_ITEMS} items"
                        )
                    }),
                ));
            }
            let mut responses = Vec::new();
            for item in items {
                // DRVR-002 dual-routing: reject scan_buffer in batches
                // under either the legacy bare name OR the canonical
                // namespaced form. Both share the per-frame size budget
                // and would explode batch-response memory.
                let method_name = jsonrpc_method_name(&item);
                if method_name == Some(midedit::SCAN_BUFFER_METHOD)
                    || method_name == Some(anvil_intercept_proto::protocol::ANVIL_SCAN_BUFFER)
                {
                    if let JsonRpcBatchResponseId::Request(response_id) =
                        jsonrpc_batch_response_id(&item)
                    {
                        // Echo a valid traceparent on the rejection so
                        // batch consumers can still correlate per-item.
                        // Invalid headers fall through to None and are
                        // silently ignored (the request is being rejected
                        // anyway).
                        let traceparent = item
                            .as_object()
                            .and_then(|map| extract_traceparent(map).ok().flatten());
                        responses.push(scan_buffer_batch_error(response_id, traceparent));
                    }
                    continue;
                }
                if let Some(response) = handle_jsonrpc_request(
                    item,
                    dispatcher,
                    scan_buffer,
                    status_provider,
                    peer_pid,
                    cross_check,
                    save_time.as_deref_mut(),
                )
                .await
                {
                    responses.push(response);
                }
            }
            if responses.is_empty() {
                None
            } else {
                Some(Value::Array(responses))
            }
        }
        item => {
            handle_jsonrpc_request(
                item,
                dispatcher,
                scan_buffer,
                status_provider,
                peer_pid,
                cross_check,
                save_time,
            )
            .await
        }
    }
}

fn jsonrpc_method_name(value: &Value) -> Option<&str> {
    let Value::Object(map) = value else {
        return None;
    };
    map.get("method").and_then(Value::as_str)
}

enum JsonRpcBatchResponseId {
    Request(Option<Value>),
    Notification,
}

fn jsonrpc_batch_response_id(value: &Value) -> JsonRpcBatchResponseId {
    let Value::Object(map) = value else {
        return JsonRpcBatchResponseId::Notification;
    };
    if map.contains_key("id") {
        JsonRpcBatchResponseId::Request(valid_jsonrpc_id(map.get("id")))
    } else {
        JsonRpcBatchResponseId::Notification
    }
}

fn scan_buffer_batch_error(response_id: Option<Value>, traceparent: Option<&str>) -> Value {
    jsonrpc_error(
        response_id,
        traceparent,
        -32600,
        "Invalid Request",
        json!({"reason": "scan_buffer is not supported in JSON-RPC batches"}),
    )
}

/// USAGE-004: the explicit allowlist of JSON-RPC methods that count as
/// *user-initiated command invocations* and therefore emit one
/// `command.invoked` usage row when dispatched. Founder decision
/// (2026-06-18): the GCTX query tools (no CLI-side USAGE-001 equivalent,
/// so daemon rows are net-new signal) plus the operator `unblock-*`
/// verbs (whose CLI-side row is suppressed so the daemon row is the
/// single source of truth). Every other dispatchable method is internal
/// machinery (scan/save/status), session lifecycle, or a server→client
/// message and is deliberately excluded. Both wire spellings of each
/// bare-name verb are listed so the classifier matches whatever the
/// client sends. New user-facing methods opt in here deliberately — see
/// `command_invoked_allowlist_classifies_every_namespaced_method`.
pub const COMMAND_INVOKED_ALLOWLIST: &[&str] = &[
    anvil_intercept_proto::protocol::ANVIL_GCTX_SEARCH_SYMBOLS,
    anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_DEPENDENTS,
    anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_CALLERS,
    anvil_intercept_proto::protocol::ANVIL_GCTX_IMPACT_OF_CHANGE,
    anvil_intercept_proto::protocol::ANVIL_GCTX_AFFECTED_TESTS,
    anvil_intercept_proto::protocol::ANVIL_GCTX_GRAPH_STATS,
    anvil_intercept_proto::protocol::ANVIL_GCTX_GRAPH_EDGES,
    anvil_intercept_proto::protocol::ANVIL_GCTX_GET_SNIPPET,
    anvil_intercept_proto::protocol::ANVIL_GCTX_SYMBOL_CONTEXT,
    "unblock-cascade",
    "fence.unblock-cascade",
    "unblock-worktree",
    "fence.unblock-worktree",
];

/// USAGE-004: true when `method` is a user-initiated command invocation
/// that should emit a `command.invoked` usage row. See
/// [`COMMAND_INVOKED_ALLOWLIST`].
pub fn is_command_invoked_method(method: &str) -> bool {
    COMMAND_INVOKED_ALLOWLIST.contains(&method)
}

/// USAGE-004: emit a `command.invoked` usage row for every allowlisted,
/// dispatchable request in `value`, which may be a single JSON-RPC object
/// or a batch array. Called from `handle_connection` just before the
/// frame is dispatched — emission is keyed on *invocation*, not outcome
/// (parity with the CLI producer, which records regardless of exit
/// status). A row is recorded only for a frame that will actually reach
/// dispatch: a batch that `handle_jsonrpc_value` would reject wholesale
/// (empty, or over [`MAX_JSONRPC_BATCH_ITEMS`]) emits nothing, so a
/// single oversized control frame can never drive more sink writes than
/// it can dispatch requests (Council: write-amplification / over-count).
/// Sink failures are swallowed inside the emitter. Non-allowlisted
/// methods, non-2.0 frames, frames whose envelope the dispatcher would
/// reject, and non-request shapes produce no row.
fn emit_command_invocations(value: &Value, emitter: &CommandInvokedEmitter, timestamp: &str) {
    match value {
        Value::Array(items) => {
            // Mirror `handle_jsonrpc_value`'s batch guards: an empty or
            // oversized batch is rejected wholesale and dispatches
            // nothing, so it must record nothing.
            if items.is_empty() || items.len() > MAX_JSONRPC_BATCH_ITEMS {
                return;
            }
            for item in items {
                emit_one_command_invocation(item, emitter, timestamp);
            }
        }
        item => emit_one_command_invocation(item, emitter, timestamp),
    }
}

/// USAGE-004: emit at most one `command.invoked` row for a single
/// JSON-RPC request object. See [`emit_command_invocations`].
fn emit_one_command_invocation(value: &Value, emitter: &CommandInvokedEmitter, timestamp: &str) {
    let Some(map) = value.as_object() else {
        return;
    };
    // Only frames the dispatcher would accept count — apply the SAME
    // front-matter checks as `handle_jsonrpc_request` so a row is never
    // recorded for a frame that the dispatcher then rejects (Council:
    // lenient-emit vs strict-dispatch divergence → phantom rows). A
    // malformed `principal`/`traceparent` is a hard rejection there, so
    // it must suppress the row here too; an *absent* optional field is
    // fine and resolves to `anonymous`/`None`.
    if map.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return;
    }
    let Some(method) = map.get("method").and_then(Value::as_str) else {
        return;
    };
    if !is_command_invoked_method(method) {
        return;
    }
    let (Ok(principal), Ok(traceparent)) = (extract_principal(map), extract_traceparent(map))
    else {
        // The dispatcher will reject this frame for the same reason; do
        // not record a phantom invocation.
        return;
    };
    let params = map.get("params").unwrap_or(&Value::Null);
    emitter.try_emit(&CommandInvokedEmissionRequest {
        method,
        principal,
        params,
        timestamp,
        traceparent,
    });
}

#[allow(clippy::too_many_lines)] // MLP2-026 pushed line count from 99 to 101 via the additional cross_check/peer_pid threading; splitting would obscure the per-method routing.
async fn handle_jsonrpc_request<D: SessionDispatcher>(
    value: Value,
    dispatcher: &Arc<D>,
    scan_buffer: &ScanBufferService,
    status_provider: &Arc<dyn StatusProvider>,
    // MLP2-025b: per-connection peer PID; threaded to
    // `handle_scan_buffer_jsonrpc`. See `handle_connection` doc.
    peer_pid: Option<u32>,
    // MLP2-025b: optional cross-check capability bundle.
    cross_check: Option<&CrossCheckContext>,
    // DSV-005: per-connection save-time verb dispatcher (`None` ⇒ verbs off).
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    let Value::Object(map) = value else {
        return Some(jsonrpc_error(
            None,
            None,
            -32600,
            "Invalid Request",
            json!({
                "reason": "request must be an object"
            }),
        ));
    };

    let id = map.get("id").cloned();
    let has_id = map.contains_key("id");
    let response_id = valid_jsonrpc_id(id.as_ref());
    if response_id.is_none() && id.is_some() {
        return Some(jsonrpc_error(
            None,
            None,
            -32600,
            "Invalid Request",
            json!({
                "reason": "id must be a string, number, or null within size limits"
            }),
        ));
    }

    // TRACE-001 / ADR-035: `traceparent` is the cross-pipe correlation
    // key. Extract and validate before any other shape check so a
    // malformed header is rejected with a deterministic error code, and
    // a valid one round-trips through every response (success or error)
    // unchanged.
    let traceparent = match extract_traceparent(&map) {
        Ok(tp) => tp,
        Err(JsonRpcFailure {
            code,
            message,
            data,
        }) => {
            // Header was unparseable — we deliberately do NOT echo it
            // on the rejection response (round-trip is only contracted
            // for valid headers).
            return jsonrpc_request_error(response_id, None, !has_id, code, message, data);
        }
    };

    // USAGE-004: the optional envelope `principal` is the salted-hash
    // identity the client attaches so daemon usage rows are attributable.
    // Extract alongside `traceparent`: a malformed/over-cap value is a
    // deterministic rejection. This binding enforces the principal wire
    // contract for requests that reach dispatch; the usage row itself is
    // produced upstream in `handle_connection` (via `emit_command_invocations`)
    // before this function is called, which applies the same strict
    // checks so it never records a frame this path would reject.
    let _principal = match extract_principal(&map) {
        Ok(p) => p,
        Err(JsonRpcFailure {
            code,
            message,
            data,
        }) => {
            return jsonrpc_request_error(response_id, traceparent, !has_id, code, message, data);
        }
    };

    if map.get("jsonrpc") != Some(&Value::String("2.0".to_owned())) {
        return Some(jsonrpc_error(
            response_id,
            traceparent,
            -32600,
            "Invalid Request",
            json!({"reason": "jsonrpc must be \"2.0\""}),
        ));
    }

    let Some(method) = map.get("method").and_then(Value::as_str) else {
        return Some(jsonrpc_error(
            response_id,
            traceparent,
            -32600,
            "Invalid Request",
            json!({"reason": "method must be a string"}),
        ));
    };
    let is_notification = !has_id;
    let params = map.get("params").unwrap_or(&Value::Null);
    let trace_context = traceparent.and_then(|raw| TraceContext::parse(raw).ok());
    let dispatch_span = jsonrpc_dispatch_span(method, is_notification, trace_context.as_ref());

    // Mid-edit scan: dual-routed under DRVR-002.
    //
    // - `midedit::SCAN_BUFFER_METHOD` (`"scan_buffer"`): the legacy
    //   bare-name form RTAI-002 / RTAI-008's contract suite is pinned
    //   on. Cannot break the existing 12 fixtures.
    // - `anvil_intercept_proto::protocol::ANVIL_SCAN_BUFFER`
    //   (`"anvil/scan_buffer"`): the canonical namespaced form drivers
    //   advertise in their manifest under DRVR-008's
    //   capability-negotiation rule. The proto-crate doc-comment
    //   already promises both names route to the same handler;
    //   landing the alias here makes that promise true on the wire.
    //
    // Both names share the same shape contract, request limits, and
    // response envelope.
    if method == midedit::SCAN_BUFFER_METHOD
        || method == anvil_intercept_proto::protocol::ANVIL_SCAN_BUFFER
    {
        return handle_scan_buffer_jsonrpc(
            &map,
            method,
            params,
            response_id,
            traceparent,
            is_notification,
            scan_buffer,
            peer_pid,
            cross_check,
        )
        .instrument(dispatch_span)
        .await;
    }

    // Status query: dual-routed under DRVR-002 / INTD-011.
    //
    // - `LEGACY_QUERY_STATUS_METHOD` (`"query_status"`): the bare-name
    //   form INTD-011 originally pinned. The CLI (`anvil intercept
    //   status`) and the existing 37-fixture conformance suite still
    //   speak it; we cannot break that contract until every consumer
    //   migrates.
    // - `anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY` (`"anvil/status/query"`):
    //   the canonical namespaced form DRVR-002 promised drivers when the
    //   protocol module shipped. Drivers that import the published
    //   constant must hit a live route, not a `Method not found`.
    //
    // Both names route to the same handler. The proto crate is
    // imported directly (not duplicated as a string literal) so any
    // future rename on the canonical side propagates here without a
    // silent drift.
    if method == LEGACY_QUERY_STATUS_METHOD
        || method == anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY
    {
        return dispatch_span.in_scope(|| {
            handle_query_status_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                status_provider,
            )
        });
    }

    // Save-time verbs (DSV-005): `validate_paths` / `workspace_status` /
    // `request_full_scan`. Special-method routed (like scan_buffer / status)
    // because they need the per-connection admitted-root set + shared save-time
    // state, which `dispatch_session_jsonrpc` (session-registry only) lacks.
    if method == anvil_intercept_proto::protocol::ANVIL_VALIDATE_PATHS
        || method == anvil_intercept_proto::protocol::ANVIL_WORKSPACE_STATUS
        || method == anvil_intercept_proto::protocol::ANVIL_REQUEST_FULL_SCAN
        || method == anvil_intercept_proto::protocol::ANVIL_WITNESS_APPEND
    {
        return dispatch_span.in_scope(|| {
            handle_save_time_jsonrpc(
                method,
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    // GCTX read verb (GCTX-010 / ADR-084): identity-only symbol search. Routed
    // on its own arm — separate from the save-time verbs above — but reuses the
    // same per-connection `SaveTimeConn` (admitted-root set + warm cache) via the
    // `GctxDispatch` supertrait. Never touches `validate_paths` / the enforcement
    // hot path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_SEARCH_SYMBOLS {
        return dispatch_span.in_scope(|| {
            handle_gctx_jsonrpc(params, response_id, traceparent, is_notification, save_time)
        });
    }

    // GCTX dependents traversal (GCTX-011 / ADR-084): same read-only `GctxDispatch`
    // surface as the symbol search above — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_DEPENDENTS {
        return dispatch_span.in_scope(|| {
            handle_gctx_find_dependents_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    // GCTX caller traversal (GCTX-014 / ADR-084): same read-only `GctxDispatch`
    // surface — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_CALLERS {
        return dispatch_span.in_scope(|| {
            handle_gctx_find_callers_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    // GCTX impact-of-change report (GCTX-012 / ADR-084): same read-only
    // `GctxDispatch` surface — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_IMPACT_OF_CHANGE {
        return dispatch_span.in_scope(|| {
            handle_gctx_impact_jsonrpc(params, response_id, traceparent, is_notification, save_time)
        });
    }

    // GCTX source-snippet extraction (GCTX-021 / ADR-084 / PV-9): same read-only
    // `GctxDispatch` surface — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_GET_SNIPPET {
        return dispatch_span.in_scope(|| {
            handle_gctx_get_snippet_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    // GCTX bounded symbol-context slice (GCTX-023 / ADR-084 / PV-9): same
    // read-only `GctxDispatch` surface — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_SYMBOL_CONTEXT {
        return dispatch_span.in_scope(|| {
            handle_gctx_symbol_context_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    // GCTX affected-tests report (GCTX-013 / ADR-084): same read-only
    // `GctxDispatch` surface — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_AFFECTED_TESTS {
        return dispatch_span.in_scope(|| {
            handle_gctx_affected_tests_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    // GCTX graph-stats summary (GCTX-030 / ADR-084): same read-only
    // `GctxDispatch` surface — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_GRAPH_STATS {
        return dispatch_span.in_scope(|| {
            handle_gctx_graph_stats_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    // GCTX graph-edges enumeration (GCTX-030 / ADR-084): same read-only
    // `GctxDispatch` surface — never the enforcement path.
    if method == anvil_intercept_proto::protocol::ANVIL_GCTX_GRAPH_EDGES {
        return dispatch_span.in_scope(|| {
            handle_gctx_graph_edges_jsonrpc(
                params,
                response_id,
                traceparent,
                is_notification,
                save_time,
            )
        });
    }

    dispatch_span.in_scope(|| {
        dispatch_session_jsonrpc(
            method,
            params,
            response_id,
            traceparent,
            is_notification,
            dispatcher,
            peer_pid,
            cross_check,
            save_time,
        )
    })
}

/// Legacy JSON-RPC method name for INTD-011 daemon status queries.
/// Pinned at the bare `query_status` literal — the
/// `anvil intercept status` CLI command and pre-DRVR-002 driver
/// consumers speak this form. Both this constant and
/// [`anvil_intercept_proto::protocol::ANVIL_STATUS_QUERY`] route to the
/// same handler in [`handle_jsonrpc_request`]; new consumers SHOULD
/// prefer the canonical `anvil/status/query` name.
pub const LEGACY_QUERY_STATUS_METHOD: &str = "query_status";

/// Backwards-compatible alias for [`LEGACY_QUERY_STATUS_METHOD`]. v0.5
/// callers imported `QUERY_STATUS_METHOD` directly; preserve the
/// re-export so the rename does not break external consumers.
pub const QUERY_STATUS_METHOD: &str = LEGACY_QUERY_STATUS_METHOD;

fn handle_query_status_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    status_provider: &Arc<dyn StatusProvider>,
) -> Option<Value> {
    if !matches!(params, Value::Null) {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            is_notification,
            -32602,
            "Invalid params",
            json!({"reason": "query_status does not accept params"}),
        );
    }
    if is_notification {
        // INTD-011 status is a request-shaped query — treating a
        // notification as a no-op matches the JSON-RPC contract for
        // notifications and avoids spamming an unanswered status
        // computation on the worker thread.
        return None;
    }
    let snapshot = status_provider.query_status();
    let wire = snapshot.to_wire();
    match serde_json::to_value(&wire) {
        Ok(result) => Some(jsonrpc_success(response_id, traceparent, result)),
        Err(err) => jsonrpc_request_error(
            response_id,
            traceparent,
            is_notification,
            -32603,
            "Internal error",
            json!({"error": err.to_string()}),
        ),
    }
}

/// JSON-RPC application error code for a save-time verb refused because its
/// `workspace_root` is not admitted on the connection (allowlist confinement).
const SAVE_TIME_NOT_ADMITTED_CODE: i64 = -32010;

/// CIB-154: JSON-RPC application error code for a save-time verb refused because
/// the connection is already at its per-connection admitted-root budget. In the
/// implementation-defined server-error range (`-32000..=-32099`), sequenced
/// after `-32010` (workspace not admitted).
const SAVE_TIME_ROOT_BUDGET_EXCEEDED_CODE: i64 = -32011;

/// Route a save-time verb (DSV-005) to the per-connection dispatcher. The
/// dispatch arm has already matched `method` against the three save-time
/// constants, so the `else` branch here is `request_full_scan`.
fn handle_save_time_jsonrpc(
    method: &str,
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    // Verbs are request-shaped (they mutate / read per-worktree assurance), so
    // a notification cannot carry a verdict back. Drop it but log — a save-time
    // verb sent as a notification is a client mistake, not a no-op to swallow
    // silently (mirrors the scan_buffer notification handling).
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::save_time",
            %method,
            "ignoring save-time verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        // No save-time state wired (tests, embedded callers, Windows). The verb
        // exists in the protocol but is not served here.
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "save-time validation is not enabled on this daemon"}),
        );
    };

    if method == anvil_intercept_proto::protocol::ANVIL_VALIDATE_PATHS {
        match serde_json::from_value::<ValidatePathsRequest>(params.clone()) {
            Ok(request) => {
                // DPO-001: measure the verdict and, on success, emit a save-time
                // `gate_evaluated` Kindling row (pass and fail). Decoupled from
                // the DSV-044 telemetry correlation/session gate — it fires on
                // the verdict alone. Emission is bounded + non-blocking: the
                // sink trait returns immediately and failure is swallowed inside
                // the emitter, so the verdict response always reaches the client.
                let started = Instant::now();
                let result = dispatch.validate_paths(&request);
                if let Ok(ref response) = result
                    && let Some(emitter) = dispatch.observation_emitter()
                {
                    let elapsed = started.elapsed();
                    let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
                    let timestamp =
                        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    let gate_eval_id = derive_gate_eval_id(traceparent);
                    let emission = SaveTimeEmissionRequest {
                        gate_eval_id: &gate_eval_id,
                        timestamp: &timestamp,
                        duration_ms,
                    };
                    // `request.paths` is a change-descriptor set; the row records
                    // the root-relative path strings only (no content, no change
                    // kind), matching the paths-only privacy contract. Skip the
                    // clone entirely when paths are not opted in (the default) —
                    // the count is still recorded.
                    let file_count = request.paths.len();
                    let paths: Vec<String> = if emitter.include_paths() {
                        request.paths.iter().map(|c| c.path.clone()).collect()
                    } else {
                        Vec::new()
                    };
                    let _ = emitter.try_emit(
                        &emission,
                        &response.diagnostics,
                        file_count,
                        &paths,
                        Instant::now(),
                    );
                }
                save_time_result(result, response_id, traceparent)
            }
            Err(err) => save_time_invalid_params(response_id, traceparent, &err),
        }
    } else if method == anvil_intercept_proto::protocol::ANVIL_WORKSPACE_STATUS {
        match serde_json::from_value::<WorkspaceStatusRequest>(params.clone()) {
            Ok(request) => save_time_result(
                dispatch.workspace_status(&request),
                response_id,
                traceparent,
            ),
            Err(err) => save_time_invalid_params(response_id, traceparent, &err),
        }
    } else if method == anvil_intercept_proto::protocol::ANVIL_WITNESS_APPEND {
        match serde_json::from_value::<WitnessAppendRequest>(params.clone()) {
            Ok(request) => {
                save_time_result(dispatch.witness_append(&request), response_id, traceparent)
            }
            Err(err) => save_time_invalid_params(response_id, traceparent, &err),
        }
    } else if method == anvil_intercept_proto::protocol::ANVIL_REQUEST_FULL_SCAN {
        match serde_json::from_value::<RequestFullScanRequest>(params.clone()) {
            Ok(request) => save_time_result(
                dispatch.request_full_scan(&request),
                response_id,
                traceparent,
            ),
            Err(err) => save_time_invalid_params(response_id, traceparent, &err),
        }
    } else {
        // Defence in depth: the caller's guard (the save-time method set) is the
        // only path here, so every known verb is handled above. A method that
        // reaches this arm means the guard and this dispatch drifted out of sync —
        // reply `Method not found` rather than panicking the daemon thread.
        jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "unrouted save-time verb", "method": method}),
        )
    }
}

/// Route the GCTX read verb (GCTX-010 / ADR-084) to the per-connection
/// dispatcher. Mirrors [`handle_save_time_jsonrpc`]: request-shaped (a
/// notification cannot carry results back), and a listener with no save-time
/// state replies `Method not found` (which the MCP consumer maps to
/// `Unavailable`). Graph degradation (warming / cold) is **not** an error here —
/// it rides in-band in the response `outcome` (CE-7); only a refused root or an
/// anchor IO error surfaces as a JSON-RPC error, via the shared
/// [`save_time_result`].
fn handle_gctx_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxSearchSymbolsRequest>(params.clone()) {
        Ok(request) => {
            save_time_result(dispatch.search_symbols(&request), response_id, traceparent)
        }
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX dependents verb (GCTX-011 / ADR-084) to the per-connection
/// dispatcher. Mirrors [`handle_gctx_jsonrpc`]: request-shaped, and a listener
/// with no save-time state replies `Method not found` (mapped to `Unavailable`
/// by the MCP consumer). Graph degradation rides in-band in the response
/// `outcome` (CE-7); only a refused root / anchor IO error is a JSON-RPC error.
fn handle_gctx_find_dependents_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxFindDependentsRequest>(params.clone()) {
        Ok(request) => {
            save_time_result(dispatch.find_dependents(&request), response_id, traceparent)
        }
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX caller verb (GCTX-014 / ADR-084) to the per-connection
/// dispatcher. Mirrors [`handle_gctx_find_dependents_jsonrpc`].
fn handle_gctx_find_callers_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxFindCallersRequest>(params.clone()) {
        Ok(request) => save_time_result(dispatch.find_callers(&request), response_id, traceparent),
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX get-snippet verb (GCTX-021 / ADR-084 / PV-9) to the
/// per-connection dispatcher. Mirrors [`handle_gctx_find_callers_jsonrpc`].
fn handle_gctx_get_snippet_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxGetSnippetRequest>(params.clone()) {
        Ok(request) => save_time_result(dispatch.get_snippet(&request), response_id, traceparent),
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX symbol-context verb (GCTX-023 / ADR-084 / PV-9) to the
/// per-connection dispatcher. Mirrors [`handle_gctx_get_snippet_jsonrpc`].
fn handle_gctx_symbol_context_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxSymbolContextRequest>(params.clone()) {
        Ok(request) => {
            save_time_result(dispatch.symbol_context(&request), response_id, traceparent)
        }
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX graph-stats verb (GCTX-030 / ADR-084) to the per-connection
/// dispatcher. Mirrors [`handle_gctx_find_callers_jsonrpc`].
fn handle_gctx_graph_stats_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxGraphStatsRequest>(params.clone()) {
        Ok(request) => save_time_result(dispatch.graph_stats(&request), response_id, traceparent),
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX graph-edges verb (GCTX-030 / ADR-084) to the per-connection
/// dispatcher. Mirrors [`handle_gctx_find_callers_jsonrpc`].
fn handle_gctx_graph_edges_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxGraphEdgesRequest>(params.clone()) {
        Ok(request) => save_time_result(dispatch.graph_edges(&request), response_id, traceparent),
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX impact-of-change verb (GCTX-012 / ADR-084) to the
/// per-connection dispatcher. Mirrors [`handle_gctx_jsonrpc`].
fn handle_gctx_impact_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxImpactOfChangeRequest>(params.clone()) {
        Ok(request) => save_time_result(
            dispatch.impact_of_change(&request),
            response_id,
            traceparent,
        ),
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// Route the GCTX affected-tests verb (GCTX-013 / ADR-084) to the per-connection
/// dispatcher. Mirrors [`handle_gctx_impact_jsonrpc`].
fn handle_gctx_affected_tests_jsonrpc(
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::gctx",
            "ignoring gctx verb sent as a notification: request id required",
        );
        return None;
    }
    let Some(dispatch) = save_time else {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            false,
            -32601,
            "Method not found",
            json!({"reason": "graph-context delivery is not enabled on this daemon"}),
        );
    };
    match serde_json::from_value::<GctxAffectedTestsRequest>(params.clone()) {
        Ok(request) => {
            save_time_result(dispatch.affected_tests(&request), response_id, traceparent)
        }
        Err(err) => save_time_invalid_params(response_id, traceparent, &err),
    }
}

/// A save-time request body that did not deserialise ⇒ `Invalid params`.
fn save_time_invalid_params(
    response_id: Option<Value>,
    traceparent: Option<&str>,
    err: &serde_json::Error,
) -> Option<Value> {
    jsonrpc_request_error(
        response_id,
        traceparent,
        false,
        -32602,
        "Invalid params",
        json!({"reason": err.to_string()}),
    )
}

/// Serialise a save-time verb outcome into a JSON-RPC success / error envelope.
fn save_time_result<T: serde::Serialize>(
    result: Result<T, SaveTimeError>,
    response_id: Option<Value>,
    traceparent: Option<&str>,
) -> Option<Value> {
    match result {
        Ok(response) => match serde_json::to_value(response) {
            Ok(value) => Some(jsonrpc_success(response_id, traceparent, value)),
            Err(err) => jsonrpc_request_error(
                response_id,
                traceparent,
                false,
                -32603,
                "Internal error",
                json!({"error": err.to_string()}),
            ),
        },
        Err(SaveTimeError::NotAdmitted {
            root,
            allow_entries,
        }) => {
            // A refusal is operationally meaningful (allowlist wall or a
            // vanished root) — surface it so an operator can diagnose a
            // `workspace-not-admitted` reply without reading the wire. The
            // refused path + allow-entry count + remediation hint go to the
            // SERVER log only; the wire reply below stays static and path-free
            // (N5 / CIB-091b — no path detail leaves the daemon).
            tracing::warn!(
                target: "anvil_intercept::save_time",
                workspace_root = %root.display(),
                allow_entries,
                "save-time verb refused: workspace not admitted \
                 (allowlist mode admits only configured allow entries; \
                 if the path still exists, run `anvil workspace allow <root>` \
                 to admit it; if the path no longer resolves, verify the \
                 client-named root)",
            );
            jsonrpc_request_error(
                response_id,
                traceparent,
                false,
                SAVE_TIME_NOT_ADMITTED_CODE,
                "Workspace not admitted",
                json!({"reason": "workspace-not-admitted"}),
            )
        }
        Err(SaveTimeError::RootBudgetExceeded { root, budget }) => {
            // CIB-154: the connection has hit its distinct-admitted-root ceiling.
            // Surface the refused path + budget to the SERVER log only (an
            // operator diagnosing a descriptor-exhaustion defence); the wire
            // reply below stays static and path-free (N5 / CIB-091b — no path
            // detail leaves the daemon), mirroring the `NotAdmitted` arm.
            tracing::warn!(
                target: "anvil_intercept::save_time",
                workspace_root = %root.display(),
                budget,
                "save-time verb refused: per-connection admitted-root budget \
                 exceeded (this connection already holds the maximum number of \
                 distinct workspace roots; close an unused connection or raise \
                 `enforcement.dos.max_admitted_roots` if this is a legitimate \
                 multi-root workflow)",
            );
            jsonrpc_request_error(
                response_id,
                traceparent,
                false,
                SAVE_TIME_ROOT_BUDGET_EXCEEDED_CODE,
                "Workspace root budget exceeded",
                json!({"reason": "workspace-root-budget-exceeded"}),
            )
        }
        Err(SaveTimeError::Io(err)) => {
            // N5 / CIB-091b follow-up: the raw `io::Error` Display can confirm the
            // existence/accessibility of a probed absolute path (an existence
            // oracle pairing with 091b). Log the OS detail server-side only — as
            // the `NotAdmitted` arm already does — and return a STATIC wire reason,
            // never `err.to_string()`.
            tracing::warn!(
                target: "anvil_intercept::save_time",
                error = %err,
                "save-time verb failed: i/o error resolving or opening the workspace root",
            );
            jsonrpc_request_error(
                response_id,
                traceparent,
                false,
                -32603,
                "Internal error",
                json!({"error": "workspace-io-error"}),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)] // MLP2-026 adds peer_pid + cross_check for UnblockCascade routing.
fn dispatch_session_jsonrpc<D: SessionDispatcher>(
    method: &str,
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    dispatcher: &Arc<D>,
    peer_pid: Option<u32>,
    cross_check: Option<&CrossCheckContext>,
    mut save_time: Option<&mut (dyn SaveTimeDispatch + '_)>,
) -> Option<Value> {
    let command = match command_from_jsonrpc(method, params) {
        Ok(command) => command,
        Err(JsonRpcFailure {
            code,
            message,
            data,
        }) => {
            return jsonrpc_request_error(
                response_id,
                traceparent,
                is_notification,
                code,
                message,
                data,
            );
        }
    };

    match dispatch_command(&command, dispatcher, peer_pid, cross_check) {
        Ok(result) => {
            if let IpcCommand::RegisterSession {
                session_id,
                worktree,
                ..
            } = &command
                && let Some(save_time) = &mut save_time
            {
                save_time.set_originating_session(session_id.as_str(), worktree);
            }
            if is_notification {
                None
            } else {
                Some(jsonrpc_success(response_id, traceparent, result))
            }
        }
        Err(err) => {
            // CIB-153: heartbeat is a fire-and-forget JSON-RPC
            // notification, so `jsonrpc_request_error` returns `None`
            // (no wire response) on the notification path — without
            // this log a genuine `PeerOwnershipMismatch` (now only
            // possible for lineage-bearing sessions) would be silent in
            // both client and daemon. Log server-side so the refusal is
            // diagnosable, mirroring the sibling refusal paths in this
            // file.
            tracing::warn!(
                target: "anvil_intercept::ipc",
                method = %method,
                error = %err,
                "session dispatch returned error"
            );
            jsonrpc_request_error(
                response_id,
                traceparent,
                is_notification,
                -32603,
                "Internal error",
                json!({"error": err.clone()}),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)] // MLP2-025b adds peer_pid + cross_check beside the existing JSON-RPC framing parameters; per-call state, not bundleable without test churn.
async fn handle_scan_buffer_jsonrpc(
    map: &serde_json::Map<String, Value>,
    method: &str,
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    scan_buffer: &ScanBufferService,
    // MLP2-025b: per-connection peer PID. Used by the cross-check
    // below.
    peer_pid: Option<u32>,
    // MLP2-025b: optional cross-check capability bundle. When
    // `Some`, the daemon runs the env-tag spoof cross-check before
    // invoking the rule engine; on `Cross::Spoofed` the write is
    // blocked and a worktree-level fence is recorded. When `None`,
    // the cross-check is skipped (legacy semantics).
    cross_check: Option<&CrossCheckContext>,
) -> Option<Value> {
    if let Err(JsonRpcFailure {
        code,
        message,
        data,
    }) = validate_scan_buffer_request_shape(map, method)
    {
        return jsonrpc_request_error(
            response_id,
            traceparent,
            is_notification,
            code,
            message,
            data,
        );
    }
    if is_notification {
        tracing::warn!(
            target: "anvil_intercept::ipc",
            "ignoring scan_buffer notification: request id required"
        );
        eprintln!("anvil-intercept: ignoring scan_buffer notification: request id required");
        return None;
    }
    match scan_buffer_from_jsonrpc(
        params,
        method,
        traceparent,
        scan_buffer,
        peer_pid,
        cross_check,
    )
    .await
    {
        Ok(result) => Some(jsonrpc_success(response_id, traceparent, result)),
        Err(JsonRpcFailure {
            code,
            message,
            data,
        }) => jsonrpc_request_error(
            response_id,
            traceparent,
            is_notification,
            code,
            message,
            data,
        ),
    }
}

/// Parse the optional `traceparent` field from a JSON-RPC envelope.
///
/// TRACE-001 contract (see ADR-035): when present the value MUST be a
/// W3C `traceparent` header (version 00). On success the function
/// returns the borrowed raw string; the **producer is the source of
/// truth** for the bytes, and they round-trip onto the response
/// unchanged. The strict parser guarantees only canonical forms reach
/// the round-trip echo, so reflecting `raw` (rather than
/// re-serialising via `TraceContext::as_header`) is safe.
///
/// Absent is the default — `traceparent` is optional on every method.
///
/// Same-UID peers can supply any valid context: the daemon does not
/// mint trace IDs and cannot detect ID-fixation. Trace integrity for
/// exported spans is the exporter's concern, not the envelope's
/// (accepted risk per ADR-035).
fn extract_traceparent(
    map: &serde_json::Map<String, Value>,
) -> Result<Option<&str>, JsonRpcFailure> {
    let Some(value) = map.get("traceparent") else {
        return Ok(None);
    };
    let Some(raw) = value.as_str() else {
        return Err(invalid_request("traceparent must be a string"));
    };
    TraceContext::parse(raw)
        .map_err(|err| invalid_request(format!("traceparent is invalid: {err}")))?;
    Ok(Some(raw))
}

/// USAGE-004: extract the optional envelope `principal`, the salted-hash
/// identity the client attaches so daemon usage rows carry the same
/// principal as CLI rows. Mirrors [`extract_traceparent`]'s discipline:
/// absent or JSON `null` both yield `None` (the producer resolves that
/// to `"anonymous"`, parity with the unauthenticated CLI path); a
/// non-string is a hard `Invalid Request`; an over-cap string is
/// rejected so a caller cannot smuggle an unbounded field through. The
/// hex shape is deliberately NOT validated here (see [`MAX_PRINCIPAL_BYTES`]).
fn extract_principal(map: &serde_json::Map<String, Value>) -> Result<Option<&str>, JsonRpcFailure> {
    match map.get("principal") {
        // Absent or explicit JSON null both resolve to None (→ anonymous).
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => {
            if raw.len() > MAX_PRINCIPAL_BYTES {
                return Err(invalid_request("principal exceeds the maximum length"));
            }
            Ok(Some(raw.as_str()))
        }
        Some(_) => Err(invalid_request("principal must be a string")),
    }
}

fn valid_jsonrpc_id(id: Option<&Value>) -> Option<Value> {
    match id {
        Some(Value::Null) => Some(Value::Null),
        Some(Value::String(value)) if value.len() <= MAX_JSONRPC_ID_BYTES => {
            Some(Value::String(value.clone()))
        }
        Some(Value::Number(value)) if value.to_string().len() <= 64 => {
            Some(Value::Number(value.clone()))
        }
        _ => None,
    }
}

fn jsonrpc_request_error(
    id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    code: i64,
    message: &'static str,
    data: impl serde::Serialize,
) -> Option<Value> {
    if is_notification {
        None
    } else {
        Some(jsonrpc_error(id, traceparent, code, message, data))
    }
}

fn jsonrpc_error(
    id: Option<Value>,
    traceparent: Option<&str>,
    code: i64,
    message: &'static str,
    data: impl serde::Serialize,
) -> Value {
    let capacity = 3 + usize::from(traceparent.is_some());
    let mut response = serde_json::Map::with_capacity(capacity);
    response.insert("jsonrpc".to_owned(), Value::String("2.0".to_owned()));
    response.insert(
        "error".to_owned(),
        json!({"code": code, "message": message, "data": data}),
    );
    response.insert("id".to_owned(), id.unwrap_or(Value::Null));
    if let Some(tp) = traceparent {
        response.insert("traceparent".to_owned(), Value::String(tp.to_owned()));
    }
    Value::Object(response)
}

/// TRACE-001 round-trip helper: build the canonical success envelope and
/// echo `traceparent` when the producer sent one. Consumes `result` so
/// the caller does not pay for a clone in the hot path.
fn jsonrpc_success(id: Option<Value>, traceparent: Option<&str>, result: Value) -> Value {
    let capacity = 3 + usize::from(traceparent.is_some());
    let mut map = serde_json::Map::with_capacity(capacity);
    map.insert("jsonrpc".to_owned(), Value::String("2.0".to_owned()));
    map.insert("result".to_owned(), result);
    map.insert("id".to_owned(), id.unwrap_or(Value::Null));
    if let Some(tp) = traceparent {
        map.insert("traceparent".to_owned(), Value::String(tp.to_owned()));
    }
    Value::Object(map)
}

struct JsonRpcFailure {
    code: i64,
    message: &'static str,
    data: Value,
}

#[allow(clippy::too_many_lines)] // MLP2-074 adds the report-process arm; this is a flat per-method dispatch table and is most readable inline.
fn command_from_jsonrpc(method: &str, params: &Value) -> Result<IpcCommand, JsonRpcFailure> {
    match method {
        "list-sessions" | "session.list" => {
            if matches!(params, Value::Null) {
                Ok(IpcCommand::ListSessions)
            } else {
                Err(invalid_params(
                    method,
                    "list-sessions does not accept params",
                ))
            }
        }
        "heartbeat" | "session.heartbeat" => params_object(params, method).and_then(|params| {
            let session_id = anvil_intercept_proto::SessionId::new(required_string(
                params,
                "session_id",
                method,
            )?);
            Ok(IpcCommand::Heartbeat { session_id })
        }),
        "register-session" | "session.register" => {
            params_object(params, method).and_then(|params| {
                let session_id = anvil_intercept_proto::SessionId::new(required_string(
                    params,
                    "session_id",
                    method,
                )?);
                let worktree = required_string(params, "worktree", method)?;
                // MLP2-023: `agent_tag` is optional. Absence yields
                // the legacy single-session-per-worktree path; a
                // present `agent_tag` opts into the composite key.
                // Malformed objects (wrong shape) surface as
                // invalid-params rather than being silently dropped
                // so a typo at the launcher is caught at the boundary.
                let agent_tag = match params.get("agent_tag") {
                    Some(value) if value.is_null() => None,
                    Some(value) => Some(
                        serde_json::from_value::<anvil_intercept_proto::session::AgentTag>(
                            value.clone(),
                        )
                        .map_err(|err| {
                            invalid_params(
                                method,
                                format!("agent_tag failed to deserialise: {err}"),
                            )
                        })?,
                    ),
                    None => None,
                };
                // MLP2-025b: optional launcher PID + pid_starttime
                // anchor. Same shape contract as `agent_tag`: absent
                // or null both fold to `None`; a present-but-malformed
                // object is a hard parse failure.
                let lineage = match params.get("lineage") {
                    Some(value) if value.is_null() => None,
                    Some(value) => Some(
                        serde_json::from_value::<anvil_intercept_proto::session::LineageAnchor>(
                            value.clone(),
                        )
                        .map_err(|err| {
                            invalid_params(method, format!("lineage failed to deserialise: {err}"))
                        })?,
                    ),
                    None => None,
                };
                Ok(IpcCommand::RegisterSession {
                    session_id,
                    worktree: PathBuf::from(worktree.as_str()),
                    agent_tag,
                    lineage,
                })
            })
        }
        "unregister-session" | "session.unregister" => {
            params_object(params, method).and_then(|params| {
                let session_id = anvil_intercept_proto::SessionId::new(required_string(
                    params,
                    "session_id",
                    method,
                )?);
                Ok(IpcCommand::UnregisterSession { session_id })
            })
        }
        // MLP2-074: post-spawn lineage-anchor narrowing. The launcher
        // emits this with `pid`, `pgid`, `pid_starttime`,
        // `job_object_name` keys; the daemon parses only the trio it
        // needs (`session_id`, `pid`, `pid_starttime`) and silently
        // ignores `pgid` / `job_object_name` so the wire shape can
        // grow additively as the daemon's per-platform process
        // bookkeeping fills in. Method name accepts the
        // launcher-emitted `session.report_process` underscore form
        // and the kebab-case wire discriminator
        // (`report-process` / `session.report-process`) so both
        // future-canonical and legacy spellings route to the same
        // handler. See `crates/anvil-run/src/spawn.rs::report_to_daemon`.
        "session.report_process"
        | "session.report-process"
        | "report-process"
        | "report_process" => params_object(params, method).and_then(|params| {
            let session_id = anvil_intercept_proto::SessionId::new(required_string(
                params,
                "session_id",
                method,
            )?);
            let pid_u64 = required_u64(params, "pid", method)?;
            let pid = u32::try_from(pid_u64).map_err(|_| {
                invalid_params(method, format!("pid must fit in u32, got {pid_u64}"))
            })?;
            let pid_starttime = required_u64(params, "pid_starttime", method)?;
            Ok(IpcCommand::ReportProcess {
                session_id,
                pid,
                pid_starttime,
            })
        }),
        "unblock-cascade" | "fence.unblock-cascade" => {
            params_object(params, method).and_then(|params| {
                let worktree = required_string(params, "worktree", method)?;
                // MLP2-026: any client-supplied `operator` is silently
                // ignored — the daemon derives it server-side from
                // peer credentials. Accept the shape on the wire so
                // future cross-host audit variants can opt in.
                Ok(IpcCommand::UnblockCascade {
                    worktree: PathBuf::from(worktree.as_str()),
                    operator: None,
                })
            })
        }
        "unblock-worktree" | "fence.unblock-worktree" => {
            params_object(params, method).and_then(|params| {
                let worktree = required_string(params, "worktree", method)?;
                Ok(IpcCommand::UnblockWorktree {
                    worktree: PathBuf::from(worktree.as_str()),
                })
            })
        }
        _ => Err(JsonRpcFailure {
            code: -32601,
            message: "Method not found",
            data: json!({"method": method}),
        }),
    }
}

fn validate_scan_buffer_request_shape(
    map: &serde_json::Map<String, Value>,
    method: &str,
) -> Result<(), JsonRpcFailure> {
    for key in map.keys() {
        if !matches!(
            key.as_str(),
            "jsonrpc" | "method" | "params" | "id" | "traceparent"
        ) {
            return Err(invalid_request(
                "scan_buffer requests only allow jsonrpc, method, params, id, and traceparent fields",
            ));
        }
    }

    let params = params_object(map.get("params").unwrap_or(&Value::Null), method)?;
    for key in params.keys() {
        // MLP2-025b adds `env_agent_tag` as the writer-side raw
        // ANVIL_AGENT_TAG carrier read by the daemon's spoof
        // cross-check. The TS driver-client emits this field when
        // `process.env.ANVIL_AGENT_TAG` is set and non-empty
        // (`packages/anvil-driver-client/.../validate-mid-edit.ts`),
        // so the daemon allowlist must accept it — without this,
        // tagged scan_buffer requests are rejected at the schema
        // gate before the cross-check can read the tag. Absent or
        // null tags intentionally take the `Cross::Untagged` path.
        if !matches!(
            key.as_str(),
            "path" | "text" | "version" | "mode" | "env_agent_tag" | "session_id"
        ) {
            return Err(invalid_params(
                method,
                "scan_buffer params only allow path, text, version, mode, env_agent_tag, and session_id fields",
            ));
        }
    }

    Ok(())
}

fn trace_method_label(method: &str) -> Cow<'_, str> {
    if method.len() <= MAX_TRACE_METHOD_LEN {
        Cow::Borrowed(method)
    } else {
        const ELLIPSIS: &str = "...";
        let max_prefix_len = MAX_TRACE_METHOD_LEN - ELLIPSIS.len();
        let mut end = 0;
        for (index, ch) in method.char_indices() {
            let next = index + ch.len_utf8();
            if next > max_prefix_len {
                break;
            }
            end = next;
        }
        Cow::Owned(format!("{}{ELLIPSIS}", &method[..end]))
    }
}

fn jsonrpc_dispatch_span(
    method: &str,
    is_notification: bool,
    trace_context: Option<&TraceContext>,
) -> tracing::Span {
    let method_label = trace_method_label(method);
    let dispatch_span = tracing::info_span!(
        target: "anvil_intercept::ipc",
        "jsonrpc.dispatch",
        method = %method_label,
        method_truncated = method_label.len() != method.len(),
        is_notification,
        trace_id = field::Empty,
        parent_id = field::Empty,
        trace_flags = field::Empty,
    );
    if let Some(context) = trace_context {
        bind_traceparent_to_span(&dispatch_span, context);
    }
    dispatch_span.in_scope(|| {
        tracing::info!(
            target: "anvil_intercept::ipc",
            "jsonrpc dispatch received"
        );
    });
    dispatch_span
}

/// MLP2-025b: run the write-time env-tag spoof cross-check. Returns
/// `Some(serialised_blocked_response)` when the request is spoofed and
/// should be short-circuited; `None` when the request should fall
/// through to the rule engine (untagged or matched).
///
/// Implements the §4 message-flow arrows 4–7b of
/// `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md`:
/// classify the env tag via `SessionRegistry::cross_check_env_tag`,
/// on `Cross::Spoofed` fence the worktree (via
/// `worktree_for_lineage` with file-parent fallback, per
/// MLP2-025b user-approved option (a)), emit notification +
/// `tracing::warn!`, and build a `ScanBufferResponse` carrying the
/// `SpoofBlockInfo`.
///
/// Fail-closed defaults (spec §7):
/// - `env_agent_tag` present + malformed → classify as `Spoofed`.
/// - `env_agent_tag` present + `peer_pid` `None` → classify as
///   `Spoofed` (the daemon cannot validate without a writer PID).
fn run_spoof_cross_check(
    request: &ScanBufferRequest,
    peer_pid: Option<u32>,
    ctx: &CrossCheckContext,
) -> Option<Value> {
    // Decode env tag at the boundary. Both absence and parse failure
    // fold to "no validated tag":
    // - absence: classify as `Cross::Untagged`, fall through.
    // - parse failure: classify as `Cross::Spoofed`, fail-closed.
    let raw = request.env_agent_tag.as_deref();
    let env_tag = match raw {
        None => None,
        Some(raw_value) => match anvil_attribution::env::agent_tag_from_env_value(raw_value) {
            Ok(tag) => Some(tag),
            Err(_) => {
                // Parse failure → treat as spoofed (fail-closed).
                return Some(spoof_block_response(request, peer_pid, ctx));
            }
        },
    };

    // Untagged writes are out of MLP2-025's scope — they follow the
    // pre-MLP2-025 enforcement path unchanged. `?` short-circuits to
    // `None` (the return type) when env_tag is None.
    let env_tag = env_tag?;

    // Env tag is `Some` but we have no peer PID → cannot validate
    // the lineage. Fail-closed (§7).
    let Some(writer_pid) = peer_pid else {
        return Some(spoof_block_response(request, None, ctx));
    };

    match ctx.registry.cross_check_env_tag(Some(&env_tag), writer_pid) {
        Cross::Match | Cross::Untagged => None,
        Cross::Spoofed => Some(spoof_block_response(request, Some(writer_pid), ctx)),
    }
}

/// MLP2-025b: build the response for a spoofed write. Resolves the
/// worktree to fence (option (a): registered ancestor's worktree
/// with file-parent fallback), records the fence, emits the
/// `degraded:spoofed-attribution` notification + `tracing::warn!`,
/// and serialises a `ScanBufferResponse` with `spoof_block`
/// populated.
fn spoof_block_response(
    request: &ScanBufferRequest,
    peer_pid: Option<u32>,
    ctx: &CrossCheckContext,
) -> Value {
    // Resolve the worktree to fence (option (a)):
    // 1. If we know the writer's PID, ask the registry for any
    //    registered ancestor's worktree (tag-agnostic).
    // 2. Fall back to the file's parent directory if no ancestor is
    //    registered. This is imperfect — the parent may not be a
    //    real worktree root — but it gives `is_fenced` something to
    //    match against and lets the operator-touch trail surface.
    let fence_target = peer_pid
        .and_then(|pid| ctx.registry.worktree_for_lineage(pid))
        .or_else(|| request.path.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| request.path.clone());

    // Record the fence. A failure here is logged via tracing::error
    // but does not change the verdict — the spoofed write is
    // ALWAYS blocked, even if the fence record could not be
    // persisted (spec §7 verdict).
    let fenced_worktree = match ctx.fence_store.fence_worktree_for_spoof(&fence_target) {
        Ok(record) => record.worktree,
        Err(err) => {
            tracing::error!(
                target: "anvil_intercept::ipc",
                reason = crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION,
                error = %err,
                "failed to record spoof fence; blocking write anyway",
            );
            fence_target.clone()
        }
    };

    // Emit the dual-channel telemetry (spec §8): tracing::warn!
    // for structured-log collectors, plus the notification path is
    // already covered by the fence-transition envelope the
    // FenceStore::fence_worktree_for_spoof call triggers via its
    // upstream notification bus (when wired).
    tracing::warn!(
        target: "anvil_intercept::ipc",
        reason = crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION,
        writer_pid = peer_pid,
        worktree = %fenced_worktree.display(),
        path = %request.path.display(),
        "blocking spoofed-attribution write and fencing worktree",
    );

    // Build the BlockedSpoofedAttribution response (spec §3.3).
    let response = midedit::ScanBufferResponse {
        version: request.version,
        diagnostics: Vec::new(),
        truncated: false,
        rules_sha: None,
        spoof_block: Some(SpoofBlockInfo {
            reason: crate::telemetry::DEGRADED_SPOOFED_ATTRIBUTION.to_string(),
            fenced_worktree,
        }),
    };
    // serde::to_value on this shape is infallible (only String /
    // PathBuf / numeric primitives) — unwrap matches the convention
    // elsewhere in this file.
    serde_json::to_value(&response).expect("ScanBufferResponse with SpoofBlockInfo serialises")
}

/// CLAWP-065: JSON-RPC error code for a `scan_buffer` request whose
/// claimed `session_id` does not match the connection's authenticated
/// peer-PID lineage. Server-defined per JSON-RPC 2.0 §5.1 (the
/// `-32000..=-32099` reserved range), sequenced after `-32000`
/// (Server busy) and `-32001` (Scan timed out).
const SCAN_BUFFER_SESSION_MISMATCH_CODE: i64 = -32002;

/// CLAWP-065: validate that a `scan_buffer` request claiming the
/// `claimed` session id was issued from a connection whose
/// authenticated peer lineage owns that session.
///
/// Fail-closed on every path the daemon cannot positively attribute to
/// the claimed session:
/// - no authenticated `peer_pid` (the OS gave us no peer credential) →
///   the claim cannot be bound to any lineage → reject;
/// - the writer's PID lineage carries no registered session → the
///   claim is unverifiable → reject (an unbound claim is not a free
///   pass);
/// - the lineage resolves to a *different* registered session →
///   cross-session forgery → reject.
///
/// The error `data` echoes the (client-supplied) claimed id and a
/// machine-readable `reason`, but never the owning session id —
/// disclosing it would leak another session's identity to a caller
/// that just proved it does not own it.
fn validate_scan_buffer_session_ownership(
    claimed: &str,
    peer_pid: Option<u32>,
    registry: &SessionRegistry,
) -> Result<(), JsonRpcFailure> {
    let Some(writer_pid) = peer_pid else {
        return Err(session_ownership_mismatch(
            claimed,
            "peer-credentials-unavailable",
        ));
    };
    match registry.session_for_lineage(writer_pid) {
        Some(owner) if owner.as_str() == claimed => Ok(()),
        Some(_) => Err(session_ownership_mismatch(
            claimed,
            "session-lineage-mismatch",
        )),
        None => Err(session_ownership_mismatch(
            claimed,
            "no-registered-session-on-lineage",
        )),
    }
}

/// CLAWP-065: build the structured session-ownership rejection. See
/// [`validate_scan_buffer_session_ownership`] for the disclosure rule.
fn session_ownership_mismatch(claimed: &str, reason: &'static str) -> JsonRpcFailure {
    JsonRpcFailure {
        code: SCAN_BUFFER_SESSION_MISMATCH_CODE,
        message: "Session ownership mismatch",
        data: json!({
            "reason": reason,
            "claimed_session_id": claimed,
        }),
    }
}

async fn scan_buffer_from_jsonrpc(
    params: &Value,
    method: &str,
    traceparent: Option<&str>,
    scan_buffer: &ScanBufferService,
    peer_pid: Option<u32>,
    cross_check: Option<&CrossCheckContext>,
) -> Result<Value, JsonRpcFailure> {
    let request = {
        let params = params_object(params, method)?;
        let path = required_string(params, "path", method)?;
        midedit::validate_scan_buffer_path(&path)
            .map_err(|err| invalid_params(method, err.to_string()))?;
        let text = required_string(params, "text", method)?;
        let version = required_u64(params, "version", method)?;
        let mode = required_string(params, "mode", method)?;
        let mode =
            ScanBufferMode::parse(&mode).map_err(|err| invalid_params(method, err.to_string()))?;

        // MLP2-025b: optional env-supplied AgentTag carried as raw
        // string. Same shape contract as `agent_tag` on
        // register-session — absent or null both fold to `None`;
        // value type must be string. Daemon decodes at the boundary
        // (B7) so malformed values fold to `Cross::Spoofed`.
        let env_agent_tag = match params.get("env_agent_tag") {
            Some(value) if value.is_null() => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(invalid_params(
                    method,
                    "env_agent_tag must be a string or null".to_string(),
                ));
            }
            None => None,
        };
        // CLAWP-065: optional authenticated session binding. Same shape
        // contract as `env_agent_tag` — absent or null both fold to
        // `None`; any non-string value is a hard parse failure. When
        // present, the daemon binds it to the connection's peer lineage
        // below. Bounded at `MAX_SCAN_BUFFER_SESSION_ID_BYTES` here so a
        // normal-sized frame (whose cap, `MAX_LINE_BYTES`, is far
        // larger) cannot smuggle a multi-megabyte id that the daemon
        // would clone and echo back verbatim in the rejection's
        // `claimed_session_id` — this matches the cap the oversized
        // fast-path already enforces.
        let session_id = match params.get("session_id") {
            Some(value) if value.is_null() => None,
            Some(Value::String(s)) => {
                if s.len() > MAX_SCAN_BUFFER_SESSION_ID_BYTES {
                    return Err(invalid_params(
                        method,
                        format!("session_id exceeds {MAX_SCAN_BUFFER_SESSION_ID_BYTES} byte cap"),
                    ));
                }
                Some(s.clone())
            }
            Some(_) => {
                return Err(invalid_params(
                    method,
                    "session_id must be a string or null".to_string(),
                ));
            }
            None => None,
        };
        ScanBufferRequest {
            path: PathBuf::from(path),
            text,
            version,
            mode,
            env_agent_tag,
            session_id,
        }
    };

    // CLAWP-065: session-ownership binding. A request that claims a
    // `session_id` must arrive on a connection whose authenticated
    // peer-PID lineage resolves to that same session; otherwise the
    // daemon cannot tell a legitimate mid-edit scan from one forged
    // under another session's identity, and rejects it with a
    // structured error. Enforced only when the daemon has a
    // cross-check context (i.e. a live session registry) — embedded /
    // legacy listeners with no registry have no session model to bind
    // against, matching the spoof-check posture below. An absent
    // `session_id` keeps the pre-CLAWP-065 unbound path, so today's
    // driver (which sends none) is unaffected. Runs BEFORE the spoof
    // cross-check so an unauthorised session claim fails fast without
    // triggering a worktree fence as a side effect.
    if let Some(ctx) = cross_check
        && let Some(claimed) = request.session_id.as_deref()
    {
        validate_scan_buffer_session_ownership(claimed, peer_pid, &ctx.registry)?;
    }

    // MLP2-025b: run the write-time spoof cross-check before the
    // rule engine. When `cross_check` is `None` the daemon is in a
    // legacy / test configuration and the cross-check is skipped.
    if let Some(ctx) = cross_check
        && let Some(spoof_response) = run_spoof_cross_check(&request, peer_pid, ctx)
    {
        return Ok(spoof_response);
    }
    let scan_span = tracing::info_span!(
        target: "anvil_intercept::ipc",
        "jsonrpc.scan_buffer",
        path_basename = scan_buffer_path_basename(&request.path),
        mode = ?request.mode,
        version = request.version,
    );

    // MLP2-006: capture inputs the Kindling notification fan-out
    // needs (file path + start timestamp). We measure elapsed on
    // both sides of the await so the row's `duration_ms` reflects
    // the daemon's full handle of the call (matches the
    // `validation.service` aggregator surface).
    let file_path = request.path.to_string_lossy().into_owned();
    let scan_mode = request.mode;
    let started = Instant::now();

    let result = async {
        tracing::info!(target: "anvil_intercept::ipc", "scan_buffer dispatched");
        scan_buffer.scan_buffer(request).await
    }
    .instrument(scan_span)
    .await
    .map_err(|err| scan_buffer_failure(method, &err))?;

    // MLP2-006: emit the `gate_evaluated` Kindling row. Mid-edit
    // calls only — pre-write samples are a separate budget class
    // per ADR-031 and would mix observation kinds. Failure is
    // logged inside the emitter and never bubbles back to the
    // caller, so the scan response always reaches the driver.
    if matches!(scan_mode, ScanBufferMode::MidEdit)
        && let Some(emitter) = scan_buffer.observation_emitter()
    {
        let elapsed = started.elapsed();
        let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let gate_eval_id = derive_gate_eval_id(traceparent);
        let emission = MidEditEmissionRequest {
            gate_eval_id: &gate_eval_id,
            file_path: &file_path,
            timestamp: &timestamp,
            duration_ms,
        };
        let _ = emitter.try_emit(&emission, &result, Instant::now());
    }

    serde_json::to_value(result).map_err(|err| JsonRpcFailure {
        code: -32603,
        message: "Internal error",
        data: json!({"error": err.to_string()}),
    })
}

/// MLP2-006: derive a stable `gate_eval_id` from the JSON-RPC
/// envelope's `traceparent` so the Kindling row joins back to the
/// originating telemetry span (MLP2-008 contract). The W3C
/// parent-id (16 lower-hex chars) is the upstream span id and is
/// what consumers join against. Falls back to a fresh UUID v4 when
/// the extractor yields `None` — i.e. the producer omitted
/// `traceparent`, or supplied one that does not parse — so the row is
/// never emitted with a placeholder id.
///
/// MLP2-008: the parent-id extraction itself lives in
/// [`crate::kindling_observation::gate_eval_id_from_traceparent`] — the
/// single source shared with the RTAI-007 telemetry envelope — so a
/// Kindling row and its mid-edit telemetry envelope carry identical
/// join keys. This wrapper only adds the row-side UUID-v4 fallback.
fn derive_gate_eval_id(traceparent: Option<&str>) -> String {
    crate::kindling_observation::gate_eval_id_from_traceparent(traceparent)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

fn scan_buffer_path_basename(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("<unknown>")
}

fn scan_buffer_failure(method: &str, err: &midedit::ScanBufferError) -> JsonRpcFailure {
    match err {
        midedit::ScanBufferError::UnsupportedMode
        | midedit::ScanBufferError::PathTooLong { .. }
        | midedit::ScanBufferError::InvalidPath
        | midedit::ScanBufferError::ContentTooLarge { .. } => {
            invalid_params(method, err.to_string())
        }
        midedit::ScanBufferError::Busy => JsonRpcFailure {
            code: -32000,
            message: "Server busy",
            data: json!({"error": err.to_string()}),
        },
        midedit::ScanBufferError::TimedOut => JsonRpcFailure {
            code: -32001,
            message: "Scan timed out",
            data: json!({"error": err.to_string()}),
        },
        midedit::ScanBufferError::ServiceUnavailable
        | midedit::ScanBufferError::WorkerFailed(_) => JsonRpcFailure {
            code: -32603,
            message: "Internal error",
            data: json!({"error": err.to_string()}),
        },
    }
}

fn params_object<'a>(
    params: &'a Value,
    method: &str,
) -> Result<&'a serde_json::Map<String, Value>, JsonRpcFailure> {
    params
        .as_object()
        .ok_or_else(|| invalid_params(method, "params must be an object"))
}

fn required_string(
    params: &serde_json::Map<String, Value>,
    field: &str,
    method: &str,
) -> Result<String, JsonRpcFailure> {
    params
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| invalid_params(method, format!("{field} must be a string")))
}

fn required_u64(
    params: &serde_json::Map<String, Value>,
    field: &str,
    method: &str,
) -> Result<u64, JsonRpcFailure> {
    params
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params(method, format!("{field} must be an unsigned integer")))
}

fn invalid_params(method: &str, reason: impl Into<String>) -> JsonRpcFailure {
    JsonRpcFailure {
        code: -32602,
        message: "Invalid params",
        data: json!({"method": method, "reason": reason.into()}),
    }
}

fn invalid_request(reason: impl Into<String>) -> JsonRpcFailure {
    JsonRpcFailure {
        code: -32600,
        message: "Invalid Request",
        data: json!({"reason": reason.into()}),
    }
}

/// Read a single newline-terminated line from `reader` into `buf`,
/// rejecting any line that exceeds [`MAX_LINE_BYTES`]. Returns the
/// number of bytes read (0 on clean EOF).
async fn read_one_line<R>(reader: &mut BufReader<R>, buf: &mut String) -> Result<usize, IpcError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    // We can't use `read_line` directly because it has no size cap.
    // Read raw bytes until `\n`, bail if we exceed the cap, then
    // convert to UTF-8.
    let mut raw: Vec<u8> = Vec::with_capacity(256);
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            // EOF.
            if raw.is_empty() {
                return Ok(0);
            }
            // Trailing line without a newline: still parse it.
            break;
        }
        if let Some(idx) = chunk.iter().position(|&b| b == b'\n') {
            let take = idx + 1;
            if raw.len() + take > MAX_LINE_BYTES {
                return Err(IpcError::OversizedLine);
            }
            raw.extend_from_slice(&chunk[..take]);
            reader.consume(take);
            break;
        }
        if raw.len() + chunk.len() > MAX_LINE_BYTES {
            return Err(IpcError::OversizedLine);
        }
        raw.extend_from_slice(chunk);
        let consumed = chunk.len();
        reader.consume(consumed);
    }

    let len = raw.len();
    let s = String::from_utf8(raw).map_err(|_| IpcError::InvalidUtf8 { len })?;
    buf.push_str(&s);
    Ok(s.len())
}

fn dispatch_envelope<D: SessionDispatcher>(envelope: &IpcEnvelope, dispatcher: &Arc<D>) {
    // Notification vs request: the proto layer pins
    // `{"id": null, ...}` as identical to a missing `id`. Both yield
    // `id: None`. We do not branch on `id.is_some()` to distinguish
    // dispatch behaviour — only response routing (a future task)
    // would consult the field, and only after dispatch.
    // MLP2-026: the NDJSON dispatch_envelope path is a legacy
    // wire that does not carry the cross-check capability or
    // peer-pid context — UnblockCascade is JSON-RPC-only and
    // returns "method not found" on the NDJSON path until the
    // legacy surface is retired. Pass None for both.
    let result = dispatch_command(&envelope.command, dispatcher, None, None).map(|_| ());
    if let Err(err) = result {
        tracing::warn!(target: "anvil_intercept::ipc", error = %err, "dispatcher returned error");
        eprintln!("anvil-intercept: dispatcher returned error: {err}");
    }
}

/// MLP2-070 / #1674: re-derive the `LineageAnchor` for a
/// `register-session` request from the connection's authenticated
/// peer credentials, rejecting any wire body whose `pid` claim does
/// not match the peer.
///
/// On Linux the `pid_starttime` is also read server-side from
/// `/proc/<peer_pid>/stat`; the client's value is ignored even when
/// present. On non-Linux platforms the daemon has no portable
/// server-side `pid_starttime` reader yet (the APS spec calls for
/// `proc_pidinfo` on macOS and `GetProcessTimes` on Windows), and
/// `SessionRegistry::lookup_tag_for_lineage` is already inert on
/// those platforms because the per-PID `pid_starttime` read returns
/// `Unsupported`. To avoid turning the existing silent-inertness
/// into a hard register-time failure, non-Linux platforms still pin
/// the `pid == peer_pid` trust gate (which is the primary forgery
/// defence) but forward the client-supplied `pid_starttime` as
/// advisory. Lineage gains the daemon-derived starttime guarantee
/// when full cross-platform support lands.
///
/// Returns the verified anchor on success, or a human-readable
/// rejection string when:
/// - `peer_pid` is `None` (no authenticated peer — fail-closed; the
///   legacy NDJSON wire and any platform without `SO_PEERCRED`-style
///   peer-credential reads cannot prove the claim, so no lineage is
///   accepted on those paths);
/// - the claim's `pid` is not the peer's pid (a same-UID caller
///   trying to mint a lineage anchor for someone else's PID, which
///   was the trust-boundary defect `DeepSec` flagged as #1674); or
/// - on Linux only, the daemon cannot read `pid_starttime` for the
///   peer (e.g. the peer exited between accept and verify —
///   best-effort fail-closed).
fn verify_lineage_claim(
    claim: &anvil_intercept_proto::session::LineageAnchor,
    peer_pid: Option<u32>,
) -> Result<anvil_intercept_proto::session::LineageAnchor, String> {
    let Some(peer_pid) = peer_pid else {
        return Err(
            "register-session lineage rejected: peer credentials unavailable on this \
             connection — only authenticated launchers may seed the daemon's \
             lineage index (MLP2-070 / issue #1674)"
                .to_string(),
        );
    };
    if claim.pid != peer_pid {
        return Err(format!(
            "register-session lineage rejected: claim.pid={} does not match \
             authenticated peer pid={} (MLP2-070 / issue #1674)",
            claim.pid, peer_pid,
        ));
    }
    #[cfg(target_os = "linux")]
    {
        let pid_starttime = anvil_attribution::process::pid_starttime(peer_pid).map_err(|err| {
            format!(
                "register-session lineage rejected: cannot read pid_starttime for \
                     peer pid={peer_pid}: {err}"
            )
        })?;
        if claim.pid_starttime != pid_starttime {
            // The client's claim is treated as advisory; the
            // registry is always seeded with the value the daemon
            // read itself. A mismatch is logged at debug rather
            // than warn — well-behaved launchers can disagree
            // benignly (clock-tick rounding), and operators have
            // no actionable response.
            tracing::debug!(
                target: "anvil_intercept::ipc",
                claim_pid_starttime = claim.pid_starttime,
                daemon_pid_starttime = pid_starttime,
                peer_pid,
                "register-session lineage pid_starttime claim differs from daemon read; trusting daemon",
            );
        }
        Ok(anvil_intercept_proto::session::LineageAnchor {
            pid: peer_pid,
            pid_starttime,
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        // No portable server-side reader on this platform yet (see
        // function-level rustdoc). Forward the claim's
        // pid_starttime advisory while preserving the pid trust
        // gate. The lineage lookup is inert here today, so this
        // matches the pre-MLP2-070 wire-state without introducing a
        // new hard failure on macOS/Windows.
        Ok(anvil_intercept_proto::session::LineageAnchor {
            pid: peer_pid,
            pid_starttime: claim.pid_starttime,
        })
    }
}

/// MLP2-074 (PR #1895 review): re-derive the child's `pid_starttime`
/// server-side on Linux instead of trusting the launcher's wire
/// value. Mirrors the trust-boundary defence
/// [`verify_lineage_claim`] applies on the register path — the
/// launcher claim is advisory; the authoritative value is the one
/// the daemon reads from `/proc/<child_pid>/stat`. Without this
/// re-derivation a launcher could pin an arbitrary `pid_starttime`
/// against a real child pid and evade either MLP-014's PID-reuse
/// defence or the MLP2-025 lineage walk (the walk reads
/// `pid_starttime` live and would see a fresh value that no longer
/// matches the index).
///
/// On non-Linux platforms the daemon has no portable
/// `pid_starttime` reader yet (the spec calls for `proc_pidinfo` on
/// macOS and `GetProcessTimes` on Windows), so we forward the
/// client-supplied value as advisory — matches the existing
/// non-Linux branch of `verify_lineage_claim`.
///
/// Returns the trusted value on success, or a human-readable
/// rejection string on Linux when `pid_starttime(child_pid)` cannot
/// be read (e.g. the child exited between the launcher's spawn and
/// the daemon's read — fail-closed: the index would otherwise be
/// keyed on an attacker-chosen value).
fn verify_report_process_starttime(
    child_pid: u32,
    advisory_starttime: u64,
    _peer_pid: u32,
) -> Result<u64, String> {
    #[cfg(target_os = "linux")]
    {
        let pid_starttime =
            anvil_attribution::process::pid_starttime(child_pid).map_err(|err| {
                format!(
                    "session.report_process rejected: cannot read pid_starttime \
                     for child pid={child_pid}: {err} (MLP2-074 / PR #1895)"
                )
            })?;
        if advisory_starttime != pid_starttime {
            // Client claim is advisory; the daemon's read wins.
            // Log at debug so an operator chasing a discrepancy
            // has a breadcrumb without flooding warn-level output
            // on benign clock-tick rounding.
            tracing::debug!(
                target: "anvil_intercept::ipc",
                claim_pid_starttime = advisory_starttime,
                daemon_pid_starttime = pid_starttime,
                child_pid,
                "session.report_process pid_starttime claim differs from daemon read; trusting daemon",
            );
        }
        Ok(pid_starttime)
    }
    #[cfg(not(target_os = "linux"))]
    {
        // No portable server-side reader on this platform yet.
        // Forward the launcher's advisory value. Matches the
        // non-Linux branch of `verify_lineage_claim`.
        let _ = child_pid;
        Ok(advisory_starttime)
    }
}

/// CIB-150: the `claimed_agent_id` an activation-spine (durable
/// membership) claim is rewritten to when the connection's authenticated
/// peer is not entitled to register durable membership. Any value other
/// than `ACTIVATION_SPINE_CLAIMED_AGENT_ID` makes
/// `AgentTag::is_durable_membership` return `false`, so the downgraded
/// session drops onto the ordinary live-lease path (per-worktree cap +
/// heartbeat TTL) instead of the persisted, TTL-exempt membership set.
const DOWNGRADED_ACTIVATION_SPINE_CLAIMED_AGENT_ID: &str = "unverified-activation-spine";

/// CIB-150: authorise a wire `AgentTag` that claims durable worktree
/// membership (an activation-spine `claimed_agent_id`) against the
/// connection's authenticated peer before the daemon honours it.
///
/// Durable membership is persisted under `ANVIL_HOME`, exempt from the
/// heartbeat TTL, and drawn from a separate `registered_worktree_cap`
/// budget (ACTMO-014 / ADR-094). `AgentTag` is not authenticated
/// identity: any same-UID process can mint one claiming the
/// activation-spine id, so trusting the wire value verbatim would let an
/// unprivileged neighbour consume the durable budget and pin persisted
/// membership it never legitimately started (the trust-boundary gap
/// `DeepSec` flagged, split from CIB-113).
///
/// The daemon and the activation spine (`anvil start`,
/// `anvil workspace register`) are the *same* `anvil` binary, so the
/// authorisation test mirrors [`verify_lineage_claim`]'s peer-derivation:
/// on Linux the peer's `/proc/<peer_pid>/exe` is compared against the
/// daemon's own canonicalised [`std::env::current_exe`]. A missing peer
/// credential (legacy NDJSON wire, no `SO_PEERCRED`) or any non-Linux
/// platform (no portable peer-exe reader yet) is treated as *not*
/// authorised — matching the fail-closed, Linux-only posture of
/// `verify_lineage_claim`. The Linux path additionally refuses to trust
/// the comparison at all unless it has proven the kernel reports a foreign
/// pid's exe faithfully (see [`peer_authorised_for_durable_membership`]),
/// so a sandbox that aliases foreign `/proc/<pid>/exe` reads to the
/// reader's own binary cannot silently turn the gate into "always
/// authorised".
///
/// Returns [`None`] when the caller's tag should register unchanged — it
/// is not a durable claim, or the peer is authorised. Returns
/// [`Some`]`(downgraded)` when a durable claim from an unauthorised (or
/// unverifiable) peer must be **downgraded** to a copy whose
/// `claimed_agent_id` no longer marks durable membership. The session
/// still registers — as an ordinary live lease — rather than being
/// rejected outright, so a benign mis-tagged client is not locked out
/// (CIB-150 Expected Outcome). Returning `None` on the common path avoids
/// cloning the tag on every `RegisterSession`. The daemon's in-process
/// `register_on_start` path never routes through this dispatcher, so
/// legitimate startup durable registration is untouched — and remains the
/// durable path even where the wire gate fails closed.
fn verify_durable_membership_claim(
    tag: &anvil_intercept_proto::session::AgentTag,
    peer_pid: Option<u32>,
) -> Option<anvil_intercept_proto::session::AgentTag> {
    if !tag.is_durable_membership() || peer_authorised_for_durable_membership(peer_pid) {
        return None;
    }
    tracing::debug!(
        target: "anvil_intercept::ipc",
        driver_id = %tag.driver_id,
        claimed_agent_id = %tag.claimed_agent_id,
        peer_pid = ?peer_pid,
        "register-session durable-membership claim from an unauthorised peer downgraded to a live session (CIB-150)",
    );
    Some(anvil_intercept_proto::session::AgentTag::new(
        tag.driver_id.clone(),
        DOWNGRADED_ACTIVATION_SPINE_CLAIMED_AGENT_ID,
        tag.pid_starttime,
    ))
}

/// CIB-150: `true` when the authenticated peer behind an IPC connection
/// is entitled to register durable worktree membership — i.e. it is
/// running the *same* `anvil` binary as the daemon (the CLI and daemon
/// ship as one executable). On Linux this compares the peer's
/// `/proc/<peer_pid>/exe` symlink target against the daemon's
/// canonicalised `current_exe`. Fail-closed on every uncertainty: a
/// missing peer pid, an unreadable peer or daemon exe path, or a
/// non-Linux platform (no portable peer-exe reader yet) all return
/// `false`.
///
/// The comparison is only meaningful if the kernel reports a *foreign*
/// process's `/proc/<pid>/exe` faithfully. Some sandboxed runtimes
/// (gVisor-style micro-VMs, seen on the CI runner for this change) alias a
/// foreign pid's `exe` symlink to the *reading* process's own binary, so
/// every same-uid peer — an unrelated `sleep`, a forger, anything — reads
/// back as the daemon's `anvil` binary and the gate silently degrades to
/// "always authorised": the exact durable-budget bypass CIB-150 closes.
/// Comparing `stat` device+inode instead of the path string does not help,
/// because the alias fabricates the whole resolved file, not just its
/// name. We therefore probe this capability once per process
/// ([`foreign_exe_reads_faithful`]) and, when it cannot be trusted, refuse
/// every durable claim over the wire. That is fail-closed: legitimate
/// durable registration also flows through the daemon's in-process
/// `register_on_start` path (and `anvil workspace register --persist`),
/// which never crosses this dispatcher, so durability is preserved even
/// where the wire gate is forced strict.
fn peer_authorised_for_durable_membership(peer_pid: Option<u32>) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Some(peer_pid) = peer_pid else {
            return false;
        };
        // Refuse to trust the exe comparison unless this environment has
        // demonstrably reported a foreign pid's exe faithfully. Otherwise
        // the equality below would be forced true for any same-uid peer.
        if !foreign_exe_reads_faithful() {
            return false;
        }
        let Ok(peer_exe) = std::fs::read_link(format!("/proc/{peer_pid}/exe")) else {
            return false;
        };
        let Ok(daemon_exe) = std::env::current_exe().and_then(std::fs::canonicalize) else {
            return false;
        };
        // `/proc/<pid>/exe` already resolves to the canonical target, but
        // canonicalise defensively so both sides are normalised the same
        // way; fall back to the raw link target if it no longer resolves
        // (e.g. the binary was replaced on disk) — that simply fails the
        // equality check, which is the safe answer.
        let peer_exe = std::fs::canonicalize(&peer_exe).unwrap_or(peer_exe);
        peer_exe == daemon_exe
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = peer_pid;
        false
    }
}

/// CIB-150: `true` when this process can read a *foreign* pid's
/// `/proc/<pid>/exe` and get that process's real executable rather than an
/// alias of our own binary. Computed once and cached: the answer is a
/// property of the kernel/sandbox, not of any individual peer.
///
/// Used by [`peer_authorised_for_durable_membership`] to fail closed on
/// sandboxes that fabricate foreign exe reads (see that function's docs).
#[cfg(target_os = "linux")]
fn foreign_exe_reads_faithful() -> bool {
    static FAITHFUL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FAITHFUL.get_or_init(probe_foreign_exe_reads_faithful)
}

/// Probe whether foreign `/proc/<pid>/exe` reads are faithful by spawning
/// a short-lived canary that is guaranteed *not* to be this binary and
/// checking that the kernel reports the canary's exe as something other
/// than our own. A sandbox that aliases foreign reads to the reader's
/// binary returns our own exe here, which we treat as unfaithful.
///
/// Fail-closed on every uncertainty (cannot resolve our own exe, cannot
/// spawn the canary, cannot read its exe): the caller then refuses durable
/// wire claims, which is the safe answer.
#[cfg(target_os = "linux")]
fn probe_foreign_exe_reads_faithful() -> bool {
    let Ok(daemon_exe) = std::env::current_exe().and_then(std::fs::canonicalize) else {
        return false;
    };
    // `sleep` is POSIX, never this binary, and lives long enough to read
    // `/proc/<pid>/exe` before we reap it. If it is somehow absent we fail
    // closed rather than assume the kernel is trustworthy.
    let Ok(mut canary) = std::process::Command::new("sleep")
        .arg("30")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    let canary_pid = canary.id();
    let observed = std::fs::read_link(format!("/proc/{canary_pid}/exe")).ok();
    let _ = canary.kill();
    let _ = canary.wait();
    let Some(observed) = observed else {
        return false;
    };
    let observed = std::fs::canonicalize(&observed).unwrap_or(observed);
    // Faithful iff the canary's exe reads back as something other than our
    // own binary. Equal ⇒ the read was aliased to the reader ⇒ unfaithful.
    observed != daemon_exe
}

#[allow(clippy::too_many_arguments)]
// MLP2-026 adds peer_pid + cross_check; the chain is per-connection state, not bundleable without churn across callers.
#[allow(clippy::too_many_lines)] // MLP2-074 adds the ReportProcess arm; this is the canonical per-variant dispatch table and inlining keeps the routing visible at one glance.
fn dispatch_command<D: SessionDispatcher>(
    command: &IpcCommand,
    dispatcher: &Arc<D>,
    // MLP2-026: per-connection peer credentials. Used by
    // UnblockCascade to derive the OperatorContext audit field
    // server-side (spec §3.3 + §5.4).
    peer_pid: Option<u32>,
    // MLP2-026: optional fence-store handle for UnblockCascade.
    // None in tests / embedded callers; production wires via the
    // existing CrossCheckContext bundle.
    cross_check: Option<&CrossCheckContext>,
) -> Result<Value, String> {
    match command {
        IpcCommand::RegisterSession {
            session_id,
            worktree,
            agent_tag,
            lineage,
        } => {
            // MLP2-070 / #1674: re-derive the lineage anchor from the
            // authenticated peer rather than trusting the wire body.
            // The launcher is registering itself, so the only
            // legitimate `lineage.pid` is the peer's pid; the
            // `pid_starttime` is always read by the daemon from
            // `/proc/<peer_pid>/stat`, never accepted from the
            // client. Frames that disagree, or that supply lineage
            // over a connection with no authenticated peer, are
            // rejected before the registry is touched.
            let verified_lineage = match lineage.as_ref() {
                Some(claim) => Some(verify_lineage_claim(claim, peer_pid)?),
                None => None,
            };
            // CIB-150: an activation-spine tag claims durable worktree
            // membership (persisted, TTL-exempt, drawn from the separate
            // `registered_worktree_cap`). Honour it only when the
            // authenticated peer is the daemon's own `anvil` binary;
            // otherwise downgrade to an ordinary live session before the
            // registry keys durability off the tag, so a forged claim
            // cannot consume the durable budget. The claim is downgraded,
            // not rejected, so a benign mis-tagged client still registers.
            // `verify_durable_membership_claim` returns `Some(downgraded)`
            // only when a durable claim must be neutralised; `None` means
            // forward the caller's tag unchanged (the common path, no
            // clone).
            let downgraded_tag = agent_tag
                .as_ref()
                .and_then(|tag| verify_durable_membership_claim(tag, peer_pid));
            let verified_tag = downgraded_tag.as_ref().or(agent_tag.as_ref());
            dispatcher
                .register(
                    session_id,
                    worktree,
                    verified_tag,
                    verified_lineage.as_ref(),
                )
                .map_err(|err| err.to_string())?;
            // MLP2-071 D3: bind the registering peer as the session's
            // telemetry owner. The binding is the SubscriberId minted
            // from the connecting peer's authenticated credentials —
            // never a wire-supplied value (mirrors the MLP2-070 lineage
            // anchor pattern). The same peer later minting an identical
            // id on `SubscribeTelemetry` is what makes own-session
            // delivery work; a different same-UID peer mints a
            // different id and is denied. We need the live registry to
            // store the binding, which the production daemon supplies
            // via `cross_check`; embedded/legacy callers (no
            // `cross_check`) skip binding and the resolver
            // default-denies, which is the safe answer.
            if let (Some(ctx), Some(subscriber)) = (cross_check, mint_subscriber_id(peer_pid)) {
                ctx.registry
                    .bind_subscriber(session_id, subscriber.as_str().to_owned());
            }
            Ok(json!({"ok": true}))
        }
        IpcCommand::Heartbeat { session_id } => {
            // CIB-153: bind the heartbeat to the registering peer. The
            // dispatcher rejects a `peer_pid` that does not match the
            // session's stamped launcher pid, and fails closed on a
            // missing peer credential — same ownership contract as
            // `ReportProcess`. The typed `PeerOwnershipMismatch` is
            // mapped to the wire error string here.
            dispatcher
                .heartbeat(session_id, peer_pid)
                .map_err(|err| err.to_string())?;
            Ok(json!({"ok": true}))
        }
        IpcCommand::UnregisterSession { session_id } => Ok(json!({
            // CIB-153: same registering-peer binding as `Heartbeat` so a
            // same-UID neighbour cannot force-unregister a session it
            // never registered.
            "removed": dispatcher
                .unregister(session_id, peer_pid)
                .map_err(|err| err.to_string())?,
        })),
        IpcCommand::ReportProcess {
            session_id,
            pid,
            pid_starttime,
        } => {
            // MLP2-074: lineage-anchor narrowing requires an
            // authenticated peer pid so the daemon can prove the
            // caller is the launcher that registered the session
            // (not a same-UID neighbour). The legacy NDJSON wire and
            // any platform without `SO_PEERCRED`-style peer reads
            // surface `peer_pid: None`; reject those so we never
            // narrow on unauthenticated input.
            let Some(peer_pid) = peer_pid else {
                return Err(
                    "session.report_process requires authenticated peer credentials \
                     (MLP2-074)"
                        .to_owned(),
                );
            };
            // PR #1895 review: mirror MLP2-070's
            // `verify_lineage_claim` Linux behaviour — the
            // launcher-supplied `pid_starttime` is advisory; the
            // authoritative value is the one the daemon reads from
            // `/proc/<child_pid>/stat`. Trusting the wire value
            // would let a malicious launcher pin a chosen
            // pid_starttime against a real child pid and either
            // smuggle the anchor past PID-reuse defence or evade
            // future lineage lookups by mis-matching the value
            // `cross_check_env_tag` reads at write time. On
            // non-Linux the daemon has no portable starttime
            // reader yet (same caveat as MLP2-070), so the wire
            // value is forwarded as advisory.
            let trusted_starttime =
                verify_report_process_starttime(*pid, *pid_starttime, peer_pid)?;
            dispatcher
                .report_process(session_id, *pid, trusted_starttime, peer_pid)
                .map_err(|err| err.to_string())?;
            Ok(json!({"ok": true}))
        }
        IpcCommand::UnblockCascade {
            worktree,
            // MLP2-026: client-supplied operator field is silently
            // overwritten server-side. Spec §3.3 + §5.4.
            operator: _client_supplied,
        } => {
            let ctx = cross_check
                .ok_or_else(|| "unblock-cascade requires a daemon-backed fence store".to_owned())?;
            let cleared = ctx
                .fence_store
                .clear_cascade(worktree)
                .map_err(|err| err.to_string())?;
            if cleared {
                let operator = build_operator_context(peer_pid);
                tracing::info!(
                    target: "anvil_intercept::fence",
                    reason = crate::telemetry::DEGRADED_FENCE_CASCADE_CLEAR,
                    worktree = %worktree.display(),
                    ?operator,
                    "cascade cleared by operator",
                );
            }
            Ok(json!({"ok": cleared}))
        }
        IpcCommand::UnblockWorktree { worktree } => {
            // RCLI3-017b: per-fence unblock. Delegates to
            // `FenceStore::unblock_worktree`, which is responsible
            // for canonicalising the path, removing the in-memory
            // record, and rewriting the disk persistence file atomically.
            // Returns `ok: true` when a fence was actually removed,
            // `ok: false` when none was engaged (idempotent no-op).
            let ctx = cross_check.ok_or_else(|| {
                "unblock-worktree requires a daemon-backed fence store".to_owned()
            })?;
            let removed = ctx
                .fence_store
                .unblock_worktree(worktree)
                .map_err(|err| err.to_string())?;
            let cleared = removed.is_some();
            if cleared {
                let operator = build_operator_context(peer_pid);
                tracing::info!(
                    target: "anvil_intercept::fence",
                    worktree = %worktree.display(),
                    ?operator,
                    "fence cleared by operator",
                );
            }
            Ok(json!({"ok": cleared}))
        }
        IpcCommand::ListSessions => {
            let sessions = dispatcher.list();
            serde_json::to_value(sessions).map_err(|err| err.to_string())
        }
        IpcCommand::QueryStatus => {
            // INTD-011: the legacy NDJSON path does not carry
            // `query_status` payloads — drivers send the JSON-RPC
            // `query_status` method directly (see
            // `handle_query_status_jsonrpc`). The variant exists in
            // `IpcCommand` so future NDJSON consumers can opt in
            // without a proto break, but here the dispatch returns
            // a method-not-found-shaped error so an accidentally
            // routed frame surfaces honestly instead of silently
            // returning `null`.
            Err(
                "query_status is a JSON-RPC-only method; use the query_status JSON-RPC frame"
                    .to_owned(),
            )
        }
        // MLP2-071 (INTD-015 wire-up): subscribe / unsubscribe are
        // connection-state mutations, not command-response shapes —
        // the per-connection handler in `handle_jsonrpc_command`
        // intercepts them before reaching this dispatcher, because
        // mutating subscription state requires the per-connection
        // peer credentials + outbound channel that the generic
        // command dispatcher does not have. Reaching this arm means
        // a caller routed a `SubscribeTelemetry` / `UnsubscribeTelemetry`
        // frame through the legacy NDJSON dispatcher that bypasses
        // the JSON-RPC path; surface that honestly rather than
        // silently no-op.
        IpcCommand::SubscribeTelemetry { .. } | IpcCommand::UnsubscribeTelemetry => Err(
            "subscribe-telemetry / unsubscribe-telemetry require the JSON-RPC per-connection \
             handler; the legacy NDJSON dispatcher cannot serve them"
                .to_owned(),
        ),
    }
}

/// MLP2-026: derive an `OperatorContext` from the daemon's view of
/// the IPC peer for an `UnblockCascade` audit trail. Spec §3.3.
/// `peer_pid` is already captured by the MLP2-025b plumbing; we
/// reuse it here. `uid` and `hostname` are best-effort: failures
/// produce `None` fields rather than failing the clear (spec §7
/// — credential gaps record the gap; the clear-side authority is
/// the existing UID gate at socket-accept).
fn build_operator_context(
    peer_pid: Option<u32>,
) -> anvil_intercept_proto::session::OperatorContext {
    // The existing socket-accept gate already enforces same-UID;
    // the daemon's own UID is therefore the operator UID. Read it
    // here rather than re-querying SO_PEERCRED to keep the audit
    // record honest about which UID actually acted.
    #[cfg(unix)]
    let uid = Some(nix::unistd::Uid::current().as_raw());
    #[cfg(not(unix))]
    let uid: Option<u32> = None;

    #[cfg(unix)]
    let hostname = nix::unistd::gethostname()
        .ok()
        .and_then(|os| os.into_string().ok());
    #[cfg(not(unix))]
    let hostname: Option<String> = None;

    anvil_intercept_proto::session::OperatorContext {
        uid,
        pid: peer_pid,
        hostname,
    }
}

/// MLP2-071 D2: mint the daemon-side [`crate::fanout::SubscriberId`] for
/// an IPC peer from its authenticated credentials. NEVER from a
/// wire-supplied field — `SO_PEERCRED` reports the real connecting pid,
/// and the `pid_starttime` component defends against PID reuse, so a
/// hostile same-UID peer cannot forge another peer's id.
///
/// Components (Unix): the daemon's own `uid` + the peer `pid` + the
/// peer's `pid_starttime` from `/proc/<pid>/stat`.
///
/// **Precondition:** the `uid` here is the *daemon's* uid, used as a
/// stand-in for the peer's uid because the socket-accept gate
/// ([`validate_connected_peer_for_client`]) already rejects any peer
/// whose uid differs from the daemon's — so under that gate they are
/// equal. If that same-UID gate is ever relaxed (cross-uid telemetry,
/// a root daemon serving non-root peers), this MUST switch to the
/// peer's uid read from `SO_PEERCRED` (`peer_cred().uid()`), or two
/// distinct peers could mint the same id. The binary-path-hash
/// component D2 also describes is deferred: for the same-UID trust
/// domain, `(uid, pid, pid_starttime)` already uniquely and
/// unforgeably identifies a live process incarnation; the binary hash
/// is defense-in-depth tracked as an MLP2-071 follow-up.
///
/// Returns `None` when the peer pid is unavailable (legacy NDJSON, no
/// `SO_PEERCRED`) or its start-time cannot be read (peer exited
/// mid-handshake, or a non-Linux platform where `pid_starttime` is not
/// yet supported). A `None` mint means no binding / no subscription,
/// which the resolver default-denies — fail-closed (D2 degraded note).
#[cfg(unix)]
fn mint_subscriber_id(peer_pid: Option<u32>) -> Option<crate::fanout::SubscriberId> {
    let pid = peer_pid?;
    let uid = nix::unistd::Uid::current().as_raw();
    let starttime = anvil_attribution::process::pid_starttime(pid).ok()?;
    Some(crate::fanout::SubscriberId::new(format!(
        "peer:uid={uid}:pid={pid}:start={starttime}"
    )))
}

#[cfg(not(unix))]
fn mint_subscriber_id(_peer_pid: Option<u32>) -> Option<crate::fanout::SubscriberId> {
    // Windows subscriber minting (GetNamedPipeClientProcessId +
    // process start-time) is the MLP2-028 follow-up; until it lands the
    // non-Unix subscribe path mints no id and the resolver
    // default-denies (D2 "degraded SubscriberId" note).
    None
}

// --------------------------------------------------------------------
// Tests.
// --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enforcement::EnforcementPipeline;
    use anvil_intercept_proto::session::AgentTag;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    /// CLAWP-065 review regression: the oversized fast-path validator
    /// must accept `null` for the optional `env_agent_tag` / `session_id`
    /// fields, matching the "string or null" contract the normal parse
    /// path (`scan_buffer_from_jsonrpc`) enforces. Before the
    /// `skip_bounded_json_string_or_null` fix, a `null` here was rejected
    /// as "missing or too large", so an oversized frame diverged from a
    /// normal-sized one for the same payload.
    /// USAGE-004: an allowlisted single-object request emits exactly one
    /// `command.invoked` row carrying the method, envelope principal, and
    /// echoed traceparent.
    #[test]
    fn emit_command_invocations_emits_for_allowlisted_object() {
        use crate::kindling_observation::CommandInvokedEmitter;

        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-x");
        let value = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "anvil/gctx/search_symbols",
            "principal": "deadbeef",
            "traceparent": "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            "params": {"query": "Foo"}
        });
        emit_command_invocations(&value, &emitter, "2026-06-18T10:00:00Z");

        let rows = recorder.recorded_command_invocations();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].command, "anvil/gctx/search_symbols");
        assert_eq!(rows[0].principal, "deadbeef");
        assert_eq!(
            rows[0].traceparent.as_deref(),
            Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01")
        );
    }

    /// USAGE-004: excluded methods, non-2.0 frames, and non-request
    /// shapes produce no usage row.
    #[test]
    fn emit_command_invocations_skips_non_user_initiated() {
        use crate::kindling_observation::CommandInvokedEmitter;

        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-x");
        // Excluded internal method.
        emit_command_invocations(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "scan_buffer"}),
            &emitter,
            "t",
        );
        // Missing jsonrpc version.
        emit_command_invocations(
            &json!({"id": 1, "method": "anvil/gctx/search_symbols"}),
            &emitter,
            "t",
        );
        // Not an object.
        emit_command_invocations(&json!("scalar"), &emitter, "t");
        assert!(recorder.recorded_command_invocations().is_empty());
    }

    /// USAGE-004 conformance (R2): drive *every* method in the live
    /// `COMMAND_INVOKED_ALLOWLIST` through the real emit path and assert
    /// each produces exactly one `command.invoked` row whose `command`
    /// is the method. Because emission is a single allowlist-gated
    /// chokepoint (not per-method handlers), adding a method to the
    /// allowlist is sufficient for it to emit — and this fixture proves
    /// it, so the allowlist can never carry a method that silently fails
    /// to record. Pairs with
    /// `command_invoked_allowlist_classifies_every_namespaced_method`
    /// (which forces a new protocol method to be classified) and
    /// `emit_command_invocations_skips_non_user_initiated` (excluded → 0).
    #[test]
    fn every_allowlisted_method_emits_exactly_one_row() {
        use crate::kindling_observation::CommandInvokedEmitter;

        for method in COMMAND_INVOKED_ALLOWLIST {
            let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-conformance");
            let frame = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "principal": "deadbeef",
                "params": {}
            });
            emit_command_invocations(&frame, &emitter, "2026-06-18T12:00:00Z");

            let rows = recorder.recorded_command_invocations();
            assert_eq!(
                rows.len(),
                1,
                "allowlisted method {method} must emit exactly one command.invoked row"
            );
            assert_eq!(rows[0].command, *method, "row command must be the method");
        }
    }

    /// USAGE-004: a batch array emits one row per allowlisted item and
    /// skips the rest.
    #[test]
    fn emit_command_invocations_handles_batch() {
        use crate::kindling_observation::CommandInvokedEmitter;

        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-x");
        let batch = json!([
            {"jsonrpc": "2.0", "id": 1, "method": "anvil/gctx/find_callers"},
            {"jsonrpc": "2.0", "id": 2, "method": "scan_buffer"},
            {"jsonrpc": "2.0", "id": 3, "method": "unblock-cascade", "principal": "abc"}
        ]);
        emit_command_invocations(&batch, &emitter, "t");

        let rows = recorder.recorded_command_invocations();
        let commands: Vec<&str> = rows.iter().map(|r| r.command.as_str()).collect();
        assert_eq!(commands, ["anvil/gctx/find_callers", "unblock-cascade"]);
    }

    /// USAGE-004 (Council: lenient-emit vs strict-dispatch): a malformed
    /// envelope `principal` (or `traceparent`) is a hard rejection in
    /// `handle_jsonrpc_request`, so emission must suppress the row too —
    /// no phantom invocation for a frame the dispatcher then rejects. An
    /// *absent* principal is fine and resolves to `anonymous`.
    #[test]
    fn emit_command_invocations_skips_frames_the_dispatcher_rejects() {
        use crate::kindling_observation::CommandInvokedEmitter;

        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-x");
        // Non-string principal → dispatcher rejects → no row.
        emit_command_invocations(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "anvil/gctx/find_dependents", "principal": 42}),
            &emitter,
            "t",
        );
        // Over-cap principal → dispatcher rejects → no row.
        emit_command_invocations(
            &json!({"jsonrpc": "2.0", "id": 2, "method": "anvil/gctx/find_dependents", "principal": "x".repeat(MAX_PRINCIPAL_BYTES + 1)}),
            &emitter,
            "t",
        );
        assert!(recorder.recorded_command_invocations().is_empty());

        // Absent principal is fine → one row, anonymous.
        emit_command_invocations(
            &json!({"jsonrpc": "2.0", "id": 3, "method": "anvil/gctx/find_dependents"}),
            &emitter,
            "t",
        );
        let rows = recorder.recorded_command_invocations();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].principal, "anonymous");
    }

    /// USAGE-004 (Council: batch write-amplification): a batch the
    /// dispatcher rejects wholesale — empty, or larger than
    /// `MAX_JSONRPC_BATCH_ITEMS` — records nothing, so an oversized
    /// control frame cannot drive more sink writes than it can dispatch.
    #[test]
    fn emit_command_invocations_skips_rejected_batches() {
        use crate::kindling_observation::CommandInvokedEmitter;

        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-x");
        // Empty batch → rejected → no rows.
        emit_command_invocations(&json!([]), &emitter, "t");
        // Over-limit batch of allowlisted methods → rejected → no rows.
        let oversized: Vec<_> = (0..=MAX_JSONRPC_BATCH_ITEMS)
            .map(|i| json!({"jsonrpc": "2.0", "id": i, "method": "anvil/gctx/search_symbols"}))
            .collect();
        emit_command_invocations(&Value::Array(oversized), &emitter, "t");
        assert!(recorder.recorded_command_invocations().is_empty());

        // A batch at the limit still emits for its allowlisted items.
        let at_limit: Vec<_> = (0..MAX_JSONRPC_BATCH_ITEMS)
            .map(|i| json!({"jsonrpc": "2.0", "id": i, "method": "anvil/gctx/search_symbols"}))
            .collect();
        emit_command_invocations(&Value::Array(at_limit), &emitter, "t");
        assert_eq!(
            recorder.recorded_command_invocations().len(),
            MAX_JSONRPC_BATCH_ITEMS
        );
    }

    /// USAGE-004 (#2752): the *live listener* end-to-end. Every other
    /// USAGE-004 test drives the `emit_command_invocations` helper directly,
    /// so none of them cover the one-line call site inside
    /// `handle_connection` — reached only via `IpcListener::with_usage_emitter`
    /// — where emission is actually wired. A regression that unwired the
    /// emitter, or moved the emit after `value` is consumed by dispatch, would
    /// pass every direct-helper test yet silently stop recording usage. This
    /// runs a real `handle_connection` over a `tokio::io::duplex()` pair with a
    /// recording emitter and asserts the wiring: one allowlisted frame
    /// (`anvil/gctx/search_symbols`) records exactly one `command.invoked` row;
    /// one excluded frame (`scan_buffer`) records none. Emission is pre-dispatch
    /// and allowlist-gated, so the row lands regardless of the dispatch outcome
    /// (here `NoopDispatcher` + no save-time state replies method-not-found).
    #[cfg(any(unix, windows))]
    #[tokio::test]
    async fn handle_connection_records_usage_for_allowlisted_frame_only() {
        use crate::Shutdown;
        use crate::kindling_observation::CommandInvokedEmitter;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (emitter, recorder) = CommandInvokedEmitter::with_recorder("daemon-live");
        let (_shutdown, token) = Shutdown::new();
        let (mut client, server) = tokio::io::duplex(64 * 1024);

        // One allowlisted frame (records) + one excluded frame (does not),
        // then EOF so the handler's read loop returns cleanly.
        let mut frames = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "anvil/gctx/search_symbols",
            "principal": "deadbeef",
            "traceparent": "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            "params": {"query": "Foo"}
        })
        .to_string();
        frames.push('\n');
        frames.push_str(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "scan_buffer",
                "params": {}
            })
            .to_string(),
        );
        frames.push('\n');
        client
            .write_all(frames.as_bytes())
            .await
            .expect("write frames");
        client.shutdown().await.expect("shutdown write half");

        // Drive the real per-connection handler with the emitter wired exactly
        // as `IpcListener::with_usage_emitter` + the serve loop would wire it.
        let handler = tokio::spawn(handle_connection(
            server,
            Arc::new(NoopDispatcher),
            ScanBufferService::default(),
            Arc::new(NoopStatusProvider) as Arc<dyn StatusProvider>,
            token,
            crate::dos::IpcLimits::default(),
            None,                    // peer_pid
            None,                    // cross_check
            None,                    // save_time
            None,                    // broadcaster
            Some(Arc::new(emitter)), // usage_emitter
        ));

        // Drain the handler's responses so its writes never back-pressure; the
        // read completes when the handler returns and drops its stream half.
        let mut discard = Vec::new();
        let _ = client.read_to_end(&mut discard).await;
        handler
            .await
            .expect("handler task joins")
            .expect("handle_connection returns Ok");

        let rows = recorder.recorded_command_invocations();
        assert_eq!(
            rows.len(),
            1,
            "exactly one command.invoked row: the allowlisted frame emits, scan_buffer is excluded",
        );
        assert_eq!(rows[0].command, "anvil/gctx/search_symbols");
        assert_eq!(rows[0].principal, "deadbeef");
        assert_eq!(
            rows[0].traceparent.as_deref(),
            Some("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );
    }

    /// USAGE-004 (R2 mitigation, daemon side): every namespaced method
    /// the protocol defines must be classified as *exactly one* of
    /// user-initiated (allowlisted → emits a usage row) or internal
    /// machinery (explicitly excluded). The count pin forces a new
    /// `anvil/*` method to be triaged here rather than silently
    /// defaulting to "no usage row" (or, worse, flooding usage with
    /// machine traffic).
    #[test]
    fn command_invoked_allowlist_classifies_every_namespaced_method() {
        use anvil_intercept_proto::protocol::*;
        // Deliberately NOT user-initiated: scan/save/status machinery,
        // server→client messages, gate/suppression protocol verbs.
        const EXCLUDED: &[&str] = &[
            ANVIL_PUBLISH_DIAGNOSTICS,
            ANVIL_SCAN_BUFFER,
            ANVIL_ENFORCEMENT_ACK,
            ANVIL_GATE_REQUEST,
            ANVIL_SUPPRESSION_APPLY,
            ANVIL_STATUS_QUERY,
            ANVIL_VALIDATE_PATHS,
            ANVIL_WORKSPACE_STATUS,
            ANVIL_REQUEST_FULL_SCAN,
            // Commit-time machinery (hook → daemon), not a user-typed command.
            ANVIL_WITNESS_APPEND,
        ];
        for method in ALL_ANVIL_METHODS {
            let allowed = is_command_invoked_method(method);
            let excluded = EXCLUDED.contains(method);
            assert!(
                allowed ^ excluded,
                "method {method} must be classified as exactly one of \
                 allowlisted/excluded — a new protocol method needs a \
                 deliberate USAGE-004 decision"
            );
        }
        // Count pin: 9 GCTX query methods are allowlisted (search/dependents/
        // callers/impact/affected-tests + GCTX-030 graph_stats/graph_edges +
        // GCTX-021 get_snippet + GCTX-023 symbol_context); the rest are
        // excluded. Moving either set must move this.
        assert_eq!(
            ALL_ANVIL_METHODS.len(),
            EXCLUDED.len() + 9,
            "ALL_ANVIL_METHODS changed — reclassify the new method for USAGE-004"
        );
    }

    /// USAGE-004: the operator `unblock-*` verbs (bare-name, not in
    /// `ALL_ANVIL_METHODS`) are user-initiated under both wire spellings;
    /// internal machinery is never allowlisted regardless of spelling.
    #[test]
    fn command_invoked_allowlist_covers_unblock_and_excludes_machinery() {
        for verb in [
            "unblock-cascade",
            "fence.unblock-cascade",
            "unblock-worktree",
            "fence.unblock-worktree",
        ] {
            assert!(
                is_command_invoked_method(verb),
                "{verb} must be allowlisted"
            );
        }
        for machinery in [
            "scan_buffer",
            anvil_intercept_proto::protocol::ANVIL_SCAN_BUFFER,
            anvil_intercept_proto::protocol::ANVIL_VALIDATE_PATHS,
            "query_status",
            "session.register",
            "report-process",
        ] {
            assert!(
                !is_command_invoked_method(machinery),
                "{machinery} must NOT be allowlisted"
            );
        }
    }

    /// USAGE-004: the principal cap stays in the established
    /// identifier-cap family so the envelope field cannot be used to
    /// smuggle an unbounded string past the dispatcher.
    #[test]
    fn principal_cap_matches_identifier_family() {
        assert_eq!(MAX_PRINCIPAL_BYTES, 256);
        assert_eq!(MAX_PRINCIPAL_BYTES, MAX_SCAN_BUFFER_SESSION_ID_BYTES);
    }

    /// USAGE-004: a present, in-bounds principal string is returned
    /// verbatim (the 64-hex salted hash the client attached).
    #[test]
    fn extract_principal_returns_present_string() {
        let hash = "a".repeat(64);
        let map =
            serde_json::Map::from_iter([("principal".to_owned(), Value::String(hash.clone()))]);
        assert!(matches!(extract_principal(&map), Ok(Some(p)) if p == hash));
    }

    /// USAGE-004: an absent or `null` principal both resolve to `None`
    /// (the producer maps that to `"anonymous"`, parity with the
    /// unauthenticated CLI path) — existing clients stay wire-compatible.
    #[test]
    fn extract_principal_absent_and_null_are_none() {
        let absent = serde_json::Map::new();
        assert!(matches!(extract_principal(&absent), Ok(None)));

        let null = serde_json::Map::from_iter([("principal".to_owned(), Value::Null)]);
        assert!(matches!(extract_principal(&null), Ok(None)));
    }

    /// USAGE-004: a non-string principal is a hard `Invalid Request`.
    #[test]
    fn extract_principal_rejects_non_string() {
        let map = serde_json::Map::from_iter([("principal".to_owned(), json!(42))]);
        assert!(extract_principal(&map).is_err());
    }

    /// USAGE-004: an over-cap principal is rejected so the field cannot
    /// carry an unbounded payload.
    #[test]
    fn extract_principal_rejects_over_cap() {
        let big = "x".repeat(MAX_PRINCIPAL_BYTES + 1);
        let map = serde_json::Map::from_iter([("principal".to_owned(), Value::String(big))]);
        assert!(extract_principal(&map).is_err());
    }

    #[test]
    fn oversized_params_validator_accepts_null_optional_fields() {
        let params =
            br#"{"path":"x.ts","text":"y","version":1,"mode":"midEdit","env_agent_tag":null,"session_id":null}"#;
        let mut index = 0;
        assert!(
            validate_oversized_scan_buffer_params(params, &mut index).is_ok(),
            "null optional fields must pass the oversized fast-path validator",
        );
    }

    /// CLAWP-065: the oversized fast-path still bounds a string
    /// `session_id` at its cap — `null` acceptance must not weaken the
    /// size guard.
    #[test]
    fn oversized_params_validator_rejects_over_cap_session_id() {
        let big = "x".repeat(MAX_SCAN_BUFFER_SESSION_ID_BYTES + 1);
        let raw = format!(
            r#"{{"path":"x.ts","text":"y","version":1,"mode":"midEdit","session_id":"{big}"}}"#
        );
        let mut index = 0;
        assert!(
            validate_oversized_scan_buffer_params(raw.as_bytes(), &mut index).is_err(),
            "an over-cap session_id must still be rejected by the fast-path",
        );
    }

    // ----------------------------------------------------------------
    // MLP2-025b: cross-check wire-up tests. Target `run_spoof_cross_check`
    // directly with synthetic CrossCheckContext fixtures so the tests
    // stay platform-portable (no real IPC, no real /proc walking).
    // ----------------------------------------------------------------

    fn make_cross_check_context() -> (CrossCheckContext, tempfile::TempDir) {
        use crate::fence::FenceStore;
        use crate::registry::SessionRegistry;
        let temp = tempfile::tempdir().expect("tempdir");
        let store = FenceStore::at_path(temp.path().join("state/fences.json"));
        (
            CrossCheckContext {
                registry: Arc::new(SessionRegistry::new()),
                fence_store: Arc::new(store),
            },
            temp,
        )
    }

    fn make_request(text: &str, env_agent_tag: Option<&str>) -> ScanBufferRequest {
        ScanBufferRequest {
            path: PathBuf::from("/tmp/spoof-target/x.rs"),
            text: text.to_string(),
            version: 1,
            mode: ScanBufferMode::MidEdit,
            env_agent_tag: env_agent_tag.map(ToString::to_string),
            session_id: None,
        }
    }

    /// MLP2-025b: a write with no `env_agent_tag` always falls
    /// through to the rule engine — `Cross::Untagged` is the
    /// pre-MLP2-025 path, unchanged.
    #[test]
    fn run_spoof_cross_check_untagged_returns_none() {
        let (ctx, _temp) = make_cross_check_context();
        let request = make_request("fn main() {}", None);
        let response = run_spoof_cross_check(&request, Some(4242), &ctx);
        assert!(response.is_none(), "untagged write must fall through");
    }

    /// MLP2-025b: a write with an `env_agent_tag` but no peer PID
    /// is classified as `Cross::Spoofed` (fail-closed per spec §7).
    /// The daemon cannot validate the lineage without a writer PID.
    /// Pin both halves of the verdict: the response carries the
    /// spoof block AND the fence is recorded on disk.
    #[test]
    fn run_spoof_cross_check_present_env_tag_without_peer_pid_is_spoofed() {
        let (ctx, _temp) = make_cross_check_context();
        // Real tempdir for the file's parent so the fence-store
        // canonicalise succeeds. The file itself need not exist —
        // the fence target is the parent directory.
        let writer_wt = tempfile::tempdir().expect("writer tempdir");
        let tag = AgentTag::new("anvil-run", "claude-1", 1_700_000_000);
        let encoded = anvil_attribution::env::agent_tag_to_env_value(&tag);
        let mut request = make_request("fn main() {}", Some(&encoded));
        request.path = writer_wt.path().join("x.rs");

        let response =
            run_spoof_cross_check(&request, None, &ctx).expect("None peer pid must fail-closed");

        // Confirm the wire shape carries the spoof block.
        assert_eq!(
            response["spoof_block"]["reason"],
            "degraded:spoofed-attribution"
        );
        // Confirm the fence_store recorded the worktree.
        let fences = ctx.fence_store.load().expect("load fences");
        assert!(
            !fences.active_fences().is_empty(),
            "spoof block must record a fence; active={:?}",
            fences.active_fences()
        );
        assert_eq!(
            fences.active_fences()[0].reason,
            "degraded:spoofed-attribution"
        );
    }

    /// MLP2-025b: a write with a malformed `env_agent_tag` is
    /// classified as `Cross::Spoofed` rather than surfacing as a
    /// parse error. Pinned by spec §7 + Q3 verdict.
    #[test]
    fn run_spoof_cross_check_malformed_env_tag_is_spoofed() {
        let (ctx, _temp) = make_cross_check_context();
        let request = make_request("fn main() {}", Some("not-valid-json"));

        let response = run_spoof_cross_check(&request, Some(4242), &ctx)
            .expect("malformed env tag must be spoofed");

        assert_eq!(
            response["spoof_block"]["reason"],
            "degraded:spoofed-attribution"
        );
    }

    /// MLP2-025b: a write with an `env_agent_tag` that matches the
    /// daemon-issued tag on the writer's PID lineage falls through
    /// to the rule engine — `Cross::Match` is the legitimate path.
    /// Exercised via the test process's own PID, which the
    /// `worktree_for_lineage` walk can actually traverse via /proc.
    #[cfg(target_os = "linux")]
    #[test]
    fn run_spoof_cross_check_matching_tag_falls_through() {
        use std::time::Instant;
        let (ctx, _temp) = make_cross_check_context();
        let self_pid = std::process::id();
        // Read the live pid_starttime via the same helper the
        // production lookup uses.
        let starttime =
            anvil_attribution::process::pid_starttime(self_pid).expect("pid_starttime for self");

        let issued = AgentTag::new("anvil-run", "claude-test", starttime);
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        ctx.registry
            .register_with_lineage(
                &anvil_intercept_proto::SessionId::new("test-session"),
                worktree.path(),
                None,
                Some(&issued),
                self_pid,
                starttime,
                Instant::now(),
            )
            .expect("register with lineage");

        let encoded = anvil_attribution::env::agent_tag_to_env_value(&issued);
        let request = make_request("fn main() {}", Some(&encoded));

        let response = run_spoof_cross_check(&request, Some(self_pid), &ctx);
        assert!(
            response.is_none(),
            "matching tag must fall through to rule engine; got {response:?}"
        );

        // No fence should have been recorded.
        let fences = ctx.fence_store.load().expect("load fences");
        assert!(fences.active_fences().is_empty());
    }

    /// MLP2-025b: a write with a present `env_agent_tag` but no
    /// registered ancestor on the writer's lineage is `Spoofed`.
    /// Pinned with a synthetic PID that has no registered ancestor
    /// — the lineage walk reaches root without matching.
    #[cfg(target_os = "linux")]
    #[test]
    fn run_spoof_cross_check_no_registered_ancestor_is_spoofed() {
        let (ctx, _temp) = make_cross_check_context();
        let tag = AgentTag::new("anvil-run", "ghost", 1_700_000_000);
        let encoded = anvil_attribution::env::agent_tag_to_env_value(&tag);
        let request = make_request("fn main() {}", Some(&encoded));

        let response = run_spoof_cross_check(&request, Some(std::process::id()), &ctx)
            .expect("no registered ancestor must be spoofed");
        assert_eq!(
            response["spoof_block"]["reason"],
            "degraded:spoofed-attribution"
        );
    }

    // ---- MLP2-026: UnblockCascade dispatch ---------------------------

    /// MLP2-026: `dispatch_command` routes `UnblockCascade` to
    /// `FenceStore::clear_cascade` and returns `{"ok": true}` when
    /// a cascade was cleared. Spec §3.4 + §5.4.
    #[test]
    fn dispatch_command_unblock_cascade_clears_engaged_cascade() {
        use anvil_intercept_proto::IpcCommand;
        let (ctx, _temp) = make_cross_check_context();
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        // Engage the cascade by firing 5 fences.
        for i in 0..5 {
            ctx.fence_store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence");
        }
        assert!(ctx.fence_store.is_cascaded(worktree.path()));

        // Dispatch the unblock-cascade command.
        let dispatcher = Arc::new(NoopDispatcher);
        let command = IpcCommand::UnblockCascade {
            worktree: worktree.path().to_path_buf(),
            operator: None,
        };
        let result =
            dispatch_command(&command, &dispatcher, Some(4242), Some(&ctx)).expect("dispatch ok");
        assert_eq!(result["ok"], true);

        // Confirm the cascade is cleared.
        assert!(!ctx.fence_store.is_cascaded(worktree.path()));
    }

    /// MLP2-026: `dispatch_command` returns `{"ok": false}` when
    /// the worktree was not in cascade — idempotent operator-clear
    /// (spec §5.3 + §6 inv-3).
    #[test]
    fn dispatch_command_unblock_cascade_is_idempotent() {
        use anvil_intercept_proto::IpcCommand;
        let (ctx, _temp) = make_cross_check_context();
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        let dispatcher = Arc::new(NoopDispatcher);
        let command = IpcCommand::UnblockCascade {
            worktree: worktree.path().to_path_buf(),
            operator: None,
        };
        let result =
            dispatch_command(&command, &dispatcher, None, Some(&ctx)).expect("dispatch ok");
        assert_eq!(result["ok"], false, "no-op clear returns false");
    }

    /// MLP2-026: when no `CrossCheckContext` is wired,
    /// `UnblockCascade` returns a typed error rather than panicking
    /// or silently succeeding. Tests / embedded callers that don't
    /// expose a `fence_store` see this path.
    #[test]
    fn dispatch_command_unblock_cascade_requires_cross_check_context() {
        use anvil_intercept_proto::IpcCommand;
        let dispatcher = Arc::new(NoopDispatcher);
        let command = IpcCommand::UnblockCascade {
            worktree: PathBuf::from("/tmp/wt"),
            operator: None,
        };
        let err =
            dispatch_command(&command, &dispatcher, None, None).expect_err("no ctx must error");
        assert!(err.contains("daemon-backed fence store"), "got: {err}");
    }

    /// MLP2-026: client-supplied `operator` field is silently
    /// overwritten — the daemon ignores it and derives its own
    /// from peer credentials. Pin the "client-supplied → ignored"
    /// contract (spec §3.3 + §3.4 + Q2 verdict).
    #[test]
    fn dispatch_command_unblock_cascade_ignores_client_operator() {
        use anvil_intercept_proto::IpcCommand;
        let (ctx, _temp) = make_cross_check_context();
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        for i in 0..5 {
            ctx.fence_store
                .fence_worktree(worktree.path(), format!("fire {i}"))
                .expect("fence");
        }

        let dispatcher = Arc::new(NoopDispatcher);
        let attacker_supplied = anvil_intercept_proto::session::OperatorContext {
            uid: Some(0),
            pid: Some(1),
            hostname: Some("root-elsewhere".to_string()),
        };
        let command = IpcCommand::UnblockCascade {
            worktree: worktree.path().to_path_buf(),
            operator: Some(attacker_supplied),
        };
        // The dispatch path doesn't return the operator on the wire,
        // but the cascade was cleared — confirming the path executed
        // and the daemon-derived OperatorContext is what landed in
        // the audit (tracing event). Verifying the tracing output
        // is out of scope here; the silent-overwrite contract is
        // pinned by code review of dispatch_command + by this test
        // confirming the clear succeeded despite a hostile-looking
        // client-supplied context.
        let result =
            dispatch_command(&command, &dispatcher, Some(9999), Some(&ctx)).expect("dispatch ok");
        assert_eq!(result["ok"], true);
        assert!(!ctx.fence_store.is_cascaded(worktree.path()));
    }

    // ---- RCLI3-017b: UnblockWorktree dispatch ------------------------

    /// RCLI3-017b: `dispatch_command` routes `UnblockWorktree` to
    /// `FenceStore::unblock_worktree` and returns `{"ok": true}` when
    /// a fence record was actually removed. Mirrors the cascade
    /// dispatch test for shape parity.
    #[test]
    fn dispatch_command_unblock_worktree_clears_engaged_fence() {
        use anvil_intercept_proto::IpcCommand;
        let (ctx, _temp) = make_cross_check_context();
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        ctx.fence_store
            .fence_worktree(worktree.path(), "test fence")
            .expect("fence");

        let dispatcher = Arc::new(NoopDispatcher);
        let command = IpcCommand::UnblockWorktree {
            worktree: worktree.path().to_path_buf(),
        };
        let result =
            dispatch_command(&command, &dispatcher, Some(4242), Some(&ctx)).expect("dispatch ok");
        assert_eq!(result["ok"], true, "cleared fence must report true");

        // Re-running on the now-unfenced worktree is a no-op.
        let again =
            dispatch_command(&command, &dispatcher, Some(4242), Some(&ctx)).expect("dispatch ok");
        assert_eq!(
            again["ok"], false,
            "idempotent re-run must report false (no-op)",
        );
    }

    /// RCLI3-017b: `dispatch_command` returns `{"ok": false}` when
    /// the worktree was not fenced — idempotent operator-clear so
    /// scripts can call `unblock-worktree` unconditionally during
    /// demo / test setup without surfacing spurious failures.
    #[test]
    fn dispatch_command_unblock_worktree_is_idempotent() {
        use anvil_intercept_proto::IpcCommand;
        let (ctx, _temp) = make_cross_check_context();
        let worktree = tempfile::tempdir().expect("worktree tempdir");

        let dispatcher = Arc::new(NoopDispatcher);
        let command = IpcCommand::UnblockWorktree {
            worktree: worktree.path().to_path_buf(),
        };
        let result =
            dispatch_command(&command, &dispatcher, None, Some(&ctx)).expect("dispatch ok");
        assert_eq!(result["ok"], false, "no-op clear returns false");
    }

    /// RCLI3-017b: without a `CrossCheckContext` the dispatcher
    /// surfaces a typed error rather than panicking. Mirrors the
    /// cascade path's `dispatch_command_unblock_cascade_requires_cross_check_context`.
    #[test]
    fn dispatch_command_unblock_worktree_requires_cross_check_context() {
        use anvil_intercept_proto::IpcCommand;
        let dispatcher = Arc::new(NoopDispatcher);
        let command = IpcCommand::UnblockWorktree {
            worktree: PathBuf::from("/tmp/wt"),
        };
        let err =
            dispatch_command(&command, &dispatcher, None, None).expect_err("no ctx must error");
        assert!(err.contains("daemon-backed fence store"), "got: {err}");
    }

    // ---- MLP2-070 / #1674: lineage anchor daemon-derivation gate ----

    /// MLP2-070 / #1674: a `register-session` frame whose body lineage
    /// claims a `pid` other than the authenticated peer pid must be
    /// rejected. Without this gate a same-UID IPC caller could mint a
    /// trusted lineage entry for someone else's PID, defeating the
    /// MLP2-025 spoof cross-check. The pid trust gate is the primary
    /// forgery defence and applies on every platform — only the
    /// daemon-side `pid_starttime` re-read is Linux-only.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_register_lineage_rejects_pid_mismatch() {
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::LineageAnchor;

        let recorder = Arc::new(RecordingDispatcher::default());
        // `impl SessionDispatcher for Arc<RecordingDispatcher>`, so
        // dispatch_command's `&Arc<D>` needs an outer Arc layer.
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let peer_pid = std::process::id();
        // A PID that is not the peer's. wrapping_add ensures we stay
        // inside u32 even at the boundary.
        let victim_pid = peer_pid.wrapping_add(1);
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("attacker-session"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: None,
            lineage: Some(LineageAnchor {
                pid: victim_pid,
                pid_starttime: 1_700_000_000,
            }),
        };
        let err = dispatch_command(&command, &dispatcher, Some(peer_pid), None)
            .expect_err("lineage with mismatched pid must be rejected");
        assert!(
            err.contains("does not match authenticated peer"),
            "error must name the mismatch, got: {err}",
        );
        // Critically, the underlying dispatcher must never see the
        // forged anchor — rejection happens before `register` is
        // called, so no registry state is mutated.
        assert!(
            recorder.calls().is_empty(),
            "dispatcher.register must not be called on lineage mismatch; calls={:?}",
            recorder.calls(),
        );
    }

    /// MLP2-070 / #1674: a `register-session` frame carrying a lineage
    /// anchor over a connection that has no authenticated peer pid
    /// (e.g. the legacy NDJSON wire on line 2910, or a platform where
    /// `SO_PEERCRED` is unavailable) must be rejected — we have no way
    /// to verify the claim, so fail-closed. Pure pid-comparison logic;
    /// platform-agnostic.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_register_lineage_requires_peer_credentials() {
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::LineageAnchor;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("legacy-ndjson"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: None,
            lineage: Some(LineageAnchor {
                pid: 4242,
                pid_starttime: 1_700_000_000,
            }),
        };
        let err = dispatch_command(&command, &dispatcher, None, None)
            .expect_err("lineage without peer_pid must be rejected");
        assert!(
            err.contains("peer credentials"),
            "error must name the missing credential, got: {err}",
        );
        assert!(
            recorder.calls().is_empty(),
            "dispatcher.register must not be called when peer_pid is absent; calls={:?}",
            recorder.calls(),
        );
    }

    /// CIB-153: build a registry-backed dispatcher with one session
    /// stamped to `launcher_pid`, ready for the lifecycle-ownership
    /// dispatch tests. Uses a real [`SessionRegistry`] because the
    /// ownership check lives in its `SessionDispatcher` impl (the
    /// recorder carries no launcher anchor). `register_with_lineage`
    /// stores the given pid/starttime verbatim, so the tests stay
    /// platform-portable (no `/proc` read).
    #[cfg(unix)]
    fn registry_with_owned_session(
        session: &str,
        launcher_pid: u32,
    ) -> (Arc<SessionRegistry>, tempfile::TempDir) {
        use std::time::Instant;
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let registry = Arc::new(SessionRegistry::new());
        let issued = AgentTag::new("anvil-run", "launcher", 1_700_003_000);
        registry
            .register_with_lineage(
                &SessionId::new(session),
                worktree.path(),
                None,
                Some(&issued),
                launcher_pid,
                1_700_003_000,
                Instant::now(),
            )
            .expect("register with lineage");
        (registry, worktree)
    }

    /// CIB-153: a session registered under peer A's authenticated pid
    /// rejects a `Heartbeat` from a different injected peer B at the
    /// dispatch boundary, still accepts it from peer A, and fails
    /// closed when no peer credential is present (`peer_pid = None`).
    /// Mirrors the `dispatch_command_register_lineage_*` injected-pid
    /// pattern but against a real registry so the ownership check runs.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_heartbeat_binds_to_registering_peer() {
        use anvil_intercept_proto::IpcCommand;

        let peer_a = std::process::id();
        let peer_b = peer_a.wrapping_add(1);
        let (registry, _worktree) = registry_with_owned_session("hb-session", peer_a);
        let command = IpcCommand::Heartbeat {
            session_id: SessionId::new("hb-session"),
        };

        // Peer B (different injected pid) is rejected.
        let err = dispatch_command(&command, &registry, Some(peer_b), None)
            .expect_err("heartbeat from a non-owning peer must be rejected");
        assert!(
            err.contains("peer-ownership check failed"),
            "error must name the ownership failure, got: {err}",
        );

        // No peer credential fails closed.
        let err_none = dispatch_command(&command, &registry, None, None)
            .expect_err("heartbeat without peer credentials must fail closed");
        assert!(
            err_none.contains("peer-ownership check failed"),
            "error must name the ownership failure, got: {err_none}",
        );

        // Peer A (the registering peer) is accepted.
        dispatch_command(&command, &registry, Some(peer_a), None)
            .expect("heartbeat from the registering peer must be accepted");
    }

    /// CIB-153: the same registering-peer binding governs
    /// `UnregisterSession` — a non-owning injected peer B (and the
    /// no-credential path) cannot force-unregister the session, and it
    /// survives the rejected calls; the owning peer A removes it.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_unregister_binds_to_registering_peer() {
        use anvil_intercept_proto::IpcCommand;

        let peer_a = std::process::id();
        let peer_b = peer_a.wrapping_add(1);
        let (registry, _worktree) = registry_with_owned_session("unreg-session", peer_a);
        let command = IpcCommand::UnregisterSession {
            session_id: SessionId::new("unreg-session"),
        };

        // Peer B is rejected and the session survives.
        let err = dispatch_command(&command, &registry, Some(peer_b), None)
            .expect_err("unregister from a non-owning peer must be rejected");
        assert!(
            err.contains("peer-ownership check failed"),
            "error must name the ownership failure, got: {err}",
        );
        assert_eq!(
            registry.active_sessions().len(),
            1,
            "a rejected unregister must not remove the session",
        );

        // No peer credential fails closed; session still survives.
        dispatch_command(&command, &registry, None, None)
            .expect_err("unregister without peer credentials must fail closed");
        assert_eq!(registry.active_sessions().len(), 1);

        // Peer A removes it.
        let response = dispatch_command(&command, &registry, Some(peer_a), None)
            .expect("unregister from the registering peer must be accepted");
        assert_eq!(response["removed"], serde_json::json!(true));
        assert!(registry.active_sessions().is_empty());
    }

    /// MLP2-070 / #1674: a `register-session` frame whose `pid` matches
    /// the authenticated peer is accepted, but the `pid_starttime`
    /// passed downstream is the value the daemon reads from
    /// `/proc/<peer_pid>/stat` — the client-supplied `pid_starttime`
    /// is replaced even when it is wrong. Pins the "lineage body is
    /// advisory, not authoritative" contract from the MLP2-070 spec.
    #[cfg(target_os = "linux")]
    #[test]
    fn dispatch_command_register_lineage_overrides_client_pid_starttime() {
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::LineageAnchor;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let peer_pid = std::process::id();
        let real_starttime = anvil_attribution::process::pid_starttime(peer_pid)
            .expect("pid_starttime for self must succeed on Linux");
        // A clearly-wrong value: 1 second off from the truth. The
        // daemon must ignore this and use the value it reads itself.
        let lying_starttime = real_starttime.wrapping_add(1);
        assert_ne!(lying_starttime, real_starttime, "test fixture sanity");

        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("self-registering"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: None,
            lineage: Some(LineageAnchor {
                pid: peer_pid,
                pid_starttime: lying_starttime,
            }),
        };
        dispatch_command(&command, &dispatcher, Some(peer_pid), None)
            .expect("dispatch must succeed when claim.pid matches peer");

        let calls = recorder.calls();
        let forwarded = match calls.as_slice() {
            [RecordedCall::Register { lineage, .. }] => {
                lineage.expect("lineage must be forwarded when supplied")
            }
            other => panic!("expected single Register call, got {other:?}"),
        };
        assert_eq!(forwarded.pid, peer_pid, "pid must be the peer's");
        assert_eq!(
            forwarded.pid_starttime, real_starttime,
            "pid_starttime must be server-derived, not the client's claim",
        );
    }

    /// MLP2-070 / #1674: backwards-compat — a `register-session` with
    /// no lineage on the wire still works regardless of `peer_pid`.
    /// Pin this so the trust-boundary fix doesn't accidentally
    /// regress the legacy MLP2-023 untagged path.
    /// MLP2-070 / #1674: on platforms where the daemon has no
    /// portable server-side `pid_starttime` reader yet (macOS,
    /// Windows), the lineage anchor stored after verification keeps
    /// the client-supplied `pid_starttime` as advisory. This pins
    /// the explicit non-Linux trade-off documented in
    /// `verify_lineage_claim`'s rustdoc: hard-rejecting lineage on
    /// non-Linux would turn an existing silent-inertness path (the
    /// lookup already returns `None` on those platforms) into a new
    /// hard failure, which is a regression we explicitly avoid
    /// until cross-platform starttime readers land.
    #[cfg(all(unix, not(target_os = "linux")))]
    #[test]
    fn dispatch_command_register_lineage_forwards_advisory_starttime_on_non_linux() {
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::LineageAnchor;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let peer_pid = std::process::id();
        let advisory_starttime = 1_700_000_000;
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("self-registering-non-linux"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: None,
            lineage: Some(LineageAnchor {
                pid: peer_pid,
                pid_starttime: advisory_starttime,
            }),
        };
        dispatch_command(&command, &dispatcher, Some(peer_pid), None)
            .expect("non-Linux dispatch with matching pid must succeed");
        let calls = recorder.calls();
        let forwarded = match calls.as_slice() {
            [RecordedCall::Register { lineage, .. }] => {
                lineage.expect("lineage forwarded on non-Linux")
            }
            other => panic!("expected single Register call, got {other:?}"),
        };
        assert_eq!(forwarded.pid, peer_pid);
        assert_eq!(
            forwarded.pid_starttime, advisory_starttime,
            "non-Linux forwards the claim's advisory starttime verbatim",
        );
    }

    #[cfg(unix)]
    #[test]
    fn dispatch_command_register_without_lineage_is_unaffected() {
        use anvil_intercept_proto::IpcCommand;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("legacy-no-lineage"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: None,
            lineage: None,
        };
        // Both with and without peer_pid: the no-lineage path is the
        // pre-MLP2-025 single-session-per-worktree contract and must
        // remain reachable from every dispatch caller.
        dispatch_command(&command, &dispatcher, None, None)
            .expect("no-lineage dispatch (no peer) must still succeed");
        dispatch_command(&command, &dispatcher, Some(std::process::id()), None)
            .expect("no-lineage dispatch (with peer) must still succeed");

        assert_eq!(
            recorder.calls().len(),
            2,
            "both dispatches must reach the registry",
        );
    }

    // ----------------------------------------------------------------
    // CIB-150: verify the wire `agent_tag` durable-membership claim
    // before honouring it. Any same-UID IPC client can mint an
    // `AgentTag` claiming the activation-spine `claimed_agent_id`; the
    // daemon must only treat that as durable worktree membership when the
    // authenticated peer is independently authorised (it runs the
    // daemon's own `anvil` binary — CLI and daemon share one executable),
    // mirroring the `verify_lineage_claim` peer-derivation. An
    // unauthorised claim is DOWNGRADED to an ordinary live session, never
    // rejected, so a benign mis-tagged client still registers.
    // ----------------------------------------------------------------

    /// CIB-150: a `register-session` frame carrying the activation-spine
    /// `AgentTag` (durable worktree membership) over a connection with no
    /// authenticated peer must NOT be honoured as durable membership. The
    /// daemon cannot prove the caller is its own binary, so the claim is
    /// downgraded to an ordinary live session (registered, but
    /// non-durable) rather than rejected — a benign mis-tagged client
    /// still registers. Platform-agnostic: a missing peer credential is
    /// unauthorised on every platform.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_durable_claim_without_peer_credentials_is_downgraded() {
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let spine = AgentTag::new(
            "anvil-start",
            ACTIVATION_SPINE_CLAIMED_AGENT_ID,
            1_700_000_000,
        );
        assert!(
            spine.is_durable_membership(),
            "fixture must claim durable membership",
        );
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("forged-spine-no-peer"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: Some(spine),
            lineage: None,
        };
        dispatch_command(&command, &dispatcher, None, None)
            .expect("an unauthorised durable claim is downgraded, not rejected");
        let calls = recorder.calls();
        let forwarded = match calls.as_slice() {
            [RecordedCall::Register { agent_tag, .. }] => {
                agent_tag.clone().expect("a tag must still be forwarded")
            }
            other => panic!("expected single Register call, got {other:?}"),
        };
        assert!(
            !forwarded.is_durable_membership(),
            "an unauthorised activation-spine claim must be downgraded to a non-durable tag, got {forwarded:?}",
        );
    }

    /// True when this environment reports `peer_pid`'s `/proc/<pid>/exe` as
    /// this test binary — i.e. the foreign read was aliased to the reader
    /// (issue #3130). An unreadable exe is NOT aliased: the gate fails
    /// closed on it, which the downgrade assertions already cover.
    #[cfg(target_os = "linux")]
    fn peer_exe_reads_as_ours(peer_pid: u32) -> bool {
        let Ok(ours) = std::env::current_exe().and_then(std::fs::canonicalize) else {
            return false;
        };
        let Ok(peer) = std::fs::read_link(format!("/proc/{peer_pid}/exe")) else {
            return false;
        };
        let peer = std::fs::canonicalize(&peer).unwrap_or(peer);
        peer == ours
    }

    /// CIB-150: an activation-spine claim whose authenticated peer is a
    /// real same-UID process running a DIFFERENT binary (not the daemon's
    /// `anvil` executable) is downgraded to a live session. Pins the
    /// `/proc/<peer_pid>/exe` vs daemon `current_exe` authorisation gate
    /// on the platform where it is enforced.
    ///
    /// The premise — the helper's exe reads back as not-this-binary — is
    /// verified per-read rather than via the once-per-process
    /// [`foreign_exe_reads_faithful`] probe: the sandboxed CI runner's
    /// aliasing is time-varying (issue #3130; runs 28678325176 red vs an
    /// earlier green on identical code), so a faithful probe result does
    /// not prove the gate's own read was faithful. When either the pre- or
    /// post-dispatch read aliases to this binary the premise cannot hold
    /// and the test skips (`[SKIP]` on stderr is surfaced by nextest's
    /// `success-output = "immediate"`).
    #[cfg(target_os = "linux")]
    #[test]
    fn dispatch_command_durable_claim_from_non_anvil_peer_is_downgraded() {
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID;

        // A long-lived helper whose /proc/<pid>/exe is NOT this test
        // binary, so the daemon's exe comparison must fail.
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn a non-anvil helper process");
        let peer_pid = child.id();
        if peer_exe_reads_as_ours(peer_pid) {
            eprintln!(
                "[SKIP] dispatch_command_durable_claim_from_non_anvil_peer_is_downgraded: \
                 this environment aliases foreign /proc/<pid>/exe reads to the reader's \
                 binary (issue #3130); the non-anvil-peer premise cannot hold here"
            );
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let spine = AgentTag::new(
            "anvil-start",
            ACTIVATION_SPINE_CLAIMED_AGENT_ID,
            1_700_000_000,
        );
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("forged-spine-non-anvil-peer"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: Some(spine),
            lineage: None,
        };
        let result = dispatch_command(&command, &dispatcher, Some(peer_pid), None);
        // Re-check while the helper is still alive: a faithful pre-read
        // does not prove the gate's own read (inside dispatch) was
        // faithful on a runner with time-varying aliasing.
        let aliased_after = peer_exe_reads_as_ours(peer_pid);
        // Reap the helper regardless of the assertion outcome.
        let _ = child.kill();
        let _ = child.wait();
        result.expect("a non-anvil peer's durable claim is downgraded, not rejected");
        if aliased_after {
            eprintln!(
                "[SKIP] dispatch_command_durable_claim_from_non_anvil_peer_is_downgraded: \
                 foreign /proc/<pid>/exe read aliased to the reader's binary after \
                 dispatch (issue #3130); the gate may have seen an 'anvil' peer"
            );
            return;
        }
        let calls = recorder.calls();
        let forwarded = match calls.as_slice() {
            [RecordedCall::Register { agent_tag, .. }] => {
                agent_tag.clone().expect("a tag must still be forwarded")
            }
            other => panic!("expected single Register call, got {other:?}"),
        };
        assert!(
            !forwarded.is_durable_membership(),
            "a non-anvil peer's activation-spine claim must be downgraded, got {forwarded:?}",
        );
    }

    /// CIB-150: a legitimately authorised caller — the peer runs the same
    /// `anvil` binary as the daemon (here the test process itself, whose
    /// `/proc/<pid>/exe` equals the daemon's `current_exe`) — keeps its
    /// durable activation-spine membership. Guards against the CIB-150
    /// gate regressing genuine durable registration.
    ///
    /// The assertion is conditioned on [`foreign_exe_reads_faithful`]:
    /// where the kernel reports a foreign pid's exe faithfully (real Linux,
    /// including this dev box) the peer is provably the daemon's binary and
    /// its durable claim persists. Under a sandbox that aliases foreign exe
    /// reads to the reader's own binary (the CI micro-VM, run
    /// 28642478053), an authorised peer is *indistinguishable* from a
    /// forger — every same-uid pid reads back as `anvil` — so the gate
    /// correctly fails closed and downgrades. Durable membership is still
    /// reachable there via the daemon's in-process `register_on_start`
    /// path, which never crosses this dispatcher. Asserting persistence
    /// unconditionally is exactly what red-flagged this environment
    /// (`880 passed; 1 failed`); asserting the *contract* keeps the gate
    /// honest on both kinds of kernel.
    #[cfg(target_os = "linux")]
    #[test]
    fn dispatch_command_durable_claim_from_authorised_peer_persists() {
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let peer_pid = std::process::id();
        let worktree = tempfile::tempdir().expect("worktree tempdir");
        let spine = AgentTag::new(
            "anvil-start",
            ACTIVATION_SPINE_CLAIMED_AGENT_ID,
            1_700_000_000,
        );
        let command = IpcCommand::RegisterSession {
            session_id: SessionId::new("authorised-spine"),
            worktree: worktree.path().to_path_buf(),
            agent_tag: Some(spine),
            lineage: None,
        };
        dispatch_command(&command, &dispatcher, Some(peer_pid), None)
            .expect("an authorised durable claim must register");
        let calls = recorder.calls();
        let forwarded = match calls.as_slice() {
            [RecordedCall::Register { agent_tag, .. }] => agent_tag.clone().expect("tag forwarded"),
            other => panic!("expected single Register call, got {other:?}"),
        };
        if foreign_exe_reads_faithful() {
            assert!(
                forwarded.is_durable_membership(),
                "on a kernel with faithful foreign exe reads, an authorised \
                 activation-spine claim must remain durable, got {forwarded:?}",
            );
            assert_eq!(
                forwarded.claimed_agent_id, ACTIVATION_SPINE_CLAIMED_AGENT_ID,
                "the daemon must forward the caller's activation-spine id unchanged",
            );
        } else {
            assert!(
                !forwarded.is_durable_membership(),
                "under a sandbox that cannot verify peer exe identity, the wire \
                 gate must fail closed and downgrade, got {forwarded:?}",
            );
        }
    }

    /// CIB-150: repeated forged activation-spine claims from unauthorised
    /// peers cannot exhaust the separate `registered_worktree_cap`. Each
    /// forged claim is downgraded to a live session before the registry
    /// keys durability off the tag, so the durable membership set stays
    /// empty and a legitimate durable registration is never crowded out.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_forged_durable_claims_cannot_consume_registered_cap() {
        use crate::registry::SessionRegistry;
        use anvil_intercept_proto::IpcCommand;
        use anvil_intercept_proto::session::ACTIVATION_SPINE_CLAIMED_AGENT_ID;

        // A tiny durable budget so a genuine over-run would be observable.
        let registry = Arc::new(SessionRegistry::new().with_registered_worktree_cap(1));
        // Keep the tempdirs alive for the whole test so their canonical
        // paths stay resolvable when the registry reads them back.
        let mut worktrees = Vec::new();
        for i in 0..5 {
            let wt = tempfile::tempdir().expect("worktree tempdir");
            let spine = AgentTag::new(
                "anvil-start",
                ACTIVATION_SPINE_CLAIMED_AGENT_ID,
                1_700_000_000,
            );
            let command = IpcCommand::RegisterSession {
                session_id: SessionId::new(format!("forged-{i}")),
                worktree: wt.path().to_path_buf(),
                agent_tag: Some(spine),
                lineage: None,
            };
            // No authenticated peer → every claim is unauthorised and
            // downgraded to an ordinary (distinct-worktree) live session.
            dispatch_command(&command, &registry, None, None)
                .expect("a downgraded live session registers without error");
            worktrees.push(wt);
        }
        assert!(
            registry.registered_worktrees().is_empty(),
            "forged activation-spine claims must never enter the durable set: {:?}",
            registry.registered_worktrees(),
        );
    }

    // ----------------------------------------------------------------
    // MLP2-074: post-spawn lineage-anchor narrowing dispatch surface.
    // The launcher emits `session.report_process` after spawn; the
    // daemon authenticates the call via peer credentials, swings the
    // lineage index from the launcher to the child, and returns
    // typed errors on credential or ownership failures.
    // ----------------------------------------------------------------

    /// `dispatch_command` routes a `ReportProcess` frame to the
    /// dispatcher with the launcher's peer pid and the child's
    /// `(pid, pid_starttime)`. The child pid is a real running
    /// process (this test's own pid) so the Linux server-side
    /// `pid_starttime` re-derivation (PR #1895 review) finds a
    /// readable `/proc/<pid>/stat`; we assert the dispatcher
    /// receives the daemon-read value, not the wire-supplied one.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_report_process_forwards_peer_and_child_anchor() {
        use anvil_intercept_proto::IpcCommand;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let peer_pid = std::process::id();
        // Use the current process as the "child" so the daemon's
        // `/proc/<pid>/stat` re-derivation succeeds on Linux. On
        // non-Linux the wire value is forwarded unchanged.
        let fake_child_pid = std::process::id();
        let advisory_starttime = 1_700_100_000;
        let command = IpcCommand::ReportProcess {
            session_id: SessionId::new("sess-rp"),
            pid: fake_child_pid,
            pid_starttime: advisory_starttime,
        };
        let result = dispatch_command(&command, &dispatcher, Some(peer_pid), None)
            .expect("report_process must dispatch ok");
        assert_eq!(result["ok"], true);

        let calls = recorder.calls();
        match calls.as_slice() {
            [
                RecordedCall::ReportProcess {
                    id,
                    child_pid,
                    child_pid_starttime,
                    peer_pid: forwarded_peer,
                },
            ] => {
                assert_eq!(id, "sess-rp");
                assert_eq!(*child_pid, fake_child_pid);
                assert_eq!(*forwarded_peer, peer_pid);
                #[cfg(target_os = "linux")]
                {
                    let real = anvil_attribution::process::pid_starttime(fake_child_pid)
                        .expect("self pid_starttime must succeed on Linux");
                    assert_eq!(
                        *child_pid_starttime, real,
                        "Linux dispatch must forward the daemon-read starttime, not the wire value",
                    );
                    assert_ne!(
                        *child_pid_starttime, advisory_starttime,
                        "wire value was deliberately wrong; daemon-read must win",
                    );
                }
                #[cfg(not(target_os = "linux"))]
                {
                    assert_eq!(
                        *child_pid_starttime, advisory_starttime,
                        "non-Linux dispatch forwards the launcher's advisory starttime",
                    );
                }
            }
            other => panic!("expected single ReportProcess call, got {other:?}"),
        }
    }

    /// PR #1895 review: on Linux the daemon re-derives the child's
    /// `pid_starttime` from `/proc/<child_pid>/stat` and ignores
    /// the launcher's claim — mirrors MLP2-070's
    /// `verify_lineage_claim` behaviour for the register-side
    /// lineage anchor. Pinned so a future refactor cannot
    /// silently downgrade this trust-boundary defence.
    #[cfg(target_os = "linux")]
    #[test]
    fn dispatch_command_report_process_overrides_client_pid_starttime_on_linux() {
        use anvil_intercept_proto::IpcCommand;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let peer_pid = std::process::id();
        let child_pid = std::process::id();
        let real_starttime = anvil_attribution::process::pid_starttime(child_pid)
            .expect("pid_starttime for self must succeed on Linux");
        // A clearly-wrong claim: one tick off from the truth. The
        // daemon must read the real value and forward that.
        let lying_starttime = real_starttime.wrapping_add(1);
        assert_ne!(lying_starttime, real_starttime, "test fixture sanity");

        let command = IpcCommand::ReportProcess {
            session_id: SessionId::new("sess-override"),
            pid: child_pid,
            pid_starttime: lying_starttime,
        };
        dispatch_command(&command, &dispatcher, Some(peer_pid), None)
            .expect("dispatch must succeed when child pid_starttime is readable");

        let calls = recorder.calls();
        match calls.as_slice() {
            [
                RecordedCall::ReportProcess {
                    child_pid_starttime,
                    ..
                },
            ] => {
                assert_eq!(
                    *child_pid_starttime, real_starttime,
                    "pid_starttime forwarded to dispatcher must be the daemon-read value",
                );
            }
            other => panic!("expected single ReportProcess call, got {other:?}"),
        }
    }

    /// PR #1895 review: on Linux, if the daemon cannot read
    /// `/proc/<child_pid>/stat` (e.g. the child exited between the
    /// launcher's spawn and the daemon's read), the dispatch
    /// fails closed rather than silently committing an
    /// attacker-chosen starttime to the lineage index. The error
    /// names the failure mode so an operator can chase a benign
    /// race.
    #[cfg(target_os = "linux")]
    #[test]
    fn dispatch_command_report_process_fails_closed_when_child_pid_starttime_unreadable_on_linux() {
        use anvil_intercept_proto::IpcCommand;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let peer_pid = std::process::id();
        // A pid that almost certainly does not exist. `u32::MAX`
        // is well above the kernel's `pid_max` (default 4_194_304,
        // explicit cap 2^22 on 64-bit); the `/proc/<pid>/stat`
        // read returns ENOENT.
        let nonexistent_pid: u32 = u32::MAX;
        let command = IpcCommand::ReportProcess {
            session_id: SessionId::new("sess-gone"),
            pid: nonexistent_pid,
            pid_starttime: 1_700_100_000,
        };
        let err = dispatch_command(&command, &dispatcher, Some(peer_pid), None)
            .expect_err("dispatch must fail closed when /proc read fails");
        assert!(
            err.contains("cannot read pid_starttime"),
            "error must name the read failure, got: {err}",
        );
        assert!(
            recorder.calls().is_empty(),
            "dispatcher.report_process must not be invoked when starttime is unreadable; calls={:?}",
            recorder.calls(),
        );
    }

    /// `dispatch_command` rejects `ReportProcess` when `peer_pid` is
    /// `None` — the legacy NDJSON path and any wire without
    /// `SO_PEERCRED`-style peer reads cannot prove the caller is the
    /// launcher, so we fail closed rather than narrow on
    /// unauthenticated input.
    #[cfg(unix)]
    #[test]
    fn dispatch_command_report_process_requires_peer_credentials() {
        use anvil_intercept_proto::IpcCommand;

        let recorder = Arc::new(RecordingDispatcher::default());
        let dispatcher = Arc::new(Arc::clone(&recorder));
        let command = IpcCommand::ReportProcess {
            session_id: SessionId::new("sess-rp"),
            pid: 5151,
            pid_starttime: 1_700_100_000,
        };
        let err = dispatch_command(&command, &dispatcher, None, None)
            .expect_err("report_process without peer must error");
        assert!(
            err.contains("peer credentials"),
            "error must name the missing credential, got: {err}",
        );
        assert!(
            recorder.calls().is_empty(),
            "dispatcher.report_process must not be invoked when peer_pid is absent; calls={:?}",
            recorder.calls(),
        );
    }

    /// The JSON-RPC method-name parser accepts every spelling the
    /// launcher might use today or after a future rename — the
    /// launcher emits `session.report_process` (underscore), and we
    /// also accept the kebab-case discriminator forms so the wire
    /// remains forward-compatible with the proto enum's
    /// `rename_all`. All forms must produce the same
    /// `IpcCommand::ReportProcess` shape.
    #[test]
    fn command_from_jsonrpc_parses_report_process_aliases() {
        use anvil_intercept_proto::IpcCommand;
        let params = json!({
            "session_id": "sess-rp",
            "pid": 5151,
            "pid_starttime": 1_700_100_000_u64,
            // Launcher also sends pgid + job_object_name today; the
            // parser must silently accept and ignore them. Without
            // this tolerance the launcher's existing wire shape would
            // be a hard reject after we add the method.
            "pgid": 5151,
            "job_object_name": "anvil-intercept-sess-rp",
        });
        for method in [
            "session.report_process",
            "session.report-process",
            "report-process",
            "report_process",
        ] {
            let parsed = match command_from_jsonrpc(method, &params) {
                Ok(cmd) => cmd,
                Err(failure) => panic!(
                    "method {method} must parse, got JSON-RPC failure code={} message={}",
                    failure.code, failure.message,
                ),
            };
            match parsed {
                IpcCommand::ReportProcess {
                    session_id,
                    pid,
                    pid_starttime,
                } => {
                    assert_eq!(session_id.as_str(), "sess-rp");
                    assert_eq!(pid, 5151);
                    assert_eq!(pid_starttime, 1_700_100_000);
                }
                other => panic!("expected ReportProcess, got {other:?} for method {method}"),
            }
        }
    }

    /// Missing or malformed required fields produce typed JSON-RPC
    /// failures (Invalid params), not panics. Pin each of the three
    /// required fields so a future shape edit cannot silently
    /// downgrade one of them to optional.
    #[test]
    fn command_from_jsonrpc_rejects_report_process_with_missing_fields() {
        // Missing session_id.
        let missing_session = json!({"pid": 1, "pid_starttime": 1});
        match command_from_jsonrpc("session.report_process", &missing_session) {
            Ok(_) => panic!("missing session_id must error"),
            Err(failure) => assert_eq!(failure.code, -32602),
        }
        // Missing pid.
        let missing_pid = json!({"session_id": "x", "pid_starttime": 1});
        match command_from_jsonrpc("session.report_process", &missing_pid) {
            Ok(_) => panic!("missing pid must error"),
            Err(failure) => assert_eq!(failure.code, -32602),
        }
        // Missing pid_starttime.
        let missing_starttime = json!({"session_id": "x", "pid": 1});
        match command_from_jsonrpc("session.report_process", &missing_starttime) {
            Ok(_) => panic!("missing pid_starttime must error"),
            Err(failure) => assert_eq!(failure.code, -32602),
        }
        // pid that overflows u32 is a typed error, not a silent
        // truncation. Pinned because the launcher pulls pid from
        // `Child::id()` (always fits) but a malicious caller could
        // hand us a u64 the daemon should refuse on shape rather
        // than ignore.
        let overflow_pid = json!({
            "session_id": "x",
            "pid": u64::from(u32::MAX) + 1,
            "pid_starttime": 1,
        });
        match command_from_jsonrpc("session.report_process", &overflow_pid) {
            Ok(_) => panic!("u32-overflowing pid must error"),
            Err(failure) => assert_eq!(failure.code, -32602),
        }
    }

    /// RCLI3-017b: the JSON-RPC method-name → `IpcCommand` parser
    /// accepts both the kebab-case alias `unblock-worktree` (matching
    /// `IpcCommand` `rename_all`) and the dotted `fence.unblock-worktree`
    /// form. Pinning this protects against accidental name churn that
    /// would silently downgrade a per-fence unblock into "Method not
    /// found".
    #[test]
    fn command_from_jsonrpc_parses_unblock_worktree_aliases() {
        use anvil_intercept_proto::IpcCommand;
        let params = json!({"worktree": "/tmp/wt"});
        for method in ["unblock-worktree", "fence.unblock-worktree"] {
            let parsed = match command_from_jsonrpc(method, &params) {
                Ok(cmd) => cmd,
                Err(failure) => panic!(
                    "method {method} must parse, got JSON-RPC failure code={} message={}",
                    failure.code, failure.message,
                ),
            };
            match parsed {
                IpcCommand::UnblockWorktree { worktree } => {
                    assert_eq!(worktree, PathBuf::from("/tmp/wt"));
                }
                other => panic!("expected UnblockWorktree, got {other:?} for method {method}"),
            }
        }
    }

    /// RCLI3-017b: missing `worktree` param is a typed JSON-RPC
    /// failure (invalid params), not a panic.
    #[test]
    fn command_from_jsonrpc_rejects_unblock_worktree_without_worktree_param() {
        let params = json!({});
        match command_from_jsonrpc("unblock-worktree", &params) {
            Ok(_) => panic!("missing param must error"),
            Err(failure) => assert_eq!(failure.code, -32602, "must be Invalid params"),
        }
    }

    #[cfg(unix)]
    use anvil_intercept_proto::{SessionId, SessionRecord};
    #[cfg(unix)]
    use std::time::Duration;
    #[cfg(unix)]
    use tokio::io::AsyncWriteExt;
    #[cfg(unix)]
    use tokio::net::UnixStream;
    use tracing::field;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::Context;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    #[cfg(unix)]
    use crate::registry::RegistryError;

    #[derive(Debug, Default, Clone)]
    struct RecordedFields(Arc<Mutex<HashMap<String, String>>>);

    impl RecordedFields {
        fn get(&self, key: &str) -> Option<String> {
            self.0.lock().expect("fields").get(key).cloned()
        }
    }

    struct RecordingLayer {
        fields: RecordedFields,
    }

    impl<S> Layer<S> for RecordingLayer
    where
        S: tracing::Subscriber,
        S: for<'lookup> LookupSpan<'lookup>,
    {
        fn on_record(
            &self,
            _span: &tracing::Id,
            values: &tracing::span::Record<'_>,
            _ctx: Context<'_, S>,
        ) {
            values.record(&mut FieldVisitor {
                fields: self.fields.clone(),
            });
        }
    }

    struct FieldVisitor {
        fields: RecordedFields,
    }

    impl field::Visit for FieldVisitor {
        fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
            self.fields
                .0
                .lock()
                .expect("fields")
                .insert(field.name().to_owned(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &field::Field, value: &str) {
            self.fields
                .0
                .lock()
                .expect("fields")
                .insert(field.name().to_owned(), value.to_owned());
        }

        fn record_u64(&mut self, field: &field::Field, value: u64) {
            self.fields
                .0
                .lock()
                .expect("fields")
                .insert(field.name().to_owned(), value.to_string());
        }
    }

    #[test]
    fn trace_method_label_reserves_space_for_ellipsis() {
        let method = "a".repeat(MAX_TRACE_METHOD_LEN + 1);
        let label = trace_method_label(&method);

        assert_eq!(label.len(), MAX_TRACE_METHOD_LEN);
        assert!(label.ends_with("..."));
    }

    // MLP2-071 Phase 2: the subscribe-telemetry `session_ids` filter
    // parses absent / null / empty to `None` (no narrowing) and a
    // populated list to `Some(list)`. An empty array must NOT become an
    // always-deny allow-list (Council footgun fix).
    #[test]
    fn parse_subscriber_session_filter_treats_absent_and_empty_as_none() {
        // Absent filter.
        assert_eq!(
            parse_subscriber_session_filter(&json!({"params": {}})),
            None,
        );
        // Empty session_ids array → None (no narrowing), not allow-none.
        assert_eq!(
            parse_subscriber_session_filter(&json!({"params": {"filter": {"session_ids": []}}})),
            None,
        );
        // Populated list → Some.
        assert_eq!(
            parse_subscriber_session_filter(
                &json!({"params": {"filter": {"session_ids": ["sess-A", "sess-B"]}}})
            ),
            Some(vec!["sess-A".to_string(), "sess-B".to_string()]),
        );
    }

    #[test]
    fn trace_method_label_truncates_on_char_boundary() {
        let method = "é".repeat(MAX_TRACE_METHOD_LEN);
        let label = trace_method_label(&method);

        assert!(label.len() <= MAX_TRACE_METHOD_LEN);
        assert!(label.ends_with("..."));
        assert!(std::str::from_utf8(label.as_bytes()).is_ok());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn jsonrpc_dispatch_span_records_valid_incoming_traceparent() {
        let fields = RecordedFields::default();
        let subscriber = tracing_subscriber::registry().with(RecordingLayer {
            fields: fields.clone(),
        });
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let _guard = tracing::subscriber::set_default(subscriber);
        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": "session.list",
                "id": "trace-test",
                "traceparent": "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(
            response["traceparent"],
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        );
        assert_eq!(
            fields.get("trace_id").as_deref(),
            Some("0af7651916cd43dd8448eb211c80319c")
        );
        assert_eq!(fields.get("parent_id").as_deref(), Some("b7ad6b7169203331"));
        assert_eq!(fields.get("trace_flags").as_deref(), Some("01"));
    }

    /// DSV-005 Task 8: a `validate_paths` frame is routed to the save-time
    /// dispatch arm (not the session dispatcher) and answered with a
    /// verdict-shaped result. With a cold workspace + stubbed feed the verdict
    /// is `Partial`, which is the safe default.
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_validate_paths() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(tmp.path().join("src")).expect("mkdir");
        std::fs::write(tmp.path().join("src/a.ts"), b"export const x = 1;").expect("write");

        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);

        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_VALIDATE_PATHS,
                "id": "vp-1",
                "params": {
                    "workspace_root": tmp.path().to_string_lossy(),
                    "paths": [{"path": "src/a.ts", "change": "modified"}],
                },
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a validate_paths request returns a response");

        // Routed to the save-time handler: a verdict-shaped success, not a
        // `Method not found` from the session dispatcher.
        assert_eq!(response["id"], "vp-1");
        assert!(
            response.get("error").is_none(),
            "validate_paths must route to the save-time arm, not error: {response}",
        );
        let result = &response["result"];
        assert_eq!(result["coverage"], "partial");
        assert_eq!(result["workspace_assurance"]["state"], "stale");
        assert_eq!(result["evaluated"][0]["path"], "src/a.ts");
    }

    /// CIB-149 (relocated-bypass regression): a worktree a peer registered in the
    /// **durable registry** is NOT implicitly admitted in allowlist mode. The
    /// original CIB-149 fix sourced the "implicit primary" from the peer's
    /// registry lineage (`worktree_for_lineage`), but that worktree is just the
    /// path a same-uid client passed to `session.register` — the daemon only
    /// verified *who* the peer is (its PID lineage), never that the *path* should
    /// be admitted. Admitting it would relocate the very bypass CIB-149 closes to
    /// the register frame. This asserts the fail-closed contract: only the
    /// operator's allow entries are admitted; a registered-but-unlisted worktree
    /// is refused exactly like any other unlisted root.
    ///
    /// Linux-gated only for parity with the sibling lineage tests; the admission
    /// path itself no longer walks `/proc` (there is no implicit primary to
    /// resolve).
    #[cfg(target_os = "linux")]
    #[test]
    fn registered_worktree_is_not_implicitly_admitted_in_allowlist() {
        use crate::confinement::{
            AdmissionModeFile, AllowEntry, Confinement, ConfinementConfigFile, MatchKind,
        };
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;
        use anvil_intercept_proto::SessionId;
        use std::time::Instant;

        let allowed = tempfile::tempdir().expect("allow tempdir");
        let registered = tempfile::tempdir().expect("registered worktree tempdir");
        let unlisted = tempfile::tempdir().expect("unlisted tempdir");

        // A same-uid peer registered its self-declared worktree earlier, with a
        // lineage anchor keyed on this test process's own pid (so the registry
        // lineage would resolve it, exactly as the relocated bypass relied on).
        let (ctx, _temp) = make_cross_check_context();
        let self_pid = std::process::id();
        let starttime =
            anvil_attribution::process::pid_starttime(self_pid).expect("pid_starttime for self");
        ctx.registry
            .register_with_lineage(
                &SessionId::new("peer-session"),
                registered.path(),
                None,
                None,
                self_pid,
                starttime,
                Instant::now(),
            )
            .expect("register peer worktree with lineage");

        let confinement = Confinement::from_file(ConfinementConfigFile {
            admission: AdmissionModeFile::Allowlist,
            allow: vec![AllowEntry {
                path: allowed.path().to_path_buf(),
                kind: MatchKind::Exact,
            }],
            ..Default::default()
        });
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            confinement,
        );

        let status = |conn: &mut SaveTimeConn, dir: &std::path::Path| {
            conn.workspace_status(&WorkspaceStatusRequest {
                workspace_root: dir.to_string_lossy().into_owned(),
            })
        };

        // A fresh save-time connection is exactly what the accept-loop builds now
        // — no primary is seeded from the registry lineage at all.
        let mut conn = SaveTimeConn::new(&state);
        assert!(
            matches!(
                status(&mut conn, registered.path()),
                Err(SaveTimeError::NotAdmitted { .. })
            ),
            "a registered-but-unlisted worktree must NOT be implicitly admitted \
             (the relocated CIB-149 bypass is closed)",
        );
        assert!(
            matches!(
                status(&mut conn, unlisted.path()),
                Err(SaveTimeError::NotAdmitted { .. })
            ),
            "an unlisted, unregistered root is refused",
        );
        status(&mut conn, allowed.path()).expect("an allow-listed root is still admitted");
    }

    /// Without save-time state wired the verb is not served — the arm replies
    /// `Method not found` rather than falling through to the session dispatcher.
    #[tokio::test]
    async fn validate_paths_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_VALIDATE_PATHS,
                "id": "vp-2",
                "params": {"workspace_root": "/tmp/x", "paths": []},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// GCTX-010 / ADR-084: an `anvil/gctx/search_symbols` frame routes to the
    /// dedicated GCTX arm (not the session dispatcher) and is answered with a
    /// sealed, assurance-bearing result. A cold worktree degrades in-band to
    /// `not_ready` — a success envelope, not a JSON-RPC error.
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_gctx_search_symbols() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_SEARCH_SYMBOLS,
                "id": "gctx-1",
                "params": {"workspace_root": tmp.path().to_string_lossy(), "query": {}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a gctx search request returns a response");

        assert_eq!(response["id"], "gctx-1");
        assert!(
            response.get("error").is_none(),
            "gctx search must route to the gctx arm, not error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        assert_eq!(response["result"]["workspace_assurance"]["state"], "stale");
    }

    /// Without save-time state wired, the GCTX verb replies `Method not found`
    /// (which the MCP consumer maps to `unavailable`).
    #[tokio::test]
    async fn gctx_search_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_SEARCH_SYMBOLS,
                "id": "gctx-2",
                "params": {"workspace_root": "/tmp/x", "query": {}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// GCTX-011 / ADR-084: an `anvil/gctx/find_dependents` frame routes to the
    /// dedicated GCTX arm and is answered with a sealed, assurance-bearing result.
    /// A cold worktree degrades in-band to `not_ready` (a success envelope).
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_gctx_find_dependents() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_DEPENDENTS,
                "id": "gctx-dep-1",
                "params": {"workspace_root": tmp.path().to_string_lossy(), "query": {"file": "src/a.ts"}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a gctx find_dependents request returns a response");

        assert_eq!(response["id"], "gctx-dep-1");
        assert!(
            response.get("error").is_none(),
            "gctx find_dependents must route to the gctx arm, not error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        assert_eq!(response["result"]["workspace_assurance"]["state"], "stale");
    }

    /// Without save-time state wired, the GCTX dependents verb replies
    /// `Method not found` (which the MCP consumer maps to `unavailable`).
    #[tokio::test]
    async fn gctx_find_dependents_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_DEPENDENTS,
                "id": "gctx-dep-2",
                "params": {"workspace_root": "/tmp/x", "query": {"file": "src/a.ts"}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// GCTX-014 / ADR-084: an `anvil/gctx/find_callers` frame routes to the
    /// dedicated GCTX arm and is answered with a sealed, assurance-bearing result.
    /// A cold worktree degrades in-band to `not_ready` (a success envelope).
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_gctx_find_callers() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_CALLERS,
                "id": "gctx-callers-1",
                "params": {"workspace_root": tmp.path().to_string_lossy(), "query": {"target": {"file": "src/a.ts", "kind": "Function", "name": "handle", "ordinal": 0}}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a gctx find_callers request returns a response");

        assert_eq!(response["id"], "gctx-callers-1");
        assert!(
            response.get("error").is_none(),
            "gctx find_callers must route to the gctx arm, not error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        assert_eq!(response["result"]["workspace_assurance"]["state"], "stale");
    }

    /// Without save-time state wired, the GCTX callers verb replies `Method not
    /// found` (which the MCP consumer maps to `unavailable`).
    #[tokio::test]
    async fn gctx_find_callers_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_FIND_CALLERS,
                "id": "gctx-callers-2",
                "params": {"workspace_root": "/tmp/x", "query": {"target": {"file": "src/a.ts", "kind": "Function", "name": "handle", "ordinal": 0}}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// GCTX-012 / ADR-084: an `anvil/gctx/impact_of_change` frame routes to the
    /// dedicated GCTX arm and is answered with a sealed, assurance-bearing result.
    /// A cold worktree degrades in-band to `not_ready` (a success envelope).
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_gctx_impact_of_change() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_IMPACT_OF_CHANGE,
                "id": "gctx-impact-1",
                "params": {"workspace_root": tmp.path().to_string_lossy(), "query": {"changed_files": ["src/a.ts"]}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a gctx impact request returns a response");

        assert_eq!(response["id"], "gctx-impact-1");
        assert!(
            response.get("error").is_none(),
            "gctx impact must route to the gctx arm, not error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        assert_eq!(response["result"]["workspace_assurance"]["state"], "stale");
    }

    /// Without save-time state wired, the GCTX impact verb replies
    /// `Method not found` (which the MCP consumer maps to `unavailable`).
    #[tokio::test]
    async fn gctx_impact_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_IMPACT_OF_CHANGE,
                "id": "gctx-impact-2",
                "params": {"workspace_root": "/tmp/x", "query": {"changed_files": ["src/a.ts"]}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// GCTX-013 / ADR-084: an `anvil/gctx/affected_tests` frame routes to the
    /// dedicated GCTX arm and is answered with a sealed, assurance-bearing result.
    /// A cold worktree degrades in-band to `not_ready` (a success envelope).
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_gctx_affected_tests() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_AFFECTED_TESTS,
                "id": "gctx-tests-1",
                "params": {"workspace_root": tmp.path().to_string_lossy(), "query": {"changed_files": ["src/a.ts"]}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a gctx affected-tests request returns a response");

        assert_eq!(response["id"], "gctx-tests-1");
        assert!(
            response.get("error").is_none(),
            "gctx affected_tests must route to the gctx arm, not error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        assert_eq!(response["result"]["workspace_assurance"]["state"], "stale");
    }

    /// Without save-time state wired, the GCTX affected-tests verb replies
    /// `Method not found` (which the MCP consumer maps to `unavailable`).
    #[tokio::test]
    async fn gctx_affected_tests_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_AFFECTED_TESTS,
                "id": "gctx-tests-2",
                "params": {"workspace_root": "/tmp/x", "query": {"changed_files": ["src/a.ts"]}},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// GCTX-021 / ADR-084: `anvil/gctx/get_snippet` routes to the dedicated GCTX
    /// arm and degrades a cold worktree in-band to `not_ready`.
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_gctx_get_snippet() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_GET_SNIPPET,
                "id": "gctx-snippet-1",
                "params": {
                    "workspace_root": tmp.path().to_string_lossy(),
                    "query": {
                        "target": {
                            "file": "src/a.ts",
                            "kind": "Function",
                            "name": "greet",
                            "ordinal": 0
                        }
                    }
                },
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a gctx get_snippet request returns a response");

        assert_eq!(response["id"], "gctx-snippet-1");
        assert!(
            response.get("error").is_none(),
            "gctx get_snippet must route to the gctx arm, not error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        assert_eq!(response["result"]["workspace_assurance"]["state"], "stale");
    }

    /// Without save-time state wired, the GCTX `get_snippet` verb replies
    /// `Method not found` (which the MCP consumer maps to `unavailable`).
    #[tokio::test]
    async fn gctx_get_snippet_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_GET_SNIPPET,
                "id": "gctx-snippet-2",
                "params": {
                    "workspace_root": "/tmp/x",
                    "query": {
                        "target": {
                            "file": "src/a.ts",
                            "kind": "Function",
                            "name": "greet",
                            "ordinal": 0
                        }
                    }
                },
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// GCTX-023 / ADR-084: `anvil/gctx/symbol_context` routes to the dedicated
    /// GCTX arm and degrades a cold worktree in-band to `not_ready`.
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_gctx_symbol_context() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_SYMBOL_CONTEXT,
                "id": "gctx-ctx-1",
                "params": {
                    "workspace_root": tmp.path().to_string_lossy(),
                    "query": {
                        "selector": {
                            "file": { "file": "src/a.ts" }
                        }
                    }
                },
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("a gctx symbol_context request returns a response");

        assert_eq!(response["id"], "gctx-ctx-1");
        assert!(
            response.get("error").is_none(),
            "gctx symbol_context must route to the gctx arm, not error: {response}",
        );
        assert_eq!(response["result"]["outcome"]["status"], "not_ready");
        assert_eq!(response["result"]["workspace_assurance"]["state"], "stale");
    }

    /// Without save-time state wired, the GCTX `symbol_context` verb replies
    /// `Method not found` (which the MCP consumer maps to `unavailable`).
    #[tokio::test]
    async fn gctx_symbol_context_method_not_found_without_save_time_state() {
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_GCTX_SYMBOL_CONTEXT,
                "id": "gctx-ctx-2",
                "params": {
                    "workspace_root": "/tmp/x",
                    "query": {
                        "selector": {
                            "file": { "file": "src/a.ts" }
                        }
                    }
                },
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("response");

        assert_eq!(response["error"]["code"], -32601);
    }

    /// DSV-005: `workspace_status` and `request_full_scan` route to the
    /// save-time arm too (not just `validate_paths`), guarding against a
    /// method-constant or routing-condition typo the unit tests would miss.
    #[cfg(unix)]
    #[tokio::test]
    async fn dispatch_arm_routes_workspace_status_and_full_scan() {
        use crate::confinement::Confinement;
        use crate::save_time::{SaveTimeConn, SaveTimeState};
        use crate::workspace_pool::WorkScheduler;
        use anvil_checks::antipattern::types::AntipatternCheckConfig;

        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SaveTimeState::new(
            WorkScheduler::new().expect("scheduler"),
            AntipatternCheckConfig::default(),
            Confinement::open_default(),
        );
        let mut conn = SaveTimeConn::new(&state);
        let dispatcher = Arc::new(NoopDispatcher);
        let scan_buffer = ScanBufferService::default();
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);
        let root = tmp.path().to_string_lossy().into_owned();

        // workspace_status → Stale (B6 cold workspace), not a Method-not-found.
        let ws = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_WORKSPACE_STATUS,
                "id": "ws-1",
                "params": {"workspace_root": root},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("workspace_status response");
        assert!(ws.get("error").is_none(), "must route, not error: {ws}");
        assert_eq!(ws["result"]["workspace_assurance"]["state"], "stale");

        // request_full_scan → Pending (queued job).
        let fs = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": anvil_intercept_proto::protocol::ANVIL_REQUEST_FULL_SCAN,
                "id": "fs-1",
                "params": {"workspace_root": root},
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            Some(&mut conn as &mut dyn SaveTimeDispatch),
        )
        .await
        .expect("request_full_scan response");
        assert!(fs.get("error").is_none(), "must route, not error: {fs}");
        assert_eq!(fs["result"]["workspace_assurance"]["state"], "pending");
    }

    // ----- Recording dispatcher used by behaviour tests. ------------

    #[derive(Debug, Default)]
    #[cfg(unix)]
    struct RecordingDispatcher {
        calls: Mutex<Vec<RecordedCall>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    #[cfg(unix)]
    enum RecordedCall {
        Register {
            id: String,
            worktree: PathBuf,
            agent_tag: Option<anvil_intercept_proto::session::AgentTag>,
            lineage: Option<anvil_intercept_proto::session::LineageAnchor>,
        },
        Heartbeat(String),
        Unregister(String),
        List,
        ReportProcess {
            id: String,
            child_pid: u32,
            child_pid_starttime: u64,
            peer_pid: u32,
        },
    }

    #[cfg(unix)]
    impl RecordingDispatcher {
        fn calls(&self) -> Vec<RecordedCall> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[cfg(unix)]
    impl SessionDispatcher for Arc<RecordingDispatcher> {
        fn register(
            &self,
            id: &SessionId,
            worktree: &Path,
            agent_tag: Option<&anvil_intercept_proto::session::AgentTag>,
            lineage: Option<&anvil_intercept_proto::session::LineageAnchor>,
        ) -> Result<(), RegistryError> {
            self.calls.lock().unwrap().push(RecordedCall::Register {
                id: id.as_str().to_owned(),
                worktree: worktree.to_path_buf(),
                agent_tag: agent_tag.cloned(),
                lineage: lineage.copied(),
            });
            Ok(())
        }
        fn heartbeat(&self, id: &SessionId, _peer_pid: Option<u32>) -> Result<(), RegistryError> {
            // CIB-153: the recorder carries no launcher anchor, so it
            // no-ops the peer_pid. Ownership enforcement is exercised
            // against a real SessionRegistry dispatcher in the
            // dispatch_command_{heartbeat,unregister}_* tests below.
            self.calls
                .lock()
                .unwrap()
                .push(RecordedCall::Heartbeat(id.as_str().to_owned()));
            Ok(())
        }
        fn unregister(
            &self,
            id: &SessionId,
            _peer_pid: Option<u32>,
        ) -> Result<bool, RegistryError> {
            self.calls
                .lock()
                .unwrap()
                .push(RecordedCall::Unregister(id.as_str().to_owned()));
            Ok(true)
        }
        fn list(&self) -> Vec<SessionRecord> {
            self.calls.lock().unwrap().push(RecordedCall::List);
            Vec::new()
        }
        fn report_process(
            &self,
            id: &SessionId,
            child_pid: u32,
            child_pid_starttime: u64,
            peer_pid: u32,
        ) -> Result<(), RegistryError> {
            self.calls
                .lock()
                .unwrap()
                .push(RecordedCall::ReportProcess {
                    id: id.as_str().to_owned(),
                    child_pid,
                    child_pid_starttime,
                    peer_pid,
                });
            Ok(())
        }
    }

    // ----- Path resolution (platform-independent). ------------------

    #[test]
    fn current_user_name_reads_username_then_user_then_logname() {
        // We can't reliably mutate the process env in a unit test
        // without races, so we just verify the function returns
        // something sane for the current process.
        let user =
            current_user_name().expect("test host has at least one of USER/USERNAME/LOGNAME");
        assert!(!user.is_empty(), "user name must not be empty");
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_prefers_xdg_runtime_dir() {
        let dir = resolve_socket_dir_with_env(
            None,
            Some("/run/user/1000".into()),
            Some("/home/somebody".into()),
        )
        .expect("resolve");
        assert_eq!(dir, PathBuf::from("/run/user/1000/anvil"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_falls_back_to_home_state() {
        let dir = resolve_socket_dir_with_env(None, None, Some("/home/somebody".into()))
            .expect("resolve");
        assert_eq!(dir, PathBuf::from("/home/somebody/.local/state/anvil"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_treats_empty_xdg_as_unset() {
        let dir = resolve_socket_dir_with_env(None, Some("".into()), Some("/home/somebody".into()))
            .expect("resolve");
        assert_eq!(dir, PathBuf::from("/home/somebody/.local/state/anvil"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_anvil_home_re_roots_socket_directly_under_prefix() {
        // DISTRIB-006: ANVIL_HOME takes precedence over the runtime dir and puts
        // the socket directly under the prefix, so a candidate daemon coexists.
        let dir = resolve_socket_dir_with_env(
            Some("/opt/anvil-beta".into()),
            Some("/run/user/1000".into()),
            Some("/home/somebody".into()),
        )
        .expect("resolve");
        assert_eq!(dir, PathBuf::from("/opt/anvil-beta"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_treats_empty_anvil_home_as_unset() {
        let dir = resolve_socket_dir_with_env(
            Some("".into()),
            Some("/run/user/1000".into()),
            Some("/home/somebody".into()),
        )
        .expect("resolve");
        assert_eq!(dir, PathBuf::from("/run/user/1000/anvil"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_errors_when_no_candidate() {
        let err = resolve_socket_dir_with_env(None, None, None).unwrap_err();
        assert!(matches!(err, IpcError::NoSocketDirCandidate));
    }

    #[cfg(windows)]
    #[test]
    fn resolve_pipe_name_uses_user_suffix() {
        let name = resolve_pipe_name().expect("resolve");
        assert!(name.starts_with(r"\\.\pipe\anvil-intercept-"), "got {name}");
    }

    /// CIB-106: pure pipe-name derivation — platform-independent tests
    /// mirroring the Unix `resolve_socket_dir_with_env` coverage. The
    /// unset/blank/relative normalisation is exercised through the same
    /// `anvil_home_prefix_from` seam production uses, so client and daemon
    /// cannot disagree on the rendezvous name.
    mod pipe_name_resolution {
        use std::ffi::OsString;
        use std::path::{Path, PathBuf};

        use super::super::derive_pipe_name;

        const SID: &str = "S-1-5-21-1-2-3-1000";
        const LEGACY: &str = r"\\.\pipe\anvil-intercept-S-1-5-21-1-2-3-1000";

        /// Compose the resolver exactly the way `resolve_pipe_name` does:
        /// raw env value → normalised install-root prefix → pipe name.
        fn name_for(raw: Option<&str>, cwd: Option<&Path>) -> String {
            let prefix =
                crate::anvil_home_prefix_from(raw.map(OsString::from), cwd.map(Path::to_path_buf));
            derive_pipe_name(SID, prefix.as_deref())
        }

        /// An absolute directory that exists as a plain string on every
        /// host this test compiles on (drive-rooted on Windows).
        fn abs_root(leaf: &str) -> PathBuf {
            if cfg!(windows) {
                Path::new(r"C:\anvil-roots").join(leaf)
            } else {
                Path::new("/anvil-roots").join(leaf)
            }
        }

        /// Backward compatibility is critical: with `ANVIL_HOME` unset the
        /// pipe name is byte-for-byte the legacy `\\.\pipe\anvil-intercept-<sid>`
        /// so existing installs keep the same rendezvous point.
        #[test]
        fn unset_anvil_home_keeps_the_legacy_pipe_name() {
            assert_eq!(name_for(None, None), LEGACY);
        }

        /// Blank / whitespace-only `ANVIL_HOME` is treated as unset — same
        /// posture as the Unix socket resolver and the CLI install-root
        /// resolver.
        #[test]
        fn blank_and_whitespace_anvil_home_keep_the_legacy_pipe_name() {
            assert_eq!(name_for(Some(""), None), LEGACY);
            assert_eq!(name_for(Some("   "), None), LEGACY);
            assert_eq!(name_for(Some(" \t "), None), LEGACY);
        }

        /// A relative `ANVIL_HOME` is absolutised against the current
        /// directory, so a CLI client and a separately-spawned daemon that
        /// share the env agree on the pipe name — mirroring the Unix socket
        /// path guarantee.
        #[test]
        fn relative_anvil_home_absolutises_against_cwd() {
            let cwd = abs_root("cwd");
            let absolute = cwd.join("candidate-a");
            assert_eq!(
                name_for(Some("candidate-a"), Some(&cwd)),
                name_for(absolute.to_str(), None),
            );
        }

        /// Same install root → same name, every time (stable hash — the
        /// daemon bind and every client resolve must rendezvous).
        #[test]
        fn same_root_derives_the_same_name() {
            let root = abs_root("candidate-a");
            let first = derive_pipe_name(SID, Some(&root));
            let second = derive_pipe_name(SID, Some(&root));
            assert_eq!(first, second);
            assert_ne!(first, LEGACY, "re-rooted name must not collide with legacy");
        }

        /// Distinct install roots → distinct pipe names, so two same-user
        /// candidate daemons coexist (the DISTRIB-006 side-by-side goal).
        #[test]
        fn distinct_roots_derive_distinct_names() {
            let a = derive_pipe_name(SID, Some(&abs_root("candidate-a")));
            let b = derive_pipe_name(SID, Some(&abs_root("candidate-b")));
            assert_ne!(a, b);
        }

        /// The install-root path never appears in the pipe name (pipe names
        /// are enumerable by other local users, so a raw path would leak
        /// directory layout). The suffix is a bounded lowercase-hex hash.
        #[test]
        fn rerooted_name_does_not_leak_the_raw_path() {
            let root = abs_root("secret-install-root");
            let name = derive_pipe_name(SID, Some(&root));
            assert!(
                !name.contains("secret-install-root")
                    && !name.contains("anvil-roots")
                    && !name.contains('/')
                    && !name.contains(':'),
                "raw path text leaked into pipe name: {name}",
            );
            let suffix = name
                .strip_prefix(LEGACY)
                .expect("re-rooted name extends the legacy prefix");
            let hash = suffix
                .strip_prefix("-r")
                .expect("suffix is `-r<hash>` namespace marker");
            assert_eq!(hash.len(), 16, "bounded 64-bit hex hash: {hash}");
            assert!(
                hash.chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
                "hash must be lowercase hex: {hash}",
            );
        }

        /// Golden pin: the derivation is part of the daemon/client
        /// rendezvous contract — a refactor that changes the hash silently
        /// strands existing candidate daemons on an unreachable name.
        #[test]
        fn rerooted_name_hash_is_pinned() {
            let name = derive_pipe_name(SID, Some(Path::new("/opt/anvil-candidate")));
            assert_eq!(name, format!("{LEGACY}-r1bee9a017cfa049e"));
        }
    }

    #[cfg(windows)]
    #[tokio::test(flavor = "current_thread")]
    async fn named_pipe_scan_buffer_smoke_uses_injected_service() {
        use anvil_intercept_rules::RuleRegistry;
        use serde_json::json;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::windows::named_pipe::ClientOptions;

        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-scan-buffer-test-{}",
            std::process::id(),
        );
        let listener = IpcListener::bind_with_scan_buffer_service(
            &pipe_name,
            NoopDispatcher,
            ScanBufferService::new(crate::enforcement::EnforcementPipeline::new(
                RuleRegistry::new(),
            )),
        )
        .expect("bind listener");
        let (shutdown, token) = crate::Shutdown::new();
        let handle = tokio::spawn(async move { listener.serve(token).await });

        let mut client = ClientOptions::new().open(&pipe_name).expect("open pipe");
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": "src/auth/client.ts",
                "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
                "version": 1,
                "mode": "midEdit"
            },
            "id": "windows-scan"
        });
        client
            .write_all(format!("{frame}\n").as_bytes())
            .await
            .expect("write request");

        let mut reader = BufReader::new(client);
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("response timeout")
            .expect("read response");
        let response: Value = serde_json::from_str(line.trim_end()).expect("response json");
        assert_eq!(response["id"], "windows-scan");
        assert_eq!(response["result"]["diagnostics"], json!([]));

        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("listener timeout")
            .expect("listener join")
            .expect("listener ok");
    }

    // INTD-012 (A2 Wave 1) parity-test response shapes. Re-implemented
    // here (rather than reused from anvil-cli) so the test does not
    // couple to cli-internal struct definitions; hoisted out of the
    // test body to satisfy clippy's `too_many_lines` and
    // `items_after_statements` lints.
    #[cfg(target_os = "windows")]
    #[derive(Debug, serde::Deserialize)]
    struct WindowsParityScanBufferResult {
        version: u64,
        diagnostics: Vec<anvil_kernel_types::Diagnostic>,
        truncated: bool,
    }
    #[cfg(target_os = "windows")]
    #[derive(Debug, serde::Deserialize)]
    struct WindowsParityJsonRpcResponse {
        jsonrpc: String,
        id: Option<Value>,
        #[serde(default)]
        result: Option<WindowsParityScanBufferResult>,
        #[serde(default)]
        error: Option<Value>,
    }

    // INTD-012 (A2 Wave 1): mirror of the Linux UDS parity assertion
    // at `crates/anvil-cli/src/mcp/validation.rs::tests::
    // local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`.
    //
    // Pinned where the surface lives (the `IpcListener` named-pipe
    // path) rather than in `anvil-cli`, because the cli's
    // `request_daemon_diagnostics` is `#[cfg(unix)]` only and rewiring
    // it for Windows would change daemon error semantics — out of
    // scope for INTD-012 per the A2 Wave 1 hard rules.
    //
    // The intent is a fail-closed gate: if a future change desyncs the
    // Windows daemon-backed `scan_buffer` envelope from the embedded
    // `EnforcementPipeline` path (e.g. a new diagnostic field added on
    // one side but not the other, or a serialisation difference), this
    // test breaks at the next release-path Windows run rather than
    // shipping silently.
    #[cfg(target_os = "windows")]
    #[tokio::test(flavor = "current_thread")]
    async fn named_pipe_scan_buffer_envelope_parity_with_embedded() {
        use serde_json::json;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::windows::named_pipe::ClientOptions;

        use crate::enforcement::{EnforcementPipeline, ProposedChange, default_rule_registry};
        use anvil_intercept_rules::ChangeKind;
        use anvil_kernel_types::Mode;

        // Same fixture content as the Linux parity test in anvil-cli; a
        // unique pipe name per test run keeps concurrent runners from
        // colliding on the singleton-claiming first instance.
        const PRE_WRITE_MODE: &str = "pre-write";
        let path = "src/secret.ts";
        let content = "const token = 'ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';\n";
        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-parity-test-{}",
            std::process::id(),
        );

        // Embedded reference: the canonical pipeline path that the
        // `EnforcementPipeline::default()` chain feeds inside
        // `embedded_validate_pre_write` in `anvil-cli`. Keeping the
        // registry construction identical (`default_rule_registry()`)
        // is the load-bearing part of "parity".
        let embedded_pipeline = EnforcementPipeline::new(default_rule_registry());
        let embedded_change = ProposedChange {
            path: std::path::Path::new(path),
            change_kind: ChangeKind::Modified,
            content: Some(content.as_bytes()),
        };
        let embedded_diagnostics = embedded_pipeline.diagnostics_for_proposed_changes(
            &[embedded_change],
            &Mode::Unknown(PRE_WRITE_MODE.to_string()),
        );

        // Sanity: secret detection must produce at least one diagnostic
        // for this fixture. If this regresses, the test stops being a
        // meaningful parity gate.
        assert!(
            !embedded_diagnostics.is_empty(),
            "embedded pipeline must produce diagnostics for the secret fixture",
        );
        assert_eq!(embedded_diagnostics[0].source.rule_id, "secret-detection");

        // Daemon-backed: same registry behind the IPC listener so any
        // diagnostic-shape difference must come from the JSON-RPC
        // transport itself, not the rule pipeline.
        let daemon_pipeline = EnforcementPipeline::new(default_rule_registry());
        let listener = IpcListener::bind_with_scan_buffer_service(
            &pipe_name,
            NoopDispatcher,
            ScanBufferService::new(daemon_pipeline),
        )
        .expect("bind named-pipe listener");
        let (shutdown, token) = crate::Shutdown::new();
        let handle = tokio::spawn(async move { listener.serve(token).await });

        let mut client = ClientOptions::new().open(&pipe_name).expect("open pipe");
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": path,
                "text": content,
                "version": 1,
                "mode": "preWrite"
            },
            "id": "windows-parity"
        });
        client
            .write_all(format!("{frame}\n").as_bytes())
            .await
            .expect("write request");

        let mut reader = BufReader::new(client);
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
            .await
            .expect("response timeout")
            .expect("read response");

        // Stop the listener as soon as we have the response; assertions
        // run after teardown so a panic still tears the daemon down.
        shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("listener timeout")
            .expect("listener join")
            .expect("listener ok");

        let response: WindowsParityJsonRpcResponse =
            serde_json::from_str(line.trim_end()).expect("response json");

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(
            response.id,
            Some(Value::String("windows-parity".to_string()))
        );
        assert!(
            response.error.is_none(),
            "daemon must not return a JSON-RPC error for the secret fixture: {:?}",
            response.error,
        );
        let result = response.result.expect("result populated");
        assert_eq!(result.version, 1, "scan_buffer result version pinned at 1");
        assert!(
            !result.truncated,
            "single-secret fixture must not exceed the diagnostic cap",
        );

        // The load-bearing parity assertion. `Diagnostic` derives
        // PartialEq, so this compares every field of every diagnostic
        // including `schema_version` (the `anvil.diagnostic.v1` pin),
        // `mode`, `source`, `location`, `category`, and the optional
        // `remediation_hint`.
        assert_eq!(
            result.diagnostics, embedded_diagnostics,
            "named-pipe scan_buffer diagnostics must match embedded pipeline byte-for-byte",
        );
    }

    // ----- Unix permission and bind tests. --------------------------

    #[cfg(unix)]
    mod unix {
        use super::*;
        use nix::unistd::Uid;
        use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

        fn current_uid() -> u32 {
            Uid::current().as_raw()
        }

        #[test]
        fn ensure_dir_creates_with_mode_0700_when_missing() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("anvil");
            assert!(!target.exists());
            unix_perms::ensure_dir(&target, current_uid()).expect("create");
            let meta = std::fs::symlink_metadata(&target).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o700);
            assert_eq!(meta.uid(), current_uid());
        }

        #[test]
        fn ensure_dir_accepts_existing_with_mode_0700() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("anvil");
            std::fs::create_dir(&target).unwrap();
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
            unix_perms::ensure_dir(&target, current_uid()).expect("verify");
        }

        #[test]
        fn ensure_dir_refuses_existing_with_wrong_mode() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("anvil");
            std::fs::create_dir(&target).unwrap();
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
            let err = unix_perms::ensure_dir(&target, current_uid()).unwrap_err();
            assert!(
                matches!(err, IpcError::SocketDirPermissions { mode: 0o755, .. }),
                "unexpected error: {err:?}"
            );
        }

        #[test]
        fn ensure_dir_refuses_symlink() {
            let tmp = tempfile::tempdir().unwrap();
            let real = tmp.path().join("real");
            std::fs::create_dir(&real).unwrap();
            std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700)).unwrap();
            let link = tmp.path().join("link");
            symlink(&real, &link).unwrap();
            let err = unix_perms::ensure_dir(&link, current_uid()).unwrap_err();
            assert!(
                matches!(err, IpcError::SocketDirIsSymlink(_)),
                "got {err:?}"
            );
        }

        // -------- Peer-credential validation. ------------------------
        //
        // Both branches (Linux `SO_PEERCRED` and macOS `getpeereid`) must
        // accept a same-UID peer. We exercise the function against a
        // `UnixStream::pair()` — both ends of the pair share the calling
        // process's uid, so the peer-uid check should succeed. The test is
        // gated per-target because the function body itself diverges per
        // target; gating ensures the macOS Cross matrix entry actually
        // exercises the macOS branch rather than relying on the Linux
        // branch's coverage.
        #[cfg(target_os = "linux")]
        #[test]
        fn validate_connected_peer_accepts_same_uid_linux() {
            let (a, _b) = std::os::unix::net::UnixStream::pair().expect("socket pair");
            validate_connected_peer_for_client(&a).expect("same-uid peer accepted");
        }

        #[cfg(target_os = "macos")]
        #[test]
        fn validate_connected_peer_accepts_same_uid_macos() {
            let (a, _b) = std::os::unix::net::UnixStream::pair().expect("socket pair");
            validate_connected_peer_for_client(&a).expect("same-uid peer accepted");
        }

        // -------- Bind / serve tests against real sockets. ----------

        fn fresh_socket_path() -> (tempfile::TempDir, PathBuf) {
            let tmp = tempfile::tempdir().unwrap();
            let dir = tmp.path().join("anvil");
            // intentionally do not create — bind() should create it
            // with the correct mode.
            let path = dir.join("intercept.sock");
            (tmp, path)
        }

        #[tokio::test(flavor = "current_thread")]
        async fn bind_creates_dir_0700_and_socket_0600() {
            let (_tmp, path) = fresh_socket_path();
            let listener: IpcListener<NoopDispatcher> =
                IpcListener::bind(&path, NoopDispatcher).expect("bind");

            let dir_meta = std::fs::symlink_metadata(path.parent().unwrap()).unwrap();
            assert_eq!(dir_meta.permissions().mode() & 0o777, 0o700);

            let sock_meta = std::fs::symlink_metadata(&path).unwrap();
            assert_eq!(sock_meta.permissions().mode() & 0o777, 0o600);

            drop(listener);
        }

        #[tokio::test(flavor = "current_thread")]
        async fn client_validation_accepts_bound_owner_only_socket() {
            let (_tmp, path) = fresh_socket_path();
            let listener: IpcListener<NoopDispatcher> =
                IpcListener::bind(&path, NoopDispatcher).expect("bind");

            validate_socket_path_for_client(&path).expect("client-side validation accepts socket");

            drop(listener);
        }

        #[test]
        fn client_validation_refuses_regular_file_socket_path() {
            let (_tmp, path) = fresh_socket_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::set_permissions(
                path.parent().unwrap(),
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
            std::fs::write(&path, b"not a socket").unwrap();

            let err = validate_socket_path_for_client(&path).unwrap_err();

            assert!(
                matches!(err, IpcError::SocketPathNotASocket(_)),
                "got {err:?}"
            );
        }

        #[tokio::test(flavor = "current_thread")]
        async fn client_validation_refuses_socket_with_wide_permissions() {
            let (_tmp, path) = fresh_socket_path();
            let listener: IpcListener<NoopDispatcher> =
                IpcListener::bind(&path, NoopDispatcher).expect("bind");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();

            let err = validate_socket_path_for_client(&path).unwrap_err();

            assert!(
                matches!(err, IpcError::SocketPathPermissions { mode: 0o666, .. }),
                "got {err:?}"
            );
            drop(listener);
        }

        #[tokio::test(flavor = "current_thread")]
        async fn bind_refuses_when_dir_has_wrong_mode() {
            let tmp = tempfile::tempdir().unwrap();
            let dir = tmp.path().join("anvil");
            std::fs::create_dir(&dir).unwrap();
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
            let path = dir.join("intercept.sock");
            let err = IpcListener::bind(&path, NoopDispatcher).err().unwrap();
            assert!(
                matches!(err, IpcError::SocketDirPermissions { mode: 0o755, .. }),
                "got {err:?}"
            );
        }

        #[tokio::test(flavor = "current_thread")]
        async fn bind_refuses_when_socket_parent_is_symlink() {
            let tmp = tempfile::tempdir().unwrap();
            let real = tmp.path().join("real");
            std::fs::create_dir(&real).unwrap();
            std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700)).unwrap();
            let link = tmp.path().join("link");
            symlink(&real, &link).unwrap();
            let path = link.join("intercept.sock");
            let err = IpcListener::bind(&path, NoopDispatcher).err().unwrap();
            assert!(
                matches!(err, IpcError::SocketDirIsSymlink(_)),
                "got {err:?}"
            );
        }

        #[tokio::test(flavor = "current_thread")]
        async fn bind_refuses_when_socket_path_is_symlink() {
            let (_tmp, path) = fresh_socket_path();
            // Pre-create the parent dir at 0700.
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::set_permissions(
                path.parent().unwrap(),
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
            // Plant a dangling symlink at the socket path.
            symlink("/nonexistent/anvil-target", &path).unwrap();
            let err = IpcListener::bind(&path, NoopDispatcher).err().unwrap();
            assert!(
                matches!(err, IpcError::SocketPathIsSymlink(_)),
                "got {err:?}"
            );
        }

        #[tokio::test(flavor = "current_thread")]
        async fn bind_unlinks_stale_socket_with_no_listener() {
            let (_tmp, path) = fresh_socket_path();
            // First bind creates the socket and the parent dir at 0700.
            let first: IpcListener<NoopDispatcher> =
                IpcListener::bind(&path, NoopDispatcher).expect("first bind");
            // Drop the listener WITHOUT removing the socket file —
            // simulate a daemon that crashed before its `serve`
            // cleanup ran. `into_std` then forget gives us that.
            let std_listener = first.inner.into_std().unwrap();
            drop(std_listener);
            assert!(path.exists(), "stale socket should remain on disk");
            // Second bind sees the stale socket, fails to connect,
            // unlinks, and rebinds.
            let second: IpcListener<NoopDispatcher> =
                IpcListener::bind(&path, NoopDispatcher).expect("second bind unlinks stale");
            drop(second);
        }

        #[tokio::test(flavor = "current_thread")]
        async fn bind_refuses_when_live_listener_already_holds_path() {
            let (_tmp, path) = fresh_socket_path();
            let _first: IpcListener<NoopDispatcher> =
                IpcListener::bind(&path, NoopDispatcher).expect("first bind");
            let err = IpcListener::bind(&path, NoopDispatcher).err().unwrap();
            assert!(
                matches!(err, IpcError::AnotherDaemonRunning(_)),
                "got {err:?}"
            );
            // PID-file tie-in lands later — for INTD-002 the
            // socket-bind contention is sufficient.
        }

        #[tokio::test(flavor = "current_thread")]
        async fn bind_refuses_when_path_is_a_regular_file() {
            let (_tmp, path) = fresh_socket_path();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::set_permissions(
                path.parent().unwrap(),
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
            std::fs::write(&path, b"not a socket").unwrap();
            let err = IpcListener::bind(&path, NoopDispatcher).err().unwrap();
            assert!(
                matches!(err, IpcError::SocketPathNotASocket(_)),
                "got {err:?}"
            );
        }

        // --------- NDJSON dispatch tests. --------------------------

        async fn spawn_listener_with_dispatcher(
            path: &Path,
            dispatcher: Arc<RecordingDispatcher>,
        ) -> (
            crate::Shutdown,
            tokio::task::JoinHandle<Result<(), IpcError>>,
        ) {
            let listener = IpcListener::bind(path, Arc::clone(&dispatcher)).expect("bind");
            let (shutdown, token) = crate::Shutdown::new();
            let handle = tokio::spawn(async move { listener.serve(token).await });
            // Yield once so the listener enters its accept loop.
            tokio::task::yield_now().await;
            (shutdown, handle)
        }

        #[tokio::test(flavor = "current_thread")]
        async fn ndjson_register_session_dispatches() {
            let (_tmp, path) = fresh_socket_path();
            let dispatcher = Arc::new(RecordingDispatcher::default());
            let (shutdown, handle) =
                spawn_listener_with_dispatcher(&path, Arc::clone(&dispatcher)).await;

            let mut stream = UnixStream::connect(&path).await.expect("connect");
            let envelope = IpcEnvelope::request(
                "req-1",
                IpcCommand::RegisterSession {
                    session_id: SessionId::new("sess_abc"),
                    worktree: PathBuf::from("/tmp/wt-abc"),
                    agent_tag: None,
                    lineage: None,
                },
            );
            let mut line = serde_json::to_string(&envelope).unwrap();
            line.push('\n');
            stream.write_all(line.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();

            // Wait briefly for the handler to record the call.
            for _ in 0..50 {
                if !dispatcher.calls().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(
                dispatcher.calls(),
                vec![RecordedCall::Register {
                    id: "sess_abc".into(),
                    worktree: PathBuf::from("/tmp/wt-abc"),
                    agent_tag: None,
                    lineage: None,
                }]
            );

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .expect("listener did not return after shutdown")
                .expect("join")
                .expect("serve");
        }

        // -------- MLP2-071 Phase 2: telemetry subscriber e2e. --------
        //
        // End-to-end over a real Unix socket: a client subscribes, the
        // daemon mints its SubscriberId from `SO_PEERCRED` and registers
        // it with the broadcaster, a `broadcast(...)` reaches the client
        // as a `telemetry.event` notification frame, a cross-session
        // envelope is denied under the default policy, and unsubscribe
        // tears the stream down. Linux-gated because subscriber minting
        // needs `/proc/<pid>/stat`.
        #[cfg(target_os = "linux")]
        struct SessionAllowResolver {
            allowed: std::collections::HashSet<String>,
        }

        #[cfg(target_os = "linux")]
        impl crate::fanout::OwnershipResolver for SessionAllowResolver {
            fn is_authorised(
                &self,
                _subscriber: &crate::fanout::SubscriberId,
                originating_session_id: &str,
            ) -> bool {
                self.allowed.contains(originating_session_id)
            }
        }

        #[cfg(target_os = "linux")]
        fn telemetry_envelope(session_id: &str) -> crate::telemetry::NotificationEnvelope {
            use crate::enforcement::{EnforcementDecision, InterruptDecision};
            use crate::telemetry::{TelemetryCorrelation, TelemetryEmitter};
            let mut emitter = TelemetryEmitter::for_tests("e2e", "2026-06-08T00:00:00Z");
            let decision = EnforcementDecision::Interrupt(InterruptDecision {
                rule_id: "anvil.secret.aws".to_string(),
                message: "secret leaked".to_string(),
                line: Some(1),
                affected_paths: vec![PathBuf::from("src/secret.ts")],
            });
            let correlation = TelemetryCorrelation {
                session_id: Some(session_id.to_string()),
                originating_session_id: Some(session_id.to_string()),
                originating_driver_id: Some("driver-e2e".to_string()),
                ..TelemetryCorrelation::default()
            };
            emitter.delivered_envelope_for_decision(correlation, &decision)
        }

        #[cfg(target_os = "linux")]
        #[tokio::test(flavor = "current_thread")]
        async fn subscribe_telemetry_streams_owned_envelopes_over_socket() {
            use crate::broadcaster::TelemetryBroadcaster;
            use crate::fanout::{CrossSessionPolicy, Fanout};
            use std::collections::HashSet;

            let (_tmp, path) = fresh_socket_path();

            // The subscribing connection owns "sess-owned"; "sess-foreign"
            // is someone else's session. Default cross-session policy is
            // Deny.
            let resolver = SessionAllowResolver {
                allowed: HashSet::from(["sess-owned".to_string()]),
            };
            let fanout = Arc::new(Fanout::with_cross_session_policy(
                Box::new(resolver),
                CrossSessionPolicy::Deny,
            ));
            let broadcaster = Arc::new(TelemetryBroadcaster::new(Arc::clone(&fanout)));

            let listener = IpcListener::bind(&path, NoopDispatcher)
                .expect("bind")
                .with_broadcaster(Arc::clone(&broadcaster));
            let (shutdown, token) = crate::Shutdown::new();
            let handle = tokio::spawn(async move { listener.serve(token).await });
            tokio::task::yield_now().await;

            let stream = UnixStream::connect(&path).await.expect("connect");
            let mut reader = tokio::io::BufReader::new(stream);

            // Subscribe and read the ack. The ack confirms the daemon has
            // registered us with the broadcaster, so a subsequent
            // `broadcast` cannot race ahead of registration.
            let subscribe = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "subscribe-telemetry",
                "params": {},
                "id": "sub-1",
            });
            {
                use tokio::io::AsyncWriteExt;
                reader
                    .get_mut()
                    .write_all(format!("{subscribe}\n").as_bytes())
                    .await
                    .expect("write subscribe");
            }
            let ack = read_one_line(&mut reader).await;
            assert_eq!(ack["id"], "sub-1");
            assert_eq!(ack["result"]["subscribed"], true);
            assert_eq!(
                broadcaster.subscriber_count(),
                1,
                "the daemon must have registered the subscriber before acking"
            );

            // Own-session envelope → delivered as a telemetry.event frame.
            let outcome = broadcaster.broadcast(&telemetry_envelope("sess-owned"));
            assert_eq!(outcome.delivered, 1, "owned-session envelope must deliver");
            let frame = read_one_line(&mut reader).await;
            assert_eq!(
                frame["method"],
                crate::broadcaster::TELEMETRY_NOTIFICATION_METHOD
            );
            assert!(
                frame.get("id").is_none(),
                "telemetry frames are notifications"
            );
            assert_eq!(
                frame["params"]["correlation"]["originatingSessionId"]
                    .as_str()
                    .or_else(|| frame["params"]["correlation"]["originating_session_id"].as_str()),
                Some("sess-owned"),
            );

            // Cross-session envelope under Deny → not delivered.
            let outcome = broadcaster.broadcast(&telemetry_envelope("sess-foreign"));
            assert_eq!(outcome.delivered, 0, "cross-session deny must not deliver");

            // Unsubscribe → ack + broadcaster drops us.
            let unsubscribe = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "unsubscribe-telemetry",
                "id": "unsub-1",
            });
            {
                use tokio::io::AsyncWriteExt;
                reader
                    .get_mut()
                    .write_all(format!("{unsubscribe}\n").as_bytes())
                    .await
                    .expect("write unsubscribe");
            }
            let ack = read_one_line(&mut reader).await;
            assert_eq!(ack["id"], "unsub-1");
            assert_eq!(ack["result"]["subscribed"], false);
            for _ in 0..50 {
                if broadcaster.subscriber_count() == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(
                broadcaster.subscriber_count(),
                0,
                "unsubscribe must unregister the subscriber"
            );

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .expect("listener timeout")
                .expect("join")
                .expect("serve");
        }

        // Read exactly one NDJSON line from the connection and parse it
        // as JSON, with a timeout so a delivery bug fails the test
        // instead of hanging it.
        #[cfg(target_os = "linux")]
        async fn read_one_line<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Value {
            use tokio::io::AsyncBufReadExt;
            let mut line = String::new();
            tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line))
                .await
                .expect("read timed out")
                .expect("read line");
            serde_json::from_str(line.trim_end()).expect("parse json frame")
        }

        #[tokio::test(flavor = "current_thread")]
        async fn ndjson_malformed_line_is_skipped_and_next_line_dispatches() {
            let (_tmp, path) = fresh_socket_path();
            let dispatcher = Arc::new(RecordingDispatcher::default());
            let (shutdown, handle) =
                spawn_listener_with_dispatcher(&path, Arc::clone(&dispatcher)).await;

            let mut stream = UnixStream::connect(&path).await.expect("connect");
            // First line: structurally valid JSON but unknown command.
            stream
                .write_all(b"{\"command\":\"future-unknown\",\"session_id\":\"x\"}\n")
                .await
                .unwrap();
            // Second line: also nonsense.
            stream.write_all(b"not even json\n").await.unwrap();
            // Third line: a real envelope. Connection must still be
            // open and dispatch must fire.
            let envelope = IpcEnvelope::notification(IpcCommand::Heartbeat {
                session_id: SessionId::new("sess_xyz"),
            });
            let mut line = serde_json::to_string(&envelope).unwrap();
            line.push('\n');
            stream.write_all(line.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();

            for _ in 0..50 {
                if !dispatcher.calls().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(
                dispatcher.calls(),
                vec![RecordedCall::Heartbeat("sess_xyz".into())],
                "malformed lines must be skipped without dispatch"
            );

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }

        #[tokio::test(flavor = "current_thread")]
        async fn ndjson_unknown_command_treated_as_malformed() {
            // The proto layer pins the deserialise failure on unknown
            // commands. The dispatch layer must surface that the same
            // way it surfaces any other malformed line: warning +
            // skip, no panic, no dispatch.
            let (_tmp, path) = fresh_socket_path();
            let dispatcher = Arc::new(RecordingDispatcher::default());
            let (shutdown, handle) =
                spawn_listener_with_dispatcher(&path, Arc::clone(&dispatcher)).await;

            let mut stream = UnixStream::connect(&path).await.expect("connect");
            stream
                .write_all(b"{\"command\":\"future-unknown\",\"session_id\":\"x\"}\n")
                .await
                .unwrap();
            stream.shutdown().await.unwrap();

            // Give the handler a moment, then assert nothing
            // dispatched and the listener is still alive.
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert!(
                dispatcher.calls().is_empty(),
                "unknown command must not dispatch; got {:?}",
                dispatcher.calls()
            );

            // Connect a second time and send a real envelope to prove
            // the listener is still serving.
            let mut second = UnixStream::connect(&path).await.expect("second connect");
            let envelope = IpcEnvelope::notification(IpcCommand::ListSessions);
            let mut line = serde_json::to_string(&envelope).unwrap();
            line.push('\n');
            second.write_all(line.as_bytes()).await.unwrap();
            second.shutdown().await.unwrap();

            for _ in 0..50 {
                if !dispatcher.calls().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(dispatcher.calls(), vec![RecordedCall::List]);

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }

        /// A line of valid bytes terminated by `\n` but containing an
        /// invalid UTF-8 sequence is logged and skipped; the same
        /// connection can still send a valid envelope right after.
        /// Without this contract a single bad frame on a long-lived
        /// client stream would force a reconnect.
        #[tokio::test(flavor = "current_thread")]
        async fn ndjson_invalid_utf8_line_is_skipped_connection_continues() {
            let (_tmp, path) = fresh_socket_path();
            let dispatcher = Arc::new(RecordingDispatcher::default());
            let (shutdown, handle) =
                spawn_listener_with_dispatcher(&path, Arc::clone(&dispatcher)).await;

            let mut stream = UnixStream::connect(&path).await.expect("connect");
            // First line: a stray invalid UTF-8 byte (0xFF) followed
            // by a newline. Framed correctly, just not valid UTF-8.
            stream.write_all(b"\xFF\n").await.unwrap();
            // Second line: a real envelope on the SAME connection.
            // The contract says the bad UTF-8 must not have torn the
            // socket down.
            let envelope = IpcEnvelope::notification(IpcCommand::Heartbeat {
                session_id: SessionId::new("sess_after_bad_utf8"),
            });
            let mut line = serde_json::to_string(&envelope).unwrap();
            line.push('\n');
            stream.write_all(line.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();

            for _ in 0..50 {
                if !dispatcher.calls().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(
                dispatcher.calls(),
                vec![RecordedCall::Heartbeat("sess_after_bad_utf8".into())],
                "invalid-UTF-8 line must be skipped, not close the connection",
            );

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }

        #[tokio::test(flavor = "current_thread")]
        async fn ndjson_oversized_line_closes_connection_but_listener_continues() {
            let (_tmp, path) = fresh_socket_path();
            let dispatcher = Arc::new(RecordingDispatcher::default());
            let (shutdown, handle) =
                spawn_listener_with_dispatcher(&path, Arc::clone(&dispatcher)).await;

            // First connection: send a line larger than the transport cap. The
            // connection should be torn down with OversizedLine.
            let mut stream = UnixStream::connect(&path).await.expect("connect");
            // Cap + 1 byte of payload, no newline yet.
            let blob = vec![b'x'; MAX_LINE_BYTES + 1];
            // The peer closing on its side is fine — what we care
            // about is that the second connection still works.
            let _ = stream.write_all(&blob).await;
            let _ = stream.shutdown().await;
            drop(stream);

            // Second connection: a valid envelope dispatches.
            let mut second = UnixStream::connect(&path).await.expect("second connect");
            let envelope = IpcEnvelope::notification(IpcCommand::Heartbeat {
                session_id: SessionId::new("sess_after_oversize"),
            });
            let mut line = serde_json::to_string(&envelope).unwrap();
            line.push('\n');
            second.write_all(line.as_bytes()).await.unwrap();
            second.shutdown().await.unwrap();

            for _ in 0..100 {
                if !dispatcher.calls().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            assert_eq!(
                dispatcher.calls(),
                vec![RecordedCall::Heartbeat("sess_after_oversize".into())],
                "listener must continue serving other connections after an oversized line",
            );

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }

        #[tokio::test(flavor = "current_thread")]
        async fn shutdown_drains_inflight_handlers_within_deadline() {
            let (_tmp, path) = fresh_socket_path();
            let dispatcher = Arc::new(RecordingDispatcher::default());
            let (shutdown, handle) =
                spawn_listener_with_dispatcher(&path, Arc::clone(&dispatcher)).await;

            // Open a connection but don't send anything — the
            // handler is mid-stream waiting for a line. Shutdown
            // must still complete within the drain deadline.
            let stream = UnixStream::connect(&path).await.expect("connect");

            let started = std::time::Instant::now();
            shutdown.trigger();
            tokio::time::timeout(Duration::from_millis(750), handle)
                .await
                .expect("listener did not stop within 750 ms")
                .expect("join")
                .expect("serve");
            let elapsed = started.elapsed();
            assert!(
                elapsed < Duration::from_millis(750),
                "drain took too long: {elapsed:?}"
            );

            drop(stream);
        }

        // ----- INTD-016 DoS budget tests --------------------------

        async fn spawn_listener_with_limits(
            path: &Path,
            limits: crate::dos::IpcLimits,
        ) -> (
            crate::Shutdown,
            tokio::task::JoinHandle<Result<(), IpcError>>,
        ) {
            let listener = IpcListener::bind(path, NoopDispatcher)
                .expect("bind")
                .with_limits(limits);
            let (shutdown, token) = crate::Shutdown::new();
            let handle = tokio::spawn(async move { listener.serve(token).await });
            tokio::task::yield_now().await;
            (shutdown, handle)
        }

        /// INTD-016 (a) slow-loris handshake: a peer that connects
        /// but never sends a line gets dropped at the handshake
        /// timeout. We use a very short handshake to keep the test
        /// fast; the production default is 5 s.
        #[tokio::test(flavor = "current_thread")]
        async fn slow_loris_handshake_times_out() {
            let (_tmp, path) = fresh_socket_path();
            let limits = crate::dos::IpcLimits {
                handshake_timeout: Duration::from_millis(50),
                ..crate::dos::IpcLimits::default()
            };
            let (shutdown, handle) = spawn_listener_with_limits(&path, limits).await;

            // Connect but never send anything — the handshake
            // timeout must elapse and the listener must continue
            // serving (a second connection still works).
            let stream = UnixStream::connect(&path).await.expect("connect");
            tokio::time::sleep(Duration::from_millis(150)).await;
            // Reading from the dropped peer side should give EOF
            // because the daemon closed the connection on timeout.
            drop(stream);

            // The listener is still up — a second connection works.
            let mut second = UnixStream::connect(&path).await.expect("second connect");
            let envelope = IpcEnvelope::notification(IpcCommand::Heartbeat {
                session_id: SessionId::new("sess_after_slow_loris"),
            });
            let mut line = serde_json::to_string(&envelope).unwrap();
            line.push('\n');
            second.write_all(line.as_bytes()).await.unwrap();
            second.shutdown().await.unwrap();

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .expect("listener stop")
                .expect("join")
                .expect("serve");
        }

        /// INTD-016 (b): RPS bucket exhaustion returns a structured
        /// JSON-RPC error without terminating the connection. This
        /// is the load-bearing INTD-016 hard rule: killing the
        /// connection on rate-limit exhaustion would let innocent
        /// retries escalate.
        #[tokio::test(flavor = "current_thread")]
        async fn rps_exhaustion_returns_error_without_closing() {
            let (_tmp, path) = fresh_socket_path();
            // Burst of 2 with a slow refill; once the third request
            // hits, the bucket must be empty and the daemon must
            // return -32005 without dropping the connection.
            let limits = crate::dos::IpcLimits {
                rps_burst: 2,
                rps_sustained: 0.0,
                ..crate::dos::IpcLimits::default()
            };
            let (shutdown, handle) = spawn_listener_with_limits(&path, limits).await;

            let stream = UnixStream::connect(&path).await.expect("connect");
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = tokio::io::BufReader::new(read_half);

            for i in 0..3 {
                let frame = json!({
                    "jsonrpc": "2.0",
                    "method": "session.list",
                    "id": format!("req-{i}"),
                });
                let mut line = serde_json::to_string(&frame).unwrap();
                line.push('\n');
                write_half.write_all(line.as_bytes()).await.unwrap();
            }

            // First two responses should be successful. Third
            // response must be a -32005 rate-limit error, NOT a
            // closed connection.
            let mut last_response = String::new();
            for _ in 0..3 {
                last_response.clear();
                tokio::time::timeout(
                    Duration::from_secs(2),
                    AsyncBufReadExt::read_line(&mut reader, &mut last_response),
                )
                .await
                .expect("read response timeout")
                .expect("read response");
            }
            let response: Value = serde_json::from_str(last_response.trim_end()).unwrap();
            assert_eq!(
                response["error"]["code"], -32005,
                "third request must hit rate limit (-32005), got {response}",
            );
            // Connection must still be open: send a fourth frame and
            // confirm we either get another rate-limit error or it
            // simply does not error out at the transport level.
            let frame = json!({
                "jsonrpc": "2.0",
                "method": "session.list",
                "id": "req-after-rate-limit",
            });
            let mut line = serde_json::to_string(&frame).unwrap();
            line.push('\n');
            // Writing must succeed — connection is still open.
            write_half
                .write_all(line.as_bytes())
                .await
                .expect("connection must remain open after rate limit");

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(2), handle)
                .await
                .expect("listener stop")
                .expect("join")
                .expect("serve");
        }

        /// INTD-016 (c): a frame larger than the control-lane cap
        /// is rejected BEFORE the JSON parser runs. We pick a
        /// generous control-frame cap (4 KiB) for the test, then
        /// send an 8 KiB frame whose method is `session.list`
        /// (i.e. NOT `scan_buffer`). The response must be -32600
        /// and the connection must continue.
        #[tokio::test(flavor = "current_thread")]
        async fn oversized_control_frame_rejected_before_parse() {
            let (_tmp, path) = fresh_socket_path();
            let limits = crate::dos::IpcLimits {
                control_frame_max_bytes: 4 * 1024,
                ..crate::dos::IpcLimits::default()
            };
            let (shutdown, handle) = spawn_listener_with_limits(&path, limits).await;

            let stream = UnixStream::connect(&path).await.expect("connect");
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = tokio::io::BufReader::new(read_half);

            // Build a control-lane frame just over 4 KiB. It is
            // still smaller than the legacy 1 MiB cap so it
            // reaches the INTD-016 control cap, not the older
            // scan_buffer fast path. Method is `session.list` so
            // `is_scan_buffer_frame` returns false.
            let padding = "x".repeat(5000);
            let frame = format!(
                "{{\"jsonrpc\":\"2.0\",\"method\":\"session.list\",\"id\":\"big\",\"_pad\":\"{padding}\"}}\n",
            );
            assert!(frame.len() > 4 * 1024);
            assert!(frame.len() < LEGACY_MAX_LINE_BYTES);
            write_half.write_all(frame.as_bytes()).await.unwrap();

            let mut response_line = String::new();
            tokio::time::timeout(
                Duration::from_secs(2),
                AsyncBufReadExt::read_line(&mut reader, &mut response_line),
            )
            .await
            .expect("response timeout")
            .expect("read response");
            let response: Value = serde_json::from_str(response_line.trim_end()).unwrap();
            assert_eq!(
                response["error"]["code"], -32600,
                "oversized control frame must be rejected with -32600, got {response}",
            );
            assert!(
                response["error"]["data"]["reason"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("control-lane frame exceeds"),
                "rejection reason must mention the cap: {response}",
            );

            // Send a small frame next — connection must still work.
            let small = json!({
                "jsonrpc": "2.0",
                "method": "session.list",
                "id": "small",
            });
            let mut small_line = serde_json::to_string(&small).unwrap();
            small_line.push('\n');
            write_half.write_all(small_line.as_bytes()).await.unwrap();
            response_line.clear();
            tokio::time::timeout(
                Duration::from_secs(2),
                AsyncBufReadExt::read_line(&mut reader, &mut response_line),
            )
            .await
            .expect("second response timeout")
            .expect("read second response");
            let response: Value = serde_json::from_str(response_line.trim_end()).unwrap();
            assert!(
                response["result"].is_object() || response["result"].is_array(),
                "small frame must dispatch normally after a rejected oversized frame: {response}",
            );

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(2), handle)
                .await
                .expect("listener stop")
                .expect("join")
                .expect("serve");
        }
    }

    // ----- MLP2-006: gate_evaluated emission via the IPC handler -----

    /// MLP2-006: a finding-bearing mid-edit `scan_buffer` JSON-RPC
    /// dispatch must produce exactly one `gate_evaluated` row on the
    /// configured Kindling sink, carrying the daemon's session id,
    /// the traceparent-derived `gate_eval_id`, the request file
    /// path, and a finite `duration_ms`.
    #[tokio::test]
    async fn handle_jsonrpc_value_emits_gate_evaluated_for_finding_bearing_scan() {
        use crate::kindling_observation::MidEditObservationEmitter;

        let pipeline = EnforcementPipeline::default();
        let (emitter, sink) =
            MidEditObservationEmitter::with_recorder("11111111-1111-4111-8111-111111111111");
        let scan_buffer =
            ScanBufferService::new(pipeline).with_observation_emitter(Arc::new(emitter));

        let dispatcher = Arc::new(NoopDispatcher);
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": "scan_buffer",
                "params": {
                    "path": "src/auth/client.ts",
                    "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
                    "version": 9,
                    "mode": "midEdit"
                },
                "id": "kindling-emit",
                "traceparent": traceparent,
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("scan_buffer response");

        assert_eq!(response["id"], "kindling-emit");
        assert!(
            response["result"]["diagnostics"]
                .as_array()
                .is_some_and(|d| !d.is_empty()),
            "fixture should produce at least one finding: {response}"
        );

        let recorded = sink.recorded();
        assert_eq!(
            recorded.len(),
            1,
            "exactly one gate_evaluated row per finding-bearing scan",
        );
        let row = &recorded[0];
        assert_eq!(row.session_id, "11111111-1111-4111-8111-111111111111");
        assert_eq!(
            row.gate_eval_id, "b7ad6b7169203331",
            "gate_eval_id must derive from traceparent parent-id (MLP2-008 join key)",
        );
        assert_eq!(row.gate_id, "midEdit");
        assert_eq!(row.kind, "gate_evaluated");
        assert_eq!(
            row.inputs.changed_files,
            vec!["src/auth/client.ts".to_string()]
        );
        assert_eq!(row.inputs.file_count, 1);
    }

    /// MLP2-006: a no-finding mid-edit scan stays silent — the volume-
    /// control contract from MLP-016 propagates through the emitter.
    #[tokio::test]
    async fn handle_jsonrpc_value_stays_silent_when_scan_has_no_findings() {
        use crate::kindling_observation::MidEditObservationEmitter;
        use anvil_intercept_rules::RuleRegistry;

        let pipeline = EnforcementPipeline::new(RuleRegistry::new());
        let (emitter, recorder) =
            MidEditObservationEmitter::with_recorder("11111111-1111-4111-8111-111111111111");
        let scan_buffer =
            ScanBufferService::new(pipeline).with_observation_emitter(Arc::new(emitter));

        let dispatcher = Arc::new(NoopDispatcher);
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": "scan_buffer",
                "params": {
                    "path": "src/innocent.ts",
                    "text": "export const greet = () => 'hi';\n",
                    "version": 1,
                    "mode": "midEdit"
                },
                "id": "kindling-quiet",
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("scan response");

        assert_eq!(response["result"]["diagnostics"], json!([]));
        assert!(
            recorder.is_empty(),
            "no-finding scans must NOT produce a Kindling row",
        );
    }

    /// MLP2-006: pre-write scans bypass the mid-edit Kindling
    /// fan-out (separate budget class per ADR-031). Even with a
    /// finding present, the emitter must stay silent.
    #[tokio::test]
    async fn handle_jsonrpc_value_does_not_emit_for_pre_write_mode() {
        use crate::kindling_observation::MidEditObservationEmitter;

        let (emitter, recorder) =
            MidEditObservationEmitter::with_recorder("11111111-1111-4111-8111-111111111111");
        let scan_buffer = ScanBufferService::new(EnforcementPipeline::default())
            .with_observation_emitter(Arc::new(emitter));

        let dispatcher = Arc::new(NoopDispatcher);
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": "scan_buffer",
                "params": {
                    "path": "src/auth/client.ts",
                    "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
                    "version": 9,
                    "mode": "preWrite"
                },
                "id": "kindling-pre-write",
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("scan response");

        assert!(response["result"]["diagnostics"].is_array());
        assert!(
            recorder.is_empty(),
            "pre-write scans must NOT contribute to the mid-edit Kindling rollup",
        );
    }

    /// MLP2-006: a service without a wired emitter behaves exactly
    /// like the legacy daemon — no emission, no panic, scan
    /// response unchanged.
    #[tokio::test]
    async fn handle_jsonrpc_value_skips_emission_when_no_emitter_wired() {
        let scan_buffer = ScanBufferService::default();
        assert!(
            scan_buffer.observation_emitter().is_none(),
            "default service must not have a Kindling emitter (legacy compat)",
        );

        let dispatcher = Arc::new(NoopDispatcher);
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);

        let response = handle_jsonrpc_value(
            json!({
                "jsonrpc": "2.0",
                "method": "scan_buffer",
                "params": {
                    "path": "src/x.ts",
                    "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
                    "version": 1,
                    "mode": "midEdit"
                },
                "id": "kindling-none",
            }),
            &dispatcher,
            &scan_buffer,
            &status,
            None,
            None,
            None,
        )
        .await
        .expect("scan response");

        assert_eq!(response["id"], "kindling-none");
        // No way to assert "didn't emit" without a recorder; the
        // proof is that the response succeeds (no panic, no error).
        assert!(response["result"]["diagnostics"].is_array());
    }

    /// MLP2-006: `gate_eval_id` derivation must use the W3C
    /// traceparent's parent-id when present, falling back to a
    /// fresh UUID v4 when the producer omitted the header (so the
    /// row never carries a placeholder id).
    #[test]
    fn derive_gate_eval_id_prefers_traceparent_parent_id() {
        let with_tp = derive_gate_eval_id(Some(
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
        ));
        assert_eq!(with_tp, "b7ad6b7169203331");

        let without = derive_gate_eval_id(None);
        // UUID v4 is 36 chars including hyphens.
        assert_eq!(without.len(), 36);
        assert_eq!(without.chars().filter(|&c| c == '-').count(), 4);

        // Malformed traceparent → fallback path, never the broken header.
        let malformed = derive_gate_eval_id(Some("not-a-traceparent"));
        assert_eq!(malformed.len(), 36);
        assert_ne!(malformed, "not-a-traceparent");
    }

    /// MLP2-006: rate-window throttling at the IPC layer — a burst
    /// past the configured cap must not panic and must not impact
    /// scan responses. Uses a tiny cap so the test exercises both
    /// admit and throttle outcomes via the public emitter surface.
    #[tokio::test]
    async fn ipc_emission_throttling_does_not_perturb_scan_responses() {
        use crate::kindling_observation::{
            KindlingObservationSink, MidEditObservationEmitter, RecordingKindlingObservationSink,
        };
        use crate::rate_window::RateWindow;

        let recorder = Arc::new(RecordingKindlingObservationSink::new());
        let emitter = MidEditObservationEmitter::new(
            Arc::clone(&recorder) as Arc<dyn KindlingObservationSink>,
            // cap = 2 in a long window so the third call throttles.
            RateWindow::new(2, Duration::from_mins(1)),
            "11111111-1111-4111-8111-111111111111".into(),
        );
        let scan_buffer = ScanBufferService::new(EnforcementPipeline::default())
            .with_observation_emitter(Arc::new(emitter));

        let dispatcher = Arc::new(NoopDispatcher);
        let status: Arc<dyn StatusProvider> = Arc::new(NoopStatusProvider);
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": "src/auth/client.ts",
                "text": "const config = { api_key: 'abcdEFGH1234567890' };\n",
                "version": 1,
                "mode": "midEdit"
            },
            "id": "kindling-burst",
        });

        for _ in 0..5 {
            let response = handle_jsonrpc_value(
                frame.clone(),
                &dispatcher,
                &scan_buffer,
                &status,
                None,
                None,
                None,
            )
            .await
            .expect("scan response");
            assert!(
                response["result"]["diagnostics"]
                    .as_array()
                    .is_some_and(|d| !d.is_empty()),
                "scan response must succeed regardless of throttle state",
            );
        }
        assert_eq!(
            recorder.len(),
            2,
            "rate window must cap recorded emissions at the configured capacity",
        );
    }

    /// N5 / CIB-091b follow-up: the `SaveTimeError::Io` arm must return a STATIC
    /// wire reason — never the raw `io::Error` Display, which leaks an OS string
    /// that can confirm the existence/accessibility of a probed absolute path.
    #[test]
    fn save_time_io_error_returns_static_wire_reason_not_os_string() {
        // An io::Error whose Display embeds a probed absolute path — the leak the
        // old `err.to_string()` would have surfaced on the wire.
        let secret_path = "/secret/.env.production";
        let err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No such file or directory: {secret_path}"),
        );

        let response = save_time_result::<()>(Err(SaveTimeError::Io(err)), Some(json!(7)), None)
            .expect("a non-notification Io error yields a response");

        let data = &response["error"]["data"];
        assert_eq!(
            data["error"], "workspace-io-error",
            "the Io arm must return a static reason, got: {data:?}",
        );
        // The whole serialised envelope must not echo the probed path or the raw
        // OS message anywhere.
        let serialised = response.to_string();
        assert!(
            !serialised.contains(secret_path),
            "the wire must not echo a probed absolute path: {serialised}",
        );
        assert!(
            !serialised.contains("No such file or directory"),
            "the wire must not echo the raw OS error string: {serialised}",
        );
        // The error code is preserved.
        assert_eq!(response["error"]["code"], -32603);
    }
}
