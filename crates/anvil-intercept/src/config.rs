//! INTD-008: daemon-side enforcement config loader.
//!
//! Reads the `enforcement` block from the project `.anvil.yaml` and an
//! optional user-level config, merges with **stricter-wins** semantics,
//! and resolves the daemon's runtime enforcement policy. The wire shape
//! is owned by `anvil_intercept_proto::enforcement_config` so the MCP
//! launch shim (RTAI-006) and the daemon cannot drift on which keys /
//! aliases are accepted; this module only owns IO, defaults, merging,
//! and the daemon-specific resolved enum.
//!
//! See `plans/decisions/015-intercept-loop-enforcement.md` (AD-3) for
//! the policy contract and `plans/modules/intercept-daemon.aps.md`
//! INTD-008 for the work-item scope.
//!
//! ## Shared enforcement posture (ADR-098 AD-3)
//!
//! Since ADR-098 AD-3 the daemon [`Mode`] is the shared
//! [`anvil_kernel_types::EnforcementMode`] — the daemon and the MCP shim
//! (`crates/anvil-cli/src/mcp/enforcement.rs`) both fold into one posture
//! type with one alias table, so the two surfaces can no longer drift on
//! which values / aliases they accept. The single alias table lives in
//! kernel-types; this module owns only IO, defaults, and the merge.
//!
//! Daemon resolution (via the shared [`Mode::parse`]):
//!
//! | Raw YAML value         | Resolved [`Mode`]        |
//! | ---------------------- | ------------------------ |
//! | `off` / `advisory` /   | `Mode::Off`              |
//! | `proceed`              | (real posture now)       |
//! | `warn`                 | `Mode::Warn`             |
//! | `fence`                | `Mode::Fence`            |
//! | `interrupt` / `block`  | `Mode::Interrupt`        |
//! | unknown / missing      | default (`Mode::Warn`)   |
//!
//! `off` is a real posture now — it projects to always-`Allow` in the
//! embedded pipeline. Before AD-3 the daemon clamped `off`/`advisory`/
//! `proceed` to `Warn` because it had no `off` branch; the shared type
//! gives it one, and the `fence` / `interrupt` distinction (previously
//! collapsed to `Block` at parse time by the MCP shim) is preserved for
//! action-time projection.
//!
//! The daemon's default mode is `Warn` — observe-only-style
//! defaults align with the planless-first principle. Operators
//! opt into fencing or interruption; the daemon does not invent
//! enforcement.
//!
//! ## Stricter-wins merge
//!
//! Project `.anvil.yaml` (`<workspace_root>/.anvil.yaml`) and the
//! optional user-level `.anvil.yaml`
//! (`$XDG_CONFIG_HOME/anvil/anvil.yaml` or platform equivalent —
//! resolved by the caller, not this module) are merged so the
//! **stricter** of the two wins per field:
//!
//! - `mode`: `Interrupt > Fence > Warn`. The strictest project /
//!   user mode wins. This prevents a user-level config from
//!   weakening a project's enforcement.
//! - `on_ambiguous_ownership`: `Fence > Warn`. The strictest wins.
//!   In addition, the resolved value is **hard-capped at `Fence`**
//!   regardless of either input — AD-3's invariant.
//! - `observe_only`: `false > true`. If either side requests
//!   active enforcement (`observe_only: false`), the daemon
//!   enforces. The "weaker" choice (`observe_only: true`) only
//!   wins when both sides agree (or default is unset).

use std::path::{Path, PathBuf};

use anvil_intercept_proto::enforcement_config::{
    AnvilConfigFile, EnforcementConfigFile, TelemetryConfigFile,
};
use thiserror::Error;

/// MLP2-024 default per-worktree session cap. Sized for ~6
/// concurrent sub-agents with ~3x headroom per the trace evidence
/// in MLP-014; operators can tighten via `.anvil.yaml`. See
/// `SessionConfigFile::per_worktree_max` for the rationale.
pub const DEFAULT_PER_WORKTREE_MAX: usize = 16;

/// The daemon's resolved enforcement posture.
///
/// Since ADR-098 AD-3 this is the shared
/// [`anvil_kernel_types::EnforcementMode`] — the daemon `Mode` and the
/// MCP shim's `EnforcementMode` both fold into it, with one alias table
/// and stricter-wins `Ord` (`off < warn < fence < interrupt`). The alias
/// keeps the `config::Mode` name at the daemon call sites. `parse`,
/// `as_str`, `stricter`, and the `Default` (`Warn`, ADR-002) all live on
/// the shared type; `off` is a real posture now (previously clamped to
/// `warn` because the daemon had no `off` branch) and projects to
/// always-`Allow` in the embedded pipeline (`downgrade_decision_if_observe`).
pub use anvil_kernel_types::EnforcementMode as Mode;

