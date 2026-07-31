//! Project identity (MLP-001): read/write `anvil/project-id`.
//!
//! Plain `key: value` text; required `project_uuid`. [`ensure_project_id`] is
//! idempotent and never overwrites a malformed file. Unknown keys ignored on
//! parse (not round-tripped). Optional composite check via
//! [`ProjectIdentity::verify_against_worktree`] (MLP2-003).

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
    /// MLP2-003: root commit SHA (`git rev-list --max-parents=0
    /// HEAD`). Persisted at activation; verified against the live
    /// worktree on daemon attach. `None` for projects activated
    /// before MLP2-003 shipped — files without this field still
    /// parse cleanly.
    pub first_commit: Option<String>,
    /// MLP2-003: canonicalised `remote.origin.url`. The canonical
    /// form normalises scheme/host casing, strips the optional `.git`
    /// suffix, strips trailing slashes, and folds the `git@host:owner/repo`
    /// SSH alias into `host:owner/repo`. `None` for projects that
    /// haven't been pushed to a remote yet, or for files activated
    /// before MLP2-003 shipped.
    pub origin_canonical: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("malformed `anvil/project-id`: {0}")]
    Malformed(String),
    #[error("missing required field `project_uuid` in `anvil/project-id`")]
    MissingProjectUuid,
    /// MLP2-003: running `git` to read worktree state failed. Distinct
    /// from `Malformed` so callers can distinguish "file says X, git
    /// says Y" from "git itself broke". Construction site lands with
    /// MLP2-025 (registry-side wiring); the `#[allow(dead_code)]`
    /// goes away then.
    #[allow(dead_code)]
    #[error("git invocation failed in `{worktree}`: {message}")]
    GitInvocationFailed { worktree: PathBuf, message: String },
}

/// MLP2-003: outcome of verifying a persisted `ProjectIdentity` against
/// the live worktree. The daemon's attach path consumes this enum and
/// maps `Mismatch` onto the wire-level `degraded:identity-mismatch`
/// signal; `ForkedFromParent` is accepted as a clean attach.
///
/// No in-crate caller yet — [`attach_check`] is the public entry the
/// daemon's IPC handler will pick up in MLP2-025.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityCheck {
    /// Every recorded field (UUID, `first_commit`, `origin_canonical`)
    /// agrees with the live worktree. The cleanest possible outcome.
    Match,
    /// `forked_from` was set in the persisted file. The fork-aware
    /// path attaches without degradation: the operator has explicitly
    /// recorded the parent project's identity, so the live git state
    /// matching the recorded fork-side identity is sufficient.
    /// `parent_uuid` is echoed back so the daemon can record which
    /// fork lineage the session belongs to.
    ForkedFromParent { parent_uuid: String },
    /// One or more recorded fields disagreed with the live worktree.
    /// `reasons` is non-empty and lists the specific mismatches; the
    /// caller surfaces this as `degraded:identity-mismatch`.
    Mismatch { reasons: Vec<IdentityMismatch> },
}

/// MLP2-003: a single mismatched field. Surface for MLP2-025
/// (registry-side wiring); `#[allow(dead_code)]` until then.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityMismatch {
    /// The recorded `first_commit` did not match the live root commit.
    /// Most likely cause: a rebase or a fresh checkout from an
    /// unrelated repo with the same `anvil/project-id` copied in.
    FirstCommit { recorded: String, live: String },
    /// The recorded `origin_canonical` did not match the live remote.
    /// Most likely cause: the operator renamed the origin (e.g.
    /// migrated GitHub → GitLab) without updating `anvil/project-id`.
    OriginCanonical {
        recorded: String,
        live: Option<String>,
    },
}

