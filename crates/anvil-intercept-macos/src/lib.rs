//! macOS-only helpers for the intercept daemon.
//!
//! `anvil-intercept` forbids unsafe code. This crate keeps the narrow
//! `proc_pidinfo` / `proc_pidpath` FFI boundary in one place and exposes safe
//! process start-time and executable-path helpers.

#![cfg(target_os = "macos")]
#![deny(unsafe_op_in_unsafe_fn)]

use std::path::PathBuf;

/// CIB-160: the macOS peer-executable reader, the analogue of Linux's
/// `/proc/<pid>/exe` symlink read.
///
/// Returns the absolute on-disk path of `pid`'s running executable via
/// `proc_pidpath(2)`, or [`None`] when the process is gone, belongs to
/// another user, or the kernel refuses the read. Every failure is [`None`]
/// so the caller can fail closed.
///
/// `proc_pidpath` writes a NUL-terminated path and returns its byte length
/// (0 on failure). The buffer is `PROC_PIDPATHINFO_MAXSIZE`, the size the
/// call documents as sufficient for any path it will return.
pub fn process_image_path(pid: u32) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt as _;

    let raw = i32::try_from(pid).ok()?;
    // `PROC_PIDPATHINFO_MAXSIZE` (4 * MAXPATHLEN) is the buffer size
    // `proc_pidpath` documents; libc exposes it as a u32.
    let capacity = usize::try_from(libc::PROC_PIDPATHINFO_MAXSIZE).ok()?;
    let mut buffer = vec![0u8; capacity];
    let buffer_size = u32::try_from(buffer.len()).ok()?;
    // SAFETY: `proc_pidpath` writes at most `buffer_size` bytes into the
    // supplied buffer and returns the number written (0 on failure). The
    // buffer is a live, uniquely borrowed allocation of exactly that many
    // bytes, and nothing is read back unless the call reports success.
    let written =
        unsafe { libc::proc_pidpath(raw, buffer.as_mut_ptr().cast::<libc::c_void>(), buffer_size) };
    if written <= 0 {
        return None;
    }
    let written = usize::try_from(written).ok()?;
    if written > buffer.len() {
        // The kernel reported more than it was given room for; refuse
        // rather than index out of bounds.
        return None;
    }
    // `proc_pidpath` returns the length excluding the trailing NUL, but
    // truncate defensively at the first NUL in case a kernel ever counts it.
    let bytes = &buffer[..written];
    let bytes = match bytes.iter().position(|byte| *byte == 0) {
        Some(nul) => &bytes[..nul],
        None => bytes,
    };
    if bytes.is_empty() {
        return None;
    }
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

/// macOS process start time as microseconds since the Unix epoch
/// (`pbi_start_tvsec * 1_000_000 + pbi_start_tvusec`), read via
/// `proc_pidinfo(PROC_PIDTBSDINFO)`.
///
/// Used purely as a PID-reuse discriminator. The unit differs from Linux
/// (`/proc/<pid>/stat` boot ticks), but comparisons are same-platform only.
pub fn process_start_time(pid: u32) -> Option<u64> {
    let raw = i32::try_from(pid).ok()?;
    let mut info = zeroed_proc_bsdinfo();
    let size = i32::try_from(std::mem::size_of::<libc::proc_bsdinfo>()).ok()?;
    // SAFETY: `proc_pidinfo` with flavor `PROC_PIDTBSDINFO` writes up to
    // `size_of::<proc_bsdinfo>()` bytes into the supplied buffer. We pass a
    // zeroed, correctly sized `proc_bsdinfo` and only read it back when the
    // call reports it wrote exactly that many bytes.
    let written = unsafe {
        libc::proc_pidinfo(
            raw,
            libc::PROC_PIDTBSDINFO,
            0,
            std::ptr::from_mut(&mut info).cast(),
            size,
        )
    };
    if written != size {
        return None;
    }
    Some(
        info.pbi_start_tvsec
            .saturating_mul(1_000_000)
            .saturating_add(info.pbi_start_tvusec),
    )
}

fn zeroed_proc_bsdinfo() -> libc::proc_bsdinfo {
    // SAFETY: `proc_bsdinfo` is a plain C out-parameter buffer for
    // `proc_pidinfo`; zero initialization is valid before the kernel fills it.
    unsafe { std::mem::zeroed() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_image_path_resolves_our_own_process() {
        let path = process_image_path(std::process::id())
            .expect("our own process must resolve an image path");
        assert!(!path.as_os_str().is_empty());
        assert!(
            path.is_absolute(),
            "proc_pidpath must return an absolute path"
        );
    }

    #[test]
    fn process_image_path_is_none_for_an_impossible_pid() {
        assert!(process_image_path(0).is_none());
    }

    #[test]
    fn process_start_time_resolves_our_own_process() {
        assert!(process_start_time(std::process::id()).is_some());
    }
}
