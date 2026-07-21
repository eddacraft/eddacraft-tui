//! Windows-only Win32 helpers for the intercept daemon.
//!
//! `anvil-intercept` forbids unsafe code. This crate keeps the narrow
//! Win32 security-attribute boundary in one place and exposes a safe
//! named-pipe constructor for the daemon IPC listener.

#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]

// DSV-010 / ADR-068: the Windows save-time read-safety guard (the Unix
// `path_safety` analogue). Kept in its own module; the unsafe NtCreateFile FFI
// stays here so `anvil-intercept` keeps `#![forbid(unsafe_code)]`.
pub mod read_safety;

use std::ffi::{OsStr, c_void};
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::RawHandle;
use std::path::Path;
use std::ptr::{null_mut, slice_from_raw_parts};

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_BROKEN_PIPE, ERROR_INVALID_PARAMETER,
    ERROR_PIPE_NOT_CONNECTED, FILETIME, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
    SDDL_REVISION_1, SE_FILE_OBJECT,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_OWNER, TOKEN_QUERY, TOKEN_USER, TokenOwner,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CreateDirectoryW, CreateFileW, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
    OPEN_EXISTING, ReadFile, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT, WriteFile,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};
use windows_sys::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, GetProcessTimes, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE, TerminateProcess,
};

// `GENERIC_READ` / `GENERIC_WRITE` are not re-exported from any
// `windows-sys` 0.61 module the daemon already pulls in; pinning them
// inline keeps the Cargo feature-flag set narrow.
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;

// `GetExitCodeProcess` reports this sentinel (STATUS_PENDING, 259) for a
// process that has not exited. Pinned inline for the same reason as the
// GENERIC_* flags — it is not re-exported from a `windows-sys` 0.61 module
// the daemon already depends on.
const STILL_ACTIVE: u32 = 259;

// Minimal owner rights for duplex clients plus the server's replacement-instance
// flow. This deliberately avoids GENERIC_ALL; v1 treats same-user processes as
// inside the trust boundary, matching the Unix owner-only socket model.
const OWNER_PIPE_RIGHTS: &str = "0x12019f";

// DSV-010b config-trust (the Windows analogue of the Unix `read_trusted`
// owner-only check). Pinned inline — not re-exported from a `windows-sys` 0.61
// module the daemon already depends on. `OWNER_SECURITY_INFORMATION` selects the
// owner SID for `GetSecurityInfo`; `FILE_ATTRIBUTE_REPARSE_POINT` flags a
// symlink/junction (refused, like the Unix `O_NOFOLLOW` ELOOP).
const OWNER_SECURITY_INFORMATION: u32 = 0x0000_0001;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const ERROR_FILE_NOT_FOUND_CT: i32 = 2;
const ERROR_PATH_NOT_FOUND_CT: i32 = 3;
const ERROR_ALREADY_EXISTS_CT: i32 = 183;
// Generous ceiling for an operator config file (confinement / antipattern YAML);
// refuse rather than buffer an unbounded file behind a same-user trust boundary.
const MAX_TRUSTED_CONFIG_BYTES: u64 = 8 * 1024 * 1024;

/// Create a local-only named-pipe server with an explicit owner-only DACL.
pub fn create_owner_only_pipe_server(
    pipe_name: &str,
    instance: PipeInstance,
) -> io::Result<NamedPipeServer> {
    let mut security = OwnerOnlySecurityAttributes::new()?;
    let mut options = ServerOptions::new();
    options
        .first_pipe_instance(instance.is_first())
        .reject_remote_clients(true);

    // SAFETY: `security.as_mut_ptr()` points at a valid SECURITY_ATTRIBUTES
    // whose security descriptor remains alive until CreateNamedPipeW returns.
    unsafe { options.create_with_security_attributes_raw(pipe_name, security.as_mut_ptr()) }
}

/// Whether this named-pipe server is the singleton-claiming first instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeInstance {
    First,
    Additional,
}

impl PipeInstance {
    const fn is_first(self) -> bool {
        matches!(self, Self::First)
    }
}

/// Stable SID string for the current process token's user.
pub fn current_user_sid() -> io::Result<String> {
    current_user_sid_string()
}

/// Whether the process at the *client* end of a connected named-pipe **server**
/// handle runs under the same user SID as this (the daemon) process.
///
/// DSV-010b / ADR-070 step 4. The owner-only DACL on the pipe
/// ([`create_owner_only_pipe_server`]) already refuses a different-SID client at
/// the kernel — same-SID is the boundary, mirroring the Unix owner-only socket
/// perms. This explicit `GetNamedPipeClientProcessId → token SID` compare is the
/// belt-and-suspenders parity for the Unix `SO_PEERCRED` same-uid check (defence
/// in depth; closes the `peer_pid = None` gap, MLP2-028). Because the client
/// handle is still connected during the check, the queried PID is live, so the
/// PID-reuse window the Unix peer check also carries is in-model.
///
/// `server_handle` MUST be a connected named-pipe **server** handle, passed as a
/// [`std::os::windows::io::RawHandle`] (e.g. tokio's
/// `NamedPipeServer::as_raw_handle()` after `connect().await`) so the
/// `forbid(unsafe_code)` daemon can call this without naming a Win32 `HANDLE`.
/// The handle is borrowed, never closed.
///
/// # Errors
/// Propagates the OS error if the client PID, the client process token, or
/// either user SID cannot be read.
pub fn named_pipe_client_is_owner(server_handle: RawHandle) -> io::Result<bool> {
    let client_pid = named_pipe_client_pid(server_handle)?;
    let client_sid = process_user_sid(client_pid)?;
    let current_sid = current_user_sid_string()?;
    Ok(client_sid == current_sid)
}

