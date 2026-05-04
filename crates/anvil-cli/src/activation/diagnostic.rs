//! Layered activation diagnostic.
//!
//! `ActivationDiagnostic` is the structured probe result that backs
//! every render of `ProtectionState`. The diagnostic separates layers
//! so a surface can render "config valid, MCP startable, restart still
//! required" without collapsing distinct failures into a single opaque
//! "not protected" message.
//!
//! `verify` is intentionally narrow in this PR: it probes layers that
//! do not require LAUNCH-009 (MCP install/verify), LAUNCH-010
//! (baseline), or LAUNCH-011 (watch fallback runtime). Future PRs
//! plug in the missing layers; this PR establishes the contract.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::state::ProtectionState;

/// Stable v1 identifiers for editors / agents that ship MCP probes.
///
/// Held as a typed enum (rather than a free `String`) so callers cannot
/// silently introduce out-of-scope clients. The v1 release is locked
/// to Cursor and Claude Code per the council ratification — see
/// `RELEASE-PLAN.md` Tier A1 "Out-of-scope".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpClientId {
    Cursor,
    ClaudeCode,
}

impl McpClientId {
    pub fn label(self) -> &'static str {
        match self {
            McpClientId::Cursor => "cursor",
            McpClientId::ClaudeCode => "claude-code",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            McpClientId::Cursor => "Cursor",
            McpClientId::ClaudeCode => "Claude Code",
        }
    }
}

/// Tier of MCP attachment for a given client.
///
/// Variants form a strict ladder from "no config" up to "live evidence
/// observed". A surface that wants to claim `Protecting` needs at
/// least one client at [`McpTier::LiveValidation`]. Any tier below
/// that is more honest as `ReadyRestartRequired` or weaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTier {
    /// Client not detected on the host at all.
    NotDetected,
    /// Client detected, but its config does not include the Anvil
    /// MCP server entry.
    ConfigAbsent,
    /// Anvil MCP server entry present in client config.
    ConfigPresent,
    /// Anvil MCP server starts cleanly when invoked from the same
    /// command shape the client will use.
    ServerStartable,
    /// Server is configured and startable, but the client must be
    /// restarted before it picks up the entry.
    RestartRequired,
    /// Live MCP `anvil_validate_write` invocation has been observed
    /// from this client inside this repo.
    LiveValidation,
}

impl McpTier {
    pub fn label(self) -> &'static str {
        match self {
            McpTier::NotDetected => "not_detected",
            McpTier::ConfigAbsent => "config_absent",
            McpTier::ConfigPresent => "config_present",
            McpTier::ServerStartable => "server_startable",
            McpTier::RestartRequired => "restart_required",
            McpTier::LiveValidation => "live_validation",
        }
    }
}

/// Tier of save-time watch fallback. Watch is intentionally weaker
/// than MCP pre-write validation, so its tiers do not include an
/// equivalent of [`McpTier::LiveValidation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchTier {
    /// Watcher not requested or no fallback offered.
    NotRequested,
    /// Watcher offered but not running yet.
    Offered,
    /// Watcher process is running and watching the repo.
    Running,
}

impl WatchTier {
    pub fn label(self) -> &'static str {
        match self {
            WatchTier::NotRequested => "not_requested",
            WatchTier::Offered => "offered",
            WatchTier::Running => "running",
        }
    }
}

/// Status of the on-disk Anvil config (`.anvilrc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigStatus {
    /// No config file detected at the repo root.
    Absent,
    /// Config file present and parses cleanly.
    Valid,
    /// Config file present but cannot be parsed.
    Invalid,
}

impl ConfigStatus {
    pub fn label(self) -> &'static str {
        match self {
            ConfigStatus::Absent => "absent",
            ConfigStatus::Valid => "valid",
            ConfigStatus::Invalid => "invalid",
        }
    }
}