/// Resolved policy for ambiguous-ownership change events.
///
/// Per AD-3, ambiguous ownership is **hard-capped at `Fence`** —
/// the daemon never interrupts a process it cannot confidently
/// attribute. [`Resolved::on_ambiguous_ownership`] enforces this
/// invariant regardless of operator config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum AmbiguousOwnership {
    /// Tag the change as `attribution:unknown-agent` and surface
    /// a warning. Default — keeps the daemon honest about
    /// uncertainty without escalating.
    #[default]
    Warn,
    /// Fence the worktree on ambiguous ownership.
    Fence,
}

impl AmbiguousOwnership {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "warn" => Some(Self::Warn),
            "fence" => Some(Self::Fence),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Fence => "fence",
        }
    }

    #[must_use]
    pub fn stricter(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }
}

/// Errors raised when loading enforcement config.
///
/// Note: missing files and unknown mode strings do **not** produce
/// errors — they fall back to defaults so a fresh workspace, a
/// typo in `mode`, or a partially-written config never wedges the
/// daemon. Only structural YAML failures surface here.
#[derive(Debug, Error)]
pub enum LoadError {
    /// `.anvil.yaml` exists but is not parseable as YAML. The
    /// daemon refuses to start rather than silently falling back —
    /// an unparseable workspace config is operator error and a
    /// silent default would mask it. Tests cover the path.
    #[error("failed to parse {} as YAML: {source}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    /// IO error other than `NotFound` while reading the config
    /// file. Permission errors, etc. Missing files are silently
    /// treated as "no config provided" and fall back to defaults.
    #[error("failed to read {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Resolved daemon enforcement policy.
///
/// Represents the post-merge, post-cap state the daemon's
/// enforcement pipeline reads from. Construct via
/// [`Resolved::from_config_files`] or [`Resolved::load`]; the
/// fields are public for inspection but should not be mutated
/// after resolution — the cap invariants (ambiguous ownership
/// hard-capped at `Fence`) live in the constructor.
///
/// `Default` matches the daemon's no-config baseline:
/// `mode = Warn`, `on_ambiguous_ownership = Warn`,
/// `observe_only = false`,
/// `telemetry_allow_cross_session = false`,
/// `ipc_limits = IpcLimits::default()` (INTD-016 baseline).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub mode: Mode,
    pub on_ambiguous_ownership: AmbiguousOwnership,
    pub observe_only: bool,
    /// INTD-015: when `true`, telemetry subscribers see a redacted
    /// envelope (`rule_id` + `hash_of_path`) for cross-session
    /// events. Default `false` — cross-session events are dropped
    /// entirely, matching the 2026-04-24 council review M5
    /// (security-analyst) safe default.
    ///
    /// Stricter-wins merge: any side requesting `false` (deny)
    /// wins over the other side's `true`. The opt-in is the
    /// weaker choice.
    pub telemetry_allow_cross_session: bool,
    /// INTD-016: `DoS` protection budgets resolved from
    /// `enforcement.dos.*`. The daemon's IPC listener reads these
    /// at startup; weakening (`max_connections = 0`, etc.) is
    /// clamped inside `IpcLimits::from_config`. Project + user
    /// merge for these fields uses **stricter-wins**: a smaller
    /// `max_connections`, a smaller `rps_*`, a smaller
    /// `*_timeout_seconds`, and a smaller `control_frame_max_bytes`
    /// each represent the more restrictive enforcement posture.
    pub ipc_limits: crate::dos::IpcLimits,
    /// MLP2-024: maximum number of sessions allowed per
    /// canonicalised worktree. Stricter-wins merge (smaller value
    /// wins). The resolution layer clamps to a minimum of 1 — a
    /// `0` value would refuse every registration and is treated
    /// as an operator typo. Default (both sides unset): 16, sized
    /// for ~6 concurrent sub-agents with 3x headroom (per
    /// `SessionConfigFile::per_worktree_max` doc).
    pub session_per_worktree_max: usize,
}

impl Resolved {
    /// Resolve from a project + user pair of raw config files,
    /// applying stricter-wins merging and the ambiguous-ownership
    /// cap invariant.
    ///
    /// `None` on either side means "no file present" or "no
    /// `enforcement` block present" — the corresponding fields
    /// fall through to the other side, then to the daemon default.
    ///
    /// INTD-015's `telemetry.allow_cross_session` flag defaults to
    /// `false` — see [`Resolved::from_config_files_with_telemetry`]
    /// for the variant that consumes the telemetry block.
    #[must_use]
    pub fn from_config_files(
        project: Option<&EnforcementConfigFile>,
        user: Option<&EnforcementConfigFile>,
    ) -> Self {
        Self::from_config_files_with_telemetry(project, user, None, None)
    }

