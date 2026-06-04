//! DSV-008 Task 14 (ADR-061 §7): opt-in workspace **confinement** above the
//! same-uid trust boundary.
//!
//! The daemon's floor is `SO_PEERCRED` same-uid (contract §4). In the default
//! `Open` [admission mode](crate::workspace_admission::AdmissionMode) a
//! compromised same-uid agent can name and adopt any root it likes
//! (`workspace_admission` security C3). An operator who wants a tighter boundary
//! switches the daemon to `Allowlist` mode and lists the roots it may serve.
//! This module owns:
//!
//! - the **operator-level** confinement config (`admission = open|allowlist`,
//!   `allow = [exact + prefix]`), read owner-only from the daemon's own home
//!   prefix — never from a repo's `.anvil.yaml` (a checked-in repo file must not
//!   be able to widen the daemon's trust boundary);
//! - resolving that config into an [`AdmittedRoots`] for a connection, with the
//!   primary check-in root **implicitly admitted** in `Allowlist` mode;
//! - the read/modify/write helpers behind the `anvil workspace` CLI (DSV-008).
//!
//! ## Placement (subphase-a item 8)
//!
//! The config dir is resolved via the daemon's own [`crate::anvil_home_prefix`]
//! — the same `ANVIL_HOME`/XDG resolver [`crate::ipc::resolve_socket_dir`] uses
//! — so the daemon loads operator config with **no `anvil-cli` dependency**
//! (`anvil-cli` depends on `anvil-intercept`, not the reverse). The CLI command
//! (`crates/anvil-cli/src/commands/workspace.rs`) is a thin caller of the
//! mutators here.
//!
//! ## Fail closed + loud
//!
//! A **missing** config file is not a failure — it folds into the default
//! `Open` mode (`Ok`). But a config that exists yet cannot be trusted (wrong
//! owner, group/world-writable, or unparseable) must **fail closed + loud**:
//! [`load_or_fail_closed`] logs the error at `error` and returns the most
//! restrictive posture ([`Confinement::fail_closed`] — `Allowlist` with an
//! empty allow set, so only the connection's primary root is ever admitted).
//! The error is never silently swallowed into a permissive default.
//!
//! ## Platform scope
//!
//! The config/file/path layer is platform-neutral, but the owner-only read
//! check and [`Confinement::to_admitted_roots`] are `cfg(unix)` — the daemon's
//! `validate_paths` enforcement point is itself Unix-only in sub-phase A
//! (Windows named-pipe parity is tracked separately). On a non-Unix build the
//! owner-only check is a no-op; confinement is not a supported enforcement
//! boundary there.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Refusal reason surfaced when a verb names a root that confinement does not
/// admit (allowlist mode). The dispatch arm that returns this to the client is
/// DSV-005 wiring; the constant is frozen here so the surfaces agree on it.
pub const WORKSPACE_NOT_ADMITTED: &str = "workspace-not-admitted";

/// Basename of the operator confinement config under the resolved config dir.
const CONFIG_FILE_NAME: &str = "workspace.yaml";

/// Errors loading or persisting the operator confinement config. Every variant
/// is a *loud* failure the daemon must treat as fail-closed — none degrades
/// silently to a permissive default.
#[derive(Debug, Error)]
pub enum ConfinementError {
    /// No `ANVIL_HOME`/XDG/HOME candidate to resolve the config dir from.
    #[error(
        "cannot resolve a confinement config directory (no ANVIL_HOME, XDG_CONFIG_HOME, or HOME)"
    )]
    NoConfigDir,
    /// IO error reading or writing the config file.
    #[error("confinement config IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The config file is not owner-only (wrong owner or group/world-writable).
    /// Refusing to read it fails closed rather than trusting a file another
    /// principal could rewrite.
    #[error(
        "confinement config {path} is not owner-only (mode {mode:#o}, owner uid {owner_uid}, current uid {current_uid})"
    )]
    NotOwnerOnly {
        path: PathBuf,
        mode: u32,
        owner_uid: u32,
        current_uid: u32,
    },
    /// The config path is a symlink. A symlinked config could redirect the read
    /// to a file another principal controls, so it is refused (distinct from
    /// [`ConfinementError::NotOwnerOnly`] — there is no owner/mode to report).
    #[error("confinement config {0} is a symlink — refusing (it could redirect the read)")]
    SymlinkedConfig(PathBuf),
    /// The config file exists but does not parse.
    #[error("confinement config {path} is malformed: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    /// Serialising the config for a write failed.
    #[error("could not serialise confinement config: {0}")]
    Serialize(#[source] serde_yaml::Error),
    /// An allow entry is the filesystem root, which as a prefix would admit
    /// every path and silently nullify allowlist confinement.
    #[error("allow entry {0} is the filesystem root — refusing (it would admit everything)")]
    RootAllowEntry(PathBuf),
}

