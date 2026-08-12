//! Bounded, TOCTOU-resistant reads for untrusted `.anvil/` content.
//!
//! Callers may pre-check that a directory entry is a regular file, but a
//! concurrent swap can still replace that entry with a symlink or FIFO before
//! the open. [`read_capped`] therefore opens the path once with platform
//! no-follow (and non-blocking) flags, validates the opened handle is a
//! regular file via `fstat`, and only then reads at most `cap` bytes from that
//! same handle. The read can neither follow a swapped symlink, block on a
//! FIFO, nor run away on a device.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Read up to `cap` bytes of `path` as a UTF-8 string.
///
/// Returns `Ok(None)` when the file exceeds `cap` (it is treated as too large,
/// not truncated-and-used). The read is hard-bounded at `cap + 1` bytes.
///
/// On Unix the open uses `O_NOFOLLOW | O_NONBLOCK` so a final-component symlink
/// is refused (`ELOOP`) and a FIFO/device is not waited on; the open handle is
/// then checked to be a regular file before any read. On non-Unix platforms a
/// best-effort `symlink_metadata` check runs before `File::open` (residual
/// race remains on those platforms).
///
/// # Errors
/// Propagates the open/read error (incl. a non-UTF-8 body, which a binary
/// device would surface as `InvalidData`, and non-regular targets after open)
/// so callers can skip the entry.
pub(crate) fn read_capped(path: &Path, cap: u64) -> io::Result<Option<String>> {
    let file = open_regular_nofollow(path)?;
    let mut buf = String::new();
    // Read one byte past the cap: if that byte exists, the file is over the cap.
    let read = file.take(cap + 1).read_to_string(&mut buf)?;
    if read as u64 > cap {
        Ok(None)
    } else {
        Ok(Some(buf))
    }
}

/// Open `path` for reading only when the final component is a regular file,
/// without following a leaf symlink and without blocking on a FIFO.
fn open_regular_nofollow(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        // O_NOFOLLOW: refuse a leaf symlink (ELOOP) instead of following it —
        // closes the check/open TOCTOU against a concurrent symlink swap.
        // O_NONBLOCK: open of a FIFO does not block waiting for a writer —
        // closes the hang class when a regular file is swapped for a pipe.
        // O_CLOEXEC: keep the handle out of child processes.
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)
        {
            Ok(file) => file,
            Err(e) if e.raw_os_error() == Some(libc::ELOOP) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "symlink refused (O_NOFOLLOW)",
                ));
            }
            Err(e) => return Err(e),
        };
        // Metadata from the open fd (fstat), not a second path lookup.
        let meta = file.metadata()?;
        if !meta.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        Ok(file)
    }

    #[cfg(not(unix))]
    {
        // Best-effort: refuse known non-files before open. A concurrent swap
        // between symlink_metadata and open remains possible on these platforms.
        let meta = std::fs::symlink_metadata(path)?;
        if !meta.file_type().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        File::open(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

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

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_leaf_without_following() {
        // A symlink to external JSON must not be followed — even when the
        // target is a valid regular file. This is the open-time half of the
        // TOCTOU class (file_type check then File::open).
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("outside.json");
        std::fs::write(&target, r#"{ "secret": "leaked" }"#).expect("write target");
        let link = tmp.path().join("state.json");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let err = read_capped(&link, 1024).expect_err("symlink leaf must fail open");
        assert_ne!(
            err.kind(),
            io::ErrorKind::NotFound,
            "refusal must not look like a missing file: {err}"
        );
        // Ensure we did not accidentally read the target through some path.
        // (If open followed, read would succeed with the secret body.)
        assert!(
            err.kind() == io::ErrorKind::InvalidInput
                || err.raw_os_error() == Some(libc::ELOOP),
            "expected symlink refusal, got: {err:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_fifo_without_blocking() {
        // A FIFO swapped in for a regular file must not hang the reader.
        // O_NONBLOCK + is_file() fstat rejects it promptly.
        let tmp = tempfile::tempdir().expect("tempdir");
        let fifo = tmp.path().join("state.json");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("spawn mkfifo");
        assert!(status.success(), "mkfifo failed: {status}");

        let start = Instant::now();
        let result = read_capped(&fifo, 1024);
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "FIFO open must not block (elapsed {elapsed:?})"
        );
        let err = result.expect_err("FIFO must be refused");
        assert_eq!(
            err.kind(),
            io::ErrorKind::InvalidInput,
            "FIFO refusal should be InvalidInput, got: {err:?}"
        );
    }
}
