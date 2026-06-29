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
//! The config/file/path layer is platform-neutral, and since DSV-010b
//! [`Confinement::to_admitted_roots`] is served on both Unix and Windows (the
//! `validate_paths` enforcement point now answers over the Windows named pipe
//! too — ADR-070 Stage 2). The **owner-only trusted read** (`read_trusted`) has
//! a per-platform impl: on Unix an `O_NOFOLLOW` open + owner-uid + no-foreign-
//! write (mode) check; on Windows (DSV-010b hardening) a reparse-point refusal +
//! `GetSecurityInfo` owner-SID match (via `anvil-intercept-win32`), with the
//! no-foreign-write property coming from the owner-only config dir
//! ([`create_owner_only_dir`]) plus the per-user profile ACLs. Both fail closed
//! on an untrusted (symlinked / foreign-owned) config.

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
    /// (Windows, DSV-010b) the config file's owner SID is not the current user's
    /// — refused, since another principal could rewrite the trust boundary. The
    /// Windows analogue of [`ConfinementError::NotOwnerOnly`] (which reports a
    /// Unix mode + uids that have no Windows meaning).
    #[cfg(windows)]
    #[error(
        "confinement config {path} is owned by another principal (owner SID {owner_sid}, current {current_sid}) — refusing"
    )]
    NotOwnerSid {
        path: PathBuf,
        owner_sid: String,
        current_sid: String,
    },
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
    /// (ACTMO-019) Refusing to write back a config whose on-disk format version
    /// is newer than this binary understands — the lenient read dropped its
    /// unknown keys, so writing would silently lose them. Fail loud instead.
    #[error(
        "confinement config is format version {version}, newer than this Anvil understands ({current}) — \
         refusing to write (it would drop keys); upgrade Anvil to edit it"
    )]
    FutureConfigVersion { version: u32, current: u32 },
    /// (ACTMO-019) A `register_on_start` entry is not an absolute path. The CLI
    /// only ever stores canonicalised absolute roots; a relative entry can only
    /// come from a hand-edit and would resolve against the daemon's cwd at
    /// startup, registering a surprising directory. Refused on write.
    #[error("register_on_start entry {0} is not an absolute path — refusing")]
    RelativeRegisterOnStart(PathBuf),
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

/// The default confinement config format version — the implicit version of
/// every config written before ACTMO-019 (which carried no `version:` key).
const DEFAULT_CONFIG_VERSION: u32 = 1;

/// The current confinement config format version this daemon writes and fully
/// understands (ACTMO-019). Bumped from the implicit `1` when the top-level
/// [`ConfinementConfigFile::register_on_start`] key was added.
///
/// A config at **or below** this version is parsed *strictly*
/// (`deny_unknown_fields` — an unknown key is a loud parse error, preserving the
/// typo-protection contract). A config at a **higher** version is parsed
/// *leniently*: keys this daemon does not know are dropped with a `warn` rather
/// than failing the load closed, so a config written by a newer Anvil never
/// collapses an older daemon's confinement trust floor (ADR-094 decision 5,
/// forward-compat). Because `register_on_start` is opt-in and serialised only
/// when non-empty, a pure-confinement config stays at version `1` and remains
/// byte-compatible with pre-ACTMO-019 daemons.
pub const CURRENT_CONFIG_VERSION: u32 = 2;

/// The top-level keys this daemon knows. A key outside this set is a typo at the
/// known format version (loud parse error) or a forward-compat key at a higher
/// version (dropped with a `warn`).
const KNOWN_CONFIG_KEYS: &[&str] = &["version", "admission", "allow", "register_on_start"];

fn default_config_version() -> u32 {
    DEFAULT_CONFIG_VERSION
}

/// `version: 1` is the implicit default and is omitted on write so a
/// pure-confinement config stays byte-compatible with pre-ACTMO-019 daemons.
// `skip_serializing_if` requires a `fn(&T) -> bool` signature, so the `&u32` is
// not optional here despite `u32` being `Copy`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_config_version(version: &u32) -> bool {
    *version <= DEFAULT_CONFIG_VERSION
}

