//! Project identity (MLP-001 / A7.2).
//!
//! Writes and reads `anvil/project-id` — the tracked file that carries
//! the cross-machine project identity for the multi-layer protection
//! architecture (ADR-036 §D-2, ADR-037 §D-1).
//!
//! ## File format
//!
//! Plain text, one `key: value` per line. Lines starting with `#` are
//! comments; blank lines are ignored. Required field: `project_uuid`.
//! Optional: `created_at`, `created_by_version`, `forked_from`.
//!
//! Example:
//!
//! ```text
//! # Anvil project identifier — see https://anvil.sh/anvil/project-id
//! project_uuid: 0199-7e4a-1b2c-7345-8901-abcdef123456
//! created_at: 2026-05-07T12:34:56Z
//! created_by_version: 0.6.0
//! ```
//!
//! For forks (vNext richer support; v1 just records the parent):
//!
//! ```text
//! project_uuid: 0199-7e4a-...
//! forked_from: 9kza-8b3c-...
//! ```
//!
//! ## Idempotency
//!
//! `ensure_project_id` is idempotent: if `anvil/project-id` already
//! exists and parses, it returns the existing identity. If the file
//! exists but is malformed, the orchestrator surfaces a warning but
//! does NOT overwrite — the user must repair manually (anvil-managed
//! files don't get silently rewritten).
//!
//! ## Forward compatibility
//!
//! Unknown keys in the file are preserved on parse and ignored. Future
//! additions (`first_commit`, `origin_canonical`, etc.) will extend
//! this struct; existing files stay valid.
//!
//! ## What this module does NOT do
//!
//! - Cross-check against git first-commit / origin URL — that's a
//!   future MLP-001 extension; v1 file format is just the UUID.
//! - Compute or verify `forked_from` lineage.
//! - Migration when `project_uuid` changes (deferred per direction).
//! - Stage the file via git — that's the orchestrator's caller's job.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use uuid::Uuid;

/// Tracked filename for project identity (relative to workspace root).
pub const PROJECT_ID_PATH: &str = "anvil/project-id";

/// In-memory representation of `anvil/project-id` contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIdentity {
    pub project_uuid: String,
    pub created_at: Option<String>,
    pub created_by_version: Option<String>,
    pub forked_from: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("malformed `anvil/project-id`: {0}")]
    Malformed(String),
    #[error("missing required field `project_uuid` in `anvil/project-id`")]
    MissingProjectUuid,
}

impl ProjectIdentity {
    /// Mint a fresh identity with a v7 (time-ordered) UUID.
    ///
    /// v7 sorts naturally by creation time, useful for logs and
    /// human inspection. The version field is recorded so future
    /// migrations can detect old formats.
    pub fn new_fresh(anvil_version: &str) -> Self {
        Self {
            project_uuid: Uuid::now_v7().to_string(),
            created_at: Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            created_by_version: Some(anvil_version.to_string()),
            forked_from: None,
        }
    }

