//! Park-and-swap support for Windows self-update (CIB-362).
//!
//! Windows locks a running executable's *contents* — the loaded image
//! section blocks deletion and overwrite with a sharing violation — but
//! renaming the file is a directory-namespace operation and stays
//! allowed. Renaming the live `anvil.exe` aside therefore frees the
//! name for the installer to create a fresh binary while every current
//! holder (the intercept daemon, editor `anvil mcp serve` children, and
//! the `anvil update` process itself) keeps executing the parked image.
//! This is the same technique rustup uses for `rustup self update`.
//!
//! A parked file cannot be deleted until its last holder exits, so
//! parks are left behind under a recognisable name
//! (`anvil.exe.old-<pid>`) and swept best-effort at the start of later
//! update runs. The parked name deliberately does not end in `.exe`, so
//! leftovers in a `PATH` directory never become invocable commands.
//!
//! Everything here is plain `std::fs`, so the logic compiles and is
//! unit-tested on every platform; only the "rename succeeds while the
//! image is mapped" semantics is Windows-specific.

use std::io;
use std::path::{Path, PathBuf};

/// Marker between the binary file name and the uniquifier. Also the
/// sweep match prefix — keep the two in lockstep.
const PARK_MARKER: &str = ".old-";

/// How many candidate park names to probe before giving up. Each
/// leftover requires a still-running holder from a previous update with
/// the same PID-derived name, so collisions beyond one are pathological.
const MAX_PARK_ATTEMPTS: u32 = 16;

/// A live binary renamed out of the way of the installer.
#[derive(Debug)]
pub(crate) struct ParkedBinary {
    pub original: PathBuf,
    pub parked: PathBuf,
}

/// Rename `exe` aside so its name becomes free for the installer.
///
/// Returns `Ok(None)` when nothing exists at `exe` — nothing to park,
/// and the installer can create the file freely.
pub(crate) fn park(exe: &Path) -> io::Result<Option<ParkedBinary>> {
    // `try_exists`, not `exists`: a stat failure (permissions, an antivirus
    // hold) must surface as the actionable swap-failure decline, not be read
    // as "nothing to park" only to fail later inside the installer.
    if !exe.try_exists()? {
        return Ok(None);
    }
    let pid = std::process::id();
    let mut last_err = None;
    for attempt in 0..MAX_PARK_ATTEMPTS {
        let candidate = park_sibling(exe, pid, attempt)?;
        if candidate.try_exists()? {
            continue;
        }
        match std::fs::rename(exe, &candidate) {
            Ok(()) => {
                return Ok(Some(ParkedBinary {
                    original: exe.to_path_buf(),
                    parked: candidate,
                }));
            }
            // The source vanished between the probe and the rename — a
            // concurrent updater parked it first. The name is free, which
            // is all the installer needs.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            // A concurrent updater can claim the candidate between the
            // existence probe and the rename; try the next name.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last_err = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("no free park name after {MAX_PARK_ATTEMPTS} attempts"),
        )
    }))
}

/// Restore a parked binary to its original name after a failed install,
/// replacing any partial file the installer left behind.
pub(crate) fn unpark(parked: &ParkedBinary) -> io::Result<()> {
    std::fs::rename(&parked.parked, &parked.original)
}

/// What a sweep found beside the binary.
#[derive(Debug, Default)]
pub(crate) struct SweepOutcome {
    /// Parks whose holders have exited; now deleted.
    pub removed: Vec<PathBuf>,
    /// Parks a process still holds (or that otherwise refused
    /// deletion); a later run retries.
    pub still_held: Vec<PathBuf>,
}

/// Delete leftover parked binaries beside `exe` whose holders have
/// exited. Deletion failures are expected while a holder is still
/// running and are reported, not raised — sweeping is opportunistic
/// hygiene, never a reason to fail an update.
pub(crate) fn sweep_stale_parks(exe: &Path) -> SweepOutcome {
    let mut outcome = SweepOutcome::default();
    let (Some(dir), Some(file_name)) = (exe.parent(), exe.file_name().and_then(|n| n.to_str()))
    else {
        return outcome;
    };
    let prefix = format!("{file_name}{PARK_MARKER}");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return outcome;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(&prefix) {
            continue;
        }
        let path = entry.path();
        match std::fs::remove_file(&path) {
            Ok(()) => outcome.removed.push(path),
            Err(_) => outcome.still_held.push(path),
        }
    }
    outcome
}

