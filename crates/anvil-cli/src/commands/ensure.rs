//! Bare `anvil` daily ensure surface (ADR-114 / ONSW).
//!
//! Turns protection on for an already-activated worktree without reinstall
//! consent: daemon ensure, worktree registration when registerable, and MCP
//! ensure-only for already-owned entries. Never installs NotPresent MCP,
//! workflows, or hooks — those stay on `anvil start`.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;

use crate::GlobalArgs;
use crate::activation;
use crate::activation::diagnostic::ConfigStatus;
use crate::activation::mcp_client::AnvilEntry;
use crate::activation::orchestrator::install::{InstallOutcome, ensure_existing_mcp_entries};
use crate::output::AlreadyReported;
use crate::registration::{self, WorktreeRegistration};
use crate::util;

/// Human recovery when the repo has never been activated (config Absent).
pub(crate) const NOT_ACTIVATED_MESSAGE: &str =
    "anvil: not activated in this repository. Run `anvil start` to activate protection.\n\
     anvil: new here? Run `anvil welcome` for a guided tour.";

/// Human recovery when MCP was never installed (or was declined).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const MCP_NOT_INSTALLED_MESSAGE: &str =
    "anvil: MCP not installed for this machine — run `anvil start` to configure it \
     (or `anvil start --no-mcp` if you only want daemon-backed protection).";

#[derive(Debug, Serialize)]
struct EnsureJsonReport {
    surface: &'static str,
    protection: String,
    config: String,
    daemon: String,
    worktree: String,
    mcp: String,
    next: Option<String>,
}

pub fn run(global: &GlobalArgs) -> anyhow::Result<()> {
    let root = util::workspace_root().unwrap_or_else(|_| PathBuf::from("."));
    let root = root.as_path();

    let probe = activation::verify(root);
    if probe.config == ConfigStatus::Absent {
        return report_not_activated(global, root);
    }

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
    let daemon_outcome = crate::commands::intercept::ensure_save_time_daemon(capability);
    let daemon_line = format_daemon_outcome(&daemon_outcome);

    // Worktree registration when cwd is registerable (no project-init writes).
    let worktree_line = match registration::registerable_worktree(root) {
        Ok(path) => format_worktree_registration(registration::register_worktree_with_daemon(
            &path,
        )),
        Err(reason) => format!("worktree: not registerable ({reason})"),
    };

    // MCP ensure-only (skip entirely under ANVIL_NO_MCP).
    let mcp_line = if mcp_opt_out() {
        "mcp: skipped (`ANVIL_NO_MCP`)".to_string()
    } else {
        match std::env::current_exe() {
            Ok(exe) => {
                let fresh = AnvilEntry::local_stdio(exe);
                let home = util::user_home_dir();
                let summary = ensure_existing_mcp_entries(root, home.as_deref(), &fresh);
                format_mcp_line(&summary.report, summary.managed, summary.absent_for_recovery)
            }
            Err(err) => format!("mcp: could not resolve anvil executable ({err})"),
        }
    };

    // Final protection probe after ensure.
    let diagnostic = activation::verify(root);
    let protection = diagnostic.protection_state();
    let next = next_action_line(protection, &mcp_line);

    if global.json {
        let doc = EnsureJsonReport {
            surface: "ensure",
            protection: protection.label().to_string(),
            config: diagnostic.config.label().to_string(),
            daemon: daemon_line,
            worktree: worktree_line,
            mcp: mcp_line,
            next,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).context("serialise ensure report")?
        );
        if daemon_failed(&daemon_outcome) {
            return Err(AlreadyReported.into());
        }
        return Ok(());
    }

    println!("anvil ensure");
    println!("  protection: {}", protection.label());
    println!("  {}", protection.headline());
    println!("  {daemon_line}");
    println!("  {worktree_line}");
    println!("  {mcp_line}");
    if let Some(next) = next {
        println!("  next: {next}");
    }

    if daemon_failed(&daemon_outcome) {
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
            worktree: root.display().to_string(),
            mcp: "skipped".to_string(),
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

fn daemon_opt_out() -> bool {
    std::env::var_os("ANVIL_NO_DAEMON").is_some_and(|value| !value.is_empty())
}

fn mcp_opt_out() -> bool {
    std::env::var_os("ANVIL_NO_MCP").is_some_and(|value| !value.is_empty())
}

fn format_daemon_outcome(outcome: &anvil_intercept::ensure::EnsureOutcome) -> String {
    use anvil_intercept::ensure::EnsureOutcome;
    match outcome {
        EnsureOutcome::Reused => "daemon: running".to_string(),
        EnsureOutcome::Started => "daemon: started".to_string(),
        EnsureOutcome::NoStart { reason } => {
            format!("daemon: not started ({})", reason.as_str())
        }
        EnsureOutcome::Failed { recovery } => format!("daemon: failed — {recovery}"),
    }
}

fn daemon_failed(outcome: &anvil_intercept::ensure::EnsureOutcome) -> bool {
    matches!(
        outcome,
        anvil_intercept::ensure::EnsureOutcome::Failed { .. }
    )
}

fn format_worktree_registration(outcome: WorktreeRegistration) -> String {
    match outcome {
        WorktreeRegistration::Registered => {
            "worktree: registered with the save-time daemon".to_string()
        }
        WorktreeRegistration::Refreshed => {
            "worktree: registration refreshed".to_string()
        }
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
        return format!("mcp: updated {repaired} anvil-owned entr(y/ies)");
    }
    if managed > 0 {
        return "mcp: anvil entry present".to_string();
    }
    if absent_for_recovery > 0 {
        return "mcp: not installed — run `anvil start` to configure".to_string();
    }
    "mcp: no anvil-owned entry to ensure".to_string()
}

fn next_action_line(
    state: activation::state::ProtectionState,
    mcp_line: &str,
) -> Option<String> {
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

    #[test]
    fn not_activated_message_names_start_and_welcome() {
        assert!(NOT_ACTIVATED_MESSAGE.contains("anvil start"));
        assert!(NOT_ACTIVATED_MESSAGE.contains("anvil welcome"));
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
}
