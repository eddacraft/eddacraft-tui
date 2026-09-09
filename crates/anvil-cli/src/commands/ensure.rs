//! Bare `anvil` daily ensure surface (ADR-114 / ONSW).
//!
//! Turns protection on for an already-activated worktree without reinstall
//! consent: daemon ensure, worktree registration when registerable, and MCP
//! ensure-only for already-owned entries. Never installs `NotPresent` MCP,
//! workflows, or hooks — those stay on `anvil start`.

use std::path::{Path, PathBuf};

use anvil_intercept::ensure::EnsureOutcome;
use anyhow::Context;
use serde::Serialize;

use crate::GlobalArgs;
use crate::activation;
use crate::activation::diagnostic::ConfigStatus;
use crate::activation::mcp_client::AnvilEntry;
use crate::activation::orchestrator::install::{InstallOutcome, ensure_existing_mcp_entries};
use crate::output::AlreadyReported;
use crate::registration::{
    self, SaveTimeDriverReadiness, WorktreeRegistration, WorktreeRegistrationReport,
};
use crate::util;

/// Human recovery when the repo has never been activated (config Absent).
pub(crate) const NOT_ACTIVATED_MESSAGE: &str = "anvil: not activated in this repository. Run `anvil start` to activate protection.\n\
     anvil: new here? Run `anvil welcome` for a guided tour.";

/// Human recovery when cwd is not a registerable git worktree.
pub(crate) const NOT_REGISTERABLE_MESSAGE: &str = "anvil: not a registerable git worktree. Run bare `anvil` from a \
     repository working tree (linked worktrees are fine).";

/// Human recovery when MCP was never installed (or was declined).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const MCP_NOT_INSTALLED_MESSAGE: &str = "anvil: MCP not installed for this machine — run `anvil start` to configure it \
     (or `anvil start --no-mcp` if you only want daemon-backed protection).";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EnsureReadinessState {
    Starting,
    Ready,
    Disabled,
    Degraded,
    Failed,
}

impl EnsureReadinessState {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Disabled => "disabled",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct EnsureComponentReadiness {
    state: EnsureReadinessState,
    detail: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct EnsureReadinessComponents {
    config: EnsureComponentReadiness,
    daemon: EnsureComponentReadiness,
    worktree: EnsureComponentReadiness,
    save_time: EnsureComponentReadiness,
    mcp: EnsureComponentReadiness,
}

#[derive(Debug, Serialize)]
pub(crate) struct EnsureReadiness {
    pub(crate) state: EnsureReadinessState,
    pub(crate) selected_coverage_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) failing_component: Option<&'static str>,
    pub(crate) components: EnsureReadinessComponents,
}

impl EnsureReadiness {
    pub(crate) fn failed(&self) -> bool {
        self.state == EnsureReadinessState::Failed
    }

    pub(crate) fn component_summary(&self) -> String {
        format!(
            "config={} daemon={} worktree={} save_time={} mcp={}",
            self.components.config.state.label(),
            self.components.daemon.state.label(),
            self.components.worktree.state.label(),
            self.components.save_time.state.label(),
            self.components.mcp.state.label(),
        )
    }