impl std::fmt::Display for IdentityMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstCommit { recorded, live } => write!(
                f,
                "first_commit recorded `{recorded}` but worktree HEAD's root commit is `{live}`"
            ),
            Self::OriginCanonical { recorded, live } => match live {
                Some(live) => write!(
                    f,
                    "origin_canonical recorded `{recorded}` but worktree origin canonicalises to `{live}`"
                ),
                None => write!(
                    f,
                    "origin_canonical recorded `{recorded}` but worktree has no `remote.origin.url` configured"
                ),
            },
        }
    }
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
            first_commit: None,
            origin_canonical: None,
        }
    }

    /// Render as the on-disk format described in this module's docs.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("# anvil project identifier — see https://github.com/eddacraft/anvil\n");
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
        if let Some(commit) = &self.first_commit {
            let _ = writeln!(out, "first_commit: {commit}");
        }
        if let Some(origin) = &self.origin_canonical {
            let _ = writeln!(out, "origin_canonical: {origin}");
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
        let mut first_commit: Option<String> = None;
        let mut origin_canonical: Option<String> = None;

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
                "first_commit" => first_commit = Some(value.to_string()),
                "origin_canonical" => origin_canonical = Some(value.to_string()),
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

        // MLP2-003: validate first_commit shape — must be 40-char
        // lowercase hex (a full SHA-1 git OID). Shorter / mixed-case
        // / non-hex forms would silently fail to compare against
        // `git rev-list` output, masking real mismatches.
        if let Some(commit) = &first_commit
            && !is_full_git_sha1(commit)
        {
            return Err(IdentityError::Malformed(format!(
                "first_commit `{commit}` is not a 40-character lowercase hex SHA-1"
            )));
        }
        // MLP2-003: origin_canonical shape — non-empty, no
        // surrounding whitespace, no control characters. We do NOT
        // re-canonicalise here because that's the caller's job; we
        // only refuse obviously-bad values that would never match a
        // canonicalise() output.
        if let Some(origin) = &origin_canonical
            && !is_valid_canonical_origin(origin)
        {
            return Err(IdentityError::Malformed(format!(
                "origin_canonical `{origin}` is empty, has surrounding whitespace, \
                 or contains control characters"
            )));
        }

        Ok(Self {
            project_uuid,
            created_at,
            created_by_version,
            forked_from,
            first_commit,
            origin_canonical,
        })
    }

    /// MLP2-003: cross-check the persisted identity against the live
    /// worktree's git state.
    ///
    /// Reads:
    /// - Root commit SHA via `git rev-list --max-parents=0 HEAD`.
    ///   A worktree with no commits yet is treated as a `Match`
    ///   when `first_commit` is `None`, otherwise as a
    ///   `Mismatch::FirstCommit` with an empty `live` value.
    /// - Origin URL via `git config --get remote.origin.url`,
    ///   then canonicalised via [`canonicalise_origin`]. A worktree
    ///   with no origin configured matches `origin_canonical: None`,
    ///   otherwise yields `Mismatch::OriginCanonical { live: None }`.
    ///
    /// Returns [`IdentityCheck::Match`] when every recorded field
    /// agrees, [`IdentityCheck::ForkedFromParent`] when
    /// `forked_from` is set (the operator has declared a fork —
    /// the daemon attaches without degradation, MLP2-003 §Fork
    /// detection), and [`IdentityCheck::Mismatch`] otherwise.
    #[allow(dead_code)]
    pub fn verify_against_worktree(&self, worktree: &Path) -> Result<IdentityCheck, IdentityError> {
        let live_first_commit = read_first_commit(worktree)?;
        let live_origin_canonical = read_origin_canonical(worktree)?;

        let mut reasons: Vec<IdentityMismatch> = Vec::new();

        match (&self.first_commit, &live_first_commit) {
            (Some(recorded), Some(live)) if recorded != live => {
                reasons.push(IdentityMismatch::FirstCommit {
                    recorded: recorded.clone(),
                    live: live.clone(),
                });
            }
            (Some(recorded), None) => {
                reasons.push(IdentityMismatch::FirstCommit {
                    recorded: recorded.clone(),
                    live: String::new(),
                });
            }
            // (None, _) and (Some==Some) are both Match for this
            // field. `recorded == None` means activation pre-dated
            // MLP2-003; we cannot cross-check it.
            _ => {}
        }

        match (&self.origin_canonical, &live_origin_canonical) {
            (Some(recorded), Some(live)) if recorded != live => {
                reasons.push(IdentityMismatch::OriginCanonical {
                    recorded: recorded.clone(),
                    live: Some(live.clone()),
                });
            }
            (Some(recorded), None) => {
                reasons.push(IdentityMismatch::OriginCanonical {
                    recorded: recorded.clone(),
                    live: None,
                });
            }
            _ => {}
        }

        if reasons.is_empty() {
            return Ok(IdentityCheck::Match);
        }

        // Fork-detection rule: if `forked_from` is set the operator
        // has declared a fork, so the daemon accepts the attach
        // without degradation. The recorded identity is taken at
        // face value for this session; cross-checking the parent
        // project's `first_commit` is a future task.
        if let Some(parent_uuid) = &self.forked_from {
            return Ok(IdentityCheck::ForkedFromParent {
                parent_uuid: parent_uuid.clone(),
            });
        }

        Ok(IdentityCheck::Mismatch { reasons })
    }
}

/// MLP2-003: canonicalise a `remote.origin.url` into a stable
/// comparison form.
///
/// The git remote URL ecosystem allows several spellings for the same
/// remote — HTTPS vs SSH alias, `.git` suffix optional, trailing
/// slashes optional, scheme/host casing nondeterministic across tools.
/// All of those reduce to the same canonical string here so the
/// comparison in [`ProjectIdentity::verify_against_worktree`] does
/// not flag a benign reformat as a mismatch.
///
/// Canonical form: `host/owner/repo` for known forges
/// (`github.com`, `gitlab.com`, `bitbucket.org`, generic forge URLs)
/// after the following normalisations:
/// - `git@host:owner/repo[.git]` → `host/owner/repo`
/// - `ssh://git@host/owner/repo[.git]` → `host/owner/repo`
/// - `https://host/owner/repo[.git]` (or `http://`) → `host/owner/repo`
/// - Any trailing `/`, `.git`, or whitespace is stripped.
/// - Scheme and host are lowercased; path case is preserved
///   (GitHub paths are case-insensitive in routing but case-preserving
///   in display, and `git remote get-url` echoes whatever the operator
///   typed — preserving case keeps a renamed-to-different-case repo
///   visible as a mismatch rather than silently merged).
///
/// An empty or whitespace-only input returns the empty string; the
/// caller decides whether that counts as "no origin" or "malformed".
#[allow(dead_code)]
#[must_use]
pub fn canonicalise_origin(url: &str) -> String {
    let raw = url.trim();
    if raw.is_empty() {
        return String::new();
    }

    // SSH alias `git@host:path` (no `://`). Identifiable by the
    // leading `git@` plus a colon, with NO double-slash following
    // the colon (those would be a real URL).
    if let Some(rest) = raw.strip_prefix("git@")
        && let Some((host, path)) = rest.split_once(':')
        && !path.starts_with("//")
    {
        return canonicalise_host_path(host, path);
    }

    // URL with scheme. Strip scheme + optional userinfo, then split
    // host from path.
    let after_scheme = if let Some((scheme, rest)) = raw.split_once("://") {
        let _scheme_lower = scheme.to_ascii_lowercase();
        rest
    } else {
        raw
    };
    // Strip userinfo (e.g. `git@`).
    let after_user = after_scheme
        .split_once('@')
        .map_or(after_scheme, |(_, r)| r);
    if let Some((host, path)) = after_user.split_once('/') {
        return canonicalise_host_path(host, path);
    }

    // Couldn't extract a host/path pair — return the trimmed form so
    // an exact recorded match still works.
    raw.to_string()
}

#[allow(dead_code)]
fn canonicalise_host_path(host: &str, path: &str) -> String {
    let host = host.trim().to_ascii_lowercase();
    let path = path.trim().trim_start_matches('/').trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        host
    } else {
        format!("{host}/{path}")
    }
}