/// Build the parked sibling path for `exe`: same directory (renames
/// must stay on one volume), name `<file>.old-<pid>` with `-<attempt>`
/// appended on collision.
fn park_sibling(exe: &Path, pid: u32, attempt: u32) -> io::Result<PathBuf> {
    let file_name = exe.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "executable has no file name")
    })?;
    let suffix = if attempt == 0 {
        format!("{file_name}{PARK_MARKER}{pid}")
    } else {
        format!("{file_name}{PARK_MARKER}{pid}-{attempt}")
    };
    Ok(exe.with_file_name(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        std::fs::write(path, b"binary bytes").expect("write test file");
    }

    #[test]
    fn park_renames_beside_original_and_frees_the_name() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("anvil.exe");
        touch(&exe);

        let parked = park(&exe).expect("park succeeds").expect("file existed");
        assert!(
            !exe.exists(),
            "original name must be free for the installer"
        );
        assert!(parked.parked.exists(), "parked file must survive");
        assert_eq!(parked.parked.parent(), Some(dir.path()));
        let name = parked.parked.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with("anvil.exe.old-"),
            "park name must be sweepable: {name}"
        );
        assert!(
            !Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe")),
            "a parked file on PATH must not be an invocable command: {name}"
        );
    }

    #[test]
    fn park_of_missing_file_is_a_clean_none() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("anvil.exe");
        assert!(park(&exe).expect("missing file is not an error").is_none());
    }

    #[test]
    fn park_skips_an_occupied_candidate_name() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("anvil.exe");
        touch(&exe);
        // Simulate a leftover park from a previous run of this same PID.
        let occupied = park_sibling(&exe, std::process::id(), 0).unwrap();
        touch(&occupied);

        let parked = park(&exe).expect("park succeeds").expect("file existed");
        assert_ne!(parked.parked, occupied, "must not clobber a live park");
        assert!(occupied.exists());
        assert!(!exe.exists());
    }

    #[test]
    fn unpark_restores_over_a_partial_install() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("anvil.exe");
        touch(&exe);
        let parked = park(&exe).unwrap().unwrap();
        // Installer wrote a partial file, then the update failed.
        std::fs::write(&exe, b"partial").unwrap();

        unpark(&parked).expect("restore must replace the partial file");
        assert_eq!(std::fs::read(&exe).unwrap(), b"binary bytes");
        assert!(!parked.parked.exists());
    }

    #[test]
    fn sweep_removes_only_matching_parks() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("anvil.exe");
        touch(&exe);
        let stale_a = dir.path().join("anvil.exe.old-111");
        let stale_b = dir.path().join("anvil.exe.old-222-1");
        let unrelated = dir.path().join("anvil.exe.bak");
        let other_tool = dir.path().join("cargo.exe.old-111");
        for p in [&stale_a, &stale_b, &unrelated, &other_tool] {
            touch(p);
        }

        let outcome = sweep_stale_parks(&exe);
        let mut removed = outcome.removed.clone();
        removed.sort();
        assert_eq!(removed, vec![stale_a.clone(), stale_b.clone()]);
        assert!(outcome.still_held.is_empty());
        assert!(!stale_a.exists() && !stale_b.exists());
        assert!(exe.exists(), "the live binary is never swept");
        assert!(unrelated.exists() && other_tool.exists());
    }

    #[test]
    fn sweep_of_unreadable_directory_is_empty_not_fatal() {
        let outcome = sweep_stale_parks(Path::new("/nonexistent/dir/anvil.exe"));
        assert!(outcome.removed.is_empty());
        assert!(outcome.still_held.is_empty());
    }

    #[test]
    fn park_names_are_deterministic_per_attempt() {
        let exe = Path::new("/bin/anvil.exe");
        let first = park_sibling(exe, 42, 0).unwrap();
        let retry = park_sibling(exe, 42, 3).unwrap();
        assert_eq!(first, Path::new("/bin/anvil.exe.old-42"));
        assert_eq!(retry, Path::new("/bin/anvil.exe.old-42-3"));
    }
}
