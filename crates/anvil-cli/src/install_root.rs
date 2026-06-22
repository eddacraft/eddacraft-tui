//! Install-root resolution for the `ANVIL_HOME` / `--anvil-home` override
//! (DISTRIB-006, [ADR-060]).
//!
//! anvil keeps two distinct categories of state:
//!
//! - **Install/user-owned** — user config (today under the home dir), the daemon
//!   socket/PID (under the runtime dir), and kernel cache/logs. When `ANVIL_HOME`
//!   (or `--anvil-home`, which takes precedence) is set, these re-root under the
//!   prefix: `<ANVIL_HOME>/user/`, `<ANVIL_HOME>/intercept.sock` +
//!   `<ANVIL_HOME>/intercept.pid`, and `<ANVIL_HOME>/cache/`. This lets a
//!   pre-release candidate run side-by-side with the production install without
//!   colliding on the user config, the daemon socket, or logs.
//! - **Per-project** — `<root>/.anvil/` (baseline, cache, witness) and
//!   `<root>/anvil/project-id`. Per ADR-060 **Option (a)**, `ANVIL_HOME` does
//!   **not** re-root project discovery: candidate tests run against the *real*
//!   repo so witness continuity and baseline durability are preserved. To contain
//!   the corruption risk that sharing otherwise carries, durable project-state
//!   *mutations* (baseline refresh/write, witness append, cutoff pinning) run in a
//!   read-only / dry-run posture under a non-default `ANVIL_HOME` unless the
//!   operator opts in with `--touch-project-state`.
//!
//! Unsetting `ANVIL_HOME` returns to platform-default behaviour byte-for-byte —
//! no path changes, no new fields, no guard — for the 99% of users who never set
//! it (ADR-060 §3).
//!
//! The `--anvil-home` and `--touch-project-state` flags become the canonical
//! `ANVIL_HOME` / `ANVIL_TOUCH_PROJECT_STATE` environment override at the top of
//! `main` (see `main::reexec_for_install_root`): the crate forbids `unsafe_code`,
//! so rather than `set_var` the flag, `main` re-execs once with the variable set
//! in the child environment. Every downstream resolution — including the spawned
//! daemon, which inherits the environment — then reads one source of truth. Pure
//! `*_from` helpers take the resolved environment explicitly so they unit-test
//! without mutating global state.
//!
//! [ADR-060]: ../../../plans/decisions/060-anvil-home-install-root-override.md

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Environment variable that re-roots install-owned state.
pub const ANVIL_HOME_ENV: &str = "ANVIL_HOME";

/// Environment variable (set by `--touch-project-state`) that opts a non-default
/// `ANVIL_HOME` session back into durable per-project mutations.
pub const TOUCH_PROJECT_STATE_ENV: &str = "ANVIL_TOUCH_PROJECT_STATE";

/// Resolved root for install-owned state.
///
/// `overridden` is `Some(prefix)` when `ANVIL_HOME` re-roots install state, and
/// `None` for the platform default (in which case callers keep their existing
/// path derivation untouched, preserving byte-for-byte default behaviour).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRoot {
    overridden: Option<PathBuf>,
}

impl InstallRoot {
    /// The platform-default root (no `ANVIL_HOME` override).
    #[must_use]
    pub fn default_root() -> Self {
        Self { overridden: None }
    }

    /// A root explicitly re-rooted under `prefix`.
    #[must_use]
    pub fn overridden(prefix: PathBuf) -> Self {
        Self {
            overridden: Some(prefix),
        }
    }

    /// `true` when `ANVIL_HOME` re-roots install state.
    #[must_use]
    pub fn is_overridden(&self) -> bool {
        self.overridden.is_some()
    }

    /// The override prefix, if any.
    #[must_use]
    pub fn prefix(&self) -> Option<&Path> {
        self.overridden.as_deref()
    }

    /// User-owned state directory under the prefix (`<ANVIL_HOME>/user/`), or
    /// `None` when callers should keep the platform default.
    #[must_use]
    pub fn user_dir(&self) -> Option<PathBuf> {
        self.overridden.as_ref().map(|p| p.join("user"))
    }

    /// Kernel cache/logs directory under the prefix (`<ANVIL_HOME>/cache/`), or
    /// `None` when callers should keep the platform default.
    #[must_use]
    pub fn cache_dir(&self) -> Option<PathBuf> {
        self.overridden.as_ref().map(|p| p.join("cache"))
    }
}

