//! Fixture helpers shared by this crate's unit tests.

/// A temporary directory pinned to `0700`.
///
/// `tempfile::tempdir()` creates directories with the *default*
/// permissions — `0777` masked by the process umask — so a host running
/// the common `umask 002` gets a group-writable `0775` directory.
/// [`crate::write_capsule`] refuses to stage a capsule beneath a parent
/// that another OS identity can write to (`validate_staging_parent`), so
/// every fixture directory that becomes a capsule's parent must pin its
/// mode rather than inherit it. Without this the whole capsule suite
/// passes under `umask 022` and fails under `umask 002`.
///
/// Passing the mode to `mkdir` is umask-safe: a umask can only clear
/// bits, and `0700` has none to clear.
#[cfg(unix)]
pub(crate) fn private_tempdir() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;

    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}

/// Non-Unix hosts have no umask to defeat and no staging-parent identity
/// check, so the plain constructor is already private enough.
#[cfg(not(unix))]
pub(crate) fn private_tempdir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