    pub(crate) fn failure_action(&self) -> Option<String> {
        match self.failing_component {
            Some("config") => Some("run `anvil start` to repair project configuration".to_owned()),
            Some("daemon") => Some("run `anvil start` to restore the save-time daemon".to_owned()),
            Some("worktree") => {
                Some("run `anvil start` to retry failed worktree registration".to_owned())
            }
            Some("save_time") => {
                Some("run `anvil start` to restore the failed save-time driver".to_owned())
            }
            Some("mcp") => Some("run `anvil start` to retry the failed MCP repair".to_owned()),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
struct EnsureJsonReport {
    surface: &'static str,
    protection: String,
    config: String,
    daemon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    daemon_version_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    daemon_version_after: Option<String>,
    worktree: String,
    mcp: String,
    readiness: EnsureReadiness,
    next: Option<String>,
}

pub fn run(global: &GlobalArgs) -> anyhow::Result<()> {
    let root = util::workspace_root().unwrap_or_else(|_| PathBuf::from("."));
    let root = root.as_path();

    let probe = activation::verify(root);
    if probe.config == ConfigStatus::Absent {
        return report_not_activated(global, root);
    }

    // Worktree validation gate (ONSW-002 / ADR-114): bare ensure is for
    // registerable working trees only. Refuse early so worktree validation
    // fails closed outside a repo (or inside `.git` / bare repos).
    let worktree_path = match registration::registerable_worktree(root) {
        Ok(path) => path,
        Err(reason) => return report_not_registerable(global, root, &reason.to_string()),
    };

    // Daemon ensure (idempotent). Bare is the on-switch: allow spawn even in
    // non-interactive contexts so scripts can turn protection on without a TTY.
    let capability = if daemon_opt_out() {
        anvil_intercept::ensure::StartCapability::NoSpawn(
            anvil_intercept::ensure::NoStartReason::OptOut,
        )
    } else {
        anvil_intercept::ensure::StartCapability::MaySpawn
    };
    if matches!(
        capability,
        anvil_intercept::ensure::StartCapability::MaySpawn
    ) && !global.json
    {
        eprintln!("anvil: ensuring the per-user save-time daemon is running…");
    }
    let daemon_outcome = crate::commands::intercept::ensure_save_time_daemon_report(capability);
    let daemon_line = format_daemon_outcome(&daemon_outcome);

    // Durable worktree registration (no project-init writes).
    let registration_report = registration::register_worktree_with_daemon(&worktree_path);
    let worktree_line = format_worktree_registration(&registration_report);

    // MCP ensure-only (skip entirely under ANVIL_NO_MCP).
    let mcp_disabled = mcp_opt_out();
    let (mcp_line, mut mcp_state) = if mcp_disabled {
        (
            "mcp: skipped (`ANVIL_NO_MCP`)".to_string(),
            EnsureReadinessState::Disabled,
        )
    } else {
        let fresh = AnvilEntry::preferred_stdio();
        let home = util::user_home_dir();
        let summary = ensure_existing_mcp_entries(root, home.as_deref(), &fresh);
        let rewritten = summary
            .report
            .per_client
            .values()
            .any(|outcome| matches!(outcome, InstallOutcome::Installed { .. }));
        let recycled = daemon_outcome.recycle.is_some();
        let poke = crate::commands::mcp_heal::poke_if_needed(
            crate::commands::mcp_heal::PokeReason::Changed {
                configs_rewritten: rewritten,
                daemon_recycled: recycled,
            },
        );
        if let Err(error) = &poke {
            crate::commands::mcp_heal::warn_poke_failure(error);
        }
        let line = format_mcp_line_with_poke(
            &summary.report,
            summary.managed,
            summary.absent_for_recovery,
            poke.ok().as_ref(),
        );
        let state = if summary.report.aggregated_failure().is_some() {
            EnsureReadinessState::Failed
        } else if rewritten {
            EnsureReadinessState::Starting
        } else if summary.managed > 0 {
            EnsureReadinessState::Degraded
        } else {
            EnsureReadinessState::Disabled
        };
        (line, state)
    };

    // Final protection probe after ensure.
    let diagnostic = activation::verify(root);
    if matches!(
        mcp_state,
        EnsureReadinessState::Starting | EnsureReadinessState::Degraded
    ) && diagnostic.mcp_pre_write_live()
    {
        mcp_state = EnsureReadinessState::Ready;
    }
    let protection = diagnostic.protection_state();
    let readiness = classify_readiness(
        diagnostic.config,
        &daemon_outcome,
        &registration_report,
        save_time_driver_opt_out(),
        mcp_state,
        &mcp_line,
    );
    let next = readiness
        .failure_action()
        .or_else(|| next_action_line(protection, &mcp_line));
    emit_ensure_report(
        global,
        &diagnostic,
        &daemon_outcome,
        daemon_line,
        worktree_line,
        mcp_line,
        readiness,
        next,
    )
}

#[allow(clippy::too_many_arguments)]
fn emit_ensure_report(
    global: &GlobalArgs,
    diagnostic: &activation::diagnostic::ActivationDiagnostic,
    daemon_outcome: &crate::commands::daemon_recycle::SaveTimeDaemonOutcome,
    daemon_line: String,
    worktree_line: String,
    mcp_line: String,
    readiness: EnsureReadiness,
    next: Option<String>,
) -> anyhow::Result<()> {
    let protection = diagnostic.protection_state();
    let readiness_failed = readiness.failed();
    if global.json {
        let (daemon_version_before, daemon_version_after) = daemon_outcome
            .recycle
            .as_ref()
            .map_or((None, None), |recycle| {
                (Some(recycle.before.clone()), Some(recycle.after.clone()))
            });
        let doc = EnsureJsonReport {
            surface: "ensure",
            protection: protection.label().to_string(),
            config: diagnostic.config.label().to_string(),
            daemon: daemon_line,
            daemon_version_before,
            daemon_version_after,
            worktree: worktree_line,
            mcp: mcp_line,
            readiness,
            next,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).context("serialise ensure report")?
        );
        if readiness_failed {
            return Err(AlreadyReported.into());
        }
        return Ok(());
    }

    println!("anvil ensure");
    println!("  protection: {}", protection.label());
    println!("  {}", protection.headline());
    println!("  readiness: {}", readiness.state.label());
    println!("  components: {}", readiness.component_summary());
    println!("  {daemon_line}");
    println!("  {worktree_line}");
    println!("  {mcp_line}");
    if let Some(next) = next {
        println!("  next: {next}");
    }

    if readiness.failed() {
        return Err(AlreadyReported.into());
    }
    Ok(())
}

fn report_not_activated(global: &GlobalArgs, root: &Path) -> anyhow::Result<()> {
    if global.json {
        let doc = EnsureJsonReport {
            surface: "ensure",
            protection: "needs_action".to_string(),
            config: "absent".to_string(),
            daemon: "skipped".to_string(),
            daemon_version_before: None,
            daemon_version_after: None,
            worktree: root.display().to_string(),
            mcp: "skipped".to_string(),
            readiness: preflight_readiness("config", "project is not activated"),
            next: Some("run `anvil start` to activate".to_string()),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).context("serialise ensure report")?
        );
    } else {
        eprintln!("{NOT_ACTIVATED_MESSAGE}");
    }
    Err(AlreadyReported.into())
}

fn report_not_registerable(global: &GlobalArgs, root: &Path, reason: &str) -> anyhow::Result<()> {
    if global.json {
        let doc = EnsureJsonReport {
            surface: "ensure",
            protection: "needs_action".to_string(),
            config: "valid".to_string(),
            daemon: "skipped".to_string(),
            daemon_version_before: None,
            daemon_version_after: None,
            worktree: format!("not registerable ({reason}) at {}", root.display()),
            mcp: "skipped".to_string(),
            readiness: preflight_readiness("worktree", reason),
            next: Some("run bare `anvil` from a git working tree".to_string()),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).context("serialise ensure report")?
        );
    } else {
        eprintln!("{NOT_REGISTERABLE_MESSAGE}");
        eprintln!("anvil: {reason}");
    }
    Err(AlreadyReported.into())
}

fn daemon_opt_out() -> bool {
    std::env::var_os("ANVIL_NO_DAEMON").is_some_and(|value| !value.is_empty())
}

fn mcp_opt_out() -> bool {
    std::env::var_os("ANVIL_NO_MCP").is_some_and(|value| !value.is_empty())
}

pub(crate) fn save_time_driver_opt_out() -> bool {
    std::env::var_os("ANVIL_NO_SAVE_TIME_DRIVER").is_some_and(|value| !value.is_empty())
}

fn component(state: EnsureReadinessState, detail: impl Into<String>) -> EnsureComponentReadiness {
    EnsureComponentReadiness {
        state,
        detail: detail.into(),
    }
}

fn preflight_readiness(component_name: &'static str, detail: &str) -> EnsureReadiness {
    let mut components = EnsureReadinessComponents {
        config: component(EnsureReadinessState::Disabled, "not evaluated"),
        daemon: component(EnsureReadinessState::Disabled, "not evaluated"),
        worktree: component(EnsureReadinessState::Disabled, "not evaluated"),
        save_time: component(EnsureReadinessState::Disabled, "not evaluated"),
        mcp: component(EnsureReadinessState::Disabled, "not evaluated"),
    };
    match component_name {
        "config" => components.config = component(EnsureReadinessState::Failed, detail),
        "worktree" => components.worktree = component(EnsureReadinessState::Failed, detail),
        _ => {}
    }
    EnsureReadiness {
        state: EnsureReadinessState::Failed,
        selected_coverage_ready: false,
        failing_component: Some(component_name),
        components,
    }
}

pub(crate) fn classify_readiness(
    config: ConfigStatus,
    daemon: &crate::commands::daemon_recycle::SaveTimeDaemonOutcome,
    registration: &WorktreeRegistrationReport,
    driver_disabled: bool,
    mcp_state: EnsureReadinessState,
    mcp_detail: &str,
) -> EnsureReadiness {
    let config_state = match config {
        ConfigStatus::Valid => EnsureReadinessState::Ready,
        ConfigStatus::Absent | ConfigStatus::Invalid => EnsureReadinessState::Failed,
    };
    let daemon_state = if daemon.failed() {
        EnsureReadinessState::Failed
    } else {
        match &daemon.ensure {
            EnsureOutcome::Reused | EnsureOutcome::Started => EnsureReadinessState::Ready,
            EnsureOutcome::NoStart { .. } => EnsureReadinessState::Disabled,
            EnsureOutcome::Failed { .. } => unreachable!("handled by failed()"),
        }
    };
    let worktree_state = match &registration.registration {
        WorktreeRegistration::Registered | WorktreeRegistration::Refreshed => {
            EnsureReadinessState::Ready
        }
        WorktreeRegistration::DaemonUnavailable
            if daemon_state == EnsureReadinessState::Disabled =>
        {
            EnsureReadinessState::Disabled
        }
        WorktreeRegistration::DaemonUnavailable
        | WorktreeRegistration::Fenced(_)
        | WorktreeRegistration::CapExceeded(_)
        | WorktreeRegistration::Rejected(_) => EnsureReadinessState::Failed,
    };
    let save_time_state = classify_save_time_readiness(
        daemon_state,
        worktree_state,
        registration.driver.as_ref(),
        driver_disabled,
    );

    let states = [
        ("config", config_state),
        ("daemon", daemon_state),
        ("worktree", worktree_state),
        ("save_time", save_time_state),
        ("mcp", mcp_state),
    ];
    let selected_states = [save_time_state, mcp_state];
    let selected_coverage_ready = selected_states
        .iter()
        .any(|state| *state != EnsureReadinessState::Disabled)
        && selected_states.iter().all(|state| {
            matches!(
                state,
                EnsureReadinessState::Ready | EnsureReadinessState::Disabled
            )
        })
        && !states
            .iter()
            .any(|(_, state)| *state == EnsureReadinessState::Failed);
    let failing_component = states
        .iter()
        .find_map(|(name, state)| (*state == EnsureReadinessState::Failed).then_some(*name));
    let state = if failing_component.is_some() {
        EnsureReadinessState::Failed
    } else if selected_coverage_ready {
        EnsureReadinessState::Ready
    } else if states
        .iter()
        .any(|(_, state)| *state == EnsureReadinessState::Starting)
    {
        EnsureReadinessState::Starting
    } else if states
        .iter()
        .any(|(_, state)| *state == EnsureReadinessState::Degraded)
    {
        EnsureReadinessState::Degraded
    } else {
        EnsureReadinessState::Disabled
    };

    let save_time_detail = match &registration.driver {
        Some(SaveTimeDriverReadiness::Attached { evidence }) => evidence.map_or_else(
            || "attached without readiness evidence".to_owned(),
            |evidence| save_time_evidence_label(evidence).to_owned(),
        ),
        Some(SaveTimeDriverReadiness::Failed) => "driver failed".to_owned(),
        Some(SaveTimeDriverReadiness::Absent) => "driver absent".to_owned(),
        Some(SaveTimeDriverReadiness::Unknown(error)) => error.clone(),
        None => "no driver evidence".to_owned(),
    };
    EnsureReadiness {
        state,
        selected_coverage_ready,
        failing_component,
        components: EnsureReadinessComponents {
            config: component(config_state, config.label()),
            daemon: component(daemon_state, format_daemon_outcome(daemon)),
            worktree: component(worktree_state, format_worktree_registration(registration)),
            save_time: component(save_time_state, save_time_detail),
            mcp: component(mcp_state, mcp_detail),
        },
    }
}

fn classify_save_time_readiness(
    daemon: EnsureReadinessState,
    worktree: EnsureReadinessState,
    driver: Option<&SaveTimeDriverReadiness>,
    disabled: bool,
) -> EnsureReadinessState {
    use anvil_intercept_proto::status::SaveTimeDriverEvidenceV1;

    if disabled || daemon == EnsureReadinessState::Disabled {
        return EnsureReadinessState::Disabled;
    }
    if worktree != EnsureReadinessState::Ready {
        return EnsureReadinessState::Failed;
    }
    match driver {
        Some(SaveTimeDriverReadiness::Attached {
            evidence:
                Some(
                    SaveTimeDriverEvidenceV1::WatchesInstalled
                    | SaveTimeDriverEvidenceV1::FreshActivity,
                ),
        }) => EnsureReadinessState::Ready,
        Some(SaveTimeDriverReadiness::Attached {
            evidence: Some(SaveTimeDriverEvidenceV1::Spawned),
        }) => EnsureReadinessState::Starting,
        Some(SaveTimeDriverReadiness::Attached {
            evidence: Some(SaveTimeDriverEvidenceV1::Unknown) | None,
        })
        | None => EnsureReadinessState::Degraded,
        Some(
            SaveTimeDriverReadiness::Failed
            | SaveTimeDriverReadiness::Absent
            | SaveTimeDriverReadiness::Unknown(_),
        ) => EnsureReadinessState::Failed,
    }
}

fn save_time_evidence_label(
    evidence: anvil_intercept_proto::status::SaveTimeDriverEvidenceV1,
) -> &'static str {
    use anvil_intercept_proto::status::SaveTimeDriverEvidenceV1;
    match evidence {
        SaveTimeDriverEvidenceV1::Spawned => "spawned",
        SaveTimeDriverEvidenceV1::WatchesInstalled => "watches-installed",
        SaveTimeDriverEvidenceV1::FreshActivity => "fresh-activity",
        SaveTimeDriverEvidenceV1::Unknown => "unknown",
    }
}