// --------------------------------------------------------------------
// On-disk file form.
// --------------------------------------------------------------------

/// How an allow entry matches an incoming canonical root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchKind {
    /// Match this canonical root exactly.
    #[default]
    Exact,
    /// Match this canonical root and its entire subtree.
    Prefix,
}

/// Admission mode as written in the config file. Maps onto the runtime
/// [`AdmissionMode`]; kept distinct so the wire vocabulary (`open`/`allowlist`)
/// is owned here and the runtime enum stays an admission concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdmissionModeFile {
    /// First-touch adopt (default).
    #[default]
    Open,
    /// Confinement: only allow-listed roots (+ primary) are admitted.
    Allowlist,
}

#[cfg(unix)]
impl From<AdmissionModeFile> for crate::workspace_admission::AdmissionMode {
    fn from(value: AdmissionModeFile) -> Self {
        use crate::workspace_admission::AdmissionMode;
        match value {
            AdmissionModeFile::Open => AdmissionMode::Open,
            AdmissionModeFile::Allowlist => AdmissionMode::Allowlist,
        }
    }
}

#[cfg(unix)]
impl From<crate::workspace_admission::AdmissionMode> for AdmissionModeFile {
    fn from(value: crate::workspace_admission::AdmissionMode) -> Self {
        use crate::workspace_admission::AdmissionMode;
        match value {
            AdmissionMode::Open => AdmissionModeFile::Open,
            AdmissionMode::Allowlist => AdmissionModeFile::Allowlist,
        }
    }
}

/// A single operator allow entry: a path plus how it matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowEntry {
    /// The allowed root (exact or subtree, per `kind`).
    pub path: PathBuf,
    /// `exact` (default) or `prefix`.
    #[serde(rename = "match", default)]
    pub kind: MatchKind,
}

/// The operator confinement config file shape.
///
/// `deny_unknown_fields`: an unknown/misspelt key (e.g. `admissoin:`) is a
/// *parse error*, not a silently-ignored default — so a typo fails closed +
/// loud rather than degrading to permissive `open`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfinementConfigFile {
    /// `open` (default) or `allowlist`.
    #[serde(default)]
    pub admission: AdmissionModeFile,
    /// Allow entries (only meaningful in `allowlist` mode).
    #[serde(default)]
    pub allow: Vec<AllowEntry>,
}

impl ConfinementConfigFile {
    /// Insert or update an allow entry for `path` with the given match kind.
    /// Idempotent on `path`; re-adding flips an existing entry's kind.
    pub fn upsert_allow(&mut self, path: PathBuf, kind: MatchKind) {
        if let Some(existing) = self.allow.iter_mut().find(|e| e.path == path) {
            existing.kind = kind;
        } else {
            self.allow.push(AllowEntry { path, kind });
        }
    }

    /// Remove the allow entry for `path`. Returns whether anything was removed.
    pub fn remove_allow(&mut self, path: &Path) -> bool {
        let before = self.allow.len();
        self.allow.retain(|e| e.path != path);
        self.allow.len() != before
    }
}

// --------------------------------------------------------------------
// Resolved runtime form.
// --------------------------------------------------------------------

/// The resolved confinement policy a connection is admitted under.
///
/// The mode + allow roots are platform-neutral; the daemon-side bridge into the
/// `cfg(unix)` admission machinery is [`Confinement::to_admitted_roots`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confinement {
    mode: AdmissionModeFile,
    exact: Vec<PathBuf>,
    prefixes: Vec<PathBuf>,
}

impl Confinement {
    /// The default permissive posture: `Open` mode, no allow list. This is what
    /// a *missing* config file resolves to.
    #[must_use]
    pub fn open_default() -> Self {
        Self {
            mode: AdmissionModeFile::Open,
            exact: Vec::new(),
            prefixes: Vec::new(),
        }
    }

    /// The most restrictive posture: `Allowlist` mode with an empty allow set,
    /// so [`Self::to_admitted_roots`] admits only the connection's primary
    /// root. Returned by [`load_or_fail_closed`] when the config cannot be
    /// trusted — fail closed, never open.
    #[must_use]
    pub fn fail_closed() -> Self {
        Self {
            mode: AdmissionModeFile::Allowlist,
            exact: Vec::new(),
            prefixes: Vec::new(),
        }
    }