/// MLP2-003: run `git rev-list --max-parents=0 HEAD` in `worktree`
/// and return the single resulting SHA. Returns `Ok(None)` when the
/// worktree has no commits yet (the command exits non-zero with a
/// recognised "unknown revision" stderr); other failures surface as
/// `IdentityError::GitInvocationFailed`.
#[allow(dead_code)]
pub fn read_first_commit(worktree: &Path) -> Result<Option<String>, IdentityError> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(["rev-list", "--max-parents=0", "HEAD"])
        .output()
        .map_err(|err| IdentityError::GitInvocationFailed {
            worktree: worktree.to_path_buf(),
            message: format!("spawning `git rev-list`: {err}"),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Empty repos and just-init'd worktrees report "unknown
        // revision or path not in the working tree" or "fatal: bad
        // revision 'HEAD'". Both are "no commits yet" — treat as None.
        let lower = stderr.to_ascii_lowercase();
        if lower.contains("unknown revision")
            || lower.contains("bad revision")
            || lower.contains("does not have any commits")
            || lower.contains("not a git repository")
        {
            return Ok(None);
        }
        return Err(IdentityError::GitInvocationFailed {
            worktree: worktree.to_path_buf(),
            message: format!("`git rev-list` exited non-zero: {stderr}"),
        });
    }
    // `--max-parents=0` returns ONE sha per root commit. Octopus
    // merges and shallow clones can produce more than one root —
    // we take the first line (sorted by topology, which is stable
    // for a given history) and rely on the operator-side fork
    // semantics to flag the rare "history was rewritten to add a
    // new root" case as a mismatch.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().next().map_or("", str::trim);
    if first.is_empty() {
        return Ok(None);
    }
    if !is_full_git_sha1(first) {
        return Err(IdentityError::GitInvocationFailed {
            worktree: worktree.to_path_buf(),
            message: format!("`git rev-list` returned unexpected output: {first}"),
        });
    }
    Ok(Some(first.to_string()))
}

