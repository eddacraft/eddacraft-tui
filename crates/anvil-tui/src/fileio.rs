//! Bounded reads for untrusted `.anvil/` content.
//!
//! Even after a no-follow regular-file check, reading a path with
//! [`std::fs::read_to_string`] re-opens it and would follow a symlink swapped in
//! after the check (a check/read TOCTOU) — and an unbounded read of a device or
//! FIFO (`/dev/zero`, `/dev/urandom`) hangs or exhausts memory. [`read_capped`]
//! removes both: it reads at most `cap` bytes from a single open handle, so the
//! read can neither run away nor be retargeted by a separate stat.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Read up to `cap` bytes of `path` as a UTF-8 string.
///
/// Returns `Ok(None)` when the file exceeds `cap` (it is treated as too large,
/// not truncated-and-used). The read is hard-bounded at `cap + 1` bytes, so a
/// device/FIFO target cannot hang or exhaust memory.
///
/// # Errors
/// Propagates the open/read error (incl. a non-UTF-8 body, which a binary device
/// surfaces as an `InvalidData` error) so callers can skip it.
pub(crate) fn read_capped(path: &Path, cap: u64) -> io::Result<Option<String>> {
    let mut buf = String::new();
    // Read one byte past the cap: if that byte exists, the file is over the cap.
    let read = File::open(path)?.take(cap + 1).read_to_string(&mut buf)?;
    if read as u64 > cap {
        Ok(None)
    } else {
        Ok(Some(buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_small_files_and_caps_large_ones() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let small = tmp.path().join("small.txt");
        std::fs::write(&small, "hello").expect("write");
        assert_eq!(
            read_capped(&small, 1024).expect("read"),
            Some("hello".into())
        );

        let big = tmp.path().join("big.txt");
        std::fs::write(&big, "x".repeat(100)).expect("write");
        assert_eq!(
            read_capped(&big, 10).expect("read"),
            None,
            "over-cap -> None"
        );
        // Exactly at the cap is accepted.
        assert!(read_capped(&big, 100).expect("read").is_some());
    }
}
