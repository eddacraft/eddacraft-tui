//! INTD-002: NDJSON IPC listener.
//!
//! The daemon listens on a Unix domain socket (Linux/macOS) or a named
//! pipe (Windows) and parses one JSON envelope per line. This module
//! owns:
//!
//! - **Path resolution** — `$XDG_RUNTIME_DIR/anvil` (else
//!   `$HOME/.local/state/anvil`) on Unix; `\\.\pipe\anvil-intercept-<user>`
//!   on Windows. The launcher (DRVR-001) reads the same algorithm.
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

use anvil_intercept_proto::{IpcCommand, IpcEnvelope};
use anvil_observability::{TraceContext, bind_traceparent_to_span};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::task::JoinSet;
use tracing::{Instrument, field};

use crate::ShutdownToken;
use crate::dos::{IpcLimits, RpsBucket};
use crate::enforcement::CONTENT_SIZE_CAP_BYTES_USIZE;
use crate::kindling_observation::MidEditEmissionRequest;
use crate::midedit::{self, ScanBufferMode, ScanBufferRequest, ScanBufferService};
use crate::registry::SessionDispatcher;
use crate::status::{DaemonStatus, StatusProvider};

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
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopStatusProvider;

impl StatusProvider for NoopStatusProvider {
    fn query_status(&self) -> DaemonStatus {
        crate::status::build_status(
            Vec::new(),
            &[],
            None,
            std::time::Instant::now(),
            std::time::Instant::now(),
            env!("CARGO_PKG_VERSION"),
            crate::status::IpcState::Serving,
            None,
            None,
        )
    }
}

impl SessionDispatcher for NoopDispatcher {
    fn register(
        &self,
        _id: &anvil_intercept_proto::SessionId,
        _worktree: &Path,
        _agent_tag: Option<&anvil_intercept_proto::session::AgentTag>,
    ) -> Result<(), crate::registry::RegistryError> {
        Ok(())
    }
    fn heartbeat(
        &self,
        _id: &anvil_intercept_proto::SessionId,
    ) -> Result<(), crate::registry::RegistryError> {
        Ok(())
    }
    fn unregister(
        &self,
        _id: &anvil_intercept_proto::SessionId,
    ) -> Result<bool, crate::registry::RegistryError> {
        Ok(false)
    }
    fn list(&self) -> Vec<anvil_intercept_proto::SessionRecord> {
        Vec::new()
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
        std::env::var_os("XDG_RUNTIME_DIR"),
        std::env::var_os("HOME"),
    )
}