/// Layered probe result for the wow-start activation flow.
///
/// Surfaces should NOT compute `ProtectionState` from a subset of
/// these fields ad-hoc — call [`ActivationDiagnostic::protection_state`]
/// so the mapping stays in one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationDiagnostic {
    /// Status of the on-disk config (`.anvilrc`).
    pub config: ConfigStatus,
    /// Tier reached for each detected MCP client. Clients below
    /// [`McpTier::ConfigPresent`] are still emitted so the surface
    /// can show "Cursor: not detected" without dropping the row.
    pub mcp: BTreeMap<McpClientId, McpTier>,
    /// Tier of the save-time watch fallback.
    pub watch: WatchTier,
    /// True when the repo has a baselined finding set (LAUNCH-010).
    /// Surfaces use this to decide whether to label the next signal
    /// as "first new finding" vs "first finding".
    pub baseline_present: bool,
    /// Last activation error, if any. Set by retry / verification
    /// flows when a previous activation attempt aborted. Cleared on
    /// the first successful re-run.
    pub last_error: Option<String>,
    /// True when the repo's languages are all `unsupported` per
    /// the future repo language profile (LAUNCH-015). Defaults to
    /// `false` until that probe lands; PR 5 wires this through.
    pub all_languages_unsupported: bool,
}

impl ActivationDiagnostic {
    /// Map the layered diagnostic onto a single
    /// [`ProtectionState`] word. The mapping is the canonical truth
    /// table for activation copy and is exercised by the unit tests
    /// at the bottom of this file.
    pub fn protection_state(&self) -> ProtectionState {
        // Hard error wins over everything — never claim coverage if
        // we already know activation failed or config is broken.
        if self.last_error.is_some() {
            return ProtectionState::Error;
        }
        if matches!(self.config, ConfigStatus::Invalid) {
            return ProtectionState::Error;
        }

        // Live MCP evidence is the only path to `Protecting`.
        let highest_mcp = self.highest_mcp_tier();
        if matches!(highest_mcp, Some(McpTier::LiveValidation)) {
            return ProtectionState::Protecting;
        }

        // `RestartRequired` is the literal "one restart from live"
        // tier. `ServerStartable` is earlier in the chain (the server
        // spawns, but we have no evidence a client restart will pick
        // it up) — collapsing it to `ReadyRestartRequired` would
        // over-claim, so it falls through to `NeedsAction` /
        // `Watching` like the weaker tiers.
        let restart_pending = matches!(highest_mcp, Some(McpTier::RestartRequired));

        if matches!(self.watch, WatchTier::Running) {
            // Watch is honest fallback coverage. If MCP is literally
            // one step from live, surface that stronger label so the
            // user knows a restart will graduate them — but never
            // promote `ServerStartable` to that label.
            if restart_pending {
                return ProtectionState::ReadyRestartRequired;
            }
            return ProtectionState::Watching;
        }

        if restart_pending {
            return ProtectionState::ReadyRestartRequired;
        }

        // No literal `Protecting` / `ReadyRestartRequired` /
        // `Watching` claim is available. If the repo's languages are
        // all unsupported AND MCP is below the restart-pending tier,
        // tell the user honestly — `NeedsAction` would suggest "run
        // `anvil start`" but no further setup will help here.
        if self.all_languages_unsupported {
            return ProtectionState::Unsupported;
        }

        // Anything else — config absent, MCP not detected, watch not
        // requested — means the user has actionable next steps.
        ProtectionState::NeedsAction
    }

    fn highest_mcp_tier(&self) -> Option<McpTier> {
        self.mcp.values().copied().max()
    }
}

/// Probe activation state at `root`. PR 2 lands the contract; deeper
/// probes (real MCP detection, baseline, watch identity) plug into
/// this function in PR 3, PR 4, and PR 5.
pub fn verify(root: &Path) -> ActivationDiagnostic {
    let config = probe_config_status(root);
    let baseline_present = probe_baseline_present(root);

    // Until PR 3 (LAUNCH-009) lands, we do not probe the user's
    // editor configs. The diagnostic exposes the empty map honestly
    // so surfaces render "MCP: not detected" rather than guessing.
    let mcp: BTreeMap<McpClientId, McpTier> = BTreeMap::new();

    // Until PR 3 (LAUNCH-011) lands, we do not introspect the
    // running watcher process. Surfaces render "watch: not requested".
    let watch = WatchTier::NotRequested;

    ActivationDiagnostic {
        config,
        mcp,
        watch,
        baseline_present,
        last_error: None,
        all_languages_unsupported: false,
    }
}