/// PID of the process at the client end of a connected named-pipe server handle.
fn named_pipe_client_pid(server_handle: RawHandle) -> io::Result<u32> {
    let mut pid: u32 = 0;
    // SAFETY: `server_handle` is a live, connected named-pipe server handle the
    // caller owns (`RawHandle` is `*mut c_void`, i.e. a Win32 `HANDLE`); `&mut
    // pid` is a valid out parameter. The call only reads the client PID and
    // never closes the handle.
    let ok = unsafe { GetNamedPipeClientProcessId(server_handle as HANDLE, &mut pid) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(pid)
}

/// Whether the process at the *server* end of a connected named-pipe client
/// handle runs under the same user SID as this process.
///
/// The per-user pipe name and owner-only server DACL prevent a legitimate
/// daemon from accepting another user's client. They do not prevent a
/// different user from pre-creating the predictable pipe name with a
/// permissive DACL. Authenticate the connected server before any request bytes
/// leave this process so such a pipe cannot receive document contents or forge
/// a daemon response.
fn named_pipe_server_is_owner(client_handle: RawHandle) -> io::Result<bool> {
    let server_pid = named_pipe_server_pid(client_handle)?;
    let server_sid = process_user_sid(server_pid)?;
    sid_matches_current_user(&server_sid)
}

/// PID of the process at the server end of a connected named-pipe client
/// handle.
fn named_pipe_server_pid(client_handle: RawHandle) -> io::Result<u32> {
    let mut pid: u32 = 0;
    // SAFETY: `client_handle` is a live, connected named-pipe client handle the
    // caller owns; `&mut pid` is a valid out parameter. The call only reads the
    // server PID and never closes the handle.
    let ok = unsafe { GetNamedPipeServerProcessId(client_handle as HANDLE, &mut pid) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(pid)
}

fn sid_matches_current_user(peer_sid: &str) -> io::Result<bool> {
    Ok(peer_sid == current_user_sid_string()?)
}

/// The token-user SID string of the process identified by `pid`.
fn process_user_sid(pid: u32) -> io::Result<String> {
    let process = ProcessHandle::open_query(pid)?;
    let mut token: HANDLE = null_mut();
    // SAFETY: `process.0` is a live process handle opened with
    // PROCESS_QUERY_LIMITED_INFORMATION (sufficient for OpenProcessToken with
    // TOKEN_QUERY); `&mut token` is a valid out pointer for an owned token
    // handle we wrap in `Token` (closed exactly once on drop).
    let ok = unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Token(token);
    sid_string_from_token(token.0)
}

/// Synchronous, owner-only named-pipe client. Mirrors
/// [`create_owner_only_pipe_server`] for callers running outside any
/// tokio runtime — specifically the `anvil intercept status` CLI
/// command, which needs a single round-trip JSON-RPC call without
/// dragging in the daemon's async machinery.
///
/// The handle is wrapped in [`OwnerOnlyPipeClient`], a small RAII
/// type that closes via `CloseHandle` on drop in the same style as
/// [`JobObject`]. All `unsafe` for `CreateFileW`, `WriteFile`,
/// `ReadFile`, and `CloseHandle` is quarantined to this crate so
/// `anvil-intercept` can keep `#![forbid(unsafe_code)]`.
///
/// The trust model is mutual same-user authentication. The daemon-side
/// owner-only DACL rejects clients from a different SID, while the connected
/// client validates the server process SID before returning the handle to its
/// caller. Any identity lookup failure is terminal and the handle is closed,
/// so request or document bytes cannot be sent to an unauthenticated server.
pub fn connect_owner_only_pipe_client(pipe_name: &str) -> io::Result<OwnerOnlyPipeClient> {
    let wide = wide_null(pipe_name);
    // SAFETY: `wide` is a null-terminated UTF-16 string owned for the
    // duration of the call. `CreateFileW` with `OPEN_EXISTING` and a
    // null security descriptor either returns INVALID_HANDLE_VALUE on
    // failure (with the OS error in `GetLastError`) or an owned handle
    // we wrap in `OwnerOnlyPipeClient` and close exactly once on drop.
    // FILE_FLAG_OVERLAPPED is intentionally NOT set: the CLI flow is
    // synchronous, and synchronous handles use `WriteFile`/`ReadFile`
    // directly without an OVERLAPPED structure.
    //
    // SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION caps the security
    // quality of service on the pipe: without an explicit SQOS, a named-pipe
    // server can impersonate the connecting client at SecurityImpersonation
    // level and act on its behalf. Pinning SecurityIdentification lets the
    // server identify the client (for its owner-only ACL check) but not
    // impersonate it — defence-in-depth even though the owner-only DACL
    // already restricts the peer to the same SID.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let client = OwnerOnlyPipeClient(handle);
    if !named_pipe_server_is_owner(client.raw_handle() as RawHandle)? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "named-pipe server is owned by a different user SID",
        ));
    }
    Ok(client)
}

/// Owned synchronous named-pipe client handle. Closes via
/// `CloseHandle` on drop. Mirrors the [`JobObject`] RAII pattern.
///
/// Callers MUST treat this as the only owner of the underlying
/// handle — duplicating it via `DuplicateHandle` and outliving the
/// drop here is undefined behaviour the same way it is for any
/// raw Win32 handle. The `unsafe` boundary is the read/write
/// helpers below; everything they expose to the rest of the
/// workspace is plain `&mut self` IO.
pub struct OwnerOnlyPipeClient(HANDLE);

// SAFETY: A Win32 named-pipe HANDLE is a kernel-object reference; the
// kernel handles its own internal synchronisation for ReadFile /
// WriteFile, and ownership transfer between threads (Send) is safe.
// We do NOT implement Sync — concurrent reads or writes from multiple
// threads on the same pipe would interleave at the JSON-RPC framing
// boundary and corrupt the protocol.
unsafe impl Send for OwnerOnlyPipeClient {}

impl std::fmt::Debug for OwnerOnlyPipeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OwnerOnlyPipeClient")
            .field(&format_args!("{:p}", self.0))
            .finish()
    }
}

impl OwnerOnlyPipeClient {
    /// Borrow the underlying Win32 handle. Callers must not close
    /// the returned handle — drop this `OwnerOnlyPipeClient` instead.
    pub fn raw_handle(&self) -> HANDLE {
        self.0
    }