#[cfg(unix)]
fn resolve_socket_dir_with_env(
    xdg_runtime_dir: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, IpcError> {
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
/// Format: `\\.\pipe\anvil-intercept-<current-user-sid>`. The launcher
/// (`DriverClient` in DRVR-001) MUST resolve the path with the same
/// algorithm — the helper here is `pub` so DRVR-001 can re-export
/// rather than re-implement. The suffix is the token SID, not an env
/// username, so account-name spoofing and local/domain username
/// collisions do not change the rendezvous point.
///
/// The actual derivation lives in
/// [`anvil_intercept_win32::pipe_name_for_current_user`] so the CLI
/// status client (which speaks synchronous Win32 IO and does not link
/// the daemon) can reuse the exact same string without depending on
/// `anvil-intercept`.
#[cfg(windows)]
pub fn resolve_pipe_name() -> Result<String, IpcError> {
    Ok(anvil_intercept_win32::pipe_name_for_current_user()?)
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
                            joinset.spawn(async move {
                                let _connection_permit = connection_permit;
                                if let Err(err) = handle_connection(stream, dispatcher, scan_buffer, conn_status, conn_token, limits).await {
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
                            joinset.spawn(async move {
                                let _connection_permit = connection_permit;
                                if let Err(err) = handle_connection(connected_server, dispatcher, scan_buffer, conn_status, conn_token, limits).await {
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
// Per-connection handler.
// --------------------------------------------------------------------

#[allow(clippy::too_many_lines)] // INTD-016 layered budgets share a single connection loop; splitting obscures the per-frame ordering of RPS / size / parse checks.
async fn handle_connection<D: SessionDispatcher, R: AsyncRead + AsyncWrite + Unpin>(
    stream: R,
    dispatcher: Arc<D>,
    scan_buffer: ScanBufferService,
    status_provider: Arc<dyn StatusProvider>,
    mut token: ShutdownToken,
    limits: IpcLimits,
) -> Result<(), IpcError> {
    let mut reader = BufReader::new(stream);
    let mut buf = String::new();

    // INTD-016: per-connection RPS bucket.
    let mut bucket = RpsBucket::from_limits(&limits, std::time::Instant::now());
    // INTD-016: handshake timeout — first line must arrive within
    // `limits.handshake_timeout` of accept. After the first line is
    // framed, subsequent reads use `limits.idle_timeout`.
    let mut first_frame_seen = false;

    loop {
        buf.clear();
        let read = match read_connection_line_with_deadline(
            &mut reader,
            &mut buf,
            &mut token,
            if first_frame_seen {
                limits.idle_timeout
            } else {
                limits.handshake_timeout
            },
        )
        .await?
        {
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
                    if let Some(response) =
                        handle_jsonrpc_value(value, &dispatcher, &scan_buffer, &status_provider)
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
    handle_jsonrpc_value(value, dispatcher, scan_buffer, &status).await
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
                if let Some(response) =
                    handle_jsonrpc_request(item, dispatcher, scan_buffer, status_provider).await
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
        item => handle_jsonrpc_request(item, dispatcher, scan_buffer, status_provider).await,
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

async fn handle_jsonrpc_request<D: SessionDispatcher>(
    value: Value,
    dispatcher: &Arc<D>,
    scan_buffer: &ScanBufferService,
    status_provider: &Arc<dyn StatusProvider>,
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

    dispatch_span.in_scope(|| {
        dispatch_session_jsonrpc(
            method,
            params,
            response_id,
            traceparent,
            is_notification,
            dispatcher,
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

fn dispatch_session_jsonrpc<D: SessionDispatcher>(
    method: &str,
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    dispatcher: &Arc<D>,
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

    match dispatch_command(&command, dispatcher) {
        Ok(result) => {
            if is_notification {
                None
            } else {
                Some(jsonrpc_success(response_id, traceparent, result))
            }
        }
        Err(err) => jsonrpc_request_error(
            response_id,
            traceparent,
            is_notification,
            -32603,
            "Internal error",
            json!({"error": err.clone()}),
        ),
    }
}

async fn handle_scan_buffer_jsonrpc(
    map: &serde_json::Map<String, Value>,
    method: &str,
    params: &Value,
    response_id: Option<Value>,
    traceparent: Option<&str>,
    is_notification: bool,
    scan_buffer: &ScanBufferService,
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
    match scan_buffer_from_jsonrpc(params, method, traceparent, scan_buffer).await {
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
                Ok(IpcCommand::RegisterSession {
                    session_id,
                    worktree: PathBuf::from(worktree.as_str()),
                    agent_tag,
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
        if !matches!(key.as_str(), "path" | "text" | "version" | "mode") {
            return Err(invalid_params(
                method,
                "scan_buffer params only allow path, text, version, and mode fields",
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

async fn scan_buffer_from_jsonrpc(
    params: &Value,
    method: &str,
    traceparent: Option<&str>,
    scan_buffer: &ScanBufferService,
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

        ScanBufferRequest {
            path: PathBuf::from(path),
            text,
            version,
            mode,
        }
    };
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
/// the producer omitted `traceparent` so the row is never emitted
/// with a placeholder id.
fn derive_gate_eval_id(traceparent: Option<&str>) -> String {
    if let Some(raw) = traceparent
        && let Ok(ctx) = TraceContext::parse(raw)
    {
        return ctx.parent_id().to_string();
    }
    uuid::Uuid::new_v4().to_string()
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
    let result = dispatch_command(&envelope.command, dispatcher).map(|_| ());
    if let Err(err) = result {
        tracing::warn!(target: "anvil_intercept::ipc", error = %err, "dispatcher returned error");
        eprintln!("anvil-intercept: dispatcher returned error: {err}");
    }
}

fn dispatch_command<D: SessionDispatcher>(
    command: &IpcCommand,
    dispatcher: &Arc<D>,
) -> Result<Value, String> {
    match command {
        IpcCommand::RegisterSession {
            session_id,
            worktree,
            agent_tag,
        } => {
            dispatcher
                .register(session_id, worktree, agent_tag.as_ref())
                .map_err(|err| err.to_string())?;
            Ok(json!({"ok": true}))
        }
        IpcCommand::Heartbeat { session_id } => {
            dispatcher
                .heartbeat(session_id)
                .map_err(|err| err.to_string())?;
            Ok(json!({"ok": true}))
        }
        IpcCommand::UnregisterSession { session_id } => Ok(json!({
            "removed": dispatcher
                .unregister(session_id)
                .map_err(|err| err.to_string())?,
        })),
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
    }
}

// --------------------------------------------------------------------
// Tests.
// --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enforcement::EnforcementPipeline;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

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

    #[test]
    fn trace_method_label_truncates_on_char_boundary() {
        let method = "é".repeat(MAX_TRACE_METHOD_LEN);
        let label = trace_method_label(&method);

        assert!(label.len() <= MAX_TRACE_METHOD_LEN);
        assert!(label.ends_with("..."));
        assert!(std::str::from_utf8(label.as_bytes()).is_ok());
    }

    #[tokio::test]
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
        },
        Heartbeat(String),
        Unregister(String),
        List,
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
        ) -> Result<(), RegistryError> {
            self.calls.lock().unwrap().push(RecordedCall::Register {
                id: id.as_str().to_owned(),
                worktree: worktree.to_path_buf(),
                agent_tag: agent_tag.cloned(),
            });
            Ok(())
        }
        fn heartbeat(&self, id: &SessionId) -> Result<(), RegistryError> {
            self.calls
                .lock()
                .unwrap()
                .push(RecordedCall::Heartbeat(id.as_str().to_owned()));
            Ok(())
        }
        fn unregister(&self, id: &SessionId) -> Result<bool, RegistryError> {
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
            Some("/run/user/1000".into()),
            Some("/home/somebody".into()),
        )
        .expect("resolve");
        assert_eq!(dir, PathBuf::from("/run/user/1000/anvil"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_falls_back_to_home_state() {
        let dir =
            resolve_socket_dir_with_env(None, Some("/home/somebody".into())).expect("resolve");
        assert_eq!(dir, PathBuf::from("/home/somebody/.local/state/anvil"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_treats_empty_xdg_as_unset() {
        let dir = resolve_socket_dir_with_env(Some("".into()), Some("/home/somebody".into()))
            .expect("resolve");
        assert_eq!(dir, PathBuf::from("/home/somebody/.local/state/anvil"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_socket_dir_errors_when_no_candidate() {
        let err = resolve_socket_dir_with_env(None, None).unwrap_err();
        assert!(matches!(err, IpcError::NoSocketDirCandidate));
    }

    #[cfg(windows)]
    #[test]
    fn resolve_pipe_name_uses_user_suffix() {
        let name = resolve_pipe_name().expect("resolve");
        assert!(name.starts_with(r"\\.\pipe\anvil-intercept-"), "got {name}");
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
                }]
            );

            shutdown.trigger();
            tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .expect("listener did not return after shutdown")
                .expect("join")
                .expect("serve");
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
            let response = handle_jsonrpc_value(frame.clone(), &dispatcher, &scan_buffer, &status)
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
}
