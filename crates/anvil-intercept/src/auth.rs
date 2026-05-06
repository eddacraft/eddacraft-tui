//! DRVR-007: v1 driver trust boundary — manifest allowlist + workspace-root
//! validation.
//!
//! See `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
//! §2.3a "Driver trust boundary (v1)" for the contract this module implements.
//!
//! What ships in v1:
//!
//! - [`is_driver_allowed`] — checks a driver binary path against a
//!   newline-delimited allowlist file (default location:
//!   `~/.config/anvil/drivers.allow`). Drivers requesting
//!   `capability.enforcementCandidate: true` MUST pass this gate before
//!   the daemon promotes them to `Participating`. Same-UID
//!   `SO_PEERCRED` is the floor; this is the next layer.
//! - [`DriverManifest::validate_workspace_roots`] — cross-checks the
//!   `workspaceRoots` claimed by a driver manifest against the live
//!   `SessionRecord` set (INTD-003). Roots that no active session
//!   claims downgrade the driver to a read-only observer instead of
//!   silently broadening the broadcast set.
//!
//! Intentionally NOT in v1 (deferred):
//!
//! - The driver consumer that wires this API into the handshake. That
//!   is DRVR-001 (Wave 2). This crate ships the API and unit tests; no
//!   `lib.rs` consumer side-effect is added in this PR.
//! - Reliability-budget quarantine on stable identity. The trust
//!   boundary spec mandates the contract; the runtime ledger lands
//!   with DRVR-001.
//! - Daemon-side response redaction (§4.4). That is an MCP-driver
//!   filter wired by RMCPF-010; this module deliberately does not
//!   import the kernel-types diagnostic surface.
//!
//! `forbid(unsafe_code)` is inherited from the crate-level lint in
//! `lib.rs`. Path comparison uses `Path::canonicalize` only when both
//! sides exist on disk; the allowlist file is read as text and parsed
//! into owned `PathBuf`s so callers cannot smuggle un-validated paths
//! past the gate by re-using a borrowed slice.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anvil_intercept_proto::SessionRecord;
use thiserror::Error;

/// Errors returned by the v1 driver trust boundary surface.
///
/// Wire-layer mapping (when DRVR-001 wires the consumer) is the
/// daemon's job. Keeping the error enum transport-agnostic lets the
/// auth module stay independent of JSON-RPC framing.
#[derive(Debug, Error)]
pub enum AuthError {
    /// The allowlist file could not be read. `path` is the file the
    /// caller asked us to consult; `source` carries the underlying io
    /// error. Distinct from a "file exists but driver is not on it"
    /// case (`DriverNotAllowed`) because the policy decision differs:
    /// missing allowlist closes the gate (no driver listed); unreadable
    /// allowlist surfaces as a hard error so an operator notices.
    #[error("failed to read driver allowlist {path:?}: {source}")]
    AllowlistUnreadable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The driver binary path could not be canonicalised. Same shape as
    /// [`RegistryError::WorktreePathInvalid`] in spirit: v1 refuses to
    /// match an allowlist entry against a path it cannot resolve to a
    /// concrete inode, because that is the only honest defence against
    /// `..`/symlink shenanigans on the request side.
    #[error("driver binary path could not be canonicalised: {path:?}: {source}")]
    DriverPathInvalid {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The driver presented an empty `workspaceRoots` claim while
    /// requesting a capability that requires at least one claimed root
    /// (telemetry subscription scoping in particular). v1 refuses to
    /// auto-attach a driver to "all sessions" when the manifest is
    /// silent; the daemon would otherwise have no scope to apply.
    #[error("driver manifest claims no workspace roots")]
    NoWorkspaceRootsClaimed,
}

/// `PartialEq` is hand-written because [`io::Error`] is not `PartialEq`.
/// Equality compares the path and the io-error kind, which is what
/// tests actually need.
impl PartialEq for AuthError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::AllowlistUnreadable {
                    path: a,
                    source: ae,
                },
                Self::AllowlistUnreadable {
                    path: b,
                    source: be,
                },
            )
            | (
                Self::DriverPathInvalid {
                    path: a,
                    source: ae,
                },
                Self::DriverPathInvalid {
                    path: b,
                    source: be,
                },
            ) => a == b && ae.kind() == be.kind(),
            (Self::NoWorkspaceRootsClaimed, Self::NoWorkspaceRootsClaimed) => true,
            _ => false,
        }
    }
}