    /// Variant of [`Resolved::from_config_files`] that also
    /// consumes the project + user `telemetry` blocks (INTD-015).
    /// Used by [`Resolved::load`].
    #[must_use]
    pub fn from_config_files_with_telemetry(
        project: Option<&EnforcementConfigFile>,
        user: Option<&EnforcementConfigFile>,
        project_telemetry: Option<&TelemetryConfigFile>,
        user_telemetry: Option<&TelemetryConfigFile>,
    ) -> Self {
        let project_mode = project
            .and_then(|p| p.mode.as_deref())
            .and_then(Mode::parse);
        let user_mode = user.and_then(|u| u.mode.as_deref()).and_then(Mode::parse);

        // Stricter-wins: take the highest-strictness value present.
        // If neither side declared a mode, fall back to default.
        let mode = match (project_mode, user_mode) {
            (Some(p), Some(u)) => p.stricter(u),
            (Some(m), None) | (None, Some(m)) => m,
            (None, None) => Mode::default(),
        };

        let project_amb = project
            .and_then(|p| p.on_ambiguous_ownership.as_deref())
            .and_then(AmbiguousOwnership::parse);
        let user_amb = user
            .and_then(|u| u.on_ambiguous_ownership.as_deref())
            .and_then(AmbiguousOwnership::parse);
        let on_ambiguous_ownership = match (project_amb, user_amb) {
            (Some(p), Some(u)) => p.stricter(u),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => AmbiguousOwnership::default(),
        };

        let project_observe = project.and_then(|p| p.observe_only);
        let user_observe = user.and_then(|u| u.observe_only);
        // Stricter-wins for `observe_only`: if either side requests
        // active enforcement (`false`), enforcement is on. The
        // weaker `true` only wins if at least one side opts in and
        // no side opts out. Default (both sides absent) is active
        // enforcement — operators must explicitly opt into the
        // dry-run path.
        let observe_only = if project_observe == Some(false) || user_observe == Some(false) {
            false
        } else {
            project_observe.or(user_observe).unwrap_or(false)
        };

        // INTD-015: stricter-wins for `allow_cross_session`. The
        // safe choice is `false` (deny). If either side requests
        // deny, the resolved value is deny.
        let project_cross = project_telemetry.and_then(|t| t.allow_cross_session);
        let user_cross = user_telemetry.and_then(|t| t.allow_cross_session);
        let telemetry_allow_cross_session =
            if project_cross == Some(false) || user_cross == Some(false) {
                false
            } else {
                project_cross.or(user_cross).unwrap_or(false)
            };

        let ipc_limits = resolve_ipc_limits(project.map(|p| &p.dos), user.map(|u| &u.dos));

        // MLP2-024: stricter-wins on `per_worktree_max` (smaller
        // value wins). `None` on both sides → daemon default of
        // 16. A `0` value on either side is clamped to 1 — operator
        // typo defence (matches `IpcLimits::from_config` pattern;
        // refusing every registration is never the desired outcome).
        let project_cap = project.and_then(|p| p.session.per_worktree_max);
        let user_cap = user.and_then(|u| u.session.per_worktree_max);
        let merged_cap = match (project_cap, user_cap) {
            (Some(p), Some(u)) => Some(p.min(u)),
            (Some(v), None) | (None, Some(v)) => Some(v),
            (None, None) => None,
        };
        let session_per_worktree_max = merged_cap.unwrap_or(DEFAULT_PER_WORKTREE_MAX).max(1);

        Self {
            mode,
            on_ambiguous_ownership,
            observe_only,
            telemetry_allow_cross_session,
            ipc_limits,
            session_per_worktree_max,
        }
    }

    /// Resolve from project workspace root + optional user config
    /// file path. Reads `<workspace_root>/.anvil.yaml` and (if
    /// `Some`) the user config; merges; returns the resolved
    /// policy.
    ///
    /// Missing files are silently treated as "no config" — only
    /// malformed YAML or non-NotFound IO errors surface as
    /// `LoadError`.
    pub fn load(workspace_root: &Path, user_config_path: Option<&Path>) -> Result<Self, LoadError> {
        let project_path = workspace_root.join(".anvil.yaml");
        let project = read_config_file(&project_path)?;
        let user = match user_config_path {
            Some(path) => read_config_file(path)?,
            None => None,
        };
        Ok(Self::from_config_files_with_telemetry(
            project.as_ref().map(|p| &p.enforcement),
            user.as_ref().map(|u| &u.enforcement),
            project.as_ref().map(|p| &p.telemetry),
            user.as_ref().map(|u| &u.telemetry),
        ))
    }