    /// Resolve a parsed config file into a runtime policy.
    #[must_use]
    pub fn from_file(file: ConfinementConfigFile) -> Self {
        let mut exact = Vec::new();
        let mut prefixes = Vec::new();
        for entry in file.allow {
            match entry.kind {
                MatchKind::Exact => exact.push(entry.path),
                MatchKind::Prefix => prefixes.push(entry.path),
            }
        }
        Self {
            mode: file.admission,
            exact,
            prefixes,
        }
    }

    /// This connection's resolved admission mode.
    #[must_use]
    pub fn mode(&self) -> AdmissionModeFile {
        self.mode
    }

    /// The number of operator allow entries (exact + prefix). Used by the
    /// `anvil status` surface to render `confined: N` in `Allowlist` mode
    /// (DSV-007 Task 17). The connection's implicitly-admitted primary root is
    /// not counted — this is the *configured* allow-list size, not the effective
    /// admitted-root count for any one connection.
    #[must_use]
    pub fn allow_count(&self) -> usize {
        self.exact.len() + self.prefixes.len()
    }

    /// Build the per-connection [`AdmittedRoots`](crate::workspace_admission::AdmittedRoots)
    /// this confinement implies. The DSV-005 dispatch arm calls this once per
    /// connection to gate `validate_paths`.
    ///
    /// - `Open`: a first-touch-adopt set (the allow list is irrelevant).
    /// - `Allowlist`: an `AllowPolicy` over the canonicalised exact + prefix
    ///   allow roots, **plus the canonicalised `primary_root`** so the primary
    ///   check-in root is admitted even with an empty allow list — *unless* it
    ///   cannot be canonicalised (deleted mid-connection), in which case it is
    ///   dropped with a `warn` and the connection is refused (safe). Allow
    ///   entries that do not currently resolve are likewise dropped with a
    ///   `warn` (they cannot match a real, openable root anyway); a
    ///   filesystem-root prefix entry is ignored with a `warn` (it would admit
    ///   everything — the write path rejects it, this guards hand-edited files).
    #[cfg(unix)]
    #[must_use]
    pub fn to_admitted_roots(
        &self,
        primary_root: &Path,
    ) -> crate::workspace_admission::AdmittedRoots {
        use crate::workspace_admission::{AdmittedRoots, AllowPolicy};

        // Canonicalise an allow entry, logging (never silently dropping) one
        // that does not resolve — a silent drop would mask operator
        // misconfiguration as an unexplained `workspace-not-admitted` refusal.
        fn canonicalise(p: &Path, what: &str) -> Option<PathBuf> {
            match std::fs::canonicalize(p) {
                Ok(canonical) => Some(canonical),
                Err(error) => {
                    tracing::warn!(
                        path = %p.display(), %error,
                        "confinement: dropping unresolvable {what} allow entry"
                    );
                    None
                }
            }
        }

        match self.mode {
            AdmissionModeFile::Open => AdmittedRoots::new_open(),
            AdmissionModeFile::Allowlist => {
                let mut exact: Vec<PathBuf> = self
                    .exact
                    .iter()
                    .filter_map(|p| canonicalise(p, "exact"))
                    .collect();
                // The primary check-in root is implicitly admitted.
                match std::fs::canonicalize(primary_root) {
                    Ok(primary) => exact.push(primary),
                    Err(error) => tracing::warn!(
                        path = %primary_root.display(), %error,
                        "confinement: primary check-in root did not resolve — \
                         allowlist mode will refuse this connection"
                    ),
                }
                let prefixes: Vec<PathBuf> = self
                    .prefixes
                    .iter()
                    .filter_map(|p| canonicalise(p, "prefix"))
                    .filter(|canonical| {
                        if canonical.parent().is_none() {
                            tracing::warn!(
                                path = %canonical.display(),
                                "confinement: ignoring filesystem-root prefix allow entry \
                                 (it would admit every path)"
                            );
                            false
                        } else {
                            true
                        }
                    })
                    .collect();
                AdmittedRoots::new_allowlist_with_policy(AllowPolicy::new(exact, prefixes))
            }
        }
    }
}

// --------------------------------------------------------------------
// Config-dir resolution (mirrors the daemon's own home resolver).
// --------------------------------------------------------------------