/// The operator confinement config file shape.
///
/// Unknown-key handling is **version-gated** (ACTMO-019): at or below
/// [`CURRENT_CONFIG_VERSION`] the typed parse uses `deny_unknown_fields`, so an
/// unknown/misspelt key (e.g. `admissoin:`) is a *parse error*, not a
/// silently-ignored default — a typo fails closed + loud rather than degrading
/// to permissive `open`. Above it, [`parse_config_file`] drops unknown keys so a
/// newer-format config never fails an older daemon closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfinementConfigFile {
    /// Config format version. Absent ⇒ `1` (pre-ACTMO-019). Bumped to
    /// [`CURRENT_CONFIG_VERSION`] when `register_on_start` is non-empty; omitted
    /// on write at the default so existing files stay byte-compatible.
    #[serde(
        default = "default_config_version",
        skip_serializing_if = "is_default_config_version"
    )]
    pub version: u32,
    /// `open` (default) or `allowlist`.
    #[serde(default)]
    pub admission: AdmissionModeFile,
    /// Allow entries (only meaningful in `allowlist` mode).
    #[serde(default)]
    pub allow: Vec<AllowEntry>,
    /// ACTMO-019: worktrees the daemon durably registers on startup. A
    /// **separate top-level key**, deliberately *not* a field on
    /// [`AllowEntry`] — confinement admission ("what the daemon may serve") and
    /// registration membership ("what is actively protected") are distinct sets.
    /// Empty by default and omitted on write so it never appears unless an
    /// operator opts in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub register_on_start: Vec<PathBuf>,
}

impl Default for ConfinementConfigFile {
    fn default() -> Self {
        Self {
            version: DEFAULT_CONFIG_VERSION,
            admission: AdmissionModeFile::default(),
            allow: Vec::new(),
            register_on_start: Vec::new(),
        }
    }
}

impl ConfinementConfigFile {
    /// Add `path` to the durable `register_on_start` set, bumping the config
    /// format version so the file is marked as carrying the ACTMO-019 key.
    /// Idempotent — returns whether the set changed.
    pub fn add_register_on_start(&mut self, path: PathBuf) -> bool {
        if self.register_on_start.iter().any(|p| p == &path) {
            return false;
        }
        self.register_on_start.push(path);
        // Never *downgrade* the version. A file read at a higher (future) format
        // version keeps it, so the write guard (`write_config_file_to`) refuses
        // to clobber a newer file rather than silently dropping its keys.
        self.version = self.version.max(CURRENT_CONFIG_VERSION);
        true
    }