    /// Map the resolved `telemetry_allow_cross_session` flag onto
    /// the [`crate::fanout::CrossSessionPolicy`] the fan-out
    /// reads. Centralised here so the policy mapping has a single
    /// source of truth — adding a future variant (e.g. per-rule
    /// allow, per-driver allow) only touches this function.
    #[must_use]
    pub fn cross_session_policy(&self) -> crate::fanout::CrossSessionPolicy {
        if self.telemetry_allow_cross_session {
            crate::fanout::CrossSessionPolicy::Redact
        } else {
            crate::fanout::CrossSessionPolicy::Deny
        }
    }
}

/// Resolve the daemon's enforcement config from
/// `<cwd>/.anvil.yaml`. The single load helper used by both daemon
/// entry points (`anvil intercept start` and the standalone
/// `anvil-intercept` binary) so the contract — propagate
/// `LoadError`, never silently fall back — is enforced in exactly
/// one place.
///
/// **Caller contract.** The returned `Err` MUST propagate to the
/// binary's exit code. Silently degrading to `Resolved::default()`
/// reintroduces the #1671-class wire-up gap that this loader exists
/// to close: the operator writes `enforcement.dos.*` or
/// `enforcement.session.per_worktree_max` in YAML, the daemon
/// parses it, then a downstream error throws the whole `Resolved`
/// out and the daemon silently runs at defaults. The
/// `LoadError::Parse` / `LoadError::Io` doc-comments spell this out;
/// the unit test
/// `load_for_daemon_cwd_propagates_parse_error_at_call_site` pins
/// it so a future refactor that re-introduces the silent fallback
/// trips a regression rather than shipping.
///
/// **Behaviour.**
///
/// * Missing `.anvil.yaml` → `Ok(Resolved::default())`. This is the
///   documented "no operator config" outcome, not a fallback —
///   `Resolved::load` folds `NotFound` into the `Ok` branch.
/// * Present and valid → `Ok(resolved)`.
/// * Present and malformed → `Err(LoadError::Parse)`. Fatal.
/// * IO error other than `NotFound` → `Err(LoadError::Io)`. Fatal.
/// * `std::env::current_dir()` fails → `Ok(Resolved::default())`
///   with an `eprintln!` diagnostic. This is an environment
///   condition (restricted chroot, removed CWD) — there is no
///   operator-supplied `.anvil.yaml` being silently overridden,
///   because the loader cannot locate one in the first place. The
///   distinction matters: it is the "no operator config" outcome,
///   not a parse failure on a present file.
pub fn load_for_daemon_cwd() -> Result<Resolved, LoadError> {
    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!(
                "anvil: cannot resolve CWD for config lookup ({err}); \
                 starting daemon on defaults"
            );
            return Ok(Resolved::default());
        }
    };
    load_for_daemon_cwd_at(&cwd)
}

/// Split of [`load_for_daemon_cwd`] that takes the CWD as an
/// argument so tests can pin the propagation contract without
/// touching `std::env::set_current_dir` (which is process-wide
/// state and forces test serialization).
pub(crate) fn load_for_daemon_cwd_at(cwd: &Path) -> Result<Resolved, LoadError> {
    Resolved::load(cwd, None)
}

impl Default for Resolved {
    /// Daemon's no-config baseline. Matches
    /// `from_config_files(None, None)` so `Resolved::default()`
    /// stays interchangeable with "operator wrote nothing
    /// anywhere" across the existing test suite.
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            on_ambiguous_ownership: AmbiguousOwnership::default(),
            observe_only: false,
            telemetry_allow_cross_session: false,
            ipc_limits: crate::dos::IpcLimits::default(),
            session_per_worktree_max: DEFAULT_PER_WORKTREE_MAX,
        }
    }
}