fn format_daemon_outcome(
    outcome: &crate::commands::daemon_recycle::SaveTimeDaemonOutcome,
) -> String {
    crate::commands::daemon_recycle::format_save_time_daemon_outcome(outcome)
}

/// JREL-003: the `worktree:` line. The membership wording is unchanged; when
/// membership succeeded but the daemon reported the save-time driver failed,
/// the line says so rather than letting a refreshed membership pass as
/// coverage.
fn format_worktree_registration(report: &registration::WorktreeRegistrationReport) -> String {
    let driver_failed = report.driver_failed();
    let line = format_worktree_membership(&report.registration);
    if driver_failed {
        format!("{line}; save-time driver failed — inspect `anvil intercept status`")
    } else {
        line
    }
}

fn format_worktree_membership(outcome: &WorktreeRegistration) -> String {
    match outcome {
        WorktreeRegistration::Registered => {
            "worktree: registered with the save-time daemon".to_string()
        }
        WorktreeRegistration::Refreshed => "worktree: registration refreshed".to_string(),
        WorktreeRegistration::DaemonUnavailable => {
            "worktree: daemon unavailable for registration".to_string()
        }
        WorktreeRegistration::Fenced(message) => format!("worktree: fenced — {message}"),
        WorktreeRegistration::CapExceeded(message) => {
            format!("worktree: registration cap exceeded — {message}")
        }
        WorktreeRegistration::Rejected(message) => {
            format!("worktree: registration rejected — {message}")
        }
    }
}