    /// Remove `path` from the `register_on_start` set. Returns whether anything
    /// was removed. When the set empties, the version drops back to the default
    /// so the file becomes byte-compatible with pre-ACTMO-019 daemons again —
    /// but only if it is *our* bump (`CURRENT_CONFIG_VERSION`); a higher (future)
    /// version is left intact so we never misrepresent a newer file's format.
    pub fn remove_register_on_start(&mut self, path: &Path) -> bool {
        let before = self.register_on_start.len();
        self.register_on_start.retain(|p| p != path);
        let removed = self.register_on_start.len() != before;
        if removed && self.register_on_start.is_empty() && self.version == CURRENT_CONFIG_VERSION {
            self.version = DEFAULT_CONFIG_VERSION;
        }
        removed
    }

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
/// admission machinery is [`Confinement::to_admitted_roots`] (served on both Unix
/// and Windows since DSV-010b).
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
    // DSV-010b: served on both Unix and Windows now that `AdmittedRoots` holds a
    // platform-neutral `WorkspaceAnchor`; the body is pure path logic.
    #[cfg(any(unix, windows))]
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
/// DSV-010b: served on both Unix and Windows (the save-time daemon answers over
/// the Windows named pipe and loads its operator antipattern config there too).
/// The body is the same `config_dir_from` resolver `config_path` uses, so it is
/// platform-neutral; the resolved file is read through the per-platform
/// owner-only [`read_trusted`] on both targets.
#[cfg(any(unix, windows))]
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

/// Windows (DSV-010b): the owner-only trusted read, the analogue of the Unix
/// `O_NOFOLLOW` + owner-uid check. Refuses a reparse point (symlink/junction →
/// [`ConfinementError::SymlinkedConfig`]) and a file owned by another principal
/// ([`ConfinementError::NotOwnerSid`]); reads the verified handle otherwise. The
/// unsafe `GetSecurityInfo` / reparse-detection FFI is quarantined in
/// `anvil-intercept-win32` so this crate keeps `forbid(unsafe_code)`.
#[cfg(windows)]
pub(crate) fn read_trusted(path: &Path) -> Result<Option<String>, ConfinementError> {
    use anvil_intercept_win32::TrustedConfigRead;
    match anvil_intercept_win32::read_trusted_config(path) {
        Ok(TrustedConfigRead::NotFound) => Ok(None),
        Ok(TrustedConfigRead::Reparse) => {
            Err(ConfinementError::SymlinkedConfig(path.to_path_buf()))
        }
        Ok(TrustedConfigRead::NotOwner {
            owner_sid,
            current_sid,
        }) => Err(ConfinementError::NotOwnerSid {
            path: path.to_path_buf(),
            owner_sid,
            current_sid,
        }),
        Ok(TrustedConfigRead::Trusted(raw)) => Ok(Some(raw)),
        Err(source) => Err(ConfinementError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Exotic non-Unix/-Windows targets have no trusted-read primitive; plain read.
#[cfg(not(any(unix, windows)))]
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
    Ok(Confinement::from_file(parse_config_file(&raw, path)?))
}

/// Parse a confinement config body with **version-gated** unknown-key handling
/// (ACTMO-019). The single parse path shared by [`load_from`] and
/// [`read_config_file_from`] so the runtime policy and the CLI's editable view
/// never disagree on what a config means.
///
/// - At or below [`CURRENT_CONFIG_VERSION`] (which covers every pre-ACTMO-019
///   file, implicitly version `1`): parse *strictly*. `deny_unknown_fields`
///   turns a misspelt key into a loud [`ConfinementError::Parse`] rather than a
///   silent permissive default — the typo-protection contract is unchanged.
/// - Above it: a newer Anvil wrote this file. Drop top-level keys this daemon
///   does not know (logged, never silent) and parse the remainder, so a
///   newer-format config never fails an older daemon **closed** and collapses
///   the confinement trust floor (ADR-094 decision 5).
fn parse_config_file(raw: &str, path: &Path) -> Result<ConfinementConfigFile, ConfinementError> {
    let parse_err = |source| ConfinementError::Parse {
        path: path.to_path_buf(),
        source,
    };

    // Peek at the declared format version without committing to the strict
    // typed shape, so a forward-version file can be downgraded to a lenient
    // parse. Keep the parsed value to reuse below instead of parsing twice. An
    // unreadable document falls through to the strict parse, which produces the
    // canonical error.
    let peeked: Option<serde_yaml::Value> = serde_yaml::from_str(raw).ok();
    let declared_version = peeked
        .as_ref()
        .and_then(serde_yaml::Value::as_mapping)
        .and_then(|map| map.get("version"))
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(u64::from(DEFAULT_CONFIG_VERSION));

    if declared_version <= u64::from(CURRENT_CONFIG_VERSION) {
        return serde_yaml::from_str(raw).map_err(parse_err);
    }

    // Newer format than this daemon understands: keep only known keys, then
    // parse the cleaned mapping (so `deny_unknown_fields` sees nothing foreign).
    // Consume the already-parsed value so kept entries move in without cloning.
    let Some(serde_yaml::Value::Mapping(mapping)) = peeked else {
        // A non-mapping at a future version is still nonsense — let the strict
        // parse produce the canonical, well-formed error.
        return serde_yaml::from_str(raw).map_err(parse_err);
    };
    let mut clean = serde_yaml::Mapping::new();
    let mut dropped: Vec<String> = Vec::new();
    for (key, val) in mapping {
        if key
            .as_str()
            .is_some_and(|name| KNOWN_CONFIG_KEYS.contains(&name))
        {
            clean.insert(key, val);
        } else {
            dropped.push(
                key.as_str()
                    .map_or_else(|| format!("{key:?}"), str::to_owned),
            );
        }
    }
    if !dropped.is_empty() {
        tracing::warn!(
            target: "anvil_intercept::confinement",
            path = %path.display(),
            version = declared_version,
            current = CURRENT_CONFIG_VERSION,
            dropped = ?dropped,
            "confinement config is a newer format version than this daemon \
             understands — ignoring unknown keys (forward-compat); confinement \
             trust floor preserved",
        );
    }
    serde_yaml::from_value(serde_yaml::Value::Mapping(clean)).map_err(parse_err)
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
    parse_config_file(&raw, path)
}

/// Read the on-disk config file at the resolved [`config_path`].
pub fn read_config_file() -> Result<ConfinementConfigFile, ConfinementError> {
    read_config_file_from(&config_path()?)
}

/// ACTMO-019: load the operator's `register_on_start` worktree list from the
/// confinement config at `path`. The daemon calls this at startup to durably
/// register the configured worktrees (atop the persisted ACTMO-014 set). A
/// missing config or an empty key yields an empty list; a malformed/untrusted
/// config surfaces a loud `Err` the caller logs (the daemon proceeds with the
/// persisted set rather than failing startup — per-connection admission
/// independently fails closed on a bad config).
pub fn load_register_on_start_from(path: &Path) -> Result<Vec<PathBuf>, ConfinementError> {
    Ok(read_config_file_from(path)?.register_on_start)
}

/// Load `register_on_start` from the resolved [`config_path`].
pub fn load_register_on_start() -> Result<Vec<PathBuf>, ConfinementError> {
    load_register_on_start_from(&config_path()?)
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
    // Refuse to clobber a config newer than this binary understands: the lenient
    // read dropped its unknown keys, so writing would silently lose them.
    if file.version > CURRENT_CONFIG_VERSION {
        return Err(ConfinementError::FutureConfigVersion {
            version: file.version,
            current: CURRENT_CONFIG_VERSION,
        });
    }
    reject_root_allow_entries(file)?;
    reject_relative_register_on_start(file)?;
    if let Some(parent) = path.parent() {
        create_owner_only_dir(parent)?;
    }
    let body = serde_yaml::to_string(file).map_err(ConfinementError::Serialize)?;
    write_atomic_owner_only(path, body.as_bytes())
}

/// Reject a `register_on_start` entry that is not absolute. The daemon resolves
/// these against its own cwd at startup, so a relative entry would register a
/// surprising directory; the CLI only ever stores canonical absolute roots, so a
/// relative entry can only be a hand-edit mistake.
fn reject_relative_register_on_start(file: &ConfinementConfigFile) -> Result<(), ConfinementError> {
    if let Some(entry) = file.register_on_start.iter().find(|p| !p.is_absolute()) {
        return Err(ConfinementError::RelativeRegisterOnStart(entry.clone()));
    }
    Ok(())
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

/// Windows (DSV-010b): create the config dir with an owner-only DACL — the
/// analogue of the Unix 0700 dir. The unsafe `CreateDirectoryW` + SDDL FFI is
/// quarantined in `anvil-intercept-win32`.
#[cfg(windows)]
fn create_owner_only_dir(dir: &Path) -> Result<(), ConfinementError> {
    anvil_intercept_win32::create_owner_only_dir(dir).map_err(|source| ConfinementError::Io {
        path: dir.to_path_buf(),
        source,
    })
}

#[cfg(not(any(unix, windows)))]
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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

    // ----------------------------------------------------------------
    // ACTMO-019: `register_on_start` key + format-version forward-compat.
    // ----------------------------------------------------------------

    #[test]
    fn register_on_start_roundtrips_and_bumps_version() {
        // Adding a `register_on_start` entry bumps the config version to 2 and
        // round-trips through write/read; the runtime `Confinement` is
        // unaffected (registration membership is a distinct set from admission).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        let mut file = ConfinementConfigFile::default();
        assert_eq!(file.version, DEFAULT_CONFIG_VERSION);
        assert!(file.add_register_on_start(PathBuf::from("/srv/wt-a")));
        assert!(file.add_register_on_start(PathBuf::from("/srv/wt-b")));
        // Idempotent — re-adding the same path is a no-op.
        assert!(!file.add_register_on_start(PathBuf::from("/srv/wt-a")));
        assert_eq!(file.version, CURRENT_CONFIG_VERSION);

        write_config_file_to(&path, &file).expect("write");
        let raw = std::fs::read_to_string(&path).expect("read raw");
        assert!(raw.contains("version: 2"), "version is serialised: {raw}");
        assert!(
            raw.contains("register_on_start:"),
            "the key is serialised: {raw}"
        );

        let back = read_config_file_from(&path).expect("read");
        assert_eq!(back, file);
        assert_eq!(
            load_register_on_start_from(&path).expect("load list"),
            vec![PathBuf::from("/srv/wt-a"), PathBuf::from("/srv/wt-b")]
        );

        // Admission is untouched by registration membership.
        assert_eq!(load_from(&path).expect("load"), Confinement::open_default());
    }

    #[test]
    fn pure_confinement_config_stays_version_1_and_byte_compatible() {
        // A config with no `register_on_start` must NOT gain a `version:` key,
        // so it stays byte-compatible with a pre-ACTMO-019 daemon.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        let mut file = ConfinementConfigFile::default();
        file.upsert_allow(PathBuf::from("/srv/x"), MatchKind::Exact);
        write_config_file_to(&path, &file).expect("write");

        let raw = std::fs::read_to_string(&path).expect("read raw");
        assert!(
            !raw.contains("version:"),
            "version omitted at the default: {raw}"
        );
        assert!(
            !raw.contains("register_on_start:"),
            "empty key omitted: {raw}"
        );

        // Removing the last entry drops the version back to the default.
        let mut bumped = ConfinementConfigFile::default();
        bumped.add_register_on_start(PathBuf::from("/srv/wt"));
        assert_eq!(bumped.version, CURRENT_CONFIG_VERSION);
        assert!(bumped.remove_register_on_start(Path::new("/srv/wt")));
        assert_eq!(
            bumped.version, DEFAULT_CONFIG_VERSION,
            "version drops back when the set empties"
        );
    }

    #[test]
    fn typo_still_fails_closed_at_known_version() {
        // The format-version scheme must NOT weaken typo protection at the known
        // version: a misspelt key is still a loud parse error (deny_unknown_fields),
        // even alongside a valid `register_on_start` key.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        write_owner_only(
            &path,
            "version: 2\nadmissoin: allowlist\nregister_on_start:\n  - /srv/wt\n",
        );
        let err = load_from(&path).expect_err("a typo at the known version must still error");
        assert!(
            matches!(err, ConfinementError::Parse { .. }),
            "expected Parse (deny_unknown_fields preserved), got {err:?}"
        );
    }

    #[test]
    fn newer_format_version_does_not_fail_closed() {
        // ADR-094 decision 5 forward-compat: a config written by a NEWER Anvil
        // (a higher format version carrying a key this daemon does not know) must
        // be read leniently — the unknown key is dropped, the known fields load,
        // and confinement is NOT collapsed to fail-closed. This is the mechanism
        // that stops a `register_on_start`-bearing config from breaking an older
        // daemon's trust floor on a version skew.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        write_owner_only(
            &path,
            "version: 99\nadmission: allowlist\nallow:\n  - path: /srv/x\n\
             register_on_start:\n  - /srv/wt\nsome_future_key:\n  nested: value\n",
        );

        // The known fields parse; the unknown future key is ignored, not fatal.
        let file = read_config_file_from(&path).expect("a newer-version config loads leniently");
        assert_eq!(file.admission, AdmissionModeFile::Allowlist);
        assert_eq!(
            file.allow.len(),
            1,
            "the allow entry survives the lenient parse"
        );
        assert_eq!(file.register_on_start, vec![PathBuf::from("/srv/wt")]);

        // Crucially, the runtime admission policy is the file's `allowlist` with
        // its allow entry, NOT the fail-closed posture (allowlist + EMPTY allow)
        // — an older daemon keeps serving its configured roots.
        let confinement = load_from(&path).expect("lenient load");
        assert_eq!(confinement.mode(), AdmissionModeFile::Allowlist);
        assert_eq!(confinement.allow_count(), 1);
        assert_ne!(
            confinement,
            Confinement::fail_closed(),
            "a forward-version config must not collapse to fail-closed"
        );
    }

    #[test]
    fn write_refuses_a_future_version_config() {
        // Council M-2: a config read at a higher (future) format version had its
        // unknown keys dropped on the lenient read, so writing it back would lose
        // them. Refuse loudly rather than silently downgrade + drop.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        let mut file = ConfinementConfigFile {
            version: 99,
            ..Default::default()
        };
        // Mutating register_on_start must NOT downgrade the version below 99...
        assert!(file.add_register_on_start(PathBuf::from("/srv/wt")));
        assert_eq!(file.version, 99, "add never downgrades a future version");
        // ...and the write refuses it rather than dropping the future keys.
        let err =
            write_config_file_to(&path, &file).expect_err("future-version write must be refused");
        assert!(
            matches!(
                err,
                ConfinementError::FutureConfigVersion { version: 99, .. }
            ),
            "expected FutureConfigVersion, got {err:?}"
        );
        assert!(!path.exists(), "nothing is written when refused");
    }

    #[test]
    fn write_rejects_relative_register_on_start_entry() {
        // Council m-2: a hand-edited relative entry would resolve against the
        // daemon's cwd at startup — refuse it on write.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.yaml");
        let mut file = ConfinementConfigFile::default();
        file.add_register_on_start(PathBuf::from("../etc"));
        let err = write_config_file_to(&path, &file)
            .expect_err("relative register_on_start must be refused");
        assert!(
            matches!(err, ConfinementError::RelativeRegisterOnStart(_)),
            "expected RelativeRegisterOnStart, got {err:?}"
        );
        assert!(!path.exists(), "nothing is written when refused");
    }
}