/// Stricter-wins merge for the INTD-016 `DoS` budgets. Each side may
/// declare `enforcement.dos.*`; the merge picks the **more
/// restrictive** value per field:
///
/// - `max_connections`: smaller wins (fewer simultaneous peers).
/// - `rps_sustained` / `rps_burst`: smaller wins (lower throughput).
/// - `handshake_timeout_seconds` / `idle_timeout_seconds`: smaller
///   wins (faster cut-off for slow / idle peers).
/// - `control_frame_max_bytes`: smaller wins (smaller attack
///   surface).
///
/// A field absent on both sides falls through to
/// [`crate::dos::IpcLimits::default`]. Clamping of unsafe values
/// (zero connection cap, etc.) happens inside
/// `IpcLimits::from_config` so this layer can stay symmetric.
fn resolve_ipc_limits(
    project: Option<&anvil_intercept_proto::enforcement_config::DosConfigFile>,
    user: Option<&anvil_intercept_proto::enforcement_config::DosConfigFile>,
) -> crate::dos::IpcLimits {
    use anvil_intercept_proto::enforcement_config::DosConfigFile;

    fn pick<T: Ord + Copy>(a: Option<T>, b: Option<T>) -> Option<T> {
        match (a, b) {
            (Some(x), Some(y)) => Some(x.min(y)),
            (Some(x), None) | (None, Some(x)) => Some(x),
            (None, None) => None,
        }
    }
    fn pick_f64(a: Option<f64>, b: Option<f64>) -> Option<f64> {
        match (a, b) {
            (Some(x), Some(y)) => Some(if x <= y { x } else { y }),
            (Some(x), None) | (None, Some(x)) => Some(x),
            (None, None) => None,
        }
    }

    let p = project.copied().unwrap_or_default();
    let u = user.copied().unwrap_or_default();

    let merged = DosConfigFile {
        max_connections: pick(p.max_connections, u.max_connections),
        rps_sustained: pick_f64(p.rps_sustained, u.rps_sustained),
        rps_burst: pick(p.rps_burst, u.rps_burst),
        handshake_timeout_seconds: pick(p.handshake_timeout_seconds, u.handshake_timeout_seconds),
        idle_timeout_seconds: pick(p.idle_timeout_seconds, u.idle_timeout_seconds),
        control_frame_max_bytes: pick(p.control_frame_max_bytes, u.control_frame_max_bytes),
    };
    crate::dos::IpcLimits::from_config(&merged)
}

