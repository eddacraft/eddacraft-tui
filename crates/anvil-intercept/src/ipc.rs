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
//!   so the 1 MiB per-line cap is enforced byte-by-byte before UTF-8
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

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anvil_intercept_proto::{IpcCommand, IpcEnvelope};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::task::JoinSet;

use crate::ShutdownToken;
use crate::registry::SessionDispatcher;

/// Maximum size of a single NDJSON line, in bytes. Lines larger than
/// this cause the connection to be torn down with [`IpcError::OversizedLine`]
/// — protects the daemon from a same-UID peer streaming an unbounded
/// blob into one line.
pub const MAX_LINE_BYTES: usize = 1 << 20; // 1 MiB

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

impl SessionDispatcher for NoopDispatcher {
    fn register(
        &self,
        _id: &anvil_intercept_proto::SessionId,
        _worktree: &Path,
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

/// Resolve the Windows named-pipe path used by the daemon.
///
/// Format: `\\.\pipe\anvil-intercept-<current-user-sid>`. The launcher
/// (`DriverClient` in DRVR-001) MUST resolve the path with the same
/// algorithm — the helper here is `pub` so DRVR-001 can re-export
/// rather than re-implement. The suffix is the token SID, not an env
/// username, so account-name spoofing and local/domain username
/// collisions do not change the rendezvous point.
#[cfg(windows)]
pub fn resolve_pipe_name() -> Result<String, IpcError> {
    let sid = anvil_intercept_win32::current_user_sid()?;
    Ok(format!(r"\\.\pipe\anvil-intercept-{sid}"))
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
    #[cfg(windows)]
    inner: tokio::net::windows::named_pipe::NamedPipeServer,
    #[cfg(windows)]
    pipe_name: String,
    #[cfg(windows)]
    dispatcher: Arc<D>,
    #[cfg(not(any(unix, windows)))]
    _marker: std::marker::PhantomData<D>,
}

impl<D: SessionDispatcher> IpcListener<D> {
    /// Bind a fresh listener at the platform-default path with the
    /// supplied dispatcher. Performs the full directory and socket
    /// permission ladder.
    #[cfg(unix)]
    pub fn bind_default(dispatcher: D) -> Result<Self, IpcError> {
        let socket_path = resolve_socket_path()?;
        Self::bind(&socket_path, dispatcher)
    }

    /// Bind a fresh listener at `path`. The path's parent directory
    /// is checked / created with the strict permission ladder. The
    /// socket file itself is `fchmod`-ed to 0600 before connections
    /// are accepted.
    #[cfg(unix)]
    pub fn bind(path: &Path, dispatcher: D) -> Result<Self, IpcError> {
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
                            let dispatcher = Arc::clone(&dispatcher);
                            let conn_token = token.clone();
                            joinset.spawn(async move {
                                if let Err(err) = handle_connection(stream, dispatcher, conn_token).await {
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
        let pipe_name = resolve_pipe_name()?;
        Self::bind(&pipe_name, dispatcher)
    }

    /// Bind a Windows named pipe using an owner-only DACL and local-only clients.
    pub fn bind(pipe_name: &str, dispatcher: D) -> Result<Self, IpcError> {
        let server = anvil_intercept_win32::create_owner_only_pipe_server(
            pipe_name,
            anvil_intercept_win32::PipeInstance::First,
        )?;
        Ok(Self {
            inner: server,
            pipe_name: pipe_name.to_owned(),
            dispatcher: Arc::new(dispatcher),
        })
    }

    /// Accept named-pipe clients until `token` fires, spawning one handler per client.
    pub async fn serve(self, mut token: ShutdownToken) -> Result<(), IpcError> {
        let mut server = self.inner;
        let pipe_name = self.pipe_name;
        let dispatcher = self.dispatcher;
        let mut joinset: JoinSet<()> = JoinSet::new();

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
                            let dispatcher = Arc::clone(&dispatcher);
                            let conn_token = token.clone();
                            joinset.spawn(async move {
                                if let Err(err) = handle_connection(connected_server, dispatcher, conn_token).await {
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

async fn handle_connection<D: SessionDispatcher, R: AsyncRead + AsyncWrite + Unpin>(
    stream: R,
    dispatcher: Arc<D>,
    mut token: ShutdownToken,
) -> Result<(), IpcError> {
    let mut reader = BufReader::new(stream);
    let mut buf = String::new();

    loop {
        buf.clear();
        let read = tokio::select! {
            biased;
            () = token.cancelled() => return Ok(()),
            res = read_one_line(&mut reader, &mut buf) => match res {
                Ok(n) => n,
                Err(IpcError::InvalidUtf8 { len }) => {
                    // Per the module doc: malformed-frame errors are
                    // logged and skipped, the connection stays open.
                    // Invalid UTF-8 is a malformed frame too — not a
                    // reason to disconnect a long-lived client mid
                    // stream.
                    tracing::warn!(
                        target: "anvil_intercept::ipc",
                        bytes = len,
                        "skipping NDJSON line: invalid UTF-8",
                    );
                    eprintln!(
                        "anvil-intercept: skipping NDJSON line ({len} bytes): invalid UTF-8",
                    );
                    continue;
                }
                Err(err) => return Err(err),
            },
        };
        if read == 0 {
            // Peer closed cleanly.
            return Ok(());
        }
        // `read_line` keeps the trailing `\n`; trim it before parsing.
        let line = buf.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(value) => {
                if is_jsonrpc_frame(&value) {
                    if let Some(response) = handle_jsonrpc_value(value, &dispatcher) {
                        let mut response = serde_json::to_string(&response).map_err(|err| {
                            io::Error::other(format!("serialise JSON-RPC response: {err}"))
                        })?;
                        response.push('\n');
                        reader.get_mut().write_all(response.as_bytes()).await?;
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
                    -32700,
                    "Parse error",
                    json!({"reason": err.to_string()}),
                );
                let mut response = serde_json::to_string(&response).map_err(|err| {
                    io::Error::other(format!("serialise JSON-RPC parse error: {err}"))
                })?;
                response.push('\n');
                reader.get_mut().write_all(response.as_bytes()).await?;
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

#[doc(hidden)]
#[cfg(feature = "bench-internals")]
pub fn handle_jsonrpc_value_for_benchmark<D: SessionDispatcher>(
    value: Value,
    dispatcher: &Arc<D>,
) -> Option<Value> {
    handle_jsonrpc_value(value, dispatcher)
}

fn is_jsonrpc_frame(value: &Value) -> bool {
    match value {
        Value::Array(_) => true,
        Value::Object(map) => map.contains_key("jsonrpc") || map.contains_key("method"),
        _ => false,
    }
}

fn handle_jsonrpc_value<D: SessionDispatcher>(value: Value, dispatcher: &Arc<D>) -> Option<Value> {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                return Some(jsonrpc_error(
                    None,
                    -32600,
                    "Invalid Request",
                    json!({
                        "reason": "batch must not be empty"
                    }),
                ));
            }
            let responses: Vec<Value> = items
                .into_iter()
                .filter_map(|item| handle_jsonrpc_request(item, dispatcher))
                .collect();
            if responses.is_empty() {
                None
            } else {
                Some(Value::Array(responses))
            }
        }
        item => handle_jsonrpc_request(item, dispatcher),
    }
}

fn handle_jsonrpc_request<D: SessionDispatcher>(
    value: Value,
    dispatcher: &Arc<D>,
) -> Option<Value> {
    let Value::Object(map) = value else {
        return Some(jsonrpc_error(
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
            -32600,
            "Invalid Request",
            json!({
                "reason": "id must be a string, number, or null"
            }),
        ));
    }

    if map.get("jsonrpc") != Some(&Value::String("2.0".to_owned())) {
        return Some(jsonrpc_error(
            response_id,
            -32600,
            "Invalid Request",
            json!({"reason": "jsonrpc must be \"2.0\""}),
        ));
    }

    let Some(method) = map.get("method").and_then(Value::as_str) else {
        return Some(jsonrpc_error(
            response_id,
            -32600,
            "Invalid Request",
            json!({"reason": "method must be a string"}),
        ));
    };
    let is_notification = !has_id;
    let params = map.get("params").unwrap_or(&Value::Null);

    let command = match command_from_jsonrpc(method, params) {
        Ok(command) => command,
        Err(JsonRpcFailure {
            code,
            message,
            data,
        }) => {
            return jsonrpc_request_error(response_id, is_notification, code, message, data);
        }
    };

    match dispatch_command(&command, dispatcher) {
        Ok(result) => {
            if is_notification {
                None
            } else {
                Some(json!({"jsonrpc": "2.0", "result": result, "id": response_id}))
            }
        }
        Err(err) => jsonrpc_request_error(
            response_id,
            is_notification,
            -32603,
            "Internal error",
            json!({"error": err.clone()}),
        ),
    }
}

fn valid_jsonrpc_id(id: Option<&Value>) -> Option<Value> {
    if let Some(value @ (Value::Null | Value::String(_) | Value::Number(_))) = id {
        Some(value.clone())
    } else {
        None
    }
}

fn jsonrpc_request_error(
    id: Option<Value>,
    is_notification: bool,
    code: i64,
    message: &'static str,
    data: impl serde::Serialize,
) -> Option<Value> {
    if is_notification {
        None
    } else {
        Some(jsonrpc_error(id, code, message, data))
    }
}

fn jsonrpc_error(
    id: Option<Value>,
    code: i64,
    message: &'static str,
    data: impl serde::Serialize,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message,
            "data": data,
        },
        "id": id.unwrap_or(Value::Null),
    })
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
                Ok(IpcCommand::RegisterSession {
                    session_id,
                    worktree: PathBuf::from(worktree.as_str()),
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

fn invalid_params(method: &str, reason: impl Into<String>) -> JsonRpcFailure {
    JsonRpcFailure {
        code: -32602,
        message: "Invalid params",
        data: json!({"method": method, "reason": reason.into()}),
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
        } => {
            dispatcher
                .register(session_id, worktree)
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
    }
}

// --------------------------------------------------------------------
// Tests.
// --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use anvil_intercept_proto::{SessionId, SessionRecord};
    #[cfg(unix)]
    use std::sync::Mutex;
    #[cfg(unix)]
    use std::time::Duration;
    #[cfg(unix)]
    use tokio::io::AsyncWriteExt;
    #[cfg(unix)]
    use tokio::net::UnixStream;

    #[cfg(unix)]
    use crate::registry::RegistryError;

    // ----- Recording dispatcher used by behaviour tests. ------------

    #[derive(Debug, Default)]
    #[cfg(unix)]
    struct RecordingDispatcher {
        calls: Mutex<Vec<RecordedCall>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    #[cfg(unix)]
    enum RecordedCall {
        Register { id: String, worktree: PathBuf },
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
        fn register(&self, id: &SessionId, worktree: &Path) -> Result<(), RegistryError> {
            self.calls.lock().unwrap().push(RecordedCall::Register {
                id: id.as_str().to_owned(),
                worktree: worktree.to_path_buf(),
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

            // First connection: send a line larger than 1 MiB. The
            // connection should be torn down with OversizedLine.
            let mut stream = UnixStream::connect(&path).await.expect("connect");
            // 1 MiB + 1 byte of payload, no newline yet.
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
    }
}