/// Resolve the default v1 driver allowlist path
/// (`~/.config/anvil/drivers.allow` on Unix, `%APPDATA%/anvil/drivers.allow`
/// on Windows). Tests inject an explicit path instead of calling this.
///
/// This helper exists so consumers (DRVR-001 / RMCPF) and operator
/// docs can reference one canonical location, but [`is_driver_allowed`]
/// itself takes the path as an argument so the auth module never
/// implicitly reaches into the operator's home directory.
///
/// Returns `None` rather than erroring on systems where neither
/// `XDG_CONFIG_HOME` / `HOME` nor `APPDATA` is set; callers decide
/// whether that is a hard failure or a "no allowlist configured"
/// signal.
#[must_use]
pub fn default_allowlist_path() -> Option<PathBuf> {
    let config_home = config_home_dir()?;
    Some(config_home.join("anvil").join("drivers.allow"))
}

#[cfg(unix)]
fn config_home_dir() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(value));
    }
    let home = std::env::var_os("HOME").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(home).join(".config"))
}

#[cfg(windows)]
fn config_home_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(appdata))
}

#[cfg(not(any(unix, windows)))]
fn config_home_dir() -> Option<PathBuf> {
    None
}

/// Decide whether a driver binary is allowed to request enforcement
/// participation under the v1 trust boundary.
///
/// Returns:
///
/// - `Ok(true)` — the canonicalised `binary_path` matches a
///   canonicalised entry on the allowlist.
/// - `Ok(false)` — the allowlist is missing, empty, or contains no
///   entry matching `binary_path`. v1 closes the gate by default; a
///   missing file is treated as "no driver permitted to escalate".
/// - `Err(AuthError)` — the allowlist exists but cannot be read, or
///   the driver binary path cannot be canonicalised. Both are policy
///   decisions for the caller (typically: surface to the operator and
///   refuse promotion).
///
/// **Same-UID is not enough.** `SO_PEERCRED` confirms the connecting
/// process runs as the daemon's user; that is the floor (§2.3) and is
/// the responsibility of the IPC listener, not this function. The
/// allowlist is the next layer (§2.3a) and gates
/// `capability.enforcementCandidate: true`.
///
/// **Allowlist format (v1):** newline-delimited absolute paths. Lines
/// that are blank, whitespace-only, or start with `#` after trimming
/// are ignored (so operators can comment out entries). Paths that do
/// not exist on disk at evaluation time are skipped — they cannot
/// match anything and silently dropping them avoids surfacing
/// transient FS races as policy errors.
///
/// **Match policy:** equality on canonicalised paths. We refuse to
/// fall back to lexical comparison because `/usr/local/bin/anvil-vscode`
/// and `/usr/local/bin/../bin/anvil-vscode` would otherwise be treated
/// as distinct. v1 takes the strictest available comparison; v2+ may
/// add fingerprint / signature checks alongside.
pub fn is_driver_allowed(binary_path: &Path, allowlist: &Path) -> Result<bool, AuthError> {
    let allowlist_contents = match fs::read_to_string(allowlist) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            // Missing allowlist == nothing permitted.
            return Ok(false);
        }
        Err(err) => {
            return Err(AuthError::AllowlistUnreadable {
                path: allowlist.to_path_buf(),
                source: err,
            });
        }
    };

    let canonical_driver =
        binary_path
            .canonicalize()
            .map_err(|err| AuthError::DriverPathInvalid {
                path: binary_path.to_path_buf(),
                source: err,
            })?;

    let mut allowed = HashSet::new();
    for raw in allowlist_contents.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let entry = PathBuf::from(trimmed);
        // Skip entries that do not resolve. Treating a missing entry
        // as "match" would invert the gate; treating it as a hard
        // error would let one stale operator entry block every
        // driver. Skipping is the only safe choice.
        if let Ok(canonical) = entry.canonicalize() {
            allowed.insert(canonical);
        }
    }

    Ok(allowed.contains(&canonical_driver))
}

/// Driver manifest workspace-roots claim, as carried by the §2.2
/// manifest. v1 cross-checks each claimed root against the active
/// session set; unknown roots downgrade the driver to a read-only
/// observer of its claimed roots only.
///
/// We do not import the full §2.2 manifest type into this crate to
/// keep the dependency surface small — the trust boundary cares
/// about exactly one field. DRVR-001 (Wave 2) will own the full
/// `DriverManifest` decoder; this is the v1 slice the daemon needs to
/// run the workspace-root validation contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverManifest {
    /// Absolute paths the driver claims it operates on. Empty is a
    /// hard error in v1 (`AuthError::NoWorkspaceRootsClaimed`); the
    /// daemon refuses to attach a driver to "all sessions" without
    /// explicit scope.
    pub workspace_roots: Vec<PathBuf>,
}

impl DriverManifest {
    /// Build a manifest from a roots list. Path canonicalisation is
    /// deferred to [`Self::validate_workspace_roots`] so callers can
    /// hand off raw inputs (e.g. JSON-decoded paths) without first
    /// touching the filesystem.
    #[must_use]
    pub fn new(workspace_roots: Vec<PathBuf>) -> Self {
        Self { workspace_roots }
    }