    /// Write the entire buffer to the pipe in a single
    /// synchronous call. Returns an error if the pipe accepts
    /// fewer bytes than `buf.len()` — the daemon's per-line JSON-RPC
    /// framing relies on the request landing as one frame.
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        // `WriteFile` accepts a u32 byte count; cap at u32::MAX to
        // avoid silent truncation for bizarrely-large request frames.
        // The CLI status request is a few hundred bytes so this is a
        // theoretical guard, not a practical concern.
        let len = u32::try_from(buf.len())
            .map_err(|_| io::Error::other("named-pipe write exceeds u32 byte cap"))?;
        let mut written: u32 = 0;
        // SAFETY: `self.0` is an owned, live pipe handle. `buf` is a
        // valid slice of `len` bytes, and `&mut written` is a valid
        // u32 out parameter. `null_mut()` for OVERLAPPED matches the
        // synchronous handle returned by `connect_owner_only_pipe_client`.
        let ok = unsafe { WriteFile(self.0, buf.as_ptr(), len, &mut written, null_mut()) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if written as usize != buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!(
                    "named-pipe short write: wrote {} of {} bytes",
                    written,
                    buf.len(),
                ),
            ));
        }
        Ok(())
    }

    /// Read up to `buf.len()` bytes from the pipe. Returns the
    /// number of bytes read; `0` indicates the daemon side closed
    /// the pipe (EOF). Partial reads are normal on Windows pipes
    /// when the message is smaller than the buffer — callers should
    /// loop or accumulate until they hit a frame boundary or the
    /// caller-supplied cap.
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = u32::try_from(buf.len())
            .map_err(|_| io::Error::other("named-pipe read exceeds u32 byte cap"))?;
        let mut read: u32 = 0;
        // SAFETY: `self.0` is an owned, live pipe handle; `buf` is a
        // valid mutable slice of `len` bytes; `&mut read` is a valid
        // u32 out parameter; OVERLAPPED is null for synchronous IO.
        let ok = unsafe { ReadFile(self.0, buf.as_mut_ptr(), len, &mut read, null_mut()) };
        if ok == 0 {
            let err = io::Error::last_os_error();
            // A clean named-pipe EOF arrives as `ReadFile` returning 0 with
            // `GetLastError() == ERROR_BROKEN_PIPE` once data has flowed.
            // `ERROR_PIPE_NOT_CONNECTED` is the same EOF signal when the
            // server end is already gone before the first read (or on
            // message-mode pipes). The documented contract maps both to
            // `Ok(0)` so callers can treat them as a normal close rather
            // than an I/O failure.
            if err.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32)
                || err.raw_os_error() == Some(ERROR_PIPE_NOT_CONNECTED as i32)
            {
                return Ok(0);
            }
            return Err(err);
        }
        Ok(read as usize)
    }
}

impl std::io::Read for OwnerOnlyPipeClient {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Delegate to the inherent read so callers that need a generic
        // `R: Read` (BufReader, anvil_run::ipc::read_one_line, etc.) can
        // accept this pipe client directly.
        OwnerOnlyPipeClient::read(self, buf)
    }
}

impl std::io::Write for OwnerOnlyPipeClient {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // The inherent helper is `write_all`; the trait's `write`
        // contract permits partial writes, but the pipe client refuses
        // short writes so this either writes the full slice or errors.
        OwnerOnlyPipeClient::write_all(self, buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Win32 named pipes are synchronous and `WriteFile` returns
        // only after the kernel has accepted the buffer, so there is
        // no userspace buffer to drain.
        Ok(())
    }
}

impl Drop for OwnerOnlyPipeClient {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            // SAFETY: `self.0` is an owned pipe handle returned by
            // `CreateFileW` and is closed exactly once here.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Return whether a process is live, conservatively treating access-denied as live.
///
/// A successfully-opened handle is further checked with
/// [`ProcessHandle::is_running`]: the process object outlives the process
/// itself while any handle is open, so "could open the handle" alone would
/// report an exited-but-unreaped process (or a closing one) as live.
pub fn process_exists(pid: u32) -> io::Result<bool> {
    match ProcessHandle::open_query(pid) {
        Ok(handle) => handle.is_running(),
        Err(err) if err.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) => Ok(false),
        Err(err) if err.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => Ok(true),
        Err(err) => Err(err),
    }
}

/// Terminate a process by PID.
///
/// Used by `anvil intercept stop` on Windows, where the daemon can run as a
/// headless background process without a Unix-style signal channel. The caller
/// is responsible for PID-reuse defence before invoking this helper; the
/// intercept daemon library checks the PID file's recorded creation time before
/// reaching this boundary.
pub fn terminate_process(pid: u32) -> io::Result<()> {
    // SAFETY: OpenProcess returns either NULL+last_error or a valid owned handle
    // that ProcessHandle closes on drop.
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            return Ok(());
        }
        return Err(err);
    }
    let process = ProcessHandle(handle);
    // SAFETY: `process.0` is an owned live process handle opened with
    // PROCESS_TERMINATE. Exit code 1 matches the existing job-object forced
    // termination path.
    let ok = unsafe { TerminateProcess(process.0, 1) };
    if ok == 0 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

/// Owned Win32 Job Object handle. Drops via `CloseHandle`. The intercept
/// daemon (INTD-006) uses Job Objects as the Windows equivalent of the
/// Unix process-group interrupt: a session's process and every child it
/// spawns is assigned to the same job, so `TerminateJobObject` reliably
/// stops the entire group even when the agent has spawned grandchildren
/// outside the daemon's view.
///
/// The `unsafe` for handle creation, assignment, and termination is
/// quarantined here so `anvil-intercept` keeps `#![forbid(unsafe_code)]`.
pub struct JobObject(HANDLE);