fn read_config_file(path: &Path) -> Result<Option<AnvilConfigFile>, LoadError> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let parsed: AnvilConfigFile =
                serde_yaml::from_str(&content).map_err(|source| LoadError::Parse {
                    path: path.to_path_buf(),
                    source,
                })?;
            Ok(Some(parsed))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(LoadError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_anvil_yaml(dir: &Path, body: &str) {
        fs::write(dir.join(".anvil.yaml"), body).expect("write fixture");
    }

    fn write_user_config(path: &Path, body: &str) {
        fs::write(path, body).expect("write user config");
    }

    // -------- Mode parsing --------

    #[test]
    fn mode_parses_canonical_daemon_vocabulary() {
        assert_eq!(Mode::parse("warn"), Some(Mode::Warn));
        assert_eq!(Mode::parse("fence"), Some(Mode::Fence));
        assert_eq!(Mode::parse("interrupt"), Some(Mode::Interrupt));
    }

    #[test]
    fn mode_parses_block_alias_as_interrupt() {
        // Per the wave-1 brief: treat `block` and `interrupt` as
        // the same fence-on-error semantic. RTAI-006 collapses
        // both to `Block`; the daemon collapses both to
        // `Interrupt` (the strictest of the two — fence-on-error
        // PLUS process-group interrupt when attribution allows).
        assert_eq!(Mode::parse("block"), Some(Mode::Interrupt));
        assert_eq!(Mode::parse("BLOCK"), Some(Mode::Interrupt));
        assert_eq!(Mode::parse("  block  "), Some(Mode::Interrupt));
    }

    #[test]
    fn mode_parses_off_aliases_to_off_posture() {
        // ADR-098 AD-3: `off` is a real posture now (the shared type has
        // an `off` branch), so `off`/`advisory`/`proceed` resolve to
        // `Off` rather than the pre-AD-3 `Warn` clamp. `Off` projects to
        // always-`Allow` in the embedded pipeline.
        assert_eq!(Mode::parse("off"), Some(Mode::Off));
        assert_eq!(Mode::parse("advisory"), Some(Mode::Off));
        assert_eq!(Mode::parse("proceed"), Some(Mode::Off));
    }

    #[test]
    fn mode_unknown_returns_none() {
        assert_eq!(Mode::parse("nope"), None);
        assert_eq!(Mode::parse(""), None);
        // Common typos must not silently match.
        assert_eq!(Mode::parse("interupt"), None);
        assert_eq!(Mode::parse("fenc"), None);
    }

    #[test]
    fn mode_stricter_picks_highest_severity() {
        assert_eq!(Mode::Warn.stricter(Mode::Fence), Mode::Fence);
        assert_eq!(Mode::Fence.stricter(Mode::Warn), Mode::Fence);
        assert_eq!(Mode::Interrupt.stricter(Mode::Fence), Mode::Interrupt);
        assert_eq!(Mode::Fence.stricter(Mode::Interrupt), Mode::Interrupt);
        assert_eq!(Mode::Warn.stricter(Mode::Warn), Mode::Warn);
        // ADR-098 AD-3: `Off` is the weakest posture — `warn` wins over it.
        assert_eq!(Mode::Off.stricter(Mode::Warn), Mode::Warn);
        assert_eq!(Mode::Warn.stricter(Mode::Off), Mode::Warn);
        assert_eq!(Mode::Off.stricter(Mode::Off), Mode::Off);
    }

    #[test]
    fn off_and_warn_merge_picks_warn_either_order() {
        // ADR-098 AD-3: `off` is a real posture (weakest under
        // stricter-wins). A project↔user merge of `off` and `warn`
        // resolves to `warn` regardless of which side declares which — a
        // user's `off` cannot weaken a project's `warn`, and a user's
        // `warn` raises a project's `off`.
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        write_anvil_yaml(workspace.path(), "enforcement:\n  mode: off\n");
        write_user_config(&user_path, "enforcement:\n  mode: warn\n");
        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert_eq!(
            resolved.mode,
            Mode::Warn,
            "project off + user warn must resolve to warn",
        );

        write_anvil_yaml(workspace.path(), "enforcement:\n  mode: warn\n");
        write_user_config(&user_path, "enforcement:\n  mode: off\n");
        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert_eq!(
            resolved.mode,
            Mode::Warn,
            "project warn + user off must resolve to warn",
        );
    }

    #[test]
    fn ambiguous_ownership_parses_canonical_vocabulary() {
        assert_eq!(
            AmbiguousOwnership::parse("warn"),
            Some(AmbiguousOwnership::Warn)
        );
        assert_eq!(
            AmbiguousOwnership::parse("fence"),
            Some(AmbiguousOwnership::Fence),
        );
        assert_eq!(
            AmbiguousOwnership::parse("FENCE"),
            Some(AmbiguousOwnership::Fence)
        );
        assert_eq!(AmbiguousOwnership::parse("interrupt"), None);
    }

    // -------- Defaults / missing files --------

    #[test]
    fn missing_anvil_yaml_returns_default_resolved() {
        let workspace = tempdir().expect("workspace");
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert_eq!(resolved, Resolved::default());
        assert_eq!(resolved.mode, Mode::Warn);
        assert_eq!(resolved.on_ambiguous_ownership, AmbiguousOwnership::Warn,);
        assert!(!resolved.observe_only);
    }

    #[test]
    fn missing_user_config_path_falls_back_to_project_only() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(
            workspace.path(),
            "enforcement:\n  mode: fence\n  observe_only: true\n",
        );
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert_eq!(resolved.mode, Mode::Fence);
        assert!(resolved.observe_only);
    }

    #[test]
    fn nonexistent_user_config_path_does_not_error() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");
        // user_path intentionally not created — the daemon must
        // tolerate "user has no global anvil config yet".
        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert_eq!(resolved, Resolved::default());
    }

    // -------- Malformed YAML --------

    #[test]
    fn malformed_project_yaml_returns_parse_error() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(workspace.path(), "not valid: yaml: [");
        let err = Resolved::load(workspace.path(), None).expect_err("malformed yaml");
        match err {
            LoadError::Parse { path, .. } => {
                assert!(path.ends_with(".anvil.yaml"));
            }
            err @ LoadError::Io { .. } => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn malformed_user_yaml_returns_parse_error() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");
        write_user_config(&user_path, "not valid: yaml: [");
        let err =
            Resolved::load(workspace.path(), Some(&user_path)).expect_err("malformed user yaml");
        match err {
            LoadError::Parse { path, .. } => assert_eq!(path, user_path),
            err @ LoadError::Io { .. } => panic!("unexpected error: {err:?}"),
        }
    }

    // -------- Daemon-startup helper propagation contract --------

    /// #1671 audit closure: the daemon-startup helper
    /// [`load_for_daemon_cwd`] must propagate parse errors so a
    /// malformed `.anvil.yaml` fails daemon startup with a non-zero
    /// exit. Pre-PR-1721 the binary call sites did
    /// `match ... { Err(_) => Resolved::default() }` and a typo in
    /// YAML silently disabled every configured enforcement knob —
    /// the same "operator wrote a knob, daemon ignored it" bug
    /// class this loader exists to close.
    ///
    /// Exercised via the test-only `load_for_daemon_cwd_at(cwd)`
    /// split so the test does not need to `std::env::set_current_dir`
    /// (process-wide state that would force serialization across
    /// the suite).
    #[test]
    fn load_for_daemon_cwd_propagates_parse_error_at_call_site() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(workspace.path(), "not valid: yaml: [");

        let result = super::load_for_daemon_cwd_at(workspace.path());

        match result {
            Err(LoadError::Parse { path, .. }) => {
                assert!(
                    path.ends_with(".anvil.yaml"),
                    "parse error must name the offending file; got {}",
                    path.display()
                );
            }
            Ok(resolved) => panic!(
                "load_for_daemon_cwd_at MUST propagate a parse error from a malformed \
                 .anvil.yaml. Silent fallback to `Resolved::default()` reintroduces \
                 the #1671-class wire-up gap: the operator wrote a knob, the daemon \
                 ignored it. got Ok({resolved:?})"
            ),
            Err(other) => panic!("expected LoadError::Parse, got {other:?}"),
        }
    }

    /// Companion: a missing `.anvil.yaml` is **not** an error — it is
    /// the documented "no operator config" outcome (a fresh
    /// workspace, no operator-supplied policy). The helper folds it
    /// into `Ok(Resolved::default())` via `Resolved::load`'s
    /// `NotFound`-tolerant branch, distinct from the parse-failure
    /// path above.
    #[test]
    fn load_for_daemon_cwd_treats_missing_file_as_no_config() {
        let workspace = tempdir().expect("workspace");
        // Deliberately do NOT write `.anvil.yaml` — the directory is
        // empty.

        let resolved =
            super::load_for_daemon_cwd_at(workspace.path()).expect("missing file is not fatal");
        assert_eq!(
            resolved,
            Resolved::default(),
            "missing .anvil.yaml must yield the no-config baseline"
        );
    }

    // -------- Project + user merge with stricter-wins --------

    #[test]
    fn project_and_user_merge_picks_stricter_mode() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        write_anvil_yaml(workspace.path(), "enforcement:\n  mode: warn\n");
        write_user_config(&user_path, "enforcement:\n  mode: interrupt\n");

        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert_eq!(
            resolved.mode,
            Mode::Interrupt,
            "user-config interrupt must win over project-config warn",
        );
    }

    #[test]
    fn project_stricter_than_user_wins_over_user() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        write_anvil_yaml(workspace.path(), "enforcement:\n  mode: interrupt\n");
        write_user_config(&user_path, "enforcement:\n  mode: warn\n");

        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert_eq!(
            resolved.mode,
            Mode::Interrupt,
            "stricter project mode must win — user cannot weaken project policy",
        );
    }

    #[test]
    fn project_and_user_merge_picks_stricter_ambiguous_ownership() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        write_anvil_yaml(
            workspace.path(),
            "enforcement:\n  on_ambiguous_ownership: warn\n",
        );
        write_user_config(
            &user_path,
            "enforcement:\n  on_ambiguous_ownership: fence\n",
        );

        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert_eq!(resolved.on_ambiguous_ownership, AmbiguousOwnership::Fence,);
    }

    #[test]
    fn observe_only_false_wins_over_true() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        // Project sets observe_only: false (active enforcement);
        // user requests observe_only: true (dry run). Active
        // enforcement is the stricter choice — false wins.
        write_anvil_yaml(workspace.path(), "enforcement:\n  observe_only: false\n");
        write_user_config(&user_path, "enforcement:\n  observe_only: true\n");

        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert!(
            !resolved.observe_only,
            "any side requesting active enforcement disables observe_only",
        );
    }

    #[test]
    fn observe_only_true_kept_when_only_side_present() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        // Project says observe_only: true; user has no enforcement
        // block at all. The "weaker" (observe-only) choice wins
        // because nothing on the other side asks for active
        // enforcement.
        write_anvil_yaml(workspace.path(), "enforcement:\n  observe_only: true\n");
        write_user_config(&user_path, "enforcement: {}\n");

        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert!(resolved.observe_only);
    }

    // -------- Ambiguous-ownership hard cap --------

    #[test]
    fn ambiguous_ownership_does_not_promote_above_fence() {
        // The vocabulary itself caps at `fence` — the parse
        // function rejects `interrupt` and other strictness
        // overshoots. This test pins the invariant: even if a
        // hostile config attempts to escalate, the resolved
        // value never exceeds Fence. Combined with the parse
        // table, no operator-supplied value can promote ambiguous
        // ownership above Fence.
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(
            workspace.path(),
            "enforcement:\n  on_ambiguous_ownership: interrupt\n",
        );
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        // `interrupt` is not a valid value — falls back to the
        // default (Warn).
        assert_eq!(resolved.on_ambiguous_ownership, AmbiguousOwnership::Warn,);

        // Belt-and-braces: even if AmbiguousOwnership were to
        // grow new variants, `Fence` must remain the maximum.
        let max = AmbiguousOwnership::Warn.stricter(AmbiguousOwnership::Fence);
        assert_eq!(max, AmbiguousOwnership::Fence);
    }

    // -------- Reserved INTD-016 keys are forwards-compatible --------

    #[test]
    fn reserved_dos_keys_do_not_break_resolution() {
        let workspace = tempdir().expect("workspace");
        // INTD-016 will land DoS budgets at `enforcement.dos.*`.
        // The proto layer ignores unknown keys; INTD-008 must do
        // the same so workspaces preparing for INTD-016 don't
        // wedge today.
        write_anvil_yaml(
            workspace.path(),
            "enforcement:\n  mode: fence\n  dos:\n    max_connections: 32\n",
        );
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert_eq!(resolved.mode, Mode::Fence);
    }

    // -------- Unknown mode strings fall back to default --------

    #[test]
    fn unknown_mode_string_falls_back_to_default() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(workspace.path(), "enforcement:\n  mode: lenient\n");
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert_eq!(resolved.mode, Mode::default());
    }

    // -------- Direct from_config_files helper --------

    #[test]
    fn from_config_files_with_neither_side_returns_default() {
        let resolved = Resolved::from_config_files(None, None);
        assert_eq!(resolved, Resolved::default());
    }

    #[test]
    fn from_config_files_with_only_user_side_uses_user_values() {
        let user = EnforcementConfigFile {
            mode: Some("fence".to_string()),
            on_ambiguous_ownership: Some("fence".to_string()),
            observe_only: Some(true),
            ..EnforcementConfigFile::default()
        };
        let resolved = Resolved::from_config_files(None, Some(&user));
        assert_eq!(resolved.mode, Mode::Fence);
        assert_eq!(resolved.on_ambiguous_ownership, AmbiguousOwnership::Fence,);
        assert!(resolved.observe_only);
    }

    // -------- INTD-015 telemetry.allow_cross_session --------

    #[test]
    fn telemetry_allow_cross_session_defaults_to_deny() {
        let workspace = tempdir().expect("workspace");
        // No telemetry block at all → safe default.
        write_anvil_yaml(workspace.path(), "enforcement:\n  mode: warn\n");
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert!(
            !resolved.telemetry_allow_cross_session,
            "default MUST be deny — council finding M5 (security-analyst), 2026-04-24",
        );
        assert_eq!(
            resolved.cross_session_policy(),
            crate::fanout::CrossSessionPolicy::Deny,
        );
    }

    #[test]
    fn telemetry_allow_cross_session_true_maps_to_redact_policy() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(
            workspace.path(),
            "telemetry:\n  allow_cross_session: true\n",
        );
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert!(resolved.telemetry_allow_cross_session);
        assert_eq!(
            resolved.cross_session_policy(),
            crate::fanout::CrossSessionPolicy::Redact,
        );
    }

    #[test]
    fn telemetry_allow_cross_session_project_deny_overrides_user_allow() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        // Project explicitly denies cross-session sharing; user
        // tries to allow it. Stricter (deny) wins.
        write_anvil_yaml(
            workspace.path(),
            "telemetry:\n  allow_cross_session: false\n",
        );
        write_user_config(&user_path, "telemetry:\n  allow_cross_session: true\n");

        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert!(
            !resolved.telemetry_allow_cross_session,
            "project deny must win over user allow",
        );
    }

    // -------- MLP2-024 session.per_worktree_max --------

    #[test]
    fn session_per_worktree_max_defaults_to_sixteen_when_unset() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(workspace.path(), "enforcement:\n  mode: warn\n");
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert_eq!(resolved.session_per_worktree_max, DEFAULT_PER_WORKTREE_MAX);
        assert_eq!(resolved.session_per_worktree_max, 16);
    }

    #[test]
    fn session_per_worktree_max_honours_project_value() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(
            workspace.path(),
            "enforcement:\n  session:\n    per_worktree_max: 4\n",
        );
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert_eq!(resolved.session_per_worktree_max, 4);
    }

    #[test]
    fn session_per_worktree_max_stricter_wins_picks_smaller() {
        let workspace = tempdir().expect("workspace");
        let user_dir = tempdir().expect("user dir");
        let user_path = user_dir.path().join("anvil.yaml");

        write_anvil_yaml(
            workspace.path(),
            "enforcement:\n  session:\n    per_worktree_max: 8\n",
        );
        write_user_config(
            &user_path,
            "enforcement:\n  session:\n    per_worktree_max: 4\n",
        );
        let resolved = Resolved::load(workspace.path(), Some(&user_path)).expect("load");
        assert_eq!(
            resolved.session_per_worktree_max, 4,
            "smaller (stricter) value wins"
        );
    }

    #[test]
    fn session_per_worktree_max_zero_is_clamped_to_one() {
        let workspace = tempdir().expect("workspace");
        write_anvil_yaml(
            workspace.path(),
            "enforcement:\n  session:\n    per_worktree_max: 0\n",
        );
        let resolved = Resolved::load(workspace.path(), None).expect("load");
        assert_eq!(
            resolved.session_per_worktree_max, 1,
            "zero is operator-typo defence; clamp to 1"
        );
    }
}