fn format_mcp_line_with_poke(
    report: &activation::orchestrator::InstallReport,
    managed: usize,
    absent_for_recovery: usize,
    poke: Option<&crate::commands::mcp_heal::PokeOutcome>,
) -> String {
    if poke.is_some_and(|outcome| outcome.skipped_pin) {
        return format!(
            "mcp: auto-heal {}",
            crate::commands::mcp_heal::heal_policy().summary()
        );
    }
    format_mcp_line(report, managed, absent_for_recovery)
}

fn format_mcp_line(
    report: &activation::orchestrator::InstallReport,
    managed: usize,
    absent_for_recovery: usize,
) -> String {
    let mut repaired = 0usize;
    let mut failed = 0usize;
    for outcome in report.per_client.values() {
        match outcome {
            InstallOutcome::Installed { .. } => repaired += 1,
            InstallOutcome::Failed { .. } => failed += 1,
            InstallOutcome::Skipped { .. } => {}
        }
    }
    if failed > 0 {
        return format!("mcp: ensure failed for {failed} client(s); see logs");
    }
    if repaired > 0 {
        return format!("mcp: updated {repaired} anvil-owned entries");
    }
    if managed > 0 {
        return "mcp: anvil entry present".to_string();
    }
    if absent_for_recovery > 0 {
        return "mcp: not installed — run `anvil start` to configure".to_string();
    }
    "mcp: no anvil-owned entry to ensure".to_string()
}