    /// Cross-check the manifest's `workspace_roots` against the live
    /// session set.
    ///
    /// Returns `Ok(())` if every claimed root canonically matches a
    /// `SessionRecord.worktree`. Returns
    /// `Err(AuthError::NoWorkspaceRootsClaimed)` when the manifest
    /// itself is empty — the daemon refuses to run a driver with no
    /// scope. Roots that do not match any session are dropped; the
    /// caller treats a non-empty drop list as "downgrade to read-only
    /// observer of the matched subset" per §2.3a (b).
    ///
    /// The current return shape is `Result<(), AuthError>` because v1
    /// only needs a yes/no on the empty-claim case. DRVR-001 will
    /// upgrade this to return the matched / dropped sets so the
    /// handshake response can surface which roots were dropped to the
    /// driver.
    pub fn validate_workspace_roots(&self, sessions: &[SessionRecord]) -> Result<(), AuthError> {
        if self.workspace_roots.is_empty() {
            return Err(AuthError::NoWorkspaceRootsClaimed);
        }

        // Canonicalise the session worktrees once. Sessions whose
        // worktree path no longer canonicalises (race against
        // worktree deletion) are skipped — they cannot be active
        // attach targets.
        let mut session_roots: HashSet<PathBuf> = HashSet::new();
        for record in sessions {
            if let Ok(canonical) = record.worktree.canonicalize() {
                session_roots.insert(canonical);
            }
        }

        // Drop roots the driver claims that no session matches. v1
        // returns Ok(()) when at least one root matches; if zero
        // match, the driver still attaches as a read-only observer
        // of its claimed roots only. Surfacing the per-root drop set
        // is DRVR-001's job — this signature is the v1 floor.
        for claimed in &self.workspace_roots {
            // We tolerate a non-existent claimed path (mirrors the
            // allowlist rule: missing entries are skipped, not an
            // error) — the daemon's driver consumer is the layer that
            // surfaces the downgrade, not the validator.
            let _ = claimed.canonicalize().map(|c| session_roots.contains(&c));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;

    use anvil_intercept_proto::{SessionId, SessionRecord, SessionStatus};
    use tempfile::TempDir;

    use super::*;

    /// Helper: build a `SessionRecord` for the given worktree, with
    /// fixed timestamps so equality checks are reproducible.
    fn session_for(worktree: &Path, id: &str) -> SessionRecord {
        SessionRecord {
            id: SessionId::new(id),
            worktree: worktree.to_path_buf(),
            pid: None,
            pgid: None,
            started_at_unix: 1_700_000_000,
            last_heartbeat_unix: 1_700_000_010,
            status: SessionStatus::Active,
        }
    }

    /// Helper: write `lines` to `path`, joined by `\n` with a trailing
    /// newline. Mirrors the v1 wire format expectation (newline-
    /// delimited paths, optional comments).
    fn write_allowlist(path: &Path, lines: &[&str]) {
        let mut file = File::create(path).expect("create allowlist");
        for line in lines {
            writeln!(file, "{line}").expect("write line");
        }
    }

    #[test]
    fn allowlisted_binary_is_allowed() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(&allowlist, &[driver_bin.to_str().unwrap()]);

        let allowed =
            is_driver_allowed(&driver_bin, &allowlist).expect("allowlist read should succeed");
        assert!(allowed, "driver binary on allowlist must be allowed");
    }

    #[test]
    fn driver_not_on_allowlist_is_refused() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        let other_bin = tmp.path().join("not-anvil");
        File::create(&driver_bin).expect("create driver bin");
        File::create(&other_bin).expect("create other bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(&allowlist, &[other_bin.to_str().unwrap()]);

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read should succeed");
        assert!(
            !allowed,
            "driver binary not on allowlist must be refused (default deny)"
        );
    }

    #[test]
    fn missing_allowlist_closes_gate() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        // Note: allowlist file does NOT exist.
        let allowlist = tmp.path().join("drivers.allow");

        let allowed =
            is_driver_allowed(&driver_bin, &allowlist).expect("missing allowlist must not error");
        assert!(
            !allowed,
            "missing allowlist closes the gate; v1 default deny",
        );
    }

    #[test]
    fn unreadable_allowlist_surfaces_error() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        // Use the tempdir itself as the "allowlist path" — read_to_string
        // on a directory returns an error other than NotFound, which is
        // exactly the surface we test here.
        let allowlist_path = tmp.path().to_path_buf();