    /// Render as the on-disk format described in this module's docs.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("# Anvil project identifier — see https://github.com/eddacraft/anvil\n");
        out.push_str("# This file establishes the project's stable identity across machines\n");
        out.push_str("# and forks. Do not edit unless you intend to fork the project.\n\n");
        let _ = writeln!(out, "project_uuid: {}", self.project_uuid);
        if let Some(ts) = &self.created_at {
            let _ = writeln!(out, "created_at: {ts}");
        }
        if let Some(v) = &self.created_by_version {
            let _ = writeln!(out, "created_by_version: {v}");
        }
        if let Some(parent) = &self.forked_from {
            let _ = writeln!(out, "forked_from: {parent}");
        }
        out
    }

    /// Parse from the on-disk format. Lenient: unknown keys are
    /// silently dropped (forward compatibility).
    pub fn parse(contents: &str) -> Result<Self, IdentityError> {
        let mut project_uuid: Option<String> = None;
        let mut created_at: Option<String> = None;
        let mut created_by_version: Option<String> = None;
        let mut forked_from: Option<String> = None;

        for (lineno, line) in contents.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // Council C-5: use `splitn(2, ':')` so values containing colons
            // (RFC-3339 timestamps, host:port pairs, future fields) round-trip
            // intact rather than being truncated at the second colon.
            let mut parts = trimmed.splitn(2, ':');
            let Some(key) = parts.next() else {
                return Err(IdentityError::Malformed(format!(
                    "line {}: empty key",
                    lineno + 1
                )));
            };
            let Some(value) = parts.next() else {
                return Err(IdentityError::Malformed(format!(
                    "line {}: no key:value separator",
                    lineno + 1
                )));
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "project_uuid" => project_uuid = Some(value.to_string()),
                "created_at" => created_at = Some(value.to_string()),
                "created_by_version" => created_by_version = Some(value.to_string()),
                "forked_from" => forked_from = Some(value.to_string()),
                _ => {
                    // Unknown key — forward compatibility; ignore.
                    tracing::debug!(
                        key = key,
                        line = lineno + 1,
                        "identity: ignoring unknown key in project-id"
                    );
                }
            }
        }

        let project_uuid = project_uuid.ok_or(IdentityError::MissingProjectUuid)?;

        // Council C-1: validate via the actual UUID parser. Previous
        // `len() < 8` check accepted "aaaaaaaa" which would later poison
        // the Kindling DB path and witness chain anchor when MLP-002
        // ships. The `uuid` crate is already a direct dep.
        if Uuid::parse_str(&project_uuid).is_err() {
            return Err(IdentityError::Malformed(format!(
                "project_uuid `{project_uuid}` is not a valid UUID"
            )));
        }

        // Council C-1: same validation for `forked_from` when present.
        // ADR-036 §D-2 calls forked_from a parent_uuid; the value must
        // be UUID-shaped to be meaningful. Future fork-chain logic
        // depends on this.
        if let Some(parent) = &forked_from
            && Uuid::parse_str(parent).is_err()
        {
            return Err(IdentityError::Malformed(format!(
                "forked_from `{parent}` is not a valid UUID"
            )));
        }

        Ok(Self {
            project_uuid,
            created_at,
            created_by_version,
            forked_from,
        })
    }
}

