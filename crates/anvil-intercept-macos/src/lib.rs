//! macOS-only helpers for the intercept daemon.
//!
//! `anvil-intercept` forbids unsafe code. This crate keeps the narrow
//! `proc_pidinfo` FFI boundary in one place and exposes a safe process
//! start-time helper for PID-reuse discrimination.

#![cfg(target_os = "macos")]
#![deny(unsafe_op_in_unsafe_fn)]

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