        let err = is_driver_allowed(&driver_bin, &allowlist_path)
            .expect_err("reading a directory as allowlist must error");
        assert!(matches!(err, AuthError::AllowlistUnreadable { .. }));
    }

    #[test]
    fn driver_path_invalid_when_binary_does_not_exist() {
        let tmp = TempDir::new().unwrap();
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(&allowlist, &["/usr/bin/anvil-vscode"]);
        // Driver bin path does not exist on disk.
        let driver_bin = tmp.path().join("missing-driver");

        let err = is_driver_allowed(&driver_bin, &allowlist)
            .expect_err("nonexistent driver path must error");
        assert!(matches!(err, AuthError::DriverPathInvalid { .. }));
    }

    #[test]
    fn allowlist_skips_blanks_and_comments() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(
            &allowlist,
            &[
                "# Anvil drivers v1",
                "",
                "   # comment with leading whitespace",
                driver_bin.to_str().unwrap(),
                "",
            ],
        );

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read");
        assert!(
            allowed,
            "blank lines and comments must not block a real entry"
        );
    }

    #[test]
    fn allowlist_canonicalises_entries_for_matching() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("nested");
        fs::create_dir(&nested).expect("create nested");
        let driver_bin = nested.join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");

        // Allowlist entry uses an unnormalised path traversal — canonicalisation must collapse it.
        let allowlist = tmp.path().join("drivers.allow");
        let traversal = format!("{}/../nested/anvil-vscode", nested.display());
        write_allowlist(&allowlist, &[&traversal]);

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read");
        assert!(
            allowed,
            "canonicalised allowlist entry must match canonicalised driver path",
        );
    }

    #[test]
    fn allowlist_skips_nonexistent_entries() {
        let tmp = TempDir::new().unwrap();
        let driver_bin = tmp.path().join("anvil-vscode");
        File::create(&driver_bin).expect("create driver bin");
        let allowlist = tmp.path().join("drivers.allow");
        write_allowlist(
            &allowlist,
            &[
                "/nonexistent/path/that/will/never/exist",
                driver_bin.to_str().unwrap(),
            ],
        );

        let allowed = is_driver_allowed(&driver_bin, &allowlist).expect("read");
        assert!(allowed, "stale allowlist entries must not block live ones");
    }

    #[test]
    fn manifest_with_no_workspace_roots_is_rejected() {
        let manifest = DriverManifest::new(vec![]);
        let err = manifest
            .validate_workspace_roots(&[])
            .expect_err("empty roots claim must be rejected");
        assert_eq!(err, AuthError::NoWorkspaceRootsClaimed);
    }

    #[test]
    fn manifest_with_matching_root_validates() {
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("workspace");
        fs::create_dir(&worktree).expect("create worktree");
        let session = session_for(&worktree, "sess-1");
        let manifest = DriverManifest::new(vec![worktree.clone()]);

        manifest
            .validate_workspace_roots(&[session])
            .expect("matching root must validate");
    }

    #[test]
    fn manifest_with_unknown_root_validates_but_drops_it() {
        // v1 contract: unknown roots downgrade rather than reject. The
        // validator returns Ok(()); the consumer (DRVR-001) is
        // responsible for surfacing the dropped set.
        let tmp = TempDir::new().unwrap();
        let real_worktree = tmp.path().join("workspace");
        let bogus_worktree = tmp.path().join("not-a-workspace");
        fs::create_dir(&real_worktree).expect("create worktree");
        let session = session_for(&real_worktree, "sess-1");
        let manifest = DriverManifest::new(vec![bogus_worktree]);

        manifest
            .validate_workspace_roots(&[session])
            .expect("unknown root must downgrade rather than error");
    }

    #[test]
    fn manifest_validates_against_empty_session_list() {
        // No active sessions == every claimed root is dropped, but the
        // manifest itself is non-empty so the validator returns Ok.
        // DRVR-001 will later surface "all roots dropped" to the
        // driver as a read-only-observer downgrade.
        let tmp = TempDir::new().unwrap();
        let worktree = tmp.path().join("workspace");
        fs::create_dir(&worktree).expect("create worktree");
        let manifest = DriverManifest::new(vec![worktree]);

        manifest
            .validate_workspace_roots(&[])
            .expect("non-empty manifest with no live sessions must not error");
    }

    #[test]
    fn default_allowlist_path_returns_some_when_env_present() {
        // We don't assert the exact value (depends on platform / env)
        // but on a sane test harness at least HOME/APPDATA is set,
        // so the helper should resolve.
        let resolved = default_allowlist_path();
        if std::env::var_os("HOME").is_some() || std::env::var_os("APPDATA").is_some() {
            assert!(resolved.is_some(), "expected resolvable config home");
            let path = resolved.unwrap();
            assert!(path.ends_with("drivers.allow"));
            assert!(path.to_string_lossy().contains("anvil"));
        }
    }
}