/// Resolve the confinement config path from explicit candidate roots — the pure
/// core of [`config_path`], so it unit-tests without mutating the environment.
///
/// Precedence mirrors [`crate::ipc::resolve_socket_dir`]: a non-empty
/// `ANVIL_HOME` prefix re-roots the file directly under the prefix; otherwise
/// `$XDG_CONFIG_HOME/anvil`; otherwise `$HOME/.config/anvil`.
fn config_dir_from(
    anvil_home: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, ConfinementError> {
    if let Some(prefix) = anvil_home {
        return Ok(prefix);
    }
    if let Some(dir) = xdg_config_home {
        return Ok(dir.join("anvil"));
    }
    if let Some(home) = home {
        return Ok(home.join(".config").join("anvil"));
    }
    Err(ConfinementError::NoConfigDir)
}

fn config_path_from(
    anvil_home: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, ConfinementError> {
    Ok(config_dir_from(anvil_home, xdg_config_home, home)?.join(CONFIG_FILE_NAME))
}

/// The operator confinement config path for the current user. Resolved via the
/// daemon's own [`crate::anvil_home_prefix`] (item 8 — no `anvil-cli` path).
pub fn config_path() -> Result<PathBuf, ConfinementError> {
    config_path_from(
        crate::anvil_home_prefix(),
        non_empty_env("XDG_CONFIG_HOME"),
        non_empty_env("HOME").or_else(|| non_empty_env("USERPROFILE")),
    )
}

/// The anvil operator config **directory** (no filename) for the current user,
/// resolved with the same `ANVIL_HOME`/XDG/HOME precedence as [`config_path`].
/// Shared with other daemon-owned operator configs that live beside the
/// confinement file — e.g. the save-time antipattern config
/// ([`crate::antipattern_config`]) — so every operator surface resolves its
/// directory through one daemon-owned resolver, never an `anvil-cli` path.
///
/// `#[cfg(unix)]`: the only caller is the (Unix-gated) `antipattern_config`
/// loader, so this is dead code on Windows (DSV-010a). DSV-010b revisits the
/// daemon's Windows operator-config surface.
#[cfg(unix)]
pub(crate) fn anvil_config_dir() -> Result<PathBuf, ConfinementError> {
    config_dir_from(
        crate::anvil_home_prefix(),
        non_empty_env("XDG_CONFIG_HOME"),
        non_empty_env("HOME").or_else(|| non_empty_env("USERPROFILE")),
    )
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

// --------------------------------------------------------------------
// Owner-only load.
// --------------------------------------------------------------------

/// Owner-only metadata predicate: owned by the current uid and not
/// group/world-writable. Applied to an *already-open fd's* metadata so the
/// check and the read see the same inode. Group/world *read* is tolerated (an
/// allowlist of paths is not a secret); only *write* by another principal
/// (`0o022`) — which could rewrite the trust boundary — and non-owner files are
/// refused. Returns the offending `(mode, owner_uid, current_uid)` on violation.
#[cfg(unix)]
fn owner_only_violation(meta: &std::fs::Metadata) -> Option<(u32, u32, u32)> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let current_uid = nix::unistd::Uid::current().as_raw();
    let owner_uid = meta.uid();
    let mode = meta.permissions().mode() & 0o777;
    if owner_uid != current_uid || mode & 0o022 != 0 {
        Some((mode, owner_uid, current_uid))
    } else {
        None
    }
}

/// Read a trusted config file owner-only, eliminating the read-before-check
/// TOCTOU: the file is opened **once** with `O_NOFOLLOW` (a symlinked leaf is
/// refused, not followed), its metadata is taken from the open fd (`fstat`), the
/// owner-only invariant is enforced on *that* metadata, and the bytes are read
/// from the same fd. A missing file is `Ok(None)`.
///
/// Intermediate path components are still resolved by the open; the loader
/// trusts the integrity of the resolved config *directory* and relies on the
/// `SO_PEERCRED` same-uid floor for it (only the same uid can rewrite their own
/// `~/.config` / `ANVIL_HOME`). That is in-model — confinement tightens *within*
/// the same-uid boundary, it does not claim a cross-uid one.
///
/// `pub(crate)` so other daemon-owned operator configs that live beside the
/// confinement file (e.g. [`crate::antipattern_config`]) reuse this one audited
/// owner-only reader rather than re-implementing the TOCTOU-safe open. Those
/// callers map [`ConfinementError`] into their own error at the boundary, so
/// the confinement-flavoured Display strings never surface for them.
#[cfg(unix)]
pub(crate) fn read_trusted(path: &Path) -> Result<Option<String>, ConfinementError> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        // `O_NOFOLLOW` on a symlinked leaf surfaces as `ELOOP` — a symlinked
        // config is untrusted (it could redirect to another principal's file).
        Err(err) if err.raw_os_error() == Some(nix::libc::ELOOP) => {
            return Err(ConfinementError::SymlinkedConfig(path.to_path_buf()));
        }
        Err(source) => {
            return Err(ConfinementError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let meta = file.metadata().map_err(|source| ConfinementError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if let Some((mode, owner_uid, current_uid)) = owner_only_violation(&meta) {
        return Err(ConfinementError::NotOwnerOnly {
            path: path.to_path_buf(),
            mode,
            owner_uid,
            current_uid,
        });
    }
    let mut raw = String::new();
    file.read_to_string(&mut raw)
        .map_err(|source| ConfinementError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(Some(raw))
}

#[cfg(not(unix))]
pub(crate) fn read_trusted(path: &Path) -> Result<Option<String>, ConfinementError> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ConfinementError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Load + resolve the confinement config from an explicit path.
///
/// A missing file folds into [`Confinement::open_default`] (`Ok`). A present
/// file must be owner-only and parse; otherwise this returns a *loud* `Err`
/// (the caller — [`load_or_fail_closed`] in production — fails closed on it).
pub fn load_from(path: &Path) -> Result<Confinement, ConfinementError> {
    let Some(raw) = read_trusted(path)? else {
        return Ok(Confinement::open_default());
    };
    let file: ConfinementConfigFile =
        serde_yaml::from_str(&raw).map_err(|source| ConfinementError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(Confinement::from_file(file))
}

/// Load the confinement config from the resolved [`config_path`].
pub fn load() -> Result<Confinement, ConfinementError> {
    load_from(&config_path()?)
}

/// Production loader: load the confinement config, **failing closed + loud** on
/// an *untrusted* config. A broken or untrusted config (wrong owner, symlinked,
/// malformed) is logged at `error` and the most restrictive
/// [`Confinement::fail_closed`] posture is returned, so it never silently opens
/// the daemon.
///
/// [`ConfinementError::NoConfigDir`] is the one exception: it means no config
/// *location* could be resolved (no `ANVIL_HOME`/`XDG_CONFIG_HOME`/`HOME`) — an
/// absent config, not an untrusted one. It is treated like a missing file
/// (default `open`) with a `warn`, so a daemon that resolves its socket via
/// `XDG_RUNTIME_DIR` but lacks a config-dir env var is not silently forced into
/// primary-root-only allowlist mode.
#[must_use]
pub fn load_or_fail_closed() -> Confinement {
    resolve_or_fail_closed(load())
}

/// Pure policy mapper for [`load_or_fail_closed`] — unit-testable without
/// mutating process env. `Ok` passes through; `NoConfigDir` (absent location)
/// defaults to `open` with a `warn`; every other (untrusted/malformed) error
/// fails closed with an `error`.
fn resolve_or_fail_closed(result: Result<Confinement, ConfinementError>) -> Confinement {
    match result {
        Ok(confinement) => confinement,
        Err(ConfinementError::NoConfigDir) => {
            tracing::warn!(
                "no confinement config directory could be resolved \
                 (no ANVIL_HOME/XDG_CONFIG_HOME/HOME) — defaulting to open admission"
            );
            Confinement::open_default()
        }
        Err(err) => {
            tracing::error!(
                error = %err,
                "confinement config load failed — failing closed (allowlist, primary-root only)"
            );
            Confinement::fail_closed()
        }
    }
}

// --------------------------------------------------------------------
// Persisting (the `anvil workspace` CLI write path).
// --------------------------------------------------------------------

/// Read the on-disk config file (missing → default), without resolving it into
/// a runtime [`Confinement`]. Used by the `anvil workspace` CLI to display and
/// mutate entries while preserving the exact/prefix distinction.
pub fn read_config_file_from(path: &Path) -> Result<ConfinementConfigFile, ConfinementError> {
    let Some(raw) = read_trusted(path)? else {
        return Ok(ConfinementConfigFile::default());
    };
    serde_yaml::from_str(&raw).map_err(|source| ConfinementError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// Read the on-disk config file at the resolved [`config_path`].
pub fn read_config_file() -> Result<ConfinementConfigFile, ConfinementError> {
    read_config_file_from(&config_path()?)
}

/// Reject an allow entry that is a filesystem root (its parent is `None`): as a
/// prefix it would admit every path and silently nullify allowlist confinement
/// (and an empty path is meaningless). Refusing it loudly keeps the operator
/// from foot-gunning their own boundary.
fn reject_root_allow_entries(file: &ConfinementConfigFile) -> Result<(), ConfinementError> {
    if let Some(entry) = file.allow.iter().find(|e| e.path.parent().is_none()) {
        return Err(ConfinementError::RootAllowEntry(entry.path.clone()));
    }
    Ok(())
}

/// Write the config file owner-only (dir `0700`, file `0600`), creating the
/// config dir if needed. The write is **atomic** (temp sibling created `0600`,
/// then renamed into place) so a concurrent daemon read never observes a
/// truncated/partial config. Returns the path written.
pub fn write_config_file_to(
    path: &Path,
    file: &ConfinementConfigFile,
) -> Result<(), ConfinementError> {
    reject_root_allow_entries(file)?;
    if let Some(parent) = path.parent() {
        create_owner_only_dir(parent)?;
    }
    let body = serde_yaml::to_string(file).map_err(ConfinementError::Serialize)?;
    write_atomic_owner_only(path, body.as_bytes())
}

/// Atomically write `body` to `path` with `0600` permissions from creation: a
/// per-process temp sibling is created `O_NOFOLLOW`/`0600`, written, synced, and
/// renamed over `path`. No world-readable window (the temp is `0600` at
/// `open`), no torn config (rename is atomic), and concurrent `anvil workspace`
/// invocations use distinct temp names so they cannot clobber each other's temp.
#[cfg(unix)]
fn write_atomic_owner_only(path: &Path, body: &[u8]) -> Result<(), ConfinementError> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(CONFIG_FILE_NAME);
    let tmp = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let io = |source| ConfinementError::Io {
        path: tmp.clone(),
        source,
    };
    let mut handle = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(&tmp)
        .map_err(&io)?;
    handle.write_all(body).map_err(&io)?;
    handle.sync_all().map_err(&io)?;
    // Re-assert 0600 in case a stale temp pre-existed with wider bits.
    set_owner_only_file(&tmp)?;
    std::fs::rename(&tmp, path).map_err(|source| ConfinementError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
fn write_atomic_owner_only(path: &Path, body: &[u8]) -> Result<(), ConfinementError> {
    std::fs::write(path, body).map_err(|source| ConfinementError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Write the config file to the resolved [`config_path`].
pub fn write_config_file(file: &ConfinementConfigFile) -> Result<PathBuf, ConfinementError> {
    let path = config_path()?;
    write_config_file_to(&path, file)?;
    Ok(path)
}

#[cfg(unix)]
fn create_owner_only_dir(dir: &Path) -> Result<(), ConfinementError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).map_err(|source| ConfinementError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|source| {
        ConfinementError::Io {
            path: dir.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn create_owner_only_dir(dir: &Path) -> Result<(), ConfinementError> {
    std::fs::create_dir_all(dir).map_err(|source| ConfinementError::Io {
        path: dir.to_path_buf(),
        source,
    })
}

#[cfg(unix)]
fn set_owner_only_file(path: &Path) -> Result<(), ConfinementError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|source| {
        ConfinementError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_owner_only(path: &Path, body: &str) {
        std::fs::write(path, body).expect("write config");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .expect("chmod 0600");
        }
    }

    #[cfg(unix)]
    #[test]
    fn open_mode_auto_adopts() {
        // The default (open) confinement adopts any nameable root on first
        // touch — the allow list is irrelevant.
        let confinement = Confinement::open_default();
        assert_eq!(confinement.mode(), AdmissionModeFile::Open);

        let primary = tempfile::tempdir().expect("tempdir");
        let other = tempfile::tempdir().expect("tempdir");
        let mut roots = confinement.to_admitted_roots(primary.path());
        assert!(
            roots.authorise(other.path()).expect("io").is_some(),
            "open mode auto-adopts an unlisted root"
        );
    }

    #[cfg(unix)]
    #[test]
    fn allowlist_refuses_unlisted() {
        let allowed = tempfile::tempdir().expect("tempdir");
        let primary = tempfile::tempdir().expect("tempdir");
        let other = tempfile::tempdir().expect("tempdir");

        let confinement = Confinement::from_file(ConfinementConfigFile {
            admission: AdmissionModeFile::Allowlist,
            allow: vec![AllowEntry {
                path: allowed.path().to_path_buf(),
                kind: MatchKind::Exact,
            }],
        });
        let mut roots = confinement.to_admitted_roots(primary.path());

        assert!(
            roots.authorise(allowed.path()).expect("io").is_some(),
            "an allow-listed root is admitted"
        );
        assert!(
            roots.authorise(other.path()).expect("io").is_none(),
            "an unlisted root is refused in allowlist mode"
        );
    }

    #[cfg(unix)]
    #[test]
    fn primary_root_implicitly_admitted() {
        // Allowlist mode with an EMPTY allow list still admits the connection's
        // primary check-in root.
        let primary = tempfile::tempdir().expect("tempdir");
        let confinement = Confinement::from_file(ConfinementConfigFile {
            admission: AdmissionModeFile::Allowlist,
            allow: Vec::new(),
        });
        let mut roots = confinement.to_admitted_roots(primary.path());
        assert!(
            roots.authorise(primary.path()).expect("io").is_some(),
            "the primary check-in root is implicitly admitted"
        );
    }

    #[cfg(unix)]
    #[test]
    fn prefix_entry_matches_subtree() {
        let parent = tempfile::tempdir().expect("tempdir");
        let child = parent.path().join("nested/project");
        std::fs::create_dir_all(&child).expect("mkdir child");
        let primary = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("tempdir");

        let confinement = Confinement::from_file(ConfinementConfigFile {
            admission: AdmissionModeFile::Allowlist,
            allow: vec![AllowEntry {
                path: parent.path().to_path_buf(),
                kind: MatchKind::Prefix,
            }],
        });
        let mut roots = confinement.to_admitted_roots(primary.path());

        assert!(
            roots.authorise(&child).expect("io").is_some(),
            "a root beneath a prefix allow entry is admitted"
        );
        assert!(
            roots.authorise(outside.path()).expect("io").is_none(),
            "a root outside every prefix is refused"
        );
    }

    #[cfg(unix)]
    #[test]
    fn config_load_failure_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        // Malformed: `admission` must be a string scalar, not a mapping.
        write_owner_only(&path, "admission:\n  not: a-scalar\n");

        let err = load_from(&path).expect_err("malformed config must error");
        assert!(
            matches!(err, ConfinementError::Parse { .. }),
            "expected a parse error, got {err:?}"
        );

        // The production posture on such an error is fail-closed: allowlist mode
        // admitting only the primary root.
        let closed = Confinement::fail_closed();
        assert_eq!(closed.mode(), AdmissionModeFile::Allowlist);
        let primary = tempfile::tempdir().expect("tempdir");
        let other = tempfile::tempdir().expect("tempdir");
        let mut roots = closed.to_admitted_roots(primary.path());
        assert!(
            roots.authorise(primary.path()).expect("io").is_some(),
            "fail-closed still serves the primary root"
        );
        assert!(
            roots.authorise(other.path()).expect("io").is_none(),
            "fail-closed admits nothing else"
        );
    }

    #[test]
    fn missing_config_is_open_not_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        let confinement = load_from(&path).expect("missing config folds into Ok(open)");
        assert_eq!(confinement, Confinement::open_default());
    }

    #[test]
    fn allowlist_not_read_from_repo_dotanvil() {
        // A repo `.anvil.yaml` must never feed the daemon's trust boundary.
        // Put an allowlist in a repo's `.anvil.yaml`, point the confinement
        // loader at a SEPARATE (missing) operator path, and assert the repo
        // file had no effect — the daemon stays in default open mode.
        let repo = tempfile::tempdir().expect("tempdir");
        write_owner_only(
            &repo.path().join(".anvil.yaml"),
            "admission: allowlist\nallow:\n  - path: /etc\n",
        );
        let operator_dir = tempfile::tempdir().expect("tempdir");
        let operator_path = operator_dir.path().join("workspace.yaml");

        let confinement = load_from(&operator_path).expect("load operator config");
        assert_eq!(
            confinement,
            Confinement::open_default(),
            "the repo .anvil.yaml must not be read as confinement config"
        );
    }

    #[test]
    fn confinement_config_dir_resolved_via_anvil_home_prefix() {
        // item 8: ANVIL_HOME re-roots the config file directly under the prefix,
        // mirroring resolve_socket_dir — not a separate anvil-cli path.
        let prefix = PathBuf::from("/var/lib/anvil-candidate");
        let resolved =
            config_path_from(Some(prefix.clone()), None, None).expect("resolve via ANVIL_HOME");
        assert_eq!(resolved, prefix.join(CONFIG_FILE_NAME));
        assert!(
            resolved.starts_with(&prefix),
            "config sits under ANVIL_HOME"
        );

        // XDG and HOME fallbacks when ANVIL_HOME is unset.
        let xdg = config_path_from(None, Some(PathBuf::from("/home/op/.config")), None)
            .expect("resolve via XDG");
        assert_eq!(xdg, Path::new("/home/op/.config/anvil/workspace.yaml"));
        let home = config_path_from(None, None, Some(PathBuf::from("/home/op")))
            .expect("resolve via HOME");
        assert_eq!(home, Path::new("/home/op/.config/anvil/workspace.yaml"));

        // No candidate at all is a loud error, never a silent default.
        assert!(matches!(
            config_path_from(None, None, None),
            Err(ConfinementError::NoConfigDir)
        ));
    }

    #[test]
    fn roundtrip_write_read_preserves_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("workspace.yaml");
        let mut file = ConfinementConfigFile {
            admission: AdmissionModeFile::Allowlist,
            allow: Vec::new(),
        };
        file.upsert_allow(PathBuf::from("/srv/a"), MatchKind::Exact);
        file.upsert_allow(PathBuf::from("/srv/tree"), MatchKind::Prefix);

        write_config_file_to(&path, &file).expect("write");
        let back = read_config_file_from(&path).expect("read");
        assert_eq!(back, file);

        // upsert is idempotent on path and flips kind; remove deletes.
        let mut mutated = back;
        mutated.upsert_allow(PathBuf::from("/srv/a"), MatchKind::Prefix);
        assert_eq!(
            mutated
                .allow
                .iter()
                .filter(|e| e.path == Path::new("/srv/a"))
                .count(),
            1
        );
        assert_eq!(
            mutated
                .allow
                .iter()
                .find(|e| e.path == Path::new("/srv/a"))
                .unwrap()
                .kind,
            MatchKind::Prefix
        );
        assert!(mutated.remove_allow(Path::new("/srv/tree")));
        assert!(
            !mutated.remove_allow(Path::new("/srv/tree")),
            "second remove is a no-op"
        );
    }

    #[cfg(unix)]
    #[test]
    fn group_writable_config_fails_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        std::fs::write(&path, "admission: allowlist\n").expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o660)).expect("chmod");

        let err = load_from(&path).expect_err("group-writable config must be refused");
        assert!(
            matches!(err, ConfinementError::NotOwnerOnly { .. }),
            "expected NotOwnerOnly, got {err:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn world_writable_config_fails_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        std::fs::write(&path, "admission: allowlist\n").expect("write");
        // Other-write (0o002) is the second half of the 0o022 mask.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o606)).expect("chmod");

        let err = load_from(&path).expect_err("world-writable config must be refused");
        assert!(
            matches!(err, ConfinementError::NotOwnerOnly { .. }),
            "expected NotOwnerOnly, got {err:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_config_is_refused_not_followed() {
        // A symlink at the config path could redirect the read to a file another
        // principal controls — O_NOFOLLOW refuses it (ELOOP → NotOwnerOnly)
        // rather than reading the target's bytes.
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().join("real-allowlist.yaml");
        write_owner_only(&real, "admission: allowlist\n");
        let link = dir.path().join("workspace.yaml");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        let err = load_from(&link).expect_err("a symlinked config must be refused");
        assert!(
            matches!(err, ConfinementError::SymlinkedConfig(_)),
            "expected SymlinkedConfig (symlink refused), got {err:?}"
        );
    }

    #[test]
    fn no_config_dir_defaults_to_open_untrusted_fails_closed() {
        // An unresolvable config *location* (absent env) is absence, not an
        // untrusted config — it defaults to open (like a missing file), not
        // primary-root-only allowlist.
        assert_eq!(
            resolve_or_fail_closed(Err(ConfinementError::NoConfigDir)),
            Confinement::open_default(),
            "NoConfigDir is absence → open, not fail-closed"
        );
        // A genuinely untrusted/malformed config still fails closed.
        assert_eq!(
            resolve_or_fail_closed(Err(ConfinementError::SymlinkedConfig(PathBuf::from("/x")))),
            Confinement::fail_closed(),
            "an untrusted config fails closed"
        );
        // Ok passes through unchanged.
        assert_eq!(
            resolve_or_fail_closed(Ok(Confinement::open_default())),
            Confinement::open_default()
        );
    }

    #[test]
    fn unknown_config_key_fails_closed_not_silently_open() {
        // A misspelt key (`admissoin`) must be a loud parse error, not a silent
        // fall-through to the permissive `open` default (deny_unknown_fields).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        write_owner_only(&path, "admissoin: allowlist\n");

        let err = load_from(&path).expect_err("a typo'd key must error, not default to open");
        assert!(
            matches!(err, ConfinementError::Parse { .. }),
            "expected Parse (deny_unknown_fields), got {err:?}"
        );
    }

    #[test]
    fn root_prefix_allow_entry_is_rejected_on_write() {
        // `/` as a prefix would admit every path — the write path refuses it so
        // an operator cannot silently nullify their own allowlist.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        let file = ConfinementConfigFile {
            admission: AdmissionModeFile::Allowlist,
            allow: vec![AllowEntry {
                path: PathBuf::from("/"),
                kind: MatchKind::Prefix,
            }],
        };
        let err = write_config_file_to(&path, &file).expect_err("root prefix must be rejected");
        assert!(
            matches!(err, ConfinementError::RootAllowEntry(_)),
            "expected RootAllowEntry, got {err:?}"
        );
        assert!(
            !path.exists(),
            "nothing is written when the config is rejected"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_is_owner_only_and_leaves_no_temp() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        let file = ConfinementConfigFile {
            admission: AdmissionModeFile::Allowlist,
            allow: vec![AllowEntry {
                path: PathBuf::from("/srv/x"),
                kind: MatchKind::Exact,
            }],
        };
        write_config_file_to(&path, &file).expect("write");

        let mode = std::fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config is written owner-only from creation");

        // The atomic temp sibling must not survive the rename.
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "no temp file left behind: {leftovers:?}"
        );
    }
}