/// Resolve the install root from an explicit `ANVIL_HOME` value.
///
/// An unset or empty value is the platform default. A relative path is made
/// absolute against `cwd` so the CLI and the daemon (a separate process) agree on
/// the prefix; an absolute path is taken as-is. `ANVIL_HOME` is expected to be
/// absolute in practice — the `cwd` fallback only guards a directly-set relative
/// value.
#[must_use]
pub fn resolve_install_root_from(anvil_home: Option<&OsStr>, cwd: &Path) -> InstallRoot {
    let raw = match anvil_home {
        Some(raw) if !raw.is_empty() => raw,
        _ => return InstallRoot::default_root(),
    };
    // A whitespace-only value (e.g. an accidental `export ANVIL_HOME=" "`) is
    // treated as unset, so a blank export does not silently activate the override
    // and the write-guard against a useless path. Non-UTF-8 values can't be
    // trimmed and are taken as-is.
    if raw.to_str().is_some_and(|s| s.trim().is_empty()) {
        return InstallRoot::default_root();
    }
    let p = PathBuf::from(raw);
    let abs = if p.is_absolute() { p } else { cwd.join(p) };
    InstallRoot::overridden(abs)
}

/// Whether durable per-project mutations are gated (refused / dry-run) for this
/// session: true only under a non-default `ANVIL_HOME` without the
/// `--touch-project-state` opt-in.
#[must_use]
pub fn project_writes_gated_from(root: &InstallRoot, touch_project_state: Option<&OsStr>) -> bool {
    root.is_overridden() && !is_truthy(touch_project_state)
}

/// Truthiness for the opt-in env var: set and not one of the conventional
/// false-y spellings (`""`, `"0"`, `"false"`, `"no"`, `"off"` — case-insensitive).
fn is_truthy(value: Option<&OsStr>) -> bool {
    match value.and_then(OsStr::to_str) {
        None => false,
        Some(s) => {
            let s = s.trim();
            !s.is_empty()
                && !matches!(
                    s.to_ascii_lowercase().as_str(),
                    "0" | "false" | "no" | "off"
                )
        }
    }
}

/// Refusal returned when a durable per-project mutation is attempted under a
/// gated (`ANVIL_HOME`, no opt-in) session. The `Display` text is operator-facing
/// and mirrors the `--accept-suspicious` posture: name the blocked operation and
/// the exact opt-in to re-run with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWriteGated {
    operation: String,
    prefix: Option<PathBuf>,
}

impl ProjectWriteGated {
    #[must_use]
    pub fn new(operation: impl Into<String>, prefix: Option<PathBuf>) -> Self {
        Self {
            operation: operation.into(),
            prefix,
        }
    }
}

impl std::fmt::Display for ProjectWriteGated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let where_ = self
            .prefix
            .as_deref()
            .map(|p| format!(" (ANVIL_HOME={})", p.display()))
            .unwrap_or_default();
        write!(
            f,
            "Refusing {} under a non-default ANVIL_HOME{} — an unreleased \
             candidate must not silently mutate this project's durable state \
             (baseline / witness / cutoff). Re-run with --touch-project-state if \
             you deliberately want this candidate to write the real project.",
            self.operation, where_
        )
    }
}

impl std::error::Error for ProjectWriteGated {}

/// Gate a durable per-project mutation. Returns `Err(ProjectWriteGated)` when the
/// session is gated; `Ok(())` otherwise. `operation` names the blocked action for
/// the refusal message (e.g. `"baseline write"`, `"witness append"`).
///
/// # Errors
/// Returns [`ProjectWriteGated`] when running under a non-default `ANVIL_HOME`
/// without `--touch-project-state`.
pub fn ensure_project_write_allowed_from(
    root: &InstallRoot,
    touch_project_state: Option<&OsStr>,
    operation: &str,
) -> Result<(), ProjectWriteGated> {
    if project_writes_gated_from(root, touch_project_state) {
        Err(ProjectWriteGated::new(
            operation,
            root.prefix().map(Path::to_path_buf),
        ))
    } else {
        Ok(())
    }
}

// ---- environment-backed wrappers -------------------------------------------
// Thin shims that read the process environment and delegate to the pure helpers
// above. Production call sites use these; tests exercise the `*_from` functions.

/// Resolve the install root from the live process environment.
#[must_use]
pub fn install_root() -> InstallRoot {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_install_root_from(std::env::var_os(ANVIL_HOME_ENV).as_deref(), &cwd)
}