/// MLP2-003: run `git config --get remote.origin.url` in `worktree`,
/// canonicalise the result, and return it. `Ok(None)` when origin
/// is not configured (the command exits 1 with no output); other
/// failures surface as `IdentityError::GitInvocationFailed`.
#[allow(dead_code)]
pub fn read_origin_canonical(worktree: &Path) -> Result<Option<String>, IdentityError> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .map_err(|err| IdentityError::GitInvocationFailed {
            worktree: worktree.to_path_buf(),
            message: format!("spawning `git config`: {err}"),
        })?;
    if !output.status.success() {
        // git config --get returns 1 (and empty stdout) when the
        // key is not set. Treat that path as "no origin configured"
        // rather than a hard error so freshly-`git init`ed worktrees
        // do not trip the daemon's verify.
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.stderr.is_empty() || stderr.trim().is_empty() {
            return Ok(None);
        }
        if stderr.to_ascii_lowercase().contains("not a git repository") {
            return Ok(None);
        }
        return Err(IdentityError::GitInvocationFailed {
            worktree: worktree.to_path_buf(),
            message: format!("`git config` exited non-zero: {stderr}"),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.lines().next().map_or("", str::trim);
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(canonicalise_origin(trimmed)))
}

fn is_full_git_sha1(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn is_valid_canonical_origin(value: &str) -> bool {
    !value.is_empty() && value == value.trim() && !value.bytes().any(|b| b.is_ascii_control())
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

    // Council C-10 / Copilot review: refuse if `anvil/` is a symlink.
    // We check before AND after `create_dir_all` to close the TOCTOU
    // window where a same-UID attacker could replace the directory
    // with a symlink between our pre-check and our write.
    refuse_if_symlink(parent)?;
    fs::create_dir_all(parent)?;
    refuse_if_symlink(parent)?;

    let tmp_name = format!("project-id.tmp.{}", Uuid::new_v4().simple());
    let tmp_path = parent.join(tmp_name);
    let write_result = (|| -> Result<(), IdentityError> {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(identity.render().as_bytes())?;
        f.sync_all()?;
        // Copilot review: handle the concurrent-creation race
        // explicitly. On Windows, `fs::rename` fails with
        // ErrorKind::AlreadyExists if another process raced us to
        // create the file; on Unix it overwrites silently. In both
        // cases we want to converge on the on-disk identity, so we
        // treat AlreadyExists as success and let the post-rename
        // re-read return whichever UUID won.
        match fs::rename(&tmp_path, &path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(e.into()),
        }
    })();

    if let Err(e) = write_result {
        // Council C-6: clean up temp file if anything failed mid-write.
        // Ignore secondary failure — best-effort.
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    // If the AlreadyExists path was taken, the temp file is still on
    // disk. Clean it up best-effort.
    if tmp_path.exists() {
        let _ = fs::remove_file(&tmp_path);
    }

    // Council C-2: re-read from disk after rename so concurrent
    // callers (two `anvil start` racing on the same FS) converge on
    // the same identity. The locally-minted `identity` is discarded
    // in favour of whatever actually persisted.
    let contents = fs::read_to_string(&path)?;
    ProjectIdentity::parse(&contents)
}

/// MLP2-033: mint a fresh project identity, recording the previous
/// `project_uuid` (if any) as `forked_from`. Always writes — this is
/// the explicit-operator-intent counterpart to [`ensure_project_id`]'s
/// idempotent read.
///
/// `--new-identity` on `anvil start` and `anvil baseline` flows
/// through here. The expected fork tree:
///
/// - **Parent** project: `project_uuid = A`, no `forked_from`.
/// - **Child** clone (no flag): inherits `A` via `ensure_project_id`
///   — the on-disk file is checked in and survives `git clone`.
/// - **Grandchild** clone of the child (with `--new-identity`):
///   mints fresh `project_uuid = B`, records `forked_from = A`. The
///   chain is single-deep — `forked_from` carries the immediate
///   parent only, not a list. Re-running `--new-identity` mints
///   *another* fresh UUID and overwrites `forked_from` with the most
///   recent UUID (lossy, by design — operator's explicit intent each
///   time).
///
/// Same TOCTOU + symlink-refusal pattern as [`ensure_project_id`],
/// with one extra check: the project-id file *itself* is also
/// refused if it's a symlink, since the overwrite would otherwise
/// follow the link out of the repo (asymmetric with
/// [`ensure_project_id`], which only ever creates a fresh file and
/// so only needs to guard the parent directory).
///
/// The atomic temp-then-rename overwrites any existing file on both
/// POSIX (rename replaces) and Windows (modern `std::fs::rename`
/// passes `MOVEFILE_REPLACE_EXISTING`). After the rename the file is
/// re-read from disk so concurrent `--new-identity` callers converge
/// on the same persisted identity (council C-2 pattern). Note that
/// `parent_uuid` is captured *before* the symlink checks and rename,
/// so in a concurrent-mint race the loser's `forked_from` records
/// the parent UUID it observed at read-time — which the winning
/// rename has since overwritten. The losing write is itself
/// overwritten by the winner, so the on-disk state is always
/// consistent (no diverged `forked_from`); the race is documented
/// here purely for future readers.
pub fn mint_new_identity(
    root: &Path,
    anvil_version: &str,
) -> Result<ProjectIdentity, IdentityError> {
    let path = project_id_path(root);

    // Capture the previous UUID before we overwrite. Read failures
    // and parse failures are silently treated as "no parent" — the
    // operator's explicit intent is to detach from whatever was
    // there, so a malformed predecessor file shouldn't block the
    // mint. (`ensure_project_id` propagates parse errors instead;
    // the asymmetry is deliberate — `--new-identity` is destructive
    // by design.)
    let parent_uuid = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| ProjectIdentity::parse(&s).ok())
            .map(|p| p.project_uuid)
    } else {
        None
    };

    let parent = path.parent().ok_or_else(|| {
        IdentityError::Malformed(format!(
            "project-id path `{}` has no parent directory",
            path.display()
        ))
    })?;

    refuse_if_symlink(parent)?;
    fs::create_dir_all(parent)?;
    refuse_if_symlink(parent)?;
    refuse_if_symlink(&path)?;

    let mut identity = ProjectIdentity::new_fresh(anvil_version);
    identity.forked_from = parent_uuid;

    let tmp_name = format!("project-id.tmp.{}", Uuid::new_v4().simple());
    let tmp_path = parent.join(tmp_name);
    let write_result = (|| -> Result<(), IdentityError> {
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(identity.render().as_bytes())?;
        f.sync_all()?;
        // `std::fs::rename` is atomic-replace on POSIX and (since
        // Rust 1.66) on Windows via `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`.
        // Unlike `ensure_project_id`'s rename, we *want* to overwrite
        // here — that is the entire point of `--new-identity`.
        fs::rename(&tmp_path, &path)?;
        Ok(())
    })();

    if let Err(e) = write_result {
        // Council quick #C-2 (MINOR): surface temp-file cleanup
        // failures to tracing so a stale `project-id.tmp.<uuid>`
        // after a disk-full mid-rename isn't silently abandoned.
        // The error from `fs::remove_file` is otherwise swallowed
        // (best-effort), which on a long-lived dev tree leads to
        // an accumulation of orphaned temp files inside `anvil/`.
        if let Err(rm_err) = fs::remove_file(&tmp_path) {
            tracing::warn!(
                error = %rm_err,
                path = %tmp_path.display(),
                "mint_new_identity: failed to clean up temp file after write error",
            );
        }
        return Err(e);
    }

    // Convergence: re-read so two racing `--new-identity` calls both
    // observe whichever UUID actually persisted (council C-2 pattern,
    // mirrors `ensure_project_id`).
    let contents = fs::read_to_string(&path)?;
    ProjectIdentity::parse(&contents)
}

/// Refuse if `path` exists and is a symlink. Used twice in
/// `ensure_project_id` to close the TOCTOU window between the
/// pre-check and the write.
fn refuse_if_symlink(path: &Path) -> Result<(), IdentityError> {
    if path.exists() {
        let meta = path.symlink_metadata()?;
        if meta.file_type().is_symlink() {
            return Err(IdentityError::Malformed(format!(
                "`{}` is a symlink; refusing to write project-id outside the repo",
                path.display()
            )));
        }
    }
    Ok(())
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

/// MLP2-003: outcome of the daemon attach identity check. The full
/// "attach reads `anvil/project-id` + git" sequence collapses into
/// one of these variants; callers map them onto the wire-level
/// `degraded:identity-mismatch` surface or onto a hard rejection.
///
/// Variants:
///
/// - [`AttachStatus::Clean`] — every recorded field matched; the
///   attach proceeds without degradation.
/// - [`AttachStatus::Fork`] — `forked_from` was set, so the operator
///   has declared a fork and the daemon attaches without
///   degradation; `parent_uuid` is echoed so the daemon can record
///   the fork lineage on the session.
/// - [`AttachStatus::Mismatch`] — one or more recorded fields
///   disagreed with the live worktree; surface as
///   `degraded:identity-mismatch`. The wire-level signal name is
///   carried in [`Self::DEGRADED_REASON`] so consumers don't
///   re-spell it.
/// - [`AttachStatus::ProjectIdMissing`] — no `anvil/project-id`
///   file is present, so the daemon falls back to the pre-MLP2-003
///   "trust the launcher" path. Tracked distinctly from `Clean`
///   so MLP2-025's spoof check can require a project-id at attach
///   time.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachStatus {
    /// All recorded identity fields match the live worktree.
    Clean(ProjectIdentity),
    /// `forked_from` was set on the persisted identity; the attach
    /// proceeds without degradation.
    Fork {
        identity: ProjectIdentity,
        parent_uuid: String,
    },
    /// One or more recorded identity fields disagree with the live
    /// worktree. Callers surface this as `degraded:identity-mismatch`
    /// per the wire-level vocabulary pinned by
    /// [`AttachStatus::DEGRADED_REASON`].
    Mismatch {
        identity: ProjectIdentity,
        reasons: Vec<IdentityMismatch>,
    },
    /// No `anvil/project-id` file is present; nothing to verify.
    /// Returned as its own variant so MLP2-025's spoof check can
    /// require a project-id on tagged attach paths while the
    /// pre-MLP2-003 untagged path keeps working.
    ProjectIdMissing,
}

#[allow(dead_code)]
impl AttachStatus {
    /// Wire-level signal name surfaced on `Mismatch`. Pinned as a
    /// constant so consumers (the daemon's status surface, the CLI
    /// renderer, future witness-chain attribution) don't drift on
    /// the string.
    pub const DEGRADED_REASON: &'static str = "degraded:identity-mismatch";

    /// `true` when the attach should proceed without degradation
    /// (`Clean` or `Fork`).
    #[must_use]
    pub fn is_attach_ok(&self) -> bool {
        matches!(self, Self::Clean(_) | Self::Fork { .. })
    }

    /// `true` when the wire-level `degraded:identity-mismatch`
    /// signal applies.
    #[must_use]
    pub fn is_identity_mismatch(&self) -> bool {
        matches!(self, Self::Mismatch { .. })
    }
}

/// MLP2-003: read `anvil/project-id` from `worktree`, run the
/// composite-identity check against the live git state, and return
/// the resulting [`AttachStatus`].
///
/// This is the single entry point the daemon's attach path consumes:
///
/// - Reads `anvil/project-id` via [`read_project_id`]; absence
///   yields [`AttachStatus::ProjectIdMissing`].
/// - On presence, calls [`ProjectIdentity::verify_against_worktree`]
///   and maps its outcome onto [`AttachStatus`] variants:
///   `IdentityCheck::Match` → `Clean`, `ForkedFromParent` → `Fork`,
///   `Mismatch` → `Mismatch`.
///
/// Returns `Err` only on filesystem / git invocation failures the
/// caller cannot meaningfully recover from; identity disagreements
/// are surfaced as [`AttachStatus::Mismatch`], not as `Err`, so the
/// caller can map them onto the wire-level
/// `degraded:identity-mismatch` signal without losing the failed
/// fields.
#[allow(dead_code)]
pub fn attach_check(worktree: &Path) -> Result<AttachStatus, IdentityError> {
    let Some(identity) = read_project_id(worktree)? else {
        return Ok(AttachStatus::ProjectIdMissing);
    };
    Ok(match identity.verify_against_worktree(worktree)? {
        IdentityCheck::Match => AttachStatus::Clean(identity),
        IdentityCheck::ForkedFromParent { parent_uuid } => AttachStatus::Fork {
            identity,
            parent_uuid,
        },
        IdentityCheck::Mismatch { reasons } => AttachStatus::Mismatch { identity, reasons },
    })
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
# anvil project identifier
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
        // MLP2-003: `first_commit` and `origin_canonical` are now
        // recognised + validated, so the fixture uses real-shaped
        // values; an unknown future key still rounds-trips harmlessly.
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
first_commit: a3b2ea4e1234567890abcdef1234567890abcdef
origin_canonical: github.com/eddacraft/anvil
unknown_future_field: whatever
";
        let parsed = ProjectIdentity::parse(contents).unwrap();
        assert_eq!(parsed.project_uuid, "01997e4a-1b2c-7345-8901-abcdef123456");
        assert_eq!(
            parsed.first_commit.as_deref(),
            Some("a3b2ea4e1234567890abcdef1234567890abcdef")
        );
        assert_eq!(
            parsed.origin_canonical.as_deref(),
            Some("github.com/eddacraft/anvil")
        );
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
        assert!(rendered.contains("anvil project identifier"));
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

    // ── MLP2-003 — composite identity check ─────────────────────────

    /// MLP2-003: the new optional fields round-trip through render
    /// and parse without loss.
    #[test]
    fn first_commit_and_origin_canonical_round_trip() {
        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: Some("2026-05-07T12:34:56Z".into()),
            created_by_version: Some("0.7.0".into()),
            forked_from: None,
            first_commit: Some("a3b2ea4e1234567890abcdef1234567890abcdef".into()),
            origin_canonical: Some("github.com/eddacraft/anvil".into()),
        };
        let rendered = id.render();
        let back = ProjectIdentity::parse(&rendered).unwrap();
        assert_eq!(back, id);
    }

    /// MLP2-003: `first_commit` must be a 40-char lowercase hex sha.
    #[test]
    fn parse_rejects_first_commit_that_is_not_full_sha1() {
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
first_commit: short-sha
";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(matches!(err, IdentityError::Malformed(_)));
    }

    /// MLP2-003: uppercase hex in `first_commit` is rejected — git
    /// always emits lowercase, so a mixed-case value is an authored
    /// mistake we should catch loudly.
    #[test]
    fn parse_rejects_first_commit_with_uppercase_hex() {
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
first_commit: A3B2EA4E1234567890ABCDEF1234567890ABCDEF
";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(matches!(err, IdentityError::Malformed(_)));
    }

    /// MLP2-003: `origin_canonical` must not be empty or carry
    /// whitespace / control characters.
    #[test]
    fn parse_rejects_empty_origin_canonical() {
        let contents = "\
project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456
origin_canonical:
";
        let err = ProjectIdentity::parse(contents).unwrap_err();
        assert!(matches!(err, IdentityError::Malformed(_)));
    }

    /// MLP2-003: SSH alias `git@host:owner/repo[.git]` collapses to
    /// the same canonical form as the HTTPS spelling.
    #[test]
    fn canonicalise_origin_collapses_ssh_alias_and_https() {
        let ssh = canonicalise_origin("git@github.com:eddacraft/anvil.git");
        let https = canonicalise_origin("https://github.com/eddacraft/anvil.git");
        let no_dotgit = canonicalise_origin("https://github.com/eddacraft/anvil");
        let trailing_slash = canonicalise_origin("https://github.com/eddacraft/anvil/");
        let mixed_case_host = canonicalise_origin("https://GitHub.com/eddacraft/anvil.git");
        assert_eq!(ssh, "github.com/eddacraft/anvil");
        assert_eq!(https, "github.com/eddacraft/anvil");
        assert_eq!(no_dotgit, "github.com/eddacraft/anvil");
        assert_eq!(trailing_slash, "github.com/eddacraft/anvil");
        assert_eq!(mixed_case_host, "github.com/eddacraft/anvil");
    }

    /// MLP2-003: `ssh://git@host/path` (the explicit-scheme SSH form)
    /// also canonicalises to the same shape.
    #[test]
    fn canonicalise_origin_handles_explicit_ssh_scheme() {
        let out = canonicalise_origin("ssh://git@github.com/eddacraft/anvil.git");
        assert_eq!(out, "github.com/eddacraft/anvil");
    }

    /// MLP2-003: path case is preserved so a rename of `Anvil` →
    /// `anvil` on the forge surfaces as a mismatch.
    #[test]
    fn canonicalise_origin_preserves_path_case() {
        let upper = canonicalise_origin("https://github.com/eddacraft/Anvil.git");
        let lower = canonicalise_origin("https://github.com/eddacraft/anvil.git");
        assert_ne!(upper, lower);
        assert_eq!(upper, "github.com/eddacraft/Anvil");
    }

    /// MLP2-003: empty / whitespace-only input returns the empty
    /// string — the caller (`read_origin_canonical`) treats that as
    /// "no origin configured".
    #[test]
    fn canonicalise_origin_empty_input_returns_empty() {
        assert_eq!(canonicalise_origin(""), "");
        assert_eq!(canonicalise_origin("   "), "");
    }

    /// MLP2-003: a fresh worktree (no commits) is treated as
    /// "no `first_commit`" rather than an error.
    #[test]
    fn read_first_commit_returns_none_for_empty_repo() {
        let dir = TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        let commit = read_first_commit(dir.path()).expect("read");
        assert!(
            commit.is_none(),
            "empty repo should report no first commit; got {commit:?}"
        );
    }

    /// MLP2-003: a repo with one commit reports that commit's SHA
    /// as its `first_commit` and a worktree-rooted `verify` against
    /// a recorded matching identity returns `Match`.
    #[test]
    fn verify_against_worktree_matches_recorded_first_commit_and_origin() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/eddacraft/anvil.git");
        let live_first = read_first_commit(dir.path()).unwrap().unwrap();
        let live_origin = read_origin_canonical(dir.path()).unwrap().unwrap();

        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: None,
            created_by_version: None,
            forked_from: None,
            first_commit: Some(live_first.clone()),
            origin_canonical: Some(live_origin),
        };
        let check = id.verify_against_worktree(dir.path()).unwrap();
        assert_eq!(check, IdentityCheck::Match);
    }

    /// MLP2-003: a renamed origin (different URL than what
    /// `anvil/project-id` records) surfaces as `OriginCanonical`
    /// mismatch — the operator switched forges without updating the
    /// project-id file.
    #[test]
    fn verify_flags_renamed_origin_as_mismatch() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/eddacraft/anvil.git");
        let live_first = read_first_commit(dir.path()).unwrap().unwrap();

        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: None,
            created_by_version: None,
            forked_from: None,
            first_commit: Some(live_first),
            origin_canonical: Some("gitlab.com/eddacraft/anvil".into()),
        };
        let check = id.verify_against_worktree(dir.path()).unwrap();
        match check {
            IdentityCheck::Mismatch { reasons } => {
                assert_eq!(reasons.len(), 1);
                assert!(matches!(
                    reasons[0],
                    IdentityMismatch::OriginCanonical { .. }
                ));
            }
            other => panic!("expected OriginCanonical mismatch, got {other:?}"),
        }
    }

    /// MLP2-003: a rebased history (root commit changed) surfaces as
    /// `FirstCommit` mismatch.
    #[test]
    fn verify_flags_rebased_history_as_mismatch() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/eddacraft/anvil.git");
        let live_first = read_first_commit(dir.path()).unwrap().unwrap();
        // Force a different root by recording a fake SHA.
        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: None,
            created_by_version: None,
            forked_from: None,
            first_commit: Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into()),
            origin_canonical: Some(read_origin_canonical(dir.path()).unwrap().unwrap()),
        };
        let check = id.verify_against_worktree(dir.path()).unwrap();
        match check {
            IdentityCheck::Mismatch { reasons } => {
                assert_eq!(reasons.len(), 1);
                match &reasons[0] {
                    IdentityMismatch::FirstCommit { recorded, live } => {
                        assert_eq!(recorded, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
                        assert_eq!(live, &live_first);
                    }
                    IdentityMismatch::OriginCanonical { .. } => {
                        panic!("expected FirstCommit mismatch, got an OriginCanonical")
                    }
                }
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    /// MLP2-003: when `forked_from` is set, an otherwise-mismatching
    /// identity still attaches cleanly — the operator has declared a
    /// fork, so the live git state differing from the parent project's
    /// `first_commit` / `origin_canonical` is expected.
    #[test]
    fn verify_accepts_fork_when_forked_from_set() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/me/my-anvil-fork.git");

        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: None,
            created_by_version: None,
            forked_from: Some("9b8a7c6d-5e4f-3210-fedc-ba0987654321".into()),
            // Recorded values are the parent project's — they will
            // mismatch live but `forked_from` lets the attach through.
            first_commit: Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into()),
            origin_canonical: Some("github.com/eddacraft/anvil".into()),
        };
        let check = id.verify_against_worktree(dir.path()).unwrap();
        assert_eq!(
            check,
            IdentityCheck::ForkedFromParent {
                parent_uuid: "9b8a7c6d-5e4f-3210-fedc-ba0987654321".into()
            }
        );
    }

    /// MLP2-003: a `ProjectIdentity` activated before MLP2-003
    /// shipped has `first_commit = None` and `origin_canonical = None`
    /// — the verify path skips both checks rather than reporting
    /// them as mismatches.
    #[test]
    fn verify_skips_fields_that_were_not_recorded() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/eddacraft/anvil.git");

        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: None,
            created_by_version: None,
            forked_from: None,
            first_commit: None,
            origin_canonical: None,
        };
        let check = id.verify_against_worktree(dir.path()).unwrap();
        assert_eq!(check, IdentityCheck::Match);
    }

    /// MLP2-003: when `first_commit` was recorded but the live
    /// worktree has no commits yet, surface a mismatch with an
    /// empty `live` value rather than silently passing.
    #[test]
    fn verify_flags_empty_repo_against_recorded_first_commit() {
        let dir = TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dir.path())
            .status()
            .expect("git init");

        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: None,
            created_by_version: None,
            forked_from: None,
            first_commit: Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into()),
            origin_canonical: None,
        };
        let check = id.verify_against_worktree(dir.path()).unwrap();
        match check {
            IdentityCheck::Mismatch { reasons } => {
                assert_eq!(reasons.len(), 1);
                match &reasons[0] {
                    IdentityMismatch::FirstCommit { recorded, live } => {
                        assert_eq!(recorded, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
                        assert!(
                            live.is_empty(),
                            "empty repo should surface empty live first_commit"
                        );
                    }
                    IdentityMismatch::OriginCanonical { .. } => {
                        panic!("expected FirstCommit mismatch, got an OriginCanonical")
                    }
                }
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    /// MLP2-003: missing origin in the worktree surfaces against a
    /// recorded origin as an `OriginCanonical` mismatch with
    /// `live = None`.
    #[test]
    fn verify_flags_missing_origin_against_recorded_origin() {
        let dir = TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        commit_initial_file(dir.path());

        let id = ProjectIdentity {
            project_uuid: "01997e4a-1b2c-7345-8901-abcdef123456".into(),
            created_at: None,
            created_by_version: None,
            forked_from: None,
            first_commit: None,
            origin_canonical: Some("github.com/eddacraft/anvil".into()),
        };
        let check = id.verify_against_worktree(dir.path()).unwrap();
        match check {
            IdentityCheck::Mismatch { reasons } => {
                assert_eq!(reasons.len(), 1);
                match &reasons[0] {
                    IdentityMismatch::OriginCanonical { recorded, live } => {
                        assert_eq!(recorded, "github.com/eddacraft/anvil");
                        assert!(
                            live.is_none(),
                            "no origin configured should surface live=None"
                        );
                    }
                    IdentityMismatch::FirstCommit { .. } => {
                        panic!("expected OriginCanonical mismatch, got a FirstCommit")
                    }
                }
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    /// MLP2-003 attach contract: a worktree with no `anvil/project-id`
    /// surfaces as `ProjectIdMissing` so MLP2-025's spoof check can
    /// require the file on tagged attach paths while pre-MLP2-003
    /// callers fall through cleanly.
    #[test]
    fn attach_check_returns_project_id_missing_when_file_absent() {
        let dir = TempDir::new().unwrap();
        let status = attach_check(dir.path()).unwrap();
        assert_eq!(status, AttachStatus::ProjectIdMissing);
        assert!(!status.is_attach_ok());
        assert!(!status.is_identity_mismatch());
    }

    /// MLP2-003 attach contract: when the recorded identity matches
    /// the live worktree, attach is `Clean` and the identity is
    /// returned for the daemon to record on the session.
    #[test]
    fn attach_check_clean_when_recorded_matches_live() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/eddacraft/anvil.git");
        let live_first = read_first_commit(dir.path()).unwrap().unwrap();
        let live_origin = read_origin_canonical(dir.path()).unwrap().unwrap();
        // Write a matching anvil/project-id by hand so the test does
        // not depend on `ensure_project_id`'s currently-no-MLP2-003
        // mint path.
        std::fs::create_dir_all(dir.path().join("anvil")).unwrap();
        std::fs::write(
            dir.path().join(PROJECT_ID_PATH),
            format!(
                "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n\
                 first_commit: {live_first}\n\
                 origin_canonical: {live_origin}\n"
            ),
        )
        .unwrap();
        let status = attach_check(dir.path()).unwrap();
        assert!(status.is_attach_ok());
        match status {
            AttachStatus::Clean(id) => {
                assert_eq!(id.first_commit.as_deref(), Some(live_first.as_str()));
                assert_eq!(id.origin_canonical.as_deref(), Some(live_origin.as_str()));
            }
            other => panic!("expected Clean, got {other:?}"),
        }
    }

    /// MLP2-003 attach contract: a `forked_from` declaration accepts
    /// the attach without degradation even when `first_commit` / origin
    /// disagree with the recorded values.
    #[test]
    fn attach_check_fork_passes_through_when_forked_from_set() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/me/my-fork.git");
        std::fs::create_dir_all(dir.path().join("anvil")).unwrap();
        std::fs::write(
            dir.path().join(PROJECT_ID_PATH),
            "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n\
             forked_from: 9b8a7c6d-5e4f-3210-fedc-ba0987654321\n\
             first_commit: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n\
             origin_canonical: github.com/eddacraft/anvil\n",
        )
        .unwrap();
        let status = attach_check(dir.path()).unwrap();
        assert!(status.is_attach_ok());
        match status {
            AttachStatus::Fork { parent_uuid, .. } => {
                assert_eq!(parent_uuid, "9b8a7c6d-5e4f-3210-fedc-ba0987654321");
            }
            other => panic!("expected Fork, got {other:?}"),
        }
    }

    /// MLP2-003 attach contract: a renamed origin surfaces as a
    /// `Mismatch` and the wire-level `degraded:identity-mismatch`
    /// signal applies.
    #[test]
    fn attach_check_mismatch_carries_degraded_signal() {
        let dir = TempDir::new().unwrap();
        init_git_with_origin(dir.path(), "https://github.com/eddacraft/anvil.git");
        let live_first = read_first_commit(dir.path()).unwrap().unwrap();
        std::fs::create_dir_all(dir.path().join("anvil")).unwrap();
        std::fs::write(
            dir.path().join(PROJECT_ID_PATH),
            format!(
                "project_uuid: 01997e4a-1b2c-7345-8901-abcdef123456\n\
                 first_commit: {live_first}\n\
                 origin_canonical: gitlab.com/eddacraft/anvil\n"
            ),
        )
        .unwrap();
        let status = attach_check(dir.path()).unwrap();
        assert!(status.is_identity_mismatch());
        assert!(!status.is_attach_ok());
        assert_eq!(AttachStatus::DEGRADED_REASON, "degraded:identity-mismatch");
        match status {
            AttachStatus::Mismatch { reasons, .. } => {
                assert_eq!(reasons.len(), 1);
                assert!(matches!(
                    reasons[0],
                    IdentityMismatch::OriginCanonical { .. }
                ));
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    /// Test-helper: spin up a tempdir-backed git repo with the
    /// supplied origin URL, one commit, and the user identity set so
    /// `git commit` does not refuse. Returns the live first-commit
    /// sha implicitly via subsequent calls to `read_first_commit`.
    fn init_git_with_origin(root: &Path, origin: &str) {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .expect("git command spawn");
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init", "--quiet"]);
        run(&["config", "user.name", "test"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "commit.gpgsign", "false"]);
        run(&["remote", "add", "origin", origin]);
        commit_initial_file(root);
    }

    /// Test-helper: create one file and commit it.
    fn commit_initial_file(root: &Path) {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .expect("git command spawn");
            assert!(status.success(), "git {args:?} failed");
        };
        // Ensure committer identity is set even if `init_git_with_origin`
        // wasn't used (the missing-origin test path init's git itself).
        let _ = std::process::Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(root)
            .status();
        let _ = std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(root)
            .status();
        let _ = std::process::Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(root)
            .status();
        std::fs::write(root.join("README.md"), "hello\n").expect("write readme");
        run(&["add", "README.md"]);
        run(&["commit", "-m", "initial", "--quiet"]);
    }

    #[cfg(unix)]
    #[test]
    fn ensure_refuses_when_anvil_is_a_symlink() {
        // Council C-10: a symlink at `anvil/` could route writes
        // outside the repo. Refuse rather than silently follow.
        // Unix-only: Windows symlinks require elevated permissions
        // unless dev mode is enabled, so we don't run this on Windows.
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
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

    // ---- MLP2-033: mint_new_identity ---------------------------

    #[test]
    fn mint_new_identity_on_empty_repo_acts_like_fresh() {
        // No prior identity → mint behaves like `ensure_project_id`'s
        // fresh-mint path: a v7 UUID, no `forked_from`.
        let dir = TempDir::new().unwrap();
        let id = mint_new_identity(dir.path(), "0.6.0").unwrap();
        assert!(Uuid::parse_str(&id.project_uuid).is_ok());
        assert_eq!(
            id.forked_from, None,
            "no parent on a virgin repo, so forked_from must be None"
        );
        // Round-trip via the on-disk file.
        let loaded = read_project_id(dir.path()).unwrap().unwrap();
        assert_eq!(loaded, id);
    }

    #[test]
    fn mint_new_identity_records_existing_uuid_as_forked_from() {
        // The headline fork tree from the MLP2-033 validation: parent
        // uuid A → grandchild B with forked_from=A.
        let dir = TempDir::new().unwrap();
        let parent = ensure_project_id(dir.path(), "0.6.0").unwrap();
        let parent_uuid = parent.project_uuid.clone();

        let child = mint_new_identity(dir.path(), "0.6.0").unwrap();
        assert_ne!(
            child.project_uuid, parent_uuid,
            "mint must produce a fresh UUID, not echo the parent"
        );
        assert_eq!(
            child.forked_from.as_deref(),
            Some(parent_uuid.as_str()),
            "forked_from must record the previous project_uuid"
        );

        // The on-disk file was overwritten with the new identity.
        let loaded = read_project_id(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.project_uuid, child.project_uuid);
        assert_eq!(loaded.forked_from.as_deref(), Some(parent_uuid.as_str()));
    }

    #[test]
    fn mint_new_identity_is_not_idempotent_each_call_remints() {
        // Unlike `ensure_project_id` (idempotent on existing identity),
        // `mint_new_identity` is destructive-by-design: each call writes
        // a new UUID, with the *previous* UUID recorded as forked_from.
        // Re-running loses earlier ancestors — the chain is single-deep
        // by spec.
        let dir = TempDir::new().unwrap();
        let first = mint_new_identity(dir.path(), "0.6.0").unwrap();
        let second = mint_new_identity(dir.path(), "0.6.0").unwrap();
        let third = mint_new_identity(dir.path(), "0.6.0").unwrap();
        assert_ne!(first.project_uuid, second.project_uuid);
        assert_ne!(second.project_uuid, third.project_uuid);
        assert_eq!(
            second.forked_from.as_deref(),
            Some(first.project_uuid.as_str())
        );
        assert_eq!(
            third.forked_from.as_deref(),
            Some(second.project_uuid.as_str())
        );
        // The first mint had no parent.
        assert_eq!(first.forked_from, None);
    }

    #[test]
    fn mint_new_identity_treats_malformed_existing_as_no_parent() {
        // Operator's explicit intent is to detach. A garbled previous
        // file shouldn't block the mint — we simply have no recordable
        // parent UUID. (Asymmetric with `ensure_project_id`, which
        // propagates the parse error; documented on `mint_new_identity`.)
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("anvil")).unwrap();
        std::fs::write(
            dir.path().join("anvil/project-id"),
            "not-a-valid-key: garbage\n",
        )
        .unwrap();

        let id = mint_new_identity(dir.path(), "0.6.0").unwrap();
        assert!(Uuid::parse_str(&id.project_uuid).is_ok());
        assert_eq!(id.forked_from, None);
    }

    #[test]
    #[cfg(unix)]
    fn mint_new_identity_refuses_when_anvil_is_a_symlink() {
        // Mirrors the ensure_project_id symlink-refusal pin. Unix-only
        // for the same reason (Windows symlinks need dev mode).
        let dir = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), dir.path().join("anvil")).unwrap();
        let err = mint_new_identity(dir.path(), "0.6.0").unwrap_err();
        assert!(
            matches!(err, IdentityError::Malformed(_)),
            "expected Malformed for symlink anvil/, got {err:?}"
        );
        assert!(
            !elsewhere.path().join("project-id").exists(),
            "must not write project-id through the symlink"
        );
    }
}