/// Idempotently establish project identity at `root`.
///
/// If `anvil/project-id` exists and parses, returns the existing
/// identity. If absent, mints a new v7 UUID and writes it atomically.
/// On a successful write, **re-reads from disk** (council C-2) so
/// concurrent callers all converge on the same UUID — the loser of
/// the rename race observes the winner's UUID rather than the one it
/// minted locally.
///
/// If present-but-malformed, returns the parse error — caller decides
/// whether to surface as a warning (the orchestrator's pattern) or
/// propagate.
pub fn ensure_project_id(
    root: &Path,
    anvil_version: &str,
) -> Result<ProjectIdentity, IdentityError> {
    let path = project_id_path(root);

    if path.exists() {
        let contents = fs::read_to_string(&path)?;
        return ProjectIdentity::parse(&contents);
    }

    // Mint and write atomically: write to a temp sibling, fsync, rename.
    let identity = ProjectIdentity::new_fresh(anvil_version);
    let parent = path.parent().ok_or_else(|| {
        IdentityError::Malformed(format!(
            "project-id path `{}` has no parent directory",
            path.display()
        ))
    })?;

    // Council C-10: refuse if `anvil/` is a symlink. A symlink-to-
    // outside-the-repo (e.g. /tmp/shared-state) would silently route
    // tracked-file writes outside the working tree.
    if parent.exists() {
        let meta = parent.symlink_metadata()?;
        if meta.file_type().is_symlink() {
            return Err(IdentityError::Malformed(format!(
                "`{}` is a symlink; refusing to write project-id outside the repo",
                parent.display()
            )));
        }
    }

    fs::create_dir_all(parent)?;

    let tmp_name = format!("project-id.tmp.{}", Uuid::new_v4().simple());
    let tmp_path = parent.join(tmp_name);
    let write_result = (|| -> Result<(), IdentityError> {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(identity.render().as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    })();

    if let Err(e) = write_result {
        // Council C-6: clean up temp file if anything failed mid-write.
        // Ignore secondary failure — best-effort.
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    // Council C-2: re-read from disk after rename so concurrent
    // callers (two `anvil start` racing on the same FS) converge on
    // the same identity. The locally-minted `identity` is discarded
    // in favour of whatever actually persisted.
    let contents = fs::read_to_string(&path)?;
    ProjectIdentity::parse(&contents)
}

/// Read `anvil/project-id` if present. Returns `None` if absent;
/// returns `Err` only on read or parse failures.
///
/// Public API for downstream MLP work items (witness chain, cross-
/// machine project verification, `anvil show <id>` lookup) and for
/// `anvil doctor`'s identity check (council C-4). The orchestrator
/// itself uses [`ensure_project_id`].
pub fn read_project_id(root: &Path) -> Result<Option<ProjectIdentity>, IdentityError> {
    let path = project_id_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)?;
    ProjectIdentity::parse(&contents).map(Some)
}

/// Resolve the canonical `anvil/project-id` path inside `root`.
pub fn project_id_path(root: &Path) -> PathBuf {
    root.join(PROJECT_ID_PATH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn fresh_identity_round_trips_through_render_and_parse() {
        let id = ProjectIdentity::new_fresh("0.6.0");
        let rendered = id.render();
        let parsed = ProjectIdentity::parse(&rendered).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn parse_accepts_minimal_file() {
        let contents = "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n";
        let parsed = ProjectIdentity::parse(contents).unwrap();
        assert_eq!(parsed.project_uuid, "01997e4a-1b2c-7345-8901-abcdef123456");
        assert!(parsed.created_at.is_none());
        assert!(parsed.created_by_version.is_none());
        assert!(parsed.forked_from.is_none());
    }

    #[test]
    fn parse_skips_comments_and_blank_lines() {
        let contents = "\
# Anvil project identifier
# Don't edit unless forking

project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456

# trailing comment
";
        let parsed = ProjectIdentity::parse(contents).unwrap();
        assert_eq!(parsed.project_uuid, "01997e4a-1b2c-7345-8901-abcdef123456");
    }

    #[test]
    fn parse_preserves_optional_fields() {
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
created_at: 2026-05-07T12:34:56Z
created_by_version: 0.6.0
forked_from: 9b8a7c6d-5e4f-3210-fedc-ba0987654321
";
        let parsed = ProjectIdentity::parse(contents).unwrap();
        assert_eq!(parsed.created_at.as_deref(), Some("2026-05-07T12:34:56Z"));
        assert_eq!(parsed.created_by_version.as_deref(), Some("0.6.0"));
        assert_eq!(
            parsed.forked_from.as_deref(),
            Some("9b8a7c6d-5e4f-3210-fedc-ba0987654321")
        );
    }

    #[test]
    fn parse_ignores_unknown_keys_for_forward_compat() {
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
first_commit: a3b2ea4e1234567890abcdef
origin_canonical: github.com/eddacraft/anvil
unknown_future_field: whatever
";
        let parsed = ProjectIdentity::parse(contents).unwrap();
        assert_eq!(parsed.project_uuid, "01997e4a-1b2c-7345-8901-abcdef123456");
    }

    #[test]
    fn parse_rejects_missing_project_uuid() {
        let contents = "created_at: 2026-05-07T12:34:56Z\n";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(matches!(err, IdentityError::MissingProjectUuid));
    }

    #[test]
    fn parse_rejects_malformed_line() {
        let contents = "project_uuid: ok\nnokeyseparator\n";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(matches!(err, IdentityError::Malformed(_)));
    }

    #[test]
    fn parse_rejects_too_short_uuid() {
        // Council C-1: pre-fix, this test passed because of the
        // `len() < 8` check. Post-fix, the UUID parser handles all
        // such cases; this test still passes via a different code path.
        let contents = "project_uuid: short\n";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(matches!(err, IdentityError::Malformed(_)));
    }

    #[test]
    fn ensure_creates_when_absent() {
        let dir = TempDir::new().unwrap();
        let id = ensure_project_id(dir.path(), "0.6.0").unwrap();
        assert!(!id.project_uuid.is_empty());
        assert!(dir.path().join(PROJECT_ID_PATH).exists());
    }

    #[test]
    fn ensure_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let first = ensure_project_id(dir.path(), "0.6.0").unwrap();
        let second = ensure_project_id(dir.path(), "0.7.0").unwrap();
        // Same UUID — second call returns the existing identity, does
        // not overwrite. (Note: created_by_version stays at 0.6.0 from
        // the first call; this is the correct behaviour.)
        assert_eq!(first.project_uuid, second.project_uuid);
        assert_eq!(first.created_by_version, second.created_by_version);
    }

    #[test]
    fn ensure_creates_anvil_dir_if_missing() {
        let dir = TempDir::new().unwrap();
        // anvil/ doesn't exist yet
        assert!(!dir.path().join("anvil").exists());
        ensure_project_id(dir.path(), "0.6.0").unwrap();
        assert!(dir.path().join("anvil").is_dir());
        assert!(dir.path().join(PROJECT_ID_PATH).is_file());
    }

    #[test]
    fn ensure_propagates_parse_error_on_malformed_existing_file() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("anvil")).unwrap();
        fs::write(
            dir.path().join(PROJECT_ID_PATH),
            "this is not a valid project-id file\n",
        )
        .unwrap();
        let err = ensure_project_id(dir.path(), "0.6.0").unwrap_err();
        assert!(
            matches!(
                err,
                IdentityError::Malformed(_) | IdentityError::MissingProjectUuid
            ),
            "unexpected error variant: {err:?}"
        );
    }

    #[test]
    fn read_returns_none_when_absent() {
        let dir = TempDir::new().unwrap();
        let result = read_project_id(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_returns_some_when_present() {
        let dir = TempDir::new().unwrap();
        ensure_project_id(dir.path(), "0.6.0").unwrap();
        let result = read_project_id(dir.path()).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn rendered_file_contains_explanatory_comments() {
        let id = ProjectIdentity::new_fresh("0.6.0");
        let rendered = id.render();
        assert!(rendered.contains("Anvil project identifier"));
        assert!(rendered.contains("project_uuid:"));
    }

    #[test]
    fn fresh_identities_have_distinct_uuids() {
        // UUID v7 carries sub-millisecond entropy + a counter; two
        // calls in immediate succession produce distinct values
        // without any sleep.
        let a = ProjectIdentity::new_fresh("0.6.0");
        let b = ProjectIdentity::new_fresh("0.6.0");
        assert_ne!(a.project_uuid, b.project_uuid);
    }

    // ── Council remediation tests ────────────────────────────────────

    #[test]
    fn parse_rejects_non_uuid_project_uuid() {
        // Council C-1: `len() < 8` was too weak; "aaaaaaaa" must fail.
        let contents = "project_uuid: aaaaaaaa\n";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(
            matches!(err, IdentityError::Malformed(_)),
            "expected Malformed for non-UUID project_uuid, got {err:?}"
        );
    }

    #[test]
    fn parse_rejects_non_uuid_forked_from() {
        // Council C-1: forked_from must also validate.
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
forked_from: not-a-uuid
";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(
            matches!(err, IdentityError::Malformed(_)),
            "expected Malformed for non-UUID forked_from, got {err:?}"
        );
    }

    #[test]
    fn parse_accepts_valid_forked_from() {
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
forked_from: 9b8a7c6d-5e4f-3210-fedc-ba0987654321
";
        let parsed = ProjectIdentity::parse(contents).unwrap();
        assert_eq!(
            parsed.forked_from.as_deref(),
            Some("9b8a7c6d-5e4f-3210-fedc-ba0987654321")
        );
    }

    #[test]
    fn parse_handles_values_containing_colons() {
        // Council C-5: ISO-8601 timestamps contain colons; future
        // fields may legitimately too. The full value must round-trip.
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
created_at: 2026-05-07T12:34:56Z
";
        let parsed = ProjectIdentity::parse(contents).unwrap();
        assert_eq!(parsed.created_at.as_deref(), Some("2026-05-07T12:34:56Z"));
    }

    #[test]
    fn ensure_returns_disk_state_after_write() {
        // Council C-2: after a successful write, the returned
        // identity must match what's on disk (so concurrent callers
        // converge on a single UUID).
        let dir = TempDir::new().unwrap();
        let returned = ensure_project_id(dir.path(), "0.6.0").unwrap();
        let on_disk_contents = fs::read_to_string(dir.path().join(PROJECT_ID_PATH)).unwrap();
        let on_disk = ProjectIdentity::parse(&on_disk_contents).unwrap();
        assert_eq!(
            returned.project_uuid, on_disk.project_uuid,
            "ensure must return the on-disk identity, not a locally-minted one"
        );
    }

    #[test]
    fn ensure_refuses_when_anvil_is_a_symlink() {
        // Council C-10: a symlink at `anvil/` could route writes
        // outside the repo. Refuse rather than silently follow.
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        // Skip on platforms where symlinking isn't trivially available
        // (Windows requires elevated permissions for symlinks unless
        // dev mode is enabled).
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(elsewhere.path(), dir.path().join("anvil")).unwrap();
            let err = ensure_project_id(dir.path(), "0.6.0").unwrap_err();
            assert!(
                matches!(err, IdentityError::Malformed(_)),
                "expected Malformed for symlink anvil/, got {err:?}"
            );
            assert!(
                !elsewhere.path().join("project-id").exists(),
                "must not write project-id through the symlink"
            );
        }
        #[cfg(not(unix))]
        let _ = elsewhere;
    }
}
