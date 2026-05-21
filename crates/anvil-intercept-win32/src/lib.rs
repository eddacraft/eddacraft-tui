//! Windows-only Win32 helpers for the intercept daemon.
//!
//! `anvil-intercept` forbids unsafe code. This crate keeps the narrow
//! Win32 security-attribute boundary in one place and exposes a safe
//! named-pipe constructor for the daemon IPC listener.

#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{OsStr, c_void};
use std::io;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null_mut, slice_from_raw_parts};

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, FILETIME, HANDLE,
    INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, ReadFile, WriteFile,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

// `GENERIC_READ` / `GENERIC_WRITE` are not re-exported from any
// `windows-sys` 0.61 module the daemon already pulls in; pinning them
// inline keeps the Cargo feature-flag set narrow.
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;

// Minimal owner rights for duplex clients plus the server's replacement-instance
// flow. This deliberately avoids GENERIC_ALL; v1 treats same-user processes as
// inside the trust boundary, matching the Unix owner-only socket model.
const OWNER_PIPE_RIGHTS: &str = "0x12019f";

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

/// Compute the per-user named-pipe rendezvous path
/// (`\\.\pipe\anvil-intercept-<sid>`).
///
/// This is the canonical pipe name shared by the daemon-side
/// `IpcListener` and any owner-only client (the `anvil intercept`
/// CLI today; the `DriverClient` from DRVR-001 once the Windows port
/// of the launcher lands). The SID — not the env username — is the
/// suffix so account-name spoofing and local/domain username
/// collisions cannot move the rendezvous point.
///
/// `anvil_intercept::ipc::resolve_pipe_name` re-exports this helper
/// so consumers that already depend on the daemon crate keep working
/// without pulling in `anvil-intercept-win32` directly.
pub fn pipe_name_for_current_user() -> io::Result<String> {
    let sid = current_user_sid_string()?;
    Ok(format!(r"\\.\pipe\anvil-intercept-{sid}"))
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
/// The trust model is the daemon-side ACL: the named pipe is
/// created with the owner-only DACL by
/// [`create_owner_only_pipe_server`], so a client connecting from a
/// different SID is rejected by the kernel. Defence-in-depth pipe-
/// owner validation on the client side is intentionally skipped in
/// v1 — see the security note in the inline doc comment below — but
/// could be layered on later via `GetSecurityInfo` without changing
/// this signature.
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
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(OwnerOnlyPipeClient(handle))
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
            return Err(io::Error::last_os_error());
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
pub fn process_exists(pid: u32) -> io::Result<bool> {
    match ProcessHandle::open_query(pid) {
        Ok(_handle) => Ok(true),
        Err(err) if err.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) => Ok(false),
        Err(err) if err.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => Ok(true),
        Err(err) => Err(err),
    }
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

fn current_user_sid_string() -> io::Result<String> {
    let token = Token::current_process()?;
    let mut len = 0;
    // SAFETY: First call intentionally passes a null buffer to obtain the
    // required byte count in `len`.
    unsafe {
        GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut len);
    }
    if len == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut buffer = vec![0_u8; len as usize];
    // SAFETY: `buffer` is valid for `len` bytes and receives a TOKEN_USER.
    let ok = unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            len,
            &mut len,
        )
    };
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

    /// Pin: `pipe_name_for_current_user` is deterministic across
    /// calls within a process. The daemon and the CLI must compute
    /// the same name — flaking here means a CLI/daemon mismatch
    /// shipped silently.
    #[test]
    fn pipe_name_for_current_user_is_stable() {
        let first = pipe_name_for_current_user().expect("pipe name (first)");
        let second = pipe_name_for_current_user().expect("pipe name (second)");
        assert_eq!(first, second);
        assert!(
            first.starts_with(r"\\.\pipe\anvil-intercept-"),
            "expected `\\.\\pipe\\anvil-intercept-<sid>`, got {first}",
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
            let payload = b"hello-from-cli\n";
            client.write_all(payload).expect("client write");
            let mut buf = [0_u8; 64];
            let n = client.read(&mut buf).expect("client read");
            assert_eq!(&buf[..n], payload);
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
}