fn probe_config_status(root: &Path) -> ConfigStatus {
    let rc = root.join(".anvilrc");
    let Ok(contents) = std::fs::read_to_string(&rc) else {
        return ConfigStatus::Absent;
    };

    // Empty / whitespace-only / BOM-only files are accepted by
    // serde_yaml as `Null` and would otherwise pass as `Valid`. That
    // would silently mask an editor that truncated the user's config.
    // Treat them as `Invalid` so the surface flags the problem.
    let trimmed = contents.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return ConfigStatus::Invalid;
    }

    // The init command writes one of JSON, YAML, or TOML — accept
    // any parser succeeding as proof of validity. This matches the
    // `gather_profile` heuristic in `commands::status`.
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return ConfigStatus::Valid;
    }
    if serde_yaml::from_str::<serde_yaml::Value>(trimmed).is_ok() {
        return ConfigStatus::Valid;
    }
    if toml::from_str::<toml::Value>(trimmed).is_ok() {
        return ConfigStatus::Valid;
    }

    ConfigStatus::Invalid
}

fn probe_baseline_present(root: &Path) -> bool {
    // The baseline file shape is owned by LAUNCH-010 (PR 4). PR 2
    // probes only the directory existence so this layer is safe to
    // ship before the writer lands; PR 4 narrows the probe to the
    // exact file shape it picks.
    root.join(".anvil").join("baseline.json").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn empty_diagnostic() -> ActivationDiagnostic {
        ActivationDiagnostic {
            config: ConfigStatus::Absent,
            mcp: BTreeMap::new(),
            watch: WatchTier::NotRequested,
            baseline_present: false,
            last_error: None,
            all_languages_unsupported: false,
        }
    }

    #[test]
    fn fresh_repo_with_no_config_needs_action() {
        let d = empty_diagnostic();
        assert_eq!(d.protection_state(), ProtectionState::NeedsAction);
    }

    #[test]
    fn invalid_config_renders_as_error() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Invalid;
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn last_error_wins_over_everything() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::LiveValidation);
        d.last_error = Some("startup probe timed out".into());
        // Even with live MCP evidence the Error layer must win,
        // because the last activation attempt aborted.
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn live_mcp_evidence_yields_protecting() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::LiveValidation);
        assert_eq!(d.protection_state(), ProtectionState::Protecting);
    }

    #[test]
    fn restart_required_tier_yields_ready_restart_required() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::ClaudeCode, McpTier::RestartRequired);
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn watch_running_alone_yields_watching() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.watch = WatchTier::Running;
        assert_eq!(d.protection_state(), ProtectionState::Watching);
    }

    #[test]
    fn watch_running_plus_restart_pending_prefers_restart_label() {
        // Adversarial guard: if MCP is one step from live AND watch
        // is running, the surface should nudge toward the literally
        // stronger state (`ReadyRestartRequired`) rather than letting
        // the user assume `Watching` is the best they can get.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::RestartRequired);
        d.watch = WatchTier::Running;
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn watch_running_plus_server_startable_does_not_overclaim() {
        // Council remediation: ServerStartable is NOT one restart from
        // live — the client may not pick up the entry without further
        // setup. With watch running, the truer state is the watch
        // fallback, never `ReadyRestartRequired`.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::ServerStartable);
        d.watch = WatchTier::Running;
        assert_eq!(d.protection_state(), ProtectionState::Watching);
    }

    #[test]
    fn server_startable_without_watch_falls_to_needs_action() {
        // Council remediation: ServerStartable alone is not enough
        // for a `ReadyRestartRequired` claim — promote only on
        // `RestartRequired`. The user has actionable next steps.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::ClaudeCode, McpTier::ServerStartable);
        assert_eq!(d.protection_state(), ProtectionState::NeedsAction);
    }

    #[test]
    fn no_mcp_and_all_languages_unsupported_yields_unsupported() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::Unsupported);
    }

    #[test]
    fn unsupported_languages_with_partial_mcp_yields_unsupported() {
        // Council remediation: when languages are out-of-scope and
        // MCP has not yet reached `RestartRequired`, telling the user
        // to "run anvil start" is misleading because no further setup
        // will produce coverage. Surface `Unsupported` instead.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::ConfigPresent);
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::Unsupported);
    }

    #[test]
    fn unsupported_languages_yield_to_restart_required_when_one_step_away() {
        // The user is literally one restart from secrets coverage —
        // tell them, do not collapse to `Unsupported`.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::RestartRequired);
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn unsupported_languages_yield_to_protecting_when_live() {
        // Even on unsupported languages, secrets checks still run
        // through MCP. Live evidence wins.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::LiveValidation);
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::Protecting);
    }

    #[test]
    fn supported_language_without_mcp_is_needs_action_not_unsupported() {
        // Adversarial guard: do not mislabel a supported repo as
        // unsupported just because activation hasn't completed.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.all_languages_unsupported = false;
        assert_eq!(d.protection_state(), ProtectionState::NeedsAction);
    }

    #[test]
    fn highest_mcp_tier_picks_strongest() {
        let mut d = empty_diagnostic();
        d.mcp.insert(McpClientId::Cursor, McpTier::ConfigPresent);
        d.mcp.insert(McpClientId::ClaudeCode, McpTier::ServerStartable);
        assert_eq!(d.highest_mcp_tier(), Some(McpTier::ServerStartable));
    }

    #[test]
    fn verify_handles_missing_anvilrc() {
        let dir = TempDir::new().unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Absent);
        assert!(d.mcp.is_empty());
        assert_eq!(d.watch, WatchTier::NotRequested);
        assert!(!d.baseline_present);
        assert!(d.last_error.is_none());
        assert_eq!(d.protection_state(), ProtectionState::NeedsAction);
    }

    #[test]
    fn verify_recognises_valid_json_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            r#"{"profile":"default","checks":[]}"#,
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_recognises_valid_yaml_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile: default\nchecks: []\n",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_recognises_valid_toml_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile = \"default\"\nchecks = []\n",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_flags_unparseable_config_as_invalid() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "{this is not valid in any format::").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn verify_flags_empty_config_as_invalid() {
        // Council remediation: empty file passes serde_yaml as
        // `Null` and would otherwise be reported as Valid — that
        // would mask an editor that truncated the user's config.
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn verify_flags_whitespace_only_config_as_invalid() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "   \n\t\n").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_handles_bom_prefixed_config() {
        // A BOM-prefixed file with otherwise-valid YAML should still
        // be Valid — only BOM-only / whitespace-only are Invalid.
        let dir = TempDir::new().unwrap();
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(b"profile: default\nchecks: []\n");
        fs::write(dir.path().join(".anvilrc"), bytes).unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_flags_bom_only_config_as_invalid() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), b"\xEF\xBB\xBF").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_detects_baseline_marker() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".anvil")).unwrap();
        fs::write(dir.path().join(".anvil").join("baseline.json"), "{}").unwrap();
        let d = verify(dir.path());
        assert!(d.baseline_present);
    }

    #[test]
    fn idempotent_reverify_is_pure() {
        // LAUNCH-012 acceptance: re-running verification performs no
        // writes and leaves the config unchanged. The probe never
        // mutates state, but pin it with a test that diffs the
        // directory mtime before/after a double verify.
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile: default\nchecks: []\n",
        )
        .unwrap();
        let before = fs::metadata(dir.path().join(".anvilrc"))
            .unwrap()
            .modified()
            .unwrap();
        let _ = verify(dir.path());
        let _ = verify(dir.path());
        let after = fs::metadata(dir.path().join(".anvilrc"))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(before, after, "verify must not mutate the config file");
    }
}