impl JobObject {
    /// Create a new job object with an explicit owner-only DACL. The
    /// caller is responsible for assigning processes to it via
    /// [`JobObject::assign_process`] and terminating it via
    /// [`terminate_job_object`].
    pub fn create_owner_only() -> io::Result<Self> {
        let sid = current_user_sid_string()?;
        let sddl = owner_only_job_sddl(&sid);
        let descriptor = security_descriptor_from_sddl(&sddl)?;
        let mut attrs = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.as_ptr(),
            bInheritHandle: 0,
        };
        // SAFETY: `attrs.lpSecurityDescriptor` is a valid descriptor that
        // is alive for the duration of `CreateJobObjectW`. Passing
        // `null_mut()` for the name creates an unnamed job per MSDN.
        let handle =
            unsafe { CreateJobObjectW(&mut attrs as *mut SECURITY_ATTRIBUTES, null_mut()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        // The descriptor only needs to live until CreateJobObjectW
        // returns; the OS copies the relevant ACLs into the kernel
        // object. Drop happens here at end-of-scope.
        drop(descriptor);
        Ok(Self(handle))
    }

    /// Assign a process by PID to this job. The PID-reuse defence
    /// (matching `started_at_unix` against the registry record) is the
    /// caller's responsibility — see `crate::interrupt` in
    /// `anvil-intercept`. This helper only opens the process with the
    /// minimum rights required and calls `AssignProcessToJobObject`.
    pub fn assign_process(&self, pid: u32) -> io::Result<()> {
        // SAFETY: OpenProcess returns either NULL+last_error or a valid
        // owned handle that we close when `proc` drops.
        let proc_handle = unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_SET_QUOTA, 0, pid) };
        if proc_handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let proc = ProcessHandle(proc_handle);
        // SAFETY: `self.0` is an owned, live job handle and `proc.0` is
        // an owned process handle opened above; both are valid for the
        // duration of this call.
        let ok = unsafe { AssignProcessToJobObject(self.0, proc.0) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Borrow the underlying Win32 handle. Callers must not close the
    /// returned handle; use [`terminate_job_object`] or drop this
    /// `JobObject` to release the underlying kernel object.
    pub fn raw_handle(&self) -> HANDLE {
        self.0
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` is an owned job handle returned by
            // CreateJobObjectW and is closed exactly once here.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Terminate every process assigned to the supplied job. INTD-006's
/// Windows interrupt path calls this in lieu of the Unix
/// SIGINT → SIGTERM → SIGKILL ladder: Windows has no equivalent of
/// `SIGINT` for non-console processes and Job Objects already provide
/// the "kill the whole group atomically" semantics. The exit code is
/// pinned at `1` (matching `pitchfork@cea18d7`'s default for forced
/// termination); a future revision may surface a distinct exit code so
/// downstream tooling can tell intercept-driven termination apart from
/// natural exit, but v1 keeps it simple.
pub fn terminate_job_object(job: &JobObject) -> io::Result<()> {
    // SAFETY: `job.0` is an owned, live job handle; `1` is the exit
    // code applied to every process in the job per MSDN.
    let ok = unsafe { TerminateJobObject(job.0, 1) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn owner_only_job_sddl(sid: &str) -> String {
    // Owner = current user, DACL = owner-only with no inheritance.
    // GA (generic all) is acceptable on the job object itself because
    // the trust boundary is per-user; the intercept daemon and the
    // agent run under the same UID, and the job object is unnamed so
    // it cannot be opened by name from another user.
    format!("O:{sid}D:P(A;;GA;;;{sid})")
}

/// Windows process creation time as raw FILETIME ticks, if the process can be queried.
pub fn process_creation_time(pid: u32) -> io::Result<Option<u64>> {
    let handle = match ProcessHandle::open_query(pid) {
        Ok(handle) => handle,
        Err(err) if err.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) => return Ok(None),
        Err(err) if err.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => return Ok(None),
        Err(err) => return Err(err),
    };

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: all FILETIME pointers are valid out parameters and `handle` is a
    // live process handle opened with PROCESS_QUERY_LIMITED_INFORMATION.
    let ok = unsafe { GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(Some(
        (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime),
    ))
}

struct OwnerOnlySecurityAttributes {
    attrs: SECURITY_ATTRIBUTES,
    _descriptor: LocalMem,
}

impl OwnerOnlySecurityAttributes {
    fn new() -> io::Result<Self> {
        let sid = current_user_sid_string()?;
        let sddl = owner_only_pipe_sddl(&sid);
        let descriptor = security_descriptor_from_sddl(&sddl)?;
        let attrs = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.as_ptr(),
            bInheritHandle: 0,
        };
        Ok(Self {
            attrs,
            _descriptor: descriptor,
        })
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        &mut self.attrs as *mut SECURITY_ATTRIBUTES as *mut c_void
    }
}

struct Token(HANDLE);

impl Token {
    fn current_process() -> io::Result<Self> {
        let mut handle: HANDLE = null_mut();
        // SAFETY: `handle` is a valid out pointer; GetCurrentProcess returns a
        // pseudo-handle valid for OpenProcessToken.
        let ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(handle))
    }
}

struct ProcessHandle(HANDLE);

impl ProcessHandle {
    fn open_query(pid: u32) -> io::Result<Self> {
        // SAFETY: OpenProcess takes a PID value and returns either a null handle
        // plus last-error, or an owned handle closed by ProcessHandle::drop.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(handle))
    }

    /// Whether the process is still running.
    ///
    /// `OpenProcess` succeeding only proves the kernel process *object*
    /// still exists — it persists after the process exits for as long as
    /// any handle to it is open — so an openable handle does NOT imply a
    /// live process. Confirm via `GetExitCodeProcess`: a running process
    /// reports `STILL_ACTIVE`, an exited one reports its real exit code.
    ///
    /// Caveat: a process that genuinely exits with code 259 is
    /// indistinguishable from a running one here; that collision is
    /// accepted (it matches `GetExitCodeProcess`'s documented contract and
    /// is the convention the daemon's liveness check relies on).
    fn is_running(&self) -> io::Result<bool> {
        let mut exit_code: u32 = 0;
        // SAFETY: `self.0` is an owned process handle opened with
        // PROCESS_QUERY_LIMITED_INFORMATION, which is sufficient for
        // GetExitCodeProcess; `exit_code` is a valid out-pointer for the
        // duration of the call.
        let ok = unsafe { GetExitCodeProcess(self.0, &mut exit_code) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(exit_code == STILL_ACTIVE)
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` is an owned process handle returned by
            // OpenProcess and is closed exactly once here.
            unsafe { CloseHandle(self.0) };
        }
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` is an owned token handle returned by
            // OpenProcessToken and is closed exactly once here.
            unsafe { CloseHandle(self.0) };
        }
    }
}

struct LocalMem(*mut c_void);

impl LocalMem {
    fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

impl Drop for LocalMem {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: LocalFree accepts pointers allocated by the Win32 SDDL
            // conversion helpers used below.
            unsafe { LocalFree(self.0) };
        }
    }
}

/// The current process token's user SID as a string
/// (e.g. `S-1-5-21-…-1001`).
///
/// This is the identity anchor for the per-user named-pipe rendezvous:
/// `anvil_intercept::ipc::resolve_pipe_name` combines it with the active
/// install root (`ANVIL_HOME`, CIB-106) to derive the canonical pipe name.
/// The SID — not an env username — is used so account-name spoofing and
/// local/domain username collisions cannot move the rendezvous point.
/// Consumers MUST NOT format a pipe name from this directly; resolve it
/// through `anvil_intercept::ipc::resolve_pipe_name` so the daemon and
/// every client agree on the install-root-aware name.
pub fn current_user_sid_string() -> io::Result<String> {
    let token = Token::current_process()?;
    sid_string_from_token(token.0)
}

/// The current process token's **owner** SID — the default owner Windows stamps
/// on objects the token creates. Equals the user SID for a normal user, and the
/// Administrators group SID for an elevated token. Used by [`read_trusted_config`]
/// so an admin-created (Administrators-owned) config still validates as trusted.
fn current_token_owner_sid() -> io::Result<String> {
    let token = Token::current_process()?;
    let mut len = 0;
    // SAFETY: first call passes a null buffer to obtain the required byte count.
    unsafe {
        GetTokenInformation(token.0, TokenOwner, null_mut(), 0, &mut len);
    }
    if len == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0_u8; len as usize];
    // SAFETY: `buffer` is valid for `len` bytes and receives a TOKEN_OWNER.
    let ok = unsafe {
        GetTokenInformation(
            token.0,
            TokenOwner,
            buffer.as_mut_ptr().cast(),
            len,
            &mut len,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let token_owner = buffer.as_ptr().cast::<TOKEN_OWNER>();
    // SAFETY: a successful GetTokenInformation(TokenOwner) populated a TOKEN_OWNER
    // at the start of `buffer`; the contained SID pointer is valid while it lives.
    let sid = unsafe { (*token_owner).Owner };
    let mut sid_string = null_mut();
    // SAFETY: `sid` is a valid token-owner SID; `sid_string` receives a
    // LocalAlloc-owned wide string released by `LocalMem`.
    let ok = unsafe { ConvertSidToStringSidW(sid, &mut sid_string) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let sid_string = LocalMem(sid_string.cast());
    wide_ptr_to_string(sid_string.as_ptr().cast())
}

/// Extract the `TOKEN_USER` SID from an open access token and render it as a
/// string. Shared by the current-process SID lookup and the peer-SID check
/// (DSV-010b) so both go through one audited `GetTokenInformation(TokenUser)`
/// path. `token` is borrowed; the caller owns and closes it.
fn sid_string_from_token(token: HANDLE) -> io::Result<String> {
    let mut len = 0;
    // SAFETY: First call intentionally passes a null buffer to obtain the
    // required byte count in `len`.
    unsafe {
        GetTokenInformation(token, TokenUser, null_mut(), 0, &mut len);
    }
    if len == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut buffer = vec![0_u8; len as usize];
    // SAFETY: `buffer` is valid for `len` bytes and receives a TOKEN_USER.
    let ok =
        unsafe { GetTokenInformation(token, TokenUser, buffer.as_mut_ptr().cast(), len, &mut len) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    let token_user = buffer.as_ptr().cast::<TOKEN_USER>();
    // SAFETY: successful GetTokenInformation(TokenUser) populated a TOKEN_USER
    // at the start of `buffer`; the contained SID pointer is valid while the
    // buffer is alive.
    let sid = unsafe { (*token_user).User.Sid };
    let mut sid_string = null_mut();
    // SAFETY: `sid` is a valid token-user SID and `sid_string` is a valid out
    // pointer for a LocalAlloc-owned wide string.
    let ok = unsafe { ConvertSidToStringSidW(sid, &mut sid_string) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let sid_string = LocalMem(sid_string.cast());

    wide_ptr_to_string(sid_string.as_ptr().cast())
}

fn security_descriptor_from_sddl(sddl: &str) -> io::Result<LocalMem> {
    let wide = wide_null(sddl);
    let mut descriptor = null_mut();
    // SAFETY: `wide` is a null-terminated SDDL string. `descriptor` receives a
    // LocalAlloc-owned SECURITY_DESCRIPTOR released by LocalMem::drop.
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(LocalMem(descriptor.cast()))
}

fn owner_only_pipe_sddl(sid: &str) -> String {
    format!("O:{sid}D:P(A;;{OWNER_PIPE_RIGHTS};;;{sid})")
}

// --------------------------------------------------------------------
// DSV-010b config-trust: the Windows analogue of the Unix
// `confinement::read_trusted` owner-only check + 0700 config dir.
// --------------------------------------------------------------------

/// Outcome of [`read_trusted_config`] — the Windows analogue of the Unix
/// `confinement::read_trusted` owner-only check.
#[derive(Debug)]
pub enum TrustedConfigRead {
    /// No file exists at the path (the caller folds this to its default config,
    /// like the Unix missing-file path).
    NotFound,
    /// The path is a reparse point (symlink/junction): refused, since it could
    /// redirect the read to a file another principal controls — the Windows
    /// analogue of the Unix `O_NOFOLLOW` → `ELOOP` refusal.
    Reparse,
    /// The file's owner SID is not the current user's: refused (another
    /// principal could rewrite the trust boundary).
    NotOwner {
        owner_sid: String,
        current_sid: String,
    },
    /// Owner-verified and not a reparse point — the file contents.
    Trusted(String),
}

/// Read an operator config file only if it is trusted: not a reparse point
/// (symlink/junction) **and** owned by the current user. The Windows analogue of
/// the Unix `read_trusted` (`O_NOFOLLOW` + owner-uid + no-foreign-write). The
/// "no foreign write" property comes from the owner-only config directory
/// ([`create_owner_only_dir`]) plus the per-user profile ACLs Windows applies
/// under `%APPDATA%`/`%USERPROFILE%`; this function verifies owner + no-redirect
/// at read time, reading the **verified handle** (never re-opening the path).
///
/// # Errors
/// Propagates a genuine OS error (open / file-info / security-query / read
/// failure). A missing file, a reparse point, or a foreign owner are reported via
/// [`TrustedConfigRead`] — not as errors — so the caller can fail-open (missing)
/// or fail-closed (untrusted) per its own policy.
pub fn read_trusted_config(path: &Path) -> io::Result<TrustedConfigRead> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    // `FILE_FLAG_OPEN_REPARSE_POINT` opens a symlink/junction AS the link object
    // (not followed), so a reparse point is detected + refused below rather than
    // silently reading its target. The full share mask
    // (`READ | WRITE | DELETE`, as elsewhere in this crate) avoids a spurious
    // sharing-violation IO error — and thus a fail-closed daemon — when an editor
    // or another process has the config file open; it does not weaken the
    // owner-SID / reparse checks.
    // SAFETY: `wide` is a NUL-terminated path; the returned handle is owned by
    // `OwnedFileHandle` and closed exactly once on drop. Other args are scalars.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let err = io::Error::last_os_error();
        return match err.raw_os_error() {
            Some(c) if c == ERROR_FILE_NOT_FOUND_CT || c == ERROR_PATH_NOT_FOUND_CT => {
                Ok(TrustedConfigRead::NotFound)
            }
            _ => Err(err),
        };
    }
    let handle = OwnedFileHandle(handle);

    // Refuse a reparse point (symlink / junction).
    // SAFETY: `info` is a valid out struct; `handle.0` is a live file handle.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(handle.0, &mut info) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Ok(TrustedConfigRead::Reparse);
    }

    // Verify the file owner SID is one this process "owns": its token user SID,
    // or its token OWNER SID — the default owner Windows stamps on objects the
    // token creates. For a normal user these are equal (the user SID); for an
    // elevated/admin token the OWNER is the Administrators group (S-1-5-32-544),
    // so admin-created files are owned by Administrators, not the user. Accepting
    // both is the faithful Windows analogue of the Unix `owner-uid == current-uid`
    // check (which has no such split); a file owned by a genuinely different
    // principal is still refused.
    let user_sid = current_user_sid_string()?;
    let token_owner_sid = current_token_owner_sid()?;
    let owner_sid = owner_sid_of_handle(handle.0)?;
    if !owner_sid.eq_ignore_ascii_case(&user_sid)
        && !owner_sid.eq_ignore_ascii_case(&token_owner_sid)
    {
        return Ok(TrustedConfigRead::NotOwner {
            owner_sid,
            current_sid: user_sid,
        });
    }

    // Trusted: read the verified handle.
    Ok(TrustedConfigRead::Trusted(read_handle_to_string(handle.0)?))
}

/// The owner SID (as a string) of an open file handle, via
/// `GetSecurityInfo(OWNER_SECURITY_INFORMATION)`.
fn owner_sid_of_handle(handle: HANDLE) -> io::Result<String> {
    let mut owner_sid: *mut c_void = null_mut();
    let mut descriptor: *mut c_void = null_mut();
    // SAFETY: `handle` is a live file handle. `owner_sid` receives a pointer INTO
    // the returned security `descriptor` (LocalAlloc-owned, freed by `LocalMem`);
    // the group/DACL/SACL out-params are null because only the owner is queried.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner_sid,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let descriptor = LocalMem(descriptor.cast());
    if owner_sid.is_null() {
        return Err(io::Error::other(
            "GetSecurityInfo returned a null owner SID",
        ));
    }
    let mut sid_string = null_mut();
    // SAFETY: `owner_sid` is a valid SID inside the live `descriptor`;
    // `sid_string` receives a LocalAlloc-owned wide string freed by `LocalMem`.
    let ok = unsafe { ConvertSidToStringSidW(owner_sid, &mut sid_string) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let sid_string = LocalMem(sid_string.cast());
    let owner = wide_ptr_to_string(sid_string.as_ptr().cast());
    // The owner SID has been copied into `owner`; the descriptor + string can go.
    drop(descriptor);
    owner
}

/// Read an open file handle to a `String`, refusing more than
/// [`MAX_TRUSTED_CONFIG_BYTES`] (a same-user peer must not make the daemon buffer
/// an unbounded config).
fn read_handle_to_string(handle: HANDLE) -> io::Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let mut read: u32 = 0;
        // SAFETY: `handle` is a live read handle; `chunk` is a valid mutable
        // buffer; `&mut read` is a valid out param; null OVERLAPPED = sync IO.
        let ok = unsafe {
            ReadFile(
                handle,
                chunk.as_mut_ptr(),
                chunk.len() as u32,
                &mut read,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if read == 0 {
            break;
        }
        if buf.len() as u64 + u64::from(read) > MAX_TRUSTED_CONFIG_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                format!("config exceeds the {MAX_TRUSTED_CONFIG_BYTES}-byte trusted-read ceiling"),
            ));
        }
        buf.extend_from_slice(&chunk[..read as usize]);
    }
    String::from_utf8(buf).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

/// Create `dir` (and any missing parents) with an owner-only DACL — the Windows
/// analogue of the Unix 0700 config dir, so a **newly-created** config dir
/// restricts modify access to the current user (admins / SYSTEM excepted, as
/// always on Windows). Idempotent: an already-existing directory is accepted
/// **without** re-applying or validating its DACL (it may have been created by
/// another tool, or be an operator-pointed `ANVIL_HOME`), so this is not by
/// itself a guarantee for pre-existing dirs — the read-time owner + reparse check
/// in [`read_trusted_config`] is the authoritative trust gate regardless.
///
/// # Errors
/// Propagates a directory-creation failure other than "already exists".
pub fn create_owner_only_dir(dir: &Path) -> io::Result<()> {
    // Parents only need to exist; the leaf carries the owner-only DACL.
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let sid = current_user_sid_string()?;
    let descriptor = security_descriptor_from_sddl(&owner_only_dir_sddl(&sid))?;
    let attrs = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.as_ptr(),
        bInheritHandle: 0,
    };
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: `wide` is a NUL-terminated path; `attrs` holds a valid descriptor
    // alive for the call. CreateDirectoryW copies the ACLs into the new object.
    let ok = unsafe { CreateDirectoryW(wide.as_ptr(), &attrs) };
    drop(descriptor);
    if ok == 0 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(ERROR_ALREADY_EXISTS_CT) {
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

fn owner_only_dir_sddl(sid: &str) -> String {
    // Owner = current user; protected DACL (no inheritance from the parent)
    // granting full access to the owner only, inherited by children (OICI). No
    // world / authenticated-user ACE.
    format!("O:{sid}D:P(A;OICI;FA;;;{sid})")
}

/// Owned file `HANDLE` from `CreateFileW`; closes via `CloseHandle` on drop.
struct OwnedFileHandle(HANDLE);

impl Drop for OwnedFileHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            // SAFETY: owned handle from `CreateFileW`, closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }
}

fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain([0]).collect()
}

fn wide_ptr_to_string(ptr: *const u16) -> io::Result<String> {
    if ptr.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Win32 returned a null SID string",
        ));
    }
    let mut len = 0;
    // SAFETY: `ptr` points to a null-terminated UTF-16 string returned by
    // ConvertSidToStringSidW.
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
    }
    // SAFETY: The preceding loop found the terminator, so this slice covers the
    // initialized UTF-16 code units excluding the trailing null.
    let slice = unsafe { &*slice_from_raw_parts(ptr, len) };
    String::from_utf16(slice).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_only_security_attributes_can_be_built() {
        let attrs = OwnerOnlySecurityAttributes::new().expect("build owner-only attributes");
        assert!(!attrs.attrs.lpSecurityDescriptor.is_null());
        assert_eq!(attrs.attrs.bInheritHandle, 0);
    }

    #[test]
    fn owner_only_sddl_does_not_grant_generic_all() {
        let sddl = owner_only_pipe_sddl("S-1-5-21-1-2-3-1000");
        assert!(!sddl.contains("GA"));
        assert!(sddl.contains(OWNER_PIPE_RIGHTS));
    }

    #[test]
    fn current_process_liveness_and_creation_time_can_be_queried() {
        let pid = std::process::id();
        assert!(process_exists(pid).expect("query liveness"));
        assert!(
            process_creation_time(pid)
                .expect("query creation time")
                .is_some()
        );
    }

    #[test]
    fn exited_process_with_open_handle_reports_not_live() {
        // Regression for the bug this fix closes: the kernel process object
        // outlives the process while any handle to it is open, so a bare
        // `OpenProcess` success would mis-report an exited process as live.
        // Hold the `Child` handle open (do not drop it) so the object — and
        // thus the PID → object mapping `OpenProcess` resolves — survives the
        // exit, then assert `process_exists` consults `GetExitCodeProcess`
        // and reports the process as not-live.
        let mut child = std::process::Command::new("cmd")
            .args(["/C", "exit", "0"])
            .spawn()
            .expect("spawn short-lived child");
        let pid = child.id();
        child.wait().expect("wait for child to exit");

        assert!(
            !process_exists(pid).expect("query liveness of exited process"),
            "an exited process (handle still open) must report not-live",
        );

        drop(child);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn creates_owner_only_pipe_server() {
        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-win32-test-{}",
            std::process::id(),
        );
        let server =
            create_owner_only_pipe_server(&pipe_name, PipeInstance::First).expect("create pipe");
        assert!(server.info().is_ok());
    }

    #[test]
    fn creates_and_terminates_owner_only_job_object() {
        // INTD-006 Windows path: create an unnamed job with an
        // owner-only DACL, then terminate it. The lifecycle must
        // complete without leaking handles or surfacing an OS error.
        let job = JobObject::create_owner_only().expect("create job");
        assert!(!job.raw_handle().is_null(), "raw handle is non-null");
        terminate_job_object(&job).expect("terminate empty job");
        // Drop runs CloseHandle; a second terminate after drop would
        // be a use-after-free, so we simply let the test exit here.
    }

    #[test]
    fn owner_only_job_sddl_does_not_grant_world_access() {
        let sddl = owner_only_job_sddl("S-1-5-21-1-2-3-1000");
        assert!(
            !sddl.contains("WD") && !sddl.contains("AU"),
            "world / authenticated-user must not appear in job SDDL: {sddl}",
        );
        assert!(
            sddl.contains("S-1-5-21-1-2-3-1000"),
            "owner SID must appear: {sddl}",
        );
    }

    /// Pin: `current_user_sid_string` is deterministic across calls
    /// within a process. It anchors the pipe-name rendezvous
    /// (`anvil_intercept::ipc::resolve_pipe_name`, CIB-106) — flaking
    /// here means a CLI/daemon mismatch shipped silently.
    #[test]
    fn current_user_sid_string_is_stable() {
        let first = current_user_sid_string().expect("user SID (first)");
        let second = current_user_sid_string().expect("user SID (second)");
        assert_eq!(first, second);
        assert!(
            first.starts_with("S-1-"),
            "expected an `S-1-…` SID string, got {first}",
        );
    }

    /// Client-side server authentication must fail closed when the peer SID
    /// differs from the caller. The integration round-trip below covers the
    /// positive same-SID path through `GetNamedPipeServerProcessId`; this pins
    /// the cross-SID decision independently of which account runs Windows CI.
    #[test]
    fn named_pipe_server_authentication_rejects_a_different_sid() {
        let current = current_user_sid_string().expect("current user SID");
        let different = if current == "S-1-5-18" {
            "S-1-5-19"
        } else {
            "S-1-5-18"
        };

        assert!(
            !sid_matches_current_user(different).expect("compare server SID"),
            "a server process with a different SID must be rejected",
        );
    }

    /// Round-trip: server bind via the existing helper, client
    /// connect via the new sync helper, write and read a single
    /// JSON-line frame on a private per-test pipe name. The server
    /// half intentionally uses tokio (named-pipe servers are async
    /// in tokio); the client half is the synchronous CLI surface.
    #[test]
    fn connect_owner_only_pipe_client_round_trips_against_local_server() {
        use std::thread;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-win32-client-test-{}",
            std::process::id(),
        );
        // Multi-thread runtime so a worker thread drives the async
        // server task while the main thread runs the synchronous
        // client below. `current_thread` would deadlock here — the
        // only thread that could poll the server task would be the
        // main thread, which is blocked on `client_thread.join()`.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let server = {
            let _guard = runtime.enter();
            create_owner_only_pipe_server(&pipe_name, PipeInstance::First)
                .expect("bind owner-only server")
        };

        let server_task = runtime.spawn(async move {
            let mut server = server;
            server.connect().await.expect("server accept");
            let mut request = [0_u8; 64];
            let n = server.read(&mut request).await.expect("server read");
            // Echo the bytes back so the client sees a deterministic
            // payload. Newline-terminated to mirror the real JSON-RPC
            // framing the CLI uses.
            server.write_all(&request[..n]).await.expect("server write");
            server.shutdown().await.expect("server shutdown");
        });

        let client_pipe_name = pipe_name.clone();
        let client_thread = thread::spawn(move || {
            let mut client =
                connect_owner_only_pipe_client(&client_pipe_name).expect("client connect");
            assert!(
                named_pipe_server_is_owner(client.raw_handle() as RawHandle)
                    .expect("query connected server identity"),
                "the local same-user server must authenticate before writes",
            );
            let payload = b"hello-from-cli\n";
            client.write_all(payload).expect("client write");
            let mut buf = [0_u8; 64];
            let n = client.read(&mut buf).expect("client read");
            assert_eq!(&buf[..n], payload);
            // The server `shutdown`s after echoing, so the next read
            // hits a clean EOF. On Windows that arrives as `ReadFile`
            // returning 0 with `GetLastError() == ERROR_BROKEN_PIPE`;
            // pin the documented `Ok(0)` contract so the EOF signal is
            // never re-surfaced as an OS error.
            let eof = client
                .read(&mut buf)
                .expect("client read at EOF must be Ok");
            assert_eq!(eof, 0, "post-payload read must report EOF as Ok(0)");
        });

        client_thread.join().expect("client thread joins");
        runtime.block_on(server_task).expect("server task joins");
    }

    /// Sanity: connecting to a pipe name no daemon ever bound
    /// returns NotFound rather than a generic OS error. The CLI
    /// distinguishes "daemon down" from "daemon refused" on this
    /// signal, so it has to be predictable.
    #[test]
    fn connect_to_nonexistent_pipe_returns_not_found_error() {
        let nonexistent = format!(
            r"\\.\pipe\anvil-intercept-win32-nope-{}",
            std::process::id(),
        );
        let err = connect_owner_only_pipe_client(&nonexistent)
            .expect_err("connecting to a missing pipe must error");
        assert_eq!(err.kind(), io::ErrorKind::NotFound, "got: {err:?}");
    }

    /// DSV-010b: a same-process client validates as owner. The daemon's
    /// Windows accept loop calls `named_pipe_client_is_owner` on the connected
    /// server handle as the belt-and-suspenders parity for the Unix
    /// `SO_PEERCRED` same-uid check; a client opened from this very process
    /// shares the user SID, so the check must return `Ok(true)`.
    #[test]
    fn named_pipe_client_is_owner_true_for_a_same_process_client() {
        use std::os::windows::io::AsRawHandle;
        use std::thread;
        use std::time::Duration;

        let pipe_name = format!(
            r"\\.\pipe\anvil-intercept-win32-owner-test-{}",
            std::process::id(),
        );
        // Multi-thread runtime: a worker drives the async server task while the
        // main thread runs the synchronous client (mirrors the round-trip test).
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let server = {
            let _guard = runtime.enter();
            create_owner_only_pipe_server(&pipe_name, PipeInstance::First).expect("bind server")
        };

        let server_task = runtime.spawn(async move {
            let server = server;
            server.connect().await.expect("server accept");
            // Same-process client ⇒ same user SID ⇒ owner. Query the live PID
            // via the connected server handle (never closing it).
            named_pipe_client_is_owner(server.as_raw_handle()).map_err(|e| e.to_string())
        });

        let client_pipe_name = pipe_name.clone();
        let client_thread = thread::spawn(move || {
            let client = connect_owner_only_pipe_client(&client_pipe_name).expect("client connect");
            // Hold the client open while the server queries the live client PID.
            thread::sleep(Duration::from_millis(200));
            drop(client);
        });

        let owner = runtime.block_on(server_task).expect("server task joins");
        client_thread.join().expect("client thread joins");
        assert_eq!(
            owner,
            Ok(true),
            "a same-process (same-user) client must validate as owner",
        );
    }

    // ---- DSV-010b config-trust (real filesystem; Windows-only) ----

    /// A file the test process owns (it just created it) reads as `Trusted` with
    /// the exact contents — the owner-SID match + verified-handle read path.
    #[test]
    fn read_trusted_config_reads_an_owned_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("confinement.yaml");
        std::fs::write(&path, b"admission: open\n").expect("write config");

        match read_trusted_config(&path).expect("trusted read") {
            TrustedConfigRead::Trusted(raw) => assert_eq!(raw, "admission: open\n"),
            other => panic!("a same-process-owned file must be Trusted, got {other:?}"),
        }
    }

    /// A missing file is `NotFound` (the caller folds it to the default config),
    /// not an error.
    #[test]
    fn read_trusted_config_missing_is_not_found() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("does-not-exist.yaml");
        assert!(matches!(
            read_trusted_config(&path).expect("trusted read"),
            TrustedConfigRead::NotFound
        ));
    }

    /// A file **symlink** is a reparse point and is refused (`Reparse`) — it
    /// could redirect the read to another principal's file. Symlink creation
    /// needs Developer Mode / admin, so skip cleanly where unprivileged.
    #[test]
    fn read_trusted_config_refuses_a_symlinked_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("real.yaml"), b"admission: open\n").expect("write");
        let link = tmp.path().join("link.yaml");
        if std::os::windows::fs::symlink_file("real.yaml", &link).is_err() {
            eprintln!("skipping: symlink creation requires privilege on this runner");
            return;
        }
        assert!(
            matches!(
                read_trusted_config(&link).expect("trusted read"),
                TrustedConfigRead::Reparse
            ),
            "a symlinked config must be refused as a reparse point",
        );
    }

    /// `create_owner_only_dir` creates the directory and is idempotent (a second
    /// call on the existing directory succeeds).
    #[test]
    fn create_owner_only_dir_creates_and_is_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("anvil").join("config");
        create_owner_only_dir(&dir).expect("create owner-only dir");
        assert!(dir.is_dir(), "the owner-only config dir must exist");
        // Idempotent: a second call on the existing dir is Ok.
        create_owner_only_dir(&dir).expect("second create is idempotent");
        // And a trusted read of a file placed in it round-trips.
        let cfg = dir.join("antipattern.yaml");
        std::fs::write(&cfg, b"patterns: []\n").expect("write config");
        match read_trusted_config(&cfg).expect("trusted read") {
            TrustedConfigRead::Trusted(raw) => assert_eq!(raw, "patterns: []\n"),
            other => panic!("config in the owner-only dir must be Trusted, got {other:?}"),
        }
    }
}
