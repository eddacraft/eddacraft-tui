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
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, FILETIME, HANDLE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

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
}