/// Whether `ANVIL_TOUCH_PROJECT_STATE` is set truthy in the live environment.
/// Used by the `main` re-exec guard to detect when the environment already
/// reflects `--touch-project-state`.
#[must_use]
pub fn env_touch_is_truthy() -> bool {
    is_truthy(std::env::var_os(TOUCH_PROJECT_STATE_ENV).as_deref())
}

/// Whether durable per-project mutations are gated for this process.
#[must_use]
pub fn project_writes_gated() -> bool {
    project_writes_gated_from(
        &install_root(),
        std::env::var_os(TOUCH_PROJECT_STATE_ENV).as_deref(),
    )
}

/// Gate a durable per-project mutation against the live environment.
///
/// # Errors
/// Returns [`ProjectWriteGated`] when running under a non-default `ANVIL_HOME`
/// without `--touch-project-state`.
pub fn ensure_project_write_allowed(operation: &str) -> Result<(), ProjectWriteGated> {
    ensure_project_write_allowed_from(
        &install_root(),
        std::env::var_os(TOUCH_PROJECT_STATE_ENV).as_deref(),
        operation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn os(s: &str) -> OsString {
        OsString::from(s)
    }

    #[test]
    fn unset_anvil_home_is_platform_default() {
        let root = resolve_install_root_from(None, Path::new("/work"));
        assert!(!root.is_overridden());
        assert_eq!(root.prefix(), None);
        assert_eq!(root.user_dir(), None);
        assert_eq!(root.cache_dir(), None);
    }

    #[test]
    fn empty_anvil_home_is_platform_default() {
        let root = resolve_install_root_from(Some(OsStr::new("")), Path::new("/work"));
        assert!(!root.is_overridden());
    }

    #[test]
    fn whitespace_only_anvil_home_is_platform_default() {
        for blank in ["   ", " ", "\t"] {
            let root = resolve_install_root_from(Some(OsStr::new(blank)), Path::new("/work"));
            assert!(!root.is_overridden(), "{blank:?} should not override");
        }
    }

    #[test]
    fn absolute_anvil_home_is_taken_as_is() {
        let root =
            resolve_install_root_from(Some(OsStr::new("/opt/anvil-beta")), Path::new("/work"));
        assert!(root.is_overridden());
        assert_eq!(root.prefix(), Some(Path::new("/opt/anvil-beta")));
        assert_eq!(root.user_dir(), Some(PathBuf::from("/opt/anvil-beta/user")));
        assert_eq!(
            root.cache_dir(),
            Some(PathBuf::from("/opt/anvil-beta/cache"))
        );
    }

    #[test]
    fn relative_anvil_home_is_absolutised_against_cwd() {
        let root = resolve_install_root_from(Some(OsStr::new("candidate")), Path::new("/work"));
        assert_eq!(root.prefix(), Some(Path::new("/work/candidate")));
    }

    #[test]
    fn default_root_does_not_gate_project_writes() {
        let root = InstallRoot::default_root();
        assert!(!project_writes_gated_from(&root, None));
        assert!(!project_writes_gated_from(&root, Some(OsStr::new("1"))));
        assert!(ensure_project_write_allowed_from(&root, None, "baseline write").is_ok());
    }

    #[test]
    fn overridden_root_gates_project_writes_without_opt_in() {
        let root = InstallRoot::overridden(PathBuf::from("/opt/anvil-beta"));
        assert!(project_writes_gated_from(&root, None));
        let err = ensure_project_write_allowed_from(&root, None, "witness append")
            .expect_err("must be gated");
        let msg = err.to_string();
        assert!(msg.contains("witness append"), "names the operation: {msg}");
        assert!(
            msg.contains("--touch-project-state"),
            "names the opt-in: {msg}"
        );
        assert!(msg.contains("/opt/anvil-beta"), "names the prefix: {msg}");
    }

    #[test]
    fn touch_project_state_opt_in_permits_writes() {
        let root = InstallRoot::overridden(PathBuf::from("/opt/anvil-beta"));
        for truthy in ["1", "true", "yes", "anything"] {
            assert!(
                !project_writes_gated_from(&root, Some(&os(truthy))),
                "{truthy} should permit writes"
            );
            assert!(
                ensure_project_write_allowed_from(&root, Some(&os(truthy)), "baseline write")
                    .is_ok()
            );
        }
    }

    #[test]
    fn falsey_opt_in_still_gates() {
        let root = InstallRoot::overridden(PathBuf::from("/opt/anvil-beta"));
        for falsey in ["", "0", "false", "FALSE", "no", "off", "OFF", "  "] {
            assert!(
                project_writes_gated_from(&root, Some(&os(falsey))),
                "{falsey:?} should still gate"
            );
        }
    }
}
