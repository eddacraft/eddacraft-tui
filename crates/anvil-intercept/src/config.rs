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
//! ## Mode reconciliation with RTAI-006
//!
//! RTAI-006 in `crates/anvil-cli/src/mcp/enforcement.rs` resolves to a
//! 3-variant enum (`Block | Warn | Off`) because the MCP
//! `validate_write` tool only needs three decision modes. The daemon
//! distinguishes "fence the worktree" from "interrupt the process
//! group" — both are fence-on-error semantics, but the daemon picks
//! between them based on whether attribution is certain.
//!
//! Wave 1 brief: treat `block` and `interrupt` as the same
//! fence-on-error semantic. The daemon canonical vocabulary is
//! `warn | fence | interrupt`; the alias table accepted at this
//! layer matches the proto-layer doc and is duplicated in the
//! `Mode::parse` match arm.
//!
//! Daemon resolution (this module):
//!
//! | Raw YAML value         | Resolved [`Mode`]        |
//! | ---------------------- | ------------------------ |
//! | `warn`                 | `Mode::Warn`             |
//! | `fence`                | `Mode::Fence`            |
//! | `interrupt` / `block`  | `Mode::Interrupt`        |
//! | `off` / `advisory` /   | `Mode::Warn` (clamped —  |
//! | `proceed`              | INTD has no "off"        |
//! |                        | branch)                  |
//! | unknown / missing      | default (`Mode::Warn`)   |
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

/// Resolved enforcement strictness for the daemon.
///
/// Variants are ordered for stricter-wins comparison. Do not rely on
/// the discriminant values — use [`Mode::stricter`] to merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Mode {
    /// Diagnostics are surfaced as warnings; no fence, no interrupt.
    /// The daemon's default — operators opt into stricter modes.
    #[default]
    Warn,
    /// On rule violation, the daemon fences the worktree. Active
    /// agent processes are not signalled; subsequent registrations
    /// against the fenced worktree are refused.
    Fence,
    /// On rule violation, the daemon issues a process-group
    /// interrupt against the attributing session, then fences. Only
    /// honoured when attribution is certain — ambiguous ownership
    /// is hard-capped at `Fence` per AD-3.
    Interrupt,
}

impl Mode {
    /// Parse a raw `.anvil.yaml` value into the daemon's mode enum.
    /// Case-insensitive; whitespace trimmed. Unknown values return
    /// `None` so the caller can fall back to the default.
    ///
    /// The alias table matches `EnforcementConfigFile.mode` doc
    /// comment in `anvil_intercept_proto::enforcement_config`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            // RTAI-006-only aliases (off / advisory / proceed) are
            // accepted by the daemon vocabulary but clamped to Warn
            // — INTD has no "off" branch by spec, so a workspace
            // mistakenly setting `off` on the daemon side still
            // gets visibility (warnings + telemetry) instead of a
            // silent no-op.
            "warn" | "off" | "advisory" | "proceed" => Some(Self::Warn),
            "fence" => Some(Self::Fence),
            "interrupt" | "block" => Some(Self::Interrupt),
            _ => None,
        }
    }

    /// Canonical string form (for telemetry / status surfaces).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Fence => "fence",
            Self::Interrupt => "interrupt",
        }
    }

    /// Return the stricter of two modes (project↔user merge).
    #[must_use]
    pub fn stricter(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }
}

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
/// `telemetry_allow_cross_session = false`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

        Self {
            mode,
            on_ambiguous_ownership,
            observe_only,
            telemetry_allow_cross_session,
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
    fn mode_parses_off_aliases_clamped_to_warn() {
        assert_eq!(Mode::parse("off"), Some(Mode::Warn));
        assert_eq!(Mode::parse("advisory"), Some(Mode::Warn));
        assert_eq!(Mode::parse("proceed"), Some(Mode::Warn));
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
}