fn next_action_line(state: activation::state::ProtectionState, mcp_line: &str) -> Option<String> {
    use activation::state::ProtectionState;
    if mcp_line.contains("not installed") {
        return Some("run `anvil start` to install MCP (optional)".to_string());
    }
    match state {
        ProtectionState::Protecting | ProtectionState::Watching => None,
        ProtectionState::ReadyRestartRequired => {
            Some("restart your editor so MCP attaches".to_string())
        }
        ProtectionState::NeedsAction => Some("run `anvil start` to finish activation".to_string()),
        ProtectionState::Unsupported => {
            Some("see `anvil status --verify` for coverage".to_string())
        }
        ProtectionState::Error => Some("run `anvil start --verify` / `anvil doctor`".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activation::orchestrator::install::{InstallOutcome, InstallReport};
    use crate::activation::state::ProtectionState;
    use std::collections::BTreeMap;

    fn daemon(ensure: EnsureOutcome) -> crate::commands::daemon_recycle::SaveTimeDaemonOutcome {
        crate::commands::daemon_recycle::SaveTimeDaemonOutcome::from_ensure(ensure)
    }

    fn registered(driver: SaveTimeDriverReadiness) -> WorktreeRegistrationReport {
        WorktreeRegistrationReport {
            registration: WorktreeRegistration::Registered,
            driver: Some(driver),
        }
    }

    #[test]
    fn not_activated_message_names_start_and_welcome() {
        assert!(NOT_ACTIVATED_MESSAGE.contains("anvil start"));
        assert!(NOT_ACTIVATED_MESSAGE.contains("anvil welcome"));
    }

    #[test]
    fn not_registerable_message_mentions_worktree() {
        assert!(NOT_REGISTERABLE_MESSAGE.contains("worktree"));
    }

    #[test]
    fn mcp_not_installed_message_names_start() {
        assert!(MCP_NOT_INSTALLED_MESSAGE.contains("anvil start"));
    }

    #[test]
    fn format_mcp_line_absent_points_at_start() {
        let report = InstallReport::default();
        let line = format_mcp_line(&report, 0, 2);
        assert!(line.contains("not installed"), "{line}");
        assert!(line.contains("anvil start"), "{line}");
    }

    #[test]
    fn format_mcp_line_managed_without_writes_is_present() {
        let line = format_mcp_line(&InstallReport::default(), 1, 0);
        assert!(line.contains("present"), "{line}");
    }

    #[test]
    fn format_mcp_line_repaired_counts_writes() {
        let mut per_client = BTreeMap::new();
        per_client.insert(
            crate::activation::diagnostic::McpClientId::Cursor,
            InstallOutcome::Installed {
                path: PathBuf::from("/tmp/mcp.json"),
                drift: crate::activation::mcp_client::DriftClass::SafeDrift {
                    reason: "path".into(),
                },
            },
        );
        let report = InstallReport {
            per_client,
            hooks_active: false,
        };
        let line = format_mcp_line(&report, 1, 0);
        assert!(line.contains("updated"), "{line}");
    }

    #[test]
    fn next_action_none_when_protecting() {
        assert!(
            next_action_line(ProtectionState::Protecting, "mcp: anvil entry present").is_none()
        );
    }

    #[test]
    fn next_action_when_mcp_missing() {
        let next = next_action_line(
            ProtectionState::Watching,
            "mcp: not installed — run `anvil start` to configure",
        );
        assert!(next.unwrap().contains("anvil start"));
    }

    #[test]
    fn format_mcp_line_empty_managed() {
        let line = format_mcp_line(&InstallReport::default(), 0, 0);
        assert!(line.contains("no anvil-owned"), "{line}");
    }

    #[test]
    fn skip_reason_up_to_date_does_not_claim_not_installed() {
        let line = format_mcp_line(&InstallReport::default(), 1, 0);
        assert!(!line.contains("not installed"), "{line}");
        assert!(line.contains("present"), "{line}");
    }

    #[test]
    fn no_mcp_is_ready_when_save_time_watches_are_installed() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Reused),
            &registered(SaveTimeDriverReadiness::Attached {
                evidence: Some(
                    anvil_intercept_proto::status::SaveTimeDriverEvidenceV1::WatchesInstalled,
                ),
            }),
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Ready);
        assert!(readiness.selected_coverage_ready);
        assert_eq!(readiness.failing_component, None);
    }

    #[test]
    fn spawned_driver_is_starting_not_ready() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Reused),
            &registered(SaveTimeDriverReadiness::Attached {
                evidence: Some(anvil_intercept_proto::status::SaveTimeDriverEvidenceV1::Spawned),
            }),
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Starting);
        assert!(!readiness.selected_coverage_ready);
        assert_eq!(readiness.failing_component, None);
    }

    #[test]
    fn failed_driver_is_a_typed_save_time_failure() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Reused),
            &registered(SaveTimeDriverReadiness::Failed),
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Failed);
        assert_eq!(readiness.failing_component, Some("save_time"));
        assert_eq!(
            readiness.failure_action().as_deref(),
            Some("run `anvil start` to restore the failed save-time driver")
        );
    }

    #[test]
    fn absent_driver_after_the_readiness_budget_is_a_failure() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Reused),
            &registered(SaveTimeDriverReadiness::Absent),
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Failed);
        assert!(!readiness.selected_coverage_ready);
        assert_eq!(readiness.failing_component, Some("save_time"));
    }

    #[test]
    fn attachment_without_readiness_evidence_is_degraded() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Reused),
            &registered(SaveTimeDriverReadiness::Attached { evidence: None }),
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Degraded);
        assert!(!readiness.selected_coverage_ready);
        assert_eq!(readiness.failing_component, None);
    }

    #[test]
    fn registration_refusal_is_a_typed_worktree_failure() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Reused),
            &WorktreeRegistrationReport {
                registration: WorktreeRegistration::CapExceeded("cap reached".to_owned()),
                driver: None,
            },
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Failed);
        assert_eq!(readiness.failing_component, Some("worktree"));
    }

    #[test]
    fn failed_mcp_repair_wins_even_when_save_time_is_ready() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Reused),
            &registered(SaveTimeDriverReadiness::Attached {
                evidence: Some(
                    anvil_intercept_proto::status::SaveTimeDriverEvidenceV1::FreshActivity,
                ),
            }),
            false,
            EnsureReadinessState::Failed,
            "MCP repair failed",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Failed);
        assert!(!readiness.selected_coverage_ready);
        assert_eq!(readiness.failing_component, Some("mcp"));
    }

    #[test]
    fn unresponsive_daemon_is_a_typed_daemon_failure() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::Failed {
                recovery: "inspect daemon log".to_owned(),
            }),
            &WorktreeRegistrationReport {
                registration: WorktreeRegistration::DaemonUnavailable,
                driver: None,
            },
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Failed);
        assert_eq!(readiness.failing_component, Some("daemon"));
    }

    #[test]
    fn mcp_only_is_ready_when_a_live_session_is_observed() {
        let readiness = classify_readiness(
            ConfigStatus::Valid,
            &daemon(EnsureOutcome::NoStart {
                reason: anvil_intercept::ensure::NoStartReason::OptOut,
            }),
            &WorktreeRegistrationReport {
                registration: WorktreeRegistration::DaemonUnavailable,
                driver: None,
            },
            true,
            EnsureReadinessState::Ready,
            "MCP live validation observed",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Ready);
        assert!(readiness.selected_coverage_ready);
        assert_eq!(readiness.failing_component, None);
    }

    #[test]
    fn invalid_config_is_a_typed_configuration_failure() {
        let readiness = classify_readiness(
            ConfigStatus::Invalid,
            &daemon(EnsureOutcome::Reused),
            &registered(SaveTimeDriverReadiness::Attached {
                evidence: Some(
                    anvil_intercept_proto::status::SaveTimeDriverEvidenceV1::WatchesInstalled,
                ),
            }),
            false,
            EnsureReadinessState::Disabled,
            "MCP deliberately omitted",
        );
        assert_eq!(readiness.state, EnsureReadinessState::Failed);
        assert!(!readiness.selected_coverage_ready);
        assert_eq!(readiness.failing_component, Some("config"));
    }
}
